//! Online leaderboards on Supabase (free tier): a `scores` table plus a public
//! `replays` storage bucket. See `supabase/setup.sql`.
//!
//! All network calls run on background threads and report back over a channel.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// Built-in project. Empty = online leaderboards off unless set in settings.toml
/// (`board_url` / `board_key`). The anon key is public by design; row-level
/// security only allows reading and inserting.
pub const DEFAULT_URL: &str = "";
pub const DEFAULT_KEY: &str = "";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Entry {
    #[serde(default, skip_serializing)]
    pub id: Option<i64>,
    #[serde(default, skip_serializing)]
    pub created_at: Option<String>,
    pub scenario: String,
    pub scenario_name: String,
    pub player: String,
    pub player_id: String,
    pub score: f32,
    pub accuracy: f32,
    pub kills: i32,
    pub hits: i32,
    pub shots: i32,
    pub cm360: f32,
    pub hfov: f32,
    pub duration: f32,
    pub game_version: String,
    #[serde(default)]
    pub replay_path: Option<String>,
}

impl Entry {
    pub fn date(&self) -> String {
        self.created_at.as_deref().map(|d| d.chars().take(10).collect()).unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub url: String,
    pub key: String,
}

pub enum Msg {
    Top { scenario: String, result: Result<Vec<Entry>, String> },
    Submitted { scenario: String, score: f32, result: Result<(usize, usize), String> },
    Replay { label: String, result: Result<Vec<u8>, String> },
}

pub struct Client {
    pub cfg: Option<Config>,
    tx: Sender<Msg>,
    pub rx: Receiver<Msg>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(30))
        .try_proxy_from_env(true)
        .user_agent(&format!("AimForge/{}", crate::update::VERSION))
        .build()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

impl Config {
    fn get(&self, path: &str) -> ureq::Request {
        agent()
            .get(&format!("{}{}", self.url.trim_end_matches('/'), path))
            .set("apikey", &self.key)
            .set("Authorization", &format!("Bearer {}", self.key))
    }
    fn post(&self, path: &str) -> ureq::Request {
        agent()
            .post(&format!("{}{}", self.url.trim_end_matches('/'), path))
            .set("apikey", &self.key)
            .set("Authorization", &format!("Bearer {}", self.key))
    }

    /// Best run per player for a scenario, highest first.
    pub fn top(&self, scenario: &str, limit: usize) -> Result<Vec<Entry>, String> {
        let path = format!("/rest/v1/leaderboard?scenario=eq.{}&order=score.desc&limit={limit}", enc(scenario));
        let body = self.get(&path).call().map_err(net_err)?.into_string().map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| format!("bad leaderboard data: {e}"))
    }

    /// (rank, total players) for a score on a scenario.
    pub fn rank(&self, scenario: &str, score: f32) -> Result<(usize, usize), String> {
        let count = |filter: &str| -> Result<usize, String> {
            let path = format!("/rest/v1/leaderboard?scenario=eq.{}{filter}&select=id", enc(scenario));
            let resp = self.get(&path).set("Prefer", "count=exact").set("Range", "0-0").call().map_err(net_err)?;
            let total = resp
                .header("Content-Range")
                .and_then(|h| h.rsplit('/').next())
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            Ok(total)
        };
        let better = count(&format!("&score=gt.{score}"))?;
        let total = count("")?;
        Ok((better + 1, total.max(better + 1)))
    }

    /// Upload the replay (if any) then insert the score row.
    pub fn submit(&self, mut entry: Entry, replay: Option<Vec<u8>>) -> Result<(), String> {
        if let Some(bytes) = replay {
            let name = format!(
                "{}/{}-{}.afr",
                crate::scenario::safe_file_name(&entry.scenario).replace(' ', "_"),
                entry.player_id,
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            );
            let res = self
                .post(&format!("/storage/v1/object/replays/{}", enc_path(&name)))
                .set("Content-Type", "application/octet-stream")
                .set("x-upsert", "false")
                .send_bytes(&bytes);
            if res.is_ok() {
                entry.replay_path = Some(name);
            }
        }
        let body = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        self.post("/rest/v1/scores")
            .set("Content-Type", "application/json")
            .set("Prefer", "return=minimal")
            .send_string(&body)
            .map_err(net_err)?;
        Ok(())
    }

    pub fn replay(&self, path: &str) -> Result<Vec<u8>, String> {
        let url = format!("/storage/v1/object/public/replays/{}", enc_path(path));
        let resp = self.get(&url).call().map_err(net_err)?;
        let mut data = vec![];
        std::io::Read::read_to_end(&mut std::io::Read::take(resp.into_reader(), 16 * 1024 * 1024), &mut data)
            .map_err(|e| e.to_string())?;
        Ok(data)
    }
}

fn enc_path(p: &str) -> String {
    p.split('/').map(enc).collect::<Vec<_>>().join("/")
}

fn net_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            let msg: String = body.chars().take(160).collect();
            format!("server said {code}: {msg}")
        }
        other => format!("offline ({other})"),
    }
}

impl Client {
    pub fn new(url: &str, key: &str) -> Client {
        let (tx, rx) = channel();
        let (u, k) = if url.trim().is_empty() { (DEFAULT_URL, DEFAULT_KEY) } else { (url.trim(), key.trim()) };
        let cfg = (!u.is_empty() && !k.is_empty()).then(|| Config { url: u.to_string(), key: k.to_string() });
        Client { cfg, tx, rx }
    }

    pub fn online(&self) -> bool {
        self.cfg.is_some()
    }

    pub fn fetch_top(&self, scenario: &str) {
        let Some(cfg) = self.cfg.clone() else { return };
        let tx = self.tx.clone();
        let scenario = scenario.to_string();
        std::thread::spawn(move || {
            let result = cfg.top(&scenario, 100);
            let _ = tx.send(Msg::Top { scenario, result });
        });
    }

    pub fn submit(&self, entry: Entry, replay: Option<Vec<u8>>) {
        let Some(cfg) = self.cfg.clone() else { return };
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let scenario = entry.scenario.clone();
            let score = entry.score;
            let result = cfg.submit(entry, replay).and_then(|_| cfg.rank(&scenario, score));
            let top = cfg.top(&scenario, 100);
            let _ = tx.send(Msg::Submitted { scenario: scenario.clone(), score, result });
            let _ = tx.send(Msg::Top { scenario, result: top });
        });
    }

    pub fn fetch_replay(&self, path: &str, label: &str) {
        let Some(cfg) = self.cfg.clone() else { return };
        let tx = self.tx.clone();
        let (path, label) = (path.to_string(), label.to_string());
        std::thread::spawn(move || {
            let result = cfg.replay(&path);
            let _ = tx.send(Msg::Replay { label, result });
        });
    }
}

/// Random per-install id (not personal data; just groups your runs).
pub fn new_player_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i ^ std::process::id() as u64);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    // uuid-shaped for the database column
    format!("{}-{}-4{}-a{}-{}", &out[0..8], &out[8..12], &out[13..16], &out[17..20], &out[20..32])
}

pub fn clean_name(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(24).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_names() {
        let a = new_player_id();
        let b = new_player_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.matches('-').count(), 4);
        assert_eq!(clean_name("  Matthew\n  "), "Matthew");
        assert_eq!(clean_name(&"x".repeat(40)).len(), 24);
        assert_eq!(enc("1w4ts reload"), "1w4ts%20reload");
    }

    #[test]
    fn entry_json_shape() {
        let e = Entry { scenario: "pasu voltaic".into(), player: "M".into(), ..Default::default() };
        let j = serde_json::to_value(&e).unwrap();
        assert!(j.get("id").is_none() && j.get("created_at").is_none());
        let back: Entry = serde_json::from_str(r#"{"id":3,"created_at":"2026-11-22T10:00:00Z","scenario":"a","scenario_name":"A","player":"p","player_id":"x","score":1.5,"accuracy":0.9,"kills":3,"hits":4,"shots":5,"cm360":30,"hfov":103,"duration":60,"game_version":"0.4.0","replay_path":null}"#).unwrap();
        assert_eq!(back.date(), "2026-11-22");
        assert_eq!(back.id, Some(3));
    }
}
