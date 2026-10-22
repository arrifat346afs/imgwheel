#![allow(clippy::collapsible_if)]

pub mod cli;
pub mod daemon;
pub mod hypr;
pub mod notify;
pub mod pill;
pub mod selection;
pub mod settings_ui;
pub mod wheel;

use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, OverwritePolicy, SingleOutputLocation};
use converter::{
    ConversionOptions, ImageFormat, available_formats, convert_file, detect_format_from_path,
    resolve_output_path,
};
use eframe::egui::{self, Color32, Ui, ViewportBuilder};
use eframe::{App, Frame};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use tracing::{Level, error};
use tracing_subscriber::FmtSubscriber;

use cli::{Cli, Commands};
use daemon::{IpcRequest, run_daemon, try_forward_to_daemon};
use hypr::{get_cursor_position, hyprland_config_snippet};
use notify::send_notification;
use pill::{BatchProgress, PillApp};
use settings_ui::SettingsApp;
use wheel::{WheelAction, WheelSegment, WheelStyle, WheelWidget};

struct WheelApp {
    config: Config,
    segments: Vec<WheelSegment>,
    selected_format: Option<ImageFormat>,
    files: Vec<PathBuf>,
    should_close: bool,
    wheel_style: WheelStyle,
    has_ever_focused: bool,
    startup_time: std::time::Instant,
}

impl WheelApp {
    fn new(config: Config, files: Vec<PathBuf>) -> Self {
        let caps = available_formats();

        // Check if single file source format is known
        let single_source_format = if files.len() == 1 {
            detect_format_from_path(&files[0]).ok()
        } else {
            None
        };

        let segments: Vec<WheelSegment> = config
            .formats
            .items
            .iter()
            .filter(|i| i.enabled)
            .map(|i| {
                let fmt = match ImageFormat::from_ext(i.format.extension()) {
                    Some(f) => f,
                    None => ImageFormat::Png,
                };
                let cap = caps.iter().find(|c| c.format == fmt);
                let is_avail = cap.map(|c| c.available).unwrap_or(true);
                let unavail_reason = cap.and_then(|c| c.reason.clone());

                // If single file and already in this format, grey out / disable
                let is_same_format = single_source_format == Some(fmt);
                let final_avail = is_avail && !is_same_format;
                let reason = if is_same_format {
                    Some("Already this format".to_string())
                } else {
                    unavail_reason
                };

                WheelSegment {
                    id: i.format.extension().to_string(),
                    label: i.label.clone(),
                    enabled: true,
                    available: final_avail,
                    unavailable_reason: reason,
                }
            })
            .collect();

        let wheel_op = config.appearance.wheel_opacity.clamp(0.2, 1.0);

        let style = WheelStyle {
            outer_radius: config.appearance.outer_radius,
            inner_radius: config.appearance.inner_radius,
            gap_width: config.appearance.gap_width,
            segment_color: Color32::from_rgba_premultiplied(
                ((config.appearance.segment_color[0] as f32) * wheel_op).round() as u8,
                ((config.appearance.segment_color[1] as f32) * wheel_op).round() as u8,
                ((config.appearance.segment_color[2] as f32) * wheel_op).round() as u8,
                ((config.appearance.segment_color[3] as f32) * wheel_op).round() as u8,
            ),
            hover_color: Color32::from_rgba_premultiplied(
                ((config.appearance.hover_color[0] as f32) * wheel_op).round() as u8,
                ((config.appearance.hover_color[1] as f32) * wheel_op).round() as u8,
                ((config.appearance.hover_color[2] as f32) * wheel_op).round() as u8,
                ((config.appearance.hover_color[3] as f32) * wheel_op).round() as u8,
            ),
            text_color: Color32::from_rgba_premultiplied(
                config.appearance.text_color[0],
                config.appearance.text_color[1],
                config.appearance.text_color[2],
                config.appearance.text_color[3],
            ),
            disabled_color: Color32::from_gray(50),
            gap_color: Color32::from_rgba_premultiplied(
                config.appearance.gap_color[0],
                config.appearance.gap_color[1],
                config.appearance.gap_color[2],
                config.appearance.gap_color[3],
            ),
            font_size: config.appearance.font_size,
            font_bold: config.appearance.font_bold,
            animation_speed: config.appearance.animation_speed,
        };

        Self {
            config,
            segments,
            selected_format: None,
            files,
            should_close: false,
            wheel_style: style,
            has_ever_focused: false,
            startup_time: std::time::Instant::now(),
        }
    }
}

impl App for WheelApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();

        // Track focus: window starts without focus on some Wayland compositors.
        let has_focus = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        if has_focus {
            self.has_ever_focused = true;
        }

        // Close on focus loss only after the window has been focused and grace period elapsed
        if self.config.behavior.close_on_focus_loss
            && self.has_ever_focused
            && self.startup_time.elapsed().as_millis() > 500
            && !has_focus
        {
            self.should_close = true;
        }

        if self.should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let center_hint = if self.files.len() > 1 {
            Some(format!("{} files", self.files.len()))
        } else {
            None
        };

        let widget = WheelWidget::new(&self.segments)
            .style(WheelStyle {
                outer_radius: self.wheel_style.outer_radius,
                inner_radius: self.wheel_style.inner_radius,
                gap_width: self.wheel_style.gap_width,
                segment_color: self.wheel_style.segment_color,
                hover_color: self.wheel_style.hover_color,
                text_color: self.wheel_style.text_color,
                disabled_color: self.wheel_style.disabled_color,
                gap_color: self.wheel_style.gap_color,
                font_size: self.wheel_style.font_size,
                font_bold: self.wheel_style.font_bold,
                animation_speed: self.wheel_style.animation_speed,
            })
            .center_text(center_hint);

        let action: WheelAction = widget.show(ui);

        if action.cancel_clicked || action.outside_clicked {
            self.should_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if let Some(seg_idx) = action.clicked_segment {
            if let Some(seg) = self.segments.get(seg_idx) {
                if seg.available {
                    if let Some(target_fmt) = ImageFormat::from_ext(&seg.id) {
                        self.selected_format = Some(target_fmt);
                        self.should_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn build_conversion_options(config: &Config, cli_quality: Option<u8>) -> ConversionOptions {
    let opts = &config.formats.options;
    ConversionOptions {
        jpg_quality: cli_quality.unwrap_or(opts.jpg_quality),
        webp_quality: cli_quality.unwrap_or(opts.webp_quality),
        webp_lossless: opts.webp_lossless,
        png_compression: opts.png_compression,
        avif_quality: cli_quality.unwrap_or(opts.avif_quality),
        avif_speed: opts.avif_speed,
        heic_quality: cli_quality.unwrap_or(opts.heic_quality),
        heif_quality: cli_quality.unwrap_or(opts.heif_quality),
        bmp_bit_depth: opts.bmp_bit_depth,
        eps_raster_dpi: opts.eps_raster_dpi,
        strip_metadata: opts.strip_metadata,
        auto_rotate_exif: opts.auto_rotate_exif,
        resize_enabled: opts.resize_enabled,
        max_width: opts.max_width,
        max_height: opts.max_height,
        alpha_background_color: opts.alpha_background_color,
        overwrite: config.behavior.overwrite_policy == OverwritePolicy::Always,
    }
}

fn execute_single_conversion(
    file: &Path,
    target_format: ImageFormat,
    config: &Config,
    options: &ConversionOptions,
    out_dir_override: Option<&Path>,
) -> Result<PathBuf> {
    let file_stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid filename stem")?;

    let parent_dir = if let Some(out_dir) = out_dir_override {
        out_dir.to_path_buf()
    } else {
        match config.behavior.single_output_location {
            SingleOutputLocation::BesideOriginal => file
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            SingleOutputLocation::InFolder => {
                let p = file.parent().unwrap_or_else(|| Path::new("."));
                p.join(&config.behavior.batch_output_folder)
            }
        }
    };

    let dest_path = resolve_output_path(
        &parent_dir,
        file_stem,
        target_format.extension(),
        options.overwrite,
    );

    convert_file(file, &dest_path, target_format, options)?;
    Ok(dest_path)
}

fn execute_batch_conversions(
    files: &[PathBuf],
    target_format: ImageFormat,
    config: &Config,
    options: &ConversionOptions,
    out_dir_override: Option<&Path>,
    tx: Option<Sender<BatchProgress>>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> (usize, usize) {
    let total = files.len();
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let success_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let fail_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    files.par_iter().for_each(|file| {
        if let Some(ref flag) = cancel_flag {
            if flag.load(Ordering::SeqCst) {
                return;
            }
        }

        let parent_dir = if let Some(out_dir) = out_dir_override {
            out_dir.to_path_buf()
        } else {
            let p = file.parent().unwrap_or_else(|| Path::new("."));
            p.join(&config.behavior.batch_output_folder)
        };

        let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("image");

        let dest_path = resolve_output_path(
            &parent_dir,
            file_stem,
            target_format.extension(),
            options.overwrite,
        );

        match convert_file(file, &dest_path, target_format, options) {
            Ok(_) => {
                success_counter.fetch_add(1, Ordering::SeqCst);
            }
            Err(e) => {
                fail_counter.fetch_add(1, Ordering::SeqCst);
                error!("Failed to convert {:?}: {}", file, e);
            }
        }

        let curr = counter.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(ref transmitter) = tx {
            let _ = transmitter.send(BatchProgress::Progress {
                current: curr,
                total,
                target_name: target_format.display_name().to_string(),
            });
        }
    });

    let is_cancelled = cancel_flag
        .map(|f| f.load(Ordering::SeqCst))
        .unwrap_or(false);

    let success = success_counter.load(Ordering::SeqCst);
    let fail = fail_counter.load(Ordering::SeqCst);

    if let Some(ref transmitter) = tx {
        if is_cancelled {
            let _ = transmitter.send(BatchProgress::Cancelled);
        } else {
            let _ = transmitter.send(BatchProgress::Done {
                success,
                failure: fail,
                total,
            });
        }
    }

    (success, fail)
}

fn install_integrations_cli() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let home_path = PathBuf::from(home);

    println!("Installing ImgWheel file manager integrations...");

    // 1. Nautilus Extension
    let nautilus_ext_dir = home_path.join(".local/share/nautilus-python/extensions");
    let _ = std::fs::create_dir_all(&nautilus_ext_dir);
    let nautilus_ext_file = nautilus_ext_dir.join("imgwheel.py");
    let nautilus_code = include_str!("../../../data/file-manager/nautilus/imgwheel-nautilus.py");
    if std::fs::write(&nautilus_ext_file, nautilus_code).is_ok() {
        println!("  ✓ Nautilus Python extension installed at {:?}", nautilus_ext_file);
    }

    // 2. Nautilus Script
    let nautilus_scripts_dir = home_path.join(".local/share/nautilus/scripts");
    let _ = std::fs::create_dir_all(&nautilus_scripts_dir);
    let nautilus_script_file = nautilus_scripts_dir.join("Convert with ImgWheel");
    let script_code = include_str!("../../../data/file-manager/nautilus/Convert with ImgWheel");
    if std::fs::write(&nautilus_script_file, script_code).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &nautilus_script_file,
                std::fs::Permissions::from_mode(0o755),
            );
        }
        println!("  ✓ Nautilus Script installed at {:?}", nautilus_script_file);
    }

    // 3. Dolphin Service Menus
    let dolphin_dirs = [
        home_path.join(".local/share/kservices5/ServiceMenus"),
        home_path.join(".local/share/kio/servicemenus"),
    ];
    let dolphin_code = include_str!("../../../data/file-manager/dolphin/imgwheel.desktop");
    for dir in &dolphin_dirs {
        let _ = std::fs::create_dir_all(dir);
        let dest = dir.join("imgwheel.desktop");
        if std::fs::write(&dest, dolphin_code).is_ok() {
            println!("  ✓ Dolphin Service Menu installed at {:?}", dest);
        }
    }

    // 4. Nemo Action
    let nemo_dir = home_path.join(".local/share/nemo/actions");
    let _ = std::fs::create_dir_all(&nemo_dir);
    let nemo_file = nemo_dir.join("imgwheel.nemo_action");
    let nemo_code = include_str!("../../../data/file-manager/nemo/imgwheel.nemo_action");
    if std::fs::write(&nemo_file, nemo_code).is_ok() {
        println!("  ✓ Nemo Action installed at {:?}", nemo_file);
    }

    // 5. Desktop File
    let apps_dir = home_path.join(".local/share/applications");
    let _ = std::fs::create_dir_all(&apps_dir);
    let desktop_file = apps_dir.join("imgwheel.desktop");
    let desktop_code = include_str!("../../../data/imgwheel.desktop");
    if std::fs::write(&desktop_file, desktop_code).is_ok() {
        println!("  ✓ Desktop application entry installed at {:?}", desktop_file);
    }

    println!("\nAll integrations installed successfully!");
    println!("Tip: If using Nautilus, restart it to reload python extensions: `nautilus -q`");
}

fn open_settings_window(config: Config) {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("ImgWheel Settings")
            .with_app_id("imgwheel-settings")
            .with_inner_size([960.0, 680.0])
            .with_min_inner_size([740.0, 520.0])
            .with_transparent(true)
            .with_resizable(true),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "ImgWheel Settings",
        native_options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::new(config)))),
    );
}

fn parse_wheel_position(arg: &Option<String>) -> Option<(f32, f32)> {
    if let Some(pos_str) = arg {
        let parts: Vec<&str> = pos_str.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (
                parts[0].trim().parse::<f32>(),
                parts[1].trim().parse::<f32>(),
            ) {
                return Some((x, y));
            }
        }
    }
    get_cursor_position()
}

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let cli = Cli::parse();
    let config = Config::load();

    // 1. Subcommands
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::InstallHyprland => {
                println!("{}", hyprland_config_snippet());
                std::process::exit(0);
            }
            Commands::InstallIntegrations => {
                install_integrations_cli();
                std::process::exit(0);
            }
            Commands::Settings => {
                open_settings_window(config);
                std::process::exit(0);
            }
            Commands::Daemon => {
                let rt =
                    tokio::runtime::Runtime::new().expect("Failed to initialize Tokio runtime");
                rt.block_on(async move {
                    if let Err(e) = run_daemon(config).await {
                        error!("Daemon error: {}", e);
                    }
                });
                std::process::exit(0);
            }
        }
    }

    // 2. Collect and resolve input files (CLI, stdin, file manager cache, active window Ctrl+C, clipboard, picker)
    let files = selection::resolve_input_files(&cli.files, cli.stdin, cli.demo);
    if files.is_empty() {
        // User cancelled or no valid image files found
        std::process::exit(0);
    }

    // 3. Try forwarding to running daemon if direct CLI mode without skipping wheel
    if cli.to.is_none() {
        let req = IpcRequest {
            files: files.clone(),
            to: cli.to.clone(),
            quality: cli.quality,
            out_dir: cli.out_dir.clone(),
        };
        if try_forward_to_daemon(&req) {
            std::process::exit(0);
        }
    }

    let options = build_conversion_options(&config, cli.quality);

    // 4. Direct conversion without wheel (--to)
    if let Some(ref target_ext) = cli.to {
        let target_fmt = match ImageFormat::from_ext(target_ext) {
            Some(fmt) => fmt,
            None => {
                error!("Unsupported target format: {}", target_ext);
                std::process::exit(2);
            }
        };

        if files.len() == 1 {
            match execute_single_conversion(
                &files[0],
                target_fmt,
                &config,
                &options,
                cli.out_dir.as_deref(),
            ) {
                Ok(dest) => {
                    if config.behavior.notifications {
                        send_notification(
                            "Conversion successful",
                            &format!("Saved to {}", dest.display()),
                            false,
                        );
                    }
                    std::process::exit(0);
                }
                Err(e) => {
                    error!("Conversion failed: {}", e);
                    if config.behavior.notifications {
                        send_notification("Conversion failed", &format!("{}", e), true);
                    }
                    std::process::exit(2);
                }
            }
        } else {
            let (success, fail) = execute_batch_conversions(
                &files,
                target_fmt,
                &config,
                &options,
                cli.out_dir.as_deref(),
                None,
                None,
            );
            if config.behavior.notifications {
                let msg = format!(
                    "Converted {} / {} images to {}",
                    success,
                    files.len(),
                    target_fmt.display_name()
                );
                send_notification("Batch conversion completed", &msg, fail > 0);
            }
            if fail == 0 {
                std::process::exit(0);
            } else if success > 0 {
                std::process::exit(1);
            } else {
                std::process::exit(2);
            }
        }
    }

    // 5. Radial Wheel UI mode
    let diameter = config.appearance.outer_radius * 2.0;
    let window_size = diameter + 30.0;

    let mut viewport = ViewportBuilder::default()
        .with_title("ImgWheel")
        .with_app_id("imgwheel")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top()
        .with_resizable(false)
        .with_inner_size([window_size, window_size]);

    if let Some((cx, cy)) = parse_wheel_position(&cli.wheel_at) {
        let half = window_size / 2.0;
        viewport = viewport.with_position([cx - half, cy - half]);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let selected_format = Arc::new(std::sync::Mutex::new(None));
    let selected_format_clone = Arc::clone(&selected_format);

    let app_files = files.clone();
    let app_config = config.clone();

    let res = eframe::run_native(
        "ImgWheel",
        native_options,
        Box::new(move |_cc| {
            let wheel_app = WheelApp::new(app_config, app_files);
            Ok(Box::new(WheelAppWrapper {
                app: wheel_app,
                chosen_format_out: selected_format_clone,
            }))
        }),
    );

    if let Err(e) = res {
        error!("Error launching wheel window: {}", e);
        std::process::exit(2);
    }

    let picked_format = *selected_format.lock().unwrap();
    let target_fmt = match picked_format {
        Some(fmt) => fmt,
        None => {
            // Cancelled or closed without selection
            std::process::exit(0);
        }
    };

    // 6. Handle chosen format execution
    if files.len() == 1 {
        match execute_single_conversion(
            &files[0],
            target_fmt,
            &config,
            &options,
            cli.out_dir.as_deref(),
        ) {
            Ok(dest) => {
                if config.behavior.notifications {
                    send_notification(
                        "Conversion successful",
                        &format!("Saved to {}", dest.display()),
                        false,
                    );
                }
                std::process::exit(0);
            }
            Err(e) => {
                error!("Conversion failed: {}", e);
                if config.behavior.notifications {
                    send_notification("Conversion failed", &format!("{}", e), true);
                }
                std::process::exit(2);
            }
        }
    } else {
        // Multiple images: launch Pill progress window & background rayon task
        let (tx, rx) = channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel_flag);

        let batch_files = files.clone();
        let batch_config = config.clone();
        let batch_options = options.clone();
        let batch_out_dir = cli.out_dir.clone();

        std::thread::spawn(move || {
            execute_batch_conversions(
                &batch_files,
                target_fmt,
                &batch_config,
                &batch_options,
                batch_out_dir.as_deref(),
                Some(tx),
                Some(cancel_clone),
            );
        });

        let pill_width = config.appearance.pill.width;
        let pill_height = config.appearance.pill.height;

        let mut pill_viewport = ViewportBuilder::default()
            .with_title("ImgWheel Progress")
            .with_app_id("imgwheel-pill")
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_inner_size([pill_width + 20.0, pill_height + 20.0]);

        if let Some((cx, cy)) = parse_wheel_position(&cli.wheel_at) {
            let half_w = (pill_width + 20.0) / 2.0;
            let half_h = (pill_height + 20.0) / 2.0;
            pill_viewport = pill_viewport.with_position([cx - half_w, cy - half_h]);
        }

        let pill_options = eframe::NativeOptions {
            viewport: pill_viewport,
            ..Default::default()
        };

        let pill_cfg = config.appearance.pill.clone();
        let _ = eframe::run_native(
            "ImgWheel Progress",
            pill_options,
            Box::new(move |_cc| Ok(Box::new(PillApp::new(pill_cfg, rx, cancel_flag)))),
        );

        if config.behavior.open_folder_after_batch {
            let sample_parent = files[0].parent().unwrap_or_else(|| Path::new("."));
            let target_folder = sample_parent.join(&config.behavior.batch_output_folder);
            let _ = std::process::Command::new("xdg-open")
                .arg(target_folder)
                .spawn();
        }

        std::process::exit(0);
    }
}

struct WheelAppWrapper {
    app: WheelApp,
    chosen_format_out: Arc<std::sync::Mutex<Option<ImageFormat>>>,
}

impl App for WheelAppWrapper {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {
        self.app.ui(ui, frame);
        if let Some(fmt) = self.app.selected_format {
            *self.chosen_format_out.lock().unwrap() = Some(fmt);
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color(visuals)
    }
}
