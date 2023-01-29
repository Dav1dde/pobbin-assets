use super::file::VarDataReader;
use crate::BundleFile;

pub trait Row {
    const FILE: &'static str;
    const SIZE: usize;

    type Item<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>) -> Self::Item<'a>;
}

impl<T: Row> BundleFile for T {
    const NAME: &'static str = T::FILE;

    type Output = super::DatFile<'static, Self>;

    fn from(data: Vec<u8>) -> Self::Output {
        Self::Output::new(std::borrow::Cow::Owned(data))
    }
}
