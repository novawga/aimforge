//! 3D scene + crosshair drawing.

use crate::game::{Game, Target};
use crate::scenario::{TargetCfg, TargetShape};
use crate::settings::Settings;
use crate::theme;
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;

pub struct Renderer {
    sphere: Vec<(Vec3, Vec3)>, // unit positions + normals
    sphere_idx: Vec<u16>,
    capsule_split: Vec<f32>, // +1 / -1 per vertex: which hemisphere
}

const RINGS: usize = 17; // odd so no vertex ring sits on the equator
const SEGS: usize = 28;

impl Renderer {
    pub fn new() -> Renderer {
        let mut sphere = vec![];
        let mut split = vec![];
        for i in 0..=RINGS {
            let v = i as f32 / RINGS as f32;
            let phi = v * std::f32::consts::PI; // 0 = top
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
        Renderer {
            sphere,
            sphere_idx: idx,
            capsule_split: split,
        }
    }

    fn shade(&self, n: Vec3, base: Color, view: Vec3, flash: f32) -> Color {
        let l = vec3(0.35, 0.8, -0.45).normalize();
        let diff = n.dot(l).max(0.0);
        let rim = (1.0 - n.dot(-view).max(0.0)).powf(3.0) * 0.25;
        let k = 0.38 + 0.62 * diff;
        let spec = n.dot((l - view).normalize()).max(0.0).powf(24.0) * 0.35;
        let mut c = Color::new(
            base.r * k + spec + rim,
            base.g * k + spec + rim,
            base.b * k + spec + rim,
            1.0,
        );
        if flash > 0.0 {
            c.r += (1.0 - c.r) * flash * 0.7;
            c.g += (1.0 - c.g) * flash * 0.7;
            c.b += (1.0 - c.b) * flash * 0.7;
        }
        c
    }

    /// Sphere (half_len = 0) or vertical capsule centred at `c`.
    fn draw_blob(&self, c: Vec3, r: f32, half_len: f32, color: Color, eye: Vec3, flash: f32) {
        let view = (c - eye).normalize_or_zero();
        let verts: Vec<Vertex> = self
            .sphere
            .iter()
            .zip(&self.capsule_split)
            .map(|((p, n), s)| {
                let pos = c + *p * r + vec3(0.0, half_len * s, 0.0);
                let col = self.shade(*n, color, view, flash);
                Vertex::new(pos.x, pos.y, pos.z, 0.0, 0.0, col)
            })
            .collect();
        draw_mesh(&Mesh {
            vertices: verts,
            indices: self.sphere_idx.clone(),
            texture: None,
        });
    }

    pub fn draw_target(&self, t: &Target, cfg: &TargetCfg, base: Color, eye: Vec3) {
        match cfg.shape {
            TargetShape::Sphere => self.draw_blob(t.pos, t.radius, 0.0, base, eye, t.flash),
            TargetShape::Bot => {
                let g = t.bot_geom(cfg);
                let mid = (g.body_a + g.body_b) * 0.5;
                let half = (g.body_b.y - g.body_a.y) * 0.5;
                let body_col = if cfg.head_only {
                    darken(base, 0.45)
                } else {
                    base
                };
                self.draw_blob(mid, t.radius, half, body_col, eye, t.flash);
                if let Some(h) = g.head {
                    self.draw_blob(
                        h,
                        t.head_radius(cfg),
                        0.0,
                        lighten(base, 0.12),
                        eye,
                        t.flash,
                    );
                }
            }
        }
    }

    pub fn draw_world(&self, game: &Game, settings: &Settings) {
        let a = &game.scn.arena;
        let (mn, mx) = (Vec3::from(a.min), Vec3::from(a.max));
        draw_room(mn, mx, a.grid);
        let base = target_color(game, settings);
        let eye = game.eye();
        for t in &game.targets {
            self.draw_target(t, &game.scn.targets, base, eye);
        }
    }
}

pub fn target_color(game: &Game, settings: &Settings) -> Color {
    match (game.scn.targets.color, settings.target_color) {
        (Some(c), _) | (None, Some(c)) => rgb(c),
        _ => theme::t().target,
    }
}

pub fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

pub fn darken(c: Color, k: f32) -> Color {
    Color::new(c.r * (1.0 - k), c.g * (1.0 - k), c.b * (1.0 - k), c.a)
}

pub fn lighten(c: Color, k: f32) -> Color {
    Color::new(
        c.r + (1.0 - c.r) * k,
        c.g + (1.0 - c.g) * k,
        c.b + (1.0 - c.b) * k,
        c.a,
    )
}

fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3, col_a: Color, col_b: Color) {
    let v = vec![
        Vertex::new(a.x, a.y, a.z, 0.0, 0.0, col_a),
        Vertex::new(b.x, b.y, b.z, 1.0, 0.0, col_a),
        Vertex::new(c.x, c.y, c.z, 1.0, 1.0, col_b),
        Vertex::new(d.x, d.y, d.z, 0.0, 1.0, col_b),
    ];
    draw_mesh(&Mesh {
        vertices: v,
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: None,
    });
}

/// Grid room in the active theme's colours.
fn draw_room(mn: Vec3, mx: Vec3, grid: f32) {
    let th = theme::t();
    let (wall_lo, wall_hi, floor, floor_far, ceil) = (th.wall, th.wall_top, th.floor, th.floor_far, th.ceiling);
    let (x0, y0, z0, x1, y1, z1) = (mn.x, mn.y, mn.z, mx.x, mx.y, mx.z);
    quad(vec3(x0, y0, z0), vec3(x1, y0, z0), vec3(x1, y0, z1), vec3(x0, y0, z1), floor, floor_far);
    quad(vec3(x0, y1, z0), vec3(x1, y1, z0), vec3(x1, y1, z1), vec3(x0, y1, z1), ceil, ceil);
    quad(vec3(x0, y0, z1), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x0, y1, z1), wall_lo, wall_hi);
    quad(vec3(x0, y0, z0), vec3(x1, y0, z0), vec3(x1, y1, z0), vec3(x0, y1, z0), wall_lo, wall_hi);
    quad(vec3(x0, y0, z0), vec3(x0, y0, z1), vec3(x0, y1, z1), vec3(x0, y1, z0), wall_lo, wall_hi);
    quad(vec3(x1, y0, z0), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x1, y1, z0), wall_lo, wall_hi);
    if grid <= 0.0 {
        return;
    }
    let line = th.grid;
    let floor_line = th.floor_grid;
    let e = 0.01;
    let step = grid.max(0.25);
    // floor lines
    let mut x = (x0 / step).ceil() * step;
    while x <= x1 {
        draw_line_3d(vec3(x, y0 + e, z0), vec3(x, y0 + e, z1), floor_line);
        x += step;
    }
    let mut z = (z0 / step).ceil() * step;
    while z <= z1 {
        draw_line_3d(vec3(x0, y0 + e, z), vec3(x1, y0 + e, z), floor_line);
        z += step;
    }
    // wall lines: verticals on far/near walls and side walls, horizontals all round
    let mut x = (x0 / step).ceil() * step;
    while x <= x1 {
        draw_line_3d(vec3(x, y0, z1 - e), vec3(x, y1, z1 - e), line);
        draw_line_3d(vec3(x, y0, z0 + e), vec3(x, y1, z0 + e), line);
        x += step;
    }
    let mut z = (z0 / step).ceil() * step;
    while z <= z1 {
        draw_line_3d(vec3(x0 + e, y0, z), vec3(x0 + e, y1, z), line);
        draw_line_3d(vec3(x1 - e, y0, z), vec3(x1 - e, y1, z), line);
        z += step;
    }
    let mut y = (y0 / step).ceil() * step;
    while y <= y1 {
        draw_line_3d(vec3(x0, y, z1 - e), vec3(x1, y, z1 - e), line);
        draw_line_3d(vec3(x0, y, z0 + e), vec3(x1, y, z0 + e), line);
        draw_line_3d(vec3(x0 + e, y, z0), vec3(x0 + e, y, z1), line);
        draw_line_3d(vec3(x1 - e, y, z0), vec3(x1 - e, y, z1), line);
        y += step;
    }
}

pub fn camera_for(game: &Game, settings: &Settings) -> Camera3D {
    let eye = game.eye();
    let aspect = screen_width() / screen_height().max(1.0);
    Camera3D {
        position: eye,
        target: eye + game.dir(),
        up: Vec3::Y,
        fovy: settings.vfov_radians(aspect),
        ..Default::default()
    }
}

/// Project a world point to screen pixels (None if behind the camera).
pub fn world_to_screen(cam: &Camera3D, p: Vec3) -> Option<Vec2> {
    let m = cam.matrix();
    let c = m * vec4(p.x, p.y, p.z, 1.0);
    if c.w <= 0.01 {
        return None;
    }
    let ndc = vec2(c.x / c.w, c.y / c.w);
    Some(vec2(
        (ndc.x + 1.0) * 0.5 * screen_width(),
        (1.0 - ndc.y) * 0.5 * screen_height(),
    ))
}
