//! Valorant-style crosshair: outlines, centre dot, inner and outer lines.
//! Imports and exports Valorant crosshair profile codes ("0;P;c;5;h;0;...").

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Lines {
    pub show: bool,
    pub length: f32,
    /// Separate vertical length (Valorant "allow vertical scaling").
    pub split_vertical: bool,
    pub vertical_length: f32,
    pub thickness: f32,
    pub offset: f32,
    pub opacity: f32,
}

impl Lines {
    pub fn inner_default() -> Lines {
        Lines { show: true, length: 6.0, split_vertical: false, vertical_length: 6.0, thickness: 2.0, offset: 3.0, opacity: 0.8 }
    }
    pub fn outer_default() -> Lines {
        Lines { show: true, length: 2.0, split_vertical: false, vertical_length: 2.0, thickness: 2.0, offset: 10.0, opacity: 0.35 }
    }
}

impl Default for Lines {
    fn default() -> Self {
        Lines::inner_default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Crosshair {
    pub color: [u8; 3],
    pub outline: bool,
    pub outline_thickness: f32,
    pub outline_opacity: f32,
    pub dot: bool,
    pub dot_thickness: f32,
    pub dot_opacity: f32,
    pub inner: Lines,
    pub outer: Lines,
}

impl Default for Crosshair {
    /// A clean, readable default: cyan inner lines, no outer lines.
    fn default() -> Self {
        Crosshair {
            color: [0, 255, 255],
            outline: true,
            outline_thickness: 1.0,
            outline_opacity: 0.5,
            dot: false,
            dot_thickness: 2.0,
            dot_opacity: 1.0,
            inner: Lines { length: 5.0, offset: 3.0, opacity: 1.0, ..Lines::inner_default() },
            outer: Lines { show: false, ..Lines::outer_default() },
        }
    }
}

/// Valorant's preset colour slots.
pub const VALORANT_COLORS: [[u8; 3]; 8] = [
    [255, 255, 255],
    [0, 255, 0],
    [127, 255, 0],
    [223, 255, 0],
    [255, 255, 0],
    [0, 255, 255],
    [255, 0, 255],
    [255, 0, 0],
];

impl Crosshair {
    /// Valorant's own defaults (what an empty code "0" means).
    pub fn valorant_default() -> Crosshair {
        Crosshair {
            color: [255, 255, 255],
            outline: true,
            outline_thickness: 1.0,
            outline_opacity: 0.5,
            dot: false,
            dot_thickness: 2.0,
            dot_opacity: 1.0,
            inner: Lines::inner_default(),
            outer: Lines::outer_default(),
        }
    }

    /// Parse a Valorant crosshair profile code. Uses the primary ("P") crosshair.
    pub fn from_valorant_code(code: &str) -> Result<Crosshair, String> {
        let code = code.trim().trim_matches('"');
        let parts: Vec<&str> = code.split(';').map(str::trim).collect();
        if parts.first() != Some(&"0") {
            return Err("not a Valorant crosshair code (should start with \"0;\")".into());
        }
        let mut c = Crosshair::valorant_default();
        let mut section = "";
        let mut custom: Option<[u8; 3]> = None;
        let mut i = 1;
        while i < parts.len() {
            let k = parts[i];
            if matches!(k, "P" | "A" | "S") {
                section = k;
                i += 1;
                continue;
            }
            let Some(v) = parts.get(i + 1).copied() else { break };
            i += 2;
            if section != "P" {
                continue;
            }
            let f = || v.parse::<f32>().map_err(|_| format!("bad value for {k}: {v}"));
            let b = || Ok::<bool, String>(v == "1");
            match k {
                "c" => {
                    let idx = v.parse::<usize>().map_err(|_| format!("bad colour index {v}"))?;
                    if idx < 8 {
                        c.color = VALORANT_COLORS[idx];
                    }
                }
                "u" => {
                    let h = v.trim_start_matches('#');
                    let byte = |j: usize| h.get(j..j + 2).and_then(|s| u8::from_str_radix(s, 16).ok());
                    if let (Some(r), Some(g), Some(bb)) = (byte(0), byte(2), byte(4)) {
                        custom = Some([r, g, bb]);
                    }
                }
                "h" => c.outline = b()?,
                "t" => c.outline_thickness = f()?,
                "o" => c.outline_opacity = f()?,
                "d" => c.dot = b()?,
                "z" => c.dot_thickness = f()?,
                "a" => c.dot_opacity = f()?,
                "0b" => c.inner.show = b()?,
                "0t" => c.inner.thickness = f()?,
                "0l" => c.inner.length = f()?,
                "0v" => c.inner.vertical_length = f()?,
                "0g" => c.inner.split_vertical = b()?,
                "0o" => c.inner.offset = f()?,
                "0a" => c.inner.opacity = f()?,
                "1b" => c.outer.show = b()?,
                "1t" => c.outer.thickness = f()?,
                "1l" => c.outer.length = f()?,
                "1v" => c.outer.vertical_length = f()?,
                "1g" => c.outer.split_vertical = b()?,
                "1o" => c.outer.offset = f()?,
                "1a" => c.outer.opacity = f()?,
                _ => {} // firing/movement error, spectate options... not used here
            }
        }
        if let Some(u) = custom {
            // "c;8" + "u;RRGGBBAA" = custom colour; a "u" always wins when present.
            c.color = u;
        }
        if !c.inner.split_vertical {
            c.inner.vertical_length = c.inner.length;
        }
        if !c.outer.split_vertical {
            c.outer.vertical_length = c.outer.length;
        }
        Ok(c)
    }

    /// Export as a Valorant profile code (all values written explicitly).
    pub fn to_valorant_code(&self) -> String {
        let n = |v: f32| {
            let s = format!("{:.3}", v);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        };
        let b = |v: bool| if v { "1" } else { "0" };
        let mut s = String::from("0;P;");
        match VALORANT_COLORS.iter().position(|c| *c == self.color) {
            Some(i) => s.push_str(&format!("c;{i};")),
            None => s.push_str(&format!(
                "c;8;u;{:02X}{:02X}{:02X}FF;",
                self.color[0], self.color[1], self.color[2]
            )),
        }
        s.push_str(&format!(
            "h;{};t;{};o;{};d;{};z;{};a;{};",
            b(self.outline),
            n(self.outline_thickness),
            n(self.outline_opacity),
            b(self.dot),
            n(self.dot_thickness),
            n(self.dot_opacity)
        ));
        for (p, l) in [("0", &self.inner), ("1", &self.outer)] {
            s.push_str(&format!(
                "{p}b;{};{p}t;{};{p}l;{};{p}v;{};{p}g;{};{p}o;{};{p}a;{};{p}m;0;{p}f;0;",
                b(l.show),
                n(l.thickness),
                n(l.length),
                n(l.vertical_length),
                b(l.split_vertical),
                n(l.offset),
                n(l.opacity)
            ));
        }
        s.trim_end_matches(';').to_string()
    }

    /// Draw centred at (cx, cy). `scale` = screen height / 1080 (Valorant values are 1080p pixels).
    pub fn draw(&self, cx: f32, cy: f32, scale: f32) {
        let cx = cx.floor();
        let cy = cy.floor();
        let col = Color::from_rgba(self.color[0], self.color[1], self.color[2], 255);
        let ot = if self.outline { (self.outline_thickness * scale).max(1.0).round() } else { 0.0 };
        let oc = Color::new(0.0, 0.0, 0.0, self.outline_opacity.clamp(0.0, 1.0));
        let px = |v: f32| (v * scale).round();

        let rect = |x: f32, y: f32, w: f32, h: f32, a: f32| {
            if w <= 0.0 || h <= 0.0 || a <= 0.0 {
                return;
            }
            if ot > 0.0 {
                draw_rectangle(x - ot, y - ot, w + 2.0 * ot, h + 2.0 * ot, Color::new(0.0, 0.0, 0.0, oc.a * a.min(1.0).max(0.4)));
            }
            draw_rectangle(x, y, w, h, Color::new(col.r, col.g, col.b, a.clamp(0.0, 1.0)));
        };
        let lines = |l: &Lines| {
            if !l.show {
                return;
            }
            let t = px(l.thickness).max(1.0);
            let len = px(l.length);
            let vlen = px(if l.split_vertical { l.vertical_length } else { l.length });
            let off = px(l.offset);
            // Centre thickness on the pixel grid: odd thickness straddles the centre pixel.
            let h0 = cy - (t / 2.0).floor();
            let v0 = cx - (t / 2.0).floor();
            let a = l.opacity;
            rect(cx + off + 1.0, h0, len, t, a); // right
            rect(cx - off - len, h0, len, t, a); // left
            rect(v0, cy + off + 1.0, t, vlen, a); // down
            rect(v0, cy - off - vlen, t, vlen, a); // up
        };
        lines(&self.outer);
        lines(&self.inner);
        if self.dot {
            let d = px(self.dot_thickness).max(1.0);
            let o = (d / 2.0).floor();
            rect(cx - o, cy - o, d, d, self.dot_opacity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_code() {
        // cyan, no outlines, centre dot, no outer lines
        let c = Crosshair::from_valorant_code("0;P;c;5;h;0;d;1;z;3;0l;4;0o;2;0a;1;1b;0").unwrap();
        assert_eq!(c.color, [0, 255, 255]);
        assert!(!c.outline);
        assert!(c.dot);
        assert_eq!(c.dot_thickness, 3.0);
        assert_eq!(c.inner.length, 4.0);
        assert_eq!(c.inner.offset, 2.0);
        assert_eq!(c.inner.opacity, 1.0);
        assert!(!c.outer.show);
        // defaults kept for unspecified fields
        assert_eq!(c.inner.thickness, 2.0);
    }

    #[test]
    fn parse_custom_colour_and_sections() {
        let c = Crosshair::from_valorant_code("0;s;1;P;c;8;u;12AB34FF;0t;1;A;c;1;0t;9").unwrap();
        assert_eq!(c.color, [0x12, 0xAB, 0x34]);
        assert_eq!(c.inner.thickness, 1.0); // ADS section value ignored
    }

    #[test]
    fn default_code() {
        assert_eq!(Crosshair::from_valorant_code("0").unwrap(), Crosshair::valorant_default());
        assert!(Crosshair::from_valorant_code("hello").is_err());
    }

    #[test]
    fn export_roundtrip() {
        let mut c = Crosshair::default();
        c.color = [10, 200, 30];
        c.inner.split_vertical = true;
        c.inner.vertical_length = 9.0;
        c.outer.show = true;
        c.outline_opacity = 0.75;
        let code = c.to_valorant_code();
        assert_eq!(Crosshair::from_valorant_code(&code).unwrap(), c, "{code}");
        let code2 = Crosshair::valorant_default().to_valorant_code();
        assert!(code2.starts_with("0;P;c;0;"), "{code2}");
    }
}
