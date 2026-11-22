//! Main menu: top navigation, scenario browser and playlists.

use crate::app::{App, SettingsPage, SortMode, Tab};
use crate::playlist;
use crate::scenario::{self, Category, FireMode, Source, TargetShape};
use crate::theme::{self, category};
use crate::ui::{fmt_score, hash_id, rrect, size, Kind, Ui};
use macroquad::prelude::*;

const TOPBAR: f32 = 64.0;
const PAD: f32 = 32.0;

impl App {
    pub fn frame_menu(&mut self) {
        self.set_grab(false);
        let t = theme::t();
        clear_background(t.bg);
        if is_key_pressed(KeyCode::F5) {
            self.reload_all();
        }
        let (sw, sh) = (screen_width(), screen_height());
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();

        // Top bar.
        let bar_h = sc(TOPBAR);
        let x0 = sc(PAD);
        let wm_w = self.ui.strong("AimForge", x0, (bar_h - sc(20.0)) * 0.5 - sc(2.0), 19.0, t.text);
        let mut x = x0 + wm_w + sc(28.0);
        for (tab, label) in [(Tab::Scenarios, "Scenarios"), (Tab::Playlists, "Playlists"), (Tab::Settings, "Settings")] {
            let (clicked, w) = self.ui.nav(x, 0.0, bar_h, label, self.tab == tab);
            if clicked {
                self.tab = tab;
                self.page_scroll = 0.0;
            }
            x += w;
        }
        let chip = Rect::new(sw - x0, 0.0, 0.0, bar_h);
        // Update status, top right.
        use crate::update::Status;
        let upd = match &self.updater.status {
            Status::Downloading { version, done, total } => {
                let pct = if *total > 0 { *done as f32 / *total as f32 * 100.0 } else { 0.0 };
                Some((format!("Downloading v{version}  {pct:.0}%"), false))
            }
            Status::Installed { version } => Some((format!("Restart to update to v{version}"), true)),
            Status::Available { version, .. } => Some((format!("v{version} available"), true)),
            _ => None,
        };
        if let Some((label, clickable)) = upd {
            let h = 32.0;
            let w = self.ui.button_w(&label, h);
            let r = Rect::new(chip.x - w - sc(10.0), (bar_h - sc(h)) * 0.5, w, sc(h));
            if clickable {
                if self.ui.button(r, &label, Kind::Primary) {
                    match &self.updater.status {
                        Status::Installed { .. } => {
                            self.settings.save(&self.paths.settings);
                            self.updater.restart();
                        }
                        _ => {
                            self.tab = Tab::Settings;
                            self.settings_page = SettingsPage::Files;
                        }
                    }
                }
            } else {
                rrect(r, sc(3.0), t.surface_hover);
                self.ui.text_mid(&label, r.x + sc(18.0), r, size::SMALL, t.text_dim, true);
            }
        }
        self.ui.divider(0.0, bar_h - 1.0, sw);

        let content = Rect::new(x0, bar_h + sc(24.0), sw - 2.0 * x0, sh - bar_h - sc(24.0) - sc(PAD));
        match self.tab {
            Tab::Scenarios => self.page_scenarios(content),
            Tab::Playlists => self.page_playlists(content),
            Tab::Settings => self.page_settings(content),
        }
    }

    fn filtered(&self) -> Vec<usize> {
        let words: Vec<String> = self.search.to_lowercase().split_whitespace().map(String::from).collect();
        let mut v: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| self.cat_filter.is_none_or(|c| c == e.category))
            .filter(|(_, e)| words.iter().all(|w| e.lower.contains(w.as_str())))
            .map(|(i, _)| i)
            .collect();
        let h = &self.history;
        let e = &self.entries;
        match self.sort {
            SortMode::Name => {}
            SortMode::Plays => v.sort_by_key(|&i| std::cmp::Reverse(h.plays(&e[i].name))),
            SortMode::Recent => {
                v.sort_by(|&a, &b| h.last_played(&e[b].name).unwrap_or("").cmp(h.last_played(&e[a].name).unwrap_or("")))
            }
            SortMode::Best => v.sort_by(|&a, &b| {
                let (x, y) = (h.best(&e[a].name), h.best(&e[b].name));
                y.unwrap_or(f32::MIN).partial_cmp(&x.unwrap_or(f32::MIN)).unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
        v
    }

    // -------------------------------------------------------------- scenarios

    fn page_scenarios(&mut self, r: Rect) {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();

        // Toolbar.
        let search_id = hash_id("search");
        let search_r = Rect::new(r.x, r.y, sc(360.0), sc(40.0));
        let before = self.search.clone();
        self.ui.text_field(search_id, search_r, &mut self.search, "Search scenarios");
        // Start typing anywhere to search.
        if self.ui.focus.is_none() && !self.ui.chars.is_empty() && !is_key_down(KeyCode::LeftControl) {
            self.ui.focus = Some(search_id);
            for c in self.ui.chars.clone() {
                if !c.is_control() {
                    self.search.push(c);
                }
            }
        }
        if self.search != before {
            self.list_scroll = 0.0;
        }
        let cats = ["All", "Clicking", "Tracking", "Switching", "Timing", "Strafe"];
        let cat_idx = self.cat_filter.map(|c| Category::ALL.iter().position(|x| *x == c).unwrap() + 1).unwrap_or(0);
        let seg = Rect::new(search_r.x + search_r.w + sc(12.0), r.y, sc(470.0), sc(40.0));
        if let Some(i) = self.ui.segmented(seg, &cats, cat_idx) {
            self.cat_filter = if i == 0 { None } else { Some(Category::ALL[i - 1]) };
            self.list_scroll = 0.0;
        }
        let sorts = ["Name", "Most played", "Recent", "Best"];
        let modes = [SortMode::Name, SortMode::Plays, SortMode::Recent, SortMode::Best];
        let sort_r = Rect::new(r.x + r.w - sc(330.0), r.y, sc(330.0), sc(40.0));
        if let Some(i) = self.ui.segmented(sort_r, &sorts, modes.iter().position(|m| *m == self.sort).unwrap_or(0)) {
            self.sort = modes[i];
        }

        // List.
        let top = r.y + sc(56.0);
        let list = Rect::new(r.x, top, (r.w * 0.6).round(), r.h - (top - r.y));
        self.ui.card(list);
        let rows = self.filtered();
        let cols = [list.x + sc(20.0), list.x + list.w - sc(340.0), list.x + list.w - sc(190.0), list.x + list.w - sc(110.0)];
        let head_y = list.y + sc(14.0);
        self.ui.text(&format!("{} scenarios", rows.len()), cols[0], head_y, size::CAPTION, t.text_faint);
        self.ui.text("Type", cols[1], head_y, size::CAPTION, t.text_faint);
        self.ui.text("Plays", cols[2], head_y, size::CAPTION, t.text_faint);
        self.ui.text("High score", cols[3], head_y, size::CAPTION, t.text_faint);
        let body = Rect::new(list.x, list.y + sc(40.0), list.w, list.h - sc(48.0));
        let row_h = sc(42.0);
        self.ui.scroll(body, &mut self.list_scroll, rows.len() as f32 * row_h);
        let first = (self.list_scroll / row_h).floor() as usize;
        let visible = (body.h / row_h).ceil() as usize + 1;
        let mut play: Option<String> = None;
        let sel_key = self.selected.as_deref().map(scenario::key);
        for (vi, &ei) in rows.iter().enumerate().skip(first).take(visible) {
            let y = body.y + vi as f32 * row_h - self.list_scroll;
            if y < body.y - 0.5 || y + row_h > body.y + body.h + 0.5 {
                continue;
            }
            let rr = Rect::new(body.x + sc(8.0), y, body.w - sc(20.0), row_h);
            let e = &self.entries[ei];
            let sel = sel_key.as_deref() == Some(e.lower.as_str());
            if sel {
                rrect(rr, sc(3.0), t.surface_active);
            } else if self.ui.hover(rr) {
                rrect(rr, sc(3.0), t.surface_hover);
                self.ui.hand = true;
            }
            let custom = e.source == Source::Custom;
            let tag_w = if custom { self.ui.measure("custom", size::CAPTION, false) + sc(14.0) } else { 0.0 };
            let name = self.ui.fit(&e.name, size::BODY, cols[1] - cols[0] - sc(20.0) - tag_w, false);
            let nw = self.ui.text_mid(&name, cols[0], rr, size::BODY, t.text, false);
            if custom {
                self.ui.text_mid("custom", cols[0] + nw + sc(10.0), rr, size::CAPTION, t.accent, false);
            }
            draw_circle(cols[1] + sc(4.0), rr.y + rr.h * 0.5, sc(3.5), category(e.category));
            self.ui.text_mid(short_cat(e.category), cols[1] + sc(16.0), rr, size::SMALL, t.text_dim, false);
            let plays = self.history.plays(&e.name);
            self.ui.text_mid(&if plays > 0 { plays.to_string() } else { "–".into() }, cols[2], rr, size::SMALL, t.text_dim, false);
            let best = self.history.best(&e.name).map(fmt_score).unwrap_or_else(|| "–".into());
            self.ui.text_mid(&best, cols[3], rr, size::SMALL, if plays > 0 { t.text } else { t.text_faint }, plays > 0);
            if self.ui.click_in(rr) {
                if sel && get_time() - self.last_click < 0.35 {
                    play = Some(e.name.clone());
                }
                self.selected = Some(e.name.clone());
                self.last_click = get_time();
            }
        }
        if rows.is_empty() {
            self.ui.text("Nothing matches that search.", cols[0], body.y + sc(16.0), size::BODY, t.text_dim);
        }
        if is_key_pressed(KeyCode::Enter) && self.ui.focus.is_none() {
            if let Some(n) = self.selected.clone() {
                play = Some(n);
            }
        }
        if let Some(n) = play {
            self.run = None;
            self.start(&n);
            return;
        }

        let d = Rect::new(list.x + list.w + sc(20.0), list.y, r.x + r.w - (list.x + list.w + sc(20.0)), list.h);
        self.scenario_details(d);
    }

    fn scenario_details(&mut self, d: Rect) {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        self.ui.card(d);
        let pad = sc(28.0);
        let inner_w = d.w - 2.0 * pad;
        let Some(name) = self.selected.clone() else {
            self.ui.text("Select a scenario", d.x + pad, d.y + pad, size::HEADING, t.text);
            self.ui.text("Double-click or press Enter to play.", d.x + pad, d.y + pad + sc(28.0), size::SMALL, t.text_dim);
            return;
        };
        if self.sel_cache.as_ref().map(|c| scenario::key(&c.0)) != Some(scenario::key(&name)) {
            let (scn, src) = self.lib.get(&name);
            self.sel_cache = Some((name.clone(), scn, src));
        }
        let (_, scn, src) = self.sel_cache.clone().unwrap();
        let mut y = d.y + pad;
        draw_circle(d.x + pad + sc(4.0), y + sc(8.0), sc(4.0), category(scn.category));
        self.ui.text(scn.category.label(), d.x + pad + sc(16.0), y, size::SMALL, t.text_dim);
        y += sc(28.0);
        for line in self.ui.wrap(&scn.name, size::TITLE, inner_w, true).iter().take(2) {
            self.ui.strong(line, d.x + pad, y, size::TITLE, t.text);
            y += sc(36.0);
        }
        let tg = &scn.targets;
        let what = format!(
            "{:.0} s  ·  {} {}{}  ·  {}  ·  {}",
            scn.duration,
            tg.max_alive,
            if tg.shape == TargetShape::Bot { "bot" } else { "sphere" },
            if tg.max_alive == 1 { "" } else { "s" },
            match scn.weapon.mode {
                FireMode::Semi => "click to shoot",
                FireMode::Auto => "hold to shoot",
            },
            src.label()
        );
        for line in self.ui.wrap(&what, size::SMALL, inner_w, false) {
            self.ui.text(&line, d.x + pad, y, size::SMALL, t.text_dim);
            y += sc(20.0);
        }
        y += sc(6.0);
        for line in self.ui.wrap(&scn.description, size::SMALL, inner_w, false).iter().take(2) {
            self.ui.text(line, d.x + pad, y, size::SMALL, t.text_faint);
            y += sc(20.0);
        }
        y += sc(22.0);

        // High score / plays (one quiet line).
        let runs = self.history.runs(&name);
        let best = self.history.best(&name);
        self.ui.text("High score", d.x + pad, y, size::CAPTION, t.text_faint);
        self.ui.text("Plays", d.x + pad + sc(170.0), y, size::CAPTION, t.text_faint);
        y += sc(18.0);
        self.ui.strong(&best.map(fmt_score).unwrap_or_else(|| "–".into()), d.x + pad, y, 26.0, t.text);
        self.ui.strong(&runs.len().to_string(), d.x + pad + sc(170.0), y, 26.0, t.text);
        y += sc(50.0);

        // Graph.
        let btn_area = sc(124.0);
        let gh = (d.y + d.h - pad - btn_area - y).max(sc(40.0));
        let scores: Vec<f32> = runs.iter().rev().take(30).rev().map(|r| r.score).collect();
        draw_graph(&self.ui, Rect::new(d.x + pad + sc(44.0), y + sc(8.0), inner_w - sc(44.0), gh - sc(16.0)), &scores);

        // Actions.
        let by = d.y + d.h - pad - sc(104.0);
        if self.ui.button(Rect::new(d.x + pad, by, inner_w, sc(48.0)), "Play", Kind::Primary) {
            self.run = None;
            self.start(&name);
            return;
        }
        let half = (inner_w - sc(10.0)) * 0.5;
        let by2 = by + sc(58.0);
        let edit_label = if src == Source::Custom { "Show file" } else { "Export to file" };
        if self.ui.button(Rect::new(d.x + pad, by2, half, sc(42.0)), edit_label, Kind::Secondary) {
            if src == Source::Custom {
                if let Some(p) = self.lib.custom_path(&name) {
                    crate::open_path(p.parent().unwrap_or(&self.paths.scenarios));
                }
            } else {
                match self.lib.export(&scn) {
                    Ok(p) => {
                        self.refresh_entries();
                        self.toast(format!("Saved {} (edit it, then press F5)", p.display()));
                    }
                    Err(e) => self.toast(format!("Export failed: {e}")),
                }
            }
        }
        if self.ui.button(Rect::new(d.x + pad + half + sc(10.0), by2, half, sc(42.0)), "Add to My Playlist", Kind::Secondary) {
            self.add_to_my_playlist(&name);
        }
    }

    fn add_to_my_playlist(&mut self, name: &str) {
        let path = self.paths.playlists.join("My Playlist.json");
        let mut entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| playlist::parse_json(&t).ok())
            .map(|x| x.1)
            .unwrap_or_default();
        entries.push(playlist::Entry { name: name.to_string(), count: 1 });
        match playlist::save_json(&path, "My Playlist", &entries) {
            Ok(()) => {
                let (pl, _) = playlist::load_dir(&self.paths.playlists);
                self.playlists = pl;
                self.toast(format!("Added to My Playlist ({} scenarios)", entries.len()));
            }
            Err(e) => self.toast(format!("Could not save playlist: {e}")),
        }
    }

    // -------------------------------------------------------------- playlists

    fn page_playlists(&mut self, r: Rect) {
        let t = theme::t();
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        if self.playlists.is_empty() {
            self.ui.card(r);
            let x = r.x + sc(32.0);
            self.ui.strong("No playlists yet", x, r.y + sc(32.0), size::TITLE, t.text);
            self.ui.text("Drop KovaaK's playlist files (.json or .plo) into this folder, then press F5:", x, r.y + sc(80.0), size::BODY, t.text_dim);
            self.ui.text(&self.paths.playlists.display().to_string(), x, r.y + sc(108.0), size::BODY, t.text);
            let busy = self.playlist_job.is_some();
            let label = if busy { "Downloading..." } else { "Download community playlists" };
            if self.ui.button(Rect::new(x, r.y + sc(150.0), self.ui.button_w(label, 42.0), sc(42.0)), label, Kind::Primary) && !busy {
                self.download_playlists();
            }
            let bx = x + self.ui.button_w(label, 42.0) + sc(10.0);
            if self.ui.button(Rect::new(bx, r.y + sc(150.0), sc(160.0), sc(42.0)), "Open folder", Kind::Secondary) {
                crate::open_path(&self.paths.playlists);
            }
            return;
        }
        // Left: playlists grouped by folder.
        let list = Rect::new(r.x, r.y, sc(380.0), r.h);
        self.ui.card(list);
        let row_h = sc(38.0);
        let group_h = sc(40.0);
        let mut rows: Vec<(Option<usize>, String)> = vec![];
        let mut last_group: Option<&str> = None;
        for (i, p) in self.playlists.iter().enumerate() {
            if last_group != Some(p.group.as_str()) {
                rows.push((None, if p.group.is_empty() { "Playlists".into() } else { pretty_group(&p.group) }));
                last_group = Some(p.group.as_str());
            }
            rows.push((Some(i), p.name.clone()));
        }
        let heights: Vec<f32> = rows.iter().map(|(i, _)| if i.is_some() { row_h } else { group_h }).collect();
        let area = Rect::new(list.x, list.y + sc(8.0), list.w, list.h - sc(16.0));
        self.ui.scroll(area, &mut self.pl_scroll, heights.iter().sum::<f32>());
        let mut y = area.y - self.pl_scroll;
        for ((idx, label), h) in rows.iter().zip(&heights) {
            if y >= area.y - 0.5 && y + h <= area.y + area.h + 0.5 {
                match idx {
                    None => {
                        self.ui.text(label, list.x + sc(20.0), y + sc(18.0), size::CAPTION, t.text_faint);
                    }
                    Some(i) => {
                        let rr = Rect::new(list.x + sc(8.0), y, list.w - sc(20.0), *h);
                        let sel = *i == self.pl_selected;
                        if sel {
                            rrect(rr, sc(3.0), t.surface_active);
                        } else if self.ui.hover(rr) {
                            rrect(rr, sc(3.0), t.surface_hover);
                            self.ui.hand = true;
                        }
                        let fitted = self.ui.fit(label, size::BODY, rr.w - sc(64.0), false);
                        self.ui.text_mid(&fitted, rr.x + sc(12.0), rr, size::BODY, if sel { t.text } else { t.text_dim }, false);
                        let n = self.playlists[*i].entries.len();
                        self.ui.text_right(&n.to_string(), rr.x + rr.w - sc(12.0), rr.y + (rr.h - sc(size::SMALL)) * 0.5 - sc(1.0), size::SMALL, t.text_faint, false);
                        if self.ui.click_in(rr) {
                            if self.pl_selected != *i {
                                self.pl_entry_scroll = 0.0;
                            }
                            self.pl_selected = *i;
                        }
                    }
                }
            }
            y += h;
        }

        // Right: contents.
        let d = Rect::new(list.x + list.w + sc(20.0), r.y, r.x + r.w - (list.x + list.w + sc(20.0)), r.h);
        self.ui.card(d);
        let pi = self.pl_selected.min(self.playlists.len() - 1);
        let p = self.playlists[pi].clone();
        let pad = sc(28.0);
        let title = self.ui.fit(&p.name, size::TITLE, d.w - pad * 2.0 - sc(200.0), true);
        self.ui.strong(&title, d.x + pad, d.y + pad, size::TITLE, t.text);
        self.ui.text(
            &format!("{} scenarios  ·  {} runs  ·  about {} min", p.entries.len(), p.total_runs(), p.total_runs()),
            d.x + pad,
            d.y + pad + sc(40.0),
            size::SMALL,
            t.text_dim,
        );
        if self.ui.button(Rect::new(d.x + d.w - pad - sc(180.0), d.y + pad, sc(180.0), sc(44.0)), "Start playlist", Kind::Primary) {
            self.start_playlist(pi, 0);
            return;
        }
        let body = Rect::new(d.x, d.y + sc(108.0), d.w, d.h - sc(116.0));
        self.ui.divider(d.x + pad, body.y - sc(8.0), d.w - 2.0 * pad);
        let rh = sc(44.0);
        self.ui.scroll(body, &mut self.pl_entry_scroll, p.entries.len() as f32 * rh);
        let cx = [d.x + pad, d.x + pad + sc(36.0), d.x + d.w - sc(330.0), d.x + d.w - sc(250.0)];
        let mut start: Option<usize> = None;
        for (i, e) in p.entries.iter().enumerate() {
            let y = body.y + i as f32 * rh - self.pl_entry_scroll;
            if y < body.y - 0.5 || y + rh > body.y + body.h + 0.5 {
                continue;
            }
            let rr = Rect::new(d.x + sc(12.0), y, d.w - sc(24.0), rh);
            let hov = self.ui.hover(rr);
            if hov {
                rrect(rr, sc(3.0), t.surface_hover);
            }
            let cat = self
                .entries
                .iter()
                .find(|x| x.lower == scenario::key(&e.name))
                .map(|x| x.category)
                .unwrap_or(Category::Clicking);
            self.ui.text_mid(&format!("{}", i + 1), cx[0], rr, size::SMALL, t.text_faint, false);
            draw_circle(cx[1] - sc(12.0), rr.y + rr.h * 0.5, sc(3.0), category(cat));
            self.ui.text_mid(&self.ui.fit(&e.name, size::BODY, cx[2] - cx[1] - sc(16.0), false), cx[1], rr, size::BODY, t.text, false);
            if e.count > 1 {
                self.ui.text_mid(&format!("×{}", e.count), cx[2], rr, size::SMALL, t.text_dim, false);
            }
            let best = self.history.best(&e.name).map(fmt_score).unwrap_or_else(|| "–".into());
            self.ui.text_mid(&best, cx[3], rr, size::SMALL, t.text_dim, false);
            if hov {
                let br = Rect::new(rr.x + rr.w - sc(130.0), rr.y + sc(6.0), sc(122.0), rh - sc(12.0));
                if self.ui.button(br, "Start here", Kind::Secondary) {
                    start = Some(i);
                }
            }
        }
        if let Some(i) = start {
            self.start_playlist(pi, i);
        }
    }
}

fn short_cat(c: Category) -> &'static str {
    match c {
        Category::Clicking => "Clicking",
        Category::Tracking => "Tracking",
        Category::Switching => "Switching",
        Category::Timing => "Timing",
        Category::Strafe => "Strafe",
    }
}

fn pretty_group(g: &str) -> String {
    g.rsplit('/')
        .next()
        .unwrap_or(g)
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Score line chart with best line.
pub fn draw_graph(ui: &Ui, r: Rect, scores: &[f32]) {
    let t = theme::t();
    if scores.is_empty() {
        let msg = "Your score history will appear here";
        let cx = r.x - ui.s(22.0) + r.w * 0.5;
        ui.text_center(msg, cx, r.y + r.h * 0.45, size::SMALL, t.text_faint, false);
        return;
    }
    let mx = scores.iter().cloned().fold(f32::MIN, f32::max);
    let mn = scores.iter().cloned().fold(f32::MAX, f32::min);
    let pad = ((mx - mn) * 0.15).max(mx.abs() * 0.05).max(1.0);
    let (lo, hi) = (mn - pad, mx + pad);
    let n = scores.len();
    let px = |i: usize| if n <= 1 { r.x + r.w * 0.5 } else { r.x + r.w * i as f32 / (n - 1) as f32 };
    let py = |v: f32| r.y + r.h - (v - lo) / (hi - lo) * r.h;
    for k in 0..=3 {
        let y = r.y + r.h * k as f32 / 3.0;
        draw_line(r.x, y, r.x + r.w, y, 1.0, theme::with_alpha(t.border, 0.7));
    }
    ui.text_right(&fmt_score(mx), r.x - ui.s(10.0), py(mx) - ui.s(8.0), size::CAPTION, t.text_faint, false);
    if py(mn) - py(mx) > ui.s(22.0) {
        ui.text_right(&fmt_score(mn), r.x - ui.s(10.0), py(mn) - ui.s(8.0), size::CAPTION, t.text_faint, false);
    }
    // soft fill
    for i in 1..n {
        let (x0, x1) = (px(i - 1), px(i));
        let (y0, y1) = (py(scores[i - 1]), py(scores[i]));
        let base = r.y + r.h;
        let fill = theme::with_alpha(t.accent, 0.10);
        draw_triangle(vec2(x0, y0), vec2(x1, y1), vec2(x0, base), fill);
        draw_triangle(vec2(x1, y1), vec2(x1, base), vec2(x0, base), fill);
    }
    for i in 1..n {
        draw_line(px(i - 1), py(scores[i - 1]), px(i), py(scores[i]), ui.s(2.0), t.accent);
    }
    let best_i = scores.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|x| x.0).unwrap_or(0);
    draw_circle(px(best_i), py(scores[best_i]), ui.s(4.5), t.good);
    let last = n - 1;
    draw_circle(px(last), py(scores[last]), ui.s(4.5), t.text);
}
