//! Scenario simulation: player, weapon, targets, scoring. No rendering here.

use crate::scenario::*;
use crate::stats::{KillEvent, RunRecord};
use macroquad::math::{vec3, Vec3};

// ---------------------------------------------------------------------------
// Small deterministic RNG (xorshift64*).
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        if b <= a {
            a
        } else {
            a + (b - a) * self.f()
        }
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    pub fn sign(&mut self) -> f32 {
        if self.f() < 0.5 {
            -1.0
        } else {
            1.0
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

pub fn aim_dir(yaw_deg: f64, pitch_deg: f64) -> Vec3 {
    let (y, p) = (yaw_deg.to_radians(), pitch_deg.to_radians());
    vec3(
        (-y.sin() * p.cos()) as f32,
        p.sin() as f32,
        (y.cos() * p.cos()) as f32,
    )
}

pub fn ray_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32) -> Option<f32> {
    let oc = o - c;
    let b = oc.dot(d);
    let cc = oc.dot(oc) - r * r;
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t > 0.0 {
        Some(t)
    } else {
        let t2 = -b + disc.sqrt();
        (t2 > 0.0).then_some(t2)
    }
}

/// Ray vs capsule (segment a-b, radius r). Returns distance along the ray.
pub fn ray_capsule(o: Vec3, d: Vec3, a: Vec3, b: Vec3, r: f32) -> Option<f32> {
    // Closest points between ray and segment.
    let u = b - a;
    let w = o - a;
    let uu = u.dot(u);
    let ud = u.dot(d);
    let uw = u.dot(w);
    let dw = d.dot(w);
    let denom = uu - ud * ud; // |d| = 1
    let mut s = if denom.abs() > 1e-6 {
        (uw - ud * dw) / denom
    } else {
        0.0
    };
    s = s.clamp(0.0, 1.0);
    let p = a + u * s;
    // Exact test against the sphere at the closest segment point, then refine.
    let t = ray_sphere(o, d, p, r)?;
    let hit = o + d * t;
    // Refine: project hit onto segment and re-test.
    let s2 = if uu > 1e-6 {
        ((hit - a).dot(u) / uu).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let p2 = a + u * s2;
    ray_sphere(o, d, p2, r).or(Some(t))
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Target {
    /// Unique per spawn (handy for custom HUD/debug code).
    #[allow(dead_code)]
    pub id: u64,
    /// Sphere centre, or bot feet.
    pub pos: Vec3,
    pub vel: Vec3,
    pub desired: Vec3,
    pub radius: f32,
    pub health: f32,
    pub max_health: f32,
    pub spawn_time: f32,
    pub last_damage: f32,
    pub change_timer: f32,
    pub blink_timer: f32,
    pub flash: f32,
    pub dir_sign: f32,
    pub orbit_angle: f32,
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub slot: Option<usize>,
    pub damage_taken: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct BotGeom {
    pub body_a: Vec3,
    pub body_b: Vec3,
    pub head: Option<Vec3>,
}

impl Target {
    /// A static stand-in target for previews.
    pub fn preview(id: u64, pos: Vec3, radius: f32) -> Target {
        Target {
            id,
            pos,
            vel: Vec3::ZERO,
            desired: Vec3::ZERO,
            radius,
            health: 1.0,
            max_health: 1.0,
            spawn_time: 0.0,
            last_damage: 0.0,
            change_timer: 0.0,
            blink_timer: 0.0,
            flash: 0.0,
            dir_sign: 1.0,
            orbit_angle: 0.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            slot: None,
            damage_taken: 0.0,
        }
    }

    pub fn bot_geom(&self, cfg: &TargetCfg) -> BotGeom {
        let r = self.radius;
        let scale = r / cfg.radius.max(1e-4);
        let hr = cfg.head_radius * scale.sqrt();
        let h = cfg.height;
        let p = self.pos;
        let (top, head) = if hr > 0.0 {
            (h - 2.0 * hr - 0.04 - r, Some(p + vec3(0.0, h - hr, 0.0)))
        } else {
            (h - r, None)
        };
        BotGeom {
            body_a: p + vec3(0.0, r, 0.0),
            body_b: p + vec3(0.0, top.max(r), 0.0),
            head,
        }
    }

    pub fn head_radius(&self, cfg: &TargetCfg) -> f32 {
        cfg.head_radius * (self.radius / cfg.radius.max(1e-4)).sqrt()
    }

    /// Point used for "where is it" (labels, separation).
    pub fn center(&self, cfg: &TargetCfg) -> Vec3 {
        match cfg.shape {
            TargetShape::Sphere => self.pos,
            TargetShape::Bot => self.pos + vec3(0.0, cfg.height * 0.55, 0.0),
        }
    }

    pub fn top(&self, cfg: &TargetCfg) -> Vec3 {
        match cfg.shape {
            TargetShape::Sphere => self.pos + vec3(0.0, self.radius, 0.0),
            TargetShape::Bot => self.pos + vec3(0.0, cfg.height, 0.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f64,
    pub pitch: f64,
    pub on_ground: bool,
    pub strafe_sign: f32,
    pub strafe_timer: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Countdown,
    Running,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameEvent {
    Hit { kill: bool, head: bool },
    Miss,
    Expire,
}

#[derive(Default, Clone, Copy)]
pub struct MoveKeys {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

pub struct Game {
    pub scn: Scenario,
    pub player: Player,
    pub targets: Vec<Target>,
    pub phase: Phase,
    pub countdown: f32,
    /// Seconds into the run.
    pub time: f32,
    pub rng: Rng,
    pub events: Vec<GameEvent>,

    // weapon
    pub trigger: bool,
    pub auto_fire: bool,
    next_shot: f32,
    pub ammo: u32,
    pub reload_until: f32,

    // spawning
    next_id: u64,
    spawned: u32,
    pending: Vec<f32>,
    last_slot: Option<usize>,
    bmin: Vec3,
    bmax: Vec3,
    spawn_eye: Vec3,
    start_yaw: f32,

    // stats
    pub score: f32,
    pub shots: u32,
    pub hits: u32,
    pub kills: u32,
    pub expired: u32,
    pub damage_done: f32,
    pub damage_possible: f32,
    pub kill_log: Vec<KillEvent>,
    since_kill_shots: u32,
    since_kill_hits: u32,
    since_kill_damage: f32,
    pub last_hit_time: f32,
    /// Seconds spent with the crosshair on a target (tracking feedback).
    pub on_target_time: f32,
}

impl Game {
    pub fn new(scn: Scenario, countdown: f32, auto_fire_tracking: bool, seed: u64) -> Game {
        let pc = scn.player.clone();
        let player = Player {
            pos: Vec3::from(pc.position),
            vel: Vec3::ZERO,
            yaw: pc.yaw as f64,
            pitch: pc.pitch as f64,
            on_ground: true,
            strafe_sign: 1.0,
            strafe_timer: 0.0,
        };
        let m = &scn.movement;
        let (bmin, bmax) = match (m.bounds_min, m.bounds_max) {
            (Some(a), Some(b)) => (Vec3::from(a), Vec3::from(b)),
            _ if scn.spawn.shape == SpawnShape::Box && m.pattern != MovePattern::Static => {
                (Vec3::from(scn.spawn.box_min), Vec3::from(scn.spawn.box_max))
            }
            _ => (Vec3::from(scn.arena.min), Vec3::from(scn.arena.max)),
        };
        let amin = Vec3::from(scn.arena.min);
        let amax = Vec3::from(scn.arena.max);
        let auto_fire = auto_fire_tracking && scn.weapon.mode == FireMode::Auto;
        let spawn_eye = player.pos + vec3(0.0, pc.eye_height, 0.0);
        let mut g = Game {
            ammo: scn.weapon.magazine,
            player,
            targets: vec![],
            phase: if countdown > 0.0 {
                Phase::Countdown
            } else {
                Phase::Running
            },
            countdown,
            time: 0.0,
            rng: Rng::new(seed),
            events: vec![],
            trigger: false,
            auto_fire,
            next_shot: 0.0,
            reload_until: 0.0,
            next_id: 1,
            spawned: 0,
            pending: vec![],
            last_slot: None,
            bmin: bmin.max(amin),
            bmax: bmax.min(amax),
            spawn_eye,
            start_yaw: pc.yaw,
            score: 0.0,
            shots: 0,
            hits: 0,
            kills: 0,
            expired: 0,
            damage_done: 0.0,
            damage_possible: 0.0,
            kill_log: vec![],
            since_kill_shots: 0,
            since_kill_hits: 0,
            since_kill_damage: 0.0,
            last_hit_time: -10.0,
            on_target_time: 0.0,
            scn,
        };
        for _ in 0..g.scn.targets.max_alive {
            g.spawn_target();
        }
        g
    }

    pub fn eye(&self) -> Vec3 {
        self.player.pos + vec3(0.0, self.scn.player.eye_height, 0.0)
    }

    pub fn dir(&self) -> Vec3 {
        aim_dir(self.player.yaw, self.player.pitch)
    }

    pub fn time_left(&self) -> f32 {
        (self.scn.duration - self.time).max(0.0)
    }

    pub fn accuracy(&self) -> f32 {
        if self.shots == 0 {
            0.0
        } else {
            self.hits as f32 / self.shots as f32
        }
    }

    /// Mouse movement in counts.
    pub fn look(&mut self, dx: f64, dy: f64, deg_per_count: f64, invert_y: bool) {
        self.player.yaw = (self.player.yaw + dx * deg_per_count).rem_euclid(360.0);
        let dy = if invert_y { -dy } else { dy };
        self.player.pitch = (self.player.pitch - dy * deg_per_count).clamp(-89.0, 89.0);
    }

    pub fn trigger_down(&mut self) {
        self.trigger = true;
        if self.phase == Phase::Countdown && self.scn.weapon.mode == FireMode::Semi {
            return;
        }
        if self.phase != Phase::Running {
            return;
        }
        if self.time >= self.next_shot {
            self.fire();
            let rate = self.scn.weapon.fire_rate;
            self.next_shot = if rate > 0.0 {
                self.time + 1.0 / rate
            } else {
                self.time
            };
        }
    }

    pub fn trigger_up(&mut self) {
        self.trigger = false;
    }

    fn fire(&mut self) {
        if self.scn.weapon.magazine > 0 {
            if self.time < self.reload_until {
                return;
            }
            if self.ammo == 0 {
                self.ammo = self.scn.weapon.magazine;
            }
            self.ammo -= 1;
            if self.ammo == 0 {
                self.reload_until = self.time + self.scn.weapon.reload_time;
            }
        }
        let o = self.eye();
        let d = self.dir();
        self.shots += 1;
        self.since_kill_shots += 1;
        self.damage_possible += self.scn.weapon.damage;
        match self.pick(o, d) {
            Some((idx, head)) => self.apply_hit(idx, head),
            None => {
                self.score -= self.scn.scoring.per_miss;
                self.events.push(GameEvent::Miss);
            }
        }
    }

    /// Nearest target under the ray, with headshot flag.
    pub fn pick(&self, o: Vec3, d: Vec3) -> Option<(usize, bool)> {
        let cfg = &self.scn.targets;
        let mut best: Option<(f32, usize, bool)> = None;
        for (i, t) in self.targets.iter().enumerate() {
            let hit = match cfg.shape {
                TargetShape::Sphere => ray_sphere(o, d, t.pos, t.radius).map(|x| (x, false)),
                TargetShape::Bot => {
                    let g = t.bot_geom(cfg);
                    let head = g
                        .head
                        .and_then(|h| ray_sphere(o, d, h, t.head_radius(cfg)))
                        .map(|x| (x, true));
                    let body = if cfg.head_only {
                        None
                    } else {
                        ray_capsule(o, d, g.body_a, g.body_b, t.radius).map(|x| (x, false))
                    };
                    match (head, body) {
                        (Some(h), Some(b)) => Some(if h.0 <= b.0 + 0.05 { h } else { b }),
                        (h, b) => h.or(b),
                    }
                }
            };
            if let Some((dist, head)) = hit {
                if best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, i, head));
                }
            }
        }
        best.map(|(_, i, h)| (i, h))
    }

    fn apply_hit(&mut self, idx: usize, head: bool) {
        let w = &self.scn.weapon;
        let dmg = w.damage
            * if head {
                w.headshot_multiplier.max(0.0)
            } else {
                1.0
            };
        let sc = self.scn.scoring.clone();
        let invincible = self.scn.targets.invincible;
        self.hits += 1;
        self.since_kill_hits += 1;
        self.since_kill_damage += dmg;
        self.damage_done += dmg;
        self.last_hit_time = self.time;
        self.score += sc.per_hit + sc.per_damage * dmg;
        let t = &mut self.targets[idx];
        t.flash = 1.0;
        t.last_damage = self.time;
        t.damage_taken += dmg;
        let mut kill = false;
        if !invincible {
            t.health -= dmg;
            if t.health <= 1e-3 {
                kill = true;
            }
        }
        self.events.push(GameEvent::Hit { kill, head });
        if kill {
            let t = self.targets.remove(idx);
            self.kills += 1;
            self.score += sc.per_kill;
            self.kill_log.push(KillEvent {
                time: self.time,
                ttk: self.time - t.spawn_time,
                shots: self.since_kill_shots,
                hits: self.since_kill_hits,
                damage: self.since_kill_damage,
            });
            self.since_kill_shots = 0;
            self.since_kill_hits = 0;
            self.since_kill_damage = 0.0;
            self.last_slot = t.slot;
            self.queue_respawn();
        }
    }

    fn queue_respawn(&mut self) {
        let t = self.time + self.scn.targets.respawn_delay.max(0.0);
        self.pending.push(t);
    }

    fn total_left(&self) -> bool {
        self.scn.targets.total == 0 || self.spawned < self.scn.targets.total
    }

    // -----------------------------------------------------------------------
    // Spawning
    // -----------------------------------------------------------------------

    fn floor(&self) -> f32 {
        self.scn.arena.min[1]
    }

    fn spawn_target(&mut self) {
        if !self.total_left() {
            return;
        }
        let cfg = self.scn.targets.clone();
        let sp = self.scn.spawn.clone();
        let mut best_pos = None;
        let mut slot = None;
        for attempt in 0..48 {
            let (p, s) = self.random_spawn_point(&sp);
            let p = self.place(p, &cfg);
            let ok = self.targets.iter().all(|t| {
                let a = t.center(&cfg);
                let b = match cfg.shape {
                    TargetShape::Sphere => p,
                    TargetShape::Bot => p + vec3(0.0, cfg.height * 0.55, 0.0),
                };
                a.distance(b) >= sp.min_separation.max(cfg.radius * 2.2)
            }) && (s.is_none()
                || (s != self.last_slot && self.targets.iter().all(|t| t.slot != s)));
            if ok || attempt == 47 {
                best_pos = Some(p);
                slot = s;
                if ok {
                    break;
                }
            }
        }
        let pos = best_pos.unwrap_or(Vec3::from(sp.box_min));
        let mut t = Target {
            id: self.next_id,
            pos,
            vel: Vec3::ZERO,
            desired: Vec3::ZERO,
            radius: cfg.radius,
            health: cfg.health,
            max_health: cfg.health,
            spawn_time: self.time,
            last_damage: -100.0,
            change_timer: 0.0,
            blink_timer: self
                .rng
                .range(self.scn.movement.blink_min, self.scn.movement.blink_max),
            flash: 0.0,
            dir_sign: self.rng.sign(),
            orbit_angle: 0.0,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            slot,
            damage_taken: 0.0,
        };
        let c = self.spawn_eye;
        t.orbit_angle = (pos.z - c.z).atan2(pos.x - c.x);
        t.orbit_radius = vec3(pos.x - c.x, 0.0, pos.z - c.z).length().max(2.0);
        self.next_id += 1;
        self.spawned += 1;
        self.targets.push(t);
    }

    fn random_spawn_point(&mut self, sp: &SpawnCfg) -> (Vec3, Option<usize>) {
        match sp.shape {
            SpawnShape::Box => (
                vec3(
                    self.rng.range(sp.box_min[0], sp.box_max[0]),
                    self.rng.range(sp.box_min[1], sp.box_max[1]),
                    self.rng.range(sp.box_min[2], sp.box_max[2]),
                ),
                None,
            ),
            SpawnShape::Arc => {
                let yaw = self.start_yaw + self.rng.range(sp.yaw_min, sp.yaw_max);
                let pitch = self.rng.range(sp.pitch_min, sp.pitch_max);
                let d = self.rng.range(sp.dist_min, sp.dist_max);
                (self.spawn_eye + aim_dir(yaw as f64, pitch as f64) * d, None)
            }
            SpawnShape::Grid => {
                let cols = sp.grid_cols.max(1) as usize;
                let rows = sp.grid_rows.max(1) as usize;
                let n = cols * rows;
                let i = (self.rng.f() * n as f32) as usize % n;
                let (cx, cy) = (i % cols, i / cols);
                let center = Vec3::from(sp.grid_center);
                let to_eye = (self.spawn_eye - center) * vec3(1.0, 0.0, 1.0);
                let fwd = if to_eye.length() > 0.01 {
                    to_eye.normalize()
                } else {
                    vec3(0.0, 0.0, -1.0)
                };
                let right = vec3(fwd.z, 0.0, -fwd.x);
                let ox = (cx as f32 - (cols as f32 - 1.0) * 0.5) * sp.grid_spacing;
                let oy = (cy as f32 - (rows as f32 - 1.0) * 0.5) * sp.grid_spacing;
                (center + right * ox + vec3(0.0, oy, 0.0), Some(i))
            }
        }
    }

    /// Clamp a spawn point into the arena and snap bots to the floor.
    fn place(&self, mut p: Vec3, cfg: &TargetCfg) -> Vec3 {
        let amin = Vec3::from(self.scn.arena.min);
        let amax = Vec3::from(self.scn.arena.max);
        let m = cfg.radius + 0.05;
        p.x = clamp_mid(p.x, amin.x + m, amax.x - m);
        p.z = clamp_mid(p.z, amin.z + m, amax.z - m);
        match cfg.shape {
            TargetShape::Sphere => p.y = clamp_mid(p.y, amin.y + m, amax.y - m),
            TargetShape::Bot => {
                let airborne = matches!(self.scn.movement.pattern, MovePattern::Glider);
                if !airborne && self.scn.spawn.shape == SpawnShape::Arc {
                    p.y = self.floor();
                }
                p.y = clamp_mid(p.y, amin.y, amax.y - cfg.height);
            }
        }
        p
    }

    // -----------------------------------------------------------------------
    // Update
    // -----------------------------------------------------------------------

    pub fn update(&mut self, dt: f32, keys: MoveKeys) {
        let dt = dt.clamp(0.0, 0.1);
        self.update_player(dt, keys);
        match self.phase {
            Phase::Countdown => {
                self.countdown -= dt;
                if self.countdown <= 0.0 {
                    self.phase = Phase::Running;
                    self.next_shot = 0.0;
                }
                return;
            }
            Phase::Finished => return,
            Phase::Running => {}
        }
        self.time += dt;

        // Auto-fire weapons.
        if self.scn.weapon.mode == FireMode::Auto && (self.trigger || self.auto_fire) {
            let interval = 1.0 / self.scn.weapon.fire_rate.max(1.0);
            if self.next_shot < self.time - interval {
                self.next_shot = self.time - interval * 0.999;
            }
            let mut guard = 0;
            while self.next_shot <= self.time && guard < 16 {
                self.fire();
                self.next_shot += interval;
                guard += 1;
            }
        }
        // Tracking feedback.
        if self.pick(self.eye(), self.dir()).is_some() {
            self.on_target_time += dt;
        }

        self.update_targets(dt);

        // Respawns.
        let now = self.time;
        let mut due = 0;
        self.pending.retain(|&t| {
            if t <= now {
                due += 1;
                false
            } else {
                true
            }
        });
        for _ in 0..due {
            if (self.targets.len() as u32) < self.scn.targets.max_alive {
                self.spawn_target();
            }
        }
        // Keep the field full (e.g. after total changes).
        while (self.targets.len() as u32 + self.pending.len() as u32) < self.scn.targets.max_alive
            && self.total_left()
        {
            self.queue_respawn();
        }

        let cleared = self.scn.targets.total > 0 && !self.total_left() && self.targets.is_empty();
        if self.time >= self.scn.duration || cleared {
            self.time = self.time.min(self.scn.duration);
            self.phase = Phase::Finished;
            self.trigger = false;
        }
    }

    fn update_player(&mut self, dt: f32, keys: MoveKeys) {
        let pc = self.scn.player.clone();
        let yaw = self.player.yaw.to_radians() as f32;
        let fwd = vec3(-yaw.sin(), 0.0, yaw.cos());
        let right = vec3(-yaw.cos(), 0.0, -yaw.sin());
        let mut wish = Vec3::ZERO;
        if pc.can_move {
            if keys.forward {
                wish += fwd;
            }
            if keys.back {
                wish -= fwd;
            }
            if keys.right {
                wish += right;
            }
            if keys.left {
                wish -= right;
            }
        }
        let mut hv = if wish.length_squared() > 0.0 {
            wish.normalize() * pc.move_speed
        } else {
            Vec3::ZERO
        };
        if pc.forced_strafe > 0.0 && self.phase == Phase::Running {
            self.player.strafe_timer -= dt;
            if self.player.strafe_timer <= 0.0 {
                self.player.strafe_sign = -self.player.strafe_sign;
                self.player.strafe_timer =
                    self.rng.range(pc.forced_strafe_min, pc.forced_strafe_max);
            }
            // Strafe along the world X axis so the path stays inside the arena.
            hv += vec3(self.player.strafe_sign * pc.forced_strafe, 0.0, 0.0);
        }
        self.player.vel.x = hv.x;
        self.player.vel.z = hv.z;
        let floor = self.scn.player.position[1].max(self.floor());
        if self.player.on_ground && keys.jump && pc.can_move {
            self.player.vel.y = pc.jump_velocity;
            self.player.on_ground = false;
        }
        if !self.player.on_ground {
            self.player.vel.y -= pc.gravity * dt;
        }
        self.player.pos += self.player.vel * dt;
        if self.player.pos.y <= floor {
            self.player.pos.y = floor;
            self.player.vel.y = 0.0;
            self.player.on_ground = true;
        }
        let amin = Vec3::from(self.scn.arena.min);
        let amax = Vec3::from(self.scn.arena.max);
        let m = 0.4;
        let px = self.player.pos.x;
        self.player.pos.x = clamp_mid(px, amin.x + m, amax.x - m);
        if self.player.pos.x != px && pc.forced_strafe > 0.0 {
            self.player.strafe_sign = -self.player.strafe_sign;
        }
        self.player.pos.z = clamp_mid(self.player.pos.z, amin.z + m, amax.z - m);
    }

    fn update_targets(&mut self, dt: f32) {
        let cfg = self.scn.targets.clone();
        let mv = self.scn.movement.clone();
        let floor = self.floor().max(match cfg.shape {
            TargetShape::Bot => self.bmin.y.min(self.floor() + 0.0),
            TargetShape::Sphere => self.floor(),
        });
        let eye = self.eye();
        let time = self.time;
        let mut expired = vec![];
        for i in 0..self.targets.len() {
            let mut t = self.targets[i].clone();
            t.flash = (t.flash - dt * 6.0).max(0.0);
            // Lifetime.
            if cfg.lifetime > 0.0 && time - t.spawn_time >= cfg.lifetime {
                expired.push(i);
                continue;
            }
            // Shrink.
            if cfg.shrink_rate > 0.0 {
                t.radius = (t.radius - cfg.shrink_rate * dt).max(cfg.min_radius);
            }
            // Regen.
            if !cfg.invincible && cfg.regen_rate > 0.0 && time - t.last_damage >= cfg.regen_delay {
                t.health = (t.health + cfg.regen_rate * dt).min(t.max_health);
            }
            // Blink.
            if mv.blink_max > 0.0 {
                t.blink_timer -= dt;
                if t.blink_timer <= 0.0 {
                    let (p, _) = self.random_spawn_point(&self.scn.spawn.clone());
                    t.pos = self.place(p, &cfg);
                    t.blink_timer = self.rng.range(mv.blink_min, mv.blink_max);
                }
            }
            self.move_target(&mut t, &cfg, &mv, dt, eye, floor);
            self.targets[i] = t;
        }
        for &i in expired.iter().rev() {
            let t = self.targets.remove(i);
            self.expired += 1;
            self.score -= self.scn.scoring.per_expire;
            self.last_slot = t.slot;
            self.events.push(GameEvent::Expire);
            self.queue_respawn();
        }
    }

    fn move_target(
        &mut self,
        t: &mut Target,
        cfg: &TargetCfg,
        mv: &MovementCfg,
        dt: f32,
        eye: Vec3,
        floor: f32,
    ) {
        let is_bot = cfg.shape == TargetShape::Bot;
        let r = if is_bot { cfg.radius } else { t.radius };
        // Effective bounds for the target reference point.
        let mut lo = self.bmin + Vec3::splat(r);
        let mut hi = self.bmax - Vec3::splat(r);
        if is_bot {
            lo.y = floor;
            hi.y = (self.bmax.y - cfg.height).max(floor);
        }
        for k in 0..3 {
            if lo[k] > hi[k] {
                let mid = (self.bmin[k] + self.bmax[k]) * 0.5;
                lo[k] = mid;
                hi[k] = mid;
            }
        }
        let ground_y = if is_bot {
            floor
        } else {
            (floor + t.radius).max(lo.y)
        };

        t.change_timer -= dt;
        let change = t.change_timer <= 0.0;
        if change {
            t.change_timer = self.rng.range(mv.change_min, mv.change_max);
        }
        let speed = |rng: &mut Rng| rng.range(mv.speed_min, mv.speed_max);
        match mv.pattern {
            MovePattern::Static => return,
            MovePattern::Bounce => {
                if change {
                    let mut v = vec3(
                        if mv.axes[0] {
                            self.rng.range(-1.0, 1.0)
                        } else {
                            0.0
                        },
                        if mv.axes[1] && !is_bot {
                            self.rng.range(-1.0, 1.0)
                        } else {
                            0.0
                        },
                        if mv.axes[2] {
                            self.rng.range(-1.0, 1.0)
                        } else {
                            0.0
                        },
                    );
                    if v.length_squared() < 1e-4 {
                        v = vec3(
                            if mv.axes[0] { 1.0 } else { 0.0 },
                            if mv.axes[1] { 1.0 } else { 0.0 },
                            0.0,
                        );
                    }
                    t.desired = v.normalize_or_zero() * speed(&mut self.rng);
                }
                approach(&mut t.vel, t.desired, mv.accel, dt);
            }
            MovePattern::Strafe | MovePattern::Air => {
                let to_p = (eye - t.pos) * vec3(1.0, 0.0, 1.0);
                let axis = if to_p.length() > 0.01 {
                    let n = to_p.normalize();
                    vec3(n.z, 0.0, -n.x)
                } else {
                    Vec3::X
                };
                if change {
                    if self.rng.chance(0.8) {
                        t.dir_sign = -t.dir_sign;
                    }
                    let s = speed(&mut self.rng);
                    t.desired = axis * t.dir_sign * s;
                    if mv.pattern == MovePattern::Strafe
                        && mv.jump_chance > 0.0
                        && t.pos.y <= ground_y + 0.01
                        && self.rng.chance(mv.jump_chance)
                    {
                        t.vel.y = self.rng.range(mv.jump_min, mv.jump_max);
                    }
                } else {
                    // Keep strafing perpendicular as the player moves.
                    let s = t.desired.length();
                    t.desired = axis * t.dir_sign * s;
                }
                let vy = t.vel.y;
                let mut hv = vec3(t.vel.x, 0.0, t.vel.z);
                approach(&mut hv, t.desired, mv.accel, dt);
                t.vel = vec3(hv.x, vy, hv.z);
                let grounded = t.pos.y <= ground_y + 0.01;
                if mv.pattern == MovePattern::Air && grounded && t.vel.y <= 0.0 {
                    t.vel.y = self.rng.range(mv.jump_min, mv.jump_max);
                }
                if !grounded || t.vel.y > 0.0 {
                    t.vel.y -= mv.gravity * dt;
                }
            }
            MovePattern::Popcorn => {
                if t.pos.y <= ground_y + 0.01 && t.vel.y <= 0.0 {
                    t.vel.y = self.rng.range(mv.jump_min, mv.jump_max);
                    let a = self.rng.range(0.0, std::f32::consts::TAU);
                    let s = speed(&mut self.rng);
                    t.vel.x = a.cos() * s;
                    t.vel.z = a.sin() * s * if mv.axes[2] { 1.0 } else { 0.3 };
                }
                t.vel.y -= mv.gravity * dt;
            }
            MovePattern::Orbit => {
                if change || t.orbit_speed == 0.0 {
                    if self.rng.chance(0.75) {
                        t.dir_sign = -t.dir_sign;
                    }
                    t.desired.x = t.dir_sign * speed(&mut self.rng);
                }
                let target_w = t.desired.x;
                let mut w = vec3(t.orbit_speed, 0.0, 0.0);
                approach(&mut w, vec3(target_w, 0.0, 0.0), mv.accel, dt);
                t.orbit_speed = w.x;
                t.orbit_angle += t.orbit_speed / t.orbit_radius * dt;
                let c = self.spawn_eye;
                let np = vec3(
                    c.x + t.orbit_angle.cos() * t.orbit_radius,
                    t.pos.y,
                    c.z + t.orbit_angle.sin() * t.orbit_radius,
                );
                t.vel = (np - t.pos) / dt.max(1e-4);
                t.pos = np;
                t.pos.x = t.pos.x.clamp(lo.x, hi.x);
                t.pos.z = t.pos.z.clamp(lo.z, hi.z);
                return;
            }
            MovePattern::Glider => {
                if change {
                    let a = self.rng.range(0.0, std::f32::consts::TAU);
                    let s = speed(&mut self.rng);
                    t.desired = vec3(a.cos() * s, 0.0, a.sin() * s * 0.5);
                }
                let mut hv = vec3(t.vel.x, 0.0, t.vel.z);
                approach(&mut hv, t.desired, mv.accel, dt);
                t.vel = vec3(hv.x, -mv.gravity, hv.z);
                if t.pos.y <= ground_y + 0.01 {
                    t.pos.y = hi.y;
                }
            }
        }

        t.pos += t.vel * dt;
        // Reflect off bounds.
        for k in 0..3 {
            if t.pos[k] < lo[k] {
                t.pos[k] = lo[k];
                if k == 1
                    && t.vel[k] < 0.0
                    && matches!(
                        mv.pattern,
                        MovePattern::Air | MovePattern::Strafe | MovePattern::Popcorn
                    )
                {
                    t.vel[k] = 0.0;
                } else if k == 1 && mv.pattern == MovePattern::Glider {
                    // handled above
                } else {
                    t.vel[k] = t.vel[k].abs();
                    t.desired[k] = t.desired[k].abs();
                    if matches!(mv.pattern, MovePattern::Strafe | MovePattern::Air) && k != 1 {
                        t.dir_sign = -t.dir_sign;
                    }
                }
            } else if t.pos[k] > hi[k] {
                t.pos[k] = hi[k];
                t.vel[k] = -t.vel[k].abs();
                t.desired[k] = -t.desired[k].abs();
                if matches!(mv.pattern, MovePattern::Strafe | MovePattern::Air) && k != 1 {
                    t.dir_sign = -t.dir_sign;
                }
            }
        }
        if t.pos.y < ground_y {
            t.pos.y = ground_y;
        }
        // Keep-out bubble around the player.
        let keep = mv.min_player_dist;
        if keep > 0.0 {
            let mut d = t.pos - eye;
            if is_bot {
                d.y = 0.0;
            }
            let len = d.length();
            if len < keep && len > 1e-4 {
                let n = d / len;
                let push = n * (keep - len);
                t.pos += push;
                let vn = t.vel.dot(n);
                if vn < 0.0 {
                    t.vel -= n * (2.0 * vn);
                }
                let dn = t.desired.dot(n);
                if dn < 0.0 {
                    t.desired -= n * (2.0 * dn);
                }
                for k in 0..3 {
                    t.pos[k] = t.pos[k].clamp(lo[k].min(hi[k]), hi[k].max(lo[k]));
                }
            }
        }
    }

    pub fn record(&self, cm360: f32, hfov: f32) -> RunRecord {
        let avg_ttk = if self.kill_log.is_empty() {
            0.0
        } else {
            self.kill_log.iter().map(|k| k.ttk).sum::<f32>() / self.kill_log.len() as f32
        };
        RunRecord {
            score: (self.score * 10.0).round() / 10.0,
            accuracy: self.accuracy(),
            kills: self.kills,
            shots: self.shots,
            hits: self.hits,
            damage_done: self.damage_done,
            damage_possible: self.damage_possible,
            avg_ttk,
            duration: self.time,
            date: crate::stats::now_string(),
            cm360,
            hfov,
            replay: None,
        }
    }
}

fn approach(v: &mut Vec3, target: Vec3, accel: f32, dt: f32) {
    if accel <= 0.0 {
        *v = target;
        return;
    }
    let diff = target - *v;
    let step = accel * dt;
    if diff.length() <= step {
        *v = target;
    } else {
        *v += diff.normalize() * step;
    }
}

fn clamp_mid(v: f32, lo: f32, hi: f32) -> f32 {
    if lo > hi {
        (lo + hi) * 0.5
    } else {
        v.clamp(lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_tests() {
        let o = Vec3::ZERO;
        let d = vec3(0.0, 0.0, 1.0);
        assert!(ray_sphere(o, d, vec3(0.0, 0.0, 10.0), 0.5).is_some());
        assert!(ray_sphere(o, d, vec3(0.6, 0.0, 10.0), 0.5).is_none());
        assert!(ray_capsule(o, d, vec3(0.0, -1.0, 10.0), vec3(0.0, 1.0, 10.0), 0.3).is_some());
        assert!(ray_capsule(o, d, vec3(0.5, -1.0, 10.0), vec3(0.5, 1.0, 10.0), 0.3).is_none());
    }

    #[test]
    fn aim_convention() {
        let f = aim_dir(0.0, 0.0);
        assert!((f - vec3(0.0, 0.0, 1.0)).length() < 1e-5);
        // Positive yaw turns right (towards -X).
        assert!(aim_dir(90.0, 0.0).x < -0.99);
        assert!(aim_dir(0.0, 90.0).y > 0.99);
    }

    #[test]
    fn click_kills_target_in_front() {
        let mut s = Scenario::default();
        s.targets.max_alive = 1;
        s.spawn.box_min = [0.0, 1.6, 10.0];
        s.spawn.box_max = [0.0, 1.6, 10.0];
        let mut g = Game::new(s, 0.0, false, 7);
        g.update(0.016, MoveKeys::default());
        g.trigger_down();
        assert_eq!(g.kills, 1);
        assert_eq!(g.hits, 1);
        assert!(g.score > 0.0);
    }

    #[test]
    fn every_playlist_style_runs() {
        let names = [
            "1w4ts reload",
            "Pasu Voltaic",
            "Popcorn Voltaic",
            "B180T Voltaic",
            "Air Angelic 4 Voltaic",
            "Close Fast Strafes Invincible",
            "Controlsphere",
            "FuglaaXYZ Voltaic",
            "Floating Heads Timing 400%",
            "Manwing Strafe Gallery V3",
            "Tile Frenzy 360 Strafing 400Prct Track",
            "Fortnite Gliders 360 Invincible",
            "Revolving Tracking Thin",
            "Pokeball Frenzy Auto 1w4ts",
            "Vertical Switching small lg",
            "patCircleSwitch NR",
        ];
        for n in names {
            let mut s = crate::synth::synthesize(n);
            sanitize(&mut s);
            let mut g = Game::new(s, 0.0, true, 42);
            g.trigger_down();
            for _ in 0..(60 * 70) {
                g.look(3.0, 0.5, 0.022, false);
                g.update(
                    1.0 / 60.0,
                    MoveKeys {
                        left: true,
                        ..Default::default()
                    },
                );
                for t in &g.targets {
                    assert!(t.pos.is_finite(), "{n}: non-finite position");
                    let amin = Vec3::from(g.scn.arena.min) - Vec3::splat(0.01);
                    let amax = Vec3::from(g.scn.arena.max) + Vec3::splat(0.01);
                    assert!(
                        t.pos.cmpge(amin).all() && t.pos.cmple(amax).all(),
                        "{n}: target left arena {:?}",
                        t.pos
                    );
                }
            }
            assert_eq!(g.phase, Phase::Finished, "{n}");
            assert!(
                !g.targets.is_empty()
                    || g.scn.targets.total > 0
                    || g.scn.targets.respawn_delay > 0.0,
                "{n}"
            );
        }
    }
}

#[cfg(test)]
mod corpus_tests {
    use super::*;

    /// `PLAYLIST_DIR=/path/to/playlists cargo test --no-default-features corpus -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn corpus() {
        let dir = std::env::var("PLAYLIST_DIR").expect("set PLAYLIST_DIR");
        let (pls, errs) = crate::playlist::load_dir(std::path::Path::new(&dir));
        println!("playlists: {}  errors: {}", pls.len(), errs.len());
        for e in &errs {
            println!("  ERR {e}");
        }
        let mut names: Vec<String> = pls
            .iter()
            .flat_map(|p| p.entries.iter().map(|e| e.name.clone()))
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by_key(|n| n.to_lowercase());
        let mut by_arch: std::collections::BTreeMap<String, usize> = Default::default();
        let mut by_cat: std::collections::BTreeMap<String, usize> = Default::default();
        for n in &names {
            let mut s = crate::synth::synthesize(n);
            sanitize(&mut s);
            *by_arch
                .entry(format!("{:?}", crate::synth::archetype_of(n)))
                .or_default() += 1;
            *by_cat.entry(format!("{:?}", s.category)).or_default() += 1;
            let mut g = Game::new(s, 0.0, true, 99);
            g.trigger_down();
            let mut frames = 0;
            while g.phase != Phase::Finished && frames < 60 * 400 {
                // Crude aimbot: look at the nearest target so hits/kills happen.
                if let Some(t) = g.targets.first() {
                    let aim = if g.scn.targets.head_only {
                        t.bot_geom(&g.scn.targets)
                            .head
                            .unwrap_or(t.center(&g.scn.targets))
                    } else {
                        t.center(&g.scn.targets)
                    };
                    let to = (aim - g.eye()).normalize_or_zero();
                    let yaw = (-to.x).atan2(to.z).to_degrees() as f64;
                    let pitch = to.y.asin().to_degrees() as f64;
                    g.player.yaw = yaw;
                    g.player.pitch = pitch;
                }
                if frames % 7 == 0 && g.scn.weapon.mode == FireMode::Semi {
                    g.trigger_up();
                    g.trigger_down();
                }
                g.update(1.0 / 120.0, MoveKeys::default());
                for t in &g.targets {
                    assert!(t.pos.is_finite() && t.radius > 0.0, "{n}");
                    let amin = Vec3::from(g.scn.arena.min) - Vec3::splat(0.05);
                    let amax = Vec3::from(g.scn.arena.max) + Vec3::splat(0.05);
                    assert!(
                        t.pos.cmpge(amin).all() && t.pos.cmple(amax).all(),
                        "{n}: left arena {:?}",
                        t.pos
                    );
                }
                frames += 1;
            }
            assert_eq!(g.phase, Phase::Finished, "{n} never finished");
            assert!(g.hits > 0, "{n}: aimbot never hit anything");
        }
        println!("unique scenario names: {}", names.len());
        println!("by archetype: {by_arch:#?}");
        println!("by category: {by_cat:#?}");
    }
}
