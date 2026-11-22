use crate::crosshair::Crosshair;
use crate::map::MapCfg;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SensScale {
    Valorant,
    Cs2,
    Apex,
    #[default]
    #[serde(alias = "Source")]
    Kovaaks,
    Overwatch,
    /// Enter cm/360 directly (uses DPI).
    Cm360,
    /// Your own degrees-per-count multiplier.
    Custom,
}

impl SensScale {
    pub const ALL: [SensScale; 7] = [
        SensScale::Valorant,
        SensScale::Cs2,
        SensScale::Apex,
        SensScale::Overwatch,
        SensScale::Kovaaks,
        SensScale::Cm360,
        SensScale::Custom,
    ];
    /// Scales that are an actual game (for the conversion table).
    pub const GAMES: [SensScale; 5] =
        [SensScale::Valorant, SensScale::Cs2, SensScale::Apex, SensScale::Overwatch, SensScale::Kovaaks];

    pub fn label(self) -> &'static str {
        match self {
            SensScale::Valorant => "Valorant",
            SensScale::Cs2 => "CS2",
            SensScale::Apex => "Apex",
            SensScale::Overwatch => "Overwatch 2",
            SensScale::Kovaaks => "KovaaK's",
            SensScale::Cm360 => "cm/360",
            SensScale::Custom => "Custom",
        }
    }

    /// Degrees per count at sensitivity 1.
    pub fn yaw(self) -> f64 {
        match self {
            SensScale::Valorant => 0.07,
            SensScale::Cs2 | SensScale::Apex | SensScale::Kovaaks => 0.022,
            SensScale::Overwatch => 0.0066,
            SensScale::Cm360 | SensScale::Custom => 1.0,
        }
    }

    /// Default horizontal FOV for that game at 16:9.
    pub fn default_fov(self) -> Option<f32> {
        match self {
            SensScale::Valorant | SensScale::Overwatch | SensScale::Kovaaks => Some(103.0),
            SensScale::Cs2 => Some(106.26),
            _ => None,
        }
    }

    pub fn decimals(self) -> usize {
        match self {
            SensScale::Valorant => 3,
            SensScale::Overwatch => 2,
            _ => 3,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub sens_scale: SensScale,
    /// In-game sensitivity value for the chosen scale.
    pub sens: f64,
    pub cm360: f64,
    pub dpi: f64,
    pub custom_yaw: f64,
    pub invert_y: bool,
    /// Horizontal FOV in degrees (KovaaK's / Valorant default 103).
    pub hfov: f32,
    pub crosshair: Crosshair,
    pub theme: String,
    /// Overrides the theme's target colour when set.
    pub target_color: Option<[u8; 3]>,
    pub volume: f32,
    pub hit_sound: bool,
    pub kill_sound: bool,
    pub vsync: bool,
    pub fullscreen: bool,
    pub msaa: bool,
    pub show_fps: bool,
    pub countdown: f32,
    /// Hold-to-fire automatically in tracking scenarios.
    pub auto_fire_tracking: bool,
    pub show_target_health: bool,
    pub hitmarker: bool,
    /// Download and install new versions from GitHub on launch.
    pub auto_update: bool,
    /// Sky, floor, walls, fog and target look.
    pub map: MapCfg,
    pub hit_sound_name: String,
    pub kill_sound_name: String,
    /// Countdown beeps, run finished, new high score.
    pub game_sounds: bool,
    /// Soft clicks on menu buttons.
    pub ui_sounds: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sens_scale: SensScale::Kovaaks,
            sens: 1.0,
            cm360: 34.6,
            dpi: 800.0,
            custom_yaw: 0.022,
            invert_y: false,
            hfov: 103.0,
            crosshair: Crosshair::default(),
            theme: "Graphite".into(),
            target_color: None,
            volume: 0.6,
            hit_sound: true,
            kill_sound: true,
            vsync: false,
            fullscreen: true,
            msaa: true,
            show_fps: true,
            countdown: 3.0,
            auto_fire_tracking: false,
            show_target_health: true,
            hitmarker: true,
            auto_update: true,
            map: MapCfg::default(),
            hit_sound_name: "Tick".into(),
            kill_sound_name: "Ding".into(),
            game_sounds: true,
            ui_sounds: true,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        std::fs::read_to_string(path).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(t) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, t);
        }
    }

    /// Degrees of yaw per mouse count.
    pub fn deg_per_count(&self) -> f64 {
        match self.sens_scale {
            SensScale::Cm360 => {
                let counts = self.cm360.max(0.1) / 2.54 * self.dpi.max(1.0);
                360.0 / counts
            }
            SensScale::Custom => self.custom_yaw * self.sens,
            s => s.yaw() * self.sens,
        }
    }

    pub fn cm_per_360(&self) -> f64 {
        let d = self.deg_per_count().max(1e-9);
        360.0 / d / self.dpi.max(1.0) * 2.54
    }

    /// The sensitivity you'd type into `game` for the same cm/360 at this DPI.
    pub fn sens_in(&self, game: SensScale) -> f64 {
        self.deg_per_count() / game.yaw()
    }

    /// Effective DPI (sens × DPI) for the current scale, if it is a game.
    pub fn edpi(&self) -> Option<f64> {
        match self.sens_scale {
            SensScale::Cm360 | SensScale::Custom => None,
            _ => Some(self.sens * self.dpi),
        }
    }

    /// Change scale while keeping the same physical cm/360.
    pub fn convert_to(&mut self, scale: SensScale) {
        let deg = self.deg_per_count();
        let cm = self.cm_per_360();
        self.sens_scale = scale;
        match scale {
            SensScale::Cm360 => self.cm360 = round_to(cm, 2),
            SensScale::Custom => self.sens = round_to(deg / self.custom_yaw.max(1e-9), 4),
            s => self.sens = round_to(deg / s.yaw(), 4),
        }
    }

    /// Nudge sensitivity by a factor (keeps the scale).
    pub fn nudge(&mut self, factor: f64) {
        match self.sens_scale {
            SensScale::Cm360 => self.cm360 = round_to((self.cm360 / factor).clamp(1.0, 500.0), 2),
            _ => self.sens = round_to((self.sens * factor).clamp(0.0001, 1000.0), 4),
        }
    }

    pub fn sens_label(&self) -> String {
        match self.sens_scale {
            SensScale::Cm360 => format!("{:.1} cm/360", self.cm360),
            s => format!("{} {}", s.label(), trim_num(self.sens)),
        }
    }

    pub fn vfov_radians(&self, aspect: f32) -> f32 {
        let h = self.hfov.clamp(40.0, 150.0).to_radians();
        2.0 * ((h * 0.5).tan() / aspect.max(0.1)).atan()
    }
}

pub fn trim_num(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn round_to(v: f64, places: i32) -> f64 {
    let m = 10f64.powi(places);
    (v * m).round() / m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cm360_math() {
        let s = Settings { sens_scale: SensScale::Cs2, sens: 1.0, dpi: 800.0, ..Default::default() };
        // 360 / (0.022 * 800) * 2.54 = 51.95 cm
        assert!((s.cm_per_360() - 51.954).abs() < 0.01);
        let mut v = s.clone();
        v.convert_to(SensScale::Valorant);
        assert!((v.sens - 0.3143).abs() < 0.001, "{}", v.sens);
        assert!((v.cm_per_360() - s.cm_per_360()).abs() < 0.05);
        assert!((s.sens_in(SensScale::Overwatch) - 3.3333).abs() < 0.001);
    }

    #[test]
    fn valorant_to_cs2() {
        // Valorant 0.4 @ 800 DPI == CS2 1.2727
        let s = Settings { sens_scale: SensScale::Valorant, sens: 0.4, dpi: 800.0, ..Default::default() };
        assert!((s.sens_in(SensScale::Cs2) - 1.2727).abs() < 0.001);
        assert_eq!(s.edpi(), Some(320.0));
    }

    #[test]
    fn old_settings_still_load() {
        let s: Settings = toml::from_str("sens_scale = \"Source\"\nsens = 2.0\ntarget_color = [1, 2, 3]").unwrap();
        assert_eq!(s.sens_scale, SensScale::Kovaaks);
        assert_eq!(s.target_color, Some([1, 2, 3]));
    }
}
