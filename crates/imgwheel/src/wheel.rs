use egui::{
    Color32, Key, Mesh, Painter, Pos2, Sense, Shape, Stroke, Ui, Vec2,
    epaint::{FontId, TextShape},
    pos2, vec2,
};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelHit {
    None,
    Outside,
    CenterHole,
    Segment(usize),
}

#[derive(Debug, Clone)]
pub struct WheelSegment {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}

pub struct WheelStyle {
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub gap_width: f32,
    pub segment_color: Color32,
    pub hover_color: Color32,
    pub text_color: Color32,
    pub disabled_color: Color32,
    pub gap_color: Color32,
    pub font_size: f32,
    pub font_bold: bool,
    pub animation_speed: f32,
}

impl Default for WheelStyle {
    fn default() -> Self {
        Self {
            outer_radius: 170.0,
            inner_radius: 48.0,
            gap_width: 3.0,
            segment_color: Color32::from_rgb(77, 77, 77),
            hover_color: Color32::from_rgb(59, 130, 246),
            text_color: Color32::WHITE,
            disabled_color: Color32::from_rgb(50, 50, 50),
            gap_color: Color32::WHITE,
            font_size: 16.0,
            font_bold: true,
            animation_speed: 0.15,
        }
    }
}

pub fn normalize_angle(angle: f32) -> f32 {
    let two_pi = 2.0 * PI;
    let rem = angle % two_pi;
    if rem < 0.0 { rem + two_pi } else { rem }
}

pub fn hit_test_polar(
    center: Pos2,
    point: Pos2,
    inner_r: f32,
    outer_r: f32,
    num_segments: usize,
) -> WheelHit {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < inner_r {
        return WheelHit::CenterHole;
    }
    if dist > outer_r {
        return WheelHit::Outside;
    }
    if num_segments == 0 {
        return WheelHit::None;
    }

    // Angle theta from -PI to PI
    let theta = dy.atan2(dx);
    // Align so 12 o'clock (-PI / 2) is 0
    let start_offset = -PI / 2.0;
    let norm_theta = normalize_angle(theta - start_offset);

    let slice_angle = (2.0 * PI) / (num_segments as f32);
    let index = (norm_theta / slice_angle).floor() as usize;
    let clamped_index = index.min(num_segments - 1);

    WheelHit::Segment(clamped_index)
}

pub struct WheelWidget<'a> {
    pub segments: &'a [WheelSegment],
    pub style: WheelStyle,
    pub selected_index: Option<usize>,
    pub center_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WheelAction {
    pub clicked_segment: Option<usize>,
    pub hovered_segment: Option<usize>,
    pub cancel_clicked: bool,
    pub outside_clicked: bool,
}

impl<'a> WheelWidget<'a> {
    pub fn new(segments: &'a [WheelSegment]) -> Self {
        Self {
            segments,
            style: WheelStyle::default(),
            selected_index: None,
            center_text: None,
        }
    }

    pub fn style(mut self, style: WheelStyle) -> Self {
        self.style = style;
        self
    }

    pub fn selected_index(mut self, idx: Option<usize>) -> Self {
        self.selected_index = idx;
        self
    }

    pub fn center_text(mut self, text: Option<String>) -> Self {
        self.center_text = text;
        self
    }

    pub fn show(self, ui: &mut Ui) -> WheelAction {
        let num_segments = self.segments.len();
        let total_size = Vec2::splat((self.style.outer_radius + 10.0) * 2.0);
        let (rect, response) = ui.allocate_exact_size(total_size, Sense::click_and_drag());

        let center = rect.center();
        let mouse_pos = ui.input(|i| i.pointer.hover_pos());

        let mut hit = WheelHit::None;
        if let Some(pos) = mouse_pos {
            hit = hit_test_polar(
                center,
                pos,
                self.style.inner_radius,
                self.style.outer_radius,
                num_segments,
            );
        }

        let mut hovered_segment = match hit {
            WheelHit::Segment(idx) => Some(idx),
            _ => self.selected_index,
        };

        // Handle keyboard navigation
        let mut key_activated_segment = None;
        let mut cancel = false;

        ui.input(|i| {
            if i.key_pressed(Key::Escape) {
                cancel = true;
            }
            if i.key_pressed(Key::Enter) {
                if let Some(h) = hovered_segment {
                    key_activated_segment = Some(h);
                }
            }
            if i.key_pressed(Key::Tab)
                || i.key_pressed(Key::ArrowRight)
                || i.key_pressed(Key::ArrowDown)
            {
                if num_segments > 0 {
                    let next = match hovered_segment {
                        Some(curr) => (curr + 1) % num_segments,
                        None => 0,
                    };
                    hovered_segment = Some(next);
                }
            }
            if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowUp) {
                if num_segments > 0 {
                    let prev = match hovered_segment {
                        Some(curr) => (curr + num_segments - 1) % num_segments,
                        None => num_segments - 1,
                    };
                    hovered_segment = Some(prev);
                }
            }
            // Number keys 1..9
            for n in 1..=9 {
                let key = match n {
                    1 => Key::Num1,
                    2 => Key::Num2,
                    3 => Key::Num3,
                    4 => Key::Num4,
                    5 => Key::Num5,
                    6 => Key::Num6,
                    7 => Key::Num7,
                    8 => Key::Num8,
                    9 => Key::Num9,
                    _ => Key::Num0,
                };
                if i.key_pressed(key) && n - 1 < num_segments {
                    key_activated_segment = Some(n - 1);
                }
            }
        });

        let mut clicked_segment = key_activated_segment;
        let mut cancel_clicked = cancel;
        let mut outside_clicked = false;

        if response.clicked() {
            match hit {
                WheelHit::Segment(idx) => {
                    clicked_segment = Some(idx);
                }
                WheelHit::CenterHole => {
                    cancel_clicked = true;
                }
                WheelHit::Outside => {
                    outside_clicked = true;
                }
                WheelHit::None => {}
            }
        }

        self.draw(ui, center, hovered_segment);

        WheelAction {
            clicked_segment,
            hovered_segment,
            cancel_clicked,
            outside_clicked,
        }
    }

    fn draw(&self, ui: &mut Ui, center: Pos2, hovered_segment: Option<usize>) {
        let painter = ui.painter();
        let num_segments = self.segments.len();
        if num_segments == 0 {
            return;
        }

        let slice_angle = (2.0 * PI) / (num_segments as f32);
        let start_angle_offset = -PI / 2.0;

        // Draw each annular segment
        for (i, seg) in self.segments.iter().enumerate() {
            let is_hovered = hovered_segment == Some(i);
            let is_available = seg.available && seg.enabled;

            // Animate hover state smoothly
            let anim_id = ui.make_persistent_id(("wheel_seg_hover", i));
            let hover_factor = ui.ctx().animate_value_with_time(
                anim_id,
                if is_hovered { 1.0 } else { 0.0 },
                self.style.animation_speed,
            );

            let base_color = if !is_available {
                self.style.disabled_color
            } else {
                self.style.segment_color
            };

            // Interpolate color towards hover accent
            let seg_color = if is_available {
                lerp_color(base_color, self.style.hover_color, hover_factor)
            } else {
                base_color
            };

            let theta_start = start_angle_offset + (i as f32) * slice_angle;
            let theta_end = theta_start + slice_angle;
            let theta_mid = (theta_start + theta_end) / 2.0;

            // Segment expansion on hover
            let current_outer_r = self.style.outer_radius + (hover_factor * 4.0);
            let current_inner_r = self.style.inner_radius;

            // Angular gap
            let angular_gap = (self.style.gap_width / current_outer_r).min(slice_angle * 0.2);
            let half_gap = angular_gap / 2.0;

            let arc_start = theta_start + half_gap;
            let arc_end = theta_end - half_gap;

            // Draw annular sector
            draw_annular_sector(
                painter,
                center,
                current_inner_r,
                current_outer_r,
                arc_start,
                arc_end,
                seg_color,
            );

            // Draw radial label
            let r_label = (current_inner_r + current_outer_r) / 2.0;
            let label_center = pos2(
                center.x + r_label * theta_mid.cos(),
                center.y + r_label * theta_mid.sin(),
            );

            let font_id = FontId::proportional(self.style.font_size * (1.0 + hover_factor * 0.08));

            let text_color = if !is_available {
                Color32::from_gray(120)
            } else {
                self.style.text_color
            };

            // Determine text rotation angle along radial direction
            // Flip if on left side so letters never read upside down
            let angle = if theta_mid.cos() < 0.0 {
                theta_mid + PI
            } else {
                theta_mid
            };

            draw_centered_rotated_text(
                painter,
                &seg.label,
                font_id,
                label_center,
                angle,
                text_color,
            );
        }

        // Draw center hole and hint text
        let center_hovered = hovered_segment.and_then(|idx| self.segments.get(idx));
        if let Some(seg) = center_hovered {
            let hint = if !seg.available {
                seg.unavailable_reason.as_deref().unwrap_or("Unavailable")
            } else {
                self.center_text.as_deref().unwrap_or(&seg.label)
            };
            let font = FontId::proportional(12.0);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                hint,
                font,
                Color32::from_gray(210),
            );
        } else if let Some(ref text) = self.center_text {
            let font = FontId::proportional(13.0);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                text,
                font,
                Color32::from_gray(200),
            );
        }
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv_t = 1.0 - t;
    Color32::from_rgba_premultiplied(
        (a.r() as f32 * inv_t + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * inv_t + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * inv_t + b.b() as f32 * t).round() as u8,
        (a.a() as f32 * inv_t + b.a() as f32 * t).round() as u8,
    )
}

fn draw_annular_sector(
    painter: &Painter,
    center: Pos2,
    inner_r: f32,
    outer_r: f32,
    start_angle: f32,
    end_angle: f32,
    fill_color: Color32,
) {
    let steps = 24;
    let angle_span = end_angle - start_angle;
    let step_angle = angle_span / (steps as f32);

    let mut mesh = Mesh::default();

    for i in 0..=steps {
        let theta = start_angle + (i as f32) * step_angle;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let inner_pt = pos2(center.x + inner_r * cos_t, center.y + inner_r * sin_t);
        let outer_pt = pos2(center.x + outer_r * cos_t, center.y + outer_r * sin_t);

        mesh.colored_vertex(inner_pt, fill_color);
        mesh.colored_vertex(outer_pt, fill_color);

        if i > 0 {
            let curr_inner = (i * 2) as u32;
            let curr_outer = curr_inner + 1;
            let prev_inner = curr_inner - 2;
            let prev_outer = curr_inner - 1;

            mesh.add_triangle(prev_inner, prev_outer, curr_outer);
            mesh.add_triangle(prev_inner, curr_outer, curr_inner);
        }
    }

    painter.add(Shape::mesh(mesh));
}

fn draw_centered_rotated_text(
    painter: &Painter,
    text: &str,
    font_id: FontId,
    center: Pos2,
    angle: f32,
    color: Color32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font_id, color);
    let size = galley.size();

    // Center offset
    let origin_offset = vec2(-size.x / 2.0, -size.y / 2.0);
    // Rotate offset around center
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let rotated_offset = vec2(
        origin_offset.x * cos_a - origin_offset.y * sin_a,
        origin_offset.x * sin_a + origin_offset.y * cos_a,
    );

    let text_pos = center + rotated_offset;

    let shape = TextShape {
        pos: text_pos,
        galley,
        underline: Stroke::NONE,
        fallback_color: color,
        override_text_color: Some(color),
        opacity_factor: 1.0,
        angle,
    };

    painter.add(Shape::Text(shape));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polar_hit_testing_segments() {
        let center = pos2(200.0, 200.0);
        let inner_r = 50.0;
        let outer_r = 150.0;
        let num_segments = 8; // BMP (top), PNG, WebP, HEIC, HEIF, EPS, JPG, AVIF

        // 1. Center hole test
        let hit_center = hit_test_polar(center, pos2(200.0, 210.0), inner_r, outer_r, num_segments);
        assert_eq!(hit_center, WheelHit::CenterHole);

        // 2. Outside test
        let hit_outside =
            hit_test_polar(center, pos2(200.0, 400.0), inner_r, outer_r, num_segments);
        assert_eq!(hit_outside, WheelHit::Outside);

        // 3. Segment 0 (BMP, clockwise starting from 12 o'clock / -PI/2)
        // Angle 0..45 deg from 12 o'clock -> e.g. at 22.5 deg (up and slightly right)
        // dx = 100 * sin(22.5°), dy = -100 * cos(22.5°)
        let theta_mid_0 = -PI / 2.0 + (PI / 8.0);
        let pt0 = pos2(
            center.x + 100.0 * theta_mid_0.cos(),
            center.y + 100.0 * theta_mid_0.sin(),
        );
        let hit_seg0 = hit_test_polar(center, pt0, inner_r, outer_r, num_segments);
        assert_eq!(hit_seg0, WheelHit::Segment(0));

        // 4. Segment 1 (PNG, around 45° to 90° from 12 o'clock, mid is 67.5°)
        let theta_mid_1 = -PI / 2.0 + (3.0 * PI / 8.0);
        let pt1 = pos2(
            center.x + 100.0 * theta_mid_1.cos(),
            center.y + 100.0 * theta_mid_1.sin(),
        );
        let hit_seg1 = hit_test_polar(center, pt1, inner_r, outer_r, num_segments);
        assert_eq!(hit_seg1, WheelHit::Segment(1));

        // 5. Segment 7 (AVIF, last segment before 12 o'clock, top left)
        let theta_mid_7 = -PI / 2.0 + (15.0 * PI / 8.0);
        let pt7 = pos2(
            center.x + 100.0 * theta_mid_7.cos(),
            center.y + 100.0 * theta_mid_7.sin(),
        );
        let hit_seg7 = hit_test_polar(center, pt7, inner_r, outer_r, num_segments);
        assert_eq!(hit_seg7, WheelHit::Segment(7));
    }

    #[test]
    fn test_normalize_angle() {
        assert!((normalize_angle(0.0) - 0.0).abs() < 1e-6);
        assert!((normalize_angle(2.0 * PI) - 0.0).abs() < 1e-6);
        assert!((normalize_angle(-PI / 2.0) - (1.5 * PI)).abs() < 1e-6);
    }
}
