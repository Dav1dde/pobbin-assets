use std::io::Read;

use nom::bytes::streaming::take;
use nom::multi::{count, length_count};
use nom::number::streaming::{le_u32, le_u64};
use nom::sequence::Tuple;
use nom::{IResult, Parser};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Head {
    pub uncompressed_size: u32,
    pub total_payload_size: u32,
    // head_payload_size: u32,
    pub payload: HeadPayload,
}

impl Head {
    pub fn read(reader: &mut impl Read) -> Result<Self, ReadErr<nom::error::Error<()>>> {
        fn parse(inp: &[u8]) -> IResult<&[u8], Head, nom::error::Error<()>> {
            Head::parse(inp).map_err(|e| e.map_input(|_| ()))
        }

        // The header is always at least 60 bytes, then the variable data starts.
        // This means we're actually just doing two read calls for the entire header.
        nom_read(parse, 60, reader)
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (uncompressed_size, total_payload_size, _, payload)) =
            (le_u32, le_u32, le_u32, HeadPayload::parse).parse(input)?;
        // let (input, content) = take(payload.compressed_size as usize)(input)?;

        Ok((
            input,
            Self {
                uncompressed_size,
                total_payload_size,
                payload,
            },
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct HeadPayload {
    pub first_file_encode: u32,
    // unk10: u32,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub chunk_count: u32,
    pub chunk_unpacked_size: u32,
    // unk28: [u32; 4],
    pub chunk_sizes: Vec<u32>,
}

impl HeadPayload {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (
            input,
            (
                first_file_encode,
                _,
                uncompressed_size,
                compressed_size,
                chunk_count,
                chunk_unpacked_size,
                _,
            ),
        ) = (
            le_u32,
            le_u32,
            le_u64,
            le_u64,
            le_u32,
            le_u32,
            take(16usize),
        )
            .parse(input)?;

        // can't use count here because count requests one element at a time using Incomplete
        // so instead we do the quick math ourselves and request everything in one go
        let (input, chunk_sizes_data) =
            take(chunk_count as usize * std::mem::size_of::<u32>())(input)?;
        let (rem, chunk_sizes) = count(le_u32, chunk_count as usize)(chunk_sizes_data)?;
        debug_assert_eq!(rem.len(), 0);

        Ok((
            input,
            Self {
                first_file_encode,
                uncompressed_size,
                compressed_size,
                chunk_count,
                chunk_unpacked_size,
                chunk_sizes,
            },
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BundleEntry<'a> {
    pub name: &'a str,
    pub size: usize,
}

impl<'a> BundleEntry<'a> {
    pub fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, (data, size)) = (le_u32.flat_map(take), le_u32).parse(input)?;

        Ok((
            input,
            Self {
                // TODO: actually handle errors
                // I hate nom sometimes, this needs a complete custom error type?
                name: std::str::from_utf8(data).expect("invalid string"),
                size: size as usize,
            },
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct FileInfo {
    pub hash: u64,
    pub bundle_index: u32,
    pub file_offset: u32,
    pub file_size: u32,
}

impl FileInfo {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (hash, bundle_index, file_offset, file_size)) =
            (le_u64, le_u32, le_u32, le_u32).parse(input)?;
        Ok((
            input,
            Self {
                hash,
                bundle_index,
                file_offset,
                file_size,
            },
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PathRep {
    pub hash: u64,
    pub payload_offset: u32,
    pub payload_size: u32,
    pub payload_recursive_size: u32,
}

impl PathRep {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (hash, payload_offset, payload_size, payload_recursive_size)) =
            (le_u64, le_u32, le_u32, le_u32).parse(input)?;
        Ok((
            input,
            Self {
                hash,
                payload_offset,
                payload_size,
                payload_recursive_size,
            },
        ))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct IndexBundle<'a> {
    pub bundles: Vec<BundleEntry<'a>>,
    pub files: Vec<FileInfo>,
    pub head: Head,
}

impl<'a> IndexBundle<'a> {
    pub fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, bundles) = length_count(le_u32, BundleEntry::parse)(input)?;
        let (input, files) = length_count(le_u32, FileInfo::parse)(input)?;

        let (input, _reps) = length_count(le_u32, PathRep::parse)(input)?;
        let (input, head) = Head::parse(input)?;

        Ok((
            input,
            Self {
                bundles,
                files,
                head,
            },
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadErr<E> {
    Io(std::io::Error),
    Parse(nom::Err<E>),
}

pub fn nom_read<O, E, P>(
    mut parser: P,
    min_size: usize,
    mut reader: impl Read,
) -> Result<O, ReadErr<E>>
where
    P: for<'a> nom::Parser<&'a [u8], O, E>,
{
    let mut input = Vec::with_capacity(min_size);
    (&mut reader)
        .take(min_size as u64)
        .read_to_end(&mut input)
        .map_err(ReadErr::Io)?;

    loop {
        let to_read = match parser.parse(&input) {
            Ok((_, o)) => return Ok(o),
            Err(nom::Err::Incomplete(nom::Needed::Unknown)) => 1,
            Err(nom::Err::Incomplete(nom::Needed::Size(len))) => len.into(),
            Err(nom::Err::Error(e)) => return Err(ReadErr::Parse(nom::Err::Error(e))),
            Err(nom::Err::Failure(e)) => return Err(ReadErr::Parse(nom::Err::Failure(e))),
        };

        (&mut reader)
            .take(to_read as u64)
            .read_to_end(&mut input)
            .map_err(ReadErr::Io)?;
    }
}
