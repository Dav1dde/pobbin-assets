pub struct Fnv1a64(u64);

impl Fnv1a64 {
    pub fn new() -> Self {
        const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
        Self(FNV_OFFSET_BASIS)
    }

    pub fn update(&mut self, bytes: &[u8]) {
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = self.0;
        for byte in bytes {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        self.0 = hash;
    }

    pub fn finalize(self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn filepath_hash(name: &str) -> u64 {
    let mut hasher = Fnv1a64::new();
    hasher.update(name.to_lowercase().as_bytes());
    hasher.update(b"++");
    hasher.finalize()
}

#[allow(clippy::result_large_err)]
pub fn latest_patch_version() -> Result<String, ureq::Error> {
    let r = ureq::get(
        "https://raw.githubusercontent.com/poe-tool-dev/latest-patch-version/main/latest.txt",
    )
    .call()?;
    let ver = r.into_string()?;
    tracing::info!("latest patch version: {ver}");
    Ok(ver)
}
