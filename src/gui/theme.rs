//! Design tokens, transcribed from the Claude Design handoff (`themeVars()`).
//!
//! The two palettes do not share a temperature: dark is cool slate, light is
//! warm paper. That is deliberate in the design, not a transcription slip.

use ply_engine::prelude::Color;

/// Which palette the settings window paints with.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn toggled(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
}

/// Accent options the design exposes. Vermilion is the default.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Vermilion,
    Marigold,
    Blue,
    Green,
}

impl Accent {
    fn rgb(self) -> u32 {
        match self {
            Accent::Vermilion => 0xe5533c,
            Accent::Marigold => 0xf2a73b,
            Accent::Blue => 0x6f9bd1,
            Accent::Green => 0x8bb87a,
        }
    }
}

/// Every colour the window paints with, resolved for one theme + accent.
#[derive(Clone, Copy)]
pub struct Palette {
    /// Window background.
    pub win: Color,
    /// Raised card / control background.
    pub surface: Color,
    /// Second-level fill: keycap wells, chips, inactive toggle tracks.
    pub s2: Color,
    /// Popover / modal background.
    pub pop: Color,
    pub line: Color,
    pub line_strong: Color,
    pub keycap: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub accent: Color,
    /// Ink that sits *on* the accent (button labels, toggle knobs).
    pub accent_ink: Color,
    pub danger: Color,
    pub good: Color,
    pub warn: Color,
    pub warn_ink: Color,
}

/// `0xRRGGBB` -> opaque Color. Ply's `Color` components are 0.0–255.0, not
/// 0.0–1.0, and its `From<u32>` already uses that convention.
fn rgb(hex: u32) -> Color {
    Color::from(hex)
}

/// `0xRRGGBB` + CSS alpha (0.0–1.0) -> Color. The design writes these as
/// `rgba(...)`; alpha is scaled into Ply's 0–255 range.
fn rgba(hex: u32, a: f32) -> Color {
    let mut c = Color::from(hex);
    c.a = a * 255.0;
    c
}

/// Fully transparent — used where the design leaves a background or border off.
pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

/// Opaque white, for ink on danger fills.
pub const WHITE: Color = Color::rgb(255.0, 255.0, 255.0);

impl Palette {
    pub fn resolve(theme: Theme, accent: Accent) -> Self {
        match theme {
            Theme::Dark => Palette {
                win: rgb(0x16181d),
                surface: rgb(0x1e2127),
                s2: rgb(0x262a31),
                pop: rgb(0x1c1f25),
                line: rgba(0xffffff, 0.08),
                line_strong: rgba(0xffffff, 0.13),
                keycap: rgba(0xffffff, 0.06),
                text: rgb(0xeef1f5),
                muted: rgb(0x99a1ad),
                faint: rgb(0x616976),
                accent: rgb(accent.rgb()),
                accent_ink: rgb(0x0e1013),
                danger: rgb(0xe5533c),
                good: rgb(0x77c07b),
                warn: rgb(0xe8a33c),
                warn_ink: rgb(0x3a2705),
            },
            Theme::Light => Palette {
                win: rgb(0xfcfaf5),
                surface: rgb(0xf4f0e7),
                s2: rgb(0xece6da),
                pop: rgb(0xfcfaf5),
                line: rgba(0x3c2d14, 0.10),
                line_strong: rgba(0x3c2d14, 0.17),
                keycap: rgba(0x3c2d14, 0.05),
                text: rgb(0x221e16),
                muted: rgb(0x6c6455),
                faint: rgb(0xa29986),
                accent: rgb(accent.rgb()),
                accent_ink: rgb(0xfff7ea),
                danger: rgb(0xc24327),
                good: rgb(0x4e9a54),
                warn: rgb(0xc98416),
                warn_ink: rgb(0xfff4e2),
            },
        }
    }

    /// The design's `color-mix(in srgb, <a> <pct>%, <b>)`, which egui/Ply have
    /// no equivalent for. Mixes in straight sRGB, matching the CSS.
    pub fn mix(a: Color, pct: f32, b: Color) -> Color {
        let t = pct.clamp(0.0, 100.0) / 100.0;
        Color::rgba(
            a.r * t + b.r * (1.0 - t),
            a.g * t + b.g * (1.0 - t),
            a.b * t + b.b * (1.0 - t),
            a.a * t + b.a * (1.0 - t),
        )
    }
}
