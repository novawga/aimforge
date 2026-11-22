//! Small immediate-mode UI toolkit. All colours come from the active theme;
//! all sizes are in 1080p units and scaled to the window.

use crate::theme::{self, mix, with_alpha};
use macroquad::prelude::*;

/// Type scale (1080p px).
pub mod size {
    pub const DISPLAY: f32 = 56.0;
    pub const TITLE: f32 = 26.0;
    pub const HEADING: f32 = 17.0;
    pub const BODY: f32 = 15.0;
    pub const SMALL: f32 = 13.0;
    pub const CAPTION: f32 = 12.0;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
}

pub struct Ui {
    pub font: Option<Font>,
    pub bold: Option<Font>,
    pub scale: f32,
    pub mouse: Vec2,
    pub clicked: bool,
    pub down: bool,
    pub wheel: f32,
    pub chars: Vec<char>,
    pub focus: Option<u64>,
    /// A text field had focus when this frame began.
    pub focus_at_begin: bool,
    pub buf: String,
    pub drag: Option<u64>,
    pub consumed: bool,
    pub hand: bool,
    /// Widget clicks this frame (for UI sounds).
    pub clicks: u32,
}

fn try_font(paths: &[&str]) -> Option<Font> {
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            if let Ok(mut f) = load_ttf_font_from_bytes(&bytes) {
                f.set_filter(FilterMode::Linear);
                return Some(f);
            }
        }
    }
    None
}

impl Ui {
    pub fn new(asset_dir: &std::path::Path) -> Ui {
        let custom = asset_dir.join("font.ttf").to_string_lossy().to_string();
        let custom_b = asset_dir.join("font-bold.ttf").to_string_lossy().to_string();
        let font = try_font(&[
            &custom,
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        ]);
        let bold = try_font(&[
            &custom_b,
            "C:\\Windows\\Fonts\\seguisb.ttf",
            "C:\\Windows\\Fonts\\segoeuib.ttf",
            "C:\\Windows\\Fonts\\arialbd.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/usr/share/fonts/truetype/inter/Inter-SemiBold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ])
        .or_else(|| font.clone());
        Ui {
            font,
            bold,
            scale: 1.0,
            mouse: Vec2::ZERO,
            clicked: false,
            down: false,
            wheel: 0.0,
            chars: vec![],
            focus: None,
            focus_at_begin: false,
            buf: String::new(),
            drag: None,
            consumed: false,
            hand: false,
            clicks: 0,
        }
    }

    pub fn begin(&mut self) {
        self.focus_at_begin = self.focus.is_some();
        self.scale = (screen_height() / 1080.0).clamp(0.5, 3.0);
        self.mouse = mouse_position().into();
        self.clicked = is_mouse_button_pressed(MouseButton::Left);
        self.down = is_mouse_button_down(MouseButton::Left);
        self.wheel = mouse_wheel().1;
        self.chars.clear();
        while let Some(c) = get_char_pressed() {
            self.chars.push(c);
        }
        self.consumed = false;
        self.hand = false;
        self.clicks = 0;
        if !self.down {
            self.drag = None;
        }
    }

    /// True while the user is typing in a text field (this frame or the start of it),
    /// so screen hotkeys must stay quiet.
    pub fn typing(&self) -> bool {
        self.focus_at_begin || self.focus.is_some()
    }

    pub fn s(&self, v: f32) -> f32 {
        (v * self.scale).round()
    }

    pub fn hover(&self, r: Rect) -> bool {
        r.contains(self.mouse)
    }

    pub fn click_in(&mut self, r: Rect) -> bool {
        if self.clicked && !self.consumed && r.contains(self.mouse) {
            self.consumed = true;
            self.clicks += 1;
            true
        } else {
            false
        }
    }

    // ---------------------------------------------------------------- text

    fn fs(&self, size: f32) -> u16 {
        (size * self.scale).round().clamp(6.0, 400.0) as u16
    }

    fn face(&self, bold: bool) -> Option<&Font> {
        if bold {
            self.bold.as_ref()
        } else {
            self.font.as_ref()
        }
    }

    pub fn measure(&self, t: &str, size: f32, bold: bool) -> f32 {
        measure_text(t, self.face(bold), self.fs(size), 1.0).width
    }

    /// Text with its top-left at (x, y). Returns the width.
    pub fn text_ex(&self, t: &str, x: f32, y: f32, size: f32, color: Color, bold: bool) -> f32 {
        let fs = self.fs(size);
        let baseline = y + fs as f32 * 0.8;
        draw_text_ex(t, x.round(), baseline.round(), TextParams { font: self.face(bold), font_size: fs, color, ..Default::default() })
            .width
    }

    pub fn text(&self, t: &str, x: f32, y: f32, size: f32, color: Color) -> f32 {
        self.text_ex(t, x, y, size, color, false)
    }

    pub fn strong(&self, t: &str, x: f32, y: f32, size: f32, color: Color) -> f32 {
        self.text_ex(t, x, y, size, color, true)
    }

    pub fn text_right(&self, t: &str, rx: f32, y: f32, size: f32, color: Color, bold: bool) {
        let w = self.measure(t, size, bold);
        self.text_ex(t, rx - w, y, size, color, bold);
    }

    pub fn text_center(&self, t: &str, cx: f32, y: f32, size: f32, color: Color, bold: bool) {
        let w = self.measure(t, size, bold);
        self.text_ex(t, cx - w * 0.5, y, size, color, bold);
    }

    /// Vertically centre a single line in a rect.
    pub fn text_mid(&self, t: &str, x: f32, r: Rect, size: f32, color: Color, bold: bool) -> f32 {
        self.text_ex(t, x, r.y + (r.h - self.s(size) * 1.0) * 0.5 - self.s(1.0), size, color, bold)
    }

    pub fn fit(&self, t: &str, size: f32, max_w: f32, bold: bool) -> String {
        if self.measure(t, size, bold) <= max_w {
            return t.to_string();
        }
        let mut s: Vec<char> = t.chars().collect();
        while !s.is_empty() {
            s.pop();
            let c: String = s.iter().collect::<String>().trim_end().to_string() + "...";
            if self.measure(&c, size, bold) <= max_w {
                return c;
            }
        }
        String::new()
    }

    pub fn wrap(&self, text: &str, size: f32, max_w: f32, bold: bool) -> Vec<String> {
        let mut lines = vec![];
        let mut cur = String::new();
        for word in text.split_whitespace() {
            let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
            if self.measure(&cand, size, bold) > max_w && !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                cur = word.to_string();
            } else {
                cur = cand;
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        lines
    }

    // -------------------------------------------------------------- shapes

    pub fn card(&self, r: Rect) {
        let t = theme::t();
        rrect(r, self.s(4.0), t.surface);
    }

    pub fn divider(&self, x: f32, y: f32, w: f32) {
        draw_rectangle(x, y.round(), w, self.s(1.0).max(1.0), theme::t().border);
    }

    // ------------------------------------------------------------- widgets

    pub fn button(&mut self, r: Rect, label: &str, kind: Kind) -> bool {
        let t = theme::t();
        let h = self.hover(r);
        if h {
            self.hand = true;
        }
        let rad = self.s(3.0);
        let (bg, fg) = match kind {
            Kind::Primary => (if h { mix(t.accent, t.text, 0.12) } else { t.accent }, t.accent_text),
            Kind::Secondary => (if h { t.surface_active } else { t.surface_hover }, t.text),
            Kind::Ghost => (if h { t.surface_hover } else { with_alpha(t.surface_hover, 0.0) }, if h { t.text } else { t.text_dim }),
        };
        if bg.a > 0.0 {
            rrect(r, rad, bg);
        }
        let size = Self::button_font(r.h / self.scale);
        let label = self.fit(label, size, r.w - self.s(20.0), true);
        let w = self.measure(&label, size, true);
        self.text_mid(&label, r.x + (r.w - w) * 0.5, r, size, fg, true);
        self.click_in(r)
    }

    pub fn button_font(h_1080: f32) -> f32 {
        (h_1080 * 0.36).clamp(12.0, 18.0)
    }

    /// Width a button needs for its label at the given height (1080p units).
    pub fn button_w(&self, label: &str, h_1080: f32) -> f32 {
        self.measure(label, Self::button_font(h_1080), true) + self.s(36.0)
    }

    /// Top-level navigation item; returns (clicked, width).
    pub fn nav(&mut self, x: f32, y: f32, h: f32, label: &str, active: bool) -> (bool, f32) {
        let t = theme::t();
        let w = self.measure(label, size::BODY, true) + self.s(28.0);
        let r = Rect::new(x, y, w, h);
        let hov = self.hover(r);
        let col = if active { t.text } else if hov { t.text } else { t.text_dim };
        self.text_mid(label, x + self.s(14.0), r, size::BODY, col, active);
        if active {
            rrect(Rect::new(x + self.s(14.0), y + h - self.s(3.0), w - self.s(28.0), self.s(3.0)), self.s(1.5), t.accent);
        }
        (self.click_in(r), w)
    }

    /// Pill segmented control. Returns the clicked index.
    pub fn segmented(&mut self, r: Rect, options: &[&str], idx: usize) -> Option<usize> {
        let t = theme::t();
        rrect(r, self.s(4.0), t.surface_hover);
        let pad = self.s(3.0);
        let n = options.len().max(1);
        let total_text: f32 = options.iter().map(|o| self.measure(o, size::SMALL, true)).sum();
        let extra = ((r.w - 2.0 * pad) - total_text).max(0.0) / n as f32;
        let mut x = r.x + pad;
        let mut out = None;
        for (i, o) in options.iter().enumerate() {
            let w = self.measure(o, size::SMALL, true) + extra;
            let seg = Rect::new(x, r.y + pad, w, r.h - 2.0 * pad);
            let active = i == idx;
            if active {
                rrect(seg, self.s(3.0), t.surface_active);
            } else if self.hover(seg) {
                self.hand = true;
            }
            let col = if active { t.text } else { t.text_dim };
            let tw = self.measure(o, size::SMALL, true);
            self.text_mid(o, seg.x + (seg.w - tw) * 0.5, seg, size::SMALL, col, true);
            if self.click_in(seg) {
                out = Some(i);
            }
            x += w;
        }
        out
    }

    /// Small selectable pill. Returns true when clicked.
    pub fn chip(&mut self, r: Rect, label: &str, active: bool) -> bool {
        let t = theme::t();
        let h = self.hover(r);
        if h {
            self.hand = true;
        }
        let bg = if active { with_alpha(t.accent, 0.18) } else if h { t.surface_active } else { t.surface_hover };
        rrect(r, self.s(3.0), bg);
        if active {
            let b = self.s(1.5).max(1.0);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, b * 2.0, t.accent);
        }
        let label = self.fit(label, size::SMALL, r.w - self.s(12.0), active);
        let w = self.measure(&label, size::SMALL, active);
        self.text_mid(&label, r.x + (r.w - w) * 0.5, r, size::SMALL, if active { t.text } else { t.text_dim }, active);
        self.click_in(r)
    }

    /// Text with a soft drop shadow (HUD over the 3D view).
    pub fn shadow_text(&self, t: &str, cx: f32, y: f32, size: f32, color: Color, bold: bool) {
        let w = self.measure(t, size, bold);
        let o = self.s(1.5).max(1.0);
        self.text_ex(t, cx - w * 0.5 + o, y + o, size, Color::new(0.0, 0.0, 0.0, 0.55 * color.a), bold);
        self.text_ex(t, cx - w * 0.5, y, size, color, bold);
    }

    /// iOS-style switch on the right edge of `r`. Returns true if toggled.
    pub fn switch(&mut self, r: Rect, v: &mut bool) -> bool {
        let t = theme::t();
        let w = self.s(38.0);
        let h = self.s(22.0);
        let sr = Rect::new(r.x + r.w - w, r.y + (r.h - h) * 0.5, w, h);
        rrect(sr, h * 0.5, if *v { t.accent } else { t.surface_active });
        let k = h - self.s(6.0);
        let kx = if *v { sr.x + sr.w - k - self.s(3.0) } else { sr.x + self.s(3.0) };
        draw_circle(kx + k * 0.5, sr.y + sr.h * 0.5, k * 0.5, if *v { t.accent_text } else { t.text_dim });
        if self.hover(r) {
            self.hand = true;
        }
        if self.click_in(r) {
            *v = !*v;
            true
        } else {
            false
        }
    }

    pub fn slider(&mut self, id: u64, r: Rect, v: &mut f32, min: f32, max: f32, step: f32) -> bool {
        let t = theme::t();
        let th = self.s(4.0);
        let ty = r.y + (r.h - th) * 0.5;
        rrect(Rect::new(r.x, ty, r.w, th), th * 0.5, t.surface_active);
        let k = ((*v - min) / (max - min)).clamp(0.0, 1.0);
        rrect(Rect::new(r.x, ty, (r.w * k).max(th), th), th * 0.5, t.accent);
        let kx = r.x + r.w * k;
        let knob = self.s(8.0);
        let hit = Rect::new(r.x - knob, r.y, r.w + 2.0 * knob, r.h);
        let active = self.drag == Some(id);
        draw_circle(kx, r.y + r.h * 0.5, if active || self.hover(hit) { knob * 1.15 } else { knob }, t.text);
        if self.hover(hit) {
            self.hand = true;
        }
        if self.clicked && self.hover(hit) && !self.consumed {
            self.drag = Some(id);
            self.consumed = true;
        }
        if self.drag == Some(id) {
            let f = ((self.mouse.x - r.x) / r.w).clamp(0.0, 1.0);
            let mut nv = min + (max - min) * f;
            if step > 0.0 {
                nv = (nv / step).round() * step;
            }
            let changed = (nv - *v).abs() > f32::EPSILON;
            *v = nv;
            return changed;
        }
        false
    }

    /// Text input with Ctrl+V paste. Returns true on Enter or when focus leaves.
    pub fn text_field(&mut self, id: u64, r: Rect, value: &mut String, placeholder: &str) -> bool {
        self.text_field_ex(id, r, value, placeholder, false)
    }

    /// Text input that shows dots instead of the characters.
    pub fn password_field(&mut self, id: u64, r: Rect, value: &mut String, placeholder: &str) -> bool {
        self.text_field_ex(id, r, value, placeholder, true)
    }

    pub fn text_field_ex(&mut self, id: u64, r: Rect, value: &mut String, placeholder: &str, mask: bool) -> bool {
        let t = theme::t();
        let focused = self.focus == Some(id);
        let rad = self.s(3.0);
        if focused {
            let g = self.s(2.0);
            rrect(Rect::new(r.x - g, r.y - g, r.w + 2.0 * g, r.h + 2.0 * g), rad + g, with_alpha(t.accent, 0.6));
        }
        rrect(r, rad, t.surface_active);
        if self.hover(r) {
            self.hand = true;
        }
        let mut submitted = false;
        if self.click_in(r) && !focused {
            self.focus = Some(id);
        } else if self.clicked && !self.hover(r) && focused {
            self.focus = None;
            submitted = true;
        }
        if self.focus == Some(id) {
            let ctrl = is_key_down(KeyCode::LeftControl)
                || is_key_down(KeyCode::RightControl)
                || is_key_down(KeyCode::LeftSuper)
                || is_key_down(KeyCode::RightSuper);
            for c in self.chars.clone() {
                if !c.is_control() && !ctrl {
                    value.push(c);
                }
            }
            if ctrl && is_key_pressed(KeyCode::V) {
                if let Some(clip) = miniquad::window::clipboard_get() {
                    value.extend(clip.chars().filter(|c| !c.is_control()));
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                if ctrl {
                    value.clear();
                } else {
                    value.pop();
                }
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                self.focus = None;
                submitted = true;
            }
            if is_key_pressed(KeyCode::Escape) {
                self.focus = None;
            }
        }
        let pad = self.s(12.0);
        if value.is_empty() && self.focus != Some(id) {
            self.text_mid(placeholder, r.x + pad, r, size::BODY, t.text_faint, false);
        } else {
            let src = if mask { "\u{2022}".repeat(value.chars().count()) } else { value.clone() };
            let shown = self.fit_tail(&src, size::BODY, r.w - pad * 2.0 - self.s(6.0));
            let w = self.text_mid(&shown, r.x + pad, r, size::BODY, t.text, false);
            if self.focus == Some(id) && (get_time() * 1.6).fract() < 0.6 {
                let ch = self.s(size::BODY + 3.0);
                draw_rectangle(r.x + pad + w + self.s(1.0), r.y + (r.h - ch) * 0.5, self.s(1.5).max(1.0), ch, t.accent);
            }
        }
        submitted
    }

    fn fit_tail(&self, t: &str, size: f32, max_w: f32) -> String {
        let chars: Vec<char> = t.chars().collect();
        let mut start = 0;
        while start < chars.len() && self.measure(&chars[start..].iter().collect::<String>(), size, false) > max_w {
            start += 1;
        }
        chars[start..].iter().collect()
    }

    /// Numeric input. Returns true when the value changed.
    pub fn number_field(&mut self, id: u64, r: Rect, v: &mut f64, decimals: usize, min: f64, max: f64) -> bool {
        if self.focus == Some(id) {
            let mut b = std::mem::take(&mut self.buf);
            let submitted = self.text_field(id, r, &mut b, "");
            b.retain(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == ',');
            if submitted || self.focus != Some(id) {
                if let Ok(n) = b.replace(',', ".").trim().parse::<f64>() {
                    let n = n.clamp(min, max);
                    if (n - *v).abs() > 1e-12 {
                        *v = n;
                        return true;
                    }
                }
                return false;
            }
            self.buf = b;
            false
        } else {
            let mut shown = crate::settings::trim_num(round(*v, decimals));
            self.text_field(id, r, &mut shown, "");
            if self.focus == Some(id) {
                self.buf.clear();
            }
            false
        }
    }

    /// Row of colour swatches. Returns true if one was picked.
    pub fn swatches(&mut self, x: f32, y: f32, value: &mut [u8; 3], colors: &[[u8; 3]]) -> bool {
        let t = theme::t();
        let d = self.s(24.0);
        let gap = self.s(10.0);
        let mut changed = false;
        for (i, c) in colors.iter().enumerate() {
            let cx = x + i as f32 * (d + gap) + d * 0.5;
            let cy = y + d * 0.5;
            if value == c {
                draw_circle(cx, cy, d * 0.5 + self.s(3.5), t.text);
                draw_circle(cx, cy, d * 0.5 + self.s(1.5), t.surface);
            }
            draw_circle(cx, cy, d * 0.5, Color::from_rgba(c[0], c[1], c[2], 255));
            let r = Rect::new(cx - d * 0.5, cy - d * 0.5, d, d);
            if self.hover(r) {
                self.hand = true;
            }
            if self.click_in(r) {
                *value = *c;
                changed = true;
            }
        }
        changed
    }

    /// Mouse-wheel scrolling for a region, with a thin scrollbar.
    pub fn scroll(&self, area: Rect, offset: &mut f32, content_h: f32) {
        let max = (content_h - area.h).max(0.0);
        if self.hover(area) && self.wheel != 0.0 {
            let step = if self.wheel.abs() > 10.0 { self.wheel * 0.6 } else { self.wheel * self.s(80.0) };
            *offset -= step;
        }
        *offset = offset.clamp(0.0, max);
        if max > 0.0 {
            let bar_h = (area.h * area.h / content_h).max(self.s(28.0));
            let by = area.y + (area.h - bar_h) * (*offset / max);
            rrect(Rect::new(area.x + area.w - self.s(5.0), by, self.s(3.0), bar_h), self.s(1.5), theme::t().border);
        }
    }
}

fn round(v: f64, d: usize) -> f64 {
    let m = 10f64.powi(d as i32);
    (v * m).round() / m
}

/// Filled rounded rectangle (non-overlapping pieces, so alpha blends correctly).
pub fn rrect(r: Rect, radius: f32, c: Color) {
    let rad = radius.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    if rad < 0.5 {
        draw_rectangle(r.x, r.y, r.w, r.h, c);
        return;
    }
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, r.h, c);
    draw_rectangle(r.x, r.y + rad, rad, r.h - 2.0 * rad, c);
    draw_rectangle(r.x + r.w - rad, r.y + rad, rad, r.h - 2.0 * rad, c);
    let segs = 8;
    for (cx, cy, a0) in [
        (r.x + rad, r.y + rad, std::f32::consts::PI),
        (r.x + r.w - rad, r.y + rad, std::f32::consts::PI * 1.5),
        (r.x + r.w - rad, r.y + r.h - rad, 0.0),
        (r.x + rad, r.y + r.h - rad, std::f32::consts::PI * 0.5),
    ] {
        for i in 0..segs {
            let a = a0 + std::f32::consts::FRAC_PI_2 * i as f32 / segs as f32;
            let b = a0 + std::f32::consts::FRAC_PI_2 * (i + 1) as f32 / segs as f32;
            draw_triangle(
                vec2(cx, cy),
                vec2(cx + a.cos() * rad, cy + a.sin() * rad),
                vec2(cx + b.cos() * rad, cy + b.sin() * rad),
                c,
            );
        }
    }
}

pub fn hash_id(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

pub fn fmt_score(v: f32) -> String {
    if v.abs() >= 1000.0 {
        let s = format!("{:.0}", v.abs());
        let mut out = String::new();
        for (i, ch) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        if v < 0.0 {
            format!("-{out}")
        } else {
            out
        }
    } else if v.fract().abs() < 0.05 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}
