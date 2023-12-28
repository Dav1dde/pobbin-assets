use std::io::{Cursor, Read, Seek};

use dashmap::DashMap;

#[derive(Debug, thiserror::Error)]
pub enum BundleFsError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(feature = "web")]
    #[error(transparent)]
    Web(#[from] Box<ureq::Error>), // boxed because of clippy::result_large_err
    #[error(transparent)]
    Dyn(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

enum FsRead {
    File(std::fs::File),
    // If necessary, probably should add a Box<dyn Read + Seek>
    Boxed(Box<dyn std::io::Read + Send + Sync + 'static>),
    Cursor(Cursor<Vec<u8>>),
}

pub struct FileContents {
    inner: FsRead,
}

impl FileContents {
    /// Discards a certain amount of bytes as efficiently as possible.
    ///
    /// This uses seek under the hood if supported, otherwise reads
    /// and discards the bytes.
    pub fn discard(&mut self, n: u64) -> Result<(), BundleFsError> {
        match self.inner {
            FsRead::File(ref mut file) => {
                file.seek(std::io::SeekFrom::Current(n as i64))?;
            }
            FsRead::Cursor(ref mut cursor) => {
                cursor.seek(std::io::SeekFrom::Current(n as i64))?;
            }
            FsRead::Boxed(ref mut read) => {
                std::io::copy(&mut read.take(n), &mut std::io::sink())?;
            }
        };

        Ok(())
    }
}

impl std::io::Read for FileContents {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.inner {
            FsRead::File(ref mut read) => read.read(buf),
            FsRead::Boxed(ref mut read) => read.read(buf),
            FsRead::Cursor(ref mut read) => read.read(buf),
        }
    }
}

impl From<std::fs::File> for FileContents {
    fn from(file: std::fs::File) -> Self {
        Self {
            inner: FsRead::File(file),
        }
    }
}

impl From<Box<dyn std::io::Read + Send + Sync + 'static>> for FileContents {
    fn from(read: Box<dyn std::io::Read + Send + Sync + 'static>) -> Self {
        Self {
            inner: FsRead::Boxed(read),
        }
    }
}

impl From<Vec<u8>> for FileContents {
    fn from(data: Vec<u8>) -> Self {
        Self {
            inner: FsRead::Cursor(Cursor::new(data)),
        }
    }
}

pub trait Discard {
    fn discard(&mut self, n: u64) -> Result<(), BundleFsError>;
}

impl Discard for FileContents {
    fn discard(&mut self, n: u64) -> Result<(), BundleFsError> {
        self.discard(n)
    }
}

impl Discard for &mut &[u8] {
    fn discard(&mut self, n: u64) -> Result<(), BundleFsError> {
        **self = &self[self.len().min(n as usize)..];
        Ok(())
    }
}

pub trait BundleFs {
    fn get(&self, name: &str) -> Result<FileContents, BundleFsError>;
}

impl<T: BundleFs> BundleFs for &T {
    fn get(&self, name: &str) -> Result<FileContents, BundleFsError> {
        (*self).get(name)
    }
}

impl BundleFs for &dyn BundleFs {
    fn get(&self, name: &str) -> Result<FileContents, BundleFsError> {
        (*self).get(name)
    }
}

impl BundleFs for Box<dyn BundleFs> {
    fn get(&self, name: &str) -> Result<FileContents, BundleFsError> {
        self.as_ref().get(name)
    }
}

#[derive(Debug)]
pub struct LocalBundleFs {
    base: std::path::PathBuf,
}

impl LocalBundleFs {
    pub fn new(base: impl Into<std::path::PathBuf>) -> Self {
        Self { base: base.into() }
    }
}

impl BundleFs for LocalBundleFs {
    fn get(&self, name: &str) -> Result<FileContents, BundleFsError> {
        Ok(std::fs::File::open(self.base.join(name))?.into())
    }
}

pub trait Cache {
    fn get<F: BundleFs>(&self, name: &str, producer: F) -> Result<FileContents, BundleFsError>;
}

#[derive(Default)]
pub struct InMemoryCache(DashMap<String, Vec<u8>>);

impl InMemoryCache {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Cache for InMemoryCache {
    fn get<F: BundleFs>(&self, name: &str, producer: F) -> Result<FileContents, BundleFsError> {
        if let Some(data) = self.0.get(name) {
            return Ok(data.clone().into());
        }

        let data = self
            .0
            .entry(name.to_owned())
            .or_try_insert_with(|| {
                let mut data = Vec::new();
                producer.get(name)?.read_to_end(&mut data)?;
                Ok::<_, BundleFsError>(data)
            })?
            .clone();

        Ok(data.into())
    }
}

pub struct LocalCache(std::path::PathBuf);

impl LocalCache {
    pub fn new(base: impl Into<std::path::PathBuf>) -> Self {
        Self(base.into())
    }
}

impl Cache for LocalCache {
    fn get<F: BundleFs>(&self, name: &str, producer: F) -> Result<FileContents, BundleFsError> {
        let path = self.0.join(name);

        if let Ok(file) = std::fs::File::open(&path) {
            return Ok(file.into());
        }

        let mut tmp = tempfile::NamedTempFile::new_in(&self.0)?;
        let mut data = producer.get(name)?;
        std::io::copy(&mut data, &mut tmp)?;

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = match tmp.persist(path) {
            Ok(mut file) => {
                file.seek(std::io::SeekFrom::Start(0))?;
                file
            }
            Err(err) => {
                tracing::warn!("failed to rename tmp file {err:?}");
                err.file.reopen()?
            }
        };

        Ok(file.into())
    }
}

pub struct CacheBundleFs<F: BundleFs, C: Cache> {
    inner: F,
    cache: C,
}

impl<F: BundleFs, C: Cache> CacheBundleFs<F, C> {
    pub fn new(inner: F, cache: C) -> Self {
        Self { inner, cache }
    }
}

impl<F: BundleFs, C: Cache> BundleFs for CacheBundleFs<F, C> {
    fn get(&self, name: &str) -> Result<FileContents, BundleFsError> {
        self.cache.get(name, &self.inner)
    }
}

#[cfg(feature = "web")]
mod web {
    use super::*;

    #[derive(Debug)]
    pub struct WebBundleFs {
        base: String,
    }

    impl WebBundleFs {
        pub fn new(base: impl Into<String>) -> Self {
            Self { base: base.into() }
        }

        pub fn cdn(version: &str) -> Self {
            Self::new(format!("http://patch.poecdn.com/{version}/"))
        }
    }

    impl BundleFs for WebBundleFs {
        fn get(&self, name: &str) -> Result<FileContents, BundleFsError> {
            tracing::info!(name, "requesting file from web fs: {name}");
            let response = ureq::get(&format!("{}{name}", self.base))
                .call()
                .map_err(Box::new)?;
            Ok(response.into_reader().into())
        }
    }
}
#[cfg(feature = "web")]
pub use web::*;
