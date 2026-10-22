use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Bmp,
    Png,
    Webp,
    Heic,
    Heif,
    Eps,
    Jpg,
    Avif,
}

impl ImageFormat {
    pub const ALL: [ImageFormat; 8] = [
        ImageFormat::Bmp,
        ImageFormat::Png,
        ImageFormat::Webp,
        ImageFormat::Heic,
        ImageFormat::Heif,
        ImageFormat::Eps,
        ImageFormat::Jpg,
        ImageFormat::Avif,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            ImageFormat::Bmp => "BMP",
            ImageFormat::Png => "PNG",
            ImageFormat::Webp => "WebP",
            ImageFormat::Heic => "HEIC",
            ImageFormat::Heif => "HEIF",
            ImageFormat::Eps => "EPS",
            ImageFormat::Jpg => "JPG",
            ImageFormat::Avif => "AVIF",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Bmp => "bmp",
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
            ImageFormat::Heic => "heic",
            ImageFormat::Heif => "heif",
            ImageFormat::Eps => "eps",
            ImageFormat::Jpg => "jpg",
            ImageFormat::Avif => "avif",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Png => "image/png",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Heic => "image/heic",
            ImageFormat::Heif => "image/heif",
            ImageFormat::Eps => "application/postscript",
            ImageFormat::Jpg => "image/jpeg",
            ImageFormat::Avif => "image/avif",
        }
    }

    pub fn supports_alpha(&self) -> bool {
        match self {
            ImageFormat::Png
            | ImageFormat::Webp
            | ImageFormat::Avif
            | ImageFormat::Heic
            | ImageFormat::Heif => true,
            ImageFormat::Bmp | ImageFormat::Jpg | ImageFormat::Eps => false,
        }
    }

    pub fn from_ext(ext: &str) -> Option<Self> {
        let clean = ext.trim().trim_start_matches('.').to_lowercase();
        match clean.as_str() {
            "bmp" => Some(ImageFormat::Bmp),
            "png" => Some(ImageFormat::Png),
            "webp" => Some(ImageFormat::Webp),
            "heic" => Some(ImageFormat::Heic),
            "heif" => Some(ImageFormat::Heif),
            "eps" => Some(ImageFormat::Eps),
            "jpg" | "jpeg" => Some(ImageFormat::Jpg),
            "avif" => Some(ImageFormat::Avif),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_ext)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatCapability {
    pub format: ImageFormat,
    pub available: bool,
    pub reason: Option<String>,
}
