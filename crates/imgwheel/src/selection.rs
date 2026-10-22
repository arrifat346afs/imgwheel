use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

/// Percent-decodes a URL-encoded string into a standard UTF-8 string.
pub fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(h1), Some(h2)) = (h1, h2) {
                let hex_str = [h1, h2];
                if let Ok(s) = std::str::from_utf8(&hex_str) {
                    if let Ok(val) = u8::from_str_radix(s, 16) {
                        bytes.push(val);
                        continue;
                    }
                }
                // Invalid hex sequence, preserve original characters
                bytes.push(b'%');
                bytes.push(h1);
                bytes.push(h2);
                continue;
            } else {
                bytes.push(b'%');
                if let Some(h1) = h1 {
                    bytes.push(h1);
                }
                continue;
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Sanitizes a raw path string (strips file://, unquotes, percent-decodes, expands ~)
/// and checks if it points to an existing file.
pub fn sanitize_and_resolve_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip surrounding quotes if present
    let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    // Strip URI schemes
    let stripped = if let Some(rest) = unquoted.strip_prefix("file://localhost/") {
        format!("/{}", rest)
    } else if let Some(rest) = unquoted.strip_prefix("file:///") {
        format!("/{}", rest)
    } else if let Some(rest) = unquoted.strip_prefix("file://") {
        rest.to_string()
    } else {
        unquoted.to_string()
    };

    let decoded = percent_decode(&stripped);

    // Expand ~
    let resolved_str = if decoded.starts_with("~/") || decoded == "~" {
        if let Ok(home) = std::env::var("HOME") {
            decoded.replacen('~', &home, 1)
        } else {
            decoded
        }
    } else {
        decoded
    };

    let path = PathBuf::from(&resolved_str);
    if path.exists() && path.is_file() {
        Some(path.canonicalize().unwrap_or(path))
    } else {
        // Also check if relative to current working directory
        if let Ok(cwd) = std::env::current_dir() {
            let abs = cwd.join(&resolved_str);
            if abs.exists() && abs.is_file() {
                return Some(abs.canonicalize().unwrap_or(abs));
            }
        }
        None
    }
}

/// Checks whether a given path corresponds to a supported image file.
pub fn is_supported_image(path: &Path) -> bool {
    if !path.exists() || !path.is_file() {
        return false;
    }
    if converter::detect_format_from_path(path).is_ok() {
        return true;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        matches!(
            ext_lower.as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "bmp"
                | "webp"
                | "avif"
                | "heic"
                | "heif"
                | "eps"
                | "gif"
                | "tiff"
                | "tif"
                | "ico"
                | "svg"
                | "qoi"
                | "tga"
                | "pnm"
                | "ppm"
                | "hdr"
                | "exr"
        )
    } else {
        false
    }
}

/// Path to the active selection cache file.
pub fn get_selection_file_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("imgwheel_selection.txt")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/imgwheel_selection_{}.txt", uid))
    }
}

/// Reads the selection cache written by the file manager integration (e.g. Nautilus extension).
pub fn read_selection_file() -> Vec<PathBuf> {
    let path = get_selection_file_path();
    if !path.exists() {
        return Vec::new();
    }

    // Accept selection cache only if modified within the last 30 seconds
    if let Ok(meta) = fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed > std::time::Duration::from_secs(30) {
                    debug!("Selection cache file is older than 30s, ignoring");
                    return Vec::new();
                }
            }
        }
    }

    if let Ok(content) = fs::read_to_string(&path) {
        let mut results = Vec::new();
        for line in content.lines() {
            if let Some(p) = sanitize_and_resolve_path(line) {
                if is_supported_image(&p) && !results.contains(&p) {
                    results.push(p);
                }
            }
        }
        return results;
    }
    Vec::new()
}

/// Writes selected paths to the selection cache file.
pub fn write_selection_file(paths: &[PathBuf]) -> Result<()> {
    let path = get_selection_file_path();
    let content: String = paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content)?;
    Ok(())
}

/// Queries the focused window's class name via Hyprland or X11.
pub fn get_active_window_class() -> Option<String> {
    // 1. Wayland / Hyprland
    if let Ok(output) = Command::new("hyprctl").args(["activewindow", "-j"]).output() {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(class) = val.get("class").and_then(|v| v.as_str()) {
                    return Some(class.to_string());
                }
            }
        }
    }

    // 2. X11 fallback
    if let Ok(output) = Command::new("xdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
    {
        if output.status.success() {
            let cls = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !cls.is_empty() {
                return Some(cls);
            }
        }
    }

    None
}

/// Checks if the focused window is a common Linux file manager.
pub fn is_file_manager_focused() -> bool {
    if let Some(class) = get_active_window_class() {
        let lower = class.to_lowercase();
        lower.contains("nautilus")
            || lower.contains("dolphin")
            || lower.contains("thunar")
            || lower.contains("nemo")
            || lower.contains("pcmanfm")
            || lower.contains("caja")
            || lower.contains("files")
            || lower.contains("krusader")
            || lower.contains("doublecmd")
            || lower.contains("spacefm")
            || lower.contains("ranger")
            || lower.contains("yazi")
    } else {
        false
    }
}

/// Triggers a Ctrl+C copy keystroke in the currently focused window so the file manager
/// populates the clipboard with the currently selected item(s).
pub fn trigger_copy_in_focused_window() {
    let mut triggered = false;

    // 1. Try wtype (Wayland virtual keyboard)
    // Release modifiers first in case the user's hotkey (e.g. ALT+T or SUPER+C) is still being pressed
    if let Ok(mut child) = Command::new("wtype")
        .args([
            "-m", "alt", "-m", "win", "-m", "shift", "-M", "ctrl", "-k", "c", "-m", "ctrl",
        ])
        .spawn()
    {
        if let Ok(status) = child.wait() {
            if status.success() {
                triggered = true;
            }
        }
    }

    // 2. Try ydotool if wtype failed or wasn't found
    if !triggered {
        if let Ok(mut child) = Command::new("ydotool")
            .args(["key", "29:1", "46:1", "46:0", "29:0"])
            .spawn()
        {
            if let Ok(status) = child.wait() {
                if status.success() {
                    triggered = true;
                }
            }
        }
    }

    // 3. Try xdotool if on X11
    if !triggered {
        if let Ok(mut child) = Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+c"])
            .spawn()
        {
            if let Ok(status) = child.wait() {
                if status.success() {
                    triggered = true;
                }
            }
        }
    }

    if triggered {
        // Allow the file manager time to handle the event and write to the Wayland clipboard
        std::thread::sleep(std::time::Duration::from_millis(85));
    }
}

/// Reads image file paths from the Wayland or X11 clipboard.
pub fn get_files_from_clipboard() -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Check Wayland clipboard formats
    let mime_types = [
        "text/uri-list",
        "x-special/gnome-copied-files",
        "text/plain",
        "text/plain;charset=utf-8",
    ];

    for mime in mime_types {
        if let Ok(output) = Command::new("wl-paste").args(["-t", mime]).output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                parse_uri_or_path_list(&s, &mut files);
                if !files.is_empty() {
                    return files;
                }
            }
        }
    }

    // Fallback: raw wl-paste
    if let Ok(output) = Command::new("wl-paste").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            parse_uri_or_path_list(&s, &mut files);
            if !files.is_empty() {
                return files;
            }
        }
    }

    // Fallback: wl-paste primary selection
    if let Ok(output) = Command::new("wl-paste").arg("-p").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            parse_uri_or_path_list(&s, &mut files);
            if !files.is_empty() {
                return files;
            }
        }
    }

    // Fallback: X11 xclip
    for target in &["text/uri-list", "UTF8_STRING", "STRING"] {
        if let Ok(output) = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", target, "-o"])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                parse_uri_or_path_list(&s, &mut files);
                if !files.is_empty() {
                    return files;
                }
            }
        }
    }

    files
}

/// Parses lines of URIs or file paths and appends valid images to out.
pub fn parse_uri_or_path_list(raw: &str, out: &mut Vec<PathBuf>) {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed == "copy"
            || trimmed == "cut"
        {
            continue;
        }

        if let Some(path) = sanitize_and_resolve_path(trimmed) {
            if is_supported_image(&path) && !out.contains(&path) {
                out.push(path);
            }
        }
    }
}

/// Checks if raw image bytes are on the clipboard and saves them to a temp PNG.
pub fn get_image_from_clipboard() -> Option<PathBuf> {
    if let Ok(output) = Command::new("wl-paste").args(["-t", "image/png"]).output() {
        if output.status.success() && !output.stdout.is_empty() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let tmp_path =
                std::env::temp_dir().join(format!("imgwheel_clipboard_{}.png", timestamp));
            if fs::write(&tmp_path, &output.stdout).is_ok() {
                return Some(tmp_path);
            }
        }
    }

    if let Ok(output) = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "image/png", "-o"])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let tmp_path =
                std::env::temp_dir().join(format!("imgwheel_clipboard_{}.png", timestamp));
            if fs::write(&tmp_path, &output.stdout).is_ok() {
                return Some(tmp_path);
            }
        }
    }

    None
}

/// Resolves input files across all sources in hierarchical priority:
/// 1. Demo mode (generates demo image)
/// 2. CLI arguments (sanitized)
/// 3. Stdin (sanitized)
/// 4. File manager selection cache file (if recent)
/// 5. Focused file manager window (auto Ctrl+C + clipboard read)
/// 6. Clipboard file URIs
/// 7. Clipboard raw image bytes
/// 8. Fallback native file picker
pub fn resolve_input_files(
    cli_files: &[PathBuf],
    stdin: bool,
    demo: bool,
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // 1. Demo
    if demo {
        let demo_path = std::env::temp_dir().join("imgwheel_demo.png");
        let mut img = image::RgbaImage::new(128, 128);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x * 2) as u8, (y * 2) as u8, 180, 255]);
        }
        let _ = img.save(&demo_path);
        return vec![demo_path];
    }

    // 2. Direct CLI files
    for f in cli_files {
        if let Some(clean) = sanitize_and_resolve_path(&f.to_string_lossy()) {
            if !files.contains(&clean) {
                files.push(clean);
            }
        }
    }

    // 3. Stdin
    if stdin {
        let stdin_handle = std::io::stdin();
        let reader = BufReader::new(stdin_handle);
        for line in reader.lines().map_while(std::io::Result::ok) {
            if let Some(clean) = sanitize_and_resolve_path(&line) {
                if !files.contains(&clean) {
                    files.push(clean);
                }
            }
        }
    }

    if !files.is_empty() {
        return files;
    }

    // 4. File manager integration cache file
    let saved = read_selection_file();
    if !saved.is_empty() {
        info!(
            "Found {} files from active file manager selection cache",
            saved.len()
        );
        return saved;
    }

    // 5. Active file manager auto-copy
    if is_file_manager_focused() {
        info!("File manager detected as active window, querying selection via copy shortcut...");
        trigger_copy_in_focused_window();
        let from_clip = get_files_from_clipboard();
        if !from_clip.is_empty() {
            info!(
                "Successfully captured {} files from file manager selection",
                from_clip.len()
            );
            return from_clip;
        }
    }

    // 6. Direct clipboard files
    let from_clip = get_files_from_clipboard();
    if !from_clip.is_empty() {
        info!("Found {} files in clipboard", from_clip.len());
        return from_clip;
    }

    // 7. Clipboard raw image
    if let Some(raw_img) = get_image_from_clipboard() {
        info!("Captured raw image from clipboard: {:?}", raw_img);
        return vec![raw_img];
    }

    // 8. Fallback: Native file picker
    info!("No files selected or found on clipboard; prompting with file picker...");
    if let Some(selected) = rfd::FileDialog::new()
        .set_title("Select images to convert with ImgWheel")
        .pick_files()
    {
        for p in selected {
            if let Some(clean) = sanitize_and_resolve_path(&p.to_string_lossy()) {
                if !files.contains(&clean) {
                    files.push(clean);
                }
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("file%3A%2F%2Ftest"), "file://test");
        assert_eq!(
            percent_decode("%E6%97%A5%E6%9C%AC%E8%AA%9E"),
            "日本語"
        );
        assert_eq!(percent_decode("plain_string"), "plain_string");
        assert_eq!(percent_decode("incomplete%"), "incomplete%");
        assert_eq!(percent_decode("incomplete%2"), "incomplete%2");
    }

    #[test]
    fn test_parse_uri_or_path_list() {
        let tmp_file = std::env::temp_dir().join("imgwheel_test_img.png");
        let _ = image::RgbaImage::new(10, 10).save(&tmp_file);

        let list = format!(
            "# comment\ncopy\nfile://{}\n",
            tmp_file.to_string_lossy()
        );
        let mut out = Vec::new();
        parse_uri_or_path_list(&list, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], tmp_file.canonicalize().unwrap_or(tmp_file));
    }

    #[test]
    fn test_sanitize_and_resolve_encoded_path() {
        let tmp_file = std::env::temp_dir().join("imgwheel test space.png");
        let _ = image::RgbaImage::new(5, 5).save(&tmp_file);

        let uri = format!("file://{}", tmp_file.to_string_lossy().replace(' ', "%20"));
        let resolved = sanitize_and_resolve_path(&uri);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), tmp_file.canonicalize().unwrap_or(tmp_file));
    }

    #[test]
    fn test_selection_cache_roundtrip() {
        let tmp_file = std::env::temp_dir().join("imgwheel_cache_test.png");
        let _ = image::RgbaImage::new(5, 5).save(&tmp_file);

        let files = vec![tmp_file.clone()];
        assert!(write_selection_file(&files).is_ok());
        let read_back = read_selection_file();
        assert!(!read_back.is_empty());
        assert_eq!(read_back[0], tmp_file.canonicalize().unwrap_or(tmp_file));
    }
}
