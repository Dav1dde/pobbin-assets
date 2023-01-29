use std::collections::HashMap;

use super::{ooz, parse, BundleFs};

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
        let index_file = decompress(&self.fs, "Bundles2/_.index.bin", None)?;
        IndexBundle::parse(&self.fs, index_file)
    }
}

pub struct IndexBundle<F: BundleFs> {
    fs: F,
    refs: HashMap<u64, FileRef>,
}

impl<F: BundleFs> IndexBundle<F> {
    fn parse(fs: F, data: Vec<u8>) -> BundleResult<Self> {
        tracing::info!("parsing index bundle");
        let (_, ib) = parse::IndexBundle::parse(&data)?;

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

        tracing::info!("parsed {} files from index bundle", refs.len());

        Ok(Self { fs, refs })
    }

    pub fn read<T: BundleFile>(&self) -> BundleResult<Option<T::Output>> {
        self.read_by_name(T::NAME).map(|r| r.map(T::from))
    }

    pub fn read_by_name(&self, name: &str) -> BundleResult<Option<Vec<u8>>> {
        let hash = crate::filepath_hash(name);
        let Some(fref) = self.refs.get(&hash) else {
            tracing::warn!("file '{name}' not found in index bundle");
            return Ok(None);
        };

        let bundle_name = format!("Bundles2/{}.bundle.bin", fref.bundle_name);
        tracing::info!(
            "reading file '{name}' from bundle '{bundle_name}' @ {} ({} bytes)",
            fref.file_offset,
            fref.file_size
        );

        let content = decompress(&self.fs, &bundle_name, Some(fref))?;

        tracing::info!(
            "successfully loaded file '{name}' from bundle '{bundle_name}' with {} bytes",
            content.len()
        );

        Ok(Some(content))
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

fn decompress<F: BundleFs>(fs: &F, name: &str, fref: Option<&FileRef>) -> BundleResult<Vec<u8>> {
    let mut file = fs.get(name).map_err(BundleError::Fs)?;

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
