//! Icons, rasterised from `assets/icons/*.svg` by `build.rs`.
//!
//! Each is white-on-transparent. Tint at the call site by setting the
//! element's `background_color` — that is what Ply multiplies the texture by,
//! and it stands in for the design's `currentColor`.

use ply_engine::prelude::GraphicAsset;

macro_rules! icon {
    ($name:ident, $file:literal) => {
        pub static $name: GraphicAsset = GraphicAsset::Bytes {
            file_name: concat!($file, ".png"),
            data: include_bytes!(concat!(env!("OUT_DIR"), "/icons/", $file, ".png")),
        };
    };
}

// Nav rail.
icon!(NAV_BUTTONS, "nav_buttons");
icon!(NAV_SCROLL, "nav_scroll");
icon!(NAV_GESTURES, "nav_gestures");
icon!(NAV_PROFILES, "nav_profiles");
icon!(NAV_DEVICE, "nav_device");

// Button rows.
icon!(BTN_BACK, "btn_back");
icon!(BTN_FORWARD, "btn_forward");
icon!(BTN_MIDDLE, "btn_middle");

// Gesture rows.
icon!(G_TAP, "g_tap");
icon!(G_UP, "g_up");
icon!(G_DOWN, "g_down");
icon!(G_LEFT, "g_left");
icon!(G_RIGHT, "g_right");

// Window chrome.
icon!(MOON, "moon");
icon!(SUN, "sun");
icon!(MINIMIZE, "minimize");
icon!(CLOSE, "close");
icon!(CLOSE_SMALL, "close_small");

// Controls.
icon!(CHEVRON_RIGHT, "chevron_right");
icon!(CHEVRON_DOWN, "chevron_down");
icon!(CHECK, "check");
icon!(PLUS, "plus");
icon!(TRASH, "trash");
icon!(ARROW_UP, "arrow_up");
icon!(ARROW_DOWN, "arrow_down");
icon!(TARGET, "target");
icon!(REFRESH, "refresh");
icon!(BLUETOOTH, "bluetooth");
icon!(SEARCH, "search");
icon!(WARN, "warn");
icon!(ASLEEP, "asleep");
icon!(BOLT, "bolt");

// Action-picker categories.
icon!(CAT_EDIT, "cat_edit");
icon!(CAT_NAV, "cat_nav");
icon!(CAT_SYSTEM, "cat_system");
icon!(CAT_MEDIA, "cat_media");
icon!(PASSTHROUGH, "passthrough");
icon!(DISABLED, "disabled");

// App icon (Modak).
icon!(MODAK_ACTIVE, "modak_active");
icon!(MODAK_PAUSED, "modak_paused");
