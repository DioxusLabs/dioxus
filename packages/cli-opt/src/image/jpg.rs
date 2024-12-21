use image::{DynamicImage, EncodableLayout};
use std::{
    io::{BufWriter, Write},
    path::Path,
};

pub(crate) fn compress_jpg(image: DynamicImage, output_location: &Path) -> anyhow::Result<()> {
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_EXT_RGBX);
    let width = image.width() as usize;
    let height = image.height() as usize;

    comp.set_size(width, height);
    let mut comp = comp.start_compress(Vec::new())?; // any io::Write will work

    comp.write_scanlines(image.to_rgba8().as_bytes())?;

    let jpeg_bytes = comp.finish()?;

    let file = std::fs::File::create(output_location)?;
    let w = &mut BufWriter::new(file);
    w.write_all(&jpeg_bytes)?;
    Ok(())
}
