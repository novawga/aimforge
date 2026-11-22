//! Hit / kill sounds, synthesised at startup (no asset files needed).

#[cfg(feature = "audio")]
mod imp {
    use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

    pub struct Sounds {
        hit: Option<Sound>,
        kill: Option<Sound>,
        head: Option<Sound>,
        tick: Option<Sound>,
    }

    fn wav(samples: &[f32], rate: u32) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut v = Vec::with_capacity(44 + data_len as usize);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
        }
        v
    }

    fn tone(rate: u32, len: f32, f0: f32, f1: f32, decay: f32, noise: f32) -> Vec<f32> {
        let n = (rate as f32 * len) as usize;
        let mut ph = 0.0f32;
        let mut seed = 0x1234_5678u32;
        (0..n)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let f = f0 + (f1 - f0) * (t / len);
                ph += f / rate as f32 * std::f32::consts::TAU;
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let nz = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                let env = (-t * decay).exp() * (t * 2000.0).min(1.0);
                (ph.sin() * (1.0 - noise) + nz * noise) * env * 0.8
            })
            .collect()
    }

    impl Sounds {
        pub async fn load() -> Sounds {
            let r = 44100;
            let hit = load_sound_from_bytes(&wav(&tone(r, 0.05, 1900.0, 1500.0, 70.0, 0.15), r))
                .await
                .ok();
            let mut k = tone(r, 0.16, 740.0, 1480.0, 22.0, 0.05);
            for (i, s) in tone(r, 0.16, 1110.0, 2220.0, 22.0, 0.0).iter().enumerate() {
                k[i] = k[i] * 0.6 + s * 0.4;
            }
            let kill = load_sound_from_bytes(&wav(&k, r)).await.ok();
            let head = load_sound_from_bytes(&wav(&tone(r, 0.09, 2600.0, 3200.0, 45.0, 0.1), r))
                .await
                .ok();
            let tick = load_sound_from_bytes(&wav(&tone(r, 0.07, 880.0, 880.0, 40.0, 0.0), r))
                .await
                .ok();
            Sounds {
                hit,
                kill,
                head,
                tick,
            }
        }

        fn play(s: &Option<Sound>, vol: f32) {
            if let Some(s) = s {
                play_sound(
                    s,
                    PlaySoundParams {
                        looped: false,
                        volume: vol.clamp(0.0, 1.0),
                    },
                );
            }
        }
        pub fn hit(&self, vol: f32) {
            Self::play(&self.hit, vol * 0.6);
        }
        pub fn head(&self, vol: f32) {
            Self::play(&self.head, vol * 0.7);
        }
        pub fn kill(&self, vol: f32) {
            Self::play(&self.kill, vol);
        }
        pub fn tick(&self, vol: f32) {
            Self::play(&self.tick, vol * 0.5);
        }
    }
}

#[cfg(not(feature = "audio"))]
mod imp {
    pub struct Sounds;
    impl Sounds {
        pub async fn load() -> Sounds {
            Sounds
        }
        pub fn hit(&self, _: f32) {}
        pub fn head(&self, _: f32) {}
        pub fn kill(&self, _: f32) {}
        pub fn tick(&self, _: f32) {}
    }
}

pub use imp::Sounds;
