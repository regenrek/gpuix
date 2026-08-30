use std::fs::File;
use std::io::BufWriter;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder as _, RgbaImage};

pub(crate) fn write_png(path: &str, image: &RgbaImage) -> image::ImageResult<()> {
    PngEncoder::new_with_quality(
        BufWriter::new(File::create(path)?),
        CompressionType::Fast,
        FilterType::NoFilter,
    )
    .write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgba8,
    )
}
