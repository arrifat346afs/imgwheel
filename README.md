# ImgWheel 🎡

[![CI](https://github.com/arrifat346afs/imgwheel/actions/workflows/ci.yml/badge.svg)](https://github.com/arrifat346afs/imgwheel/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/arrifat346afs/imgwheel)](https://github.com/arrifat346afs/imgwheel/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A fast, beautiful, radial image format converter for Linux, designed primarily for the **Hyprland** (Wayland) compositor.

Select one or more images, trigger **ImgWheel**, and an annular wheel menu appears directly centered at your mouse cursor. Click any format segment or press a number key to convert instantly.

```
       [ BMP ]
   [AVIF]   [PNG]
  [JPG]   ( )   [WebP]
   [EPS]    [HEIC]
       [HEIF]
```

---

## ✨ Features

- **Radial Donut UI**:
  - Annular sectors computed dynamically for any number of enabled formats (2 to 12).
  - Rotated labels following each segment's radial ray from center to rim.
  - Fully transparent, borderless Wayland window (only the wheel is rendered).
  - Smooth interactive hover animations and center hole info/cancel hints.
  - Keyboard navigation: `Esc` to cancel, arrow keys or `Tab` to cycle, `Enter` to confirm, `1`-`9` number keys to select.
- **Conversion Engine**:
  - Supports **PNG, JPG, BMP, WebP, AVIF, HEIC, HEIF, EPS**.
  - Detection by **magic bytes** (content inspection) with fallback to file extensions.
  - Safe collision avoidance: Never overwrites existing files unless configured (generates `photo (1).webp`, `photo (2).webp`).
  - Atomic writes: Uses temporary files in the destination directory to ensure half-written files are never left behind.
  - Alpha preservation: Composites transparent images onto a clean configurable background color when exporting to alpha-less formats (JPG, BMP, EPS).
  - Pure Rust raster EPS generation (PostScript Level 2 `colorimage`) + Ghostscript decoding.
  - Native HEIC/HEIF decode & encode via `libheif-rs` with runtime capability checks and graceful degradation.
  - EXIF orientation auto-rotation and optional EXIF stripping.
- **Batch Processing**:
  - Rayon multi-threaded pipeline executing non-blocking streaming conversions.
  - Sleek capsule (pill) floating progress window showing current count, progress bar, cancel button, and auto-close timer.
  - Output files are organized into a subfolder (`converted/` by default).
- **Hyprland & Desktop Integration**:
  - Positions itself precisely at the cursor location querying `hyprctl cursorpos -j`.
  - Lightweight background daemon (`imgwheel daemon`) listening on `$XDG_RUNTIME_DIR/imgwheel.sock` for instant sub-15ms invocations.
  - XDG Desktop Portal `GlobalShortcuts` integration via `ashpd`.
  - Desktop notifications (`notify-rust`).
  - "Open With" file associations and integration for **Nautilus**, **Dolphin**, **Thunar**, and **Nemo**.
- **Settings & Live Preview**:
  - Live interactive wheel preview that updates in real time.
  - Custom colors, typography, radii, gap widths, animation speed, and presets (Dark, Light, Nord).
  - Format ordering (reorder clockwise slices), labeling, and compression options.
  - Instant hot-reloading via configuration file watcher.

---

## 📦 System Dependencies

| Dependency | Package (Arch Linux) | Purpose |
|---|---|---|
| **libheif** | `libheif` | HEIC and HEIF decoding & encoding |
| **ghostscript** | `ghostscript` | EPS vector/document rasterization |
| **dav1d** | `dav1d` | High performance AVIF decoding |
| **wayland / libxkbcommon** | `wayland libxkbcommon` | Wayland client support |
| **wl-clipboard** *(optional)* | `wl-clipboard` | Clipboard file URI extraction in daemon |

---

## 🚀 Installation & Building

### Option 1: Arch Linux (Pre-built Package with `pacman`)

Download the `.pkg.tar.zst` package directly from [GitHub Releases](https://github.com/arrifat346afs/imgwheel/releases) and install via pacman:

```bash
# Example for v0.1.0:
sudo pacman -U imgwheel-0.1.0-1-x86_64.pkg.tar.zst

# Or install directly from the URL:
sudo pacman -U https://github.com/arrifat346afs/imgwheel/releases/download/v0.1.0/imgwheel-0.1.0-1-x86_64.pkg.tar.zst
```

### Option 2: Arch Linux (Build with PKGBUILD)

```bash
cd packaging
makepkg -si
```

### Option 3: Cargo (Manual Build from Source)

```bash
# Clone the repository
git clone https://github.com/arrifat346afs/imgwheel.git
cd imgwheel

# Build optimized release binary
cargo build --release --bin imgwheel

# Install binary to /usr/local/bin
sudo install -Dm755 target/release/imgwheel /usr/local/bin/imgwheel

# Install desktop entry and systemd service
install -Dm644 data/imgwheel.desktop ~/.local/share/applications/imgwheel.desktop
install -Dm644 data/imgwheel-daemon.service ~/.config/systemd/user/imgwheel-daemon.service
```

---

## 🪟 Hyprland Configuration

Add the following rules to your `~/.config/hypr/hyprland.conf`:

```ini
# Keybinding to launch ImgWheel
bind = SUPER, C, exec, imgwheel

# Window rules for transparent, borderless, pinned radial wheel
windowrulev2 = float, class:^(imgwheel)$
windowrulev2 = noborder, class:^(imgwheel)$
windowrulev2 = noshadow, class:^(imgwheel)$
windowrulev2 = noblur, class:^(imgwheel)$
windowrulev2 = noanim, class:^(imgwheel)$
windowrulev2 = pin, class:^(imgwheel)$
windowrulev2 = move cursor -50% -50%, class:^(imgwheel)$
windowrulev2 = stayfocused, class:^(imgwheel)$

# Window rules for batch capsule progress window
windowrulev2 = float, class:^(imgwheel-pill)$
windowrulev2 = noborder, class:^(imgwheel-pill)$
windowrulev2 = pin, class:^(imgwheel-pill)$
```

> 💡 **Tip**: You can print these rules anytime by running:
> ```bash
> imgwheel install-hyprland
> ```

---

## ⌨️ Command Line Interface (CLI)

```bash
# Open wheel for selected file(s)
imgwheel photo.png
imgwheel image1.png image2.jpg image3.webp

# Direct conversion (skips radial wheel)
imgwheel --to webp photo.png
imgwheel --to jpg --quality 90 photo.png
imgwheel --to png --out-dir ~/Pictures/exports *.jpg

# Force wheel position (x, y)
imgwheel --wheel-at 960,540 photo.png

# Read newline-separated paths from stdin
find . -maxdepth 1 -name "*.png" | imgwheel --stdin --to webp

# Open customization & settings window
imgwheel settings

# Run background daemon (registers XDG Global Shortcut & IPC socket)
imgwheel daemon
```

### Exit Codes:
- `0`: Success (all files converted or wheel cleanly closed)
- `1`: Partial failure (some files failed during batch)
- `2`: Total failure or invalid parameters

---

## 🔄 Daemon & Instant IPC

When `imgwheel daemon` is running:
1. It creates a Unix domain socket at `$XDG_RUNTIME_DIR/imgwheel.sock`.
2. Any subsequent CLI command (`imgwheel [FILES...]`) forwards file descriptors instantly over the socket and yields in under 15ms.
3. Registers a global shortcut via the **XDG Desktop Portal GlobalShortcuts** interface (`SUPER+C` by default).

To enable the daemon on login:
```bash
systemctl --user enable --now imgwheel-daemon.service
```

---

## 📁 File Manager Integration & Auto-Selection

ImgWheel features smart selection detection:
- **Instant Hotkey Conversion**: Simply highlight/select an image in your file manager (e.g. Nautilus, Dolphin, Thunar, Nemo) and press your ImgWheel shortcut (e.g. `ALT + T` or `SUPER + C`). ImgWheel automatically detects the focused file manager, retrieves the selected file(s), and pops up the radial wheel without asking you to re-pick the file!
- **Clipboard & URI Support**: Automatically converts file URIs (`text/uri-list`, `x-special/gnome-copied-files`, `file://`), copied paths, or even raw screenshot images copied to your clipboard.

### One-Click Installer:
Run:
```bash
imgwheel install-integrations
```
This automatically installs:
- **Nautilus Python Extension** (`~/.local/share/nautilus-python/extensions/imgwheel.py`): Adds right-click "Convert with ImgWheel" and caches active selections. (Reload Nautilus with `nautilus -q`).
- **Nautilus Script** (`~/.local/share/nautilus/scripts/Convert with ImgWheel`)
- **Dolphin Service Menus** (`~/.local/share/kservices5/ServiceMenus/` & `~/.local/share/kio/servicemenus/`)
- **Nemo Action** (`~/.local/share/nemo/actions/imgwheel.nemo_action`)
- **Desktop Launcher** (`~/.local/share/applications/imgwheel.desktop`)
- **Thunar**: Import the XML snippet in `data/file-manager/thunar/uca.xml` into `~/.config/Thunar/uca.xml`.

---

## ⚙️ Configuration Reference

Configuration is saved in `~/.config/imgwheel/config.toml` and hot-reloads automatically when modified:

```toml
[appearance]
segment_color = [77, 77, 77, 255]       # Dark gray (#4d4d4d)
hover_color = [59, 130, 246, 255]       # Accent blue
text_color = [255, 255, 255, 255]        # White
gap_color = [255, 255, 255, 255]         # White gap
outer_radius = 170.0                    # Outer radius in pixels
inner_radius = 48.0                     # Hole radius in pixels
gap_width = 3.0                         # Gap width between segments
font_size = 16.0
font_bold = true
animation_speed = 0.15                  # Smooth hover animation duration (s)

[appearance.pill]
width = 320.0
height = 48.0
position = "cursor"                     # "cursor", "top_center", "bottom_center"
auto_close_delay_secs = 2.0

[formats]
# Clockwise ordering from 12 o'clock
items = [
    { format = "bmp",  enabled = true, label = "BMP" },
    { format = "png",  enabled = true, label = "png" },
    { format = "webp", enabled = true, label = "WebP" },
    { format = "heic", enabled = true, label = "HEIC" },
    { format = "heif", enabled = true, label = "HEIF" },
    { format = "eps",  enabled = true, label = "EPS" },
    { format = "jpg",  enabled = true, label = "JPG" },
    { format = "avif", enabled = true, label = "AVIF" },
]

[formats.options]
jpg_quality = 85
webp_quality = 85
webp_lossless = false
png_compression = 6
avif_quality = 80
avif_speed = 6
heic_quality = 80
heif_quality = 80
eps_raster_dpi = 300
strip_metadata = false
auto_rotate_exif = true
alpha_background_color = [255, 255, 255]

[behavior]
batch_output_folder = "converted"
single_output_location = "beside_original"  # "beside_original" or "in_folder"
overwrite_policy = "never"                  # "never", "always"
close_on_focus_loss = true
close_after_convert = true
open_folder_after_batch = false
notifications = true
global_shortcut = "SUPER+C"
```

---

## ⚠️ Known Limitations

1. **Wayland Layer-Shell Protocol**: egui uses standard xdg-shell viewports rather than `wlr-layer-shell`. ImgWheel achieves borderless, transparent, popup positioning using Hyprland's `windowrulev2` directives (`float`, `pin`, `move cursor -50% -50%`).
2. **Ghostscript for EPS**: Reading and rasterizing incoming `.eps` vector files requires `gs` (Ghostscript) installed on the system. If `gs` is missing, EPS reading is disabled, though pure Rust EPS generation remains fully operational.
3. **HEIC / HEIF System Plugins**: `libheif` requires the `hevc` encoder plugin (e.g. `x265`) installed on your Linux distribution to encode HEIC/HEIF files. If missing, the HEIC and HEIF segments are safely greyed out with an explanatory tooltip.

---

## 📜 License

MIT License. See [LICENSE](LICENSE) for details.
