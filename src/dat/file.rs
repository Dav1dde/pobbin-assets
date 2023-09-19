use std::borrow::Cow;

use super::{row::ParseError, utils::parse_u64, Row};

const VDATA_MAGIC: &[u8] = &[0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb];

#[derive(Copy, Clone)]
pub struct VarDataReader<'a>(&'a [u8]);

impl<'a> VarDataReader<'a> {
    pub fn get_string_from(&self, data: &[u8], idx: usize) -> Result<DatString<'a>, ParseError> {
        let loc = parse_u64(data, idx)?;
        self.get_string(loc)
    }

    pub fn get_string(&self, offset: u64) -> Result<DatString<'a>, ParseError> {
        let offset = offset as usize;
        let idx = self.0[offset..]
            .chunks_exact(2)
            .position(|a| a == [0, 0])
            .map(|idx| idx * 2)
            .unwrap_or(self.0.len());
        let data = self
            .0
            .get(offset..offset + idx)
            .ok_or(ParseError::NotEnoughData)?;
        Ok(DatString(data))
    }
}

#[derive(Copy, Clone)]
pub struct DatString<'a>(pub(crate) &'a [u8]);

impl<'a> DatString<'a> {
    pub fn contains(&self, p: char) -> bool {
        self.chars().any(|c| c == Ok(p))
    }

    pub fn ends_with(&self, other: &str) -> bool {
        let mut other = other.chars().rev().fuse();
        (&mut other).zip(self.chars_rev()).all(|(a, b)| Ok(a) == b) && other.next().is_none()
    }

    pub fn starts_with(&self, other: &str) -> bool {
        let mut other = other.chars().fuse();
        (&mut other).zip(self.chars()).all(|(a, b)| Ok(a) == b) && other.next().is_none()
    }

    fn chars(&self) -> impl Iterator<Item = Result<char, std::char::DecodeUtf16Error>> + '_ {
        let u16s = self
            .0
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]));
        char::decode_utf16(u16s)
    }

    fn chars_rev(&self) -> impl Iterator<Item = Result<char, std::char::DecodeUtf16Error>> + '_ {
        let u16s = self
            .0
            .chunks_exact(2)
            .rev()
            .map(|c| u16::from_le_bytes([c[0], c[1]]));
        char::decode_utf16(u16s)
    }
}

impl<'a> TryFrom<&DatString<'a>> for String {
    type Error = std::char::DecodeUtf16Error;

    fn try_from(s: &DatString<'a>) -> Result<Self, Self::Error> {
        s.chars().collect::<Result<_, _>>()
    }
}

impl<'a> TryFrom<DatString<'a>> for Cow<'static, str> {
    type Error = std::char::DecodeUtf16Error;

    fn try_from(s: DatString<'a>) -> Result<Self, Self::Error> {
        s.chars().collect::<Result<_, _>>()
    }
}

impl<'a> std::fmt::Debug for DatString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DatString({:?})", String::try_from(self))
    }
}

pub struct DatFile<'a, R: Row> {
    pub row_count: usize,
    row_size: usize,
    data: Cow<'a, [u8]>,
    boundary: usize,
    _row: std::marker::PhantomData<R>,
}

impl<'a, R: Row> DatFile<'a, R> {
    pub fn new(data: impl Into<Cow<'a, [u8]>>) -> Self {
        let data = data.into();
        // TODO: this can panic
        let row_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        // TODO: errors
        let boundary = data
            .windows(VDATA_MAGIC.len())
            .position(|window| window == VDATA_MAGIC)
            .expect("magic");

        let row_size = (boundary - 4) / row_count;

        Self {
            row_count,
            row_size,
            data,
            boundary,
            _row: Default::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = R::Item<'_>> + '_ {
        let vdr = VarDataReader(&self.data[self.boundary..]);
        self.data[4..]
            .chunks_exact(self.row_size)
            .take(self.row_count)
            .map(move |row| R::parse(row, vdr).unwrap()) // TODO: unwrap
    }

    pub fn get(&self, index: usize) -> Option<R::Item<'_>> {
        let start = 4 + index * self.row_size;
        let end = start + self.row_size;
        let vdr = VarDataReader(&self.data[self.boundary..]);
        self.data
            .get(start..end)
            .map(|row| R::parse(row, vdr).unwrap()) // TODO: unwrap
    }
}

impl<'a, R: Row> std::fmt::Debug for DatFile<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatFile")
            .field("row_count", &self.row_count)
            .finish_non_exhaustive()
    }
}
