use std::sync::Once;

pub use magick_rust::MagickError as ImageError;
use magick_rust::{
    bindings::CompositeOperator_DstOverCompositeOp, magick_wand_genesis, MagickError, MagickWand,
};

static MAGICK: Once = Once::new();

fn ensure_init() {
    MAGICK.call_once(magick_wand_genesis);
}

pub struct Dds {
    wand: MagickWand,
}

impl Dds {
    pub fn flask(&mut self) -> Result<(), MagickError> {
        let width = self.wand.get_image_width() / 3;
        let height = self.wand.get_image_height();

        let layer1 = self.wand.clone();
        let layer2 = self.wand.clone();

        layer1.crop_image(width, height, width as isize, 0)?;
        layer2.crop_image(width, height, width as isize * 2, 0)?;
        self.wand.crop_image(width, height, 0, 0)?;

        // No clue if this is correct, it looks alright...
        layer2.compose_images(&layer1, CompositeOperator_DstOverCompositeOp, true, 0, 0)?;
        self.wand
            .compose_images(&layer2, CompositeOperator_DstOverCompositeOp, true, 0, 0)?;

        Ok(())
    }

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
