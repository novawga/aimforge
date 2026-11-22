//! Replay viewer: first-person playback of a recorded run.

use crate::app::{App, Screen};
use crate::replay::{EventKind, Replay};
use crate::theme;
use crate::ui::{fmt_score, rrect, size, Kind};
use macroquad::prelude::*;

const SPEEDS: [f32; 5] = [0.25, 0.5, 1.0, 1.5, 2.0];

impl App {
    pub fn frame_replay(&mut self, dt: f32) {
        self.set_grab(false);
        let Some(v) = &mut self.viewer else {
            self.screen = Screen::Menu;
            return;
        };
        let dur = v.replay.duration().max(0.01);

        // Keys.
        let mut seek: Option<f32> = None;
        if is_key_pressed(KeyCode::Escape) {
            let from = v.from;
            self.screen = if from == Screen::Replay { Screen::Menu } else { from };
            self.viewer = None;
            return;
        }
        if is_key_pressed(KeyCode::Space) {
            if v.t >= dur {
                seek = Some(0.0);
            }
            v.playing = !v.playing || v.t >= dur;
        }
        if is_key_pressed(KeyCode::Left) {
            seek = Some((v.t - 5.0).max(0.0));
        }
        if is_key_pressed(KeyCode::Right) {
            seek = Some((v.t + 5.0).min(dur));
        }
        if is_key_pressed(KeyCode::R) {
            seek = Some(0.0);
            v.playing = true;
        }
        let si = SPEEDS.iter().position(|s| (*s - v.speed).abs() < 1e-3).unwrap_or(2);
        if is_key_pressed(KeyCode::Up) {
            v.speed = SPEEDS[(si + 1).min(SPEEDS.len() - 1)];
        }
        if is_key_pressed(KeyCode::Down) {
            v.speed = SPEEDS[si.saturating_sub(1)];
        }

        // Advance + events.
        let prev = v.t;
        if v.playing {
            v.t = (v.t + dt * v.speed).min(dur);
            if v.t >= dur {
                v.playing = false;
            }
        }
        let mut sounds: Vec<&'static str> = vec![];
        let mut shot_pulse = false;
        while v.next_event < v.replay.events.len() && v.replay.events[v.next_event].t <= v.t {
            let e = &v.replay.events[v.next_event];
            if e.t >= prev - 0.05 {
                match e.kind {
                    EventKind::Hit => {
                        v.hitmarker = 1.0;
                        v.hitmarker_kill = false;
                        sounds.push("hit");
                        shot_pulse = true;
                    }
                    EventKind::Kill => {
                        v.hitmarker = 1.0;
                        v.hitmarker_kill = true;
                        sounds.push("kill");
                        shot_pulse = true;
                    }
                    EventKind::Miss => shot_pulse = true,
                    EventKind::Expire => {}
                }
            }
            v.next_event += 1;
        }
        v.hitmarker = (v.hitmarker - dt * 7.0).max(0.0);

        // Pose the stand-in game for rendering.
        let frame = v.replay.at(v.t);
        if let Some(f) = &frame {
            v.game.player.pos = f.pos;
            v.game.player.yaw = f.yaw as f64;
            v.game.player.pitch = f.pitch as f64;
            v.game.targets = Replay::to_targets(f);
            v.game.time = v.t;
        }
        let vol = self.settings.volume * if v.speed > 1.01 { 0.5 } else { 1.0 };
        let tracking = v.game.scn.weapon.mode == crate::scenario::FireMode::Auto;
        for s in sounds {
            let name = if s == "kill" { self.settings.kill_sound_name.clone() } else { self.settings.hit_sound_name.clone() };
            let now = get_time();
            if s == "hit" && tracking && now - self.last_hit_sound < 0.085 {
                continue;
            }
            self.last_hit_sound = now;
            if name != "Off" {
                self.sounds.play(&name, vol * if s == "hit" && tracking { 0.5 } else { 0.9 });
            }
        }

        // Draw scene.
        clear_background(BLACK);
        let v = self.viewer.as_ref().unwrap();
        self.renderer.draw_scene(&v.game, &self.settings);
        let (sw, sh) = (screen_width(), screen_height());
        self.settings.crosshair.draw(sw * 0.5, sh * 0.5, sh / 1080.0);
        if shot_pulse || frame.as_ref().is_some_and(|f| f.trigger && tracking) {
            draw_circle_lines(sw * 0.5, sh * 0.5, self.ui.s(16.0), self.ui.s(1.5), Color::new(1.0, 1.0, 1.0, 0.5));
        }
        if v.hitmarker > 0.0 && self.settings.hitmarker {
            let s = self.ui.scale;
            let c = if v.hitmarker_kill { Color::new(1.0, 0.3, 0.3, v.hitmarker) } else { Color::new(1.0, 1.0, 1.0, v.hitmarker * 0.85) };
            let (i, o) = (8.0 * s, 14.0 * s);
            for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                draw_line(sw * 0.5 + dx * i, sh * 0.5 + dy * i, sw * 0.5 + dx * o, sh * 0.5 + dy * o, 1.5 * s, c);
            }
        }

        // HUD.
        let ui = &self.ui;
        let white = Color::new(1.0, 1.0, 1.0, 0.95);
        let soft = Color::new(1.0, 1.0, 1.0, 0.6);
        if let Some(f) = &frame {
            let left = (v.game.scn.duration - v.t).max(0.0);
            ui.shadow_text(&format!("{}:{:04.1}", (left / 60.0).floor() as i32, left % 60.0), sw * 0.5, ui.s(18.0), 34.0, white, true);
            let gap = ui.s(190.0);
            let acc = if f.shots > 0 { f.hits as f32 / f.shots as f32 * 100.0 } else { 0.0 };
            ui.shadow_text("SCORE", sw * 0.5 - gap, ui.s(16.0), size::CAPTION, soft, true);
            ui.shadow_text(&fmt_score(f.score), sw * 0.5 - gap, ui.s(32.0), 22.0, white, true);
            ui.shadow_text("ACCURACY", sw * 0.5 + gap, ui.s(16.0), size::CAPTION, soft, true);
            ui.shadow_text(&format!("{acc:.1}%"), sw * 0.5 + gap, ui.s(32.0), 22.0, white, true);
            ui.shadow_text(&format!("{} kills", f.kills), sw * 0.5, ui.s(72.0), size::CAPTION, soft, false);
        }
        let h = &v.replay.header;
        let who = if h.player.is_empty() { "You".to_string() } else { h.player.clone() };
        let tag = format!("REPLAY  ·  {who}  ·  {}  ·  final {}  ·  {:.1}%", h.scenario, fmt_score(h.score), h.accuracy * 100.0);
        ui.text(&ui.fit(&tag, size::SMALL, sw * 0.45, true), ui.s(20.0), ui.s(20.0), size::SMALL, theme::t().accent);
        if !v.label.is_empty() {
            ui.text(&v.label, ui.s(20.0), ui.s(42.0), size::CAPTION, soft);
        }

        // Control bar.
        let t = theme::t();
        let bar = Rect::new(ui.s(20.0), sh - ui.s(76.0), sw - ui.s(40.0), ui.s(56.0));
        let (playing, t_now, speed) = (v.playing, v.t, v.speed);
        rrect(bar, ui.s(4.0), theme::with_alpha(t.bg, 0.82));
        let s = self.ui.scale;
        let sc = |x: f32| (x * s).round();
        let bx = bar.x + sc(10.0);
        let by = bar.y + sc(10.0);
        let mut act_play = false;
        let mut act_back = false;
        if self.ui.button(Rect::new(bx, by, sc(90.0), sc(36.0)), if playing { "Pause" } else { "Play" }, Kind::Primary) {
            act_play = true;
        }
        let time_txt = format!("{:>4.1} / {:.1} s", t_now, dur);
        let tw = sc(110.0);
        self.ui.text_mid(&time_txt, bx + sc(104.0), Rect::new(0.0, by, 1.0, sc(36.0)), size::SMALL, t.text_dim, false);
        let speed_w = sc(250.0);
        let back_w = sc(90.0);
        let tl = Rect::new(bx + sc(104.0) + tw, by, bar.w - (sc(104.0) + tw) - speed_w - back_w - sc(40.0), sc(36.0));
        // timeline
        let k = (t_now / dur).clamp(0.0, 1.0);
        let track = Rect::new(tl.x, tl.y + tl.h * 0.5 - sc(2.0), tl.w, sc(4.0));
        rrect(track, sc(2.0), t.surface_active);
        rrect(Rect::new(track.x, track.y, track.w * k, track.h), sc(2.0), t.accent);
        // kill ticks
        for e in &self.viewer.as_ref().unwrap().replay.events {
            if e.kind == EventKind::Kill {
                let x = track.x + track.w * (e.t / dur).clamp(0.0, 1.0);
                draw_rectangle(x, track.y - sc(4.0), 1.0_f32.max(sc(1.0)), sc(12.0), theme::with_alpha(t.text, 0.25));
            }
        }
        draw_circle(track.x + track.w * k, track.y + track.h * 0.5, sc(7.0), t.text);
        if self.ui.down && (self.ui.hover(tl) || self.ui.drag == Some(42)) {
            self.ui.drag = Some(42);
            seek = Some(((self.ui.mouse.x - track.x) / track.w).clamp(0.0, 1.0) * dur);
        }
        let opts = ["0.25×", "0.5×", "1×", "1.5×", "2×"];
        let sidx = SPEEDS.iter().position(|x| (*x - speed).abs() < 1e-3).unwrap_or(2);
        let mut new_speed = None;
        if let Some(i) = self.ui.segmented(Rect::new(tl.x + tl.w + sc(14.0), by, speed_w, sc(36.0)), &opts, sidx) {
            new_speed = Some(SPEEDS[i]);
        }
        if self.ui.button(Rect::new(bar.x + bar.w - back_w - sc(10.0), by, back_w, sc(36.0)), "Back", Kind::Secondary) {
            act_back = true;
        }
        self.ui.text(
            "Space play/pause  ·  ← → 5 s  ·  ↑ ↓ speed  ·  R restart  ·  Esc back",
            bar.x,
            bar.y - sc(22.0),
            size::CAPTION,
            Color::new(1.0, 1.0, 1.0, 0.45),
        );

        let v = self.viewer.as_mut().unwrap();
        if act_play {
            if v.t >= dur {
                seek = Some(0.0);
                v.playing = true;
            } else {
                v.playing = !v.playing;
            }
        }
        if let Some(sp) = new_speed {
            v.speed = sp;
        }
        if let Some(t) = seek {
            v.t = t;
            v.next_event = v.replay.events.partition_point(|e| e.t <= t);
        }
        if act_back {
            let from = v.from;
            self.screen = if from == Screen::Replay { Screen::Menu } else { from };
            self.viewer = None;
        }
    }
}
