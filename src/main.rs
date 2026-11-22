//! AimForge: a KovaaK's-style aim trainer.

// No console window for release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod crosshair;
mod game;
mod input;
mod leaderboard;
mod map;
mod playlist;
mod render;
mod replay;
mod scenario;
mod screens;
mod settings;
mod stats;
mod synth;
mod theme;
mod ui;
mod update;

use macroquad::prelude::*;
use settings::Settings;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Paths {
    pub scenarios: PathBuf,
    pub playlists: PathBuf,
    pub themes: PathBuf,
    pub sounds: PathBuf,
    pub stats: PathBuf,
    pub settings: PathBuf,
    pub assets: PathBuf,
}

fn data_root() -> PathBuf {
    if let Ok(p) = std::env::var("AIMFORGE_HOME") {
        return PathBuf::from(p);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| cwd.clone());
    let mut candidates = vec![cwd.clone(), exe_dir.clone()];
    if let Some(up) = exe_dir.parent().and_then(|p| p.parent()) {
        candidates.push(up.to_path_buf()); // target/release -> project root
    }
    for c in &candidates {
        if c.join("playlists").is_dir() || c.join("scenarios").is_dir() {
            return c.clone();
        }
    }
    exe_dir
}

pub fn paths() -> Paths {
    let root = data_root();
    Paths {
        scenarios: root.join("scenarios"),
        playlists: root.join("playlists"),
        themes: root.join("themes"),
        sounds: root.join("sounds"),
        stats: root.join("stats"),
        settings: root.join("settings.toml"),
        assets: root.join("assets"),
    }
}

fn window_conf() -> Conf {
    let s = Settings::load(&paths().settings);
    Conf {
        window_title: "AimForge".to_owned(),
        window_width: 1600,
        window_height: 900,
        high_dpi: true, // keeps raw mouse counts unscaled on Windows
        fullscreen: s.fullscreen,
        sample_count: if s.msaa { 4 } else { 1 },
        window_resizable: true,
        platform: miniquad::conf::Platform { swap_interval: Some(if s.vsync { 1 } else { 0 }), ..Default::default() },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let quick = args.iter().any(|a| a == "--play" || a == "--no-intro");
    let mut app = boot(quick).await;
    // `aimforge --play "Scenario Name"` jumps straight into a scenario.
    // `--map "Synthwave"` applies a map preset.
    if let Some(name) = args.iter().position(|a| a == "--map").and_then(|i| args.get(i + 1)) {
        if let Some((_, cfg)) = map::presets().into_iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            app.settings.map = cfg;
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--play") {
        if let Some(name) = args.get(i + 1) {
            app.start(name);
        }
    }
    loop {
        app.frame();
        next_frame().await;
    }
}

/// Load the game behind the animated loading screen.
async fn boot(quick: bool) -> app::App {
    /// The animation always gets at least this long, so it reads as an intro
    /// rather than a flash. Any key or click skips the rest once loading is done.
    const MIN_SHOW: f64 = 2.4;
    const EXIT: f64 = 0.5;
    let mut b = app::Boot::new();
    if quick {
        while !b.done() {
            b.step().await;
        }
        return app::App::from_boot(b);
    }
    let t0 = get_time();
    let mut shown = 0.0f32;
    let mut exit_at: Option<f64> = None;
    loop {
        let t = get_time() - t0;
        b.ui.begin();
        // Ease the bar toward real progress so quick steps still glide.
        let target = b.progress().min(((t - 0.3) / (MIN_SHOW - 0.9)).clamp(0.0, 1.0) as f32 + if b.done() { 0.0 } else { 0.08 });
        shown += (target.max(shown) - shown) * (1.0 - (-get_frame_time() * 9.0).exp());
        let skip = b.done() && t > 0.6 && (is_mouse_button_pressed(MouseButton::Left) || get_last_key_pressed().is_some());
        if exit_at.is_none() && b.done() && (t >= MIN_SHOW || skip) {
            exit_at = Some(t);
        }
        let exit = exit_at.map(|e| ((t - e) / EXIT) as f32).unwrap_or(0.0);
        // Label follows the bar, so each step is readable even when loading is quick.
        let steps = app::Boot::STEPS;
        let status = steps[((shown * (steps.len() - 1) as f32) as usize).min(steps.len() - 1)];
        screens::intro::draw_loading(&b.ui, t, if b.done() { shown.max(0.0) } else { shown }, status, exit.min(1.0), b.ball());
        if exit >= 1.0 {
            break;
        }
        next_frame().await;
        // Heavy work after the frame is on screen, one step per frame.
        if !b.done() && t > 0.25 {
            b.step().await;
        }
    }
    let app = app::App::from_boot(b);
    next_frame().await;
    app
}

/// Open a URL in the default browser.
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    let _ = r;
}

pub fn open_path(p: &Path) {
    let _ = std::fs::create_dir_all(p);
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer").arg(p).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(p).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(p).spawn();
    let _ = r;
}
