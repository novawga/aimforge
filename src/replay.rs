//! Run replays: 60 Hz snapshots of the camera, targets and score plus shot
//! events, stored as a small compressed binary file (`.afr`).
//!
//! Replays store what happened rather than the inputs, so they play back the
//! same on every machine (no reliance on bit-exact float maths).

use crate::game::{Game, GameEvent, Target};
use crate::scenario::Scenario;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use macroquad::math::{vec3, Vec3};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"AFR1";
pub const SAMPLE_DT: f32 = 1.0 / 60.0;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Header {
    pub player: String,
    pub scenario: String,
    pub score: f32,
    pub accuracy: f32,
    pub kills: u32,
    pub hits: u32,
    pub shots: u32,
    pub duration: f32,
    pub date: String,
    pub cm360: f32,
    pub hfov: f32,
    pub game_version: String,
    /// Scenario definition as TOML, so the replay renders even if the file changes.
    pub scenario_toml: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetSnap {
    pub id: u32,
    pub pos: Vec3,
    pub radius: f32,
    /// 0..1
    pub health: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub t: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub pos: Vec3,
    pub trigger: bool,
    pub score: f32,
    pub hits: u16,
    pub shots: u16,
    pub kills: u16,
    pub targets: Vec<TargetSnap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Miss,
    Hit,
    Kill,
    Expire,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub t: f32,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Replay {
    pub header: Header,
    pub frames: Vec<Frame>,
    pub events: Vec<Event>,
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Recorder {
    frames: Vec<Frame>,
    events: Vec<Event>,
    /// Time of the next sample.
    last: Option<f32>,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder::default()
    }

    /// Call every frame after the game updated.
    pub fn sample(&mut self, g: &Game) {
        if g.phase == crate::game::Phase::Countdown {
            return;
        }
        // Fixed 60 Hz schedule (independent of frame rate).
        if let Some(next) = self.last {
            if g.time < next && g.phase != crate::game::Phase::Finished {
                return;
            }
        }
        let mut next = self.last.unwrap_or(0.0) + SAMPLE_DT;
        if next <= g.time {
            next = (g.time / SAMPLE_DT).floor() * SAMPLE_DT + SAMPLE_DT;
        }
        self.last = Some(next);
        self.frames.push(Frame {
            t: g.time,
            yaw: g.player.yaw as f32,
            pitch: g.player.pitch as f32,
            pos: g.player.pos,
            trigger: g.trigger,
            score: g.score,
            hits: g.hits.min(u16::MAX as u32) as u16,
            shots: g.shots.min(u16::MAX as u32) as u16,
            kills: g.kills.min(u16::MAX as u32) as u16,
            targets: g
                .targets
                .iter()
                .map(|t| TargetSnap {
                    id: t.id as u32,
                    pos: t.pos,
                    radius: t.radius,
                    health: if t.max_health > 0.0 { (t.health / t.max_health).clamp(0.0, 1.0) } else { 1.0 },
                })
                .collect(),
        });
    }

    pub fn event(&mut self, t: f32, e: &GameEvent) {
        let kind = match e {
            GameEvent::Miss => EventKind::Miss,
            GameEvent::Hit { kill: true, .. } => EventKind::Kill,
            GameEvent::Hit { .. } => EventKind::Hit,
            GameEvent::Expire => EventKind::Expire,
        };
        self.events.push(Event { t, kind });
    }

    pub fn finish(self, header: Header) -> Replay {
        Replay { header, frames: self.frames, events: self.events }
    }
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

fn cm(v: f32) -> i16 {
    (v * 100.0).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

impl Replay {
    pub fn encode(&self) -> Vec<u8> {
        let mut raw = Vec::with_capacity(64 * 1024);
        let header = serde_json::to_vec(&self.header).unwrap_or_default();
        raw.extend_from_slice(&(header.len() as u32).to_le_bytes());
        raw.extend_from_slice(&header);
        raw.extend_from_slice(&(self.frames.len() as u32).to_le_bytes());
        for f in &self.frames {
            raw.extend_from_slice(&f.t.to_le_bytes());
            raw.extend_from_slice(&f.yaw.to_le_bytes());
            raw.extend_from_slice(&f.pitch.to_le_bytes());
            for v in [f.pos.x, f.pos.y, f.pos.z] {
                raw.extend_from_slice(&cm(v).to_le_bytes());
            }
            raw.push(f.trigger as u8);
            raw.extend_from_slice(&f.score.to_le_bytes());
            raw.extend_from_slice(&f.hits.to_le_bytes());
            raw.extend_from_slice(&f.shots.to_le_bytes());
            raw.extend_from_slice(&f.kills.to_le_bytes());
            raw.push(f.targets.len().min(255) as u8);
            for t in f.targets.iter().take(255) {
                raw.extend_from_slice(&t.id.to_le_bytes());
                for v in [t.pos.x, t.pos.y, t.pos.z] {
                    raw.extend_from_slice(&cm(v).to_le_bytes());
                }
                raw.extend_from_slice(&((t.radius * 1000.0).round().clamp(0.0, 65535.0) as u16).to_le_bytes());
                raw.push((t.health * 255.0).round() as u8);
            }
        }
        raw.extend_from_slice(&(self.events.len() as u32).to_le_bytes());
        for e in &self.events {
            raw.extend_from_slice(&e.t.to_le_bytes());
            raw.push(match e.kind {
                EventKind::Miss => 0,
                EventKind::Hit => 1,
                EventKind::Kill => 2,
                EventKind::Expire => 3,
            });
        }
        let mut out = MAGIC.to_vec();
        let mut enc = DeflateEncoder::new(&mut out, Compression::default());
        let _ = enc.write_all(&raw);
        let _ = enc.finish();
        out
    }

    pub fn decode(data: &[u8]) -> Result<Replay, String> {
        if data.len() < 4 || &data[..4] != MAGIC {
            return Err("not an AimForge replay".into());
        }
        let mut raw = vec![];
        DeflateDecoder::new(&data[4..])
            .take(64 * 1024 * 1024)
            .read_to_end(&mut raw)
            .map_err(|e| format!("corrupt replay: {e}"))?;
        let mut r = Reader { b: &raw, i: 0 };
        let hl = r.u32()? as usize;
        let header: Header = serde_json::from_slice(r.bytes(hl)?).map_err(|e| format!("bad replay header: {e}"))?;
        let n = r.u32()? as usize;
        if n > 1_000_000 {
            return Err("replay too long".into());
        }
        let mut frames = Vec::with_capacity(n);
        for _ in 0..n {
            let t = r.f32()?;
            let yaw = r.f32()?;
            let pitch = r.f32()?;
            let pos = vec3(r.cm()?, r.cm()?, r.cm()?);
            let trigger = r.u8()? != 0;
            let score = r.f32()?;
            let hits = r.u16()?;
            let shots = r.u16()?;
            let kills = r.u16()?;
            let nt = r.u8()? as usize;
            let mut targets = Vec::with_capacity(nt);
            for _ in 0..nt {
                let id = r.u32()?;
                let pos = vec3(r.cm()?, r.cm()?, r.cm()?);
                let radius = r.u16()? as f32 / 1000.0;
                let health = r.u8()? as f32 / 255.0;
                targets.push(TargetSnap { id, pos, radius, health });
            }
            frames.push(Frame { t, yaw, pitch, pos, trigger, score, hits, shots, kills, targets });
        }
        let ne = r.u32()? as usize;
        let mut events = Vec::with_capacity(ne.min(1_000_000));
        for _ in 0..ne {
            let t = r.f32()?;
            let kind = match r.u8()? {
                0 => EventKind::Miss,
                1 => EventKind::Hit,
                2 => EventKind::Kill,
                _ => EventKind::Expire,
            };
            events.push(Event { t, kind });
        }
        Ok(Replay { header, frames, events })
    }

    pub fn scenario(&self) -> Scenario {
        let mut s: Scenario = toml::from_str(&self.header.scenario_toml).unwrap_or_default();
        if s.name == "Untitled" || s.name.is_empty() {
            s.name = self.header.scenario.clone();
        }
        crate::scenario::sanitize(&mut s);
        s
    }

    pub fn duration(&self) -> f32 {
        self.frames.last().map(|f| f.t).unwrap_or(0.0)
    }

    /// Interpolated state at time `t`.
    pub fn at(&self, t: f32) -> Option<Frame> {
        let f = &self.frames;
        if f.is_empty() {
            return None;
        }
        let i = f.partition_point(|x| x.t <= t);
        if i == 0 {
            return Some(f[0].clone());
        }
        if i >= f.len() {
            return Some(f[f.len() - 1].clone());
        }
        let (a, b) = (&f[i - 1], &f[i]);
        let k = ((t - a.t) / (b.t - a.t).max(1e-5)).clamp(0.0, 1.0);
        let mut dyaw = b.yaw - a.yaw;
        if dyaw > 180.0 {
            dyaw -= 360.0;
        } else if dyaw < -180.0 {
            dyaw += 360.0;
        }
        let targets = a
            .targets
            .iter()
            .map(|ta| match b.targets.iter().find(|tb| tb.id == ta.id) {
                Some(tb) => TargetSnap {
                    id: ta.id,
                    pos: ta.pos.lerp(tb.pos, k),
                    radius: ta.radius + (tb.radius - ta.radius) * k,
                    health: ta.health,
                },
                None => ta.clone(),
            })
            .collect();
        Some(Frame {
            t,
            yaw: (a.yaw + dyaw * k).rem_euclid(360.0),
            pitch: a.pitch + (b.pitch - a.pitch) * k,
            pos: a.pos.lerp(b.pos, k),
            trigger: a.trigger,
            score: a.score,
            hits: a.hits,
            shots: a.shots,
            kills: a.kills,
            targets,
        })
    }

    pub fn to_targets(frame: &Frame) -> Vec<Target> {
        frame
            .targets
            .iter()
            .map(|s| {
                let mut t = Target::preview(s.id as u64, s.pos, s.radius);
                t.health = s.health;
                t.max_health = 1.0;
                t
            })
            .collect()
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.i..self.i + n).ok_or("replay is truncated")?;
        self.i += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn cm(&mut self) -> Result<f32, String> {
        Ok(i16::from_le_bytes(self.bytes(2)?.try_into().unwrap()) as f32 / 100.0)
    }
}

// ---------------------------------------------------------------------------
// Local storage
// ---------------------------------------------------------------------------

pub fn dir(stats_dir: &Path) -> PathBuf {
    stats_dir.join("replays")
}

/// Save and return the file name (relative to the replays folder).
pub fn save(stats_dir: &Path, replay: &Replay, bytes: &[u8]) -> Option<String> {
    let d = dir(stats_dir).join(crate::scenario::safe_file_name(&replay.header.scenario));
    std::fs::create_dir_all(&d).ok()?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let file = d.join(format!("{stamp}_{:.0}.afr", replay.header.score));
    std::fs::write(&file, bytes).ok()?;
    file.strip_prefix(dir(stats_dir)).ok().map(|p| p.to_string_lossy().replace('\\', "/"))
}

pub fn load(stats_dir: &Path, rel: &str) -> Result<Replay, String> {
    let bytes = std::fs::read(dir(stats_dir).join(rel)).map_err(|e| format!("replay file missing: {e}"))?;
    Replay::decode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::MoveKeys;

    fn record(name: &str) -> Replay {
        let mut s = crate::synth::synthesize(name);
        crate::scenario::sanitize(&mut s);
        s.duration = 5.0;
        let mut g = Game::new(s.clone(), 0.0, true, 5);
        let mut rec = Recorder::new();
        g.trigger_down();
        while g.phase != crate::game::Phase::Finished {
            g.look(2.0, -0.5, 0.022, false);
            g.update(1.0 / 144.0, MoveKeys::default());
            for e in g.events.drain(..).collect::<Vec<_>>() {
                rec.event(g.time, &e);
            }
            rec.sample(&g);
        }
        rec.finish(Header {
            player: "tester".into(),
            scenario: s.name.clone(),
            score: g.score,
            scenario_toml: toml::to_string(&s).unwrap(),
            ..Default::default()
        })
    }

    #[test]
    fn roundtrip_and_size() {
        let r = record("B180T Voltaic");
        assert!(r.frames.len() >= 290 && r.frames.len() <= 310, "{} frames for 5 s", r.frames.len());
        let bytes = r.encode();
        let back = Replay::decode(&bytes).unwrap();
        assert_eq!(back.header, r.header);
        assert_eq!(back.frames.len(), r.frames.len());
        assert_eq!(back.events.len(), r.events.len());
        let (a, b) = (&r.frames[100], &back.frames[100]);
        assert_eq!(a.yaw, b.yaw);
        assert!((a.targets[0].pos - b.targets[0].pos).length() < 0.01);
        // ~60 s run should stay well under 1 MB
        assert!(bytes.len() * 12 < 1_000_000, "{} bytes for 5 s", bytes.len());
        assert_eq!(back.scenario().name, "B180T Voltaic");
    }

    #[test]
    fn interpolation() {
        let r = record("Pasu Voltaic");
        let f = r.at(r.frames[10].t + SAMPLE_DT * 0.5).unwrap();
        assert!(f.yaw.is_finite() && !f.targets.is_empty());
        assert!(r.at(-1.0).is_some() && r.at(1e6).is_some());
        assert!(Replay::decode(b"nope").is_err());
    }
}
