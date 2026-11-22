//! Application state and the parts shared by every screen.

use crate::game::Game;
use crate::input::Input;
use crate::playlist::{self, Playlist};
use crate::scenario::{self, Category, FireMode, Library, Source};
use crate::settings::Settings;
use crate::stats::{self, History, RunRecord};
use crate::theme::{self, NamedTheme};
use crate::ui::{rrect, size, Ui};
use crate::leaderboard::{self, Msg};
use crate::replay::{self as replay_mod, Recorder, Replay};
use crate::update::{self, Updater};
use crate::{audio, render, Paths};
use macroquad::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Playing,
    Results,
    Replay,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Scenarios,
    Playlists,
    Leaderboards,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    Sensitivity,
    Crosshair,
    Map,
    Sound,
    Theme,
    Gameplay,
    VideoAudio,
    Files,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 8] = [
        SettingsPage::Sensitivity,
        SettingsPage::Crosshair,
        SettingsPage::Map,
        SettingsPage::Sound,
        SettingsPage::Theme,
        SettingsPage::Gameplay,
        SettingsPage::VideoAudio,
        SettingsPage::Files,
    ];
    pub fn label(self) -> &'static str {
        match self {
            SettingsPage::Sensitivity => "Sensitivity & FOV",
            SettingsPage::Crosshair => "Crosshair",
            SettingsPage::Map => "Map & targets",
            SettingsPage::Sound => "Sound",
            SettingsPage::Theme => "Menu theme",
            SettingsPage::Gameplay => "Gameplay",
            SettingsPage::VideoAudio => "Video",
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
    pub replay: Option<Replay>,
    pub submit: SubmitState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SubmitState {
    /// No leaderboard server configured.
    Offline,
    /// Online, but no player name chosen yet.
    NeedName,
    Disabled,
    Sending,
    Done { rank: usize, total: usize },
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BoardMode {
    Global,
    Mine,
}

#[derive(Default)]
pub struct BoardState {
    pub loading: bool,
    pub entries: Vec<leaderboard::Entry>,
    pub error: Option<String>,
    pub fetched: f64,
}

pub struct ReplayView {
    pub replay: Replay,
    pub game: Game,
    pub t: f32,
    pub playing: bool,
    pub speed: f32,
    pub from: Screen,
    pub label: String,
    pub next_event: usize,
    pub hitmarker: f32,
    pub hitmarker_kill: bool,
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

    pub board: leaderboard::Client,
    pub boards: std::collections::HashMap<String, BoardState>,
    pub board_mode: BoardMode,
    pub board_scenario: Option<String>,
    pub board_scroll: f32,
    pub board_list_scroll: f32,
    pub board_search: String,
    pub name_buf: String,
    pub recorder: Option<Recorder>,
    pub viewer: Option<ReplayView>,
    pub results_tab: usize,
}

impl App {
    pub async fn new() -> App {
        let paths = crate::paths();
        for d in [&paths.scenarios, &paths.playlists, &paths.stats, &paths.themes, &paths.sounds] {
            let _ = std::fs::create_dir_all(d);
        }
        let mut settings = Settings::load(&paths.settings);
        // v0.2 kept a target colour override in Settings; it now lives in the map.
        if let Some(c) = settings.target_color.take() {
            settings.map.ball = c;
        }
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
            sounds: audio::Sounds::load(&paths.sounds).await,
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
            board: leaderboard::Client::new("", ""),
            boards: Default::default(),
            board_mode: BoardMode::Global,
            board_scenario: None,
            board_scroll: 0.0,
            board_list_scroll: 0.0,
            board_search: String::new(),
            name_buf: String::new(),
            recorder: None,
            viewer: None,
            results_tab: 0,
        };
        app.board = leaderboard::Client::new(&app.settings.board_url, &app.settings.board_key);
        if app.settings.player_id.len() != 36 {
            app.settings.player_id = leaderboard::new_player_id();
            app.settings.save(&app.paths.settings);
        }
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
        while let Ok(msg) = self.board.rx.try_recv() {
            match msg {
                Msg::Top { scenario, result } => {
                    let st = self.boards.entry(scenario).or_default();
                    st.loading = false;
                    st.fetched = get_time();
                    match result {
                        Ok(e) => {
                            st.entries = e;
                            st.error = None;
                        }
                        Err(e) => st.error = Some(e),
                    }
                }
                Msg::Submitted { scenario, score, result } => {
                    if let Some(r) = &mut self.result {
                        if scenario::key(&r.name) == scenario && (r.rec.score - score).abs() < 1e-3 {
                            r.submit = match result {
                                Ok((rank, total)) => SubmitState::Done { rank, total },
                                Err(e) => SubmitState::Failed(e),
                            };
                        }
                    }
                }
                Msg::Replay { label, result } => match result.and_then(|b| Replay::decode(&b)) {
                    Ok(r) => self.open_replay(r, label),
                    Err(e) => self.toast(format!("Couldn't load replay: {e}")),
                },
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
        self.recorder = Some(Recorder::new());
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
        let mut rec = g.record(self.settings.cm_per_360() as f32, self.settings.hfov);
        let name = g.scn.name.clone();
        let category = g.scn.category;
        let tracking = g.scn.weapon.mode == FireMode::Auto;
        let prev_best = self.history.best(&name);
        if self.settings.game_sounds {
            let best = prev_best.is_none_or(|b| rec.score > b) && rec.score > 0.0;
            self.sounds.play(if best { "highscore" } else { "finish" }, self.settings.volume * 0.8);
        }
        let csv = stats::write_csv(&self.paths.stats, &name, &rec, &g.kill_log, &self.settings.sens_label());

        // Replay.
        let mut replay = None;
        let mut bytes = None;
        if let Some(mut r) = self.recorder.take() {
            r.sample(g);
            let rp = r.finish(replay_mod::Header {
                player: self.settings.player_name.clone(),
                scenario: name.clone(),
                score: rec.score,
                accuracy: rec.accuracy,
                kills: rec.kills,
                hits: rec.hits,
                shots: rec.shots,
                duration: rec.duration,
                date: rec.date.clone(),
                cm360: rec.cm360,
                hfov: rec.hfov,
                game_version: update::VERSION.to_string(),
                scenario_toml: toml::to_string(&g.scn).unwrap_or_default(),
            });
            let b = rp.encode();
            rec.replay = replay_mod::save(&self.paths.stats, &rp, &b);
            replay = Some(rp);
            bytes = Some(b);
        }
        self.history.add(&name, rec.clone());
        self.history.prune_replays(&name, &replay_mod::dir(&self.paths.stats));

        // Online.
        let submit = if !self.board.online() {
            SubmitState::Offline
        } else if !self.settings.submit_scores {
            SubmitState::Disabled
        } else if self.settings.player_name.trim().is_empty() {
            SubmitState::NeedName
        } else if rec.shots == 0 {
            SubmitState::Failed("no shots fired, not submitted".into())
        } else {
            self.submit_run(&name, &rec, bytes.clone());
            SubmitState::Sending
        };
        if self.board.online() {
            self.ensure_board(&name, true);
        }

        if !self.entries.iter().any(|e| e.lower == scenario::key(&name)) {
            self.refresh_entries();
        }
        self.result = Some(ResultInfo { name, rec, prev_best, csv, category, tracking, replay, submit });
        self.screen = Screen::Results;
        self.set_grab(false);
    }

    pub fn submit_run(&mut self, name: &str, rec: &RunRecord, replay: Option<Vec<u8>>) {
        let entry = leaderboard::Entry {
            scenario: scenario::key(name),
            scenario_name: name.to_string(),
            player: leaderboard::clean_name(&self.settings.player_name),
            player_id: self.settings.player_id.clone(),
            score: rec.score,
            accuracy: rec.accuracy.clamp(0.0, 1.0),
            kills: rec.kills as i32,
            hits: rec.hits as i32,
            shots: rec.shots as i32,
            cm360: rec.cm360,
            hfov: rec.hfov,
            duration: rec.duration,
            game_version: update::VERSION.to_string(),
            ..Default::default()
        };
        if let Some(b) = self.boards.get_mut(&scenario::key(name)) {
            b.loading = true;
        }
        self.board.submit(entry, replay);
    }

    /// Fetch a scenario's global board if missing or stale.
    pub fn ensure_board(&mut self, name: &str, force: bool) {
        if !self.board.online() {
            return;
        }
        let k = scenario::key(name);
        let st = self.boards.entry(k.clone()).or_default();
        if st.loading || (!force && st.fetched > 0.0 && get_time() - st.fetched < 60.0) {
            return;
        }
        st.loading = true;
        self.board.fetch_top(&k);
    }

    pub fn open_replay(&mut self, replay: Replay, label: String) {
        let scn = replay.scenario();
        let mut game = Game::new(scn, 0.0, false, 1);
        game.phase = crate::game::Phase::Running;
        let from = if self.screen == Screen::Replay { self.viewer.as_ref().map(|v| v.from).unwrap_or(Screen::Menu) } else { self.screen };
        self.viewer = Some(ReplayView {
            replay,
            game,
            t: 0.0,
            playing: true,
            speed: 1.0,
            from,
            label,
            next_event: 0,
            hitmarker: 0.0,
            hitmarker_kill: false,
        });
        self.screen = Screen::Replay;
        self.set_grab(false);
    }

    pub fn open_local_replay(&mut self, rel: &str, label: String) {
        match replay_mod::load(&self.paths.stats, rel) {
            Ok(r) => self.open_replay(r, label),
            Err(e) => self.toast(e),
        }
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
            Screen::Replay => self.frame_replay(dt),
        }

        if self.ui.clicks > 0 && self.settings.ui_sounds && !matches!(self.screen, Screen::Playing | Screen::Replay) {
            self.sounds.play("ui", self.settings.volume * 0.45);
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
