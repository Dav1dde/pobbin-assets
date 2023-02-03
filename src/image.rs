use std::sync::Once;

use magick_rust::{magick_wand_genesis, MagickError, MagickWand};

static MAGICK: Once = Once::new();

fn ensure_init() {
    MAGICK.call_once(magick_wand_genesis);
}

pub struct Dds {
    wand: MagickWand,
}

impl Dds {
    pub fn write_blob(&self, format: &str) -> Result<Vec<u8>, MagickError> {
        self.wand.write_image_blob(format)
    }
}

impl TryFrom<&[u8]> for Dds {
    type Error = MagickError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        ensure_init();

        let wand = MagickWand::new();
        wand.read_image_blob(value)?;

        Ok(Self { wand })
    }
}
