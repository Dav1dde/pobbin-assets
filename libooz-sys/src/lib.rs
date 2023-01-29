use std::mem::MaybeUninit;

extern "C" {
    fn Ooz_Decompress(src_buf: *const u8, src_len: u32, dst: *mut u8, dst_size: usize) -> i32;
}

pub fn decompress(src: &[u8], dst: &mut [u8]) -> i32 {
    unsafe { Ooz_Decompress(src.as_ptr(), src.len() as u32, dst.as_mut_ptr(), dst.len()) }
}

pub fn decompress_uninit(src: &[u8], dst: &mut [MaybeUninit<u8>]) -> i32 {
    unsafe {
        Ooz_Decompress(
            src.as_ptr(),
            src.len() as u32,
            dst.as_mut_ptr() as *mut u8,
            dst.len(),
        )
    }
}
