//! Leaderboards: global (online) and personal boards, shared table widget.

use crate::app::{App, BoardMode};
use crate::scenario;
use crate::theme;
use crate::ui::{fmt_score, hash_id, rrect, size, Kind};
use macroquad::prelude::*;

#[derive(Clone)]
pub enum ReplayRef {
    None,
    Local(String),
    Remote(String),
}

#[derive(Clone)]
pub struct Row {
    pub rank: usize,
    pub player: String,
    pub score: f32,
    pub accuracy: f32,
    pub kills: i64,
    pub cm360: f32,
    pub date: String,
    pub me: bool,
    pub replay: ReplayRef,
}

impl App {
    /// Rows for the global board of a scenario.
    pub fn global_rows(&self, name: &str) -> Vec<Row> {
        let Some(st) = self.boards.get(&scenario::key(name)) else { return vec![] };
        st.entries
            .iter()
            .enumerate()
            .map(|(i, e)| Row {
                rank: i + 1,
                player: e.player.clone(),
                score: e.score,
                accuracy: e.accuracy,
                kills: e.kills as i64,
                cm360: e.cm360,
                date: e.date(),
                me: e.player_id == self.settings.player_id,
                replay: e.replay_path.clone().map(ReplayRef::Remote).unwrap_or(ReplayRef::None),
            })
            .collect()
    }

    /// Rows for your own runs of a scenario, best first.
    pub fn local_rows(&self, name: &str) -> Vec<Row> {
        let mut runs: Vec<_> = self.history.runs(name).iter().collect();
        runs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let me = if self.settings.player_name.is_empty() { "You".to_string() } else { self.settings.player_name.clone() };
        runs.iter()
            .enumerate()
            .map(|(i, r)| Row {
                rank: i + 1,
                player: me.clone(),
                score: r.score,
                accuracy: r.accuracy,
                kills: r.kills as i64,
                cm360: r.cm360,
                date: r.date.chars().take(10).collect(),
                me: false,
                replay: r.replay.clone().map(ReplayRef::Local).unwrap_or(ReplayRef::None),
            })
            .collect()
    }

    pub fn watch(&mut self, r: &ReplayRef, label: String) {
        match r {
            ReplayRef::None => {}
            ReplayRef::Local(rel) => self.open_local_replay(rel, label),
            ReplayRef::Remote(path) => {
                self.toast("Loading replay...");
                self.board.fetch_replay(path, &label);
            }
        }
    }

    /// Draw a board. Returns the replay the user asked to watch, with a label.
    pub fn board_table(&mut self, r: Rect, rows: &[Row], scroll_id: u8, compact: bool) -> Option<(ReplayRef, String)> {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        let cols: Vec<(&str, f32)> = if compact {
            vec![("#", 0.0), ("Player", 40.0), ("Score", r.w - 300.0 / 1.0 * s), ("Acc", r.w - 190.0 * s), ("", r.w - 96.0 * s)]
        } else {
            vec![
                ("#", 0.0),
                ("Player", 54.0 * s),
                ("Score", r.w * 0.38),
                ("Accuracy", r.w * 0.52),
                ("Kills", r.w * 0.64),
                ("cm/360", r.w * 0.73),
                ("Date", r.w * 0.82),
                ("", r.w - 96.0 * s),
            ]
        };
        let col_x = |i: usize| r.x + if compact && i == 1 { sc(40.0) } else { cols[i].1 };
        for (i, (label, _)) in cols.iter().enumerate() {
            self.ui.text(label, col_x(i) + sc(10.0), r.y, size::CAPTION, t.text_faint);
        }
        let body = Rect::new(r.x, r.y + sc(22.0), r.w, r.h - sc(22.0));
        let rh = sc(if compact { 34.0 } else { 40.0 });
        let mut scroll = if scroll_id == 0 { self.board_scroll } else { 0.0 };
        if scroll_id == 0 {
            self.ui.scroll(body, &mut scroll, rows.len() as f32 * rh);
            self.board_scroll = scroll;
        }
        let mut out = None;
        for (i, row) in rows.iter().enumerate() {
            let y = body.y + i as f32 * rh - scroll;
            if y < body.y - 0.5 || y + rh > body.y + body.h + 0.5 {
                continue;
            }
            let rr = Rect::new(r.x, y, r.w, rh);
            if row.me {
                rrect(rr, sc(3.0), theme::with_alpha(t.accent, 0.14));
            } else if self.ui.hover(rr) {
                rrect(rr, sc(3.0), t.surface_hover);
            }
            let rank_col = match row.rank {
                1 => Color::from_rgba(255, 205, 60, 255),
                2 => Color::from_rgba(200, 208, 220, 255),
                3 => Color::from_rgba(214, 140, 80, 255),
                _ => t.text_faint,
            };
            self.ui.text_mid(&row.rank.to_string(), col_x(0) + sc(10.0), rr, size::SMALL, rank_col, row.rank <= 3);
            let pw = if compact { col_x(2) - col_x(1) - sc(16.0) } else { col_x(2) - col_x(1) - sc(16.0) };
            let pname = self.ui.fit(&row.player, size::BODY, pw, row.me);
            self.ui.text_mid(&pname, col_x(1) + sc(10.0), rr, size::BODY, t.text, row.me);
            self.ui.text_mid(&fmt_score(row.score), col_x(2) + sc(10.0), rr, size::BODY, t.text, true);
            self.ui.text_mid(&format!("{:.1}%", row.accuracy * 100.0), col_x(3) + sc(10.0), rr, size::SMALL, t.text_dim, false);
            if !compact {
                self.ui.text_mid(&row.kills.to_string(), col_x(4) + sc(10.0), rr, size::SMALL, t.text_dim, false);
                self.ui.text_mid(&format!("{:.1}", row.cm360), col_x(5) + sc(10.0), rr, size::SMALL, t.text_dim, false);
                self.ui.text_mid(&row.date, col_x(6) + sc(10.0), rr, size::SMALL, t.text_faint, false);
            }
            if !matches!(row.replay, ReplayRef::None) {
                let br = Rect::new(r.x + r.w - sc(86.0), y + sc(4.0), sc(80.0), rh - sc(8.0));
                if self.ui.button(br, "Watch", Kind::Ghost) {
                    out = Some((row.replay.clone(), format!("#{} {}", row.rank, row.player)));
                }
            }
        }
        if rows.is_empty() {
            self.ui.text("No runs yet.", r.x + sc(10.0), body.y + sc(10.0), size::SMALL, t.text_faint);
        }
        out
    }

    /// Inline "choose a name" prompt. Returns true when a name was saved.
    pub fn name_prompt(&mut self, x: f32, y: f32, w: f32) -> bool {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        self.ui.text("Pick a name for the leaderboards", x, y, size::SMALL, t.text_dim);
        let field = Rect::new(x, y + sc(22.0), w - sc(110.0), sc(38.0));
        let submitted = self.ui.text_field(hash_id("lbname"), field, &mut self.name_buf, "Your name (max 24)");
        let save = self.ui.button(Rect::new(x + w - sc(100.0), y + sc(22.0), sc(100.0), sc(38.0)), "Save", Kind::Primary);
        if (save || submitted) && !crate::leaderboard::clean_name(&self.name_buf).is_empty() {
            self.settings.player_name = crate::leaderboard::clean_name(&self.name_buf);
            self.settings_dirty = true;
            self.name_buf.clear();
            return true;
        }
        false
    }

    pub fn page_boards(&mut self, r: Rect) {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();

        // Left: scenario picker.
        let list = Rect::new(r.x, r.y, sc(340.0), r.h);
        self.ui.card(list);
        let sf = Rect::new(list.x + sc(12.0), list.y + sc(12.0), list.w - sc(24.0), sc(36.0));
        let before = self.board_search.clone();
        self.ui.text_field(hash_id("bsearch"), sf, &mut self.board_search, "Search scenarios");
        if before != self.board_search {
            self.board_list_scroll = 0.0;
        }
        let words: Vec<String> = self.board_search.to_lowercase().split_whitespace().map(String::from).collect();
        // played scenarios first
        let mut names: Vec<(bool, String)> = self
            .entries
            .iter()
            .filter(|e| words.iter().all(|w| e.lower.contains(w.as_str())))
            .map(|e| (self.history.plays(&e.name) > 0, e.name.clone()))
            .collect();
        names.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_lowercase().cmp(&b.1.to_lowercase())));
        if self.board_scenario.is_none() {
            self.board_scenario = self.selected.clone().or_else(|| names.first().map(|n| n.1.clone()));
        }
        let body = Rect::new(list.x, sf.y + sf.h + sc(10.0), list.w, list.y + list.h - (sf.y + sf.h + sc(10.0)) - sc(8.0));
        let rh = sc(36.0);
        let mut scroll = self.board_list_scroll;
        self.ui.scroll(body, &mut scroll, names.len() as f32 * rh);
        self.board_list_scroll = scroll;
        let sel = self.board_scenario.clone().map(|n| scenario::key(&n));
        for (i, (played, n)) in names.iter().enumerate() {
            let y = body.y + i as f32 * rh - scroll;
            if y < body.y - 0.5 || y + rh > body.y + body.h + 0.5 {
                continue;
            }
            let rr = Rect::new(list.x + sc(8.0), y, list.w - sc(20.0), rh);
            let active = sel.as_deref() == Some(scenario::key(n).as_str());
            if active {
                rrect(rr, sc(3.0), t.surface_active);
            } else if self.ui.hover(rr) {
                rrect(rr, sc(3.0), t.surface_hover);
                self.ui.hand = true;
            }
            let col = if active || *played { t.text } else { t.text_dim };
            let label = self.ui.fit(n, size::BODY, rr.w - sc(24.0), false);
            self.ui.text_mid(&label, rr.x + sc(12.0), rr, size::BODY, col, active);
            if self.ui.click_in(rr) {
                self.board_scenario = Some(n.clone());
                self.board_scroll = 0.0;
            }
        }

        // Right: board.
        let d = Rect::new(list.x + list.w + sc(20.0), r.y, r.x + r.w - (list.x + list.w + sc(20.0)), r.h);
        self.ui.card(d);
        let pad = sc(28.0);
        let Some(name) = self.board_scenario.clone() else { return };
        let title = self.ui.fit(&name, size::TITLE, d.w - pad * 2.0 - sc(330.0), true);
        self.ui.strong(&title, d.x + pad, d.y + pad, size::TITLE, t.text);
        let mode_r = Rect::new(d.x + d.w - pad - sc(220.0), d.y + pad, sc(220.0), sc(36.0));
        let idx = if self.board_mode == BoardMode::Global { 0 } else { 1 };
        if let Some(i) = self.ui.segmented(mode_r, &["Global", "Your runs"], idx) {
            self.board_mode = if i == 0 { BoardMode::Global } else { BoardMode::Mine };
            self.board_scroll = 0.0;
        }
        let mut y = d.y + pad + sc(52.0);
        let rows = match self.board_mode {
            BoardMode::Mine => {
                self.ui.text("Your runs on this PC, best first. Replays are kept for your 10 best and 3 latest runs.", d.x + pad, y, size::SMALL, t.text_faint);
                y += sc(32.0);
                self.local_rows(&name)
            }
            BoardMode::Global => {
                if !self.board.online() {
                    self.ui.text("Online leaderboards aren't connected in this build yet.", d.x + pad, y, size::BODY, t.text_dim);
                    self.ui.text("Your own runs and replays are under \"Your runs\".", d.x + pad, y + sc(26.0), size::SMALL, t.text_faint);
                    return;
                }
                self.ensure_board(&name, false);
                let st = self.boards.get(&scenario::key(&name));
                let status = match st {
                    Some(s) if s.loading => "Loading...".to_string(),
                    Some(s) if s.error.is_some() => s.error.clone().unwrap_or_default(),
                    Some(s) => format!("{} players  ·  best run per player", s.entries.len()),
                    None => String::new(),
                };
                self.ui.text(&status, d.x + pad, y, size::SMALL, t.text_faint);
                if self.ui.button(Rect::new(d.x + d.w - pad - sc(90.0), y - sc(6.0), sc(90.0), sc(30.0)), "Refresh", Kind::Ghost) {
                    self.ensure_board(&name, true);
                }
                y += sc(32.0);
                if self.settings.player_name.is_empty() {
                    if self.name_prompt(d.x + pad, y, (d.w - pad * 2.0).min(sc(460.0))) {
                        self.toast("Name saved. Your next runs go on the leaderboard.");
                    }
                    y += sc(76.0);
                }
                self.global_rows(&name)
            }
        };
        let table = Rect::new(d.x + pad, y, d.w - pad * 2.0, d.y + d.h - y - pad);
        if let Some((rp, label)) = self.board_table(table, &rows, 0, false) {
            self.watch(&rp, format!("{label}  ·  {name}"));
        }
    }
}
