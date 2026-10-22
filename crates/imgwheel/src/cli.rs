use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "imgwheel",
    author,
    version,
    about = "A fast radial image format converter for Linux (Hyprland / Wayland / X11)"
)]
pub struct Cli {
    /// Input image files to convert
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,

    /// Skip the wheel and convert directly to target format (e.g. webp, jpg, png)
    #[arg(short = 't', long = "to")]
    pub to: Option<String>,

    /// Quality setting (1-100) for lossy formats (JPG, WebP, AVIF, HEIC)
    #[arg(short = 'q', long = "quality")]
    pub quality: Option<u8>,

    /// Custom output directory for converted images
    #[arg(short = 'o', long = "out-dir")]
    pub out_dir: Option<PathBuf>,

    /// Force wheel position at x,y coordinates
    #[arg(long = "wheel-at", value_name = "X,Y")]
    pub wheel_at: Option<String>,

    /// Read newline-separated file paths from standard input
    #[arg(long = "stdin")]
    pub stdin: bool,

    /// Preview the wheel with a sample demo image
    #[arg(long = "demo")]
    pub demo: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Open the settings and customization window
    Settings,
    /// Run as background daemon with global shortcut and IPC socket
    Daemon,
    /// Print recommended Hyprland keybind and window rules
    InstallHyprland,
    /// Install Nautilus, Dolphin, Thunar, and Nemo file manager integrations
    InstallIntegrations,
}
