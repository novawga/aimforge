//! Sound effects. Everything is synthesised at startup (FM bells, filtered
//! noise, pitch sweeps, a touch of reverb), so no audio files ship with the
//! game. Any .wav/.ogg dropped into the `sounds/` folder shows up as an extra
//! hit/kill sound choice.

/// Built-in hit sound names, in menu order.
pub const HIT_SOUNDS: &[&str] = &["Tick", "Ting", "Pop", "Click", "Bubble", "Wood", "Thump", "Off"];
/// Built-in kill sound names, in menu order.
pub const KILL_SOUNDS: &[&str] = &["Ding", "Chime", "Sparkle", "Shatter", "Coin", "Zap", "Kick", "Off"];

// ---------------------------------------------------------------------------
// Synthesis (pure functions, testable without an audio device)
// ---------------------------------------------------------------------------

pub const RATE: u32 = 44100;
const TAU: f32 = std::f32::consts::TAU;

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn buf(secs: f32) -> Vec<f32> {
    vec![0.0; (RATE as f32 * secs) as usize]
}

fn t(i: usize) -> f32 {
    i as f32 / RATE as f32
}

/// Fast attack (avoids clicks), exponential decay.
fn env(time: f32, attack: f32, decay: f32) -> f32 {
    (time / attack.max(1e-4)).min(1.0) * (-time * decay).exp()
}

/// Add an FM bell partial.
fn bell(out: &mut [f32], start: f32, f: f32, ratio: f32, index: f32, decay: f32, amp: f32) {
    let s0 = (start * RATE as f32) as usize;
    for i in s0..out.len() {
        let tt = t(i - s0);
        let e = env(tt, 0.002, decay);
        let idx = index * (-tt * decay * 1.5).exp();
        out[i] += (TAU * f * tt + idx * (TAU * f * ratio * tt).sin()).sin() * e * amp;
    }
}

/// Sine with exponential pitch sweep f0 -> f1 over `sweep` seconds.
fn sweep(out: &mut [f32], start: f32, f0: f32, f1: f32, sweep: f32, decay: f32, amp: f32) {
    let s0 = (start * RATE as f32) as usize;
    let mut ph = 0.0f32;
    for i in s0..out.len() {
        let tt = t(i - s0);
        let k = (tt / sweep).min(1.0);
        let f = f0 * (f1 / f0).powf(k);
        ph += f / RATE as f32 * TAU;
        out[i] += ph.sin() * env(tt, 0.0015, decay) * amp;
    }
}

/// Filtered noise burst. `lp` brightness (0..1), `hp` low-cut amount (0..1).
fn noise_burst(out: &mut [f32], start: f32, decay: f32, lp: f32, hp: f32, amp: f32, seed: u32) {
    let s0 = (start * RATE as f32) as usize;
    let mut n = Noise(seed.wrapping_mul(2654435761) | 1);
    let (mut l, mut prev, mut h) = (0.0f32, 0.0f32, 0.0f32);
    for i in s0..out.len() {
        let tt = t(i - s0);
        let x = n.next();
        l += lp * (x - l);
        h = hp * (h + l - prev);
        prev = l;
        out[i] += h * env(tt, 0.0008, decay) * amp;
    }
}

/// Small Schroeder reverb for sparkle on kill/score sounds.
fn reverb(x: &mut Vec<f32>, mix: f32, tail: f32) {
    let extra = (RATE as f32 * tail) as usize;
    x.extend(std::iter::repeat_n(0.0, extra));
    let mut wet = vec![0.0f32; x.len()];
    for &d in &[1116usize, 1188, 1277, 1356] {
        let mut line = vec![0.0f32; d];
        let mut idx = 0;
        for i in 0..x.len() {
            let y = line[idx];
            line[idx] = x[i] + y * 0.78;
            idx = (idx + 1) % d;
            wet[i] += y * 0.25;
        }
    }
    for &d in &[225usize, 556] {
        let mut line = vec![0.0f32; d];
        let mut idx = 0;
        for w in wet.iter_mut() {
            let b = line[idx];
            let y = -*w + b;
            line[idx] = *w + b * 0.5;
            idx = (idx + 1) % d;
            *w = y;
        }
    }
    for i in 0..x.len() {
        x[i] += wet[i] * mix;
    }
}

fn normalize(x: &mut [f32], peak: f32) {
    let m = x.iter().fold(0.0f32, |a, v| a.max(v.abs())).max(1e-6);
    for v in x.iter_mut() {
        *v = *v / m * peak;
    }
    let len = x.len();
    let n = len.min(512);
    for i in 0..n {
        x[len - 1 - i] *= i as f32 / n as f32;
    }
}

/// Render a named effect to samples. Unknown names (and "Off") give silence.
pub fn synth(name: &str) -> Vec<f32> {
    let mut o;
    match name {
        // ---- hits
        "Tick" => {
            o = buf(0.045);
            sweep(&mut o, 0.0, 3600.0, 2900.0, 0.03, 110.0, 0.7);
            noise_burst(&mut o, 0.0, 260.0, 0.9, 0.7, 0.35, 7);
        }
        "Ting" => {
            o = buf(0.22);
            bell(&mut o, 0.0, 2350.0, 1.41, 2.2, 26.0, 0.8);
            bell(&mut o, 0.0, 4700.0, 1.0, 0.0, 40.0, 0.15);
        }
        "Pop" => {
            o = buf(0.08);
            sweep(&mut o, 0.0, 1100.0, 280.0, 0.04, 55.0, 0.9);
            noise_burst(&mut o, 0.0, 400.0, 0.6, 0.6, 0.25, 3);
        }
        "Click" => {
            o = buf(0.03);
            noise_burst(&mut o, 0.0, 330.0, 0.55, 0.85, 0.8, 11);
            sweep(&mut o, 0.0, 1900.0, 1700.0, 0.02, 220.0, 0.35);
        }
        "Bubble" => {
            o = buf(0.09);
            sweep(&mut o, 0.0, 420.0, 1350.0, 0.06, 38.0, 0.9);
        }
        "Wood" => {
            o = buf(0.08);
            bell(&mut o, 0.0, 760.0, 2.51, 1.2, 70.0, 0.7);
            bell(&mut o, 0.0, 1910.0, 1.0, 0.0, 95.0, 0.3);
            noise_burst(&mut o, 0.0, 500.0, 0.45, 0.5, 0.25, 5);
        }
        "Thump" => {
            o = buf(0.12);
            sweep(&mut o, 0.0, 190.0, 70.0, 0.06, 30.0, 1.0);
            noise_burst(&mut o, 0.0, 200.0, 0.2, 0.3, 0.3, 9);
        }
        // ---- kills
        "Ding" => {
            o = buf(0.55);
            bell(&mut o, 0.0, 1320.0, 3.5, 1.6, 7.0, 0.7);
            bell(&mut o, 0.0, 2640.0, 1.0, 0.0, 11.0, 0.18);
            reverb(&mut o, 0.25, 0.25);
        }
        "Chime" => {
            o = buf(0.6);
            bell(&mut o, 0.0, 1318.5, 2.0, 1.0, 9.0, 0.55);
            bell(&mut o, 0.055, 1975.5, 2.0, 1.0, 8.0, 0.55);
            reverb(&mut o, 0.3, 0.3);
        }
        "Sparkle" => {
            o = buf(0.35);
            sweep(&mut o, 0.0, 900.0, 320.0, 0.05, 40.0, 0.6);
            for (k, f) in [2637.0, 3136.0, 3951.0].iter().enumerate() {
                bell(&mut o, 0.02 + k as f32 * 0.03, *f, 1.0, 0.0, 22.0, 0.22);
            }
            noise_burst(&mut o, 0.0, 30.0, 0.95, 0.95, 0.12, 21);
            reverb(&mut o, 0.3, 0.25);
        }
        "Shatter" => {
            o = buf(0.4);
            noise_burst(&mut o, 0.0, 14.0, 0.95, 0.9, 0.5, 13);
            let mut n = Noise(77);
            for k in 0..9 {
                let st = 0.01 + (n.next() * 0.5 + 0.5) * 0.12;
                let f = 2400.0 + (n.next() * 0.5 + 0.5) * 3800.0;
                bell(&mut o, st, f, 1.0, 0.0, 45.0 + k as f32 * 3.0, 0.16);
            }
            sweep(&mut o, 0.0, 700.0, 140.0, 0.12, 18.0, 0.35);
        }
        "Coin" => {
            o = buf(0.42);
            for (st, f, len) in [(0.0f32, 987.8f32, 0.07f32), (0.07, 1318.5, 0.35)] {
                let s0 = (st * RATE as f32) as usize;
                let e0 = ((st + len) * RATE as f32) as usize;
                for i in s0..e0.min(o.len()) {
                    let tt = t(i - s0);
                    let ph = (f * tt).fract();
                    let sq = if ph < 0.5 { 1.0 } else { -1.0 };
                    let soft = (TAU * f * tt).sin();
                    o[i] += (sq * 0.35 + soft * 0.65) * env(tt, 0.002, 6.0) * 0.5;
                }
            }
        }
        "Zap" => {
            o = buf(0.2);
            sweep(&mut o, 0.0, 2600.0, 220.0, 0.14, 16.0, 0.8);
            sweep(&mut o, 0.0, 2610.0, 215.0, 0.14, 16.0, 0.4);
        }
        "Kick" => {
            o = buf(0.3);
            sweep(&mut o, 0.0, 160.0, 42.0, 0.09, 12.0, 1.0);
            noise_burst(&mut o, 0.0, 300.0, 0.7, 0.6, 0.35, 17);
        }
        // ---- game & ui
        "count" => {
            o = buf(0.12);
            bell(&mut o, 0.0, 880.0, 2.0, 0.6, 30.0, 0.6);
        }
        "go" => {
            o = buf(0.3);
            bell(&mut o, 0.0, 1318.5, 2.0, 0.8, 12.0, 0.45);
            bell(&mut o, 0.0, 1760.0, 2.0, 0.8, 12.0, 0.45);
        }
        "finish" => {
            o = buf(0.9);
            for (k, f) in [1046.5, 1318.5, 1568.0].iter().enumerate() {
                bell(&mut o, k as f32 * 0.09, *f, 2.0, 0.9, 6.0, 0.4);
            }
            reverb(&mut o, 0.3, 0.4);
        }
        "highscore" => {
            o = buf(1.2);
            for (k, f) in [1046.5, 1318.5, 1568.0, 2093.0].iter().enumerate() {
                bell(&mut o, k as f32 * 0.085, *f, 3.0, 1.2, 4.5, 0.38);
            }
            for k in 0..6 {
                bell(&mut o, 0.34 + k as f32 * 0.045, 3136.0 + k as f32 * 260.0, 1.0, 0.0, 14.0, 0.1);
            }
            reverb(&mut o, 0.4, 0.6);
        }
        "ui" => {
            o = buf(0.03);
            sweep(&mut o, 0.0, 1500.0, 1350.0, 0.02, 160.0, 0.6);
            noise_burst(&mut o, 0.0, 400.0, 0.5, 0.7, 0.15, 31);
        }
        _ => return vec![],
    }
    normalize(&mut o, 0.85);
    o
}

pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&RATE.to_le_bytes());
    v.extend_from_slice(&(RATE * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    v
}

// ---------------------------------------------------------------------------
// Playback
// ---------------------------------------------------------------------------

#[cfg(feature = "audio")]
mod imp {
    use super::*;
    use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};
    use std::collections::HashMap;
    use std::path::Path;

    pub struct Sounds {
        map: HashMap<String, Sound>,
        /// Display names of sounds loaded from the sounds/ folder.
        pub custom: Vec<String>,
    }

    impl Sounds {
        pub async fn load(dir: &Path) -> Sounds {
            let mut map = HashMap::new();
            let names = HIT_SOUNDS.iter().chain(KILL_SOUNDS.iter()).copied().chain(["count", "go", "finish", "highscore", "ui"]);
            for n in names {
                let s = synth(n);
                if s.is_empty() {
                    continue;
                }
                if let Ok(snd) = load_sound_from_bytes(&wav(&s)).await {
                    map.insert(n.to_string(), snd);
                }
            }
            let mut custom = vec![];
            let _ = std::fs::create_dir_all(dir);
            let mut files = vec![];
            crate::scenario::collect_files(dir, &["wav", "ogg"], &mut files, 0);
            for f in files {
                let Some(stem) = f.file_stem().map(|s| s.to_string_lossy().to_string()) else { continue };
                if let Ok(bytes) = std::fs::read(&f) {
                    if let Ok(snd) = load_sound_from_bytes(&bytes).await {
                        let name = format!("File: {stem}");
                        map.insert(name.clone(), snd);
                        custom.push(name);
                    }
                }
            }
            Sounds { map, custom }
        }

        pub fn play(&self, name: &str, vol: f32) {
            if let Some(s) = self.map.get(name) {
                play_sound(s, PlaySoundParams { looped: false, volume: vol.clamp(0.0, 1.0) });
            }
        }
    }
}

#[cfg(not(feature = "audio"))]
mod imp {
    use std::path::Path;
    pub struct Sounds {
        pub custom: Vec<String>,
    }
    impl Sounds {
        pub async fn load(_dir: &Path) -> Sounds {
            Sounds { custom: vec![] }
        }
        pub fn play(&self, _name: &str, _vol: f32) {}
    }
}

pub use imp::Sounds;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_renders_clean() {
        for n in HIT_SOUNDS.iter().chain(KILL_SOUNDS).chain(&["count", "go", "finish", "highscore", "ui"]) {
            let s = synth(n);
            if *n == "Off" {
                assert!(s.is_empty());
                continue;
            }
            assert!(s.len() > 500, "{n} too short");
            assert!(s.iter().all(|v| v.is_finite() && v.abs() <= 0.9), "{n} clipped or NaN");
            assert!(s.iter().any(|v| v.abs() > 0.5), "{n} too quiet");
            assert!(s.last().unwrap().abs() < 0.01, "{n} ends with a click");
            assert_eq!(&wav(&s)[..4], b"RIFF");
        }
    }
}

#[cfg(test)]
mod export {
    /// `SOUND_OUT=dir cargo test --no-default-features export_wavs -- --ignored`
    #[test]
    #[ignore]
    fn export_wavs() {
        let dir = std::path::PathBuf::from(std::env::var("SOUND_OUT").unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        for (kind, list) in [("hit", super::HIT_SOUNDS), ("kill", super::KILL_SOUNDS), ("game", &["count", "go", "finish", "highscore", "ui"][..])] {
            for n in list {
                let s = super::synth(n);
                if !s.is_empty() {
                    std::fs::write(dir.join(format!("{kind} - {n}.wav")), super::wav(&s)).unwrap();
                }
            }
        }
    }
}
