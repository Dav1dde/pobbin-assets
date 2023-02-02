#[inline]
#[track_caller]
pub fn parse_u64(data: &[u8]) -> u64 {
    u64::from_le_bytes((&data[0..8]).try_into().unwrap())
}
