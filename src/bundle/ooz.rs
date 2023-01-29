use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum DecompressionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Failed to decompress with error {0}")]
    Ooz(i32),
}

/// Decompresses a subset of chunks from an already advanced reader.
///
/// The next chunks read from the reader are specified in `chunk_sizes`,
/// the chunks are decompressed and accumulated and returned.
///
/// `chunk_start` indicates the total offset of chunks already read or skipped,
/// this is required together with `uncompressed_size` to track the last chunk
/// which can be smaller than `chunk_unpacked_size`.
pub fn decompress(
    reader: &mut impl Read,
    chunk_unpacked_size: usize,
    chunk_sizes: &[u32],
    chunk_start: usize,
    uncompressed_size: usize,
) -> Result<Vec<u8>, DecompressionError> {
    let uncompressed_offset = chunk_start * chunk_unpacked_size;

    // +64 because there is a rumour that libooz can write past the buffer ...
    // and I am afraid, I am petrified.
    let mut content = Vec::with_capacity(chunk_unpacked_size * chunk_sizes.len() + 64);
    let content_uninit = content.spare_capacity_mut();

    let mut buffer = Vec::new();

    let mut current_size = 0usize;
    for &chunk_size in chunk_sizes {
        let chunk_size = chunk_size as usize;

        // Last chunk uncompressed size might be smaller than chunk_unpacked_size.
        let this_chunk_unpacked_size =
            chunk_unpacked_size.min(uncompressed_size - uncompressed_offset - current_size);

        buffer.resize(chunk_size, 0);
        reader.read_exact(&mut buffer)?;

        let n = libooz_sys::decompress_uninit(
            &buffer,
            &mut content_uninit[current_size..current_size + this_chunk_unpacked_size],
        );

        if n < 0 {
            return Err(DecompressionError::Ooz(n));
        }
        debug_assert_eq!(n as usize, this_chunk_unpacked_size);

        current_size += this_chunk_unpacked_size;
    }

    // SAFETY: current_size can't be bigger than the capacity,
    // we only slice over uninit, if it would exceed the capacity it would have already paniced.
    unsafe {
        debug_assert!(chunk_unpacked_size * chunk_sizes.len() >= current_size);
        content.set_len(current_size);
    };

    Ok(content)
}
