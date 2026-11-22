//! Map look: procedural skies, patterned floors/walls, arena layout, fog and
//! target appearance. Everything is drawn with small GLSL shaders, so there
//! are no texture files to ship.

use macroquad::miniquad::{Comparison, PipelineParams, UniformType};
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Layout {
    /// Walls and ceiling (classic KovaaK's room).
    #[default]
    Enclosed,
    /// Walls, open sky above.
    OpenTop,
    /// Just a huge floor fading into the sky.
    OpenField,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Enclosed, Layout::OpenTop, Layout::OpenField];
    pub fn label(self) -> &'static str {
        match self {
            Layout::Enclosed => "Room",
            Layout::OpenTop => "Open roof",
            Layout::OpenField => "Open field",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pattern {
    Plain,
    #[default]
    Grid,
    Checker,
    Tiles,
    Concrete,
    NeonGrid,
    Hex,
    Planks,
    Dots,
    Stripes,
    Marble,
}

impl Pattern {
    pub const ALL: [Pattern; 11] = [
        Pattern::Plain,
        Pattern::Grid,
        Pattern::Checker,
        Pattern::Tiles,
        Pattern::Concrete,
        Pattern::NeonGrid,
        Pattern::Hex,
        Pattern::Planks,
        Pattern::Dots,
        Pattern::Stripes,
        Pattern::Marble,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Plain => "Plain",
            Pattern::Grid => "Grid",
            Pattern::Checker => "Checker",
            Pattern::Tiles => "Tiles",
            Pattern::Concrete => "Concrete",
            Pattern::NeonGrid => "Neon grid",
            Pattern::Hex => "Hex",
            Pattern::Planks => "Planks",
            Pattern::Dots => "Dots",
            Pattern::Stripes => "Stripes",
            Pattern::Marble => "Marble",
        }
    }
    fn id(self) -> f32 {
        Pattern::ALL.iter().position(|p| *p == self).unwrap_or(0) as f32
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BallStyle {
    #[default]
    Matte,
    Glossy,
    Flat,
    Glow,
}

impl BallStyle {
    pub const ALL: [BallStyle; 4] = [BallStyle::Matte, BallStyle::Glossy, BallStyle::Flat, BallStyle::Glow];
    pub fn label(self) -> &'static str {
        match self {
            BallStyle::Matte => "Matte",
            BallStyle::Glossy => "Glossy",
            BallStyle::Flat => "Flat",
            BallStyle::Glow => "Glow",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Surface {
    pub pattern: Pattern,
    /// Base colour.
    pub a: [u8; 3],
    /// Pattern colour (lines, alternate tiles, veins...).
    pub b: [u8; 3],
    /// Pattern size in metres.
    pub scale: f32,
}

impl Default for Surface {
    fn default() -> Self {
        Surface { pattern: Pattern::Grid, a: [58, 62, 70], b: [96, 102, 112], scale: 1.0 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MapCfg {
    /// Name of the preset last applied (display only).
    pub preset: String,
    pub sky: String,
    pub layout: Layout,
    pub floor: Surface,
    pub wall: Surface,
    /// 0 = no fog, 1 = thick.
    pub fog: f32,
    pub ball: [u8; 3],
    pub ball_style: BallStyle,
}

impl Default for MapCfg {
    fn default() -> Self {
        presets().into_iter().next().map(|p| p.1).unwrap()
    }
}

pub struct SkyDef {
    pub name: &'static str,
    style: f32,
    top: &'static str,
    horizon: &'static str,
    bottom: &'static str,
    sun_dir: [f32; 3],
    sun: &'static str,
    sun_size: f32,
    c1: &'static str,
    c2: &'static str,
    stars: f32,
    clouds: f32,
}

impl SkyDef {
    pub fn horizon(&self) -> Color {
        hex(self.horizon)
    }
    pub fn top(&self) -> Color {
        hex(self.top)
    }
}

// style: 0 gradient+sun, 1 night (stars+moon), 2 aurora, 3 nebula, 4 synthwave, 5 clouds
pub const SKIES: &[SkyDef] = &[
    SkyDef { name: "Studio", style: 0.0, top: "#454a53", horizon: "#666b75", bottom: "#2a2d33", sun_dir: [0.0, 1.0, 0.0], sun: "#000000", sun_size: 0.0, c1: "#000000", c2: "#000000", stars: 0.0, clouds: 0.0 },
    SkyDef { name: "Clear Day", style: 5.0, top: "#2a66d1", horizon: "#b4d8f7", bottom: "#6d8fb0", sun_dir: [0.35, 0.55, 1.0], sun: "#fff6dc", sun_size: 0.0016, c1: "#b8c4d6", c2: "#ffffff", stars: 0.0, clouds: 0.32 },
    SkyDef { name: "Cotton Candy", style: 5.0, top: "#7fb4ff", horizon: "#ffc8e6", bottom: "#c8a8d8", sun_dir: [-0.3, 0.3, 1.0], sun: "#fff0f6", sun_size: 0.0018, c1: "#e8a8cf", c2: "#fff4fb", stars: 0.0, clouds: 0.45 },
    SkyDef { name: "Golden Hour", style: 5.0, top: "#3a5ea6", horizon: "#f7c27c", bottom: "#6e5a52", sun_dir: [0.4, 0.09, 1.0], sun: "#ffd28c", sun_size: 0.0028, c1: "#a5788c", c2: "#ffdcae", stars: 0.0, clouds: 0.28 },
    SkyDef { name: "Sunset", style: 0.0, top: "#1b2a5e", horizon: "#ff7b47", bottom: "#3a2241", sun_dir: [0.0, 0.05, 1.0], sun: "#ffc27a", sun_size: 0.004, c1: "#000000", c2: "#000000", stars: 0.0, clouds: 0.0 },
    SkyDef { name: "Dusk", style: 1.0, top: "#120f2f", horizon: "#7d3f70", bottom: "#1a1426", sun_dir: [0.0, -1.0, 0.0], sun: "#000000", sun_size: 0.0, c1: "#000000", c2: "#000000", stars: 0.012, clouds: 0.0 },
    SkyDef { name: "Overcast", style: 5.0, top: "#868e9a", horizon: "#c6cbd2", bottom: "#6c7280", sun_dir: [0.2, 0.6, 1.0], sun: "#30343a", sun_size: 0.0, c1: "#7b838e", c2: "#d9dde2", stars: 0.0, clouds: 0.85 },
    SkyDef { name: "Arctic", style: 5.0, top: "#5fa0d6", horizon: "#eaf6ff", bottom: "#c9dceb", sun_dir: [-0.4, 0.3, 1.0], sun: "#ffffff", sun_size: 0.0016, c1: "#c4d4e6", c2: "#ffffff", stars: 0.0, clouds: 0.22 },
    SkyDef { name: "Desert Haze", style: 0.0, top: "#5b8ec6", horizon: "#f3d8a9", bottom: "#c89b63", sun_dir: [0.1, 0.7, 1.0], sun: "#fffbe8", sun_size: 0.003, c1: "#000000", c2: "#000000", stars: 0.0, clouds: 0.0 },
    SkyDef { name: "Ocean Dusk", style: 1.0, top: "#081a33", horizon: "#3d7fb8", bottom: "#0a1626", sun_dir: [0.0, -1.0, 0.0], sun: "#000000", sun_size: 0.0, c1: "#000000", c2: "#000000", stars: 0.008, clouds: 0.0 },
    SkyDef { name: "Starry Night", style: 1.0, top: "#02040c", horizon: "#152442", bottom: "#03050a", sun_dir: [-0.5, 0.45, 1.0], sun: "#e2e9ff", sun_size: 0.0012, c1: "#000000", c2: "#000000", stars: 0.05, clouds: 0.0 },
    SkyDef { name: "Blood Moon", style: 1.0, top: "#0a0204", horizon: "#3b0b12", bottom: "#070102", sun_dir: [0.45, 0.35, 1.0], sun: "#ff3b2e", sun_size: 0.0045, c1: "#000000", c2: "#000000", stars: 0.03, clouds: 0.0 },
    SkyDef { name: "Aurora", style: 2.0, top: "#020611", horizon: "#0e2439", bottom: "#02040a", sun_dir: [0.0, -1.0, 0.0], sun: "#000000", sun_size: 0.0, c1: "#38ff9e", c2: "#8b5cff", stars: 0.04, clouds: 0.0 },
    SkyDef { name: "Nebula", style: 3.0, top: "#05030d", horizon: "#0e0a20", bottom: "#03020a", sun_dir: [0.0, -1.0, 0.0], sun: "#000000", sun_size: 0.0, c1: "#6b3cff", c2: "#ff3c9b", stars: 0.06, clouds: 0.0 },
    SkyDef { name: "Deep Space", style: 3.0, top: "#020509", horizon: "#06121a", bottom: "#010305", sun_dir: [-0.6, 0.25, 1.0], sun: "#bfe8ff", sun_size: 0.0009, c1: "#1fb6ca", c2: "#d24b2f", stars: 0.07, clouds: 0.0 },
    SkyDef { name: "Synthwave", style: 4.0, top: "#13052c", horizon: "#ff3f8e", bottom: "#1a0630", sun_dir: [0.0, 0.13, 1.0], sun: "#ffe066", sun_size: 0.03, c1: "#ffe066", c2: "#ff2e88", stars: 0.02, clouds: 0.0 },
    SkyDef { name: "Void", style: 0.0, top: "#060608", horizon: "#0c0c10", bottom: "#040405", sun_dir: [0.0, -1.0, 0.0], sun: "#000000", sun_size: 0.0, c1: "#000000", c2: "#000000", stars: 0.0, clouds: 0.0 },
];

pub fn sky(name: &str) -> &'static SkyDef {
    SKIES.iter().find(|s| s.name.eq_ignore_ascii_case(name)).unwrap_or(&SKIES[0])
}

pub fn hex(s: &str) -> Color {
    crate::theme::hex(s).unwrap_or(BLACK)
}

fn rgb(s: &str) -> [u8; 3] {
    let c = hex(s);
    [(c.r * 255.0).round() as u8, (c.g * 255.0).round() as u8, (c.b * 255.0).round() as u8]
}

fn preset(name: &str, sky: &str, layout: Layout, floor: (Pattern, &str, &str, f32), wall: (Pattern, &str, &str, f32), fog: f32, ball: &str, style: BallStyle) -> (String, MapCfg) {
    (
        name.to_string(),
        MapCfg {
            preset: name.to_string(),
            sky: sky.to_string(),
            layout,
            floor: Surface { pattern: floor.0, a: rgb(floor.1), b: rgb(floor.2), scale: floor.3 },
            wall: Surface { pattern: wall.0, a: rgb(wall.1), b: rgb(wall.2), scale: wall.3 },
            fog,
            ball: rgb(ball),
            ball_style: style,
        },
    )
}

pub fn presets() -> Vec<(String, MapCfg)> {
    use BallStyle::*;
    use Layout::*;
    use Pattern::*;
    vec![
        preset("Classic", "Studio", Enclosed, (Grid, "#2c2f36", "#464b54", 1.0), (Grid, "#3d4149", "#5f6670", 1.0), 0.15, "#ea3434", Matte),
        preset("Training Day", "Clear Day", OpenTop, (Tiles, "#c9ccd1", "#9aa0a8", 1.0), (Concrete, "#d8dade", "#b5b9bf", 1.0), 0.25, "#ff3d3d", Glossy),
        preset("Golden Plaza", "Golden Hour", OpenField, (Tiles, "#b89a74", "#8e7254", 1.5), (Concrete, "#c7ad8a", "#a08665", 1.0), 0.35, "#2fd3ff", Glossy),
        preset("Sunset Strip", "Sunset", OpenField, (Stripes, "#2a1d2e", "#3b2638", 2.0), (Plain, "#3a2840", "#3a2840", 1.0), 0.45, "#ffd84a", Glow),
        preset("Synthwave", "Synthwave", OpenField, (NeonGrid, "#0c0418", "#ff2e9a", 2.0), (NeonGrid, "#0c0418", "#3ad6ff", 2.0), 0.4, "#3af2ff", Glow),
        preset("Midnight Neon", "Starry Night", OpenTop, (NeonGrid, "#05070d", "#2f7dff", 1.0), (Hex, "#0a0e18", "#1f3c78", 1.2), 0.3, "#ff3df0", Glow),
        preset("Aurora Ice", "Aurora", OpenField, (Marble, "#c4d6e4", "#7d98ad", 2.0), (Marble, "#d6e3ee", "#8ba3b8", 2.0), 0.35, "#ff4f6d", Glossy),
        preset("Nebula Deck", "Nebula", OpenField, (Dots, "#0b0916", "#6b3cff", 1.0), (Hex, "#0e0b1c", "#3a1f7a", 1.2), 0.25, "#ffffff", Glow),
        preset("Space Station", "Deep Space", OpenTop, (Hex, "#1a1f26", "#2f3a46", 1.0), (Tiles, "#232a33", "#151a20", 1.0), 0.2, "#ff8a1f", Matte),
        preset("Arctic Base", "Arctic", OpenTop, (Tiles, "#eef3f8", "#c7d3df", 1.0), (Plain, "#f4f7fa", "#d5dee8", 1.0), 0.3, "#e8323c", Glossy),
        preset("Desert Ruins", "Desert Haze", OpenField, (Concrete, "#d2b07c", "#a98650", 2.0), (Planks, "#a8875a", "#7c5f3b", 1.0), 0.4, "#2a7bff", Matte),
        preset("Warehouse", "Overcast", Enclosed, (Concrete, "#55585e", "#3f4247", 2.0), (Planks, "#6b5641", "#4c3b2b", 1.0), 0.2, "#ffd23a", Matte),
        preset("Blood Moon", "Blood Moon", OpenField, (Checker, "#140507", "#22080c", 2.0), (Plain, "#1a0609", "#1a0609", 1.0), 0.5, "#f7f7f7", Glow),
        preset("Ocean Dusk", "Ocean Dusk", OpenField, (Grid, "#0c1b2c", "#1f4b75", 2.0), (Plain, "#0f2236", "#0f2236", 1.0), 0.45, "#ffb02e", Glossy),
        preset("Cotton Candy", "Cotton Candy", OpenTop, (Checker, "#f5d9ea", "#e8c2da", 1.5), (Dots, "#fbe8f3", "#f0c4de", 1.0), 0.2, "#7b5cff", Glossy),
        preset("Void", "Void", OpenField, (NeonGrid, "#050507", "#2a2a33", 2.0), (Plain, "#08080a", "#08080a", 1.0), 0.6, "#ffffff", Flat),
    ]
}

pub const BALL_COLORS: [[u8; 3]; 12] = [
    [234, 52, 52],
    [255, 70, 85],
    [255, 61, 240],
    [255, 138, 31],
    [255, 216, 58],
    [180, 255, 60],
    [60, 220, 90],
    [58, 242, 255],
    [47, 125, 255],
    [123, 92, 255],
    [255, 255, 255],
    [20, 20, 24],
];

pub const SURFACE_COLORS: [[u8; 3]; 14] = [
    [20, 21, 25],
    [44, 47, 54],
    [61, 65, 73],
    [95, 102, 112],
    [150, 156, 165],
    [216, 220, 226],
    [244, 247, 250],
    [12, 4, 24],
    [255, 46, 154],
    [58, 214, 255],
    [47, 125, 255],
    [56, 255, 158],
    [210, 176, 124],
    [107, 86, 65],
];

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

const PRECISION: &str = "#ifdef GL_FRAGMENT_PRECISION_HIGH\nprecision highp float;\n#else\nprecision mediump float;\n#endif\n";

const NOISE: &str = r#"
float hash12(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
float hash13(vec3 p3) { p3 = fract(p3 * 0.1031); p3 += dot(p3, p3.zyx + 31.32); return fract((p3.x + p3.y) * p3.z); }
float noise2(vec2 p) {
    vec2 i = floor(p); vec2 f = fract(p); f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2(1.0, 0.0)), f.x), mix(hash12(i + vec2(0.0, 1.0)), hash12(i + vec2(1.0, 1.0)), f.x), f.y);
}
float noise3(vec3 p) {
    vec3 i = floor(p); vec3 f = fract(p); f = f * f * (3.0 - 2.0 * f);
    float a = mix(hash13(i), hash13(i + vec3(1.0, 0.0, 0.0)), f.x);
    float b = mix(hash13(i + vec3(0.0, 1.0, 0.0)), hash13(i + vec3(1.0, 1.0, 0.0)), f.x);
    float c = mix(hash13(i + vec3(0.0, 0.0, 1.0)), hash13(i + vec3(1.0, 0.0, 1.0)), f.x);
    float d = mix(hash13(i + vec3(0.0, 1.0, 1.0)), hash13(i + vec3(1.0, 1.0, 1.0)), f.x);
    return mix(mix(a, b, f.y), mix(c, d, f.y), f.z);
}
float fbm2(vec2 p) { float s = 0.0; float a = 0.5; for (int i = 0; i < 5; i++) { s += a * noise2(p); p = p * 2.03 + vec2(1.7, 9.2); a *= 0.5; } return s; }
float fbm3(vec3 p) { float s = 0.0; float a = 0.5; for (int i = 0; i < 5; i++) { s += a * noise3(p); p = p * 2.03 + vec3(1.7, 9.2, 3.1); a *= 0.5; } return s; }
"#;

const SKY_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying highp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
}"#;

const SKY_FRAGMENT: &str = r#"
varying highp vec2 uv;
uniform vec3 Fwd; uniform vec3 Right; uniform vec3 Up; uniform vec2 TanFov;
uniform float Time; uniform float Style;
uniform vec3 Top; uniform vec3 Horizon; uniform vec3 Bottom;
uniform vec3 SunDir; uniform vec3 SunColor; uniform float SunSize;
uniform vec3 C1; uniform vec3 C2; uniform float Stars; uniform float Clouds;

vec3 gradient(vec3 d) {
    float h = d.y;
    if (h > 0.0) return mix(Horizon, Top, pow(clamp(h, 0.0, 1.0), 0.5));
    return mix(Horizon, Bottom, pow(clamp(-h, 0.0, 1.0), 0.35));
}
vec3 addSun(vec3 d, vec3 c) {
    float s = max(dot(d, normalize(SunDir)), 0.0);
    c += SunColor * pow(s, 6.0) * 0.18;
    c += SunColor * pow(s, 90.0) * 0.45;
    c = mix(c, SunColor * 1.15, smoothstep(1.0 - SunSize, 1.0 - SunSize * 0.55, s));
    return c;
}
float stars(vec3 d, float density) {
    vec3 p = d * 260.0; vec3 i = floor(p); float h = hash13(i);
    if (h > 1.0 - density) {
        vec3 o = vec3(hash13(i + 3.1), hash13(i + 7.7), hash13(i + 1.3)) * 0.6 + 0.2;
        float r = length(fract(p) - o);
        float tw = 0.7 + 0.3 * sin(Time * (1.5 + h * 3.0) + h * 90.0);
        return smoothstep(0.16, 0.0, r) * tw * (0.35 + 0.65 * fract(h * 17.3));
    }
    return 0.0;
}
void main() {
    vec3 d = normalize(Fwd + uv.x * TanFov.x * Right + uv.y * TanFov.y * Up);
    vec3 c = gradient(d);
    if (Style < 0.5) {
        c = addSun(d, c);
    } else if (Style < 1.5) {
        c += vec3(stars(d, Stars)) * smoothstep(-0.05, 0.15, d.y);
        c = addSun(d, c);
    } else if (Style < 2.5) {
        c += vec3(stars(d, Stars)) * smoothstep(-0.05, 0.15, d.y);
        if (d.y > 0.0) {
            vec2 q = d.xz / (d.y + 0.25);
            float a = 0.0;
            for (int k = 0; k < 3; k++) {
                float fk = float(k);
                float w = fbm2(vec2(q.x * 0.35 + fk * 3.1, Time * 0.03 + fk)) * 5.0;
                float band = sin(q.y * 0.9 + w + fk * 1.7);
                a += exp(-abs(band) * 6.0) * (0.55 - fk * 0.12);
            }
            float env = smoothstep(0.04, 0.3, d.y) * smoothstep(0.95, 0.45, d.y);
            vec3 ac = mix(C1, C2, smoothstep(0.15, 0.6, d.y));
            c += ac * a * env * 0.9;
        }
    } else if (Style < 3.5) {
        float n = fbm3(d * 2.2 + vec3(0.0, 0.0, Time * 0.004));
        float n2 = fbm3(d * 4.5 + vec3(7.0, 2.0, 1.0));
        c += C1 * pow(n, 3.2) * 1.9 + C2 * pow(n2, 4.0) * 1.7;
        c += vec3(stars(d, Stars)) + vec3(stars(d * 1.7, Stars * 0.5)) * 0.6;
        c = addSun(d, c);
    } else if (Style < 4.5) {
        c += vec3(stars(d, Stars)) * smoothstep(0.1, 0.4, d.y);
        c += C2 * exp(-abs(d.y) * 14.0) * 0.55;
        vec3 sd = normalize(SunDir);
        float s = dot(d, sd);
        float disc = smoothstep(1.0 - SunSize, 1.0 - SunSize * 0.97, s);
        float rel = (d.y - sd.y) / sqrt(SunSize * 2.0);
        float bars = step(0.5, fract(rel * 6.0 + 0.25)) + step(0.15, rel);
        disc *= clamp(bars, 0.0, 1.0) * step(-0.0, d.y);
        vec3 sc = mix(C2, C1, smoothstep(-0.8, 0.8, rel));
        c += C2 * pow(max(s, 0.0), 30.0) * 0.35;
        c = mix(c, sc, disc);
    } else {
        c = addSun(d, c);
        if (d.y > 0.0) {
            vec2 q = d.xz / (d.y + 0.12) * 0.55 + vec2(Time * 0.006, Time * 0.002);
            float n = fbm2(q * 1.3);
            float cov = smoothstep(1.0 - Clouds - 0.15, 1.0 - Clouds + 0.25, n);
            float shade = fbm2(q * 1.3 + vec2(0.07, 0.05));
            vec3 cc = mix(C2, C1, clamp((shade - n) * 4.0 + 0.4, 0.0, 1.0));
            float sunlit = pow(max(dot(d, normalize(SunDir)), 0.0), 8.0);
            cc += SunColor * sunlit * 0.25;
            c = mix(c, cc, cov * smoothstep(0.0, 0.18, d.y));
        }
    }
    gl_FragColor = vec4(c, 1.0);
}"#;

const SURF_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying highp vec2 uv;
varying highp vec3 wpos;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    wpos = position;
    color = color0 / 255.0;
}"#;

const SURF_FRAGMENT: &str = r#"
varying highp vec2 uv;
varying highp vec3 wpos;
varying lowp vec4 color;
uniform vec3 ColA; uniform vec3 ColB; uniform float Pattern; uniform float Scale;
uniform vec3 CamPos; uniform vec3 FogColor; uniform float FogDensity;

float lineAA(float x, float w, float aa) {
    float d = abs(fract(x - 0.5) - 0.5);
    return 1.0 - smoothstep(w, w + aa, d);
}
float hexDist(vec2 p) {
    p = abs(p);
    return max(dot(p, normalize(vec2(1.0, 1.7320508))), p.x);
}
void main() {
    vec2 p = uv / Scale;
    float dist = length(wpos - CamPos);
    float aa = 0.0018 * dist / Scale + 0.002;
    float fade = clamp(aa * 5.0, 0.0, 1.0); // detail fades with distance
    vec3 c = ColA;
    if (Pattern < 0.5) {
    } else if (Pattern < 1.5) {
        float major = max(lineAA(p.x, 0.012, aa), lineAA(p.y, 0.012, aa));
        float minor = max(lineAA(p.x * 4.0, 0.025, aa * 4.0), lineAA(p.y * 4.0, 0.025, aa * 4.0)) * 0.22 * (1.0 - fade);
        c = mix(ColA, ColB, max(major * (1.0 - fade * 0.6), minor));
    } else if (Pattern < 2.5) {
        float k = mod(floor(p.x) + floor(p.y), 2.0);
        c = mix(ColA, ColB, mix(k, 0.5, fade));
    } else if (Pattern < 3.5) {
        vec2 t = p * 0.5; vec2 id = floor(t);
        float v = hash12(id);
        c = ColA * (0.93 + 0.14 * v);
        float g = max(lineAA(t.x, 0.012, aa * 0.5), lineAA(t.y, 0.012, aa * 0.5));
        c = mix(c, ColB, g * (1.0 - fade * 0.5));
    } else if (Pattern < 4.5) {
        float n = fbm2(p * 0.9);
        float n2 = noise2(p * 14.0);
        c = mix(ColA, ColB, smoothstep(0.35, 0.7, n) * 0.55) * (0.95 + 0.1 * mix(n2, 0.5, fade));
    } else if (Pattern < 5.5) {
        float dx = abs(fract(p.x - 0.5) - 0.5); float dy = abs(fract(p.y - 0.5) - 0.5);
        float d = min(dx, dy);
        float line = 1.0 - smoothstep(0.006, 0.006 + aa, d);
        float glow = exp(-d * 28.0) * 0.45 * (1.0 - fade * 0.7);
        c = ColA + ColB * (line * (1.0 - fade * 0.5) + glow);
    } else if (Pattern < 6.5) {
        vec2 s = vec2(1.0, 1.7320508);
        vec2 q = p * 1.2;
        vec2 a = mod(q, s) - s * 0.5; vec2 b = mod(q + s * 0.5, s) - s * 0.5;
        vec2 g = dot(a, a) < dot(b, b) ? a : b;
        float e = 0.5 - hexDist(g);
        float edge = 1.0 - smoothstep(0.03, 0.03 + aa * 1.2, e);
        c = mix(ColA, ColB, edge * (1.0 - fade * 0.5));
    } else if (Pattern < 7.5) {
        float row = floor(p.y * 4.0);
        float along = p.x + hash12(vec2(row, 3.0)) * 3.0;
        float seg = floor(along / 2.2);
        float shade = hash12(vec2(row, seg));
        float grain = noise2(vec2(along * 6.0, p.y * 60.0));
        c = mix(ColA, ColB, shade * 0.55 + mix(grain, 0.5, fade) * 0.3);
        float seam = max(lineAA(p.y * 4.0, 0.03, aa * 4.0), lineAA(along / 2.2, 0.004, aa / 2.2));
        c *= 1.0 - seam * 0.45 * (1.0 - fade);
    } else if (Pattern < 8.5) {
        vec2 f = fract(p * 2.0) - 0.5;
        float dd = length(f);
        float dotm = 1.0 - smoothstep(0.13, 0.13 + aa * 2.0, dd);
        c = mix(ColA, ColB, dotm * (1.0 - fade * 0.6));
    } else if (Pattern < 9.5) {
        float s = abs(fract((p.x + p.y) * 0.5) - 0.5);
        float band = smoothstep(0.25 - aa, 0.25 + aa, s);
        c = mix(ColA, ColB, mix(band, 0.5, fade));
    } else {
        float n = fbm2(p * 0.7);
        float v = abs(sin((p.x + p.y * 0.6) * 1.6 + n * 7.0));
        float vein = (pow(1.0 - v, 28.0) + pow(1.0 - abs(sin(p.x * 0.9 - p.y * 1.3 + n * 9.0)), 40.0) * 0.5) * (1.0 - fade * 0.5);
        c = mix(ColA, ColB, vein) * (0.97 + 0.06 * n);
    }
    c *= color.rgb;
    float fog = 1.0 - exp(-dist * FogDensity);
    c = mix(c, FogColor, clamp(fog, 0.0, 1.0));
    gl_FragColor = vec4(c, 1.0);
}"#;

fn uniforms(list: &[(&str, UniformType)]) -> Vec<UniformDesc> {
    list.iter().map(|(n, t)| UniformDesc::new(n, *t)).collect()
}

pub struct MapRenderer {
    sky: Option<Material>,
    floor: Option<Material>,
    wall: Option<Material>,
    pub error: Option<String>,
}

fn v3(c: Color) -> Vec3 {
    vec3(c.r, c.g, c.b)
}
fn c3(c: [u8; 3]) -> Vec3 {
    vec3(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0)
}

impl MapRenderer {
    pub fn new() -> MapRenderer {
        let sky_src = format!("#version 100\n{PRECISION}{NOISE}{SKY_FRAGMENT}");
        let surf_src = format!("#version 100\n{PRECISION}{NOISE}{SURF_FRAGMENT}");
        let mut error = None;
        let sky = load_material(
            ShaderSource::Glsl { vertex: SKY_VERTEX, fragment: &sky_src },
            MaterialParams {
                uniforms: uniforms(&[
                    ("Fwd", UniformType::Float3),
                    ("Right", UniformType::Float3),
                    ("Up", UniformType::Float3),
                    ("TanFov", UniformType::Float2),
                    ("Time", UniformType::Float1),
                    ("Style", UniformType::Float1),
                    ("Top", UniformType::Float3),
                    ("Horizon", UniformType::Float3),
                    ("Bottom", UniformType::Float3),
                    ("SunDir", UniformType::Float3),
                    ("SunColor", UniformType::Float3),
                    ("SunSize", UniformType::Float1),
                    ("C1", UniformType::Float3),
                    ("C2", UniformType::Float3),
                    ("Stars", UniformType::Float1),
                    ("Clouds", UniformType::Float1),
                ]),
                ..Default::default()
            },
        )
        .map_err(|e| error = Some(format!("sky shader: {e:?}")))
        .ok();
        let surf_params = || MaterialParams {
            uniforms: uniforms(&[
                ("ColA", UniformType::Float3),
                ("ColB", UniformType::Float3),
                ("Pattern", UniformType::Float1),
                ("Scale", UniformType::Float1),
                ("CamPos", UniformType::Float3),
                ("FogColor", UniformType::Float3),
                ("FogDensity", UniformType::Float1),
            ]),
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let surf = |error: &mut Option<String>| {
            load_material(ShaderSource::Glsl { vertex: SURF_VERTEX, fragment: &surf_src }, surf_params())
                .map_err(|e| *error = Some(format!("surface shader: {e:?}")))
                .ok()
        };
        let floor = surf(&mut error);
        let wall = surf(&mut error);
        MapRenderer { sky, floor, wall, error }
    }

    /// Draw the sky behind everything, for a camera looking along `fwd`.
    /// `area` is the screen rect to fill (whole screen in game, a box for previews).
    pub fn draw_sky(&self, cfg: &MapCfg, fwd: Vec3, hfov_deg: f32, area: Rect) {
        let sk = sky(&cfg.sky);
        let Some(mat) = &self.sky else {
            draw_rectangle(area.x, area.y, area.w, area.h, sk.horizon());
            return;
        };
        let fwd = fwd.normalize_or_zero();
        let right = fwd.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(fwd).normalize_or_zero();
        let th = (hfov_deg.to_radians() * 0.5).tan();
        let tv = th * area.h / area.w.max(1.0);
        mat.set_uniform("Fwd", fwd);
        mat.set_uniform("Right", right);
        mat.set_uniform("Up", up);
        mat.set_uniform("TanFov", vec2(th, tv));
        mat.set_uniform("Time", get_time() as f32);
        mat.set_uniform("Style", sk.style);
        mat.set_uniform("Top", v3(hex(sk.top)));
        mat.set_uniform("Horizon", v3(hex(sk.horizon)));
        mat.set_uniform("Bottom", v3(hex(sk.bottom)));
        mat.set_uniform("SunDir", Vec3::from(sk.sun_dir).normalize());
        mat.set_uniform("SunColor", v3(hex(sk.sun)));
        mat.set_uniform("SunSize", sk.sun_size);
        mat.set_uniform("C1", v3(hex(sk.c1)));
        mat.set_uniform("C2", v3(hex(sk.c2)));
        mat.set_uniform("Stars", sk.stars);
        mat.set_uniform("Clouds", sk.clouds);
        gl_use_material(mat);
        let w = WHITE;
        let verts = vec![
            Vertex::new(area.x, area.y, 0.0, -1.0, 1.0, w),
            Vertex::new(area.x + area.w, area.y, 0.0, 1.0, 1.0, w),
            Vertex::new(area.x + area.w, area.y + area.h, 0.0, 1.0, -1.0, w),
            Vertex::new(area.x, area.y + area.h, 0.0, -1.0, -1.0, w),
        ];
        draw_mesh(&Mesh { vertices: verts, indices: vec![0, 1, 2, 0, 2, 3], texture: None });
        gl_use_default_material();
    }

    /// Draw floor (and walls/ceiling depending on layout). Call with a 3D camera set.
    pub fn draw_arena(&self, cfg: &MapCfg, mn: Vec3, mx: Vec3, cam_pos: Vec3) {
        let sk = sky(&cfg.sky);
        let fog_color = v3(sk.horizon());
        let fog = cfg.fog.clamp(0.0, 1.0);
        let density = fog * fog * 0.03;
        let setup = |m: &Material, s: &Surface| {
            m.set_uniform("ColA", c3(s.a));
            m.set_uniform("ColB", c3(s.b));
            m.set_uniform("Pattern", s.pattern.id());
            m.set_uniform("Scale", s.scale.clamp(0.1, 20.0));
            m.set_uniform("CamPos", cam_pos);
            m.set_uniform("FogColor", fog_color);
            m.set_uniform("FogDensity", density);
        };
        let (x0, y0, z0, x1, y1, z1) = (mn.x, mn.y, mn.z, mx.x, mx.y, mx.z);
        let shade = |k: f32| Color::new(k, k, k, 1.0);

        // Floor.
        let (fx0, fz0, fx1, fz1) = if cfg.layout == Layout::OpenField {
            let cx = (x0 + x1) * 0.5;
            let cz = (z0 + z1) * 0.5;
            (cx - 600.0, cz - 600.0, cx + 600.0, cz + 600.0)
        } else {
            (x0, z0, x1, z1)
        };
        match &self.floor {
            Some(m) => {
                setup(m, &cfg.floor);
                gl_use_material(m);
            }
            None => {}
        }
        surf_quad(
            [vec3(fx0, y0, fz0), vec3(fx1, y0, fz0), vec3(fx1, y0, fz1), vec3(fx0, y0, fz1)],
            [vec2(fx0, fz0), vec2(fx1, fz0), vec2(fx1, fz1), vec2(fx0, fz1)],
            shade(1.0),
            if self.floor.is_none() { Some(rgbc(cfg.floor.a)) } else { None },
        );
        if cfg.layout != Layout::OpenField {
            if let Some(m) = &self.wall {
                setup(m, &cfg.wall);
                gl_use_material(m);
            }
            let fallback = if self.wall.is_none() { Some(rgbc(cfg.wall.a)) } else { None };
            // far / near (along x), left / right (along z)
            surf_quad([vec3(x0, y0, z1), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x0, y1, z1)], [vec2(x0, y0), vec2(x1, y0), vec2(x1, y1), vec2(x0, y1)], shade(0.96), fallback);
            surf_quad([vec3(x0, y0, z0), vec3(x1, y0, z0), vec3(x1, y1, z0), vec3(x0, y1, z0)], [vec2(x0, y0), vec2(x1, y0), vec2(x1, y1), vec2(x0, y1)], shade(0.82), fallback);
            surf_quad([vec3(x0, y0, z0), vec3(x0, y0, z1), vec3(x0, y1, z1), vec3(x0, y1, z0)], [vec2(z0, y0), vec2(z1, y0), vec2(z1, y1), vec2(z0, y1)], shade(0.88), fallback);
            surf_quad([vec3(x1, y0, z0), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x1, y1, z0)], [vec2(z0, y0), vec2(z1, y0), vec2(z1, y1), vec2(z0, y1)], shade(0.88), fallback);
            if cfg.layout == Layout::Enclosed {
                surf_quad([vec3(x0, y1, z0), vec3(x1, y1, z0), vec3(x1, y1, z1), vec3(x0, y1, z1)], [vec2(x0, z0), vec2(x1, z0), vec2(x1, z1), vec2(x0, z1)], shade(0.72), fallback);
            }
        }
        gl_use_default_material();
    }
}

fn rgbc(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

/// A quad with world-space pattern coordinates in `uv`. If `flat` is set the
/// shader isn't available and the quad is drawn in that colour instead.
fn surf_quad(p: [Vec3; 4], uv: [Vec2; 4], shade: Color, flat: Option<Color>) {
    let col = match flat {
        Some(f) => Color::new(f.r * shade.r, f.g * shade.g, f.b * shade.b, 1.0),
        None => shade,
    };
    let v: Vec<Vertex> = (0..4).map(|i| Vertex::new(p[i].x, p[i].y, p[i].z, uv[i].x, uv[i].y, col)).collect();
    draw_mesh(&Mesh { vertices: v, indices: vec![0, 1, 2, 0, 2, 3], texture: None });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_reference_real_skies() {
        for (name, p) in presets() {
            assert!(SKIES.iter().any(|s| s.name == p.sky), "{name} uses unknown sky {}", p.sky);
        }
        for s in SKIES {
            assert!(crate::theme::hex(s.top).is_some() && crate::theme::hex(s.c2).is_some(), "{}", s.name);
        }
    }

    #[test]
    fn map_cfg_roundtrip() {
        for (_, p) in presets() {
            let t = toml::to_string(&p).unwrap();
            let back: MapCfg = toml::from_str(&t).unwrap();
            assert_eq!(back, p);
        }
    }
}
