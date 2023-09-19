use super::file::VarDataReader;
use crate::BundleFile;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("not enough data")]
    NotEnoughData,
}

pub trait Row {
    const FILE: &'static str;

    type Item<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>)
        -> Result<Self::Item<'a>, ParseError>;
}

impl<T: Row> BundleFile for T {
    const NAME: &'static str = T::FILE;

    type Output = super::DatFile<'static, Self>;

    fn from(data: Vec<u8>) -> Self::Output {
        Self::Output::new(std::borrow::Cow::Owned(data))
    }
}
