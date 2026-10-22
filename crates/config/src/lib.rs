#![allow(clippy::collapsible_if)]

use anyhow::{Context, Result};
use directories::ProjectDirs;
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;
use tracing::{error, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Bmp,
    Png,
    Webp,
    Heic,
    Heif,
    Eps,
    Jpg,
    Avif,
}

impl Format {
    pub const ALL: [Format; 8] = [
        Format::Bmp,
        Format::Png,
        Format::Webp,
        Format::Heic,
        Format::Heif,
        Format::Eps,
        Format::Jpg,
        Format::Avif,
    ];

    pub fn default_label(&self) -> &'static str {
        match self {
            Format::Bmp => "BMP",
            Format::Png => "PNG",
            Format::Webp => "WebP",
            Format::Heic => "HEIC",
            Format::Heif => "HEIF",
            Format::Eps => "EPS",
            Format::Jpg => "JPG",
            Format::Avif => "AVIF",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Format::Bmp => "bmp",
            Format::Png => "png",
            Format::Webp => "webp",
            Format::Heic => "heic",
            Format::Heif => "heif",
            Format::Eps => "eps",
            Format::Jpg => "jpg",
            Format::Avif => "avif",
        }
    }

    pub fn from_ext_or_name(s: &str) -> Option<Self> {
        let clean = s.trim().trim_start_matches('.').to_lowercase();
        match clean.as_str() {
            "bmp" => Some(Format::Bmp),
            "png" => Some(Format::Png),
            "webp" => Some(Format::Webp),
            "heic" => Some(Format::Heic),
            "heif" => Some(Format::Heif),
            "eps" => Some(Format::Eps),
            "jpg" | "jpeg" => Some(Format::Jpg),
            "avif" => Some(Format::Avif),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverwritePolicy {
    #[default]
    Never,
    Ask,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SingleOutputLocation {
    #[default]
    BesideOriginal,
    InFolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PillPosition {
    #[default]
    Cursor,
    TopCenter,
    BottomCenter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub segment_color: [u8; 4],
    pub hover_color: [u8; 4],
    pub text_color: [u8; 4],
    pub gap_color: [u8; 4],
    pub font_family: Option<String>,
    pub font_size: f32,
    pub font_bold: bool,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub gap_width: f32,
    pub corner_radius: f32,
    pub wheel_opacity: f32,
    #[serde(default = "default_window_opacity")]
    pub window_opacity: f32,
    #[serde(default = "default_widget_opacity")]
    pub widget_opacity: f32,
    pub background_dim: bool,
    pub drop_shadow: bool,
    pub animation_speed: f32,
    pub pill: PillConfig,
}

fn default_window_opacity() -> f32 {
    0.96
}

fn default_widget_opacity() -> f32 {
    0.85
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            // #4d4d4d with full alpha
            segment_color: [77, 77, 77, 255],
            hover_color: [59, 130, 246, 255], // crisp blue accent
            text_color: [255, 255, 255, 255],
            gap_color: [255, 255, 255, 255],
            font_family: None,
            font_size: 17.0,
            font_bold: true,
            outer_radius: 170.0,
            inner_radius: 48.0,
            gap_width: 3.0,
            corner_radius: 2.0,
            wheel_opacity: 1.0,
            window_opacity: 0.96,
            widget_opacity: 0.85,
            background_dim: false,
            drop_shadow: false,
            animation_speed: 0.15,
            pill: PillConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PillConfig {
    pub width: f32,
    pub height: f32,
    pub bg_color: [u8; 4],
    pub text_color: [u8; 4],
    pub accent_color: [u8; 4],
    pub position: PillPosition,
    pub auto_close_delay_secs: f32,
}

impl Default for PillConfig {
    fn default() -> Self {
        Self {
            width: 320.0,
            height: 48.0,
            bg_color: [40, 40, 40, 240],
            text_color: [255, 255, 255, 255],
            accent_color: [59, 130, 246, 255],
            position: PillPosition::Cursor,
            auto_close_delay_secs: 2.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatItemConfig {
    pub format: Format,
    pub enabled: bool,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatOptionsConfig {
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
}

impl Default for FormatOptionsConfig {
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatsConfig {
    pub items: Vec<FormatItemConfig>,
    pub options: FormatOptionsConfig,
}

impl Default for FormatsConfig {
    fn default() -> Self {
        // Order clockwise from the top: BMP, PNG, WebP, HEIC, HEIF, EPS, JPG, AVIF
        let default_items = vec![
            FormatItemConfig {
                format: Format::Bmp,
                enabled: true,
                label: "BMP".to_string(),
            },
            FormatItemConfig {
                format: Format::Png,
                enabled: true,
                label: "png".to_string(),
            },
            FormatItemConfig {
                format: Format::Webp,
                enabled: true,
                label: "WebP".to_string(),
            },
            FormatItemConfig {
                format: Format::Heic,
                enabled: true,
                label: "HEIC".to_string(),
            },
            FormatItemConfig {
                format: Format::Heif,
                enabled: true,
                label: "HEIF".to_string(),
            },
            FormatItemConfig {
                format: Format::Eps,
                enabled: true,
                label: "EPS".to_string(),
            },
            FormatItemConfig {
                format: Format::Jpg,
                enabled: true,
                label: "JPG".to_string(),
            },
            FormatItemConfig {
                format: Format::Avif,
                enabled: true,
                label: "AVIF".to_string(),
            },
        ];

        Self {
            items: default_items,
            options: FormatOptionsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub batch_output_folder: String,
    pub single_output_location: SingleOutputLocation,
    pub overwrite_policy: OverwritePolicy,
    pub close_on_focus_loss: bool,
    pub close_after_convert: bool,
    pub open_folder_after_batch: bool,
    pub notifications: bool,
    pub global_shortcut: String,
    pub autostart: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            batch_output_folder: "converted".to_string(),
            single_output_location: SingleOutputLocation::BesideOriginal,
            overwrite_policy: OverwritePolicy::Never,
            close_on_focus_loss: true,
            close_after_convert: true,
            open_folder_after_batch: false,
            notifications: true,
            global_shortcut: "SUPER+C".to_string(),
            autostart: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Config {
    pub appearance: AppearanceConfig,
    pub formats: FormatsConfig,
    pub behavior: BehaviorConfig,
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "imgwheel") {
            proj_dirs.config_dir().join("config.toml")
        } else {
            PathBuf::from(".config/imgwheel/config.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Self {
        if !path.exists() {
            let default_cfg = Self::default();
            if let Err(e) = default_cfg.save_to_path(path) {
                warn!("Could not create default config at {:?}: {}", path, e);
            }
            return default_cfg;
        }

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(mut cfg) => {
                    cfg.sanitize();
                    cfg
                }
                Err(err) => {
                    error!(
                        "Failed to parse config at {:?}: {}. Falling back to default settings.",
                        path, err
                    );
                    Self::default()
                }
            },
            Err(err) => {
                error!(
                    "Failed to read config file at {:?}: {}. Falling back to default settings.",
                    path, err
                );
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {:?}", parent))?;
        }
        let serialized =
            toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;
        fs::write(path, serialized)
            .with_context(|| format!("Failed to write config file to {:?}", path))?;
        Ok(())
    }

    pub fn sanitize(&mut self) {
        // Ensure at least 2 formats are enabled
        let enabled_count = self.formats.items.iter().filter(|i| i.enabled).count();
        if enabled_count < 2 {
            for item in &mut self.formats.items {
                item.enabled = true;
            }
        }
        // Radii bounds
        if self.appearance.outer_radius < 50.0 {
            self.appearance.outer_radius = 50.0;
        }
        if self.appearance.inner_radius < 10.0 {
            self.appearance.inner_radius = 10.0;
        }
        if self.appearance.inner_radius >= self.appearance.outer_radius - 10.0 {
            self.appearance.inner_radius = self.appearance.outer_radius * 0.3;
        }
        if self.appearance.window_opacity <= 0.05 {
            self.appearance.window_opacity = 0.96;
        } else {
            self.appearance.window_opacity = self.appearance.window_opacity.clamp(0.2, 1.0);
        }
        if self.appearance.widget_opacity <= 0.05 {
            self.appearance.widget_opacity = 0.85;
        } else {
            self.appearance.widget_opacity = self.appearance.widget_opacity.clamp(0.1, 1.0);
        }
    }
}

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Config>,
}

impl ConfigWatcher {
    pub fn watch(path: PathBuf) -> Result<Self> {
        let (tx, rx) = channel();
        let watch_path = path.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    if event.kind.is_modify() || event.kind.is_create() {
                        let new_cfg = Config::load_from_path(&watch_path);
                        let _ = tx.send(new_cfg);
                    }
                }
                Err(e) => {
                    warn!("Config file watcher error: {:?}", e);
                }
            },
            NotifyConfig::default().with_poll_interval(Duration::from_millis(500)),
        )?;

        if let Some(parent) = path.parent().filter(|p| p.exists()) {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn try_recv(&self) -> Option<Config> {
        let mut latest = None;
        while let Ok(cfg) = self.rx.try_recv() {
            latest = Some(cfg);
        }
        latest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config_validity() {
        let cfg = Config::default();
        assert_eq!(cfg.formats.items.len(), 8);
        assert!(cfg.appearance.outer_radius > cfg.appearance.inner_radius);
    }

    #[test]
    fn test_save_and_load_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut cfg = Config::default();
        cfg.appearance.font_size = 22.0;
        cfg.behavior.batch_output_folder = "custom_out".into();

        cfg.save_to_path(&path).unwrap();
        assert!(path.exists());

        let loaded = Config::load_from_path(&path);
        assert_eq!(loaded.appearance.font_size, 22.0);
        assert_eq!(loaded.behavior.batch_output_folder, "custom_out");
    }

    #[test]
    fn test_corrupt_config_fallback() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("corrupt.toml");

        fs::write(&path, "this is definitely not valid toml = [[[").unwrap();

        let loaded = Config::load_from_path(&path);
        let default_cfg = Config::default();
        assert_eq!(
            loaded.appearance.outer_radius,
            default_cfg.appearance.outer_radius
        );
        assert_eq!(
            loaded.behavior.batch_output_folder,
            default_cfg.behavior.batch_output_folder
        );
    }
}
