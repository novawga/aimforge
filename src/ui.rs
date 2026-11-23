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
    /// Eased per-widget values (hover glow, sliding highlights, switch knobs).
    anims: std::collections::HashMap<u64, f32>,
    dt: f32,
}

const INTER_REGULAR: &[u8] = &[];
const INTER_SEMIBOLD: &[u8] = &[];

fn embedded(bytes: &[u8]) -> Option<Font> {
    let mut f = load_ttf_font_from_bytes(bytes).ok()?;
    f.set_filter(FilterMode::Linear);
    Some(f)
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
        // A font.ttf in assets overrides; otherwise the bundled Inter; system fonts last.
        let font = try_font(&[&custom]).or_else(|| embedded(INTER_REGULAR)).or_else(|| try_font(&[
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        ]));
        let bold = try_font(&[&custom_b]).or_else(|| embedded(INTER_SEMIBOLD)).or_else(|| try_font(&[
            "C:\\Windows\\Fonts\\seguisb.ttf",
            "C:\\Windows\\Fonts\\segoeuib.ttf",
            "C:\\Windows\\Fonts\\arialbd.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/usr/share/fonts/truetype/inter/Inter-SemiBold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ]))
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
            anims: Default::default(),
            dt: 1.0 / 60.0,
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
        self.dt = get_frame_time().clamp(0.0, 0.1);
        if self.anims.len() > 5000 {
            self.anims.clear();
        }
        if !self.down {
            self.drag = None;
        }
    }

    /// True while the user is typing in a text field (this frame or the start of it),
    /// so screen hotkeys must stay quiet.
    pub fn typing(&self) -> bool {
        self.focus_at_begin || self.focus.is_some()
    }

    /// Ease a per-widget value toward `target`; `speed` ~ 1/seconds.
    pub fn anim(&mut self, key: u64, target: f32, speed: f32) -> f32 {
        let dt = self.dt;
        let v = self.anims.entry(key).or_insert(target);
        *v += (target - *v) * (1.0 - (-dt * speed).exp());
        if (target - *v).abs() < 0.0005 {
            *v = target;
        }
        *v
    }

    /// Hover amount 0..1 for a rect, eased.
    pub fn hover_anim(&mut self, r: Rect, salt: &str) -> f32 {
        let h = self.hover(r);
        self.anim(rect_key(r, salt), if h { 1.0 } else { 0.0 }, 16.0)
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
        panel(r, self.s(16.0), t.surface, with_alpha(t.text, 0.055), self.s(18.0));
    }

    /// Radius used for rows, inputs and buttons.
    pub fn radius(&self) -> f32 {
        self.s(10.0)
    }

    pub fn divider(&self, x: f32, y: f32, w: f32) {
        draw_rectangle(x, y.round(), w, 1.0, with_alpha(theme::t().border, 0.7));
    }

    /// Row highlight used by lists: selected, hovered (eased) or nothing.
    pub fn row_bg(&mut self, r: Rect, selected: bool) {
        let t = theme::t();
        let h = self.hover_anim(r, "row");
        let rad = self.s(9.0);
        if selected {
            rrect(r, rad, t.surface_active);
            let bar = Rect::new(r.x + self.s(4.0), r.y + r.h * 0.28, self.s(3.0), r.h * 0.44);
            rrect(bar, bar.w * 0.5, t.accent);
        } else if h > 0.0 {
            rrect(r, rad, with_alpha(t.surface_hover, h));
        }
        if self.hover(r) {
            self.hand = true;
        }
    }

    // ------------------------------------------------------------- widgets

    pub fn button(&mut self, r: Rect, label: &str, kind: Kind) -> bool {
        let t = theme::t();
        let hov = self.hover(r);
        if hov {
            self.hand = true;
        }
        let h = self.hover_anim(r, label);
        let pressed = hov && self.down;
        let rad = self.radius().min(r.h * 0.5);
        let fg = match kind {
            Kind::Primary => {
                let base = mix(t.accent, t.text, 0.10 * h);
                let base = if pressed { mix(base, BLACK, 0.12) } else { base };
                if h > 0.0 {
                    rrect_soft(Rect::new(r.x, r.y + self.s(4.0), r.w, r.h), rad + self.s(4.0), with_alpha(t.accent, 0.30 * h), self.s(22.0));
                }
                rrect_grad(r, rad, mix(base, WHITE, 0.10), base);
                rrect_stroke(r, rad, 1.0, with_alpha(WHITE, 0.10));
                t.accent_text
            }
            Kind::Secondary => {
                let base = lerp_c(t.surface_hover, t.surface_active, h);
                rrect(r, rad, if pressed { mix(base, t.bg, 0.3) } else { base });
                rrect_stroke(r, rad, 1.0, with_alpha(t.text, 0.05 + 0.05 * h));
                t.text
            }
            Kind::Ghost => {
                if h > 0.0 {
                    rrect(r, rad, with_alpha(t.surface_hover, h));
                }
                lerp_c(t.text_dim, t.text, h)
            }
        };
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

    /// Row of top-level tabs with a pill that slides to the active one.
    /// Returns (clicked index, total width).
    pub fn tabs(&mut self, x: f32, y: f32, h: f32, labels: &[&str], idx: usize) -> (Option<usize>, f32) {
        let t = theme::t();
        let gap = self.s(4.0);
        let ph = self.s(36.0).min(h);
        let py = y + (h - ph) * 0.5;
        let widths: Vec<f32> = labels.iter().map(|l| self.measure(l, size::BODY, true) + self.s(32.0)).collect();
        let mut xs = vec![];
        let mut cx = x;
        for w in &widths {
            xs.push(cx);
            cx += w + gap;
        }
        let key = hash_id("tabs") ^ (x as u64);
        let tx = self.anim(key, xs[idx.min(xs.len() - 1)], 18.0);
        let tw = self.anim(key ^ 1, widths[idx.min(widths.len() - 1)], 18.0);
        rrect(Rect::new(tx, py, tw, ph), ph * 0.5, t.surface_active);
        let mut out = None;
        for (i, l) in labels.iter().enumerate() {
            let r = Rect::new(xs[i], py, widths[i], ph);
            let hv = self.hover_anim(r, l);
            if i != idx && hv > 0.0 {
                rrect(r, ph * 0.5, with_alpha(t.surface_hover, hv * 0.8));
            }
            if self.hover(r) {
                self.hand = true;
            }
            let col = if i == idx { t.text } else { lerp_c(t.text_dim, t.text, hv) };
            let lw = self.measure(l, size::BODY, true);
            self.text_mid(l, r.x + (r.w - lw) * 0.5, r, size::BODY, col, true);
            if self.click_in(r) {
                out = Some(i);
            }
        }
        (out, cx - x - gap)
    }

    /// Pill segmented control with a sliding thumb. Returns the clicked index.
    pub fn segmented(&mut self, r: Rect, options: &[&str], idx: usize) -> Option<usize> {
        let t = theme::t();
        let rad = (r.h * 0.5).min(self.s(12.0));
        rrect(r, rad, mix(t.surface_hover, t.bg, 0.25));
        rrect_stroke(r, rad, 1.0, with_alpha(t.text, 0.04));
        let pad = self.s(3.0);
        let n = options.len().max(1);
        let total_text: f32 = options.iter().map(|o| self.measure(o, size::SMALL, true)).sum();
        let extra = ((r.w - 2.0 * pad) - total_text).max(0.0) / n as f32;
        let mut segs = vec![];
        let mut x = r.x + pad;
        for o in options {
            let w = self.measure(o, size::SMALL, true) + extra;
            segs.push(Rect::new(x, r.y + pad, w, r.h - 2.0 * pad));
            x += w;
        }
        let key = rect_key(r, "seg");
        let cur = segs[idx.min(n - 1)];
        let tx = self.anim(key, cur.x, 20.0);
        let tw = self.anim(key ^ 1, cur.w, 20.0);
        let thumb = Rect::new(tx, cur.y, tw, cur.h);
        let trad = (rad - pad).max(self.s(4.0));
        rrect_soft(Rect::new(thumb.x, thumb.y + self.s(1.5), thumb.w, thumb.h), trad, Color::new(0.0, 0.0, 0.0, 0.35), self.s(6.0));
        rrect(thumb, trad, t.surface_active);
        rrect_stroke(thumb, trad, 1.0, with_alpha(t.text, 0.06));
        let mut out = None;
        for (i, o) in options.iter().enumerate() {
            let seg = segs[i];
            let active = i == idx;
            let hv = self.hover_anim(seg, o);
            if !active && self.hover(seg) {
                self.hand = true;
            }
            let col = if active { t.text } else { lerp_c(t.text_dim, t.text, hv * 0.7) };
            let tw = self.measure(o, size::SMALL, true);
            self.text_mid(o, seg.x + (seg.w - tw) * 0.5, seg, size::SMALL, col, true);
            if self.click_in(seg) {
                out = Some(i);
            }
        }
        out
    }

    /// Small selectable pill. Returns true when clicked.
    pub fn chip(&mut self, r: Rect, label: &str, active: bool) -> bool {
        let t = theme::t();
        if self.hover(r) {
            self.hand = true;
        }
        let h = self.hover_anim(r, label);
        let rad = (r.h * 0.5).min(self.s(10.0));
        if active {
            rrect(r, rad, with_alpha(t.accent, 0.16));
            rrect_stroke(r, rad, 1.0, with_alpha(t.accent, 0.85));
        } else {
            rrect(r, rad, lerp_c(t.surface_hover, t.surface_active, h));
            rrect_stroke(r, rad, 1.0, with_alpha(t.text, 0.04));
        }
        let label = self.fit(label, size::SMALL, r.w - self.s(12.0), active);
        let w = self.measure(&label, size::SMALL, active);
        let col = if active { t.text } else { lerp_c(t.text_dim, t.text, h) };
        self.text_mid(&label, r.x + (r.w - w) * 0.5, r, size::SMALL, col, active);
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
        let w = self.s(40.0);
        let h = self.s(24.0);
        let sr = Rect::new(r.x + r.w - w, r.y + (r.h - h) * 0.5, w, h);
        let k = self.anim(rect_key(sr, "sw"), if *v { 1.0 } else { 0.0 }, 18.0);
        let hv = self.hover_anim(r, "swh");
        rrect(sr, h * 0.5, lerp_c(mix(t.surface_active, t.text, 0.04 * hv), t.accent, k));
        rrect_stroke(sr, h * 0.5, 1.0, with_alpha(t.text, 0.06 * (1.0 - k)));
        let kr = (h - self.s(6.0)) * 0.5;
        let kx = sr.x + self.s(3.0) + kr + (sr.w - self.s(6.0) - kr * 2.0) * k;
        let ky = sr.y + sr.h * 0.5;
        circle(kx, ky + self.s(1.0), kr + self.s(1.0), Color::new(0.0, 0.0, 0.0, 0.25));
        circle(kx, ky, kr, lerp_c(mix(t.text_dim, t.text, 0.4), WHITE, k));
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
        let th = self.s(5.0);
        let ty = r.y + (r.h - th) * 0.5;
        rrect(Rect::new(r.x, ty, r.w, th), th * 0.5, t.surface_active);
        let k = ((*v - min) / (max - min)).clamp(0.0, 1.0);
        rrect(Rect::new(r.x, ty, (r.w * k).max(th), th), th * 0.5, t.accent);
        let kx = r.x + r.w * k;
        let knob = self.s(9.0);
        let hit = Rect::new(r.x - knob, r.y, r.w + 2.0 * knob, r.h);
        let active = self.drag == Some(id);
        let g = self.anim(id ^ 0x51, if active || self.hover(hit) { 1.0 } else { 0.0 }, 16.0);
        let ky = r.y + r.h * 0.5;
        if g > 0.0 {
            circle(kx, ky, knob + self.s(6.0) * g, with_alpha(t.accent, 0.18 * g));
        }
        circle(kx, ky + self.s(1.0), knob + self.s(1.0), Color::new(0.0, 0.0, 0.0, 0.3));
        circle(kx, ky, knob, WHITE);
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
        let rad = self.radius().min(r.h * 0.5);
        let f = self.anim(id ^ 0xF0C, if focused { 1.0 } else { 0.0 }, 18.0);
        let hv = self.hover_anim(r, "field");
        if f > 0.0 {
            rrect_soft(r, rad + self.s(3.0), with_alpha(t.accent, 0.22 * f), self.s(14.0));
        }
        rrect(r, rad, mix(t.surface_hover, t.bg, 0.35 - 0.15 * hv));
        rrect_stroke(r, rad, 1.0, lerp_c(with_alpha(t.text, 0.07 + 0.05 * hv), t.accent, f));
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
        let pad = self.s(14.0);
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
            let r = Rect::new(cx - d * 0.5, cy - d * 0.5, d, d);
            let hv = self.hover_anim(r, "sw");
            if value == c {
                circle(cx, cy, d * 0.5 + self.s(4.0), t.text);
                circle(cx, cy, d * 0.5 + self.s(2.0), t.surface);
            }
            circle(cx, cy, d * 0.5 + self.s(1.5) * hv, Color::from_rgba(c[0], c[1], c[2], 255));
            rrect_stroke(Rect::new(cx - d * 0.5, cy - d * 0.5, d, d), d * 0.5, 1.0, with_alpha(WHITE, 0.08));
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
    pub fn scroll(&mut self, area: Rect, offset: &mut f32, content_h: f32) {
        let max = (content_h - area.h).max(0.0);
        if self.hover(area) && self.wheel != 0.0 {
            let step = if self.wheel.abs() > 10.0 { self.wheel * 0.6 } else { self.wheel * self.s(80.0) };
            *offset -= step;
        }
        *offset = offset.clamp(0.0, max);
        if max > 0.0 {
            let vis = self.hover_anim(area, "scroll");
            let bar_h = (area.h * area.h / content_h).max(self.s(32.0));
            let by = area.y + self.s(6.0) + (area.h - bar_h - self.s(12.0)) * (*offset / max);
            let w = self.s(4.0);
            let t = theme::t();
            rrect(Rect::new(area.x + area.w - w - self.s(5.0), by, w, bar_h), w * 0.5, with_alpha(t.text, 0.10 + 0.12 * vis));
        }
    }
}

fn round(v: f64, d: usize) -> f64 {
    let m = 10f64.powi(d as i32);
    (v * m).round() / m
}

// ------------------------------------------------------------ shape drawing
//
// Everything rounded is drawn as one triangle mesh with a feathered edge, so
// corners are smooth at any size and alpha blends without seams.

use macroquad::models::{draw_mesh, Mesh, Vertex};

/// Points around a rounded rect whose corners have radius `rad + off`.
fn outline(r: Rect, rad: f32, off: f32, out: &mut Vec<Vec2>) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let rr = (rad + off).max(0.0);
    // Same point count for every ring of a shape (depends on the base radius only).
    let n = ((rad + 2.0) * 0.5).clamp(2.0, 18.0) as usize;
    let corners = [
        (r.x + r.w - rad, r.y + r.h - rad, 0.0),
        (r.x + rad, r.y + r.h - rad, FRAC_PI_2),
        (r.x + rad, r.y + rad, PI),
        (r.x + r.w - rad, r.y + rad, PI * 1.5),
    ];
    for (cx, cy, a0) in corners {
        for i in 0..=n {
            let a = a0 + FRAC_PI_2 * i as f32 / n as f32;
            out.push(vec2(cx + a.cos() * rr, cy + a.sin() * rr));
        }
    }
}

/// Build and draw a rounded-rect mesh made of rings at the given (offset, alpha)
/// pairs, optionally filled in the middle. `col` gives the colour at a point.
fn rr_mesh(r: Rect, radius: f32, rings: &[(f32, f32)], fill: bool, col: &dyn Fn(Vec2) -> Color) {
    if r.w <= 0.0 || r.h <= 0.0 {
        return;
    }
    // A hair of radius keeps every ring the same length even for square corners.
    let rad = radius.min(r.w * 0.5).min(r.h * 0.5).max(0.01);
    let mut verts: Vec<Vertex> = vec![];
    let mut idx: Vec<u16> = vec![];
    let mut ring_start = vec![];
    let mut n_ring = 0;
    if fill {
        let c = vec2(r.x + r.w * 0.5, r.y + r.h * 0.5);
        let k = col(c);
        let a = rings.first().map(|x| x.1).unwrap_or(1.0);
        verts.push(Vertex::new(c.x, c.y, 0.0, 0.0, 0.0, Color::new(k.r, k.g, k.b, k.a * a)));
    }
    let mut pts = vec![];
    for &(off, a) in rings {
        pts.clear();
        outline(r, rad, off, &mut pts);
        n_ring = pts.len();
        ring_start.push(verts.len());
        for p in &pts {
            let k = col(*p);
            verts.push(Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, Color::new(k.r, k.g, k.b, k.a * a)));
        }
    }
    if verts.len() > u16::MAX as usize {
        return;
    }
    if fill && !ring_start.is_empty() {
        let s0 = ring_start[0];
        for i in 0..n_ring {
            idx.extend_from_slice(&[0, (s0 + i) as u16, (s0 + (i + 1) % n_ring) as u16]);
        }
    }
    for w in ring_start.windows(2) {
        let (a, b) = (w[0], w[1]);
        for i in 0..n_ring {
            let j = (i + 1) % n_ring;
            idx.extend_from_slice(&[(a + i) as u16, (b + i) as u16, (b + j) as u16, (a + i) as u16, (b + j) as u16, (a + j) as u16]);
        }
    }
    draw_mesh(&Mesh { vertices: verts, indices: idx, texture: None });
}

/// Filled rounded rectangle with an anti-aliased edge.
pub fn rrect(r: Rect, radius: f32, c: Color) {
    if c.a <= 0.0 {
        return;
    }
    rr_mesh(r, radius, &[(-0.5, 1.0), (0.5, 0.0)], true, &|_| c);
}

/// Rounded rectangle with a vertical gradient.
pub fn rrect_grad(r: Rect, radius: f32, top: Color, bottom: Color) {
    let f = move |p: Vec2| {
        let k = ((p.y - r.y) / r.h.max(1.0)).clamp(0.0, 1.0);
        Color::new(
            top.r + (bottom.r - top.r) * k,
            top.g + (bottom.g - top.g) * k,
            top.b + (bottom.b - top.b) * k,
            top.a + (bottom.a - top.a) * k,
        )
    };
    rr_mesh(r, radius, &[(-0.5, 1.0), (0.5, 0.0)], true, &f);
}

/// Soft blurred rounded rect: shadows and glows. `blur` is the fade width.
pub fn rrect_soft(r: Rect, radius: f32, c: Color, blur: f32) {
    if c.a <= 0.0 {
        return;
    }
    let b = blur.max(1.0);
    rr_mesh(r, radius, &[(-b * 0.5, 1.0), (-b * 0.1, 0.62), (b * 0.25, 0.22), (b * 0.5, 0.0)], true, &|_| c);
}

/// Rounded outline, `w` px wide, anti-aliased.
pub fn rrect_stroke(r: Rect, radius: f32, w: f32, c: Color) {
    if c.a <= 0.0 {
        return;
    }
    let h = w * 0.5;
    if w >= 1.0 {
        rr_mesh(r, radius, &[(-h - 0.5, 0.0), (-h + 0.5, 1.0), (h - 0.5, 1.0), (h + 0.5, 0.0)], false, &|_| c);
    } else {
        rr_mesh(r, radius, &[(-1.0, 0.0), (0.0, w), (1.0, 0.0)], false, &|_| c);
    }
}

/// Paint the outside of `r`'s rounded corners with `c` (the colour behind),
/// so anything drawn square into `r` ends up with rounded corners.
pub fn round_corners(r: Rect, radius: f32, c: Color) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let rad = radius.min(r.w * 0.5).min(r.h * 0.5);
    if rad < 1.0 {
        return;
    }
    let n = 12;
    for (cx, cy, kx, ky, a0) in [
        (r.x + rad, r.y + rad, r.x - 1.0, r.y - 1.0, PI),
        (r.x + r.w - rad, r.y + rad, r.x + r.w + 1.0, r.y - 1.0, PI * 1.5),
        (r.x + r.w - rad, r.y + r.h - rad, r.x + r.w + 1.0, r.y + r.h + 1.0, 0.0),
        (r.x + rad, r.y + r.h - rad, r.x - 1.0, r.y + r.h + 1.0, FRAC_PI_2),
    ] {
        let mut verts = vec![Vertex::new(kx, ky, 0.0, 0.0, 0.0, c)];
        let mut idx = vec![];
        for i in 0..=n {
            let a = a0 + FRAC_PI_2 * i as f32 / n as f32;
            // Half a pixel inside the arc so the edge blends with the AA fill.
            let rr = rad + 0.5;
            verts.push(Vertex::new(cx + a.cos() * rr, cy + a.sin() * rr, 0.0, 0.0, 0.0, c));
            if i > 0 {
                idx.extend_from_slice(&[0u16, i as u16, (i + 1) as u16]);
            }
        }
        draw_mesh(&Mesh { vertices: verts, indices: idx, texture: None });
    }
    rrect_stroke(r, rad, 1.0, with_alpha(WHITE, 0.06));
}

/// Smooth anti-aliased circle.
pub fn circle(x: f32, y: f32, r: f32, c: Color) {
    rrect(Rect::new(x - r, y - r, r * 2.0, r * 2.0), r, c);
}

/// Card look: soft shadow, surface fill and a hairline edge.
pub fn panel(r: Rect, radius: f32, fill: Color, edge: Color, shadow: f32) {
    if shadow > 0.0 {
        rrect_soft(Rect::new(r.x, r.y + shadow * 0.35, r.w, r.h), radius + shadow * 0.3, Color::new(0.0, 0.0, 0.0, 0.32), shadow * 2.0);
    }
    rrect(r, radius, fill);
    rrect_stroke(r, radius, 1.0, edge);
}

fn lerp_c(a: Color, b: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color::new(a.r + (b.r - a.r) * k, a.g + (b.g - a.g) * k, a.b + (b.b - a.b) * k, a.a + (b.a - a.a) * k)
}

/// Stable key for a widget's eased state, from where it is and what it is.
pub fn rect_key(r: Rect, salt: &str) -> u64 {
    let mut h = hash_id(salt);
    for v in [r.x, r.y, r.w, r.h] {
        h ^= (v.round() as i64 as u64).wrapping_add(0x9E37_79B9);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
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
