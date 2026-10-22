use crate::format::ImageFormat;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn detect_format_from_bytes(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return Some(ImageFormat::Png);
    }

    if bytes.len() >= 3 && &bytes[0..3] == b"\xFF\xD8\xFF" {
        return Some(ImageFormat::Jpg);
    }

    if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        return Some(ImageFormat::Bmp);
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::Webp);
    }

    // EPS header: DSC "%!PS-Adobe" or DOS EPS binary header (\xC5\xD0\xD3\xC6)
    if bytes.starts_with(b"%!PS-Adobe") || bytes.starts_with(b"\xC5\xD0\xD3\xC6") {
        return Some(ImageFormat::Eps);
    }

    // ISO BMFF formats (AVIF, HEIC, HEIF): check for ftyp box
    // Format: 4-byte box size, 4-byte box type b"ftyp", followed by major_brand
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return Some(ImageFormat::Avif);
        }
        if brand == b"heic" || brand == b"heix" || brand == b"hevc" || brand == b"hevx" {
            return Some(ImageFormat::Heic);
        }
        if brand == b"mif1" || brand == b"msf1" {
            return Some(ImageFormat::Heif);
        }

        // Also inspect compatible brands in the rest of ftyp box
        let box_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let limit = bytes.len().min(box_len);
        if limit >= 16 {
            let mut i = 16;
            while i + 4 <= limit {
                let comp = &bytes[i..i + 4];
                if comp == b"avif" || comp == b"avis" {
                    return Some(ImageFormat::Avif);
                }
                if comp == b"heic" || comp == b"heix" || comp == b"hevc" {
                    return Some(ImageFormat::Heic);
                }
                if comp == b"mif1" || comp == b"msf1" {
                    return Some(ImageFormat::Heif);
                }
                i += 4;
            }
        }
    }

    None
}

pub fn detect_format_from_path(path: &Path) -> Result<ImageFormat> {
    if path.exists() {
        let mut file =
            File::open(path).with_context(|| format!("Failed to open image file: {:?}", path))?;
        let mut buf = [0u8; 64];
        let bytes_read = file.read(&mut buf).unwrap_or(0);
        if let Some(fmt) = detect_format_from_bytes(&buf[..bytes_read]) {
            return Ok(fmt);
        }
    }

    // Fallback to extension
    if let Some(fmt) = ImageFormat::from_path(path) {
        return Ok(fmt);
    }

    anyhow::bail!("Unable to detect image format for file: {:?}", path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_bytes_detection() {
        assert_eq!(
            detect_format_from_bytes(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            detect_format_from_bytes(b"\xFF\xD8\xFF\xE0\0\x10JFIF"),
            Some(ImageFormat::Jpg)
        );
        assert_eq!(
            detect_format_from_bytes(b"BM\x36\0\0\0\0\0"),
            Some(ImageFormat::Bmp)
        );
        assert_eq!(
            detect_format_from_bytes(b"RIFF\x24\0\0\0WEBPVP8 "),
            Some(ImageFormat::Webp)
        );
        assert_eq!(
            detect_format_from_bytes(b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox"),
            Some(ImageFormat::Eps)
        );

        let mut avif_header = vec![0, 0, 0, 28, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f'];
        avif_header.extend_from_slice(&[0; 16]);
        assert_eq!(
            detect_format_from_bytes(&avif_header),
            Some(ImageFormat::Avif)
        );
    }
}
