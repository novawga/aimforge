//! Application state and the parts shared by every screen.

use crate::game::Game;
use crate::input::Input;
use crate::playlist::{self, Playlist};
use crate::scenario::{self, Category, FireMode, Library, Source};
use crate::settings::Settings;
use crate::stats::{self, History, RunRecord};
use crate::theme::{self, NamedTheme};
use crate::ui::{rrect, size, Ui};
use crate::update::{self, Updater};
use crate::{audio, render, Paths};
use macroquad::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Playing,
    Results,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Scenarios,
    Playlists,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    Sensitivity,
    Crosshair,
    Theme,
    Gameplay,
    VideoAudio,
    Files,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 6] = [
        SettingsPage::Sensitivity,
        SettingsPage::Crosshair,
        SettingsPage::Theme,
        SettingsPage::Gameplay,
        SettingsPage::VideoAudio,
        SettingsPage::Files,
    ];
    pub fn label(self) -> &'static str {
        match self {
            SettingsPage::Sensitivity => "Sensitivity & FOV",
            SettingsPage::Crosshair => "Crosshair",
            SettingsPage::Theme => "Theme",
            SettingsPage::Gameplay => "Gameplay",
            SettingsPage::VideoAudio => "Video & audio",
            SettingsPage::Files => "Files & controls",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Plays,
    Recent,
    Best,
}

pub struct BrowserEntry {
    pub name: String,
    pub lower: String,
    pub category: Category,
    pub source: Source,
}

pub struct PlaylistRun {
    pub pl: usize,
    pub entry: usize,
    pub rep: u32,
}

pub struct ResultInfo {
    pub name: String,
    pub rec: RunRecord,
    pub prev_best: Option<f32>,
    pub csv: Option<PathBuf>,
    pub category: Category,
    pub tracking: bool,
}

pub struct App {
    pub paths: Paths,
    pub settings: Settings,
    pub settings_dirty: bool,
    pub lib: Library,
    pub playlists: Vec<Playlist>,
    pub themes: Vec<NamedTheme>,
    pub load_errors: Vec<String>,
    pub history: History,
    pub ui: Ui,
    pub input: Input,
    pub renderer: render::Renderer,
    pub sounds: audio::Sounds,

    pub screen: Screen,
    pub tab: Tab,
    pub settings_page: SettingsPage,

    pub entries: Vec<BrowserEntry>,
    pub search: String,
    pub cat_filter: Option<Category>,
    pub sort: SortMode,
    pub list_scroll: f32,
    pub selected: Option<String>,
    pub sel_cache: Option<(String, scenario::Scenario, Source)>,
    pub last_click: f64,

    pub pl_selected: usize,
    pub pl_scroll: f32,
    pub pl_entry_scroll: f32,
    pub page_scroll: f32,
    pub xhair_code: String,

    pub game: Option<Game>,
    pub current: String,
    pub paused: bool,
    pub run: Option<PlaylistRun>,
    pub result: Option<ResultInfo>,

    pub grabbed: bool,
    pub toast: Option<(String, f64)>,
    pub last_hit_sound: f64,
    pub hitmarker: f32,
    pub hitmarker_kill: bool,
    pub fps_smooth: f32,
    pub last_countdown_tick: i32,

    pub updater: Updater,
    pub started_at: f64,
    pub restart_at: Option<f64>,
    pub playlist_job: Option<std::sync::mpsc::Receiver<Result<usize, String>>>,
}

impl App {
    pub async fn new() -> App {
        let paths = crate::paths();
        for d in [&paths.scenarios, &paths.playlists, &paths.stats, &paths.themes] {
            let _ = std::fs::create_dir_all(d);
        }
        let settings = Settings::load(&paths.settings);
        let lib = Library::load(&paths.scenarios);
        let (playlists, pl_err) = playlist::load_dir(&paths.playlists);
        let (theme_files, th_err) = theme::load_dir(&paths.themes);
        let history = History::load(&paths.stats);
        let mut load_errors = lib.errors.clone();
        load_errors.extend(pl_err);
        load_errors.extend(th_err);
        let mut themes = theme::builtins();
        themes.extend(theme_files);
        let mut app = App {
            ui: Ui::new(&paths.assets),
            input: Input::new(),
            renderer: render::Renderer::new(),
            sounds: audio::Sounds::load().await,
            paths,
            settings,
            settings_dirty: false,
            lib,
            playlists,
            themes,
            load_errors,
            history,
            screen: Screen::Menu,
            tab: Tab::Scenarios,
            settings_page: SettingsPage::Sensitivity,
            entries: vec![],
            search: String::new(),
            cat_filter: None,
            sort: SortMode::Name,
            list_scroll: 0.0,
            selected: None,
            sel_cache: None,
            last_click: -10.0,
            pl_selected: 0,
            pl_scroll: 0.0,
            pl_entry_scroll: 0.0,
            page_scroll: 0.0,
            xhair_code: String::new(),
            game: None,
            current: String::new(),
            paused: false,
            run: None,
            result: None,
            grabbed: false,
            toast: None,
            last_hit_sound: 0.0,
            hitmarker: 0.0,
            hitmarker_kill: false,
            fps_smooth: 0.0,
            last_countdown_tick: -1,
            updater: Updater::new(),
            started_at: get_time(),
            restart_at: None,
            playlist_job: None,
        };
        app.apply_theme();
        app.refresh_entries();
        if let Some(v) = app.updater.just_updated.clone() {
            app.toast(format!("Updated to v{}", v.trim_start_matches('v')));
        } else if app.settings.auto_update {
            app.updater.check(true);
        }
        // First run: fetch the community playlists if there are none.
        let mut files = vec![];
        crate::scenario::collect_files(&app.paths.playlists, &["json", "plo"], &mut files, 0);
        if files.is_empty() {
            app.download_playlists();
        }
        app
    }

    pub fn download_playlists(&mut self) {
        if self.playlist_job.is_some() {
            return;
        }
        let dest = self.paths.playlists.join("community-riddbtw");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(update::download_playlists(&dest));
        });
        self.playlist_job = Some(rx);
        self.toast("Downloading community playlists...");
    }

    fn poll_background(&mut self) {
        if let Some(rx) = &self.playlist_job {
            if let Ok(res) = rx.try_recv() {
                self.playlist_job = None;
                match res {
                    Ok(n) => {
                        let (pl, _) = playlist::load_dir(&self.paths.playlists);
                        self.playlists = pl;
                        self.refresh_entries();
                        self.toast(format!("Downloaded {n} community playlists"));
                    }
                    Err(e) => self.toast(e),
                }
            }
        }
        if self.updater.poll() {
            if let update::Status::Installed { version } = &self.updater.status {
                // Fresh launch and still in the menu: restart straight into the new version.
                if self.screen == Screen::Menu && get_time() - self.started_at < 30.0 {
                    self.restart_at = Some(get_time() + 1.5);
                    self.toast(format!("Updated to {version}, restarting..."));
                } else {
                    self.toast(format!("Update {version} installed. Restart AimForge to use it"));
                }
            }
        }
        if let Some(at) = self.restart_at {
            if get_time() >= at {
                self.settings.save(&self.paths.settings);
                self.updater.restart();
            }
        }
    }

    // ----------------------------------------------------------------- misc

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), get_time() + 3.5));
    }

    pub fn apply_theme(&mut self) {
        let found = self.themes.iter().find(|t| t.name.eq_ignore_ascii_case(&self.settings.theme));
        theme::set(found.map(|t| t.theme).unwrap_or_default());
    }

    pub fn refresh_entries(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut names = self.lib.known_names();
        for p in &self.playlists {
            names.extend(p.entries.iter().map(|e| e.name.clone()));
        }
        names.extend(self.history.played_names());
        let mut out = vec![];
        for n in names {
            let k = scenario::key(&n);
            if k.is_empty() || !seen.insert(k.clone()) {
                continue;
            }
            let (s, src) = self.lib.get(&n);
            out.push(BrowserEntry { name: n, lower: k, category: s.category, source: src });
        }
        out.sort_by(|a, b| a.lower.cmp(&b.lower));
        self.entries = out;
        self.sel_cache = None;
    }

    pub fn reload_all(&mut self) {
        self.lib.reload_custom();
        let (pl, err) = playlist::load_dir(&self.paths.playlists);
        self.playlists = pl;
        let (tf, terr) = theme::load_dir(&self.paths.themes);
        self.themes = theme::builtins();
        self.themes.extend(tf);
        self.apply_theme();
        self.load_errors = self.lib.errors.clone();
        self.load_errors.extend(err);
        self.load_errors.extend(terr);
        self.pl_selected = self.pl_selected.min(self.playlists.len().saturating_sub(1));
        self.refresh_entries();
        let n_err = self.load_errors.len();
        self.toast(format!(
            "Reloaded {} scenarios, {} playlists, {} themes{}",
            self.entries.len(),
            self.playlists.len(),
            self.themes.len(),
            if n_err > 0 { format!(" ({n_err} file errors, see Settings > Files)") } else { String::new() }
        ));
    }

    pub fn set_grab(&mut self, g: bool) {
        if self.grabbed != g {
            set_cursor_grab(g);
            show_mouse(!g);
            self.input.reset();
            self.grabbed = g;
        }
    }

    // ----------------------------------------------------------- run control

    pub fn start(&mut self, name: &str) {
        let (s, _) = self.lib.get(name);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        self.current = s.name.clone();
        self.game = Some(Game::new(s, self.settings.countdown, self.settings.auto_fire_tracking, seed));
        self.paused = false;
        self.screen = Screen::Playing;
        self.last_countdown_tick = -1;
        self.set_grab(true);
    }

    pub fn restart(&mut self) {
        let n = self.current.clone();
        self.start(&n);
    }

    pub fn start_playlist(&mut self, pl: usize, entry: usize) {
        let Some(e) = self.playlists.get(pl).and_then(|p| p.entries.get(entry)) else { return };
        let name = e.name.clone();
        self.run = Some(PlaylistRun { pl, entry, rep: 0 });
        self.start(&name);
    }

    pub fn next_in_playlist(&self) -> Option<(usize, u32, String)> {
        let r = self.run.as_ref()?;
        let p = self.playlists.get(r.pl)?;
        let e = p.entries.get(r.entry)?;
        if r.rep + 1 < e.count {
            return Some((r.entry, r.rep + 1, e.name.clone()));
        }
        let ne = p.entries.get(r.entry + 1)?;
        Some((r.entry + 1, 0, ne.name.clone()))
    }

    pub fn advance_playlist(&mut self) {
        match self.next_in_playlist() {
            Some((entry, rep, name)) => {
                if let Some(r) = &mut self.run {
                    r.entry = entry;
                    r.rep = rep;
                }
                self.start(&name);
            }
            None => {
                let name = self.run.take().and_then(|r| self.playlists.get(r.pl).map(|p| p.name.clone()));
                self.screen = Screen::Menu;
                self.toast(format!("Playlist complete: {}", name.unwrap_or_default()));
            }
        }
    }

    pub fn finish(&mut self) {
        let Some(g) = &self.game else { return };
        let rec = g.record(self.settings.cm_per_360() as f32, self.settings.hfov);
        let name = g.scn.name.clone();
        let category = g.scn.category;
        let tracking = g.scn.weapon.mode == FireMode::Auto;
        let prev_best = self.history.best(&name);
        let csv = stats::write_csv(&self.paths.stats, &name, &rec, &g.kill_log, &self.settings.sens_label());
        self.history.add(&name, rec.clone());
        if !self.entries.iter().any(|e| e.lower == scenario::key(&name)) {
            self.refresh_entries();
        }
        self.result = Some(ResultInfo { name, rec, prev_best, csv, category, tracking });
        self.screen = Screen::Results;
        self.set_grab(false);
    }

    // ----------------------------------------------------------------- frame

    pub fn frame(&mut self) {
        self.input.poll();
        self.ui.begin();
        self.poll_background();
        let dt = get_frame_time();
        let inst = 1.0 / dt.max(1e-4);
        self.fps_smooth = if self.fps_smooth == 0.0 { inst } else { self.fps_smooth * 0.95 + inst * 0.05 };

        if is_key_pressed(KeyCode::F11) {
            self.settings.fullscreen = !self.settings.fullscreen;
            set_fullscreen(self.settings.fullscreen);
            self.settings_dirty = true;
        }

        match self.screen {
            Screen::Playing => self.frame_playing(dt),
            Screen::Results => self.frame_results(),
            Screen::Menu => self.frame_menu(),
        }

        if !self.grabbed {
            miniquad::window::set_mouse_cursor(if self.ui.hand {
                miniquad::CursorIcon::Pointer
            } else {
                miniquad::CursorIcon::Default
            });
        }
        if self.settings_dirty && self.ui.drag.is_none() {
            self.settings.save(&self.paths.settings);
            self.settings_dirty = false;
        }
        self.draw_toast();
    }

    fn draw_toast(&mut self) {
        let Some((msg, until)) = &self.toast else { return };
        let left = *until - get_time();
        if left <= 0.0 {
            self.toast = None;
            return;
        }
        let t = theme::t();
        let ui = &self.ui;
        let a = (left as f32 * 3.0).min(1.0);
        let w = ui.measure(msg, size::BODY, false) + ui.s(40.0);
        let r = Rect::new(screen_width() * 0.5 - w * 0.5, screen_height() - ui.s(84.0), w, ui.s(44.0));
        rrect(r, ui.s(10.0), theme::with_alpha(theme::mix(t.surface_active, t.bg, 0.2), 0.97 * a));
        ui.text_mid(msg, r.x + ui.s(20.0), r, size::BODY, theme::with_alpha(t.text, a), false);
    }
}
