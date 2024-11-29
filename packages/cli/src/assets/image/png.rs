use std::{io::BufWriter, path::Path};

use image::DynamicImage;

pub(crate) fn compress_png(image: DynamicImage, output_location: &Path) {
    // Image loading/saving is outside scope of this library
    let width = image.width() as usize;
    let height = image.height() as usize;
    let bitmap: Vec<_> = image
        .into_rgba8()
        .pixels()
        .map(|px| imagequant::RGBA::new(px[0], px[1], px[2], px[3]))
        .collect();

    // Configure the library
    let mut liq = imagequant::new();
    liq.set_speed(5).unwrap();
    liq.set_quality(0, 99).unwrap();

    // Describe the bitmap
    let mut img = liq.new_image(&bitmap[..], width, height, 0.0).unwrap();

    // The magic happens in quantize()
    let mut res = match liq.quantize(&mut img) {
        Ok(res) => res,
        Err(err) => panic!("Quantization failed, because: {err:?}"),
    };

    let (palette, pixels) = res.remapped(&mut img).unwrap();

    let file = std::fs::File::create(output_location).unwrap();
    let w = &mut BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    let mut flattened_palette = Vec::new();
    let mut alpha_palette = Vec::new();
    for px in palette {
        flattened_palette.push(px.r);
        flattened_palette.push(px.g);
        flattened_palette.push(px.b);
        alpha_palette.push(px.a);
    }
    encoder.set_palette(flattened_palette);
    encoder.set_trns(alpha_palette);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_compression(png::Compression::Best);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
}
