//! Post-run summary.

use super::menu::draw_graph;
use crate::app::{App, Screen, SubmitState};
use crate::screens::boards::ReplayRef;
use crate::theme::{self, category};
use crate::ui::{fmt_score, rrect, size, Kind};
use macroquad::prelude::*;

impl App {
    pub fn frame_results(&mut self) {
        self.set_grab(false);
        let t = theme::t();
        clear_background(t.bg);
        let Some(res) = &self.result else {
            self.screen = Screen::Menu;
            return;
        };
        let (sw, sh) = (screen_width(), screen_height());
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        let pad = sc(48.0);
        let rec = res.rec.clone();
        let name = res.name.clone();
        let prev_best = res.prev_best;
        let cat = res.category;
        let tracking = res.tracking;
        let csv = res.csv.clone();

        // Header.
        let mut y = sc(40.0);
        draw_circle(pad + sc(4.0), y + sc(8.0), sc(4.0), category(cat));
        self.ui.text(&format!("{}  ·  Run complete", cat.label()), pad + sc(16.0), y, size::SMALL, t.text_dim);
        y += sc(26.0);
        let title = self.ui.fit(&name, size::TITLE + 6.0, sw - 2.0 * pad, true);
        self.ui.strong(&title, pad, y, size::TITLE + 6.0, t.text);
        y += sc(64.0);

        // Score card.
        let left_w = sc(560.0).min(sw * 0.42);
        let card = Rect::new(pad, y, left_w, sc(210.0));
        self.ui.card(card);
        let cx = card.x + sc(28.0);
        self.ui.text("Score", cx, card.y + sc(24.0), size::SMALL, t.text_dim);
        self.ui.strong(&fmt_score(rec.score), cx - sc(2.0), card.y + sc(48.0), size::DISPLAY, t.text);
        let new_best = prev_best.is_none_or(|b| rec.score > b);
        let (msg, col) = match prev_best {
            None => ("First run on this scenario".to_string(), t.good),
            Some(b) if new_best => (format!("New high score  +{}", fmt_score(rec.score - b)), t.good),
            Some(b) if b.abs() < 1e-6 || (rec.score - b).abs() < 1e-6 => (format!("Matched your high score of {}", fmt_score(b)), t.text_dim),
            Some(b) => (format!("{:.1}% of your high score ({})", rec.score / b * 100.0, fmt_score(b)), t.text_dim),
        };
        if new_best {
            let w = self.ui.measure(&msg, size::SMALL, true) + sc(24.0);
            let pill = Rect::new(cx, card.y + sc(140.0), w, sc(30.0));
            rrect(pill, sc(15.0), theme::with_alpha(col, 0.16));
            self.ui.text_mid(&msg, pill.x + sc(12.0), pill, size::SMALL, col, true);
        } else {
            self.ui.text(&msg, cx, card.y + sc(146.0), size::SMALL, col);
        }

        // Stats card.
        let mut items: Vec<(&str, String)> = vec![
            ("Accuracy", format!("{:.1}%", rec.accuracy * 100.0)),
            ("Hits / shots", format!("{} / {}", rec.hits, rec.shots)),
        ];
        if tracking {
            let pct = if rec.damage_possible > 0.0 { rec.damage_done / rec.damage_possible * 100.0 } else { 0.0 };
            items.push(("Damage", format!("{:.0} ({pct:.1}%)", rec.damage_done)));
        }
        if rec.kills > 0 || !tracking {
            items.push(("Kills", rec.kills.to_string()));
            items.push(("Kills / second", format!("{:.2}", rec.kills as f32 / rec.duration.max(0.1))));
            items.push(("Average time to kill", format!("{:.3} s", rec.avg_ttk)));
        }
        items.push(("Sensitivity", format!("{:.2} cm/360", rec.cm360)));
        items.push(("FOV", crate::settings::trim_num(rec.hfov as f64)));
        let rows = items.len().div_ceil(2);
        let stats = Rect::new(pad, card.y + card.h + sc(16.0), left_w, sc(40.0) + rows as f32 * sc(58.0));
        self.ui.card(stats);
        let colw = (left_w - sc(56.0)) / 2.0;
        for (i, (k, v)) in items.iter().enumerate() {
            let x = stats.x + sc(28.0) + (i % 2) as f32 * colw;
            let yy = stats.y + sc(22.0) + (i / 2) as f32 * sc(58.0);
            self.ui.text(k, x, yy, size::CAPTION, t.text_faint);
            self.ui.strong(v, x, yy + sc(20.0), 17.0, t.text);
        }

        // Right card: leaderboard / your runs / history.
        let gx = pad + left_w + sc(20.0);
        let gr = Rect::new(gx, card.y, sw - gx - pad, stats.y + stats.h - card.y);
        self.ui.card(gr);
        let tabs = ["Leaderboard", "Your runs", "History"];
        if let Some(i) = self.ui.segmented(Rect::new(gr.x + sc(20.0), gr.y + sc(16.0), sc(330.0), sc(34.0)), &tabs, self.results_tab) {
            self.results_tab = i;
        }
        let submit = self.result.as_ref().map(|r| r.submit.clone()).unwrap_or(SubmitState::Offline);
        let inner = Rect::new(gr.x + sc(20.0), gr.y + sc(66.0), gr.w - sc(40.0), gr.h - sc(86.0));
        let mut watch: Option<(ReplayRef, String)> = None;
        match self.results_tab {
            0 => {
                let (line, col) = match &submit {
                    SubmitState::Offline => ("Online leaderboards aren't connected yet. Your run and replay are saved locally.".to_string(), t.text_faint),
                    SubmitState::Disabled => ("Score submission is off (Settings > Gameplay).".to_string(), t.text_faint),
                    SubmitState::NeedName => ("Playing as guest: this run is saved on this PC.".to_string(), t.text_dim),
                    SubmitState::Sending => ("Submitting...".to_string(), t.text_dim),
                    SubmitState::Done { rank, total } => (format!("You're #{rank} of {total} players"), t.good),
                    SubmitState::Failed(e) => (format!("Not submitted: {e}"), t.bad),
                };
                let line = self.ui.fit(&line, size::SMALL, inner.w, true);
                self.ui.strong(&line, inner.x, inner.y, size::SMALL, col);
                let mut top = inner.y + sc(30.0);
                if submit == SubmitState::NeedName {
                    self.sign_in_prompt(inner.x, top, inner.w.min(sc(560.0)), "Sign in and this run goes on the global board.", Screen::Results);
                    top += sc(54.0);
                }
                if self.board.online() {
                    let rows: Vec<_> = self.global_rows(&name).into_iter().take(50).collect();
                    let area = Rect::new(inner.x, top, inner.w, inner.y + inner.h - top);
                    watch = self.board_table(area, &rows, 0, true);
                }
            }
            1 => {
                let rows = self.local_rows(&name);
                watch = self.board_table(inner, &rows, 0, true);
            }
            _ => {
                let runs = self.history.runs(&name);
                let n10 = runs.len().min(10);
                let avg = if n10 > 0 { runs[runs.len() - n10..].iter().map(|r| r.score).sum::<f32>() / n10 as f32 } else { 0.0 };
                self.ui.text(&format!("{} plays  ·  last {} avg {}", runs.len(), n10, fmt_score(avg)), inner.x, inner.y, size::SMALL, t.text_faint);
                let scores: Vec<f32> = runs.iter().rev().take(40).rev().map(|r| r.score).collect();
                draw_graph(&self.ui, Rect::new(inner.x + sc(52.0), inner.y + sc(36.0), inner.w - sc(60.0), inner.h - sc(50.0)), &scores);
            }
        }
        if let Some(p) = &csv {
            let line = self.ui.fit(&format!("Saved to {}", p.display()), size::CAPTION, gr.w, false);
            self.ui.text(&line, gr.x, gr.y + gr.h + sc(10.0), size::CAPTION, t.text_faint);
        }
        if let Some((rp, label)) = watch {
            self.watch(&rp, format!("{label}  ·  {name}"));
            return;
        }

        // Actions (bottom right).
        let bh = sc(48.0);
        let by = sh - pad - bh;
        let next = self.next_in_playlist();
        let mut x = sw - pad;
        let mut act = 0;
        if let Some((_, _, nn)) = &next {
            let label = format!("Next: {nn}");
            let w = self.ui.button_w(&label, 48.0).clamp(sc(200.0), sc(440.0));
            x -= w;
            if self.ui.button(Rect::new(x, by, w, bh), &label, Kind::Primary) {
                act = 2;
            }
            x -= sc(10.0);
        }
        let w = sc(150.0);
        x -= w;
        if self.ui.button(Rect::new(x, by, w, bh), "Play again", if next.is_some() { Kind::Secondary } else { Kind::Primary }) {
            act = 1;
        }
        x -= sc(10.0) + w;
        let has_replay = self.result.as_ref().is_some_and(|r| r.replay.is_some());
        if has_replay && self.ui.button(Rect::new(x, by, w, bh), "Watch replay", Kind::Secondary) {
            act = 4;
        }
        x -= sc(10.0) + w;
        if self.ui.button(Rect::new(x, by, w, bh), "Menu", Kind::Ghost) {
            act = 3;
        }
        self.ui.text(
            if next.is_some() { "Enter next  ·  R again  ·  V replay  ·  Esc menu" } else { "R or Space again  ·  V replay  ·  Esc menu" },
            pad,
            by + sc(16.0),
            size::SMALL,
            t.text_faint,
        );
        if !self.ui.typing() {
            if is_key_pressed(KeyCode::R) || (is_key_pressed(KeyCode::Space) && next.is_none()) {
                act = 1;
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                act = if next.is_some() { 2 } else { 1 };
            }
            if is_key_pressed(KeyCode::Escape) {
                act = 3;
            }
            if is_key_pressed(KeyCode::V) && has_replay {
                act = 4;
            }
        }
        match act {
            4 => {
                if let Some(rp) = self.result.as_ref().and_then(|r| r.replay.clone()) {
                    self.open_replay(rp, "Your last run".into());
                }
            }
            1 => self.restart(),
            2 => self.advance_playlist(),
            3 => {
                self.screen = Screen::Menu;
                if next.is_none() {
                    self.run = None;
                }
                self.selected = Some(name);
            }
            _ => {}
        }
    }
}
