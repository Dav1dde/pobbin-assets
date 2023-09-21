use crate::BundleFs;

mod gems;
mod wiki;

pub use gems::Gems;

#[derive(Debug)]
pub struct Data {
    pub gems: Gems,
}

pub fn generate<F: BundleFs>(fs: F) -> anyhow::Result<Data> {
    tracing::info!("generating gem info");
    let gems = gems::generate(fs)?;

    Ok(Data { gems })
}
