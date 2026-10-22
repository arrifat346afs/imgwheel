#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub jpg_quality: u8,
    pub webp_quality: u8,
    pub webp_lossless: bool,
    pub png_compression: u8,
    pub avif_quality: u8,
    pub avif_speed: u8,
    pub heic_quality: u8,
    pub heif_quality: u8,
    pub bmp_bit_depth: u8,
    pub eps_raster_dpi: u32,
    pub strip_metadata: bool,
    pub auto_rotate_exif: bool,
    pub resize_enabled: bool,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub alpha_background_color: [u8; 3],
    pub overwrite: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            jpg_quality: 85,
            webp_quality: 85,
            webp_lossless: false,
            png_compression: 6,
            avif_quality: 80,
            avif_speed: 6,
            heic_quality: 80,
            heif_quality: 80,
            bmp_bit_depth: 24,
            eps_raster_dpi: 300,
            strip_metadata: false,
            auto_rotate_exif: true,
            resize_enabled: false,
            max_width: None,
            max_height: None,
            alpha_background_color: [255, 255, 255],
            overwrite: false,
        }
    }
}
