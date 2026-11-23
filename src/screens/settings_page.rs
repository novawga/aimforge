//! Settings: sensitivity, crosshair, theme, gameplay, video/audio, files.

use crate::app::{App, SettingsPage};
use crate::audio::{HIT_SOUNDS, KILL_SOUNDS};
use crate::crosshair::{Crosshair, VALORANT_COLORS};
use crate::map::{self, BallStyle, Layout, Pattern, BALL_COLORS, SKIES, SURFACE_COLORS};
use crate::settings::{trim_num, SensScale, Settings};
use crate::theme::{self, NamedTheme};
use crate::ui::{hash_id, rrect, size, Kind};
use macroquad::prelude::*;



/// Toggle row bound to a settings field (copies in/out to keep the borrow checker happy).
macro_rules! sw {
    ($app:ident, $f:expr, $label:expr, $hint:expr, $($field:ident).+) => {{
        let mut v = $app.settings.$($field).+;
        let c = $app.switch_row($f, $label, $hint, &mut v);
        $app.settings.$($field).+ = v;
        c
    }};
}

/// Slider row bound to a settings field.
macro_rules! sl {
    ($app:ident, $f:expr, $label:expr, $id:expr, [$($field:ident).+], $min:expr, $max:expr, $step:expr, $fmt:expr) => {{
        let mut v = $app.settings.$($field).+;
        let c = $app.slider_row($f, $label, $id, &mut v, $min, $max, $step, $fmt);
        $app.settings.$($field).+ = v;
        c
    }};
}

/// Layout cursor for a form inside a scrolling card.
struct Form {
    x: f32,
    w: f32,
    y: f32,
    top: f32,
    bottom: f32,
    s: f32,
}

impl Form {
    fn sc(&self, v: f32) -> f32 {
        (v * self.s).round()
    }
    fn vis(&self, h: f32) -> bool {
        self.y >= self.top - 0.5 && self.y + h <= self.bottom + 0.5
    }
    /// Control rect on the right side of a row.
    fn ctrl(&self, w: f32, h: f32, row_h: f32) -> Rect {
        Rect::new(self.x + self.w - self.sc(w), self.y + (row_h - self.sc(h)) * 0.5, self.sc(w), self.sc(h))
    }
}

impl App {
    pub fn page_settings(&mut self, r: Rect) {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        // Sub-navigation and card, centred as one block.
        let pad = sc(36.0);
        let nav_w = sc(240.0);
        let card_w = (r.w - nav_w - sc(24.0)).min(sc(860.0) + 2.0 * pad);
        let total = nav_w + sc(24.0) + card_w;
        let left = r.x + ((r.w - total) * 0.5).max(0.0).round();
        let nav = Rect::new(left, r.y, nav_w, r.h);
        let mut y = nav.y;
        for p in SettingsPage::ALL {
            let rr = Rect::new(nav.x, y, nav.w, sc(40.0));
            let active = self.settings_page == p;
            self.ui.row_bg(rr, active);
            self.ui.text_mid(p.label(), rr.x + sc(18.0), rr, size::BODY, if active { t.text } else { t.text_dim }, active);
            if self.ui.click_in(rr) && !active {
                self.settings_page = p;
                self.page_scroll = 0.0;
            }
            y += sc(44.0);
        }

        let card = Rect::new(nav.x + nav.w + sc(24.0), r.y, card_w, r.h);
        self.ui.card(card);
        let inner = Rect::new(card.x + pad, card.y + sc(8.0), (card.w - 2.0 * pad).min(sc(820.0)), card.h - sc(16.0));
        let mut f = Form { x: inner.x, w: inner.w, y: inner.y + sc(20.0) - self.page_scroll, top: inner.y, bottom: inner.y + inner.h, s };
        let start = f.y;
        let mut changed = false;
        match self.settings_page {
            SettingsPage::Sensitivity => changed |= self.page_sens(&mut f),
            SettingsPage::Crosshair => changed |= self.page_crosshair(&mut f, card),
            SettingsPage::Map => changed |= self.page_map(&mut f),
            SettingsPage::Sound => changed |= self.page_sound(&mut f),
            SettingsPage::Theme => changed |= self.page_theme(&mut f),
            SettingsPage::Gameplay => changed |= self.page_gameplay(&mut f),
            SettingsPage::VideoAudio => changed |= self.page_video(&mut f),
            SettingsPage::Files => self.page_files(&mut f),
        }
        let content_h = f.y - start + sc(40.0);
        self.ui.scroll(Rect::new(card.x, card.y, card.w, card.h), &mut self.page_scroll, content_h);
        if changed {
            self.settings_dirty = true;
        }
    }

    // ------------------------------------------------------------- helpers

    fn heading(&self, f: &mut Form, title: &str, sub: Option<&str>) {
        let t = theme::t();
        if f.y > f.top + 1.0 {
            f.y += f.sc(18.0);
        }
        if f.vis(f.sc(30.0)) {
            self.ui.strong(title, f.x, f.y, size::HEADING, t.text);
        }
        f.y += f.sc(30.0);
        if let Some(sub) = sub {
            for line in self.ui.wrap(sub, size::SMALL, f.w, false) {
                if f.vis(f.sc(20.0)) {
                    self.ui.text(&line, f.x, f.y, size::SMALL, t.text_dim);
                }
                f.y += f.sc(20.0);
            }
        }
        f.y += f.sc(8.0);
    }

    /// Label on the left; returns row height. Caller draws the control with `f.ctrl`.
    fn label(&self, f: &Form, label: &str, hint: Option<&str>) -> f32 {
        let t = theme::t();
        let h = f.sc(if hint.is_some() { 60.0 } else { 52.0 });
        if f.vis(h) {
            match hint {
                Some(hint) => {
                    self.ui.text(label, f.x, f.y + f.sc(10.0), size::BODY, t.text);
                    self.ui.text(hint, f.x, f.y + f.sc(32.0), size::SMALL, t.text_faint);
                }
                None => {
                    self.ui.text_mid(label, f.x, Rect::new(f.x, f.y, 1.0, h), size::BODY, t.text, false);
                }
            }
        }
        h
    }

    fn end_row(&self, f: &mut Form, h: f32) {
        f.y += h;
        if f.vis(1.0) {
            self.ui.divider(f.x, f.y, f.w);
        }
    }

    fn switch_row(&mut self, f: &mut Form, label: &str, hint: Option<&str>, v: &mut bool) -> bool {
        let h = self.label(f, label, hint);
        let mut changed = false;
        if f.vis(h) {
            changed = self.ui.switch(Rect::new(f.x, f.y, f.w, h), v);
        }
        self.end_row(f, h);
        changed
    }

    #[allow(clippy::too_many_arguments)]
    fn slider_row(&mut self, f: &mut Form, label: &str, id: &str, v: &mut f32, min: f32, max: f32, step: f32, fmt: &dyn Fn(f32) -> String) -> bool {
        let t = theme::t();
        let h = self.label(f, label, None);
        let mut changed = false;
        if f.vis(h) {
            let c = f.ctrl(360.0, 28.0, h);
            changed = self.ui.slider(hash_id(id), Rect::new(c.x, c.y, c.w - f.sc(64.0), c.h), v, min, max, step);
            self.ui.text_right(&fmt(*v), c.x + c.w, c.y + f.sc(5.0), size::SMALL, t.text_dim, false);
        }
        self.end_row(f, h);
        changed
    }

    // --------------------------------------------------------- sensitivity

    fn page_sens(&mut self, f: &mut Form) -> bool {
        let t = theme::t();
        let mut changed = false;
        self.heading(f, "Sensitivity", Some("Pick the game you play and type the sensitivity you use there. Switching games keeps the same cm/360, so your aim feels identical."));
        if f.vis(f.sc(44.0)) {
            let opts: Vec<&str> = SensScale::ALL.iter().map(|g| g.label()).collect();
            let idx = SensScale::ALL.iter().position(|g| *g == self.settings.sens_scale).unwrap_or(0);
            if let Some(i) = self.ui.segmented(Rect::new(f.x, f.y, f.w, f.sc(42.0)), &opts, idx) {
                self.settings.convert_to(SensScale::ALL[i]);
                changed = true;
            }
        }
        f.y += f.sc(58.0);

        let scale = self.settings.sens_scale;
        match scale {
            SensScale::Cm360 => {
                let h = self.label(f, "cm / 360", Some("Mouse distance for one full turn"));
                if f.vis(h) {
                    changed |= self.ui.number_field(hash_id("cm360"), f.ctrl(160.0, 40.0, h), &mut self.settings.cm360, 2, 1.0, 500.0);
                }
                self.end_row(f, h);
            }
            SensScale::Custom => {
                let h = self.label(f, "Yaw", Some("Degrees per mouse count at sensitivity 1"));
                if f.vis(h) {
                    changed |= self.ui.number_field(hash_id("yaw"), f.ctrl(160.0, 40.0, h), &mut self.settings.custom_yaw, 5, 0.00001, 10.0);
                }
                self.end_row(f, h);
                let h = self.label(f, "Sensitivity", None);
                if f.vis(h) {
                    changed |= self.ui.number_field(hash_id("sens"), f.ctrl(160.0, 40.0, h), &mut self.settings.sens, 4, 0.0001, 1000.0);
                }
                self.end_row(f, h);
            }
            g => {
                let hint = format!("The number from your {} settings", g.label());
                let h = self.label(f, &format!("{} sensitivity", g.label()), Some(&hint));
                if f.vis(h) {
                    let c = f.ctrl(250.0, 40.0, h);
                    // -1% / +1% nudges for fine tuning.
                    if self.ui.button(Rect::new(c.x, c.y, f.sc(40.0), c.h), "–", Kind::Secondary) {
                        self.settings.nudge(0.99);
                        changed = true;
                    }
                    changed |= self.ui.number_field(hash_id("sens"), Rect::new(c.x + f.sc(48.0), c.y, c.w - f.sc(96.0), c.h), &mut self.settings.sens, 4, 0.0001, 1000.0);
                    if self.ui.button(Rect::new(c.x + c.w - f.sc(40.0), c.y, f.sc(40.0), c.h), "+", Kind::Secondary) {
                        self.settings.nudge(1.01);
                        changed = true;
                    }
                }
                self.end_row(f, h);
            }
        }
        let h = self.label(f, "Mouse DPI", None);
        if f.vis(h) {
            changed |= self.ui.number_field(hash_id("dpi"), f.ctrl(160.0, 40.0, h), &mut self.settings.dpi, 0, 50.0, 64000.0);
        }
        self.end_row(f, h);
        changed |= sw!(self, f, "Invert vertical look", None, invert_y);

        // Summary tiles.
        f.y += f.sc(20.0);
        if f.vis(f.sc(76.0)) {
            let cm = self.settings.cm_per_360();
            let mut tiles = vec![
                ("cm / 360", format!("{cm:.2}")),
                ("in / 360", format!("{:.2}", cm / 2.54)),
                ("deg / count", format!("{:.5}", self.settings.deg_per_count())),
            ];
            if let Some(e) = self.settings.edpi() {
                tiles.insert(0, ("eDPI", trim_num((e * 10.0).round() / 10.0)));
            }
            let n = tiles.len() as f32;
            let tw = (f.w - f.sc(10.0) * (n - 1.0)) / n;
            for (i, (k, v)) in tiles.iter().enumerate() {
                let tr = Rect::new(f.x + i as f32 * (tw + f.sc(10.0)), f.y, tw, f.sc(72.0));
                rrect(tr, f.sc(12.0), t.surface_hover);
                crate::ui::rrect_stroke(tr, f.sc(12.0), 1.0, theme::with_alpha(t.text, 0.04));
                self.ui.text(k, tr.x + f.sc(14.0), tr.y + f.sc(12.0), size::CAPTION, t.text_faint);
                self.ui.strong(v, tr.x + f.sc(14.0), tr.y + f.sc(34.0), 20.0, t.text);
            }
        }
        f.y += f.sc(92.0);

        // Conversion table.
        self.heading(f, "Same aim in other games", Some(&format!("At {} DPI. Click Use to switch AimForge to that game's numbers.", self.settings.dpi)));
        for g in SensScale::GAMES {
            let h = f.sc(46.0);
            if f.vis(h) {
                let current = g == self.settings.sens_scale;
                let rr = Rect::new(f.x - f.sc(12.0), f.y, f.w + f.sc(24.0), h);
                if current {
                    rrect(rr, f.sc(10.0), t.surface_hover);
                }
                self.ui.text_mid(g.label(), f.x, rr, size::BODY, if current { t.text } else { t.text_dim }, current);
                let v = self.settings.sens_in(g);
                let shown = format!("{:.*}", g.decimals(), v);
                self.ui.text_mid(&shown, f.x + f.sc(240.0), rr, size::BODY, t.text, true);
                if let Some(fov) = g.default_fov() {
                    self.ui.text_mid(&format!("FOV {}", trim_num(fov as f64)), f.x + f.sc(400.0), rr, size::SMALL, t.text_faint, false);
                }
                if !current {
                    let b = Rect::new(f.x + f.w - f.sc(80.0), f.y + f.sc(7.0), f.sc(80.0), h - f.sc(14.0));
                    if self.ui.button(b, "Use", Kind::Ghost) {
                        self.settings.convert_to(g);
                        changed = true;
                    }
                } else {
                    self.ui.text_right("current", f.x + f.w - f.sc(8.0), f.y + (h - f.sc(size::SMALL)) * 0.5, size::SMALL, t.accent, false);
                }
            }
            f.y += h;
        }

        // FOV.
        self.heading(f, "Field of view", Some("Horizontal FOV at 16:9. Valorant, Overwatch 2 and KovaaK's use 103; CS2 is 106.26."));
        let h = self.label(f, "Horizontal FOV", None);
        if f.vis(h) {
            let c = f.ctrl(420.0, 28.0, h);
            changed |= self.ui.slider(hash_id("fov"), Rect::new(c.x, c.y, c.w - f.sc(110.0), c.h), &mut self.settings.hfov, 60.0, 130.0, 1.0);
            let mut v = self.settings.hfov as f64;
            if self.ui.number_field(hash_id("fovn"), Rect::new(c.x + c.w - f.sc(90.0), c.y - f.sc(6.0), f.sc(90.0), f.sc(40.0)), &mut v, 2, 40.0, 150.0) {
                self.settings.hfov = v as f32;
                changed = true;
            }
        }
        self.end_row(f, h);
        let h = f.sc(56.0);
        if f.vis(h) {
            let mut x = f.x;
            for (label, fov) in [("Valorant · 103", 103.0), ("CS2 · 106.26", 106.26), ("Overwatch 2 · 103", 103.0), ("KovaaK's · 103", 103.0)] {
                let w = self.ui.button_w(label, 34.0);
                if self.ui.button(Rect::new(x, f.y + f.sc(10.0), w, f.sc(34.0)), label, Kind::Secondary) {
                    self.settings.hfov = fov;
                    changed = true;
                }
                x += w + f.sc(8.0);
            }
        }
        f.y += h;
        changed
    }

    // ----------------------------------------------------------- crosshair

    fn page_crosshair(&mut self, f: &mut Form, card: Rect) -> bool {
        let t = theme::t();
        let mut changed = false;
        self.heading(f, "Crosshair", Some("Same model as Valorant. Paste a Valorant crosshair profile code to import it, or copy yours to use in Valorant."));

        // Previews.
        let ph = f.sc(170.0);
        if f.vis(ph) {
            let half = (f.w - f.sc(12.0)) * 0.5;
            for (i, (zoom, label)) in [(1.0, "Actual size"), (3.0, "Zoomed 3×")].iter().enumerate() {
                let pr = Rect::new(f.x + i as f32 * (half + f.sc(12.0)), f.y, half, ph);
                draw_rectangle(pr.x, pr.y, pr.w, pr.h, t.wall);
                let step = f.sc(24.0) * zoom;
                let mut gx = pr.x + (pr.w * 0.5) % step;
                while gx < pr.x + pr.w {
                    draw_line(gx, pr.y, gx, pr.y + pr.h, 1.0, theme::with_alpha(t.grid, 0.35));
                    gx += step;
                }
                let tc = crate::render::rgb(self.settings.map.ball);
                draw_circle(pr.x + pr.w * 0.5 + f.sc(46.0) * zoom.min(1.5), pr.y + pr.h * 0.5 - f.sc(26.0), f.sc(14.0) * zoom.min(1.5), tc);
                self.settings.crosshair.draw(pr.x + pr.w * 0.5, pr.y + pr.h * 0.5, self.ui.scale * zoom);
                self.ui.text(label, pr.x + f.sc(12.0), pr.y + f.sc(10.0), size::CAPTION, theme::with_alpha(t.text, 0.75));
                crate::ui::round_corners(pr, f.sc(12.0), t.surface);
            }
        }
        f.y += ph + f.sc(16.0);

        // Valorant code.
        let h = f.sc(44.0);
        if f.vis(h) {
            let bw = self.ui.button_w("Copy code", 44.0);
            let field = Rect::new(f.x, f.y, f.w - 2.0 * bw - f.sc(16.0), h);
            let id = hash_id("xcode");
            let submitted = self.ui.text_field(id, field, &mut self.xhair_code, "Paste a Valorant crosshair code (Ctrl+V)");
            let import = self.ui.button(Rect::new(field.x + field.w + f.sc(8.0), f.y, bw, h), "Import", Kind::Primary);
            if import || (submitted && !self.xhair_code.trim().is_empty()) {
                let code = if self.xhair_code.trim().is_empty() {
                    miniquad::window::clipboard_get().unwrap_or_default()
                } else {
                    self.xhair_code.clone()
                };
                match Crosshair::from_valorant_code(&code) {
                    Ok(c) => {
                        self.settings.crosshair = c;
                        self.xhair_code.clear();
                        changed = true;
                        self.toast("Crosshair imported");
                    }
                    Err(e) => self.toast(format!("Couldn't import: {e}")),
                }
            }
            if self.ui.button(Rect::new(f.x + f.w - bw, f.y, bw, h), "Copy code", Kind::Secondary) {
                miniquad::window::clipboard_set(&self.settings.crosshair.to_valorant_code());
                self.toast("Valorant crosshair code copied");
            }
        }
        f.y += h + f.sc(12.0);

        // Colour.
        self.heading(f, "Colour", None);
        let h = f.sc(48.0);
        if f.vis(h) {
            changed |= self.ui.swatches(f.x, f.y + f.sc(10.0), &mut self.settings.crosshair.color, &VALORANT_COLORS);
            let c = self.settings.crosshair.color;
            let mut hex = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
            let id = hash_id("xhex");
            let r = Rect::new(f.x + f.w - f.sc(130.0), f.y + f.sc(4.0), f.sc(130.0), f.sc(40.0));
            if self.ui.focus == Some(id) {
                hex = std::mem::take(&mut self.ui.buf);
            }
            let done = self.ui.text_field(id, r, &mut hex, "#RRGGBB");
            if self.ui.focus == Some(id) {
                self.ui.buf = hex.clone();
            } else if done {
                if let Some(col) = theme::hex(&hex) {
                    self.settings.crosshair.color = [(col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8];
                    changed = true;
                }
                self.ui.buf.clear();
            }
        }
        f.y += h;

        let pct = |v: f32| format!("{:.0}%", v * 100.0);
        let px = |v: f32| trim_num(v as f64);
        let mut c = self.settings.crosshair.clone();
        self.heading(f, "Outlines", None);
        changed |= self.switch_row(f, "Show outlines", None, &mut c.outline);
        if c.outline {
            changed |= self.slider_row(f, "Thickness", "x_ot", &mut c.outline_thickness, 1.0, 6.0, 1.0, &px);
            changed |= self.slider_row(f, "Opacity", "x_oo", &mut c.outline_opacity, 0.0, 1.0, 0.05, &pct);
        }
        self.heading(f, "Center dot", None);
        changed |= self.switch_row(f, "Show center dot", None, &mut c.dot);
        if c.dot {
            changed |= self.slider_row(f, "Thickness", "x_dt", &mut c.dot_thickness, 1.0, 6.0, 1.0, &px);
            changed |= self.slider_row(f, "Opacity", "x_do", &mut c.dot_opacity, 0.0, 1.0, 0.05, &pct);
        }
        for (title, key, lines) in [("Inner lines", "in", &mut c.inner), ("Outer lines", "out", &mut c.outer)] {
            self.heading(f, title, None);
            changed |= self.switch_row(f, &format!("Show {}", title.to_lowercase()), None, &mut lines.show);
            if lines.show {
                changed |= self.slider_row(f, "Opacity", &format!("{key}a"), &mut lines.opacity, 0.0, 1.0, 0.05, &pct);
                changed |= self.slider_row(f, "Length", &format!("{key}l"), &mut lines.length, 0.0, 20.0, 1.0, &px);
                changed |= self.switch_row(f, "Separate vertical length", None, &mut lines.split_vertical);
                if lines.split_vertical {
                    changed |= self.slider_row(f, "Vertical length", &format!("{key}v"), &mut lines.vertical_length, 0.0, 20.0, 1.0, &px);
                } else {
                    lines.vertical_length = lines.length;
                }
                changed |= self.slider_row(f, "Thickness", &format!("{key}t"), &mut lines.thickness, 0.0, 10.0, 1.0, &px);
                changed |= self.slider_row(f, "Offset", &format!("{key}o"), &mut lines.offset, 0.0, 20.0, 1.0, &px);
            }
        }
        if changed {
            self.settings.crosshair = c;
        }
        f.y += f.sc(24.0);
        if f.vis(f.sc(40.0)) {
            if self.ui.button(Rect::new(f.x, f.y, f.sc(180.0), f.sc(40.0)), "Reset to default", Kind::Secondary) {
                self.settings.crosshair = Crosshair::default();
                changed = true;
            }
            if self.ui.button(Rect::new(f.x + f.sc(192.0), f.y, f.sc(220.0), f.sc(40.0)), "Valorant default", Kind::Secondary) {
                self.settings.crosshair = Crosshair::valorant_default();
                changed = true;
            }
        }
        f.y += f.sc(40.0);
        let _ = card;
        changed
    }

    // ------------------------------------------------------------------ map

    fn page_map(&mut self, f: &mut Form) -> bool {
        let t = theme::t();
        let mut changed = false;
        self.heading(f, "Map & targets", Some("Sky, floor, walls and target look. Start from a preset, then change anything."));

        // Live preview.
        let ph = f.sc(250.0);
        if f.vis(ph) {
            let pr = Rect::new(f.x, f.y, f.w, ph);
            self.renderer.draw_map_preview(&self.settings.map, pr);
            let name = if self.settings.map.preset.is_empty() { "Custom".to_string() } else { self.settings.map.preset.clone() };
            self.ui.shadow_text(&name, pr.x + pr.w * 0.5, pr.y + pr.h - f.sc(30.0), size::SMALL, WHITE, true);
            crate::ui::round_corners(pr, f.sc(14.0), t.surface);
        }
        f.y += ph + f.sc(14.0);

        // Presets.
        self.heading(f, "Presets", None);
        let presets = map::presets();
        let cols = 4usize;
        let gap = f.sc(8.0);
        let cw = (f.w - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = f.sc(46.0);
        for (i, (name, cfg)) in presets.iter().enumerate() {
            let r = Rect::new(f.x + (i % cols) as f32 * (cw + gap), f.y + (i / cols) as f32 * (ch + gap), cw, ch);
            if r.y < f.top - 0.5 || r.y + r.h > f.bottom + 0.5 {
                continue;
            }
            let active = self.settings.map.preset == *name;
            let sk = map::sky(&cfg.sky);
            // swatch: sky gradient + ball
            let sw = Rect::new(r.x + f.sc(6.0), r.y + f.sc(6.0), f.sc(34.0), r.h - f.sc(12.0));
            if self.ui.chip(r, "", active) {
                self.settings.map = cfg.clone();
                changed = true;
            }
            draw_rectangle(sw.x, sw.y, sw.w, sw.h * 0.5, sk.top());
            draw_rectangle(sw.x, sw.y + sw.h * 0.5, sw.w, sw.h * 0.5, crate::render::rgb(cfg.floor.a));
            draw_rectangle(sw.x, sw.y + sw.h * 0.45, sw.w, sw.h * 0.1, sk.horizon());
            draw_circle(sw.x + sw.w * 0.5, sw.y + sw.h * 0.5, f.sc(5.0), crate::render::rgb(cfg.ball));
            let label = self.ui.fit(name, size::SMALL, r.w - f.sc(56.0), active);
            self.ui.text_mid(&label, sw.x + sw.w + f.sc(10.0), r, size::SMALL, if active { t.text } else { t.text_dim }, active);
        }
        f.y += presets.len().div_ceil(cols) as f32 * (ch + gap) + f.sc(6.0);

        // Sky.
        self.heading(f, "Sky", None);
        let cols = 4usize;
        let ch = f.sc(40.0);
        for (i, sk) in SKIES.iter().enumerate() {
            let r = Rect::new(f.x + (i % cols) as f32 * (cw + gap), f.y + (i / cols) as f32 * (ch + gap), cw, ch);
            if r.y < f.top - 0.5 || r.y + r.h > f.bottom + 0.5 {
                continue;
            }
            let active = self.settings.map.sky == sk.name;
            if self.ui.chip(r, "", active) {
                self.settings.map.sky = sk.name.to_string();
                self.settings.map.preset.clear();
                changed = true;
            }
            let sw = Rect::new(r.x + f.sc(6.0), r.y + f.sc(6.0), f.sc(30.0), r.h - f.sc(12.0));
            let steps = 6;
            for k in 0..steps {
                let a = k as f32 / steps as f32;
                let c = theme::mix(sk.top(), sk.horizon(), a);
                draw_rectangle(sw.x, sw.y + sw.h * a, sw.w, sw.h / steps as f32 + 0.5, c);
            }
            self.ui.text_mid(sk.name, sw.x + sw.w + f.sc(10.0), r, size::SMALL, if active { t.text } else { t.text_dim }, active);
        }
        f.y += SKIES.len().div_ceil(cols) as f32 * (ch + gap) + f.sc(6.0);

        // Layout + fog.
        let h = self.label(f, "Arena", Some("Room has a ceiling; open layouts show the sky"));
        if f.vis(h) {
            let opts: Vec<&str> = Layout::ALL.iter().map(|l| l.label()).collect();
            let idx = Layout::ALL.iter().position(|l| *l == self.settings.map.layout).unwrap_or(0);
            if let Some(i) = self.ui.segmented(f.ctrl(360.0, 38.0, h), &opts, idx) {
                self.settings.map.layout = Layout::ALL[i];
                self.settings.map.preset.clear();
                changed = true;
            }
        }
        self.end_row(f, h);
        let pct = |v: f32| format!("{:.0}%", v * 100.0);
        if sl!(self, f, "Fog", "fog", [map.fog], 0.0, 1.0, 0.05, &pct) {
            self.settings.map.preset.clear();
            changed = true;
        }

        // Surfaces.
        for (title, which) in [("Floor", 0u8), ("Walls", 1u8)] {
            let sub = if which == 1 && self.settings.map.layout == Layout::OpenField { Some("Not shown in the Open field layout") } else { None };
            self.heading(f, title, sub);
            let mut surf = if which == 0 { self.settings.map.floor.clone() } else { self.settings.map.wall.clone() };
            let before = surf.clone();
            // pattern chips
            let pw = f.sc(98.0);
            let per_row = ((f.w + gap) / (pw + gap)).floor().max(1.0) as usize;
            for (i, p) in Pattern::ALL.iter().enumerate() {
                let r = Rect::new(f.x + (i % per_row) as f32 * (pw + gap), f.y + (i / per_row) as f32 * f.sc(40.0), pw, f.sc(32.0));
                if r.y < f.top - 0.5 || r.y + r.h > f.bottom + 0.5 {
                    continue;
                }
                if self.ui.chip(r, p.label(), surf.pattern == *p) {
                    surf.pattern = *p;
                }
            }
            f.y += Pattern::ALL.len().div_ceil(per_row) as f32 * f.sc(40.0) + f.sc(6.0);
            for (label, slot) in [("Base colour", 0u8), ("Pattern colour", 1u8)] {
                let h = f.sc(44.0);
                if f.vis(h) {
                    self.ui.text_mid(label, f.x, Rect::new(f.x, f.y, 1.0, h), size::SMALL, t.text_dim, false);
                    let v = if slot == 0 { &mut surf.a } else { &mut surf.b };
                    self.ui.swatches(f.x + f.sc(130.0), f.y + f.sc(10.0), v, &SURFACE_COLORS);
                }
                f.y += h;
            }
            let h = self.label(f, "Pattern size", None);
            if f.vis(h) {
                let c = f.ctrl(360.0, 28.0, h);
                let id = if which == 0 { "fscale" } else { "wscale" };
                self.ui.slider(hash_id(id), Rect::new(c.x, c.y, c.w - f.sc(64.0), c.h), &mut surf.scale, 0.25, 4.0, 0.25);
                self.ui.text_right(&format!("{} m", trim_num(surf.scale as f64)), c.x + c.w, c.y + f.sc(5.0), size::SMALL, t.text_dim, false);
            }
            self.end_row(f, h);
            if surf != before {
                if which == 0 {
                    self.settings.map.floor = surf;
                } else {
                    self.settings.map.wall = surf;
                }
                self.settings.map.preset.clear();
                changed = true;
            }
        }

        // Targets.
        self.heading(f, "Targets", Some("Scenarios that set their own target colour keep it."));
        let h = f.sc(48.0);
        if f.vis(h) {
            let mut c = self.settings.map.ball;
            if self.ui.swatches(f.x, f.y + f.sc(12.0), &mut c, &BALL_COLORS) {
                self.settings.map.ball = c;
                changed = true;
            }
            let id = hash_id("ballhex");
            let r = Rect::new(f.x + f.w - f.sc(130.0), f.y + f.sc(4.0), f.sc(130.0), f.sc(40.0));
            let mut hexs = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
            if self.ui.focus == Some(id) {
                hexs = std::mem::take(&mut self.ui.buf);
            }
            let done = self.ui.text_field(id, r, &mut hexs, "#RRGGBB");
            if self.ui.focus == Some(id) {
                self.ui.buf = hexs;
            } else if done {
                if let Some(col) = theme::hex(&hexs) {
                    self.settings.map.ball = [(col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8];
                    changed = true;
                }
                self.ui.buf.clear();
            }
        }
        f.y += h;
        let h = self.label(f, "Shading", None);
        if f.vis(h) {
            let opts: Vec<&str> = BallStyle::ALL.iter().map(|b| b.label()).collect();
            let idx = BallStyle::ALL.iter().position(|b| *b == self.settings.map.ball_style).unwrap_or(0);
            if let Some(i) = self.ui.segmented(f.ctrl(360.0, 38.0, h), &opts, idx) {
                self.settings.map.ball_style = BallStyle::ALL[i];
                changed = true;
            }
        }
        self.end_row(f, h);
        if let Some(e) = &self.renderer.map.error {
            f.y += f.sc(12.0);
            if f.vis(f.sc(22.0)) {
                self.ui.text(&self.ui.fit(e, size::SMALL, f.w, false), f.x, f.y, size::SMALL, t.bad);
            }
            f.y += f.sc(22.0);
        }
        changed
    }

    // ---------------------------------------------------------------- sound

    fn page_sound(&mut self, f: &mut Form) -> bool {
        let t = theme::t();
        let mut changed = false;
        self.heading(f, "Sound", None);
        let pct = |v: f32| format!("{:.0}%", v * 100.0);
        changed |= sl!(self, f, "Volume", "vol", [volume], 0.0, 1.0, 0.05, &pct);
        let vol = self.settings.volume;
        for (title, list, is_hit) in [("Hit sound", HIT_SOUNDS, true), ("Kill sound", KILL_SOUNDS, false)] {
            self.heading(f, title, Some("Click one to hear it."));
            let mut names: Vec<String> = list.iter().map(|s| s.to_string()).collect();
            names.extend(self.sounds.custom.iter().cloned());
            let gap = f.sc(8.0);
            let cw = f.sc(118.0);
            let per_row = ((f.w + gap) / (cw + gap)).floor().max(1.0) as usize;
            for (i, n) in names.iter().enumerate() {
                let r = Rect::new(f.x + (i % per_row) as f32 * (cw + gap), f.y + (i / per_row) as f32 * f.sc(42.0), cw, f.sc(34.0));
                if r.y < f.top - 0.5 || r.y + r.h > f.bottom + 0.5 {
                    continue;
                }
                let current = if is_hit { &self.settings.hit_sound_name } else { &self.settings.kill_sound_name };
                let label = n.trim_start_matches("File: ").to_string();
                if self.ui.chip(r, &label, current == n) {
                    if is_hit {
                        self.settings.hit_sound_name = n.clone();
                    } else {
                        self.settings.kill_sound_name = n.clone();
                    }
                    self.sounds.play(n, vol);
                    changed = true;
                }
            }
            f.y += names.len().div_ceil(per_row) as f32 * f.sc(42.0) + f.sc(6.0);
        }
        self.heading(f, "Other sounds", None);
        changed |= sw!(self, f, "Play hit sounds", None, hit_sound);
        changed |= sw!(self, f, "Play kill sounds", None, kill_sound);
        changed |= sw!(self, f, "Countdown, finish and high-score sounds", None, game_sounds);
        changed |= sw!(self, f, "Menu click sounds", None, ui_sounds);
        let h = self.label(f, "Your own sounds", Some("Drop .wav or .ogg files into the sounds folder, then restart"));
        if f.vis(h) && self.ui.button(f.ctrl(150.0, 36.0, h), "Open folder", Kind::Ghost) {
            crate::open_path(&self.paths.sounds);
        }
        self.end_row(f, h);
        let _ = t;
        changed
    }

    // --------------------------------------------------------------- theme

    fn page_theme(&mut self, f: &mut Form) -> bool {
        let t = theme::t();
        let mut changed = false;
        self.heading(f, "Theme", Some("Colours for the menus. (The 3D world is set in Map & targets.) Themes are .toml files in the themes folder, so you can share them or make your own."));
        let cols = 3usize;
        let gap = f.sc(14.0);
        let cw = (f.w - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = f.sc(150.0);
        let themes = self.themes.clone();
        for (i, nt) in themes.iter().enumerate() {
            let (col, row) = (i % cols, i / cols);
            let r = Rect::new(f.x + col as f32 * (cw + gap), f.y + row as f32 * (ch + gap), cw, ch);
            if r.y < f.top - 0.5 || r.y + r.h > f.bottom + 0.5 {
                continue;
            }
            let active = nt.name.eq_ignore_ascii_case(&self.settings.theme);
            if active {
                rrect(Rect::new(r.x - f.sc(2.0), r.y - f.sc(2.0), r.w + f.sc(4.0), r.h + f.sc(4.0)), f.sc(12.0), t.accent);
            }
            theme_preview(&self.ui, r, &nt.theme);
            let label_r = Rect::new(r.x, r.y + r.h - f.sc(36.0), r.w, f.sc(36.0));
            let name = self.ui.fit(&nt.name, size::SMALL, r.w - f.sc(80.0), true);
            self.ui.text_mid(&name, r.x + f.sc(12.0), label_r, size::SMALL, nt.theme.text, true);
            let tag = if nt.path.is_some() { "file" } else { "built-in" };
            self.ui.text_right(tag, r.x + r.w - f.sc(12.0), label_r.y + f.sc(11.0), size::CAPTION, nt.theme.text_faint, false);
            if self.ui.hover(r) {
                self.ui.hand = true;
            }
            if self.ui.click_in(r) && !active {
                self.settings.theme = nt.name.clone();
                self.apply_theme();
                changed = true;
            }
        }
        let rows = themes.len().div_ceil(cols);
        f.y += rows as f32 * (ch + gap) + f.sc(8.0);

        let h = f.sc(44.0);
        if f.vis(h) {
            let mut x = f.x;
            let buttons: [(&str, Kind, u8); 4] = [
                ("Import from clipboard", Kind::Primary, 0),
                ("Copy current theme", Kind::Secondary, 1),
                ("Save as file", Kind::Secondary, 2),
                ("Open themes folder", Kind::Secondary, 3),
            ];
            for (label, kind, act) in buttons {
                let w = self.ui.button_w(label, 44.0);
                if self.ui.button(Rect::new(x, f.y, w, h), label, kind) {
                    match act {
                        0 => changed |= self.import_theme_clipboard(),
                        1 => {
                            let cur = self.current_theme();
                            miniquad::window::clipboard_set(&cur.to_toml());
                            self.toast(format!("Copied \"{}\" theme to clipboard", cur.name));
                        }
                        2 => self.save_theme_file(),
                        _ => crate::open_path(&self.paths.themes),
                    }
                }
                x += w + f.sc(8.0);
            }
        }
        f.y += h + f.sc(12.0);

        changed
    }

    fn current_theme(&self) -> NamedTheme {
        self.themes
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(&self.settings.theme))
            .cloned()
            .unwrap_or_else(|| NamedTheme { name: self.settings.theme.clone(), theme: theme::t(), path: None })
    }

    fn import_theme_clipboard(&mut self) -> bool {
        let Some(text) = miniquad::window::clipboard_get() else {
            self.toast("Clipboard is empty");
            return false;
        };
        match NamedTheme::from_toml(&text) {
            Ok(mut nt) => {
                let file = self.paths.themes.join(format!("{}.toml", crate::scenario::safe_file_name(&nt.name)));
                nt.path = Some(file.clone());
                if let Err(e) = std::fs::write(&file, nt.to_toml()) {
                    self.toast(format!("Couldn't save theme: {e}"));
                    return false;
                }
                self.themes.retain(|t| !t.name.eq_ignore_ascii_case(&nt.name) || t.path.is_none());
                self.settings.theme = nt.name.clone();
                self.themes.push(nt);
                self.apply_theme();
                self.toast(format!("Imported theme \"{}\"", self.settings.theme));
                true
            }
            Err(e) => {
                self.toast(format!("Clipboard isn't a theme: {e}"));
                false
            }
        }
    }

    fn save_theme_file(&mut self) {
        let mut cur = self.current_theme();
        if cur.path.is_none() {
            cur.name = format!("{} (copy)", cur.name);
        }
        let file = self.paths.themes.join(format!("{}.toml", crate::scenario::safe_file_name(&cur.name)));
        match std::fs::write(&file, cur.to_toml()) {
            Ok(()) => {
                self.toast(format!("Saved {} (edit it, then press F5)", file.display()));
                let (tf, _) = theme::load_dir(&self.paths.themes);
                self.themes = theme::builtins();
                self.themes.extend(tf);
            }
            Err(e) => self.toast(format!("Couldn't save: {e}")),
        }
    }

    // ------------------------------------------------- gameplay / video

    fn page_gameplay(&mut self, f: &mut Form) -> bool {
        let mut changed = false;
        self.heading(f, "Gameplay", None);
        let secs = |v: f32| if v == 0.0 { "Off".to_string() } else { format!("{v:.1} s") };
        changed |= sl!(self, f, "Countdown before a run", "cd", [countdown], 0.0, 5.0, 0.5, &secs);
        changed |= sw!(self, f, "Auto-fire in tracking", Some("Fire continuously without holding the mouse button"), auto_fire_tracking);
        changed |= sw!(self, f, "Health bars", Some("On targets that take more than one hit"), show_target_health);
        changed |= sw!(self, f, "Hitmarker", None, hitmarker);
        changed |= sw!(self, f, "Show FPS", None, show_fps);

        self.heading(f, "Leaderboards", Some("Your best run per scenario appears on the global board, with its replay."));
        let (title, sub) = if self.settings.account {
            (format!("Signed in as {}", self.settings.player_name), "Your runs and replays go on the global boards")
        } else {
            ("Playing as guest".to_string(), "Sign in to put your runs on the global boards")
        };
        let h = self.label(f, &title, Some(sub));
        if f.vis(h) {
            let label = if self.settings.account { "Sign out" } else { "Sign in" };
            if self.ui.button(f.ctrl(140.0, 38.0, h), label, if self.settings.account { Kind::Secondary } else { Kind::Primary }) {
                if self.settings.account {
                    self.sign_out();
                    self.toast("Signed out");
                } else {
                    self.open_sign_in(Some(crate::app::Screen::Menu));
                }
            }
        }
        self.end_row(f, h);
        changed |= sw!(self, f, "Submit runs to the online leaderboard", Some("Off keeps everything on this PC"), submit_scores);
        changed
    }

    fn page_video(&mut self, f: &mut Form) -> bool {
        let mut changed = false;
        self.heading(f, "Video", None);
        let before = self.settings.fullscreen;
        changed |= sw!(self, f, "Fullscreen", Some("F11 toggles it anywhere"), fullscreen);
        if before != self.settings.fullscreen {
            set_fullscreen(self.settings.fullscreen);
        }
        changed |= sw!(self, f, "V-Sync", Some("Off gives the lowest input lag. Applies after restart"), vsync);
        changed |= sw!(self, f, "Anti-aliasing (MSAA 4×)", Some("Applies after restart"), msaa);
        f.y += f.sc(24.0);
        if f.vis(f.sc(40.0)) && self.ui.button(Rect::new(f.x, f.y, f.sc(220.0), f.sc(40.0)), "Reset all settings", Kind::Secondary) {
            let fs = self.settings.fullscreen;
            self.settings = Settings { fullscreen: fs, ..Default::default() };
            self.apply_theme();
            changed = true;
        }
        f.y += f.sc(40.0);
        changed
    }

    fn page_files(&mut self, f: &mut Form) {
        let t = theme::t();
        use crate::update::{self, Status};
        self.heading(f, "Updates", Some(&format!("AimForge checks github.com/{} for new versions when it starts.", update::REPO)));
        let h = self.label(f, &format!("Version {}", update::VERSION), Some(&match &self.updater.status {
            Status::Idle => "Not checked yet".to_string(),
            Status::Checking => "Checking...".to_string(),
            Status::UpToDate => "You're on the latest version".to_string(),
            Status::Downloading { version, done, total } => format!("Downloading v{version} ({} of {} KB)", done / 1024, total / 1024),
            Status::Installed { version } => format!("v{version} installed, restart to use it"),
            Status::Available { version, why, .. } if why.is_empty() => format!("v{version} is available"),
            Status::Available { version, why, .. } => format!("v{version} is available ({why})"),
            Status::Failed(e) => format!("Update check failed: {e}"),
        }));
        if f.vis(h) {
            let installed = matches!(self.updater.status, Status::Installed { .. });
            let label = if installed { "Restart now" } else { "Check for updates" };
            let busy = matches!(self.updater.status, Status::Checking | Status::Downloading { .. });
            if self.ui.button(f.ctrl(170.0, 38.0, h), label, if installed { Kind::Primary } else { Kind::Secondary }) && !busy {
                if installed {
                    self.settings.save(&self.paths.settings);
                    self.updater.restart();
                }
                self.updater.check(true);
            }
        }
        self.end_row(f, h);
        let mut auto = self.settings.auto_update;
        if self.switch_row(f, "Install updates automatically", Some("Download new versions on launch and restart into them"), &mut auto) {
            self.settings.auto_update = auto;
            self.settings_dirty = true;
        }
        let h = self.label(f, "Release notes", None);
        if f.vis(h) && self.ui.button(f.ctrl(170.0, 38.0, h), "Open on GitHub", Kind::Ghost) {
            crate::open_url(&format!("https://github.com/{}/releases", update::REPO));
        }
        self.end_row(f, h);

        self.heading(f, "Community playlists", Some(&format!("Playlists collected at github.com/{}, by their respective authors.", update::PLAYLIST_REPO)));
        let h = self.label(f, "Download or refresh", Some("Saved to playlists/community-riddbtw"));
        if f.vis(h) {
            let busy = self.playlist_job.is_some();
            if self.ui.button(f.ctrl(170.0, 38.0, h), if busy { "Downloading..." } else { "Download" }, Kind::Secondary) && !busy {
                self.download_playlists();
            }
        }
        self.end_row(f, h);

        self.heading(f, "Controls", None);
        for (k, v) in [
            ("Aim / shoot", "Mouse / left click (hold in tracking)"),
            ("Move / jump", "W A S D / Space"),
            ("Restart", "R"),
            ("Pause / back", "Esc"),
            ("Next in playlist", "Enter on the results screen"),
            ("Play selected", "Double-click or Enter"),
            ("Reload files", "F5"),
            ("Fullscreen", "F11"),
        ] {
            let h = f.sc(36.0);
            if f.vis(h) {
                self.ui.text(k, f.x, f.y + f.sc(9.0), size::BODY, t.text_dim);
                self.ui.text(v, f.x + f.sc(220.0), f.y + f.sc(9.0), size::BODY, t.text);
            }
            f.y += h;
        }
        self.heading(f, "Folders", Some("Everything AimForge reads and writes lives in plain files you can back up, edit and share."));
        let folders = [
            ("Scenarios", self.paths.scenarios.clone()),
            ("Playlists", self.paths.playlists.clone()),
            ("Themes", self.paths.themes.clone()),
            ("Stats", self.paths.stats.clone()),
        ];
        for (name, p) in folders {
            let h = self.label(f, name, None);
            if f.vis(h) {
                let shown = self.ui.fit(&p.display().to_string(), size::SMALL, f.w - f.sc(320.0), false);
                self.ui.text_mid(&shown, f.x + f.sc(140.0), Rect::new(0.0, f.y, 1.0, h), size::SMALL, t.text_dim, false);
                if self.ui.button(f.ctrl(90.0, 34.0, h), "Open", Kind::Ghost) {
                    crate::open_path(&p);
                }
            }
            self.end_row(f, h);
        }
        if !self.load_errors.is_empty() {
            self.heading(f, &format!("File problems ({})", self.load_errors.len()), None);
            for e in self.load_errors.clone().iter().take(10) {
                let h = f.sc(22.0);
                if f.vis(h) {
                    self.ui.text(&self.ui.fit(e, size::SMALL, f.w, false), f.x, f.y, size::SMALL, t.bad);
                }
                f.y += h;
            }
        }
        f.y += f.sc(24.0);
        if f.vis(f.sc(40.0)) && self.ui.button(Rect::new(f.x, f.y, f.sc(160.0), f.sc(40.0)), "Quit AimForge", Kind::Secondary) {
            self.settings.save(&self.paths.settings);
            std::process::exit(0);
        }
        f.y += f.sc(40.0);
    }
}

/// Miniature of a theme: background, a card, the accent, and the room.
fn theme_preview(ui: &crate::ui::Ui, r: Rect, th: &theme::Theme) {
    let s = ui.scale;
    let sc = |v: f32| (v * s).round();
    rrect(r, sc(10.0), th.bg);
    // room strip
    let room = Rect::new(r.x + sc(10.0), r.y + sc(10.0), r.w * 0.55, r.h - sc(56.0));
    rrect(room, sc(6.0), th.wall);
    draw_rectangle(room.x, room.y + room.h * 0.68, room.w, room.h * 0.32, th.floor);
    for k in 1..4 {
        let x = room.x + room.w * k as f32 / 4.0;
        draw_line(x, room.y, x, room.y + room.h * 0.68, 1.0, theme::with_alpha(th.grid, 0.6));
    }
    draw_circle(room.x + room.w * 0.62, room.y + room.h * 0.4, sc(7.0), th.target);
    // ui card
    let card = Rect::new(room.x + room.w + sc(8.0), room.y, r.x + r.w - sc(10.0) - (room.x + room.w + sc(8.0)), room.h);
    rrect(card, sc(6.0), th.surface);
    rrect(Rect::new(card.x + sc(8.0), card.y + sc(10.0), card.w - sc(16.0), sc(6.0)), sc(3.0), th.text_dim);
    rrect(Rect::new(card.x + sc(8.0), card.y + sc(22.0), (card.w - sc(16.0)) * 0.6, sc(6.0)), sc(3.0), th.text_faint);
    rrect(Rect::new(card.x + sc(8.0), card.y + card.h - sc(22.0), card.w - sc(16.0), sc(14.0)), sc(4.0), th.accent);
}
