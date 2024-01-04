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

    pub fn gem(&mut self) -> Result<(), MagickError> {
        // This is just a quick workaround for the new gem format, this should be implemented
        // properly with a separate gem pipeline which also supports the shaded transfigured gems.
        let width = self.wand.get_image_width();
        let height = self.wand.get_image_height();

        // A regular gem.
        if width < height + 10 {
            return Ok(());
        }

        let width = width / 3;

        let layer = self.wand.clone();

        layer.crop_image(width, height, width as isize * 2, 0)?;
        self.wand.crop_image(width, height, 0, 0)?;
        self.wand
            .compose_images(&layer, CompositeOperator_DstOverCompositeOp, true, 0, 0)?;

        Ok(())
    }

    pub fn crop(&mut self, pos: (u32, u32), size: (u32, u32)) -> Result<(), MagickError> {
        self.wand.crop_image(
            size.0 as usize,
            size.1 as usize,
            pos.0 as isize,
            pos.1 as isize,
        )
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.wand.resize_image(width, height, 0);
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
