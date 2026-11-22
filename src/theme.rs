//! Colour themes for the UI and the 3D world.
//!
//! A theme file is TOML with a name and a `[colors]` table of hex colours.
//! Any colour left out falls back to the default theme, so a theme can be as
//! small as `name = "Mine"` plus `accent = "#ff4655"`.

use macroquad::prelude::Color;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

macro_rules! theme_fields {
    ($($field:ident = $hex:literal : $doc:literal),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct Theme {
            $(#[doc = $doc] pub $field: Color,)*
        }

        impl Default for Theme {
            fn default() -> Self {
                Theme { $($field: hex($hex).unwrap(),)* }
            }
        }

        impl Theme {
            pub const KEYS: &'static [(&'static str, &'static str)] = &[$((stringify!($field), $doc)),*];

            fn get_mut(&mut self, key: &str) -> Option<&mut Color> {
                match key {
                    $(stringify!($field) => Some(&mut self.$field),)*
                    _ => None,
                }
            }

            fn get(&self, key: &str) -> Option<Color> {
                match key {
                    $(stringify!($field) => Some(self.$field),)*
                    _ => None,
                }
            }
        }
    };
}

theme_fields! {
    bg            = "#0d0f12" : "app background",
    surface       = "#15181d" : "cards and panels",
    surface_hover = "#1c2027" : "hovered rows and buttons",
    surface_active= "#232832" : "selected rows, inputs",
    border        = "#262b34" : "hairlines and input borders",
    text          = "#e8eaed" : "primary text",
    text_dim      = "#9aa0aa" : "secondary text",
    text_faint    = "#5f6672" : "captions, placeholders",
    accent        = "#4f8cff" : "primary buttons, focus, selection",
    accent_text   = "#ffffff" : "text on accent",
    good          = "#3ecf8e" : "new high score, positive deltas",
    bad           = "#f2555a" : "warnings, low time",
    hud_bg        = "#0b0d10b8" : "in-game HUD panels (with alpha)",
    world_bg      = "#1e2026" : "clear colour behind the room",
    wall          = "#3a3e46" : "wall colour (bottom)",
    wall_top      = "#4e535c" : "wall colour (top)",
    floor         = "#2c2f36" : "floor near",
    floor_far     = "#34383f" : "floor far",
    ceiling       = "#464a52" : "ceiling",
    grid          = "#5f6670" : "wall grid lines",
    floor_grid    = "#464b54" : "floor grid lines",
    target        = "#ea3434" : "default target colour",
}

pub fn hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    let v = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
    match s.len() {
        6 => Some(Color::from_rgba(v(0)?, v(2)?, v(4)?, 255)),
        8 => Some(Color::from_rgba(v(0)?, v(2)?, v(4)?, v(6)?)),
        3 => {
            let d = |i: usize| u8::from_str_radix(s.get(i..i + 1)?, 16).ok().map(|x| x * 17);
            Some(Color::from_rgba(d(0)?, d(1)?, d(2)?, 255))
        }
        _ => None,
    }
}

pub fn to_hex(c: Color) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    if b(c.a) == 255 {
        format!("#{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b))
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b), b(c.a))
    }
}

#[derive(Clone, Debug)]
pub struct NamedTheme {
    pub name: String,
    pub theme: Theme,
    /// None for built-ins.
    pub path: Option<PathBuf>,
}

impl NamedTheme {
    pub fn to_toml(&self) -> String {
        let mut out = format!("# AimForge theme. Any colour left out uses the default.\nname = {:?}\n\n[colors]\n", self.name);
        for (k, doc) in Theme::KEYS {
            let v = self.theme.get(k).map(to_hex).unwrap_or_default();
            out.push_str(&format!("{k:<15}= \"{v}\"   # {doc}\n"));
        }
        out
    }

    pub fn from_toml(text: &str) -> Result<NamedTheme, String> {
        let v: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
        let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("Imported").trim().to_string();
        let table = v.get("colors").and_then(|c| c.as_table()).or_else(|| v.as_table()).ok_or("no colours")?;
        let mut theme = Theme::default();
        let mut found = 0;
        for (k, val) in table {
            if let (Some(slot), Some(s)) = (theme.get_mut(k), val.as_str()) {
                *slot = hex(s).ok_or_else(|| format!("bad colour for {k}: {s}"))?;
                found += 1;
            }
        }
        if found == 0 {
            return Err("no recognised colours (expected keys like accent = \"#4f8cff\")".into());
        }
        Ok(NamedTheme { name, theme, path: None })
    }
}

fn builtin(name: &str, colors: &[(&str, &str)]) -> NamedTheme {
    let mut theme = Theme::default();
    for (k, v) in colors {
        if let (Some(slot), Some(c)) = (theme.get_mut(k), hex(v)) {
            *slot = c;
        }
    }
    NamedTheme { name: name.into(), theme, path: None }
}

pub fn builtins() -> Vec<NamedTheme> {
    vec![
        builtin("Graphite", &[]),
        builtin(
            "Valorant",
            &[
                ("bg", "#0f1923"),
                ("surface", "#16222d"),
                ("surface_hover", "#1d2b38"),
                ("surface_active", "#243545"),
                ("border", "#2a3b4b"),
                ("text", "#ece8e1"),
                ("text_dim", "#a3aab3"),
                ("text_faint", "#66727f"),
                ("accent", "#ff4655"),
                ("good", "#3fd6a8"),
                ("hud_bg", "#0f1923c0"),
                ("world_bg", "#1a232c"),
                ("wall", "#384350"),
                ("wall_top", "#4a5563"),
                ("floor", "#2a333d"),
                ("floor_far", "#323c47"),
                ("ceiling", "#43505d"),
                ("grid", "#596675"),
                ("floor_grid", "#414d5a"),
                ("target", "#ff4655"),
            ],
        ),
        builtin(
            "Classic Grey",
            &[
                ("bg", "#121315"),
                ("surface", "#1b1c1f"),
                ("surface_hover", "#232428"),
                ("surface_active", "#2b2d32"),
                ("border", "#2e3035"),
                ("accent", "#ff8a1f"),
                ("accent_text", "#16100a"),
                ("world_bg", "#24262b"),
                ("wall", "#5a5d63"),
                ("wall_top", "#6b6e75"),
                ("floor", "#45484e"),
                ("floor_far", "#4e5157"),
                ("ceiling", "#62656b"),
                ("grid", "#7c8088"),
                ("floor_grid", "#5d6168"),
                ("target", "#e8322e"),
            ],
        ),
        builtin(
            "Midnight",
            &[
                ("bg", "#07080c"),
                ("surface", "#0e1017"),
                ("surface_hover", "#141724"),
                ("surface_active", "#1a1e2e"),
                ("border", "#1c2030"),
                ("accent", "#8b7bff"),
                ("world_bg", "#06070b"),
                ("wall", "#141722"),
                ("wall_top", "#1a1e2c"),
                ("floor", "#0e1018"),
                ("floor_far", "#121520"),
                ("ceiling", "#171a26"),
                ("grid", "#2b3146"),
                ("floor_grid", "#1f2435"),
                ("target", "#39e6ff"),
            ],
        ),
        builtin(
            "Nord",
            &[
                ("bg", "#242933"),
                ("surface", "#2e3440"),
                ("surface_hover", "#353c4a"),
                ("surface_active", "#3b4252"),
                ("border", "#434c5e"),
                ("text", "#eceff4"),
                ("text_dim", "#a9b1c1"),
                ("text_faint", "#6d778a"),
                ("accent", "#88c0d0"),
                ("accent_text", "#1e232b"),
                ("good", "#a3be8c"),
                ("bad", "#bf616a"),
                ("world_bg", "#2e3440"),
                ("wall", "#4c566a"),
                ("wall_top", "#5a6478"),
                ("floor", "#3b4252"),
                ("floor_far", "#434c5e"),
                ("ceiling", "#56607a"),
                ("grid", "#6a758d"),
                ("floor_grid", "#525c70"),
                ("target", "#d08770"),
            ],
        ),
        builtin(
            "Daylight",
            &[
                ("bg", "#f3f4f6"),
                ("surface", "#ffffff"),
                ("surface_hover", "#f0f2f5"),
                ("surface_active", "#e6e9ef"),
                ("border", "#dde1e7"),
                ("text", "#14171c"),
                ("text_dim", "#525a66"),
                ("text_faint", "#8b93a0"),
                ("accent", "#2563eb"),
                ("good", "#14a660"),
                ("bad", "#d93a3f"),
                ("hud_bg", "#ffffffcc"),
                ("world_bg", "#c9ced6"),
                ("wall", "#b9bfc8"),
                ("wall_top", "#cfd4dc"),
                ("floor", "#a9afb9"),
                ("floor_far", "#b3b9c2"),
                ("ceiling", "#d7dbe2"),
                ("grid", "#9ba2ad"),
                ("floor_grid", "#959ca8"),
                ("target", "#e5262e"),
            ],
        ),
    ]
}

pub fn load_dir(dir: &Path) -> (Vec<NamedTheme>, Vec<String>) {
    let _ = std::fs::create_dir_all(dir);
    let mut files = vec![];
    crate::scenario::collect_files(dir, &["toml"], &mut files, 0);
    let mut out = vec![];
    let mut errs = vec![];
    for p in files {
        match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| NamedTheme::from_toml(&t)) {
            Ok(mut t) => {
                t.path = Some(p);
                out.push(t);
            }
            Err(e) => errs.push(format!("{}: {e}", p.display())),
        }
    }
    (out, errs)
}

static CURRENT: RwLock<Option<Theme>> = RwLock::new(None);

/// The active theme (cheap copy).
pub fn t() -> Theme {
    CURRENT.read().ok().and_then(|g| *g).unwrap_or_default()
}

pub fn set(theme: Theme) {
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(theme);
    }
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

pub fn mix(a: Color, b: Color, k: f32) -> Color {
    Color::new(a.r + (b.r - a.r) * k, a.g + (b.g - a.g) * k, a.b + (b.b - a.b) * k, a.a + (b.a - a.a) * k)
}

pub fn category(c: crate::scenario::Category) -> Color {
    use crate::scenario::Category::*;
    // Muted, theme-independent hues used only as small dots.
    match c {
        Clicking => Color::from_rgba(240, 150, 70, 255),
        Tracking => Color::from_rgba(90, 160, 255, 255),
        Switching => Color::from_rgba(180, 130, 255, 255),
        Timing => Color::from_rgba(235, 200, 80, 255),
        Strafe => Color::from_rgba(80, 200, 140, 255),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for b in builtins() {
            let back = NamedTheme::from_toml(&b.to_toml()).unwrap();
            assert_eq!(back.name, b.name);
            for (k, _) in Theme::KEYS {
                let (x, y) = (b.theme.get(k).unwrap(), back.theme.get(k).unwrap());
                assert!((x.r - y.r).abs() < 0.01 && (x.a - y.a).abs() < 0.01, "{} {k}", b.name);
            }
        }
    }

    #[test]
    fn shipped_theme_files_load() {
        let (themes, errs) = load_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/themes")));
        assert!(errs.is_empty(), "{errs:?}");
        assert!(themes.iter().any(|t| t.name == "Forest"));
    }

    #[test]
    fn partial_theme() {
        let t = NamedTheme::from_toml("name = \"Mine\"\n[colors]\naccent = \"#ff0000\"").unwrap();
        assert_eq!(t.theme.accent, Color::from_rgba(255, 0, 0, 255));
        assert_eq!(t.theme.bg, Theme::default().bg);
        assert!(NamedTheme::from_toml("name = \"x\"").is_err());
    }
}
