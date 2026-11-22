//! Scenario file format.
//!
//! A scenario is a plain TOML file. Every field has a default, so a file only
//! needs the parts it changes. See `SCENARIO_FORMAT.md` for the full reference.
//!
//! World units are metres. +Y is up. The player looks down +Z at yaw 0, and
//! positive yaw turns to the right (towards -X).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Category {
    #[default]
    Clicking,
    Tracking,
    Switching,
    Timing,
    Strafe,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Clicking,
        Category::Tracking,
        Category::Switching,
        Category::Timing,
        Category::Strafe,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Category::Clicking => "Clicking",
            Category::Tracking => "Tracking",
            Category::Switching => "Target Switching",
            Category::Timing => "Click Timing",
            Category::Strafe => "Strafe Aim",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Scenario {
    pub name: String,
    pub category: Category,
    pub author: String,
    pub description: String,
    /// Run length in seconds.
    pub duration: f32,
    pub player: PlayerCfg,
    pub weapon: WeaponCfg,
    pub arena: ArenaCfg,
    pub targets: TargetCfg,
    pub spawn: SpawnCfg,
    pub movement: MovementCfg,
    pub scoring: ScoringCfg,
}

impl Default for Scenario {
    fn default() -> Self {
        Scenario {
            name: "Untitled".into(),
            category: Category::Clicking,
            author: String::new(),
            description: String::new(),
            duration: 60.0,
            player: PlayerCfg::default(),
            weapon: WeaponCfg::default(),
            arena: ArenaCfg::default(),
            targets: TargetCfg::default(),
            spawn: SpawnCfg::default(),
            movement: MovementCfg::default(),
            scoring: ScoringCfg::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct PlayerCfg {
    /// Feet position.
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub eye_height: f32,
    /// WASD + jump. Strafe-aim scenarios usually also set `forced_strafe`.
    pub can_move: bool,
    pub move_speed: f32,
    pub jump_velocity: f32,
    pub gravity: f32,
    /// If > 0 the player is pushed left/right automatically at this speed
    /// (switching every `forced_strafe_min..max` seconds) – strafe-aim drills.
    pub forced_strafe: f32,
    pub forced_strafe_min: f32,
    pub forced_strafe_max: f32,
}

impl Default for PlayerCfg {
    fn default() -> Self {
        PlayerCfg {
            position: [0.0, 0.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
            eye_height: 1.6,
            can_move: true,
            move_speed: 6.0,
            jump_velocity: 5.0,
            gravity: 15.0,
            forced_strafe: 0.0,
            forced_strafe_min: 0.4,
            forced_strafe_max: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FireMode {
    /// One shot per click.
    #[default]
    Semi,
    /// Fires continuously while held (tracking "beam").
    Auto,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct WeaponCfg {
    pub mode: FireMode,
    /// Shots per second. For semi-auto this caps click rate (0 = uncapped).
    pub fire_rate: f32,
    pub damage: f32,
    pub headshot_multiplier: f32,
    /// 0 = infinite ammo.
    pub magazine: u32,
    pub reload_time: f32,
}

impl Default for WeaponCfg {
    fn default() -> Self {
        WeaponCfg {
            mode: FireMode::Semi,
            fire_rate: 0.0,
            damage: 100.0,
            headshot_multiplier: 1.0,
            magazine: 0,
            reload_time: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ArenaCfg {
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Grid line spacing on walls/floor (0 = none).
    pub grid: f32,
}

impl Default for ArenaCfg {
    fn default() -> Self {
        ArenaCfg {
            min: [-14.0, 0.0, -3.0],
            max: [14.0, 12.0, 19.5],
            grid: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TargetShape {
    #[default]
    Sphere,
    /// Humanoid capsule with optional head.
    Bot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct TargetCfg {
    pub shape: TargetShape,
    /// Sphere radius, or capsule radius for bots.
    pub radius: f32,
    /// Bot total height (feet to top of head).
    pub height: f32,
    /// Bot head radius (0 = no separate head).
    pub head_radius: f32,
    pub head_only: bool,
    pub health: f32,
    /// Infinite health; score comes from damage (tracking).
    pub invincible: bool,
    /// HP per second regenerated after `regen_delay` seconds without damage.
    pub regen_rate: f32,
    pub regen_delay: f32,
    /// Targets alive at the same time.
    pub max_alive: u32,
    /// Total spawns for the run (0 = unlimited). Run ends when all are cleared.
    pub total: u32,
    pub respawn_delay: f32,
    /// Seconds before an unkilled target disappears (0 = never). Click-timing.
    pub lifetime: f32,
    /// Radius lost per second while alive (pokeball "shrink").
    pub shrink_rate: f32,
    pub min_radius: f32,
    /// Optional fixed colour; otherwise the colour from Settings is used.
    pub color: Option<[u8; 3]>,
}

impl Default for TargetCfg {
    fn default() -> Self {
        TargetCfg {
            shape: TargetShape::Sphere,
            radius: 0.3,
            height: 1.8,
            head_radius: 0.0,
            head_only: false,
            health: 100.0,
            invincible: false,
            regen_rate: 0.0,
            regen_delay: 1.0,
            max_alive: 3,
            total: 0,
            respawn_delay: 0.0,
            lifetime: 0.0,
            shrink_rate: 0.0,
            min_radius: 0.05,
            color: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpawnShape {
    /// Uniform random inside `box_min..box_max` (world space).
    #[default]
    Box,
    /// Around the player's eye: yaw/pitch window (degrees, relative to start yaw) and distance.
    Arc,
    /// Tile-Frenzy style grid of fixed slots on a plane facing the player.
    Grid,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct SpawnCfg {
    pub shape: SpawnShape,
    pub box_min: [f32; 3],
    pub box_max: [f32; 3],
    pub yaw_min: f32,
    pub yaw_max: f32,
    pub pitch_min: f32,
    pub pitch_max: f32,
    pub dist_min: f32,
    pub dist_max: f32,
    pub grid_center: [f32; 3],
    pub grid_cols: u32,
    pub grid_rows: u32,
    pub grid_spacing: f32,
    /// Minimum distance between a new target and live targets.
    pub min_separation: f32,
}

impl Default for SpawnCfg {
    fn default() -> Self {
        SpawnCfg {
            shape: SpawnShape::Box,
            box_min: [-5.0, 1.5, 18.0],
            box_max: [5.0, 6.0, 18.0],
            yaw_min: -45.0,
            yaw_max: 45.0,
            pitch_min: -5.0,
            pitch_max: 20.0,
            dist_min: 10.0,
            dist_max: 16.0,
            grid_center: [0.0, 3.5, 18.0],
            grid_cols: 3,
            grid_rows: 3,
            grid_spacing: 1.6,
            min_separation: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MovePattern {
    #[default]
    Static,
    /// Straight lines, new random direction every change interval, reflects off bounds.
    Bounce,
    /// Left/right strafing perpendicular to the player (ADAD).
    Strafe,
    /// Strafe + gravity + constant jumping.
    Air,
    /// Gravity, hops off the floor with random horizontal velocity.
    Popcorn,
    /// Circles the player, reversing direction at change interval.
    Orbit,
    /// Slow fall with drift; respawns at the top when it lands.
    Glider,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MovementCfg {
    pub pattern: MovePattern,
    pub speed_min: f32,
    pub speed_max: f32,
    /// Seconds between direction changes.
    pub change_min: f32,
    pub change_max: f32,
    /// m/s² towards the desired velocity (0 = instant, snappy changes).
    pub accel: f32,
    /// Which axes Bounce may move on (x, y, z).
    pub axes: [bool; 3],
    pub gravity: f32,
    pub jump_min: f32,
    pub jump_max: f32,
    /// Chance (0-1) to jump at each direction change (Strafe pattern).
    pub jump_chance: f32,
    /// Teleport every `blink_min..blink_max` seconds (0 = off).
    pub blink_min: f32,
    pub blink_max: f32,
    /// Targets are kept at least this far from the player's eye (metres).
    pub min_player_dist: f32,
    /// Optional explicit movement bounds; defaults to spawn box (Box spawns) or the arena.
    pub bounds_min: Option<[f32; 3]>,
    pub bounds_max: Option<[f32; 3]>,
}

impl Default for MovementCfg {
    fn default() -> Self {
        MovementCfg {
            pattern: MovePattern::Static,
            speed_min: 3.0,
            speed_max: 5.0,
            change_min: 0.4,
            change_max: 1.2,
            accel: 0.0,
            axes: [true, true, false],
            gravity: 12.0,
            jump_min: 4.0,
            jump_max: 7.0,
            jump_chance: 0.0,
            blink_min: 0.0,
            blink_max: 0.0,
            min_player_dist: 5.0,
            bounds_min: None,
            bounds_max: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ScoringCfg {
    pub per_kill: f32,
    pub per_hit: f32,
    pub per_damage: f32,
    /// Subtracted for every shot that hits nothing.
    pub per_miss: f32,
    /// Subtracted when a timed target expires.
    pub per_expire: f32,
}

impl Default for ScoringCfg {
    fn default() -> Self {
        ScoringCfg {
            per_kill: 100.0,
            per_hit: 0.0,
            per_damage: 0.0,
            per_miss: 0.0,
            per_expire: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Shipped inside the executable.
    Builtin,
    /// A .toml file in the scenarios folder.
    Custom,
    /// Built on the fly from the scenario name.
    Generated,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Builtin => "Built-in",
            Source::Custom => "Custom file",
            Source::Generated => "Generated",
        }
    }
}

pub fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub struct Library {
    pub dir: PathBuf,
    builtin: HashMap<String, Scenario>,
    custom: HashMap<String, (Scenario, PathBuf)>,
    pub errors: Vec<String>,
}

macro_rules! builtin_files {
    ($($f:literal),* $(,)?) => { &[$(($f, include_str!(concat!("../assets/scenarios/", $f)))),*] };
}

pub const BUILTIN_FILES: &[(&str, &str)] = builtin_files!(
    "1w4ts_voltaic.toml",
    "ww3t_voltaic.toml",
    "pasu_voltaic.toml",
    "popcorn_voltaic.toml",
    "b180_voltaic.toml",
    "pgti_voltaic.toml",
    "air_angelic_4_voltaic.toml",
    "smoothbot_voltaic.toml",
    "fuglaaxyz_voltaic.toml",
    "ground_plaza_voltaic.toml",
    "air_voltaic.toml",
    "patts_voltaic.toml",
    "voxts_voltaic.toml",
    "kints_voltaic.toml",
    "b180t_voltaic.toml",
    "psalmts_voltaic.toml",
    "1wall_6targets_small.toml",
    "tile_frenzy.toml",
    "close_long_strafes_invincible.toml",
    "floating_heads_timing.toml",
    "strafe_gallery.toml",
    "pokeball_frenzy_auto_1w4ts.toml",
);

impl Library {
    pub fn load(dir: &Path) -> Library {
        let mut lib = Library {
            dir: dir.to_path_buf(),
            builtin: HashMap::new(),
            custom: HashMap::new(),
            errors: vec![],
        };
        for (file, text) in BUILTIN_FILES {
            match toml::from_str::<Scenario>(text) {
                Ok(s) => {
                    lib.builtin.insert(key(&s.name), s);
                }
                Err(e) => lib.errors.push(format!("built-in {file}: {e}")),
            }
        }
        lib.reload_custom();
        lib
    }

    pub fn reload_custom(&mut self) {
        self.custom.clear();
        self.errors.retain(|e| e.starts_with("built-in"));
        let _ = std::fs::create_dir_all(&self.dir);
        let mut files = vec![];
        collect_files(&self.dir, &["toml"], &mut files, 0);
        for path in files {
            match std::fs::read_to_string(&path) {
                Ok(text) => match toml::from_str::<Scenario>(&text) {
                    Ok(mut s) => {
                        if s.name.trim().is_empty() || s.name == "Untitled" {
                            s.name = path
                                .file_stem()
                                .map(|x| x.to_string_lossy().to_string())
                                .unwrap_or_default();
                        }
                        sanitize(&mut s);
                        self.custom.insert(key(&s.name), (s, path));
                    }
                    Err(e) => self.errors.push(format!("{}: {}", path.display(), e)),
                },
                Err(e) => self.errors.push(format!("{}: {}", path.display(), e)),
            }
        }
    }

    /// Resolve a name: custom file > built-in > generated from the name.
    pub fn get(&self, name: &str) -> (Scenario, Source) {
        let k = key(name);
        if let Some((s, _)) = self.custom.get(&k) {
            return (s.clone(), Source::Custom);
        }
        if let Some(s) = self.builtin.get(&k) {
            return (s.clone(), Source::Builtin);
        }
        let mut s = crate::synth::synthesize(name);
        sanitize(&mut s);
        (s, Source::Generated)
    }

    pub fn custom_path(&self, name: &str) -> Option<PathBuf> {
        self.custom.get(&key(name)).map(|(_, p)| p.clone())
    }

    /// Names of every scenario that exists as a file or built-in.
    pub fn known_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.builtin.values().map(|s| s.name.clone()).collect();
        for (s, _) in self.custom.values() {
            if !self.builtin.contains_key(&key(&s.name)) {
                v.push(s.name.clone());
            }
        }
        v
    }

    /// Write a scenario out as an editable .toml in the scenarios folder.
    pub fn export(&mut self, s: &Scenario) -> std::io::Result<PathBuf> {
        let path = match self.custom_path(&s.name) {
            Some(p) => p,
            None => self.dir.join(format!("{}.toml", safe_file_name(&s.name))),
        };
        let body = toml::to_string_pretty(s).map_err(|e| std::io::Error::other(e.to_string()))?;
        let text = format!(
            "# AimForge scenario file. Edit freely, then press F5 in the menu to reload.\n# Field reference: SCENARIO_FORMAT.md\n\n{body}"
        );
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(&path, text)?;
        self.reload_custom();
        Ok(path)
    }
}

/// Clamp values that would break the simulation.
pub fn sanitize(s: &mut Scenario) {
    s.duration = s.duration.clamp(1.0, 3600.0);
    let t = &mut s.targets;
    t.radius = t.radius.clamp(0.01, 10.0);
    t.min_radius = t.min_radius.clamp(0.005, t.radius);
    t.height = t.height.max(t.radius * 2.0);
    t.head_radius = t.head_radius.clamp(0.0, t.height * 0.4);
    t.max_alive = t.max_alive.clamp(1, 64);
    t.health = t.health.max(0.001);
    let w = &mut s.weapon;
    w.fire_rate = w.fire_rate.clamp(0.0, 120.0);
    if w.mode == FireMode::Auto && w.fire_rate <= 0.0 {
        w.fire_rate = 15.0;
    }
    let m = &mut s.movement;
    m.speed_max = m.speed_max.max(m.speed_min);
    m.change_min = m.change_min.max(0.02);
    m.change_max = m.change_max.max(m.change_min);
    m.jump_max = m.jump_max.max(m.jump_min);
    m.blink_max = m.blink_max.max(m.blink_min);
    let sp = &mut s.spawn;
    sp.grid_cols = sp.grid_cols.clamp(1, 32);
    sp.grid_rows = sp.grid_rows.clamp(1, 32);
    sp.dist_max = sp.dist_max.max(sp.dist_min);
    for i in 0..3 {
        if s.arena.max[i] < s.arena.min[i] {
            std::mem::swap(&mut s.arena.max[i], &mut s.arena.min[i]);
        }
        if sp.box_max[i] < sp.box_min[i] {
            std::mem::swap(&mut sp.box_max[i], &mut sp.box_min[i]);
        }
    }
    s.player.forced_strafe_max = s
        .player
        .forced_strafe_max
        .max(s.player.forced_strafe_min.max(0.05));
}

pub fn safe_file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_.+()%#".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim().trim_matches('.').to_string();
    if s.is_empty() {
        "scenario".into()
    } else {
        s
    }
}

pub fn collect_files(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 6 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_files(&p, exts, out, depth + 1);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if exts.iter().any(|x| x.eq_ignore_ascii_case(ext)) {
                out.push(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse() {
        for (f, text) in BUILTIN_FILES {
            let s: Scenario = toml::from_str(text).unwrap_or_else(|e| panic!("{f}: {e}"));
            assert!(!s.name.is_empty(), "{f}");
        }
    }

    #[test]
    fn export_roundtrip() {
        for name in [
            "Pasu Track Smooth extra small",
            "B180T Voltaic Easy",
            "Close Fast Strafes Invincible",
            "Tile Frenzy 360",
        ] {
            let mut s = crate::synth::synthesize(name);
            sanitize(&mut s);
            let text = toml::to_string_pretty(&s).unwrap();
            let back: Scenario = toml::from_str(&text).unwrap();
            assert_eq!(s, back, "{name}");
        }
    }

    #[test]
    fn shipped_example_files_load() {
        let lib = Library::load(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/scenarios")));
        assert!(lib.errors.is_empty(), "{:?}", lib.errors);
        assert_eq!(lib.get("My First Scenario").1, Source::Custom);
    }

    #[test]
    fn minimal_file() {
        let s: Scenario = toml::from_str("name = \"Mine\"\n[targets]\nradius = 0.2").unwrap();
        assert_eq!(s.name, "Mine");
        assert_eq!(s.targets.max_alive, 3);
        assert_eq!(s.duration, 60.0);
    }
}
