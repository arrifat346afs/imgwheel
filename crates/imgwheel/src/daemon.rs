use anyhow::{Context, Result};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use config::Config;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Command;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub files: Vec<PathBuf>,
    pub to: Option<String>,
    pub quality: Option<u8>,
    pub out_dir: Option<PathBuf>,
}

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("imgwheel.sock")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/imgwheel-{}.sock", uid))
    }
}

pub fn try_forward_to_daemon(req: &IpcRequest) -> bool {
    let socket_path = get_socket_path();
    if !socket_path.exists() {
        return false;
    }

    match UnixStream::connect(&socket_path) {
        Ok(mut stream) => {
            if let Ok(json) = serde_json::to_string(req) {
                let _ = writeln!(stream, "{}", json);
                let mut reader = BufReader::new(stream);
                let mut resp = String::new();
                if reader.read_line(&mut resp).is_ok() && resp.trim() == "OK" {
                    info!("Successfully forwarded request to running daemon");
                    return true;
                }
            }
            false
        }
        Err(_) => {
            // Stale socket
            let _ = fs::remove_file(socket_path);
            false
        }
    }
}

pub async fn run_daemon(config: Config) -> Result<()> {
    info!("Starting ImgWheel daemon...");
    let socket_path = get_socket_path();
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("Failed to bind IPC socket at {:?}", socket_path))?;
    listener
        .set_nonblocking(true)
        .context("Failed to set IPC socket nonblocking")?;

    info!("IPC server listening at {:?}", socket_path);

    // Spawn global shortcuts listener task
    let portal_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = init_global_shortcuts(portal_config).await {
            warn!("XDG Desktop Portal GlobalShortcuts unavailable: {}", e);
            warn!("You can bind the shortcut directly in your Hyprland configuration:");
            warn!("  bind = SUPER, C, exec, imgwheel");
        }
    });

    // Main IPC loop
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    handle_client(stream).await;
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Err(e) => {
                error!("IPC accept error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }
}

async fn handle_client(mut stream: UnixStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() {
        if let Ok(req) = serde_json::from_str::<IpcRequest>(&line) {
            info!("Daemon received IPC request for {} files", req.files.len());
            let _ = writeln!(stream, "OK");

            // Launch imgwheel process with these files
            let mut cmd =
                Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("imgwheel")));
            if let Some(ref target) = req.to {
                cmd.args(["--to", target]);
            }
            if let Some(q) = req.quality {
                cmd.args(["--quality", &q.to_string()]);
            }
            if let Some(ref out) = req.out_dir {
                cmd.args(["--out-dir", &out.to_string_lossy()]);
            }
            for file in req.files {
                cmd.arg(file);
            }
            let _ = cmd.spawn();
        }
    }
}

async fn init_global_shortcuts(config: Config) -> ashpd::Result<()> {
    info!("Initializing XDG Desktop Portal Global Shortcuts...");
    let proxy = GlobalShortcuts::new().await?;
    let session = proxy.create_session(Default::default()).await?;

    let shortcut = NewShortcut::new("imgwheel_convert", "ImgWheel Radial Converter")
        .preferred_trigger(Some(config.behavior.global_shortcut.as_str()));

    let shortcuts = [shortcut];
    let _bind_request = proxy
        .bind_shortcuts(&session, &shortcuts, None, Default::default())
        .await?
        .response()?;

    info!(
        "Global shortcut registered: {}",
        config.behavior.global_shortcut
    );

    let mut activated_stream = proxy.receive_activated().await?;
    while let Some(activated) = activated_stream.next().await {
        if activated.shortcut_id() == "imgwheel_convert" {
            info!("Global shortcut activated! Triggering ImgWheel...");
            trigger_from_shortcut();
        }
    }

    Ok(())
}

fn trigger_from_shortcut() {
    // 1. Try reading clipboard for files
    let mut files = get_files_from_clipboard();

    // 2. Fallback to file picker if clipboard has no image files
    if files.is_empty() {
        if let Some(selected) = rfd::FileDialog::new()
            .set_title("Select images to convert with ImgWheel")
            .pick_files()
        {
            files = selected;
        }
    }

    if !files.is_empty() {
        let mut cmd =
            Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("imgwheel")));
        for f in files {
            cmd.arg(f);
        }
        let _ = cmd.spawn();
    }
}

fn get_files_from_clipboard() -> Vec<PathBuf> {
    let mut files = Vec::new();
    // Try wl-paste if on wayland
    if let Ok(output) = Command::new("wl-paste")
        .args(["-t", "text/uri-list"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                let trimmed = line.trim();
                if let Some(path_str) = trimmed.strip_prefix("file://") {
                    let path = PathBuf::from(path_str);
                    if path.exists() {
                        files.push(path);
                    }
                }
            }
        }
    }
    files
}
