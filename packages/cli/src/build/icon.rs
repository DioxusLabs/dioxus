// Part of the code comes from the Tauri CLI https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/icon.rs

use anyhow::Context;
use image::{
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::FilterType,
    open, DynamicImage, ExtendedColorType, ImageBuffer, ImageEncoder, Rgba,
};
use resvg::{tiny_skia, usvg};
use xml::writer::{EmitterConfig, XmlEvent};

use std::{
    fs::{create_dir_all, write, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{error::Result, Error};

enum Source {
    Svg(resvg::usvg::Tree),
    DynamicImage(DynamicImage),
}

impl Source {
    fn width(&self) -> u32 {
        match self {
            Self::Svg(svg) => svg.size().width() as u32,
            Self::DynamicImage(i) => i.width(),
        }
    }

    fn height(&self) -> u32 {
        match self {
            Self::Svg(svg) => svg.size().height() as u32,
            Self::DynamicImage(i) => i.height(),
        }
    }

    fn resize_exact(&self, size: u32) -> Result<DynamicImage> {
        match self {
            Self::Svg(svg) => {
                let mut pixmap = tiny_skia::Pixmap::new(size, size)
                    .ok_or_else(|| Error::Other(anyhow::anyhow!("Failed to create pixmap")))?;
                let scale = size as f32 / svg.size().height();
                resvg::render(
                    svg,
                    tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixmap.as_mut(),
                );
                let img_buffer =
                    ImageBuffer::from_raw(size, size, pixmap.take()).ok_or_else(|| {
                        Error::Other(anyhow::anyhow!("Failed to create image buffer"))
                    })?;
                Ok(DynamicImage::ImageRgba8(img_buffer))
            }
            Self::DynamicImage(i) => Ok(i.resize_exact(size, size, FilterType::Lanczos3)),
        }
    }
}

#[derive(Debug)]
struct PngEntry {
    size: u32,
    out_path: PathBuf,
}

fn android_png_entries(out_dir: &Path) -> Result<Vec<PngEntry>> {
    struct AndroidEntry {
        name: &'static str,
        size: u32,
        foreground_size: u32,
    }

    let mut entries = Vec::new();

    let targets = vec![
        AndroidEntry {
            name: "hdpi",
            size: 49,
            foreground_size: 162,
        },
        AndroidEntry {
            name: "mdpi",
            size: 48,
            foreground_size: 108,
        },
        AndroidEntry {
            name: "xhdpi",
            size: 96,
            foreground_size: 216,
        },
        AndroidEntry {
            name: "xxhdpi",
            size: 144,
            foreground_size: 324,
        },
        AndroidEntry {
            name: "xxxhdpi",
            size: 192,
            foreground_size: 432,
        },
    ];

    for target in targets {
        let folder_name = format!("mipmap-{}", target.name);
        let out_folder = out_dir.join(&folder_name);

        create_dir_all(&out_folder).context("Can't create Android mipmap output directory")?;

        entries.push(PngEntry {
            out_path: out_folder.join("ic_launcher_foreground.png"),
            size: target.foreground_size,
        });
        entries.push(PngEntry {
            out_path: out_folder.join("ic_launcher_round.png"),
            size: target.size,
        });
        entries.push(PngEntry {
            out_path: out_folder.join("ic_launcher.png"),
            size: target.size,
        });
    }

    Ok(entries)
}

// Resize image and save it to disk.
fn resize_and_save_png(
    source: &Source,
    size: u32,
    file_path: &Path,
    bg_color: Option<Rgba<u8>>,
) -> Result<()> {
    let mut image = source.resize_exact(size)?;

    if let Some(bg_color) = bg_color {
        let mut bg_img = ImageBuffer::from_fn(size, size, |_, _| bg_color);
        image::imageops::overlay(&mut bg_img, &image, 0, 0);
        image = bg_img.into();
    }

    let mut out_file = BufWriter::new(File::create(file_path)?);
    write_png(image.as_bytes(), &mut out_file, size)?;
    Ok(out_file.flush()?)
}

// Encode image data as png with compression.
fn write_png<W: Write>(image_data: &[u8], w: W, size: u32) -> Result<()> {
    let encoder = PngEncoder::new_with_quality(w, CompressionType::Best, PngFilterType::Adaptive);
    encoder
        .write_image(image_data, size, size, ExtendedColorType::Rgba8)
        .map_err(anyhow::Error::from)?;
    Ok(())
}

// Generate Android VectorDrawable XML from SVG data.
// This is used for Android 8.0+ (API 26+) to support vector drawables.
// Now only supports simple SVG
// Because adaptive icons seem to be scaled up to a certain extent, margins need to be reserved
// Currently we only use one icon as the foreground and background at the same time
fn android_vector_drawable(tree: &resvg::usvg::Tree, out_dir: &Path) -> Result<()> {
    create_dir_all(out_dir.join("mipmap-anydpi-v26"))?;
    write(
        out_dir.join("mipmap-anydpi-v26").join("ic_launcher.xml"),
        include_bytes!(
            "../../assets/android/gen/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml"
        ),
    )?;

    let mut buf = Vec::new();
    let mut xml = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut buf);

    let width = tree.size().width().to_string();
    let height = tree.size().height().to_string();

    xml.write(
        XmlEvent::start_element("vector")
            .attr(
                "xmlns:android",
                "http://schemas.android.com/apk/res/android",
            )
            .attr("xmlns:aapt", "http://schemas.android.com/aapt")
            .attr("android:width", "108dp")
            .attr("android:height", "108dp")
            .attr("android:viewportWidth", &width)
            .attr("android:viewportHeight", &height),
    )
    .map_err(|e| Error::Other(anyhow::anyhow!(e)))?;

    // Here the recursive conversion node
    for node in tree.root().children() {
        usvg_node_to_vector_drawable(&node, &mut xml)?;
    }

    xml.write(XmlEvent::end_element())
        .map_err(|e| Error::Other(anyhow::anyhow!(e)))?;

    create_dir_all(out_dir.join("drawable"))?;
    write(
        out_dir.join("drawable").join("ic_launcher_background.xml"),
        &buf,
    )?;
    create_dir_all(out_dir.join("drawable-v24"))?;
    write(
        out_dir
            .join("drawable-v24")
            .join("ic_launcher_foreground.xml"),
        &buf,
    )?;
    Ok(())
}

fn usvg_node_to_vector_drawable<W: Write>(
    node: &usvg::Node,
    xml: &mut xml::writer::EventWriter<W>,
) -> Result<()> {
    match node {
        usvg::Node::Path(path) => {
            let data = svg_path_to_string(path.data());
            let fill_alpha = path
                .fill()
                .as_ref()
                .map(|f| f.opacity().get())
                .unwrap_or(1.0);
            let fill_alpha = (fill_alpha * 255.0).round() as u8;
            let fill = path
                .fill()
                .map(|f| {
                    match f.paint() {
                        usvg::Paint::Color(color) => {
                            format!(
                                "#{:02X}{:02X}{:02X}{:02X}",
                                fill_alpha, color.red, color.green, color.blue
                            )
                        }
                        _ => {
                            // Currently for gradients just uses black as a fallback
                            "#00000000".to_string()
                        }
                    }
                })
                .unwrap_or("#00000000".to_string());

            let fill_type = path
                .fill()
                .map(|f| match f.rule() {
                    usvg::FillRule::NonZero => "nonZero",
                    usvg::FillRule::EvenOdd => "evenOdd",
                })
                .unwrap_or("nonZero");

            let (stroke_color, stroke_width) = if let Some(stroke) = path.stroke() {
                let color = match stroke.paint() {
                    usvg::Paint::Color(color) => {
                        let alpha = (stroke.opacity().get() * 255.0).round() as u8;
                        format!(
                            "#{:02X}{:02X}{:02X}{:02X}",
                            alpha, color.red, color.green, color.blue
                        )
                    }
                    _ => "#00000000".to_string(),
                };
                let width = stroke.width().get();
                (color, width)
            } else {
                ("#00000000".to_string(), 0.0)
            };

            let stroke_width_str = stroke_width.to_string();

            let mut elem = XmlEvent::start_element("path")
                .attr("android:pathData", &data)
                .attr("android:fillColor", &fill)
                .attr("android:fillType", fill_type);

            if stroke_width > 0.0 {
                elem = elem
                    .attr("android:strokeWidth", &stroke_width_str)
                    .attr("android:strokeColor", &stroke_color);
            }

            xml.write(elem)
                .map_err(|e| Error::Other(anyhow::anyhow!(e)))?;
            xml.write(XmlEvent::end_element())
                .map_err(|e| Error::Other(anyhow::anyhow!(e)))?;
        }
        usvg::Node::Group(group) => {
            for child in group.children() {
                usvg_node_to_vector_drawable(child, xml)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn svg_path_to_string(path: &tiny_skia::Path) -> String {
    use tiny_skia::PathSegment;
    let mut s = String::new();
    for event in path.segments() {
        match event {
            PathSegment::MoveTo(p) => s += &format!("M {} {} ", p.x, p.y),
            PathSegment::LineTo(p) => s += &format!("L {} {} ", p.x, p.y),
            PathSegment::QuadTo(p1, p) => s += &format!("Q {} {} {} {} ", p1.x, p1.y, p.x, p.y),
            PathSegment::CubicTo(p1, p2, p) => {
                s += &format!("C {} {} {} {} {} {} ", p1.x, p1.y, p2.x, p2.y, p.x, p.y)
            }
            PathSegment::Close => s += "Z ",
        }
    }
    s.trim().to_string()
}

// Generate Android icons from a given icon path and output directory.
pub fn gen_android_icons(icon_path: &Path, out_dir: &Path) -> Result<()> {
    tracing::info!("Generating Android icons");
    // Currently we only use one icon as the foreground and background at the same time for SVG.
    let source = if icon_path.extension().and_then(|s| s.to_str()) == Some("svg") {
        let svg_data = std::fs::read(icon_path)?;
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_system_fonts();
        let mut opt = usvg::Options::default();
        opt.fontdb = std::sync::Arc::new(fontdb);
        let tree = usvg::Tree::from_data(&svg_data, &opt).context("Failed to parse SVG")?;
        Source::Svg(tree)
    } else {
        Source::DynamicImage(open(icon_path).context("Failed to open image")?)
    };

    if source.width() != source.height() {
        return Err(Error::Other(anyhow::anyhow!("Icon must be square")));
    }

    // For SVG generate Android VectorDrawable XML
    if let Source::Svg(tree) = &source {
        android_vector_drawable(tree, out_dir)?;
    }

    // For any icon file, generate PNG icons
    let entries = android_png_entries(out_dir)?;
    for entry in entries {
        resize_and_save_png(&source, entry.size, &entry.out_path, None)?;
    }
    Ok(())
}
