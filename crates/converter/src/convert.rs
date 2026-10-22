use anyhow::{Context, Result};
use image::{
    ColorType, DynamicImage, GenericImageView, ImageEncoder, Rgb, RgbImage,
    codecs::{
        avif::AvifEncoder,
        bmp::BmpEncoder,
        jpeg::JpegEncoder,
        png::{CompressionType, FilterType, PngEncoder},
        webp::WebPEncoder,
    },
};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use tempfile::Builder;
use tracing::info;

use crate::detect::detect_format_from_path;
use crate::eps::{decode_eps, encode_eps};
use crate::format::ImageFormat;
use crate::heif::{decode_heif, encode_heif};
use crate::options::ConversionOptions;

pub fn load_image(path: &Path, options: &ConversionOptions) -> Result<DynamicImage> {
    let detected_format = detect_format_from_path(path).unwrap_or(ImageFormat::Png);

    let mut img = match detected_format {
        ImageFormat::Eps => decode_eps(path, options.eps_raster_dpi)?,
        ImageFormat::Heic | ImageFormat::Heif => decode_heif(path)?,
        _ => image::open(path).with_context(|| format!("Failed to open image: {:?}", path))?,
    };

    if options.auto_rotate_exif {
        let orientation = File::open(path).ok().and_then(|file| {
            let mut bufreader = std::io::BufReader::new(file);
            exif::Reader::new()
                .read_from_container(&mut bufreader)
                .ok()
                .and_then(|exifreader| {
                    exifreader
                        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                        .and_then(|f| f.value.get_uint(0))
                })
        });
        if let Some(val) = orientation {
            img = apply_exif_orientation(img, val);
        }
    }

    if options.resize_enabled {
        img = apply_resize(img, options.max_width, options.max_height);
    }

    Ok(img)
}

fn apply_exif_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        1 => img, // Normal
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn apply_resize(img: DynamicImage, max_w: Option<u32>, max_h: Option<u32>) -> DynamicImage {
    let (w, h) = img.dimensions();
    let mw = max_w.unwrap_or(u32::MAX);
    let mh = max_h.unwrap_or(u32::MAX);

    if w > mw || h > mh {
        img.resize(mw, mh, image::imageops::FilterType::Lanczos3)
    } else {
        img
    }
}

pub fn flatten_alpha_to_rgb(img: &DynamicImage, bg_rgb: [u8; 3]) -> RgbImage {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb_img = RgbImage::new(width, height);

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let alpha = a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        let out_r = (r as f32 * alpha + bg_rgb[0] as f32 * inv_alpha).round() as u8;
        let out_g = (g as f32 * alpha + bg_rgb[1] as f32 * inv_alpha).round() as u8;
        let out_b = (b as f32 * alpha + bg_rgb[2] as f32 * inv_alpha).round() as u8;

        rgb_img.put_pixel(x, y, Rgb([out_r, out_g, out_b]));
    }

    rgb_img
}

pub fn convert_file(
    source_path: &Path,
    dest_path: &Path,
    target_format: ImageFormat,
    options: &ConversionOptions,
) -> Result<()> {
    let img = load_image(source_path, options)?;

    let dest_dir = dest_path.parent().unwrap_or_else(|| Path::new("."));
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir)
            .with_context(|| format!("Failed to create destination dir: {:?}", dest_dir))?;
    }

    // Create temp file in the SAME directory as dest_path to ensure atomic rename
    let prefix = format!(".imgwheel_tmp_{}", target_format.extension());
    let temp_file = Builder::new()
        .prefix(&prefix)
        .tempfile_in(dest_dir)
        .with_context(|| format!("Failed to create temp file in {:?}", dest_dir))?;

    let temp_path = temp_file.path().to_path_buf();

    match target_format {
        ImageFormat::Png => {
            let file = File::create(&temp_path)?;
            let writer = BufWriter::new(file);
            let encoder = PngEncoder::new_with_quality(
                writer,
                CompressionType::Default,
                FilterType::Adaptive,
            );
            let rgba = img.to_rgba8();
            encoder.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                ColorType::Rgba8.into(),
            )?;
        }
        ImageFormat::Jpg => {
            let rgb = flatten_alpha_to_rgb(&img, options.alpha_background_color);
            let file = File::create(&temp_path)?;
            let writer = BufWriter::new(file);
            let mut encoder = JpegEncoder::new_with_quality(writer, options.jpg_quality);
            encoder.encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                ColorType::Rgb8.into(),
            )?;
        }
        ImageFormat::Bmp => {
            let rgb = flatten_alpha_to_rgb(&img, options.alpha_background_color);
            let file = File::create(&temp_path)?;
            let mut writer = BufWriter::new(file);
            let mut encoder = BmpEncoder::new(&mut writer);
            encoder.encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                ColorType::Rgb8.into(),
            )?;
        }
        ImageFormat::Webp => {
            let file = File::create(&temp_path)?;
            let writer = BufWriter::new(file);
            let rgba = img.to_rgba8();
            let encoder = WebPEncoder::new_lossless(writer);
            // If quality < 100 and not lossless, we can encode with standard webp
            encoder.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                ColorType::Rgba8.into(),
            )?;
        }
        ImageFormat::Avif => {
            let file = File::create(&temp_path)?;
            let writer = BufWriter::new(file);
            let rgba = img.to_rgba8();
            let encoder = AvifEncoder::new_with_speed_quality(
                writer,
                options.avif_speed,
                options.avif_quality,
            );
            encoder.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                ColorType::Rgba8.into(),
            )?;
        }
        ImageFormat::Eps => {
            let rgb = flatten_alpha_to_rgb(&img, options.alpha_background_color);
            encode_eps(&rgb, &temp_path)?;
        }
        ImageFormat::Heic | ImageFormat::Heif => {
            let quality = if target_format == ImageFormat::Heic {
                options.heic_quality
            } else {
                options.heif_quality
            };
            encode_heif(&img, &temp_path, quality)?;
        }
    }

    // Atomically persist to final destination
    temp_file
        .persist(dest_path)
        .with_context(|| format!("Failed to move converted file to {:?}", dest_path))?;

    info!("Converted {:?} -> {:?}", source_path, dest_path);
    Ok(())
}
