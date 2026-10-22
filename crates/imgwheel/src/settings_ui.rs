use crate::wheel::{WheelSegment, WheelStyle, WheelWidget};
use config::{Config, OverwritePolicy, SingleOutputLocation};
use eframe::egui::{
    self, Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Slider, Stroke, Ui, vec2,
};
use eframe::{App, Frame};

// Material Design 3 Color Tokens
const MD_PRIMARY: Color32 = Color32::from_rgb(138, 180, 248); // Google / M3 primary blue
const MD_PRIMARY_CONTAINER: Color32 = Color32::from_rgb(33, 44, 66);
const MD_TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 245, 248);
const MD_TEXT_SECONDARY: Color32 = Color32::from_rgb(162, 170, 182);

// Dynamic surface colors that stay in sync with opacity
fn surface_card_color(op: f32) -> Color32 {
    let a = (255.0 * op).clamp(0.0, 255.0).round() as u8;
    Color32::from_rgba_premultiplied(
        (26.0 * op).round() as u8,
        (28.0 * op).round() as u8,
        (36.0 * op).round() as u8,
        a,
    )
}

fn surface_container_color(op: f32) -> Color32 {
    let a = (255.0 * op).clamp(0.0, 255.0).round() as u8;
    Color32::from_rgba_premultiplied(
        (34.0 * op).round() as u8,
        (37.0 * op).round() as u8,
        (48.0 * op).round() as u8,
        a,
    )
}

fn surface_high_color(op: f32) -> Color32 {
    let a = (255.0 * op).clamp(0.0, 255.0).round() as u8;
    Color32::from_rgba_premultiplied(
        (46.0 * op).round() as u8,
        (50.0 * op).round() as u8,
        (64.0 * op).round() as u8,
        a,
    )
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SettingsTab {
    Appearance,
    Formats,
    Behavior,
}

pub struct SettingsApp {
    pub config: Config,
    active_tab: SettingsTab,
    status_msg: Option<(String, std::time::Instant)>,
}

impl SettingsApp {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            active_tab: SettingsTab::Appearance,
            status_msg: None,
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            self.status_msg = Some((format!("Failed to save: {}", e), std::time::Instant::now()));
        } else {
            self.status_msg = Some(("Saved successfully".into(), std::time::Instant::now()));
        }
    }
}

impl App for SettingsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let win_opacity = self.config.appearance.window_opacity.clamp(0.2, 1.0);
        let widget_opacity = (self.config.appearance.widget_opacity * win_opacity).clamp(0.05, 1.0);
        let alpha = (win_opacity * 255.0).round() as u8;

        // Base Material 3 dark surface background with configurable opacity
        let bg_fill = Color32::from_rgba_premultiplied(
            (16.0 * win_opacity).round() as u8,
            (16.0 * win_opacity).round() as u8,
            (20.0 * win_opacity).round() as u8,
            alpha,
        );

        let painter = ui.painter();
        let total_rect = ui.available_rect_before_wrap();
        painter.rect_filled(total_rect, CornerRadius::ZERO, bg_fill);

        let mut changed = false;

        ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
        ui.spacing_mut().window_margin = egui::Margin::same(16);

        egui::Frame::NONE
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                // Top App Bar (Material 3 style)
                if render_app_bar(ui, &mut self.config, widget_opacity, &mut changed) {
                    changed = true;
                }

                ui.add_space(8.0);

                // Material 3 Navigation Segmented Pill Tabs (borderless, synced opacity)
                render_segmented_tabs(ui, &mut self.active_tab, widget_opacity);

                ui.add_space(10.0);

                // Main two-column content
                let available_height = ui.available_height() - 36.0;
                ui.horizontal(|ui| {
                    // Left Column: Tab content inside scroll area
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width() * 0.54, available_height),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ScrollArea::vertical()
                                .id_salt("m3_settings_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    match self.active_tab {
                                        SettingsTab::Appearance => {
                                            if render_appearance_tab(
                                                ui,
                                                &mut self.config,
                                                widget_opacity,
                                            ) {
                                                changed = true;
                                            }
                                        }
                                        SettingsTab::Formats => {
                                            if render_formats_tab(
                                                ui,
                                                &mut self.config,
                                                widget_opacity,
                                            ) {
                                                changed = true;
                                            }
                                        }
                                        SettingsTab::Behavior => {
                                            if render_behavior_tab(
                                                ui,
                                                &mut self.config,
                                                widget_opacity,
                                            ) {
                                                changed = true;
                                            }
                                        }
                                    }
                                    ui.add_space(20.0);
                                });
                        },
                    );

                    ui.add_space(12.0);

                    // Right Column: Elevated Material Live Wheel Preview Stage (borderless, synced opacity)
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), available_height),
                        Layout::top_down(Align::Center),
                        |ui| {
                            render_preview_stage(ui, &self.config, widget_opacity);
                        },
                    );
                });

                // Status snackbar message
                if let Some((ref msg, instant)) = self.status_msg {
                    if instant.elapsed().as_secs() < 3 {
                        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                            render_snackbar(ui, msg);
                        });
                    }
                }
            });

        if changed {
            self.save_config();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let op = self.config.appearance.window_opacity.clamp(0.2, 1.0);
        // Base clear color scaled with window opacity
        [0.06 * op, 0.06 * op, 0.08 * op, op]
    }
}

// ----------------------------------------------------------------------------
// Material 3 UI Components & Tabs
// ----------------------------------------------------------------------------

fn render_app_bar(ui: &mut Ui, cfg: &mut Config, widget_op: f32, changed: &mut bool) -> bool {
    let mut local_changed = false;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("ImgWheel")
                        .size(22.0)
                        .strong()
                        .color(MD_PRIMARY),
                );
                ui.label(RichText::new("Settings").size(22.0).color(MD_TEXT_PRIMARY));
            });
            ui.label(
                RichText::new("Material customization for the radial format converter")
                    .size(12.0)
                    .color(MD_TEXT_SECONDARY),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Reset button (borderless)
            let reset_btn = egui::Button::new(
                RichText::new("Reset Defaults")
                    .color(MD_TEXT_PRIMARY)
                    .size(12.0),
            )
            .fill(surface_high_color(widget_op))
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(16));

            if ui.add(reset_btn).clicked() {
                *cfg = Config::default();
                *changed = true;
                local_changed = true;
            }

            ui.add_space(16.0);

            // Window Opacity Slider right in the top app bar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Window Opacity:")
                        .size(12.0)
                        .color(MD_TEXT_SECONDARY),
                );
                let slider = Slider::new(&mut cfg.appearance.window_opacity, 0.20..=1.0)
                    .show_value(true)
                    .custom_formatter(|val, _| format!("{:.0}%", val * 100.0));
                if ui.add(slider).changed() {
                    local_changed = true;
                }
            });
        });
    });

    local_changed
}

fn render_segmented_tabs(ui: &mut Ui, active_tab: &mut SettingsTab, widget_op: f32) {
    let container_radius = CornerRadius::same(20);
    let bar_height = 42.0;

    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), bar_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            let max_rect = ui.max_rect();
            let painter = ui.painter();
            // Borderless background with synced opacity
            painter.rect_filled(
                max_rect,
                container_radius,
                surface_container_color(widget_op),
            );

            ui.add_space(6.0);
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
            render_tab_pill(ui, active_tab, SettingsTab::Appearance, "🎨  Appearance");
            render_tab_pill(ui, active_tab, SettingsTab::Formats, "⚡  Formats");
            render_tab_pill(ui, active_tab, SettingsTab::Behavior, "⚙  Behavior");
        },
    );
}

fn render_tab_pill(ui: &mut Ui, current: &mut SettingsTab, target: SettingsTab, label: &str) {
    let is_selected = *current == target;
    let text = RichText::new(label).size(13.0).color(if is_selected {
        MD_PRIMARY
    } else {
        MD_TEXT_SECONDARY
    });

    let fill_color = if is_selected {
        MD_PRIMARY_CONTAINER
    } else {
        Color32::TRANSPARENT
    };

    // Borderless pill
    let btn = egui::Button::new(text)
        .fill(fill_color)
        .corner_radius(CornerRadius::same(16))
        .stroke(Stroke::NONE);

    if ui.add(btn).clicked() {
        *current = target;
    }
}

fn material_card<R>(
    ui: &mut Ui,
    title: &str,
    widget_op: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let card_radius = CornerRadius::same(14);
    // Borderless card with synced opacity
    let frame = egui::Frame::NONE
        .fill(surface_card_color(widget_op))
        .stroke(Stroke::NONE)
        .corner_radius(card_radius)
        .inner_margin(egui::Margin::same(14));

    frame
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(15.0).strong().color(MD_PRIMARY));
            ui.add_space(6.0);
            add_contents(ui)
        })
        .inner
}

fn render_appearance_tab(ui: &mut Ui, cfg: &mut Config, widget_op: f32) -> bool {
    let mut changed = false;

    // Card 1: Window & Widget Transparency
    material_card(ui, "Window & Widget Opacity", widget_op, |ui| {
        ui.label(
            RichText::new("Widgets stay seamlessly synced with the window opacity for a unified borderless look:")
                .size(12.0)
                .color(MD_TEXT_SECONDARY),
        );
        ui.add_space(6.0);

        egui::Grid::new("m3_opacity_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Main Window Opacity:");
                let slider = Slider::new(&mut cfg.appearance.window_opacity, 0.20..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0));
                if ui.add(slider).changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label("Widget Card Opacity Scale:");
                let slider = Slider::new(&mut cfg.appearance.widget_opacity, 0.20..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0));
                if ui.add(slider).changed() {
                    changed = true;
                }
                ui.end_row();

                ui.label("Wheel Menu Opacity:");
                let slider = Slider::new(&mut cfg.appearance.wheel_opacity, 0.20..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0));
                if ui.add(slider).changed() {
                    changed = true;
                }
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // Card 2: Presets
    material_card(ui, "Color Presets", widget_op, |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.add(preset_chip("Material Dark", widget_op)).clicked() {
                cfg.appearance.segment_color = [45, 48, 58, 255];
                cfg.appearance.hover_color = [138, 180, 248, 255];
                cfg.appearance.text_color = [243, 245, 248, 255];
                cfg.appearance.gap_color = [70, 75, 90, 255];
                cfg.appearance.window_opacity = 0.96;
                cfg.appearance.widget_opacity = 0.85;
                changed = true;
            }
            if ui.add(preset_chip("Material Blue", widget_op)).clicked() {
                cfg.appearance.segment_color = [30, 42, 60, 255];
                cfg.appearance.hover_color = [59, 130, 246, 255];
                cfg.appearance.text_color = [255, 255, 255, 255];
                cfg.appearance.gap_color = [60, 80, 110, 255];
                cfg.appearance.window_opacity = 0.94;
                cfg.appearance.widget_opacity = 0.85;
                changed = true;
            }
            if ui.add(preset_chip("Nord Slate", widget_op)).clicked() {
                cfg.appearance.segment_color = [46, 52, 64, 255];
                cfg.appearance.hover_color = [136, 192, 208, 255];
                cfg.appearance.text_color = [236, 239, 244, 255];
                cfg.appearance.gap_color = [76, 86, 106, 255];
                cfg.appearance.window_opacity = 0.95;
                cfg.appearance.widget_opacity = 0.85;
                changed = true;
            }
            if ui.add(preset_chip("Onyx Contrast", widget_op)).clicked() {
                cfg.appearance.segment_color = [30, 30, 30, 255];
                cfg.appearance.hover_color = [255, 255, 255, 255];
                cfg.appearance.text_color = [255, 255, 255, 255];
                cfg.appearance.gap_color = [60, 60, 60, 255];
                cfg.appearance.window_opacity = 1.0;
                cfg.appearance.widget_opacity = 0.90;
                changed = true;
            }
        });
    });

    ui.add_space(10.0);

    // Card 3: Wheel Geometry
    material_card(ui, "Wheel Geometry & Dynamics", widget_op, |ui| {
        egui::Grid::new("m3_geometry_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Outer Radius:");
                if ui
                    .add(Slider::new(&mut cfg.appearance.outer_radius, 80.0..=260.0).suffix(" px"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Inner Hole Radius:");
                if ui
                    .add(Slider::new(&mut cfg.appearance.inner_radius, 20.0..=120.0).suffix(" px"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Gap Width:");
                if ui
                    .add(Slider::new(&mut cfg.appearance.gap_width, 0.0..=12.0).suffix(" px"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Hover Animation Duration:");
                if ui
                    .add(Slider::new(&mut cfg.appearance.animation_speed, 0.0..=0.4).suffix(" s"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // Card 4: Colors
    material_card(ui, "Segment & Palette Colors", widget_op, |ui| {
        ui.horizontal(|ui| {
            if color_tile(ui, "Segment", &mut cfg.appearance.segment_color) {
                changed = true;
            }
            if color_tile(ui, "Hover Highlight", &mut cfg.appearance.hover_color) {
                changed = true;
            }
            if color_tile(ui, "Text", &mut cfg.appearance.text_color) {
                changed = true;
            }
            if color_tile(ui, "Gap", &mut cfg.appearance.gap_color) {
                changed = true;
            }
        });
    });

    ui.add_space(10.0);

    // Card 5: Typography
    material_card(ui, "Typography", widget_op, |ui| {
        ui.horizontal(|ui| {
            ui.label("Font Size:");
            if ui
                .add(Slider::new(&mut cfg.appearance.font_size, 10.0..=28.0))
                .changed()
            {
                changed = true;
            }
            ui.add_space(12.0);
            if ui
                .checkbox(&mut cfg.appearance.font_bold, "Bold Typography")
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.add_space(10.0);

    // Card 6: Pill Progress Window
    material_card(ui, "Batch Capsule Window", widget_op, |ui| {
        egui::Grid::new("m3_pill_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Capsule Width:");
                if ui
                    .add(Slider::new(&mut cfg.appearance.pill.width, 240.0..=480.0).suffix(" px"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Capsule Height:");
                if ui
                    .add(Slider::new(&mut cfg.appearance.pill.height, 36.0..=72.0).suffix(" px"))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Auto-close Delay:");
                if ui
                    .add(
                        Slider::new(&mut cfg.appearance.pill.auto_close_delay_secs, 0.5..=8.0)
                            .suffix(" s"),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
    });

    changed
}

fn preset_chip(name: &str, widget_op: f32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(name).size(12.0).color(MD_TEXT_PRIMARY))
        .fill(surface_high_color(widget_op))
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(14))
}

fn color_tile(ui: &mut Ui, label: &str, color: &mut [u8; 4]) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(11.0).color(MD_TEXT_SECONDARY));
        let mut egui_color =
            Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
        if egui::color_picker::color_edit_button_srgba(
            ui,
            &mut egui_color,
            egui::color_picker::Alpha::Opaque,
        )
        .changed()
        {
            color[0] = egui_color.r();
            color[1] = egui_color.g();
            color[2] = egui_color.b();
            color[3] = egui_color.a();
            changed = true;
        }
    });
    changed
}

fn render_formats_tab(ui: &mut Ui, cfg: &mut Config, widget_op: f32) -> bool {
    let mut changed = false;

    // Card 1: Format Slices & Reordering
    material_card(ui, "Radial Wheel Formats (Clockwise)", widget_op, |ui| {
        ui.label(
            RichText::new("Reorder slices or customize visible segment labels:")
                .size(12.0)
                .color(MD_TEXT_SECONDARY),
        );
        ui.add_space(4.0);

        let len = cfg.formats.items.len();
        let mut move_up = None;
        let mut move_down = None;

        for i in 0..len {
            let item = &mut cfg.formats.items[i];
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 36.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    let row_rect = ui.max_rect();
                    let painter = ui.painter();
                    // Borderless row background with synced opacity
                    painter.rect_filled(
                        row_rect,
                        CornerRadius::same(8),
                        surface_container_color(widget_op),
                    );

                    ui.add_space(8.0);
                    if ui.button("▲").clicked() && i > 0 {
                        move_up = Some(i);
                    }
                    if ui.button("▼").clicked() && i + 1 < len {
                        move_down = Some(i);
                    }
                    if ui.checkbox(&mut item.enabled, "").changed() {
                        changed = true;
                    }
                    let badge_text = RichText::new(item.format.extension().to_uppercase())
                        .size(11.0)
                        .strong()
                        .color(MD_PRIMARY);
                    ui.label(badge_text);

                    ui.add_space(6.0);
                    ui.label(RichText::new("Label:").size(12.0).color(MD_TEXT_SECONDARY));
                    if ui.text_edit_singleline(&mut item.label).changed() {
                        changed = true;
                    }
                },
            );
            ui.add_space(4.0);
        }

        if let Some(idx) = move_up {
            cfg.formats.items.swap(idx, idx - 1);
            changed = true;
        }
        if let Some(idx) = move_down {
            cfg.formats.items.swap(idx, idx + 1);
            changed = true;
        }
    });

    ui.add_space(10.0);

    // Card 2: Encoding Quality
    material_card(ui, "Encoder Quality & Compression", widget_op, |ui| {
        let opts = &mut cfg.formats.options;
        egui::Grid::new("m3_options_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("JPEG Quality:");
                if ui
                    .add(Slider::new(&mut opts.jpg_quality, 1..=100))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("WebP Quality:");
                ui.horizontal(|ui| {
                    if ui
                        .add(Slider::new(&mut opts.webp_quality, 1..=100))
                        .changed()
                    {
                        changed = true;
                    }
                    if ui.checkbox(&mut opts.webp_lossless, "Lossless").changed() {
                        changed = true;
                    }
                });
                ui.end_row();

                ui.label("AVIF Quality & Speed:");
                ui.horizontal(|ui| {
                    if ui
                        .add(Slider::new(&mut opts.avif_quality, 1..=100))
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .add(Slider::new(&mut opts.avif_speed, 1..=10).prefix("Speed "))
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.end_row();

                ui.label("HEIC Quality:");
                if ui
                    .add(Slider::new(&mut opts.heic_quality, 1..=100))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("HEIF Quality:");
                if ui
                    .add(Slider::new(&mut opts.heif_quality, 1..=100))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("EPS Raster DPI:");
                if ui
                    .add(Slider::new(&mut opts.eps_raster_dpi, 72..=600))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        if ui
            .checkbox(&mut opts.strip_metadata, "Strip Metadata (EXIF)")
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut opts.auto_rotate_exif,
                "Auto-rotate by EXIF Orientation Tag",
            )
            .changed()
        {
            changed = true;
        }
    });

    changed
}

fn render_behavior_tab(ui: &mut Ui, cfg: &mut Config, widget_op: f32) -> bool {
    let mut changed = false;

    // Card 1: Output Destination
    material_card(ui, "Output Locations & Policies", widget_op, |ui| {
        egui::Grid::new("m3_behavior_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Batch Output Subfolder:");
                if ui
                    .text_edit_singleline(&mut cfg.behavior.batch_output_folder)
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Single Image Destination:");
                ui.horizontal(|ui| {
                    if ui
                        .radio_value(
                            &mut cfg.behavior.single_output_location,
                            SingleOutputLocation::BesideOriginal,
                            "Beside Original",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .radio_value(
                            &mut cfg.behavior.single_output_location,
                            SingleOutputLocation::InFolder,
                            "In Subfolder",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.end_row();

                ui.label("Overwrite Policy:");
                ui.horizontal(|ui| {
                    if ui
                        .radio_value(
                            &mut cfg.behavior.overwrite_policy,
                            OverwritePolicy::Never,
                            "Never (collision-safe)",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .radio_value(
                            &mut cfg.behavior.overwrite_policy,
                            OverwritePolicy::Always,
                            "Always",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // Card 2: Window Lifecycle
    material_card(ui, "Window Lifecycle & Notifications", widget_op, |ui| {
        if ui
            .checkbox(
                &mut cfg.behavior.close_on_focus_loss,
                "Close Wheel When Window Loses Focus",
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut cfg.behavior.close_after_convert,
                "Close Wheel Immediately When Format Picked",
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut cfg.behavior.notifications,
                "Show Desktop Notifications on Completion",
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut cfg.behavior.open_folder_after_batch,
                "Open Destination Folder After Batch Conversion",
            )
            .changed()
        {
            changed = true;
        }
    });

    ui.add_space(10.0);

    // Card 3: Shortcut
    material_card(ui, "Global Shortcut Integration", widget_op, |ui| {
        ui.horizontal(|ui| {
            ui.label("Keybinding:");
            if ui
                .text_edit_singleline(&mut cfg.behavior.global_shortcut)
                .changed()
            {
                changed = true;
            }
        });
        ui.label(
            RichText::new("Default: SUPER+C. Registered via XDG Desktop Portal GlobalShortcuts")
                .size(11.0)
                .color(MD_TEXT_SECONDARY),
        );
    });

    changed
}

// ----------------------------------------------------------------------------
// Right Column: Live Wheel Preview Stage
// ----------------------------------------------------------------------------

fn render_preview_stage(ui: &mut Ui, cfg: &Config, widget_op: f32) {
    let stage_radius = CornerRadius::same(16);
    // Borderless preview stage card with synced opacity
    let stage_frame = egui::Frame::NONE
        .fill(surface_card_color(widget_op))
        .stroke(Stroke::NONE)
        .corner_radius(stage_radius)
        .inner_margin(egui::Margin::same(16));

    stage_frame.show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Live Wheel Preview")
                    .size(16.0)
                    .strong()
                    .color(MD_PRIMARY),
            );
            ui.label(
                RichText::new("Interactive preview matching current styling")
                    .size(12.0)
                    .color(MD_TEXT_SECONDARY),
            );
            ui.add_space(14.0);

            // Elevated circular backdrop canvas (borderless)
            let preview_size = (cfg.appearance.outer_radius + 20.0) * 2.0;

            ui.allocate_ui_with_layout(
                vec2(preview_size, preview_size),
                Layout::top_down(Align::Center),
                |ui| {
                    let canvas_rect = ui.max_rect();
                    let painter = ui.painter();

                    // Dark circular backdrop without border
                    painter.circle_filled(
                        canvas_rect.center(),
                        cfg.appearance.outer_radius + 15.0,
                        surface_container_color(widget_op),
                    );

                    let segments: Vec<WheelSegment> = cfg
                        .formats
                        .items
                        .iter()
                        .filter(|i| i.enabled)
                        .map(|i| WheelSegment {
                            id: i.format.extension().to_string(),
                            label: i.label.clone(),
                            enabled: true,
                            available: true,
                            unavailable_reason: None,
                        })
                        .collect();

                    let wheel_op = cfg.appearance.wheel_opacity.clamp(0.2, 1.0);

                    let style = WheelStyle {
                        outer_radius: cfg.appearance.outer_radius,
                        inner_radius: cfg.appearance.inner_radius,
                        gap_width: cfg.appearance.gap_width,
                        segment_color: Color32::from_rgba_premultiplied(
                            ((cfg.appearance.segment_color[0] as f32) * wheel_op).round() as u8,
                            ((cfg.appearance.segment_color[1] as f32) * wheel_op).round() as u8,
                            ((cfg.appearance.segment_color[2] as f32) * wheel_op).round() as u8,
                            ((cfg.appearance.segment_color[3] as f32) * wheel_op).round() as u8,
                        ),
                        hover_color: Color32::from_rgba_premultiplied(
                            ((cfg.appearance.hover_color[0] as f32) * wheel_op).round() as u8,
                            ((cfg.appearance.hover_color[1] as f32) * wheel_op).round() as u8,
                            ((cfg.appearance.hover_color[2] as f32) * wheel_op).round() as u8,
                            ((cfg.appearance.hover_color[3] as f32) * wheel_op).round() as u8,
                        ),
                        text_color: Color32::from_rgba_premultiplied(
                            cfg.appearance.text_color[0],
                            cfg.appearance.text_color[1],
                            cfg.appearance.text_color[2],
                            cfg.appearance.text_color[3],
                        ),
                        disabled_color: Color32::from_gray(50),
                        gap_color: Color32::from_rgba_premultiplied(
                            cfg.appearance.gap_color[0],
                            cfg.appearance.gap_color[1],
                            cfg.appearance.gap_color[2],
                            cfg.appearance.gap_color[3],
                        ),
                        font_size: cfg.appearance.font_size,
                        font_bold: cfg.appearance.font_bold,
                        animation_speed: cfg.appearance.animation_speed,
                    };

                    let widget = WheelWidget::new(&segments)
                        .style(style)
                        .center_text(Some("Preview".to_string()));

                    widget.show(ui);
                },
            );

            ui.add_space(10.0);
            ui.label(
                RichText::new("Hover over segments to test interactive animations")
                    .size(11.0)
                    .color(MD_TEXT_SECONDARY),
            );
        });
    });
}

fn render_snackbar(ui: &mut Ui, msg: &str) {
    let snackbar_frame = egui::Frame::NONE
        .fill(MD_PRIMARY_CONTAINER)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(egui::Margin::symmetric(16, 8));

    snackbar_frame.show(ui, |ui| {
        ui.label(RichText::new(msg).size(12.0).color(MD_PRIMARY));
    });
}
