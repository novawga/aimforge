//! Run history, high scores and KovaaK's-style per-run CSV files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RunRecord {
    pub score: f32,
    pub accuracy: f32,
    pub kills: u32,
    pub shots: u32,
    pub hits: u32,
    pub damage_done: f32,
    pub damage_possible: f32,
    pub avg_ttk: f32,
    pub duration: f32,
    pub date: String,
    pub cm360: f32,
    pub hfov: f32,
    /// Replay file, relative to stats/replays.
    #[serde(default)]
    pub replay: Option<String>,
}

#[derive(Clone, Debug)]
pub struct KillEvent {
    pub time: f32,
    pub ttk: f32,
    pub shots: u32,
    pub hits: u32,
    pub damage: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct History {
    pub runs: HashMap<String, Vec<RunRecord>>,
    /// Display names keyed by lowercase name.
    #[serde(default)]
    pub names: HashMap<String, String>,
    #[serde(skip)]
    path: PathBuf,
}

impl History {
    pub fn load(dir: &Path) -> History {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join("history.json");
        let mut h: History = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        h.path = path;
        h
    }

    pub fn save(&self) {
        if let Ok(t) = serde_json::to_string(self) {
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn runs(&self, name: &str) -> &[RunRecord] {
        self.runs
            .get(&crate::scenario::key(name))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn best(&self, name: &str) -> Option<f32> {
        self.runs(name)
            .iter()
            .map(|r| r.score)
            .fold(None, |a, s| Some(a.map_or(s, |a: f32| a.max(s))))
    }

    pub fn plays(&self, name: &str) -> usize {
        self.runs(name).len()
    }

    pub fn last_played(&self, name: &str) -> Option<&str> {
        self.runs(name).last().map(|r| r.date.as_str())
    }

    pub fn add(&mut self, name: &str, rec: RunRecord) {
        let k = crate::scenario::key(name);
        self.names.insert(k.clone(), name.to_string());
        self.runs.entry(k).or_default().push(rec);
        self.save();
    }

    /// Keep replays for the 10 best and 3 most recent runs of a scenario; delete the rest.
    pub fn prune_replays(&mut self, name: &str, replay_dir: &Path) {
        let Some(runs) = self.runs.get_mut(&crate::scenario::key(name)) else { return };
        let mut by_score: Vec<usize> = (0..runs.len()).filter(|&i| runs[i].replay.is_some()).collect();
        by_score.sort_by(|&a, &b| runs[b].score.partial_cmp(&runs[a].score).unwrap_or(std::cmp::Ordering::Equal));
        let mut keep: std::collections::HashSet<usize> = by_score.iter().take(10).copied().collect();
        keep.extend(by_score.iter().copied().filter(|&i| i + 3 >= runs.len()));
        for i in by_score {
            if !keep.contains(&i) {
                if let Some(f) = runs[i].replay.take() {
                    let _ = std::fs::remove_file(replay_dir.join(f));
                }
            }
        }
        self.save();
    }

    pub fn played_names(&self) -> Vec<String> {
        self.runs
            .keys()
            .map(|k| self.names.get(k).cloned().unwrap_or_else(|| k.clone()))
            .collect()
    }
}

pub fn now_string() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Write `<Scenario> - Challenge - YYYY.MM.DD-HH.MM.SS Stats.csv` like KovaaK's does.
pub fn write_csv(
    dir: &Path,
    scenario: &str,
    rec: &RunRecord,
    kills: &[KillEvent],
    sens_label: &str,
) -> Option<PathBuf> {
    let runs = dir.join("runs");
    std::fs::create_dir_all(&runs).ok()?;
    let stamp = chrono::Local::now().format("%Y.%m.%d-%H.%M.%S").to_string();
    let file = runs.join(format!(
        "{} - Challenge - {} Stats.csv",
        crate::scenario::safe_file_name(scenario),
        stamp
    ));
    let mut out = String::new();
    out.push_str("Kill #,Timestamp,TTK,Shots,Hits,Accuracy,Damage Done\n");
    for (i, k) in kills.iter().enumerate() {
        let acc = if k.shots > 0 {
            k.hits as f32 / k.shots as f32
        } else {
            0.0
        };
        out.push_str(&format!(
            "{},{:.3},{:.3}s,{},{},{:.6},{:.1}\n",
            i + 1,
            k.time,
            k.ttk,
            k.shots,
            k.hits,
            acc,
            k.damage
        ));
    }
    out.push('\n');
    out.push_str(&format!("Scenario:,{}\n", scenario));
    out.push_str(&format!("Score:,{:.1}\n", rec.score));
    out.push_str(&format!("Kills:,{}\n", rec.kills));
    out.push_str(&format!("Hit Count:,{}\n", rec.hits));
    out.push_str(&format!(
        "Miss Count:,{}\n",
        rec.shots.saturating_sub(rec.hits)
    ));
    out.push_str(&format!("Accuracy:,{:.4}\n", rec.accuracy));
    out.push_str(&format!("Damage Done:,{:.1}\n", rec.damage_done));
    out.push_str(&format!("Damage Possible:,{:.1}\n", rec.damage_possible));
    out.push_str(&format!("Avg TTK:,{:.3}\n", rec.avg_ttk));
    out.push_str(&format!("Challenge Start:,{}\n", rec.date));
    out.push_str(&format!("Sens:,{}\n", sens_label));
    out.push_str(&format!("cm/360:,{:.2}\n", rec.cm360));
    out.push_str(&format!("Horiz FOV:,{:.1}\n", rec.hfov));
    std::fs::write(&file, out).ok()?;
    Some(file)
}
