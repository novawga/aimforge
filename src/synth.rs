//! Builds a playable scenario from a KovaaK's-style scenario name.
//!
//! Community playlists only store scenario *names*. This module reads the
//! conventions those names follow ("1w4ts", "ww3t", "B180T", "Close Fast
//! Strafes Invincible", "30% smaller", "Easy", "Voltaic"...) and assembles a
//! scenario of the matching type. The result is deterministic, so a name
//! always produces the same scenario and high scores stay comparable.
//! Export it from the menu to get an editable .toml.

use crate::scenario::*;
use regex::Regex;
use std::sync::OnceLock;

struct Name {
    raw: String,
    l: String,
}

impl Name {
    fn new(raw: &str) -> Self {
        let l = raw
            .to_lowercase()
            .replace(['_', '-', '(', ')', '[', ']', '+', ',', '.'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        Name {
            raw: raw.trim().to_string(),
            l,
        }
    }
    /// Whole-word match (multi-word phrases allowed).
    fn has(&self, w: &str) -> bool {
        let hay = format!(" {} ", self.l);
        hay.contains(&format!(" {w} "))
    }
    /// Substring match.
    fn sub(&self, s: &str) -> bool {
        self.l.contains(s)
    }
    fn any(&self, ws: &[&str]) -> bool {
        ws.iter().any(|w| self.has(w))
    }
    fn any_sub(&self, ss: &[&str]) -> bool {
        ss.iter().any(|s| self.sub(s))
    }
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("bad regex"))
}

static WALL_RE: OnceLock<Regex> = OnceLock::new();
static SPHERE_RE: OnceLock<Regex> = OnceLock::new();
static BOUNCE_RE: OnceLock<Regex> = OnceLock::new();
static TS_RE: OnceLock<Regex> = OnceLock::new();
static PCT_RE: OnceLock<Regex> = OnceLock::new();
static DUR_RE: OnceLock<Regex> = OnceLock::new();
static STRAFE_ACR_RE: OnceLock<Regex> = OnceLock::new();
static INV_ACR_RE: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Archetype {
    StaticWall,
    Pasu,
    Pokeball,
    Hipfire,
    TileFrenzy,
    Popcorn,
    BounceArc,
    Timing,
    Gallery,
    TargetSwitch,
    Smooth,
    Controlsphere,
    Air,
    Plaza,
    Strafes,
    Skeet,
    Xyz,
    Orbit,
    Glider,
    Vertical,
}

/// Arena presets.
const WALL_ARENA: ([f32; 3], [f32; 3]) = ([-14.0, 0.0, -3.0], [14.0, 12.0, 19.5]);
const FRONT_ARENA: ([f32; 3], [f32; 3]) = ([-22.0, 0.0, -6.0], [22.0, 14.0, 36.0]);
const ROOM_ARENA: ([f32; 3], [f32; 3]) = ([-18.0, 0.0, -18.0], [18.0, 14.0, 18.0]);

pub fn synthesize(raw: &str) -> Scenario {
    let n = Name::new(raw);
    let (arch, mut s) = base(&n);
    apply_modifiers(&n, arch, &mut s);
    s.name = n.raw.clone();
    if s.description.is_empty() {
        s.description = format!("{} Built automatically from the scenario name; export it to a file to fine-tune.", describe(arch));
    }
    s.author = "AimForge generator".into();
    s
}

fn describe(a: Archetype) -> &'static str {
    match a {
        Archetype::StaticWall => "Static targets on a wall.",
        Archetype::Pasu => "Targets drifting across a wall.",
        Archetype::Pokeball => "Static targets you hold the beam on to pop.",
        Archetype::Hipfire => "Several targets spread in front of you.",
        Archetype::TileFrenzy => "Targets light up on a fixed grid.",
        Archetype::Popcorn => "Targets hopping off the floor.",
        Archetype::BounceArc => "Targets bouncing around the room.",
        Archetype::Timing => "Targets that vanish if you're too slow.",
        Archetype::Gallery => "Shoot while being forced to strafe.",
        Archetype::TargetSwitch => "Several targets; take one down, switch to the next.",
        Archetype::Smooth => "A smoothly moving target to track.",
        Archetype::Controlsphere => "A close sphere moving all around you.",
        Archetype::Air => "An airborne bot jumping and strafing.",
        Archetype::Plaza => "A ground bot strafing with the odd jump.",
        Archetype::Strafes => "A bot strafing left and right.",
        Archetype::Skeet => "Fast targets sweeping across the wall.",
        Archetype::Xyz => "A sphere darting on every axis.",
        Archetype::Orbit => "A target circling around you.",
        Archetype::Glider => "Targets slowly gliding down.",
        Archetype::Vertical => "A target moving up and down.",
    }
}

#[cfg(test)]
pub fn archetype_of(raw: &str) -> Archetype {
    base(&Name::new(raw)).0
}

// ---------------------------------------------------------------------------
// Base templates
// ---------------------------------------------------------------------------

fn clicking() -> Scenario {
    let mut s = Scenario::default();
    s.category = Category::Clicking;
    s.arena.min = WALL_ARENA.0;
    s.arena.max = WALL_ARENA.1;
    s.weapon = WeaponCfg {
        mode: FireMode::Semi,
        fire_rate: 0.0,
        damage: 100.0,
        ..Default::default()
    };
    s.targets.health = 100.0;
    s.scoring = ScoringCfg {
        per_kill: 10.0,
        ..Default::default()
    };
    s
}

fn tracking_weapon(s: &mut Scenario) {
    s.weapon = WeaponCfg {
        mode: FireMode::Auto,
        fire_rate: 20.0,
        damage: 5.0,
        ..Default::default()
    };
    s.scoring = ScoringCfg {
        per_damage: 1.0,
        ..Default::default()
    };
}

fn tracking() -> Scenario {
    let mut s = Scenario::default();
    s.category = Category::Tracking;
    s.arena.min = FRONT_ARENA.0;
    s.arena.max = FRONT_ARENA.1;
    tracking_weapon(&mut s);
    s.targets.invincible = true;
    s.targets.max_alive = 1;
    s.targets.health = 100.0;
    s
}

fn bot(s: &mut Scenario) {
    s.targets.shape = TargetShape::Bot;
    s.targets.radius = 0.4;
    s.targets.height = 1.9;
    s.targets.head_radius = 0.18;
}

fn wall(n: u32, wide: bool) -> Scenario {
    let mut s = clicking();
    s.targets.radius = 0.42;
    s.targets.max_alive = n.max(1);
    s.spawn.shape = SpawnShape::Box;
    if wide {
        s.spawn.box_min = [-9.5, 1.2, 18.0];
        s.spawn.box_max = [9.5, 7.0, 18.0];
    } else {
        s.spawn.box_min = [-4.5, 1.5, 18.0];
        s.spawn.box_max = [4.5, 6.5, 18.0];
    }
    s.spawn.min_separation = 1.4;
    s
}

fn arc_bounds(angle: f32) -> ([f32; 3], [f32; 3]) {
    if angle >= 300.0 {
        ([-14.0, 0.4, -14.0], [14.0, 7.5, 14.0])
    } else if angle >= 150.0 {
        ([-15.0, 0.4, 5.0], [15.0, 7.5, 17.0])
    } else {
        ([-8.0, 0.4, 8.0], [8.0, 7.0, 17.0])
    }
}

fn wall_match(n: &Name) -> Option<(u32, bool, f32)> {
    let r = re(
        &WALL_RE,
        r"(\d\s*w(?:all)?|ww|wide\s*wall)\s*(\d+)\s*(targets?|tes|ts|t)?([a-z]*)",
    );
    let c = r.captures(&n.l)?;
    let count: u32 = c[2].parse().ok()?;
    let wide = c[1].starts_with("ww") || c[1].starts_with("wide");
    let size = match c.get(3).map(|m| m.as_str()).unwrap_or("") {
        "tes" => 0.6,
        "ts" => 0.78,
        _ => 1.0,
    };
    Some((count.clamp(1, 18), wide, size))
}

fn base(n: &Name) -> (Archetype, Scenario) {
    let wall_m = wall_match(n);
    let sphere_count = re(&SPHERE_RE, r"(\d+)\s*sphere")
        .captures(&n.l)
        .and_then(|c| c[1].parse::<u32>().ok());
    let bounce = re(
        &BOUNCE_RE,
        r"\b(?:b|l|bounce\s*|leap\s*)(90|180|360)(ti|t|i)?\b",
    )
    .captures(&n.l);
    let wants_track = n.any_sub(&["track"]);

    // 1. Pokeball: hold-to-kill static targets.
    if n.sub("pokeball") {
        let (count, wide, size) = wall_m.unwrap_or((
            sphere_count.unwrap_or(4),
            n.any(&["wide", "ww6t", "ww6ts", "ww3t"]),
            1.0,
        ));
        let mut s = if n.sub("hipfire") {
            hipfire(count)
        } else {
            wall(count, wide)
        };
        s.targets.radius *= size;
        tracking_weapon(&mut s);
        s.category = Category::Clicking;
        s.targets.health = 25.0;
        s.scoring = ScoringCfg {
            per_kill: 10.0,
            ..Default::default()
        };
        return (Archetype::Pokeball, s);
    }

    // 2. NwMt walls (static or pasu).
    if let Some((count, wide, size)) = wall_m {
        if n.sub("pasu") {
            let mut s = pasu(count, wants_track, n);
            s.targets.radius *= size;
            return (Archetype::Pasu, s);
        }
        let mut s = wall(count, wide);
        s.targets.radius *= size;
        if n.has("te") || n.sub("tile") {
            s.spawn.shape = SpawnShape::Grid;
            s.spawn.grid_center = [0.0, 3.8, 18.0];
            s.spawn.grid_cols = if wide { 5 } else { 4 };
            s.spawn.grid_rows = 3;
            s.spawn.grid_spacing = 1.7;
        }
        if n.sub("smallflicks") || n.has("small flicks") {
            s.spawn.box_min = [-2.2, 2.2, 18.0];
            s.spawn.box_max = [2.2, 5.2, 18.0];
            s.spawn.min_separation = 0.9;
        }
        if wants_track && !n.sub("pokeball") {
            tracking_weapon(&mut s);
            s.targets.health = 35.0;
            s.category = Category::Switching;
        }
        return (Archetype::StaticWall, s);
    }

    // 3. Pasu.
    if n.sub("pasu") {
        return (Archetype::Pasu, pasu(5, wants_track, n));
    }

    // 4. N Sphere Hipfire.
    if n.sub("hipfire") || (sphere_count.is_some() && !n.sub("smoothsphere")) {
        return (Archetype::Hipfire, hipfire(sphere_count.unwrap_or(5)));
    }

    // 5. Tile Frenzy.
    if n.sub("tile") || n.sub("frenzy") {
        let mut s = wall(3, false);
        s.targets.radius = 0.45;
        s.spawn.shape = SpawnShape::Grid;
        s.spawn.grid_center = [0.0, 3.6, 16.0];
        s.spawn.grid_cols = 3;
        s.spawn.grid_rows = 3;
        s.spawn.grid_spacing = 1.6;
        if n.has("360") || n.has("180") || n.has("90") {
            let a = if n.has("360") {
                360.0
            } else if n.has("180") {
                180.0
            } else {
                90.0
            };
            s.arena.min = ROOM_ARENA.0;
            s.arena.max = ROOM_ARENA.1;
            s.spawn.shape = SpawnShape::Arc;
            s.spawn.yaw_min = -a / 2.0;
            s.spawn.yaw_max = a / 2.0;
            s.spawn.pitch_min = -5.0;
            s.spawn.pitch_max = 25.0;
            s.spawn.dist_min = 8.0;
            s.spawn.dist_max = 12.0;
            s.spawn.min_separation = 3.0;
        }
        if n.sub("strafing") || n.sub("strafe") {
            s.category = Category::Strafe;
            s.player.forced_strafe = 4.5;
        }
        if wants_track {
            tracking_weapon(&mut s);
            s.targets.health = 35.0;
        }
        return (Archetype::TileFrenzy, s);
    }

    // 6. Popcorn.
    if n.sub("popcorn") || n.sub("leapcorn") {
        let mut s = clicking();
        s.targets.radius = 0.5;
        s.targets.max_alive = 2;
        s.spawn.box_min = [-6.0, 1.0, 12.0];
        s.spawn.box_max = [6.0, 4.0, 17.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Popcorn,
            speed_min: 1.5,
            speed_max: 3.5,
            gravity: 9.0,
            jump_min: 7.0,
            jump_max: 10.0,
            bounds_min: Some([-8.0, 0.0, 10.0]),
            bounds_max: Some([8.0, 11.0, 19.0]),
            ..Default::default()
        };
        if wants_track || n.sub("smoothness") {
            tracking_weapon(&mut s);
            s.category = Category::Tracking;
            s.targets.invincible = true;
            s.targets.max_alive = 1;
        }
        return (Archetype::Popcorn, s);
    }

    // 7. Bounce 90/180/360.
    if bounce.is_some() || n.sub("bounce") {
        let (angle, suffix) = match &bounce {
            Some(c) => (
                c[1].parse::<f32>().unwrap_or(180.0),
                c.get(2).map(|m| m.as_str()).unwrap_or(""),
            ),
            None => (if n.has("360") { 360.0 } else { 180.0 }, ""),
        };
        let mut s = clicking();
        s.arena.min = ROOM_ARENA.0;
        s.arena.max = ROOM_ARENA.1;
        s.targets.radius = 0.45;
        s.targets.max_alive = 3;
        s.spawn.shape = SpawnShape::Arc;
        s.spawn.yaw_min = -angle / 2.0 + 10.0;
        s.spawn.yaw_max = angle / 2.0 - 10.0;
        s.spawn.pitch_min = -3.0;
        s.spawn.pitch_max = 18.0;
        s.spawn.dist_min = 8.5;
        s.spawn.dist_max = 12.5;
        s.spawn.min_separation = 3.0;
        let (bmin, bmax) = arc_bounds(angle);
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 4.0,
            speed_max: 6.0,
            change_min: 0.5,
            change_max: 1.4,
            axes: [true, true, true],
            min_player_dist: 7.5,
            bounds_min: Some(bmin),
            bounds_max: Some(bmax),
            ..Default::default()
        };
        if suffix.contains('t') || wants_track {
            tracking_weapon(&mut s);
            s.category = Category::Switching;
            s.targets.health = if suffix.contains('i') { 60.0 } else { 40.0 };
        }
        return (Archetype::BounceArc, s);
    }

    // 8. Click timing / reflex.
    if n.any_sub(&[
        "floating head",
        "timing",
        "reflex",
        "reactive",
        "microshot",
        "flicker",
        "reaction",
        "reactivity",
    ]) && !wants_track
        && !n.sub("fuglaa")
        && !n.any(&["xy", "xyz"])
        && !n.sub("targetswitch")
    {
        let mut s = clicking();
        s.category = Category::Timing;
        s.targets.max_alive = 1;
        s.targets.radius = 0.38;
        s.targets.lifetime = 0.9;
        s.targets.respawn_delay = 0.25;
        s.spawn.shape = SpawnShape::Arc;
        s.spawn.yaw_min = -40.0;
        s.spawn.yaw_max = 40.0;
        s.spawn.pitch_min = -3.0;
        s.spawn.pitch_max = 22.0;
        s.spawn.dist_min = 10.0;
        s.spawn.dist_max = 14.0;
        if n.sub("floating head") {
            s.targets.max_alive = 2;
            s.targets.lifetime = 1.6;
            s.movement.pattern = MovePattern::Bounce;
            s.movement.speed_min = 0.6;
            s.movement.speed_max = 1.6;
            s.movement.axes = [true, true, true];
        }
        if n.sub("micro") {
            s.targets.radius = 0.16;
            s.spawn.yaw_min = -9.0;
            s.spawn.yaw_max = 9.0;
            s.spawn.pitch_min = 2.0;
            s.spawn.pitch_max = 14.0;
            s.targets.lifetime = 1.1;
        }
        if n.sub("flick") && !n.sub("micro") {
            s.spawn.yaw_min = -70.0;
            s.spawn.yaw_max = 70.0;
        }
        if n.sub("flickerbot") {
            s.targets.max_alive = 3;
            s.targets.lifetime = 1.2;
        }
        return (Archetype::Timing, s);
    }

    // 9. Strafe galleries (player moves).
    if n.sub("gallery") || (n.sub("strafing") && !n.sub("track")) {
        let mut s = wall(3, true);
        s.category = Category::Strafe;
        s.targets.radius = 0.38;
        s.player.forced_strafe = 5.0;
        s.player.forced_strafe_min = 0.35;
        s.player.forced_strafe_max = 0.9;
        s.arena.min = WALL_ARENA.0;
        s.arena.max = WALL_ARENA.1;
        if n.sub("anti mirror") || n.sub("strafe") || n.sub("pistol") {
            s.movement.pattern = MovePattern::Strafe;
            s.movement.speed_min = 2.0;
            s.movement.speed_max = 3.5;
            s.movement.bounds_min = Some([-10.0, 0.0, 17.0]);
            s.movement.bounds_max = Some([10.0, 12.0, 19.0]);
        }
        return (Archetype::Gallery, s);
    }

    // 10. Target switching.
    let ts_word = re(&TS_RE, r"\b([a-z]+)ts\b").captures_iter(&n.l).any(|c| {
        ![
            "target", "shot", "hit", "point", "light", "bot", "jet", "start", "short",
        ]
        .contains(&&c[1])
    });
    if ts_word
        || n.has("ts")
        || n.any_sub(&[
            "targetswitch",
            "target switch",
            "switching",
            "switch",
            "pressure",
            "tamspeed",
            "targetclick",
            "targetshotgun",
            "targetsg",
            "toonsclick",
            "ochclick",
        ])
    {
        let mut s = clicking();
        s.category = Category::Switching;
        s.arena.min = ROOM_ARENA.0;
        s.arena.max = ROOM_ARENA.1;
        s.targets.radius = 0.5;
        s.targets.max_alive = 3;
        s.spawn.shape = SpawnShape::Arc;
        s.spawn.yaw_min = -55.0;
        s.spawn.yaw_max = 55.0;
        s.spawn.pitch_min = 0.0;
        s.spawn.pitch_max = 22.0;
        s.spawn.dist_min = 9.0;
        s.spawn.dist_max = 13.0;
        s.spawn.min_separation = 3.0;
        let (bmin, bmax) = arc_bounds(180.0);
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 3.0,
            speed_max: 5.0,
            change_min: 0.35,
            change_max: 1.0,
            axes: [true, true, true],
            min_player_dist: 7.0,
            bounds_min: Some(bmin),
            bounds_max: Some(bmax),
            ..Default::default()
        };
        tracking_weapon(&mut s);
        s.targets.health = 40.0;
        if n.has("360") {
            s.spawn.yaw_min = -170.0;
            s.spawn.yaw_max = 170.0;
            let (a, b) = arc_bounds(360.0);
            s.movement.bounds_min = Some(a);
            s.movement.bounds_max = Some(b);
        }
        if n.sub("circle") {
            s.movement.pattern = MovePattern::Orbit;
            s.movement.speed_min = 4.0;
            s.movement.speed_max = 6.0;
        }
        if n.sub("smooth") {
            s.movement.accel = 14.0;
        }
        if n.sub("control") {
            s.movement.accel = 9.0;
            s.movement.speed_min = 2.5;
            s.movement.speed_max = 4.0;
        }
        if n.sub("pressure") {
            s.targets.max_alive = 4;
            s.targets.health = 30.0;
        }
        if n.sub("tamspeed") || n.has("2bp") || n.has("2bps") || n.has("2bpes") {
            s.targets.max_alive = 2;
            s.movement.pattern = MovePattern::Static;
            s.weapon = WeaponCfg::default();
            s.scoring = ScoringCfg {
                per_kill: 10.0,
                ..Default::default()
            };
            s.targets.health = 100.0;
            s.category = Category::Clicking;
        }
        if n.sub("click") || n.sub("shotgun") || n.sub("targetsg") {
            s.weapon = WeaponCfg::default();
            s.targets.health = 200.0;
            s.scoring = ScoringCfg {
                per_kill: 10.0,
                per_hit: 2.0,
                ..Default::default()
            };
        }
        return (Archetype::TargetSwitch, s);
    }

    // 11. Smooth tracking.
    if n.any_sub(&[
        "smoothbot",
        "smooth bot",
        "smoothsphere",
        "smooth sphere",
        "syw",
        "smoothness",
        "smooth",
    ]) && !n.sub("strafe")
    {
        let mut s = tracking();
        let sphere = n.sub("sphere") || n.sub("syw") || n.sub("smoothness");
        if sphere {
            s.targets.radius = 0.55;
        } else {
            bot(&mut s);
        }
        s.spawn.box_min = [-3.0, if sphere { 2.0 } else { 0.0 }, 12.0];
        s.spawn.box_max = [3.0, if sphere { 5.0 } else { 0.0 }, 12.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 3.0,
            speed_max: 5.0,
            change_min: 0.6,
            change_max: 1.6,
            accel: 10.0,
            axes: [true, sphere, false],
            bounds_min: Some([-8.0, if sphere { 1.0 } else { 0.0 }, 11.0]),
            bounds_max: Some([8.0, if sphere { 7.0 } else { 0.0 }, 13.0]),
            ..Default::default()
        };
        if n.sub("glider") {
            s.movement.axes = [true, true, false];
            s.movement.bounds_min = Some([-8.0, 1.0, 11.0]);
            s.movement.bounds_max = Some([8.0, 8.0, 13.0]);
            s.spawn.box_min[1] = 3.0;
            s.spawn.box_max[1] = 5.0;
        }
        return (Archetype::Smooth, s);
    }

    // 12. Controlsphere.
    if n.sub("controlsphere") || n.sub("control sphere") || n.has("control") {
        let mut s = tracking();
        s.arena.min = ROOM_ARENA.0;
        s.arena.max = ROOM_ARENA.1;
        s.targets.radius = 0.6;
        s.spawn.shape = SpawnShape::Arc;
        s.spawn.yaw_min = -30.0;
        s.spawn.yaw_max = 30.0;
        s.spawn.dist_min = 6.0;
        s.spawn.dist_max = 7.0;
        s.spawn.pitch_min = 5.0;
        s.spawn.pitch_max = 20.0;
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 5.0,
            speed_max: 8.0,
            change_min: 0.4,
            change_max: 1.0,
            accel: 22.0,
            axes: [true, true, true],
            bounds_min: Some([-9.0, 0.6, -9.0]),
            bounds_max: Some([9.0, 8.0, 9.0]),
            ..Default::default()
        };
        if n.sub("thin") {
            s.targets.radius = 0.35;
        }
        return (Archetype::Controlsphere, s);
    }

    // 13. Air.
    if n.has("air") || n.sub("air ") || n.l.starts_with("air") || n.has("ufo") {
        let mut s = tracking();
        bot(&mut s);
        s.spawn.box_min = [-3.0, 0.0, 12.0];
        s.spawn.box_max = [3.0, 2.0, 14.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Air,
            speed_min: 4.0,
            speed_max: 6.0,
            change_min: 0.35,
            change_max: 1.1,
            gravity: 10.0,
            jump_min: 5.0,
            jump_max: 8.0,
            bounds_min: Some([-10.0, 0.0, 9.0]),
            bounds_max: Some([10.0, 10.0, 17.0]),
            ..Default::default()
        };
        if n.sub("ufo") {
            s.movement.jump_max = 10.0;
        }
        return (Archetype::Air, s);
    }

    // 14. Ground plaza / PGTI.
    if n.any_sub(&["plaza", "pgti", "ground"]) || n.has("gp") {
        let mut s = tracking();
        bot(&mut s);
        s.spawn.box_min = [-3.0, 0.0, 10.0];
        s.spawn.box_max = [3.0, 0.0, 14.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Strafe,
            speed_min: 4.0,
            speed_max: 6.0,
            change_min: 0.25,
            change_max: 1.0,
            jump_chance: 0.15,
            jump_min: 4.5,
            jump_max: 6.0,
            gravity: 14.0,
            bounds_min: Some([-12.0, 0.0, 7.0]),
            bounds_max: Some([12.0, 8.0, 18.0]),
            ..Default::default()
        };
        if n.sub("high ground") {
            s.player.position = [0.0, 4.0, 0.0];
        }
        return (Archetype::Plaza, s);
    }

    // 15. Strafing bots.
    let strafe_acr = re(
        &STRAFE_ACR_RE,
        r"\b(?:c|m|l|k|vs|ra)?(?:fs|ls|ss)(?:e|r|ri|d)?i?\b",
    )
    .is_match(&n.l);
    if (strafe_acr
        || n.any_sub(&[
            "strafe",
            "centering",
            "thin aiming",
            "gauntlet",
            "lgc",
            "1v1",
            "pole",
            "lddh",
            "dodge",
            "thinbot",
            "unpredictable",
            "mss",
            "vss",
            "amphi",
            "manwing",
            "evasive",
            "needle",
        ])
        || n.has("lg"))
        && !n.sub("glider")
    {
        let mut s = tracking();
        bot(&mut s);
        let fast = n.sub("fast") || n.has("fs") || n.sub("cfs") || n.sub("mfs");
        let long = n.sub("long") || n.has("ls") || n.sub("clsi");
        s.spawn.box_min = [-2.0, 0.0, 12.0];
        s.spawn.box_max = [2.0, 0.0, 12.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Strafe,
            speed_min: 4.5,
            speed_max: 6.0,
            change_min: if fast {
                0.15
            } else if long {
                0.7
            } else {
                0.3
            },
            change_max: if fast {
                0.45
            } else if long {
                1.5
            } else {
                0.9
            },
            jump_chance: if n.sub("lgc") || n.sub("1v1") {
                0.15
            } else {
                0.0
            },
            bounds_min: Some([-12.0, 0.0, 6.0]),
            bounds_max: Some([12.0, 6.0, 20.0]),
            ..Default::default()
        };
        if fast {
            s.movement.speed_min = 5.5;
            s.movement.speed_max = 7.0;
        }
        if n.sub("centering") || n.sub("thin aiming") {
            s.movement.change_min = 0.8;
            s.movement.change_max = 1.6;
        }
        if n.sub("varied") || n.sub("gauntlet") || n.sub("lgc") {
            s.spawn.box_min[2] = 8.0;
            s.spawn.box_max[2] = 18.0;
        }
        if n.sub("pole") {
            s.targets.radius = 0.18;
            s.targets.height = 3.0;
            s.targets.head_radius = 0.0;
        }
        return (Archetype::Strafes, s);
    }

    // 16. Skeet.
    if n.sub("skeet") {
        let mut s = tracking();
        s.targets.radius = 0.4;
        s.spawn.box_min = [-8.0, 2.0, 15.0];
        s.spawn.box_max = [8.0, 7.0, 15.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 7.0,
            speed_max: 10.0,
            change_min: 0.8,
            change_max: 1.8,
            accel: 8.0,
            axes: [true, true, false],
            bounds_min: Some([-13.0, 1.0, 14.0]),
            bounds_max: Some([13.0, 10.0, 16.0]),
            ..Default::default()
        };
        return (Archetype::Skeet, s);
    }

    // 17. XY / XYZ sphere tracking (fuglaa).
    if n.any(&["xy", "xyz"]) || n.sub("fuglaa") || n.sub("xyz") {
        let z = n.sub("xyz") || n.sub("fuglaa");
        let mut s = tracking();
        s.targets.radius = 0.5;
        s.spawn.box_min = [-3.0, 2.0, 11.0];
        s.spawn.box_max = [3.0, 5.0, 12.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 4.0,
            speed_max: 6.5,
            change_min: 0.2,
            change_max: 0.7,
            axes: [true, true, z],
            blink_min: if n.sub("no blink") { 0.0 } else { 2.0 },
            blink_max: if n.sub("no blink") { 0.0 } else { 4.0 },
            bounds_min: Some([-8.0, 0.8, if z { 7.0 } else { 11.0 }]),
            bounds_max: Some([8.0, 8.0, if z { 15.0 } else { 12.0 }]),
            ..Default::default()
        };
        return (Archetype::Xyz, s);
    }

    // 18. Orbit / revolving.
    if n.any_sub(&["revolving", "orbit", "circle"]) {
        let mut s = tracking();
        s.arena.min = ROOM_ARENA.0;
        s.arena.max = ROOM_ARENA.1;
        bot(&mut s);
        s.spawn.shape = SpawnShape::Arc;
        s.spawn.yaw_min = -20.0;
        s.spawn.yaw_max = 20.0;
        s.spawn.dist_min = 8.0;
        s.spawn.dist_max = 10.0;
        s.movement = MovementCfg {
            pattern: MovePattern::Orbit,
            speed_min: 5.0,
            speed_max: 7.0,
            change_min: 0.6,
            change_max: 1.6,
            accel: 25.0,
            ..Default::default()
        };
        return (Archetype::Orbit, s);
    }

    // 19. Glider.
    if n.sub("glider") {
        let mut s = tracking();
        s.arena.min = ROOM_ARENA.0;
        s.arena.max = ROOM_ARENA.1;
        bot(&mut s);
        s.spawn.shape = SpawnShape::Arc;
        s.spawn.yaw_min = if n.has("360") { -170.0 } else { -50.0 };
        s.spawn.yaw_max = if n.has("360") { 170.0 } else { 50.0 };
        s.spawn.dist_min = 8.0;
        s.spawn.dist_max = 13.0;
        s.movement = MovementCfg {
            pattern: MovePattern::Glider,
            speed_min: 2.0,
            speed_max: 4.0,
            change_min: 0.8,
            change_max: 1.8,
            gravity: 1.6,
            accel: 6.0,
            ..Default::default()
        };
        return (Archetype::Glider, s);
    }

    // 20. Vertical.
    if n.sub("vertical") {
        let mut s = tracking();
        s.targets.radius = 0.5;
        s.spawn.box_min = [-2.0, 2.0, 12.0];
        s.spawn.box_max = [2.0, 6.0, 12.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            speed_min: 4.0,
            speed_max: 6.5,
            change_min: 0.3,
            change_max: 1.0,
            axes: [false, true, false],
            bounds_min: Some([-2.0, 0.6, 12.0]),
            bounds_max: Some([2.0, 10.0, 12.0]),
            ..Default::default()
        };
        if n.sub("switch") {
            s.targets.invincible = false;
            s.targets.max_alive = 3;
            s.targets.health = 40.0;
            s.category = Category::Switching;
        }
        return (Archetype::Vertical, s);
    }

    // Horizontal clicking: pasu-style but side to side only.
    if n.sub("horizontal") {
        let mut s = pasu(4, wants_track, n);
        s.movement.axes = [true, false, false];
        return (Archetype::Pasu, s);
    }

    // Fallbacks.
    if wants_track || n.sub("invincible") || n.sub("invin") || n.sub("paki") {
        let mut s = tracking();
        s.targets.radius = 0.5;
        s.spawn.box_min = [-3.0, 2.0, 12.0];
        s.spawn.box_max = [3.0, 5.0, 12.0];
        s.movement = MovementCfg {
            pattern: MovePattern::Bounce,
            accel: 10.0,
            axes: [true, true, false],
            bounds_min: Some([-8.0, 1.0, 11.0]),
            bounds_max: Some([8.0, 7.0, 13.0]),
            ..Default::default()
        };
        return (Archetype::Smooth, s);
    }
    let mut s = wall(3, n.sub("wide") || n.sub("large angle"));
    if n.any_sub(&["speed", "flick"]) {
        s.targets.radius = 0.4;
    }
    (Archetype::StaticWall, s)
}

fn pasu(count: u32, track: bool, n: &Name) -> Scenario {
    let mut s = wall(count.max(1), false);
    s.targets.radius = 0.45;
    s.spawn.box_min = [-6.0, 1.5, 18.0];
    s.spawn.box_max = [6.0, 7.0, 18.0];
    s.movement = MovementCfg {
        pattern: MovePattern::Bounce,
        speed_min: 2.5,
        speed_max: 4.5,
        change_min: 0.5,
        change_max: 1.5,
        axes: [true, true, false],
        bounds_min: Some([-7.0, 1.0, 18.0]),
        bounds_max: Some([7.0, 8.0, 18.0]),
        ..Default::default()
    };
    if track {
        tracking_weapon(&mut s);
        if n.sub("invincible") || n.sub("smooth") {
            s.category = Category::Tracking;
            s.targets.invincible = true;
            s.targets.max_alive = 1;
            s.movement.accel = if n.sub("smooth") { 10.0 } else { 0.0 };
        } else {
            s.category = Category::Switching;
            s.targets.health = 35.0;
        }
    }
    if n.sub("dodge") || n.sub("adhd") {
        s.movement.change_min = 0.2;
        s.movement.change_max = 0.6;
    }
    s
}

fn hipfire(count: u32) -> Scenario {
    let mut s = clicking();
    s.arena.min = FRONT_ARENA.0;
    s.arena.max = FRONT_ARENA.1;
    s.targets.radius = 0.4;
    s.targets.max_alive = count.clamp(1, 16);
    s.spawn.shape = SpawnShape::Arc;
    s.spawn.yaw_min = -55.0;
    s.spawn.yaw_max = 55.0;
    s.spawn.pitch_min = -4.0;
    s.spawn.pitch_max = 26.0;
    s.spawn.dist_min = 8.0;
    s.spawn.dist_max = 14.0;
    s.spawn.min_separation = 2.5;
    s
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

fn apply_modifiers(n: &Name, arch: Archetype, s: &mut Scenario) {
    // Size words (with "N% smaller/larger" phrases removed so they are not counted twice).
    let pct_re = re(&PCT_RE, r"(\d+(?:\.\d+)?)\s*(?:%|prct|percent)\s*([a-z]*)");
    let full = n;
    let stripped = Name::new(&pct_re.replace_all(&n.l, " "));
    let n = &stripped;
    let mut size = 1.0;
    if n.sub("extra extra small") || n.sub("xxs") {
        size *= 0.42;
    } else if n.sub("extra small") || n.has("xsmall") || n.has("xs") || n.sub("x small") {
        size *= 0.58;
    } else if n.sub("smaller") {
        size *= 0.72;
    } else if n.has("small") || n.has("mini") {
        size *= 0.78;
    }
    if n.has("tiny") {
        size *= 0.5;
    }
    if n.has("jumbo") || n.has("huge") {
        size *= 1.7;
    } else if n.has("larger") || n.has("large") || n.has("big") || n.has("bigger") {
        size *= 1.3;
    }
    // "30% smaller" / "25% larger" / "(25% smaller diameter)"
    let mut speed = 1.0f32;
    for c in pct_re.captures_iter(&full.l) {
        let p: f32 = c[1].parse().unwrap_or(100.0);
        match &c[2] {
            "smaller" => size *= (1.0 - p / 100.0).max(0.1),
            "larger" | "bigger" => size *= 1.0 + p / 100.0,
            "hp" | "health" | "extra" => {}
            _ if (30.0..=200.0).contains(&p) => speed *= p / 100.0,
            _ => {}
        }
    }
    let n = full;
    let thin = n.sub("thin");
    let extra_thin = n.sub("extra thin") || n.sub("thinner");
    s.targets.radius *= size;
    if s.targets.shape == TargetShape::Bot {
        s.targets.head_radius *= size.sqrt();
        if extra_thin {
            s.targets.radius *= 0.4;
        } else if thin {
            s.targets.radius *= 0.55;
        }
    } else if thin && arch != Archetype::Controlsphere {
        s.targets.radius *= if extra_thin { 0.55 } else { 0.75 };
    }

    // Difficulty words.
    let mut size_ease = 1.0f32;
    let pairs: &[(&str, f32, f32)] = &[
        ("very easy", 0.65, 1.25),
        ("easier", 0.8, 1.12),
        ("easy", 0.82, 1.12),
        ("medium", 1.0, 1.0),
        ("harder", 1.22, 0.92),
        ("hard", 1.18, 0.94),
        ("insane", 1.4, 0.85),
        ("very slow", 0.55, 1.0),
        ("slow", 0.72, 1.0),
        ("iron", 0.7, 1.2),
        ("bronze", 0.8, 1.1),
        ("silver", 0.9, 1.05),
        ("platinum", 1.05, 1.0),
        ("diamond", 1.1, 0.97),
        ("jade", 1.15, 0.95),
        ("master", 1.2, 0.93),
        ("grandmaster", 1.25, 0.9),
        ("goated", 1.1, 1.0),
        ("frantic", 1.25, 1.0),
    ];
    for (w, sp, sz) in pairs {
        if n.has(w) {
            speed *= sp;
            size_ease *= sz;
            if *w == "very easy" || *w == "very slow" {
                break;
            }
        }
    }
    s.targets.radius *= size_ease;
    s.movement.speed_min *= speed;
    s.movement.speed_max *= speed;
    s.player.forced_strafe *= speed.sqrt();
    if s.targets.lifetime > 0.0 && speed != 1.0 {
        s.targets.lifetime /= speed.sqrt();
    }

    // Distance words (moving archetypes).
    let moving = !matches!(
        arch,
        Archetype::StaticWall | Archetype::Pokeball | Archetype::TileFrenzy
    );
    if moving && matches!(s.spawn.shape, SpawnShape::Box) {
        let d = if n.has("close") || n.sub("cfs") || n.sub("cls") {
            Some(7.5)
        } else if n.sub("kindaclose") {
            Some(10.0)
        } else if n.has("far") || n.sub("long range") {
            Some(20.0)
        } else if n.sub("midrange") || n.has("mid") || n.sub("mfs") {
            Some(13.0)
        } else {
            None
        };
        if let Some(d) = d {
            let cur = (s.spawn.box_min[2] + s.spawn.box_max[2]) * 0.5;
            let off = d - cur;
            s.spawn.box_min[2] += off;
            s.spawn.box_max[2] += off;
            if let (Some(a), Some(b)) = (&mut s.movement.bounds_min, &mut s.movement.bounds_max) {
                a[2] += off;
                b[2] += off;
            }
        }
    }

    // Invincible / tracking variants.
    let inv_acr = re(
        &INV_ACR_RE,
        r"\b(?:pgti|mfsi|clsi|cfsri|cfsi|fsei|vsfsi|lfsi|clsri|rsii|msi|csi)\b",
    )
    .is_match(&n.l);
    if n.sub("invincible") || n.sub("invin") || inv_acr {
        if matches!(
            s.category,
            Category::Switching | Category::Clicking | Category::Timing
        ) && !matches!(arch, Archetype::StaticWall | Archetype::Pokeball)
        {
            tracking_weapon(s);
            s.category = Category::Tracking;
            s.targets.max_alive = 1;
            s.targets.lifetime = 0.0;
        }
        if s.category == Category::Tracking {
            s.targets.invincible = true;
        }
    }

    if n.sub("dodge") || n.sub("reactive") || n.sub("unpredictable") {
        s.movement.change_min *= 0.6;
        s.movement.change_max *= 0.6;
    }
    if n.has("static") && arch != Archetype::StaticWall {
        s.movement.pattern = MovePattern::Static;
    }
    if n.has("hs") || n.sub("heads") || n.sub("headshot") {
        if s.targets.shape == TargetShape::Bot {
            s.targets.head_only = true;
        }
    }
    if n.sub("shrink") {
        s.targets.shrink_rate = s.targets.radius * 0.35;
        s.targets.min_radius = s.targets.radius * 0.25;
    }
    if n.sub("regen") || n.sub("regenerating") {
        s.targets.regen_rate = s.targets.health * 0.8;
        s.targets.regen_delay = 0.3;
    }
    if n.has("shortened") {
        s.duration = 30.0;
    }
    if let Some(c) = re(&DUR_RE, r"\b(\d{2,3})\s*s(?:ec)?\b").captures(&n.l) {
        if let Ok(d) = c[1].parse::<f32>() {
            if (10.0..=300.0).contains(&d) {
                s.duration = d;
            }
        }
    }
    if n.sub("shotgun") {
        s.weapon.damage *= 1.0;
        s.targets.radius *= 1.1;
    }
    if s.category == Category::Tracking || s.category == Category::Switching {
        if n.sub("lg") || n.sub("lightning") {
            s.weapon.fire_rate = 20.0;
        }
    }
    s.targets.radius = s.targets.radius.max(0.06);
    s.targets.min_radius = s.targets.min_radius.min(s.targets.radius);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(name: &str) -> (Archetype, Category, bool) {
        let s = synthesize(name);
        (archetype_of(name), s.category, s.targets.invincible)
    }

    #[test]
    fn classic_names() {
        assert_eq!(cat("1w4ts reload").0, Archetype::StaticWall);
        assert_eq!(synthesize("1w4ts reload").targets.max_alive, 4);
        assert_eq!(synthesize("1wall 6targets small").targets.max_alive, 6);
        assert_eq!(synthesize("Wide Wall 3 Targets").targets.max_alive, 3);
        assert_eq!(cat("1wall5targets_pasu Reload").0, Archetype::Pasu);
        assert_eq!(cat("Pokeball Frenzy Auto 1w4ts").0, Archetype::Pokeball);
        assert_eq!(cat("6 Sphere Hipfire Voltaic").0, Archetype::Hipfire);
        assert_eq!(synthesize("6 Sphere Hipfire Voltaic").targets.max_alive, 6);
        assert_eq!(
            cat("Popcorn Goated Tracking Invincible"),
            (Archetype::Popcorn, Category::Tracking, true)
        );
        assert_eq!(cat("B180T Voltaic").1, Category::Switching);
        assert_eq!(cat("B180 Voltaic").1, Category::Clicking);
        assert_eq!(cat("voxTargetSwitch").1, Category::Switching);
        assert_eq!(cat("kinTS Voltaic").1, Category::Switching);
        assert_eq!(
            cat("Close Fast Strafes Invincible"),
            (Archetype::Strafes, Category::Tracking, true)
        );
        assert_eq!(cat("Smoothbot Voltaic").0, Archetype::Smooth);
        assert_eq!(cat("Air Angelic 4 Voltaic").0, Archetype::Air);
        assert_eq!(cat("Floating Heads Timing 400%").1, Category::Timing);
        assert_eq!(cat("Manwing Strafe Gallery V3").1, Category::Strafe);
        assert_eq!(cat("Controlsphere").0, Archetype::Controlsphere);
        assert_eq!(cat("FuglaaXYZ Voltaic No Blinks xsmall").0, Archetype::Xyz);
    }

    #[test]
    fn size_modifiers() {
        let a = synthesize("1w4ts reload").targets.radius;
        let b = synthesize("1w4ts reload 30% smaller").targets.radius;
        assert!((b / a - 0.7).abs() < 0.01, "{a} {b}");
        let c = synthesize("1w4ts 30% larger").targets.radius;
        assert!((c / a - 1.3).abs() < 0.01);
    }

    #[test]
    fn deterministic() {
        assert_eq!(
            synthesize("Pasu Track Smooth extra small"),
            synthesize("Pasu Track Smooth extra small")
        );
    }
}

#[cfg(test)]
mod fallback_probe {
    #[test]
    #[ignore]
    fn list_fallbacks() {
        let dir = std::env::var("PLAYLIST_DIR").unwrap();
        let (pls, _) = crate::playlist::load_dir(std::path::Path::new(&dir));
        let mut seen = std::collections::HashSet::new();
        for p in pls {
            for e in p.entries {
                if !seen.insert(e.name.to_lowercase()) {
                    continue;
                }
                let n = super::Name::new(&e.name);
                if super::archetype_of(&e.name) == super::Archetype::StaticWall
                    && super::wall_match(&n).is_none()
                {
                    println!("FALLBACK: {}", e.name);
                }
            }
        }
    }
}
