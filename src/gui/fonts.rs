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

/// Every font, so they can be loaded up front.
const ALL: [&FontAsset; 8] = [
    &SANS,
    &SANS_MEDIUM,
    &SANS_SEMIBOLD,
    &SANS_BOLD,
    &DISPLAY_BOLD,
    &MONO_MEDIUM,
    &MONO_SEMIBOLD,
    &MONO_BOLD,
];

/// Load every font before the first frame is laid out.
///
/// Ply loads a font lazily, from inside the *render* pass
/// (`FontManager::ensure` in `renderer::render`) — which runs after layout has
/// already measured the text. So on the first frame `FontManager::get` returns
/// `None` for a font nobody has drawn yet, and macroquad quietly measures with
/// its built-in font instead. Anything sized `Fit` around non-default-font text
/// therefore comes out to the *wrong* font's width, and since measurements are
/// cached the bad number never corrects itself: a mono chip ends up far too
/// narrow and its own text spills out of it.
///
/// Loading everything up front means every measurement, including the first,
/// uses the font the text is actually drawn in.
pub async fn preload() {
    for f in ALL {
        ply_engine::renderer::FontManager::ensure(f).await;
    }
    tracing::debug!("preloaded {} fonts", ALL.len());
}
