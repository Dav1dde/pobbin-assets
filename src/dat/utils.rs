use super::row::ParseError;

#[inline]
#[track_caller]
pub fn parse_u64(data: &[u8], idx: usize) -> Result<u64, ParseError> {
    let data = data.get(idx..idx + 8).ok_or(ParseError::NotEnoughData)?;
    Ok(u64::from_le_bytes(data.try_into().unwrap()))
}

#[inline]
#[track_caller]
pub fn parse_u32(data: &[u8], idx: usize) -> Result<u32, ParseError> {
    let data = data.get(idx..idx + 4).ok_or(ParseError::NotEnoughData)?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}
