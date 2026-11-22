//! Startup loading animation, and the animated range behind the sign-in screen.
//!
//! Everything here is a pure function of time, so it needs no state: a crosshair
//! flicks between targets on a perspective range while the game loads.

use crate::theme::{self, mix, with_alpha};
use crate::ui::{size, Ui};
use macroquad::prelude::*;
use std::f32::consts::TAU;

/// Seconds per flick at speed 1.
const PERIOD: f64 = 0.5;
/// Part of each cycle spent moving; the shot lands at the end of it.
const MOVE: f64 = 0.44;
/// How long a target's pop lingers.
const BURST: f32 = 0.5;

fn hash(k: i64, salt: u64) -> f32 {
    let mut x = (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 31;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 29;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

fn ease_out_back(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

pub struct Scene {
    pub center: Vec2,
    /// Half-height of the area targets appear in, in pixels.
    pub radius: f32,
    pub alpha: f32,
    pub ball: Color,
    pub speed: f64,
}

fn target_pos(k: i64, s: &Scene) -> Vec2 {
    // Spread targets over a wide ellipse, never too close to the previous one.
    let a = hash(k, 1) * TAU;
    let d = 0.3 + 0.7 * hash(k, 2).sqrt();
    vec2(s.center.x + a.cos() * d * s.radius * 1.9, s.center.y + a.sin() * d * s.radius)
}

/// A lit sphere with a soft glow.
pub fn draw_ball(p: Vec2, r: f32, c: Color, a: f32) {
    if r <= 0.5 || a <= 0.01 {
        return;
    }
    for i in 0..6 {
        let k = i as f32 / 6.0;
        draw_circle(p.x, p.y, r * (1.25 + k * 1.1), with_alpha(c, a * 0.05 * (1.0 - k)));
    }
    draw_circle(p.x, p.y, r, with_alpha(mix(c, BLACK, 0.45), a));
    draw_circle(p.x - r * 0.08, p.y - r * 0.1, r * 0.88, with_alpha(mix(c, BLACK, 0.15), a));
    draw_circle(p.x - r * 0.18, p.y - r * 0.22, r * 0.66, with_alpha(c, a));
    draw_circle(p.x - r * 0.3, p.y - r * 0.34, r * 0.34, with_alpha(mix(c, WHITE, 0.35), a));
    draw_circle(p.x - r * 0.36, p.y - r * 0.42, r * 0.13, with_alpha(WHITE, a * 0.85));
}

fn draw_crosshair(p: Vec2, s: f32, c: Color, a: f32) {
    let (gap, len, th) = (s * 0.55, s * 1.1, (s * 0.16).max(1.5));
    let sh = with_alpha(BLACK, a * 0.5);
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        let (x0, y0) = (p.x + dx * gap, p.y + dy * gap);
        let (x1, y1) = (p.x + dx * (gap + len), p.y + dy * (gap + len));
        draw_line(x0 + 1.0, y0 + 1.0, x1 + 1.0, y1 + 1.0, th, sh);
        draw_line(x0, y0, x1, y1, th, with_alpha(c, a));
    }
    draw_circle(p.x, p.y, th * 0.75, with_alpha(c, a));
}

/// Perspective floor grid sliding toward the viewer.
pub fn draw_range(t: f64, alpha: f32, horizon: f32) {
    let th = theme::t();
    let (sw, sh) = (screen_width(), screen_height());
    let cx = sw * 0.5;
    let line = mix(th.accent, th.text, 0.5);
    let depth_h = sh - horizon;
    // Vertical lines converge on the horizon.
    for i in -14..=14 {
        let bx = cx + i as f32 * sw * 0.11;
        let a = alpha * 0.22 * (1.0 - (i as f32).abs() / 15.0);
        draw_line(cx + (bx - cx) * 0.02, horizon, bx, sh, 1.0, with_alpha(line, a));
    }
    // Horizontal lines, closer together toward the horizon.
    let off = (t * 0.35).fract() as f32;
    for i in 1..28 {
        let d = i as f32 - off;
        let y = horizon + depth_h * (0.9 / d).min(1.2);
        if y > sh || d <= 0.05 {
            continue;
        }
        let k = (y - horizon) / depth_h;
        draw_line(0.0, y, sw, y, 1.0, with_alpha(line, alpha * 0.24 * k));
    }
    // Horizon glow.
    for i in 0..10 {
        let k = i as f32 / 10.0;
        let h = depth_h * 0.05 * (1.0 + k * 4.0);
        draw_rectangle(0.0, horizon - h * 0.5, sw, h, with_alpha(th.accent, alpha * 0.012 * (1.0 - k)));
    }
}

/// Soft radial light behind the targets.
pub fn draw_glow(c: Vec2, r: f32, col: Color, a: f32) {
    for i in 0..18 {
        let k = i as f32 / 18.0;
        draw_circle(c.x, c.y, r * (0.3 + k * 1.7), with_alpha(col, a * 0.018 * (1.0 - k)));
    }
}

/// Gridshot-style flicking: three live targets, a crosshair snapping to each and
/// popping it. Returns the crosshair position.
pub fn draw_flicks(t: f64, s: &Scene) -> Vec2 {
    let th = theme::t();
    let tt = t * s.speed;
    let cyc = tt / PERIOD;
    let k = cyc.floor() as i64;
    let f = cyc - cyc.floor();
    let ball_r = s.radius * 0.085;
    let shot_time = |j: i64| (j as f64 + MOVE) * PERIOD;

    // Live targets: the one being flicked to plus the next two.
    for j in k..k + 3 {
        if j == k && f >= MOVE {
            continue;
        }
        let spawn = shot_time(j - 3);
        let age = ((tt - spawn) / s.speed.max(0.01)) as f32;
        let sc = ease_out_back(age / 0.28);
        draw_ball(target_pos(j, s), ball_r * sc, s.ball, s.alpha * (age / 0.12).clamp(0.0, 1.0));
    }

    // Pops.
    for j in k - 2..=k {
        let age = ((tt - shot_time(j)) / s.speed.max(0.01)) as f32;
        if !(0.0..BURST).contains(&age) {
            continue;
        }
        let p = target_pos(j, s);
        let q = age / BURST;
        let a = s.alpha * (1.0 - q).powi(2);
        let rr = ball_r * (1.0 + ease_out_cubic(q) * 2.6);
        draw_poly_lines(p.x, p.y, 48, rr, 0.0, (ball_r * 0.16 * (1.0 - q)).max(1.0), with_alpha(s.ball, a));
        draw_circle(p.x, p.y, ball_r * 0.9 * (1.0 - q).powi(3), with_alpha(mix(s.ball, WHITE, 0.6), a));
        for n in 0..8 {
            let ang = n as f32 / 8.0 * TAU + hash(j, 7) * TAU;
            let d0 = ball_r * (0.6 + q * 2.2);
            let d1 = d0 + ball_r * 0.7 * (1.0 - q);
            let (c, sn) = (ang.cos(), ang.sin());
            draw_line(p.x + c * d0, p.y + sn * d0, p.x + c * d1, p.y + sn * d1, (ball_r * 0.12).max(1.0), with_alpha(s.ball, a));
        }
    }

    // Crosshair.
    let from = target_pos(k - 1, s);
    let to = target_pos(k, s);
    let m = (f / MOVE) as f32;
    let e = ease_out_cubic(m);
    let pos = from.lerp(to, e);
    if m < 1.0 {
        // motion streak
        let tail = from.lerp(to, ease_out_cubic((m - 0.18).max(0.0)));
        draw_line(tail.x, tail.y, pos.x, pos.y, (ball_r * 0.12).max(1.0), with_alpha(th.text, s.alpha * 0.18 * (1.0 - m)));
    }
    let since_shot = ((tt - shot_time(k)) / s.speed.max(0.01)) as f32;
    let kick = if (0.0..0.12).contains(&since_shot) { 1.0 - since_shot / 0.12 } else { 0.0 };
    draw_crosshair(pos, ball_r * (0.42 + kick * 0.25), mix(th.text, s.ball, 0.15), s.alpha);
    pos
}

/// The whole loading screen for one frame.
/// `t` seconds since launch, `progress` 0..1, `exit` 0..1 while fading out.
pub fn draw_loading(ui: &Ui, t: f64, progress: f32, status: &str, exit: f32, ball: Color) {
    let th = theme::t();
    let (sw, sh) = (screen_width(), screen_height());
    clear_background(th.bg);
    let fade_in = (t as f32 / 0.5).clamp(0.0, 1.0);
    let a = fade_in * (1.0 - exit);
    let horizon = sh * 0.6;
    draw_range(t, a, horizon);
    let center = vec2(sw * 0.5, sh * 0.38);
    draw_glow(center, sh * 0.42, th.accent, a);
    let scene = Scene { center, radius: sh * 0.2 * (1.0 + exit * 0.25), alpha: a, ball, speed: 1.0 };
    draw_flicks(t, &scene);

    // Wordmark, letters rising in one after another.
    let word = "AIMFORGE";
    let fs = 46.0;
    let spacing = ui.s(14.0);
    let widths: Vec<f32> = word.chars().map(|c| ui.measure(&c.to_string(), fs, true)).collect();
    let total: f32 = widths.iter().sum::<f32>() + spacing * (word.len() as f32 - 1.0);
    let mut x = sw * 0.5 - total * 0.5;
    let wy = sh * 0.66;
    for (i, c) in word.chars().enumerate() {
        let lt = ((t as f32 - 0.15 - i as f32 * 0.06) / 0.4).clamp(0.0, 1.0);
        let e = ease_out_cubic(lt);
        let col = if i < 3 { th.text } else { mix(th.text, th.accent, 0.55) };
        ui.strong(&c.to_string(), x, wy + (1.0 - e) * ui.s(18.0), fs, with_alpha(col, e * (1.0 - exit)));
        x += widths[i] + spacing;
    }

    // Progress bar.
    let bw = ui.s(360.0);
    let by = wy + ui.s(92.0);
    let bx = sw * 0.5 - bw * 0.5;
    let bh = ui.s(3.0).max(2.0);
    let ba = ((t as f32 - 0.4) / 0.4).clamp(0.0, 1.0) * (1.0 - exit);
    draw_rectangle(bx, by, bw, bh, with_alpha(th.text, 0.08 * ba));
    let fill = bw * progress.clamp(0.0, 1.0);
    draw_rectangle(bx, by, fill, bh, with_alpha(th.accent, ba));
    for i in 0..5 {
        let r = bh * (1.5 + i as f32 * 1.6);
        draw_circle(bx + fill, by + bh * 0.5, r, with_alpha(th.accent, ba * 0.1));
    }
    let line = format!("{status}");
    ui.text_center(&line, sw * 0.5, by + ui.s(18.0), size::SMALL, with_alpha(th.text_dim, ba), false);
    ui.text_right(
        &format!("v{}", crate::update::VERSION.trim_start_matches('v')),
        sw - ui.s(24.0),
        sh - ui.s(36.0),
        size::CAPTION,
        with_alpha(th.text_faint, ba),
        false,
    );
    // Final fade into the menu.
    if exit > 0.0 {
        draw_rectangle(0.0, 0.0, sw, sh, with_alpha(th.bg, ease_out_cubic(exit)));
    }
}
