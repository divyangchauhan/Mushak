//! Embedded fonts.
//!
//! fontdue (which Ply rasterises with) renders a variable font's *default*
//! instance and cannot select a weight axis, so every weight the design uses is
//! a separate static TTF. Google Fonts only publishes variable builds of these
//! three families; these statics come from each family's upstream repo.
//!
//! All three are SIL Open Font License 1.1 — see `assets/fonts/OFL-*.txt`.

use ply_engine::prelude::FontAsset;

macro_rules! font {
    ($name:ident, $file:literal) => {
        pub static $name: FontAsset = FontAsset::Bytes {
            file_name: $file,
            data: include_bytes!(concat!("../../assets/fonts/", $file)),
        };
    };
}

// Body text.
font!(SANS, "HankenGrotesk-Regular.ttf");
font!(SANS_MEDIUM, "HankenGrotesk-Medium.ttf");
font!(SANS_SEMIBOLD, "HankenGrotesk-SemiBold.ttf");
font!(SANS_BOLD, "HankenGrotesk-Bold.ttf");

// Headings and the brand wordmark.
font!(DISPLAY_BOLD, "BricolageGrotesque-Bold.ttf");

// Keycaps, hex codes, battery/DPI readouts.
font!(MONO_MEDIUM, "JetBrainsMono-Medium.ttf");
font!(MONO_SEMIBOLD, "JetBrainsMono-SemiBold.ttf");
font!(MONO_BOLD, "JetBrainsMono-Bold.ttf");
