#![allow(clippy::collapsible_if)]

pub mod convert;
pub mod detect;
pub mod eps;
pub mod format;
pub mod heif;
pub mod options;
pub mod path;

pub use convert::{convert_file, flatten_alpha_to_rgb, load_image};
pub use detect::{detect_format_from_bytes, detect_format_from_path};
pub use eps::is_ghostscript_available;
pub use format::{FormatCapability, ImageFormat};
pub use heif::is_heif_available;
pub use options::ConversionOptions;
pub use path::resolve_output_path;

pub fn available_formats() -> Vec<FormatCapability> {
    let gs_ok = is_ghostscript_available();
    let heif_ok = is_heif_available();

    vec![
        FormatCapability {
            format: ImageFormat::Bmp,
            available: true,
            reason: None,
        },
        FormatCapability {
            format: ImageFormat::Png,
            available: true,
            reason: None,
        },
        FormatCapability {
            format: ImageFormat::Webp,
            available: true,
            reason: None,
        },
        FormatCapability {
            format: ImageFormat::Heic,
            available: heif_ok,
            reason: if heif_ok {
                None
            } else {
                Some("Install libheif for HEIC support".to_string())
            },
        },
        FormatCapability {
            format: ImageFormat::Heif,
            available: heif_ok,
            reason: if heif_ok {
                None
            } else {
                Some("Install libheif for HEIF support".to_string())
            },
        },
        FormatCapability {
            format: ImageFormat::Eps,
            available: true, // Writing EPS is pure Rust; reading needs gs
            reason: if gs_ok {
                None
            } else {
                Some("Install Ghostscript (gs) to read EPS".to_string())
            },
        },
        FormatCapability {
            format: ImageFormat::Jpg,
            available: true,
            reason: None,
        },
        FormatCapability {
            format: ImageFormat::Avif,
            available: true,
            reason: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    fn create_test_image() -> RgbaImage {
        let mut img = RgbaImage::new(16, 16);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([(x * 15) as u8, (y * 15) as u8, 128, 255]);
        }
        img
    }

    #[test]
    fn test_available_formats() {
        let caps = available_formats();
        assert_eq!(caps.len(), 8);
        assert!(
            caps.iter()
                .any(|c| c.format == ImageFormat::Png && c.available)
        );
        assert!(
            caps.iter()
                .any(|c| c.format == ImageFormat::Webp && c.available)
        );
    }

    #[test]
    fn test_core_conversions_roundtrip() {
        let dir = tempdir().unwrap();
        let src_png = dir.path().join("source.png");
        let initial_img = create_test_image();
        initial_img.save(&src_png).unwrap();

        let options = ConversionOptions::default();

        // 1. PNG -> JPG
        let dest_jpg = dir.path().join("out.jpg");
        convert_file(&src_png, &dest_jpg, ImageFormat::Jpg, &options).unwrap();
        assert!(dest_jpg.exists());
        let read_jpg = load_image(&dest_jpg, &options).unwrap();
        assert_eq!(read_jpg.width(), 16);
        assert_eq!(read_jpg.height(), 16);

        // 2. JPG -> BMP
        let dest_bmp = dir.path().join("out.bmp");
        convert_file(&dest_jpg, &dest_bmp, ImageFormat::Bmp, &options).unwrap();
        assert!(dest_bmp.exists());
        let read_bmp = load_image(&dest_bmp, &options).unwrap();
        assert_eq!(read_bmp.width(), 16);
        assert_eq!(read_bmp.height(), 16);

        // 3. BMP -> WebP
        let dest_webp = dir.path().join("out.webp");
        convert_file(&dest_bmp, &dest_webp, ImageFormat::Webp, &options).unwrap();
        assert!(dest_webp.exists());
        let read_webp = load_image(&dest_webp, &options).unwrap();
        assert_eq!(read_webp.width(), 16);
        assert_eq!(read_webp.height(), 16);

        // 4. WebP -> AVIF
        let dest_avif = dir.path().join("out.avif");
        convert_file(&dest_webp, &dest_avif, ImageFormat::Avif, &options).unwrap();
        assert!(dest_avif.exists());
        let read_avif = load_image(&dest_avif, &options).unwrap();
        assert_eq!(read_avif.width(), 16);
        assert_eq!(read_avif.height(), 16);

        // 5. AVIF -> PNG
        let dest_final_png = dir.path().join("out_final.png");
        convert_file(&dest_avif, &dest_final_png, ImageFormat::Png, &options).unwrap();
        assert!(dest_final_png.exists());
        let read_final = load_image(&dest_final_png, &options).unwrap();
        assert_eq!(read_final.width(), 16);
        assert_eq!(read_final.height(), 16);

        // 6. Test EPS encoding & Ghostscript decoding (since gs is installed on system)
        let dest_eps = dir.path().join("out.eps");
        convert_file(&src_png, &dest_eps, ImageFormat::Eps, &options).unwrap();
        assert!(dest_eps.exists());
        if is_ghostscript_available() {
            let read_eps = load_image(&dest_eps, &options).unwrap();
            assert!(read_eps.width() > 0);
        }
    }
}
