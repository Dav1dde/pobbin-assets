use std::{collections::HashMap, rc::Rc};

use super::{
    ooz,
    parse::{self, PathRep},
    BundleFs,
};
use crate::Discard;

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("failed to get file from filesystem: {0}")]
    Fs(super::BundleFsError),
    #[error("failed to read file: {0}")]
    Io(std::io::Error),
    #[error("failed to parse file: {0}")]
    Parse(nom::Err<nom::error::Error<()>>),
    #[error("failed to decompress file: {0}")]
    Decompress(i32),
}

impl<T> From<nom::Err<nom::error::Error<T>>> for BundleError {
    fn from(err: nom::Err<nom::error::Error<T>>) -> Self {
        Self::Parse(err.map_input(|_| ()))
    }
}

pub type BundleResult<T> = Result<T, BundleError>;

pub struct Bundle<F: BundleFs> {
    fs: F,
}

impl<F: BundleFs> Bundle<F> {
    pub fn new(fs: F) -> Self {
        Self { fs }
    }

    pub fn index(&self) -> BundleResult<IndexBundle<&F>> {
        let file = self
            .fs
            .get("Bundles2/_.index.bin")
            .map_err(BundleError::Fs)?;
        let index_file = decompress(file, None)?;
        IndexBundle::parse(&self.fs, index_file)
    }
}

pub struct IndexBundle<F: BundleFs> {
    fs: F,
    refs: HashMap<u64, FileRef>,
    reps: Vec<PathRep>,
    data: Vec<u8>,
    path_offset: usize,
}

impl<F: BundleFs> IndexBundle<F> {
    fn parse(fs: F, data: Vec<u8>) -> BundleResult<Self> {
        tracing::trace!("parsing index bundle");
        let (rem, ib) = parse::IndexBundle::parse(&data)?;
        let path_offset = unsafe { rem.as_ptr().offset_from(data.as_ptr()) } as usize;

        let mut refs = HashMap::new();

        for file in ib.files {
            let bundle = &ib.bundles[file.bundle_index as usize];

            refs.insert(
                file.hash,
                FileRef {
                    bundle_name: bundle.name.to_owned(),
                    file_offset: file.file_offset as usize,
                    file_size: file.file_size as usize,
                },
            );
        }

        tracing::trace!("parsed {} files from index bundle", refs.len());

        Ok(Self {
            fs,
            refs,
            reps: ib.reps,
            data,
            path_offset,
        })
    }

    pub fn read<T: BundleFile>(&self) -> BundleResult<Option<T::Output>> {
        self.read_by_name(T::NAME).map(|r| r.map(T::from))
    }

    pub fn read_by_name(&self, name: &str) -> BundleResult<Option<Vec<u8>>> {
        let hash = crate::HashStrategy::Murmur3_21_2.path(name); // TODO: make configurable
        let Some(fref) = self.refs.get(&hash) else {
            tracing::warn!("file '{name}' not found in index bundle");
            return Ok(None);
        };

        let bundle_name = format!("Bundles2/{}.bundle.bin", fref.bundle_name);
        tracing::trace!(
            "reading file '{name}' from bundle '{bundle_name}' @ {} ({} bytes)",
            fref.file_offset,
            fref.file_size
        );

        let file = self.fs.get(&bundle_name).map_err(BundleError::Fs)?;
        let content = decompress(file, Some(fref))?;

        tracing::trace!(
            "successfully loaded file '{name}' from bundle '{bundle_name}' with {} bytes",
            content.len()
        );

        Ok(Some(content))
    }

    // TODO this needs to yield `Item = Result<String>`
    pub fn files(&self) -> BundleResult<impl Iterator<Item = String> + '_> {
        let data = Rc::new(decompress(&mut &self.data[self.path_offset..], None)?);

        // TODO: this could be one iterator owning reps and data without Rc but this is good enough
        // for now
        let files = self.reps.iter().flat_map(move |rep| {
            let data = data.clone();
            RepIter::new(data, rep)
        });

        Ok(files)
    }
}

struct RepIter {
    data: Rc<Vec<u8>>,
    current: usize,
    end: usize,
    base_phase: bool,
    bases: Vec<String>,
}

impl RepIter {
    fn new(data: Rc<Vec<u8>>, rep: &PathRep) -> Self {
        Self {
            data,
            current: rep.payload_offset as usize,
            end: rep.payload_offset as usize + rep.payload_size as usize,
            base_phase: false,
            bases: Vec::new(),
        }
    }
}

impl Iterator for RepIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut slice = &self.data[self.current..self.end];

        while !slice.is_empty() {
            // TODO: proper parsing
            let cmd = u32::from_le_bytes(slice[..4].try_into().unwrap());
            slice = &slice[4..];
            self.current += 4;

            if cmd == 0 {
                self.base_phase = !self.base_phase;
                if self.base_phase {
                    self.bases.clear();
                }
                continue;
            }

            // TODO panics everywhere
            let i = slice.iter().position(|&b| b == 0).unwrap();
            let s = std::str::from_utf8(&slice[..i]).unwrap(); // TODO
            slice = &slice[i + 1..];
            self.current += i + 1;

            let s = if let Some(val) = self.bases.get(cmd as usize - 1) {
                val.clone() + s
            } else {
                s.to_owned()
            };

            if self.base_phase {
                self.bases.push(s);
            } else {
                return Some(s);
            }
        }

        None
    }
}

pub trait BundleFile {
    const NAME: &'static str;

    type Output;

    fn from(data: Vec<u8>) -> Self::Output;
}

#[derive(Debug)]
struct FileRef {
    bundle_name: String,
    file_offset: usize,
    file_size: usize,
}

fn decompress(
    mut file: (impl std::io::Read + Discard),
    fref: Option<&FileRef>,
) -> BundleResult<Vec<u8>> {
    let head = parse::Head::read(&mut file).map_err(|err| match err {
        parse::ReadErr::Io(err) => BundleError::Io(err),
        parse::ReadErr::Parse(err) => err.into(),
    })?;

    let chunk_unpacked_size = head.payload.chunk_unpacked_size as usize;
    let uncompressed_size = head.payload.uncompressed_size as usize;

    let file_offset = fref.map(|fref| fref.file_offset).unwrap_or(0);
    let file_size = fref.map(|fref| fref.file_size).unwrap_or(uncompressed_size);

    // First chunk that includes a part of the targeted file.
    let num_chunk_start = file_offset / chunk_unpacked_size;
    // Last chunk that includes a part of the targeted file.
    let num_chunk_end = div_ceil(file_offset + file_size, chunk_unpacked_size);

    let chunks_start: usize = head.payload.chunk_sizes[..num_chunk_start]
        .iter()
        .map(|&s| s as usize)
        .sum();

    file.discard(chunks_start as u64).map_err(BundleError::Fs)?;

    let mut content = ooz::decompress(
        &mut file,
        chunk_unpacked_size,
        &head.payload.chunk_sizes[num_chunk_start..num_chunk_end],
        num_chunk_start,
        uncompressed_size,
    )
    .map_err(|err| match err {
        ooz::DecompressionError::Io(err) => BundleError::Io(err),
        ooz::DecompressionError::Ooz(err) => BundleError::Decompress(err),
    })?;

    // If the file does not starts at the beginning of the buffer,
    // we have to move its contents to the start of the buffer first
    // and then truncate the buffer to the file size.
    // The file offset is always 0 when `fref` is `None` (entire file decompressed).
    if file_offset > 0 {
        let file_decompress_start = file_offset - num_chunk_start * chunk_unpacked_size;
        content.copy_within(file_decompress_start..file_decompress_start + file_size, 0);
    }

    content.truncate(file_size);
    Ok(content)
}

fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
