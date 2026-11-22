//! 3D scene: sky, arena and targets.

use crate::game::{Game, Target};
use crate::map::{BallStyle, MapCfg, MapRenderer};
use crate::scenario::{TargetCfg, TargetShape};
use crate::settings::Settings;
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;

pub struct Renderer {
    sphere: Vec<(Vec3, Vec3)>, // unit positions + normals
    sphere_idx: Vec<u16>,
    capsule_split: Vec<f32>,   // +1 / -1 per vertex: which hemisphere
    pub map: MapRenderer,
}

const RINGS: usize = 17; // odd so no vertex ring sits on the equator
const SEGS: usize = 28;

impl Renderer {
    pub fn new() -> Renderer {
        let mut sphere = vec![];
        let mut split = vec![];
        for i in 0..=RINGS {
            let v = i as f32 / RINGS as f32;
            let phi = v * std::f32::consts::PI;
            for j in 0..=SEGS {
                let u = j as f32 / SEGS as f32;
                let th = u * std::f32::consts::TAU;
                let n = vec3(phi.sin() * th.cos(), phi.cos(), phi.sin() * th.sin());
                sphere.push((n, n));
                split.push(if n.y >= 0.0 { 1.0 } else { -1.0 });
            }
        }
        let mut idx = vec![];
        let w = SEGS + 1;
        for i in 0..RINGS {
            for j in 0..SEGS {
                let a = (i * w + j) as u16;
                let b = ((i + 1) * w + j) as u16;
                idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
        Renderer { sphere, sphere_idx: idx, capsule_split: split, map: MapRenderer::new() }
    }

    fn shade(&self, n: Vec3, base: Color, view: Vec3, flash: f32, style: BallStyle) -> Color {
        let l = vec3(0.35, 0.8, -0.45).normalize();
        let ndl = n.dot(l).max(0.0);
        let facing = n.dot(-view).max(0.0);
        let rim = (1.0 - facing).powf(3.0);
        let half = (l - view).normalize();
        let (r, g, b) = match style {
            BallStyle::Flat => (base.r, base.g, base.b),
            BallStyle::Matte => {
                let k = 0.4 + 0.6 * ndl;
                let spec = n.dot(half).max(0.0).powf(20.0) * 0.25;
                (base.r * k + spec + rim * 0.2, base.g * k + spec + rim * 0.2, base.b * k + spec + rim * 0.2)
            }
            BallStyle::Glossy => {
                let k = 0.32 + 0.68 * ndl;
                let spec = n.dot(half).max(0.0).powf(70.0) * 0.9 + n.dot(half).max(0.0).powf(8.0) * 0.12;
                let env = rim * 0.35;
                (base.r * k + spec + env, base.g * k + spec + env, base.b * k + spec + env)
            }
            BallStyle::Glow => {
                let core = 0.85 + 0.25 * facing;
                let halo = rim * 0.9;
                (
                    base.r * core + halo * (0.6 + base.r * 0.4),
                    base.g * core + halo * (0.6 + base.g * 0.4),
                    base.b * core + halo * (0.6 + base.b * 0.4),
                )
            }
        };
        let mut c = Color::new(r.min(1.0), g.min(1.0), b.min(1.0), 1.0);
        if flash > 0.0 {
            c.r += (1.0 - c.r) * flash * 0.7;
            c.g += (1.0 - c.g) * flash * 0.7;
            c.b += (1.0 - c.b) * flash * 0.7;
        }
        c
    }

    /// Sphere (half_len = 0) or vertical capsule centred at `c`.
    #[allow(clippy::too_many_arguments)]
    fn draw_blob(&self, c: Vec3, r: f32, half_len: f32, color: Color, eye: Vec3, flash: f32, style: BallStyle) {
        let view = (c - eye).normalize_or_zero();
        let verts: Vec<Vertex> = self
            .sphere
            .iter()
            .zip(&self.capsule_split)
            .map(|((p, n), s)| {
                let pos = c + *p * r + vec3(0.0, half_len * s, 0.0);
                let col = self.shade(*n, color, view, flash, style);
                Vertex::new(pos.x, pos.y, pos.z, 0.0, 0.0, col)
            })
            .collect();
        draw_mesh(&Mesh { vertices: verts, indices: self.sphere_idx.clone(), texture: None });
    }

    pub fn draw_target(&self, t: &Target, cfg: &TargetCfg, base: Color, eye: Vec3, style: BallStyle) {
        match cfg.shape {
            TargetShape::Sphere => self.draw_blob(t.pos, t.radius, 0.0, base, eye, t.flash, style),
            TargetShape::Bot => {
                let g = t.bot_geom(cfg);
                let mid = (g.body_a + g.body_b) * 0.5;
                let half = (g.body_b.y - g.body_a.y) * 0.5;
                let body_col = if cfg.head_only { darken(base, 0.45) } else { base };
                self.draw_blob(mid, t.radius, half, body_col, eye, t.flash, style);
                if let Some(h) = g.head {
                    self.draw_blob(h, t.head_radius(cfg), 0.0, lighten(base, 0.12), eye, t.flash, style);
                }
            }
        }
    }

    /// Full scene for a running game: sky (2D pass), then arena and targets (3D).
    pub fn draw_scene(&self, game: &Game, settings: &Settings) -> Camera3D {
        let map = &settings.map;
        set_default_camera();
        self.map.draw_sky(map, game.dir(), settings.hfov, Rect::new(0.0, 0.0, screen_width(), screen_height()));
        let cam = camera_for(game, settings);
        set_camera(&cam);
        let a = &game.scn.arena;
        self.map.draw_arena(map, Vec3::from(a.min), Vec3::from(a.max), game.eye());
        let base = target_color(game, settings);
        let eye = game.eye();
        for t in &game.targets {
            self.draw_target(t, &game.scn.targets, base, eye, map.ball_style);
        }
        set_default_camera();
        cam
    }

    /// Live preview of a map (settings page) with a slowly panning camera.
    pub fn draw_map_preview(&self, map: &MapCfg, area: Rect) {
        let t = get_time() as f32 * 0.15;
        let yaw = t.sin() * 0.45;
        let fwd = vec3(-yaw.sin(), 0.04, yaw.cos()).normalize();
        let hfov = 100.0f32;
        self.map.draw_sky(map, fwd, hfov, area);
        let eye = vec3(0.0, 1.6, 0.0);
        let sh = screen_height();
        let vfov = 2.0 * ((hfov.to_radians() * 0.5).tan() * area.h / area.w).atan();
        let cam = Camera3D {
            position: eye,
            target: eye + fwd,
            up: Vec3::Y,
            fovy: vfov,
            aspect: Some(area.w / area.h),
            viewport: Some((area.x as i32, (sh - area.y - area.h) as i32, area.w as i32, area.h as i32)),
            ..Default::default()
        };
        set_camera(&cam);
        self.map.draw_arena(map, vec3(-14.0, 0.0, -3.0), vec3(14.0, 12.0, 19.5), eye);
        let base = Color::from_rgba(map.ball[0], map.ball[1], map.ball[2], 255);
        let cfg = TargetCfg::default();
        for (i, p) in [vec3(-3.2, 2.4, 13.0), vec3(1.2, 4.0, 15.0), vec3(4.6, 2.0, 12.0)].iter().enumerate() {
            let tg = Target::preview(i as u64, *p, 0.55);
            self.draw_target(&tg, &cfg, base, eye, map.ball_style);
        }
        set_default_camera();
    }
}

pub fn target_color(game: &Game, settings: &Settings) -> Color {
    match (game.scn.targets.color, settings.target_color) {
        (Some(c), _) | (None, Some(c)) => rgb(c),
        _ => rgb(settings.map.ball),
    }
}

pub fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

pub fn darken(c: Color, k: f32) -> Color {
    Color::new(c.r * (1.0 - k), c.g * (1.0 - k), c.b * (1.0 - k), c.a)
}

pub fn lighten(c: Color, k: f32) -> Color {
    Color::new(c.r + (1.0 - c.r) * k, c.g + (1.0 - c.g) * k, c.b + (1.0 - c.b) * k, c.a)
}

pub fn camera_for(game: &Game, settings: &Settings) -> Camera3D {
    let eye = game.eye();
    let aspect = screen_width() / screen_height().max(1.0);
    Camera3D { position: eye, target: eye + game.dir(), up: Vec3::Y, fovy: settings.vfov_radians(aspect), ..Default::default() }
}

/// Project a world point to screen pixels (None if behind the camera).
pub fn world_to_screen(cam: &Camera3D, p: Vec3) -> Option<Vec2> {
    let m = cam.matrix();
    let c = m * vec4(p.x, p.y, p.z, 1.0);
    if c.w <= 0.01 {
        return None;
    }
    let ndc = vec2(c.x / c.w, c.y / c.w);
    Some(vec2((ndc.x + 1.0) * 0.5 * screen_width(), (1.0 - ndc.y) * 0.5 * screen_height()))
}
