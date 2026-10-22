use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use std::path::Path;

pub fn is_heif_available() -> bool {
    #[cfg(feature = "heif")]
    {
        libheif_rs::LibHeif::new_checked().is_ok()
    }
    #[cfg(not(feature = "heif"))]
    {
        false
    }
}

#[cfg(feature = "heif")]
pub fn decode_heif(path: &Path) -> Result<DynamicImage> {
    use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    let libheif = LibHeif::new();
    let ctx = HeifContext::read_from_file(path.to_str().context("Invalid path")?)
        .with_context(|| format!("Failed to read HEIF file: {:?}", path))?;
    let handle = ctx
        .primary_image_handle()
        .context("No primary image handle found in HEIF")?;
    let image = libheif
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
        .context("Failed to decode HEIF image data")?;

    let width = image.width();
    let height = image.height();
    let planes = image.planes();
    let interleaved = planes.interleaved.context("Missing interleaved plane")?;
    let data = interleaved.data;

    let mut rgba_vec = Vec::with_capacity((width * height * 4) as usize);
    let stride = interleaved.stride;
    for y in 0..height as usize {
        let row_start = y * stride;
        let row_end = row_start + (width as usize * 4);
        rgba_vec.extend_from_slice(&data[row_start..row_end]);
    }

    let img = RgbaImage::from_raw(width, height, rgba_vec)
        .context("Failed to create RgbaImage from HEIF decoded data")?;
    Ok(DynamicImage::ImageRgba8(img))
}

#[cfg(not(feature = "heif"))]
pub fn decode_heif(_path: &Path) -> Result<DynamicImage> {
    anyhow::bail!("HEIF/HEIC support is not compiled in or libheif is unavailable");
}

#[cfg(feature = "heif")]
pub fn encode_heif(img: &DynamicImage, output_path: &Path, quality: u8) -> Result<()> {
    use libheif_rs::{
        ColorSpace, CompressionFormat, EncoderQuality, HeifContext, Image, LibHeif, RgbChroma,
    };

    let libheif = LibHeif::new();
    let mut ctx = HeifContext::new().context("Failed to create HeifContext")?;
    let mut encoder = libheif
        .encoder_for_format(CompressionFormat::Hevc)
        .context("HEVC encoder not available in libheif")?;
    encoder
        .set_quality(EncoderQuality::Lossy(quality))
        .context("Failed to set encoder quality")?;

    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    let mut heif_img = Image::new(width, height, ColorSpace::Rgb(RgbChroma::Rgba))
        .context("Failed to create HEIF image buffer")?;
    heif_img
        .create_plane(libheif_rs::Channel::Interleaved, width, height, 8)
        .context("Failed to create HEIF plane")?;

    {
        let mut planes = heif_img.planes_mut();
        let interleaved = planes
            .interleaved
            .as_mut()
            .context("Missing interleaved plane")?;
        let stride = interleaved.stride;
        let data = &mut interleaved.data;
        let raw = rgba.as_raw();

        for y in 0..height as usize {
            let src_start = y * (width as usize * 4);
            let src_end = src_start + (width as usize * 4);
            let dst_start = y * stride;
            let dst_end = dst_start + (width as usize * 4);
            data[dst_start..dst_end].copy_from_slice(&raw[src_start..src_end]);
        }
    }

    ctx.encode_image(&heif_img, &mut encoder, None)
        .context("Failed to encode HEIF image")?;
    ctx.write_to_file(output_path.to_str().context("Invalid output path")?)
        .context("Failed to write HEIF file")?;

    Ok(())
}

#[cfg(not(feature = "heif"))]
pub fn encode_heif(_img: &DynamicImage, _output_path: &Path, _quality: u8) -> Result<()> {
    anyhow::bail!("HEIF/HEIC support is not compiled in or libheif is unavailable");
}
