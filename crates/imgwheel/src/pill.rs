use config::PillConfig;
use eframe::egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};
use eframe::{App, Frame};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum BatchProgress {
    Progress {
        current: usize,
        total: usize,
        target_name: String,
    },
    Done {
        success: usize,
        failure: usize,
        total: usize,
    },
    Cancelled,
}

pub struct PillApp {
    pub config: PillConfig,
    pub rx: Receiver<BatchProgress>,
    pub cancel_flag: Arc<AtomicBool>,
    pub current: usize,
    pub total: usize,
    pub target_name: String,
    pub is_finished: bool,
    pub is_cancelled: bool,
    pub success_count: usize,
    pub failure_count: usize,
    pub finish_time: Option<Instant>,
    pub should_close: bool,
}

impl PillApp {
    pub fn new(
        config: PillConfig,
        rx: Receiver<BatchProgress>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config,
            rx,
            cancel_flag,
            current: 0,
            total: 1,
            target_name: "Images".to_string(),
            is_finished: false,
            is_cancelled: false,
            success_count: 0,
            failure_count: 0,
            finish_time: None,
            should_close: false,
        }
    }
}

impl App for PillApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();

        // Drain incoming progress updates
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BatchProgress::Progress {
                    current,
                    total,
                    target_name,
                } => {
                    self.current = current;
                    self.total = total;
                    self.target_name = target_name;
                }
                BatchProgress::Done {
                    success,
                    failure,
                    total,
                } => {
                    self.is_finished = true;
                    self.success_count = success;
                    self.failure_count = failure;
                    self.total = total;
                    self.finish_time = Some(Instant::now());
                }
                BatchProgress::Cancelled => {
                    self.is_cancelled = true;
                    self.is_finished = true;
                    self.finish_time = Some(Instant::now());
                }
            }
        }

        // Auto-close check
        if let Some(finished_at) = self.finish_time {
            if finished_at.elapsed().as_secs_f32() >= self.config.auto_close_delay_secs {
                self.should_close = true;
            }
        }

        if self.should_close {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            return;
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        self.render_pill(ui);
    }

    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl PillApp {
    fn render_pill(&mut self, ui: &mut Ui) {
        let pill_rect = Rect::from_center_size(
            ui.available_rect_before_wrap().center(),
            vec2(self.config.width, self.config.height),
        );

        let response = ui.allocate_rect(pill_rect, Sense::click());
        if response.clicked() && self.is_finished {
            self.should_close = true;
        }

        let painter = ui.painter();
        let bg_col = Color32::from_rgba_premultiplied(
            self.config.bg_color[0],
            self.config.bg_color[1],
            self.config.bg_color[2],
            self.config.bg_color[3],
        );
        let accent_col = Color32::from_rgba_premultiplied(
            self.config.accent_color[0],
            self.config.accent_color[1],
            self.config.accent_color[2],
            self.config.accent_color[3],
        );
        let text_col = Color32::from_rgba_premultiplied(
            self.config.text_color[0],
            self.config.text_color[1],
            self.config.text_color[2],
            self.config.text_color[3],
        );

        let corner_radius = CornerRadius::same((self.config.height / 2.0).round() as u8);

        // Background capsule
        painter.rect_filled(pill_rect, corner_radius, bg_col);

        // Progress fill inside capsule
        let fraction = if self.total > 0 {
            (self.current as f32 / self.total as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        if fraction > 0.01 {
            let progress_width = (pill_rect.width() * fraction).max(self.config.height);
            let progress_rect = Rect::from_min_size(
                pill_rect.min,
                vec2(progress_width.min(pill_rect.width()), pill_rect.height()),
            );
            let progress_color = Color32::from_rgba_premultiplied(
                accent_col.r() / 2,
                accent_col.g() / 2,
                accent_col.b() / 2,
                160,
            );
            painter.rect_filled(progress_rect, corner_radius, progress_color);
        }

        // Border outline
        painter.rect_stroke(
            pill_rect,
            corner_radius,
            Stroke::new(1.0, Color32::from_white_alpha(40)),
            eframe::egui::StrokeKind::Outside,
        );

        // Status text
        let status_str = if self.is_cancelled {
            "Conversion cancelled".to_string()
        } else if self.is_finished {
            if self.failure_count == 0 {
                format!("Done: {} converted", self.success_count)
            } else {
                format!(
                    "Done: {} converted, {} failed",
                    self.success_count, self.failure_count
                )
            }
        } else {
            format!(
                "Converting {} / {} \u{2192} {}",
                self.current, self.total, self.target_name
            )
        };

        let text_pos = pos2(pill_rect.min.x + 20.0, pill_rect.center().y);
        painter.text(
            text_pos,
            Align2::LEFT_CENTER,
            status_str,
            FontId::proportional(14.0),
            text_col,
        );

        // Cancel button (X) on the right side if not finished
        if !self.is_finished {
            let cancel_size = 24.0;
            let cancel_rect = Rect::from_center_size(
                pos2(pill_rect.max.x - 22.0, pill_rect.center().y),
                vec2(cancel_size, cancel_size),
            );

            let cancel_hover = ui
                .input(|i| i.pointer.hover_pos())
                .map(|p| cancel_rect.contains(p))
                .unwrap_or(false);

            if cancel_hover {
                painter.circle_filled(cancel_rect.center(), 12.0, Color32::from_white_alpha(40));
            }

            painter.text(
                cancel_rect.center(),
                Align2::CENTER_CENTER,
                "\u{2715}",
                FontId::proportional(12.0),
                if cancel_hover {
                    Color32::WHITE
                } else {
                    Color32::from_gray(180)
                },
            );

            if ui.input(|i| {
                i.pointer
                    .button_clicked(eframe::egui::PointerButton::Primary)
            }) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if cancel_rect.contains(pos) {
                        self.cancel_flag.store(true, Ordering::SeqCst);
                    }
                }
            }
        }
    }
}
