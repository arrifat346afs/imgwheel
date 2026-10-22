# Changelog

All notable changes to **ImgWheel** will be documented in this file.

## [0.1.0] - 2026-10-22

### Added
- **Radial Wheel UI**: Transparent, borderless circular donut widget with dynamic slice tessellation (supports 2 to 12 formats), rotated radial labels, smooth hover animations, and center hole hints.
- **Conversion Engine**:
  - Pure Rust & native conversion support for PNG, JPG, BMP, WebP, AVIF, HEIC, HEIF, and EPS.
  - Ghostscript integration for rasterizing incoming EPS documents.
  - Native PostScript Level 2 raster generator for pure Rust EPS encoding.
  - `libheif-rs` integration for HEIC and HEIF encode and decode with runtime capability detection and graceful degradation.
  - `dav1d` AVIF decoder and high-speed AVIF encoder.
  - Magic bytes detection with extension fallback.
  - Collision-safe output path resolution (`photo.png` -> `photo.webp`, `photo (1).webp`).
  - Atomic writing via directory-local temporary files.
  - Alpha preservation with configurable background blending for alpha-less formats (JPG, BMP, EPS).
  - EXIF metadata handling (auto-rotate by EXIF orientation tag, strip metadata toggle).
  - Optional downscaling/resizing by max width/height preserving aspect ratio.
- **Batch Processing**:
  - Rayon multi-threaded worker pipeline streaming real-time progress.
  - Pill/capsule floating progress window with progress bar, cancellation toggle, and auto-close timer.
  - Automated output placement in subdirectories (`converted/`).
- **Hyprland & Wayland Optimization**:
  - `hyprctl cursorpos -j` query support for cursor-centric placement.
  - Dedicated windowrules configuration (`imgwheel install-hyprland`).
  - Full transparency support with Wayland `app_id` and X11 fallback.
- **Settings & Customization UI**:
  - Dedicated multi-tab settings window (`imgwheel settings`) with live interactive wheel preview.
  - Color pickers for segments, hover accents, text, gaps.
  - Geometry sliders (outer radius, inner hole radius, gap width, animation duration).
  - Drag-and-drop format reordering, renaming, and per-format encoders tuning.
  - Hot-reloading configuration watcher via `~/.config/imgwheel/config.toml`.
- **Daemon & Fast IPC**:
  - Background daemon (`imgwheel daemon`) listening on `$XDG_RUNTIME_DIR/imgwheel.sock`.
  - Sub-15ms startup via instant IPC forwarding for subsequent CLI invocations.
  - XDG Desktop Portal `GlobalShortcuts` integration via `ashpd`.
  - Clipboard detection (`text/uri-list`) and fallback native file picker (`rfd`).
- **Packaging & Desktop Integration**:
  - Systemd user service (`imgwheel-daemon.service`).
  - Desktop entry file with image MIME associations (`imgwheel.desktop`).
  - File manager integration scripts and service menus (Nautilus, Dolphin, Thunar, Nemo).
  - Arch Linux `PKGBUILD`.
