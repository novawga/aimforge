//! Playlist import: KovaaK's JSON playlists and legacy `.plo` (UE4 GVAS save) files.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub count: u32,
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub name: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    /// Folder relative to the playlists root, for grouping in the menu.
    pub group: String,
    pub entries: Vec<Entry>,
}

impl Playlist {
    pub fn total_runs(&self) -> u32 {
        self.entries.iter().map(|e| e.count).sum()
    }
}

#[derive(Deserialize, Serialize)]
struct JsonPlaylist {
    #[serde(alias = "playlist_name", alias = "PlaylistName", default)]
    #[serde(rename = "playlistName")]
    playlist_name: String,
    #[serde(alias = "scenario_list", alias = "ScenarioList", default)]
    #[serde(rename = "scenarioList")]
    scenario_list: Vec<JsonEntry>,
}

#[derive(Deserialize, Serialize)]
struct JsonEntry {
    #[serde(alias = "scenario_name", alias = "ScenarioName", default)]
    #[serde(rename = "scenarioName")]
    scenario_name: String,
    #[serde(
        alias = "play_Count",
        alias = "play_count",
        alias = "PlayCount",
        default = "one"
    )]
    #[serde(rename = "playCount")]
    play_count: i64,
}

fn one() -> i64 {
    1
}

pub fn parse_json(text: &str) -> Result<(String, Vec<Entry>), String> {
    let text = text.trim_start_matches('\u{feff}');
    let p: JsonPlaylist = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let entries = p
        .scenario_list
        .into_iter()
        .filter(|e| !e.scenario_name.trim().is_empty())
        .map(|e| Entry {
            name: e.scenario_name.trim().to_string(),
            count: e.play_count.clamp(1, 99) as u32,
        })
        .collect();
    Ok((p.playlist_name, entries))
}

/// Minimal reader for the UE4 GVAS property stream KovaaK's used for `.plo` files.
/// It looks for `PlaylistName`, `ScenarioName` and `PlayCount` properties.
pub fn parse_gvas(data: &[u8]) -> Result<(String, Vec<Entry>), String> {
    if data.len() < 4 || &data[..4] != b"GVAS" {
        return Err("not a GVAS file".into());
    }
    let mut name = String::new();
    let mut entries: Vec<Entry> = vec![];
    let mut i = 0;
    while i + 4 < data.len() {
        // FString: i32 length (incl. nul), bytes.
        let Some((s, next)) = read_fstring(data, i) else {
            i += 1;
            continue;
        };
        match s.as_str() {
            "PlaylistName" | "ScenarioName" => {
                if let Some((ty, after_ty)) = read_fstring(data, next) {
                    if ty == "StrProperty" && after_ty + 9 <= data.len() {
                        // i64 size + u8 terminator, then FString value.
                        if let Some((val, after)) = read_fstring(data, after_ty + 9) {
                            if s == "PlaylistName" {
                                name = val;
                            } else {
                                entries.push(Entry {
                                    name: val,
                                    count: 1,
                                });
                            }
                            i = after;
                            continue;
                        }
                    }
                }
            }
            "PlayCount" => {
                if let Some((ty, after_ty)) = read_fstring(data, next) {
                    if ty == "IntProperty" && after_ty + 13 <= data.len() {
                        let v = i32::from_le_bytes(
                            data[after_ty + 9..after_ty + 13].try_into().unwrap(),
                        );
                        if let Some(e) = entries.last_mut() {
                            e.count = v.clamp(1, 99) as u32;
                        }
                        i = after_ty + 13;
                        continue;
                    }
                }
            }
            _ => {}
        }
        i = next;
    }
    if entries.is_empty() {
        return Err("no scenarios found".into());
    }
    Ok((name, entries))
}

fn read_fstring(data: &[u8], at: usize) -> Option<(String, usize)> {
    let len_bytes = data.get(at..at + 4)?;
    let len = i32::from_le_bytes(len_bytes.try_into().ok()?);
    if len > 0 {
        let len = len as usize;
        if len > 512 {
            return None;
        }
        let body = data.get(at + 4..at + 4 + len)?;
        if *body.last()? != 0 {
            return None;
        }
        let text = &body[..len - 1];
        if text.iter().any(|&b| b < 0x20 && b != b'\t') {
            return None;
        }
        Some((String::from_utf8_lossy(text).to_string(), at + 4 + len))
    } else if len < 0 {
        // UTF-16LE.
        let chars = (-len) as usize;
        if chars > 512 {
            return None;
        }
        let body = data.get(at + 4..at + 4 + chars * 2)?;
        let units: Vec<u16> = body
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if *units.last()? != 0 {
            return None;
        }
        Some((
            String::from_utf16_lossy(&units[..chars - 1]),
            at + 4 + chars * 2,
        ))
    } else {
        None
    }
}

pub fn load_dir(root: &Path) -> (Vec<Playlist>, Vec<String>) {
    let _ = std::fs::create_dir_all(root);
    let mut files = vec![];
    crate::scenario::collect_files(root, &["json", "plo"], &mut files, 0);
    let mut out = vec![];
    let mut errors = vec![];
    for path in files {
        let parsed = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("plo"))
        {
            std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|d| parse_gvas(&d))
        } else {
            std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| parse_json(&t))
        };
        match parsed {
            Ok((name, entries)) if !entries.is_empty() => {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let group = path
                    .parent()
                    .and_then(|p| p.strip_prefix(root).ok())
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                out.push(Playlist {
                    name: if name.trim().is_empty() {
                        stem
                    } else {
                        name.trim().to_string()
                    },
                    path,
                    group,
                    entries,
                });
            }
            Ok(_) => {}
            Err(e) => errors.push(format!("{}: {}", path.display(), e)),
        }
    }
    out.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    (out, errors)
}

/// Save in KovaaK's JSON format (so playlists made here also work there).
pub fn save_json(path: &Path, name: &str, entries: &[Entry]) -> std::io::Result<()> {
    let p = JsonPlaylist {
        playlist_name: name.to_string(),
        scenario_list: entries
            .iter()
            .map(|e| JsonEntry {
                scenario_name: e.name.clone(),
                play_count: e.count as i64,
            })
            .collect(),
    };
    let mut v = serde_json::to_value(&p).map_err(|e| std::io::Error::other(e.to_string()))?;
    v["isFavorite"] = serde_json::Value::Bool(false);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let t = r#"{"playlistName":"X","scenarioList":[{"scenarioName":"1w4ts reload","playCount":3}],"isFavorite":false}"#;
        let (n, e) = parse_json(t).unwrap();
        assert_eq!(n, "X");
        assert_eq!(
            e,
            vec![Entry {
                name: "1w4ts reload".into(),
                count: 3
            }]
        );
    }

    #[test]
    fn gvas_synthetic() {
        fn fs(v: &mut Vec<u8>, s: &str) {
            v.extend_from_slice(&((s.len() + 1) as i32).to_le_bytes());
            v.extend_from_slice(s.as_bytes());
            v.push(0);
        }
        let mut d = b"GVAS\x02\x00\x00\x00".to_vec();
        fs(&mut d, "PlaylistName");
        fs(&mut d, "StrProperty");
        d.extend_from_slice(&[0u8; 9]);
        fs(&mut d, "My List");
        fs(&mut d, "ScenarioName");
        fs(&mut d, "StrProperty");
        d.extend_from_slice(&[0u8; 9]);
        fs(&mut d, "Pasu Voltaic");
        fs(&mut d, "PlayCount");
        fs(&mut d, "IntProperty");
        d.extend_from_slice(&[4, 0, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&5i32.to_le_bytes());
        let (n, e) = parse_gvas(&d).unwrap();
        assert_eq!(n, "My List");
        assert_eq!(
            e,
            vec![Entry {
                name: "Pasu Voltaic".into(),
                count: 5
            }]
        );
    }
}
