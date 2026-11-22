//! In-game: input → simulation → 3D → HUD, plus the pause overlay.

use crate::app::{App, Screen};
use crate::game::{GameEvent, MoveKeys, Phase};
use crate::input::InputEv;
use crate::render;
use crate::scenario::FireMode;
use crate::theme;
use crate::ui::{fmt_score, hash_id, rrect, size, Kind};
use macroquad::prelude::*;

impl App {
    pub fn frame_playing(&mut self, dt: f32) {
        if is_key_pressed(KeyCode::Escape) && self.ui.focus.is_none() {
            self.paused = !self.paused;
            self.set_grab(!self.paused);
            if let Some(g) = &mut self.game {
                g.trigger_up();
            }
        }
        if is_key_pressed(KeyCode::R) && self.ui.focus.is_none() {
            self.restart();
            return;
        }
        let dpc = self.settings.deg_per_count();
        let inv = self.settings.invert_y;
        let paused = self.paused;
        let Some(g) = &mut self.game else {
            self.screen = Screen::Menu;
            return;
        };
        if !paused {
            for ev in &self.input.events {
                match *ev {
                    InputEv::Motion(dx, dy) => g.look(dx as f64, dy as f64, dpc, inv),
                    InputEv::Down(MouseButton::Left) => g.trigger_down(),
                    InputEv::Up(MouseButton::Left) => g.trigger_up(),
                    _ => {}
                }
            }
            let keys = MoveKeys {
                forward: is_key_down(KeyCode::W),
                back: is_key_down(KeyCode::S),
                left: is_key_down(KeyCode::A),
                right: is_key_down(KeyCode::D),
                jump: is_key_down(KeyCode::Space),
            };
            g.update(dt, keys);
            if let Some(r) = &mut self.recorder {
                r.sample(g);
            }
        }

        // Feedback.
        let vol = self.settings.volume;
        let now = get_time();
        let tracking = g.scn.weapon.mode == FireMode::Auto;
        let (hit_name, kill_name) = (self.settings.hit_sound_name.clone(), self.settings.kill_sound_name.clone());
        for ev in g.events.drain(..) {
            if let Some(r) = &mut self.recorder {
                r.event(g.time, &ev);
            }
            if let GameEvent::Hit { kill, .. } = ev {
                self.hitmarker = 1.0;
                self.hitmarker_kill = kill;
                if kill && self.settings.kill_sound && kill_name != "Off" {
                    self.sounds.play(&kill_name, vol);
                } else if self.settings.hit_sound && hit_name != "Off" && (!tracking || now - self.last_hit_sound > 0.085) {
                    self.sounds.play(&hit_name, vol * if tracking { 0.5 } else { 0.9 });
                    self.last_hit_sound = now;
                }
            }
        }
        self.hitmarker = (self.hitmarker - dt * 7.0).max(0.0);
        if self.settings.game_sounds {
            if g.phase == Phase::Countdown {
                let tick = g.countdown.ceil() as i32;
                if tick != self.last_countdown_tick && tick > 0 {
                    self.last_countdown_tick = tick;
                    self.sounds.play("count", vol * 0.7);
                }
            } else if self.last_countdown_tick > 0 && g.phase == Phase::Running {
                self.last_countdown_tick = 0;
                self.sounds.play("go", vol * 0.7);
            }
        }
        let finished = g.phase == Phase::Finished;

        clear_background(BLACK);
        let cam = self.renderer.draw_scene(g, &self.settings);
        self.draw_target_bars(&cam);
        self.draw_hud();
        self.settings.crosshair.draw(screen_width() * 0.5, screen_height() * 0.5, screen_height() / 1080.0);
        self.draw_hitmarker();
        if self.paused {
            self.draw_pause();
        }
        if finished {
            self.finish();
        }
    }

    fn draw_target_bars(&self, cam: &Camera3D) {
        let Some(g) = &self.game else { return };
        if !self.settings.show_target_health || g.scn.targets.invincible || g.scn.targets.health <= g.scn.weapon.damage {
            return;
        }
        let ui = &self.ui;
        for tg in &g.targets {
            let top = tg.top(&g.scn.targets) + vec3(0.0, 0.25, 0.0);
            if let Some(p) = render::world_to_screen(cam, top) {
                let d = (tg.center(&g.scn.targets) - g.eye()).length().max(1.0);
                let w = (ui.s(1000.0) / d).clamp(ui.s(24.0), ui.s(80.0));
                let h = ui.s(4.0);
                let k = (tg.health / tg.max_health).clamp(0.0, 1.0);
                let back = Rect::new(p.x - w * 0.5, p.y - h, w, h);
                rrect(back, h * 0.5, Color::new(0.0, 0.0, 0.0, 0.55));
                rrect(Rect::new(back.x, back.y, (w * k).max(h), h), h * 0.5, Color::new(1.0, 1.0, 1.0, 0.9));
            }
        }
    }

    fn draw_hitmarker(&self) {
        if !self.settings.hitmarker || self.hitmarker <= 0.0 {
            return;
        }
        let (cx, cy) = (screen_width() * 0.5, screen_height() * 0.5);
        let s = self.ui.scale;
        let a = self.hitmarker;
        let c = if self.hitmarker_kill { Color::new(1.0, 0.3, 0.3, a) } else { Color::new(1.0, 1.0, 1.0, a * 0.85) };
        let (i, o) = (8.0 * s, 14.0 * s);
        for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            draw_line(cx + sx * i, cy + sy * i, cx + sx * o, cy + sy * o, 1.5 * s, c);
        }
    }

    /// KovaaK's-style HUD: just text over the scene, no panels.
    fn draw_hud(&self) {
        let Some(g) = &self.game else { return };
        let t = theme::t();
        let ui = &self.ui;
        let (sw, sh) = (screen_width(), screen_height());
        let sc = |v: f32| ui.s(v);
        let white = Color::new(1.0, 1.0, 1.0, 0.95);
        let soft = Color::new(1.0, 1.0, 1.0, 0.6);

        // Top centre: time, with score / accuracy either side.
        let tl = g.time_left();
        let time_str = format!("{}:{:04.1}", (tl / 60.0).floor() as i32, tl % 60.0);
        let tcol = if tl < 5.0 && g.phase == Phase::Running { t.bad } else { white };
        ui.shadow_text(&time_str, sw * 0.5, sc(18.0), 34.0, tcol, true);
        let k = (g.time / g.scn.duration).clamp(0.0, 1.0);
        let bw = sc(120.0);
        draw_rectangle(sw * 0.5 - bw * 0.5, sc(62.0), bw, sc(2.0), Color::new(1.0, 1.0, 1.0, 0.18));
        draw_rectangle(sw * 0.5 - bw * 0.5, sc(62.0), bw * k, sc(2.0), Color::new(1.0, 1.0, 1.0, 0.8));

        let gap = sc(190.0);
        ui.shadow_text("SCORE", sw * 0.5 - gap, sc(16.0), size::CAPTION, soft, true);
        ui.shadow_text(&fmt_score(g.score), sw * 0.5 - gap, sc(32.0), 22.0, white, true);
        ui.shadow_text("ACCURACY", sw * 0.5 + gap, sc(16.0), size::CAPTION, soft, true);
        ui.shadow_text(&format!("{:.1}%", g.accuracy() * 100.0), sw * 0.5 + gap, sc(32.0), 22.0, white, true);

        // Small extra line under the timer.
        let extra = if g.scn.targets.invincible {
            let on = if g.time > 0.0 { g.on_target_time / g.time * 100.0 } else { 0.0 };
            format!("{on:.0}% on target")
        } else {
            let kps = if g.time > 0.5 { g.kills as f32 / g.time } else { 0.0 };
            format!("{} kills  ·  {kps:.2}/s", g.kills)
        };
        ui.shadow_text(&extra, sw * 0.5, sc(72.0), size::CAPTION, soft, false);

        // Bottom: scenario + best (+ playlist position).
        let best = self.history.best(&g.scn.name).map(fmt_score).unwrap_or_else(|| "–".into());
        let mut line = format!("{}  ·  best {best}", g.scn.name);
        if let Some(run) = &self.run {
            if let Some(p) = self.playlists.get(run.pl) {
                line = format!("{line}  ·  {}/{}", run.entry + 1, p.entries.len());
            }
        }
        let line = ui.fit(&line, size::SMALL, sw * 0.6, false);
        ui.shadow_text(&line, sw * 0.5, sh - sc(36.0), size::SMALL, Color::new(1.0, 1.0, 1.0, 0.5), false);

        if g.scn.weapon.magazine > 0 {
            let txt = if g.time < g.reload_until { "Reloading".to_string() } else { format!("{} / {}", g.ammo, g.scn.weapon.magazine) };
            ui.shadow_text(&txt, sw - sc(90.0), sh - sc(60.0), 20.0, white, true);
        }

        if g.phase == Phase::Countdown {
            let n = g.countdown.ceil().max(1.0) as i32;
            ui.shadow_text(&n.to_string(), sw * 0.5, sh * 0.5 - sc(170.0), 64.0, white, true);
            let hint = if g.scn.weapon.mode == FireMode::Auto { "Hold to fire" } else { "Click to fire" };
            ui.shadow_text(hint, sw * 0.5, sh * 0.5 + sc(70.0), size::SMALL, soft, false);
        }
        if self.settings.show_fps {
            ui.text(&format!("{:.0} fps", self.fps_smooth), sc(16.0), sh - sc(30.0), size::CAPTION, Color::new(1.0, 1.0, 1.0, 0.4));
        }
    }

    fn draw_pause(&mut self) {
        let t = theme::t();
        let (sw, sh) = (screen_width(), screen_height());
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
        let w = sc(420.0);
        let card = Rect::new(sw * 0.5 - w * 0.5, sh * 0.5 - sc(215.0), w, sc(430.0));
        rrect(card, sc(4.0), t.surface);
        let x = card.x + sc(32.0);
        let iw = w - sc(64.0);
        let mut y = card.y + sc(30.0);
        self.ui.strong("Paused", x, y, size::TITLE, t.text);
        y += sc(38.0);
        let name = self.ui.fit(&self.current, size::SMALL, iw, false);
        self.ui.text(&name, x, y, size::SMALL, t.text_dim);
        y += sc(36.0);
        let bh = sc(46.0);
        if self.ui.button(Rect::new(x, y, iw, bh), "Resume", Kind::Primary) {
            self.paused = false;
            self.set_grab(true);
        }
        y += bh + sc(10.0);
        if self.ui.button(Rect::new(x, y, iw, bh), "Restart", Kind::Secondary) {
            self.restart();
            return;
        }
        y += bh + sc(10.0);
        if self.ui.button(Rect::new(x, y, iw, bh), "Quit to menu", Kind::Secondary) {
            self.screen = crate::app::Screen::Menu;
            self.run = None;
            self.game = None;
            return;
        }
        y += bh + sc(26.0);
        self.ui.divider(x, y, iw);
        y += sc(18.0);

        // Quick sensitivity change.
        let label = match self.settings.sens_scale {
            crate::settings::SensScale::Cm360 => "cm/360".to_string(),
            g => format!("{} sensitivity", g.label()),
        };
        self.ui.text(&label, x, y, size::SMALL, t.text_dim);
        y += sc(26.0);
        let fh = sc(40.0);
        let mut changed = false;
        if self.ui.button(Rect::new(x, y, sc(44.0), fh), "–", Kind::Secondary) {
            self.settings.nudge(0.99);
            changed = true;
        }
        let field = Rect::new(x + sc(52.0), y, iw - sc(104.0), fh);
        changed |= match self.settings.sens_scale {
            crate::settings::SensScale::Cm360 => self.ui.number_field(hash_id("p_cm"), field, &mut self.settings.cm360, 2, 1.0, 500.0),
            _ => self.ui.number_field(hash_id("p_sens"), field, &mut self.settings.sens, 4, 0.0001, 1000.0),
        };
        if self.ui.button(Rect::new(x + iw - sc(44.0), y, sc(44.0), fh), "+", Kind::Secondary) {
            self.settings.nudge(1.01);
            changed = true;
        }
        y += fh + sc(12.0);
        let info = format!("{:.2} cm/360  ·  {} DPI  ·  FOV {}", self.settings.cm_per_360(), self.settings.dpi, crate::settings::trim_num(self.settings.hfov as f64));
        self.ui.text(&info, x, y, size::CAPTION, t.text_faint);
        if changed {
            self.settings_dirty = true;
        }
    }
}
