//! AimForge: a KovaaK's-style aim trainer.

// No console window for release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod crosshair;
mod game;
mod input;
mod map;
mod playlist;
mod render;
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
    let mut app = app::App::new().await;
    // `aimforge --play "Scenario Name"` jumps straight into a scenario.
    let args: Vec<String> = std::env::args().collect();
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
