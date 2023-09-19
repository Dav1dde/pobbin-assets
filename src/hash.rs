pub enum HashStrategy {
    Fnv3_11_2,
    Murmur3_21_2,
}

impl HashStrategy {
    pub fn path(&self, path: &str) -> u64 {
        match self {
            Self::Fnv3_11_2 => {
                let mut hasher = crate::utils::Fnv1a64::new();
                hasher.update(path.to_lowercase().as_bytes());
                hasher.update(b"++");
                hasher.finalize()
            }
            Self::Murmur3_21_2 => murmur2::murmur64a(path.to_lowercase().as_bytes(), 0x1337b33f),
        }
    }
}
