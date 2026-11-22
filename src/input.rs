//! Ordered mouse input. Clicks are processed in the exact order they happened
//! relative to mouse movement, so a flick-then-click inside one frame shoots
//! where the crosshair actually was when you clicked.

use macroquad::miniquad::{self, EventHandler};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug)]
pub enum InputEv {
    /// Mouse delta in counts (raw input while the cursor is grabbed).
    Motion(f32, f32),
    Down(MouseButton),
    Up(MouseButton),
}

pub struct Input {
    id: usize,
    last: Option<Vec2>,
    pub events: Vec<InputEv>,
    dedup: Dedup,
}

/// miniquad's X11 backend subscribes to raw motion from *every* XInput device,
/// so on Linux each physical mouse report shows up 2+ times in a row (slave +
/// master device). Every real report is repeated the same number of times, so
/// the shortest run of identical deltas reveals the duplication factor; runs
/// are divided by it. On Windows/macOS the factor stays 1 and nothing changes.
#[derive(Default)]
struct Dedup {
    recent_runs: std::collections::VecDeque<u32>,
    /// Trailing run from the previous frame that may continue into this one.
    carry: Option<(f32, f32, u32)>,
    /// Run currently being counted in the raw stream (spans frames).
    counting: Option<(f32, f32, u32)>,
    /// Factor learned in a previous grabbed session, used while re-learning.
    sticky: u32,
}

impl Dedup {
    /// Most common length among recent complete runs (1..=4).
    fn factor(&self) -> u32 {
        if self.recent_runs.len() < 12 {
            return self.sticky.max(1);
        }
        let mut hist = [0u32; 5];
        for &n in &self.recent_runs {
            if (1..=4).contains(&n) {
                hist[n as usize] += 1;
            }
        }
        (1..=4)
            .max_by_key(|&k| (hist[k], std::cmp::Reverse(k)))
            .unwrap_or(1) as u32
    }

    fn record(&mut self, n: u32) {
        self.recent_runs.push_back(n);
        if self.recent_runs.len() > 256 {
            self.recent_runs.pop_front();
        }
        if self.recent_runs.len() >= 64 {
            self.sticky = self.factor();
        }
    }

    /// Forget run statistics (the event source changes when the cursor is grabbed/released).
    fn reset(&mut self) {
        self.recent_runs.clear();
        self.carry = None;
        self.counting = None;
    }

    fn process(&mut self, evs: &mut Vec<InputEv>) {
        if !cfg!(target_os = "linux") {
            return;
        }
        // 1) Learn run lengths from the raw stream, across frame boundaries.
        for e in evs.iter() {
            match (*e, self.counting) {
                (InputEv::Motion(a, b), Some((x, y, n))) if a == x && b == y => {
                    self.counting = Some((x, y, n + 1))
                }
                (InputEv::Motion(a, b), prev) => {
                    if let Some((_, _, n)) = prev {
                        self.record(n);
                    }
                    self.counting = Some((a, b, 1));
                }
                _ => {}
            }
        }
        // 2) Group consecutive identical motions into runs, continuing the carried run.
        let mut runs: Vec<(InputEv, u32)> = vec![];
        if let Some((dx, dy, n)) = self.carry.take() {
            runs.push((InputEv::Motion(dx, dy), n));
        }
        for e in evs.drain(..) {
            if let (InputEv::Motion(a, b), Some((InputEv::Motion(x, y), n))) = (e, runs.last_mut())
            {
                if a == *x && b == *y {
                    *n += 1;
                    continue;
                }
            }
            runs.push((e, 1));
        }
        let k = self.factor();
        let last = runs.len().saturating_sub(1);
        for (i, (e, n)) in runs.into_iter().enumerate() {
            match e {
                InputEv::Motion(dx, dy) => {
                    if i == last && k > 1 && n % k != 0 {
                        // Possibly split by the frame boundary: emit whole multiples now, keep the rest.
                        let whole = n - n % k;
                        if whole > 0 {
                            let s = (whole / k) as f32;
                            evs.push(InputEv::Motion(dx * s, dy * s));
                        }
                        self.carry = Some((dx, dy, n % k));
                    } else {
                        let s = n as f32 / k as f32;
                        evs.push(InputEv::Motion(dx * s, dy * s));
                    }
                }
                other => evs.push(other),
            }
        }
    }

    /// Emit anything still carried (call when a frame had no new motion).
    fn flush(&mut self, evs: &mut Vec<InputEv>) {
        if let Some((dx, dy, n)) = self.carry.take() {
            let s = n as f32 / self.factor().max(1) as f32;
            evs.push(InputEv::Motion(dx * s, dy * s));
        }
    }
}

struct Collector<'a> {
    last: &'a mut Option<Vec2>,
    out: &'a mut Vec<InputEv>,
}

impl EventHandler for Collector<'_> {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let p = vec2(x, y);
        if let Some(l) = *self.last {
            let d = p - l;
            if d.x != 0.0 || d.y != 0.0 {
                self.out.push(InputEv::Motion(d.x, d.y));
            }
        }
        *self.last = Some(p);
    }
    fn mouse_button_down_event(&mut self, b: miniquad::MouseButton, _x: f32, _y: f32) {
        self.out.push(InputEv::Down(b));
    }
    fn mouse_button_up_event(&mut self, b: miniquad::MouseButton, _x: f32, _y: f32) {
        self.out.push(InputEv::Up(b));
    }
}

impl Input {
    pub fn new() -> Input {
        Input {
            id: macroquad::input::utils::register_input_subscriber(),
            last: None,
            events: vec![],
            dedup: Dedup::default(),
        }
    }

    /// Call once per frame; fills `events`.
    pub fn poll(&mut self) {
        self.events.clear();
        let mut c = Collector {
            last: &mut self.last,
            out: &mut self.events,
        };
        macroquad::input::utils::repeat_all_miniquad_input(&mut c, self.id);
        if self.events.iter().any(|e| matches!(e, InputEv::Motion(..))) {
            self.dedup.process(&mut self.events);
        } else {
            self.dedup.flush(&mut self.events);
        }
        if debug_enabled() {
            for e in &self.events {
                eprintln!("[input] {e:?}");
            }
        }
    }

    /// Forget the last absolute position (call when grabbing / releasing the cursor).
    pub fn reset(&mut self) {
        self.last = None;
        self.dedup.reset();
    }
}

fn debug_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("AIMFORGE_DEBUG_INPUT").is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn dedup_pairs() {
        let mut d = Dedup::default();
        // Warm up with duplicated stream: every report appears twice.
        for i in 0..20 {
            let v = (i % 5 + 1) as f32;
            let mut evs = vec![InputEv::Motion(v, 0.0), InputEv::Motion(v, 0.0)];
            d.process(&mut evs);
        }
        assert_eq!(d.factor(), 2);
        let mut evs = vec![
            InputEv::Motion(3.0, 1.0),
            InputEv::Motion(3.0, 1.0),
            InputEv::Motion(3.0, 1.0),
            InputEv::Motion(3.0, 1.0),
            InputEv::Down(MouseButton::Left),
            InputEv::Motion(-2.0, 0.0),
            InputEv::Motion(-2.0, 0.0),
        ];
        d.process(&mut evs);
        d.flush(&mut evs);
        let total: f32 = evs
            .iter()
            .map(|e| {
                if let InputEv::Motion(x, _) = e {
                    *x
                } else {
                    0.0
                }
            })
            .sum();
        assert_eq!(total, 3.0 * 2.0 - 2.0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn dedup_split_across_frames() {
        let mut d = Dedup::default();
        let mut total = 0.0;
        // 40 real reports of +1, each duplicated, split at odd frame boundaries.
        let stream: Vec<InputEv> = (0..40)
            .flat_map(|i| {
                let v = if i % 2 == 0 { 1.0 } else { 2.0 };
                [InputEv::Motion(v, 0.0), InputEv::Motion(v, 0.0)]
            })
            .collect();
        let mut expected = 0.0;
        for (i, e) in stream.iter().enumerate() {
            if i % 2 == 0 {
                if let InputEv::Motion(x, _) = e {
                    expected += x;
                }
            }
        }
        // Warm-up pass so the factor is learned.
        for chunk in stream.chunks(3) {
            let mut evs = chunk.to_vec();
            d.process(&mut evs);
        }
        let mut evs = vec![];
        d.flush(&mut evs);
        // Measured pass.
        for chunk in stream.chunks(3) {
            let mut evs = chunk.to_vec();
            d.process(&mut evs);
            total += evs
                .iter()
                .map(|e| {
                    if let InputEv::Motion(x, _) = e {
                        *x
                    } else {
                        0.0
                    }
                })
                .sum::<f32>();
        }
        let mut evs = vec![];
        d.flush(&mut evs);
        total += evs
            .iter()
            .map(|e| {
                if let InputEv::Motion(x, _) = e {
                    *x
                } else {
                    0.0
                }
            })
            .sum::<f32>();
        assert!(
            (total - expected).abs() < 1e-3,
            "total {total} expected {expected}"
        );
        assert_eq!(d.factor(), 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn no_dedup_when_clean() {
        let mut d = Dedup::default();
        for i in 0..40 {
            let mut evs = vec![
                InputEv::Motion(i as f32, 0.0),
                InputEv::Motion(i as f32 + 0.5, 0.0),
            ];
            d.process(&mut evs);
        }
        assert_eq!(d.factor(), 1);
    }
}
