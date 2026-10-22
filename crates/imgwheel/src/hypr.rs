use serde::Deserialize;
use std::process::Command;
use tracing::{debug, warn};

#[derive(Debug, Deserialize)]
struct CursorPos {
    x: f64,
    y: f64,
}

pub fn is_hyprland() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

pub fn get_cursor_position() -> Option<(f32, f32)> {
    if !is_hyprland() {
        return None;
    }

    let output = Command::new("hyprctl")
        .args(["cursorpos", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        warn!("hyprctl cursorpos returned non-zero exit code");
        return None;
    }

    let parsed: CursorPos = serde_json::from_slice(&output.stdout).ok()?;
    debug!("Hyprland cursor position: ({}, {})", parsed.x, parsed.y);
    Some((parsed.x as f32, parsed.y as f32))
}

pub fn hyprland_config_snippet() -> &'static str {
    r#"# Add the following lines to your ~/.config/hypr/hyprland.conf:

bind = SUPER, C, exec, imgwheel
windowrulev2 = float, class:^(imgwheel)$
windowrulev2 = noborder, class:^(imgwheel)$
windowrulev2 = noshadow, class:^(imgwheel)$
windowrulev2 = noblur, class:^(imgwheel)$
windowrulev2 = noanim, class:^(imgwheel)$
windowrulev2 = pin, class:^(imgwheel)$
windowrulev2 = move cursor -50% -50%, class:^(imgwheel)$
windowrulev2 = stayfocused, class:^(imgwheel)$
"#
}
