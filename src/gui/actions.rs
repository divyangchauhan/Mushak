//! The action vocabulary shared by the Buttons and Gestures sections and the
//! action picker: the preset list, how an action renders as keycaps, and the
//! grouping the picker shows.

use crate::config::{vk, Action, Modifier};
use ply_engine::prelude::GraphicAsset;
use Modifier::{Alt, Ctrl, Shift, Win};

use super::icons;

/// Picker groups, in the order the design lists them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Edit,
    Navigation,
    System,
    Media,
}

impl Category {
    pub const ORDER: [Category; 4] = [
        Category::Edit,
        Category::Navigation,
        Category::System,
        Category::Media,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Category::Edit => "Edit",
            Category::Navigation => "Navigation",
            Category::System => "System",
            Category::Media => "Media",
        }
    }
    pub fn icon(self) -> &'static GraphicAsset {
        match self {
            Category::Edit => &icons::CAT_EDIT,
            Category::Navigation => &icons::CAT_NAV,
            Category::System => &icons::CAT_SYSTEM,
            Category::Media => &icons::CAT_MEDIA,
        }
    }
}

/// A named preset the picker offers.
pub struct Preset {
    pub label: &'static str,
    pub cat: Category,
    /// Keycaps as the design draws them.
    pub keys: &'static [&'static str],
    pub action: fn() -> Action,
}

/// The preset list, transcribed from the design's `this.actions`.
pub fn presets() -> Vec<Preset> {
    macro_rules! p {
        ($label:literal, $cat:expr, [$($k:literal),*], $act:expr) => {
            Preset { label: $label, cat: $cat, keys: &[$($k),*], action: || $act }
        };
    }
    vec![
        p!("Copy", Category::Edit, ["Ctrl", "C"], Action::key(&[Ctrl], vk::letter(b'C'))),
        p!("Paste", Category::Edit, ["Ctrl", "V"], Action::key(&[Ctrl], vk::letter(b'V'))),
        p!("Cut", Category::Edit, ["Ctrl", "X"], Action::key(&[Ctrl], vk::letter(b'X'))),
        p!("Undo", Category::Edit, ["Ctrl", "Z"], Action::key(&[Ctrl], vk::letter(b'Z'))),
        p!("Redo", Category::Edit, ["Ctrl", "Y"], Action::key(&[Ctrl], vk::letter(b'Y'))),
        p!("Close tab", Category::Navigation, ["Ctrl", "W"], Action::key(&[Ctrl], vk::letter(b'W'))),
        p!("New", Category::Navigation, ["Ctrl", "N"], Action::key(&[Ctrl], vk::letter(b'N'))),
        p!("New tab", Category::Navigation, ["Ctrl", "T"], Action::key(&[Ctrl], vk::letter(b'T'))),
        p!("Reopen tab", Category::Navigation, ["Ctrl", "Shift", "T"], Action::key(&[Ctrl, Shift], vk::letter(b'T'))),
        p!("Find", Category::Navigation, ["Ctrl", "F"], Action::key(&[Ctrl], vk::letter(b'F'))),
        p!("Back", Category::Navigation, ["Alt", "\u{2190}"], Action::key(&[Alt], vk::LEFT)),
        p!("Forward", Category::Navigation, ["Alt", "\u{2192}"], Action::key(&[Alt], vk::RIGHT)),
        p!("Browser back", Category::Navigation, ["\u{21E4}"], Action::key(&[], vk::BROWSER_BACK)),
        p!("Browser forward", Category::Navigation, ["\u{21E5}"], Action::key(&[], vk::BROWSER_FORWARD)),
        p!("Task View", Category::System, ["Win", "Tab"], Action::key(&[Win], vk::TAB)),
        p!("Desktop left", Category::System, ["Ctrl", "Win", "\u{2190}"], Action::key(&[Win, Ctrl], vk::LEFT)),
        p!("Desktop right", Category::System, ["Ctrl", "Win", "\u{2192}"], Action::key(&[Win, Ctrl], vk::RIGHT)),
        p!("Show desktop", Category::System, ["Win", "D"], Action::key(&[Win], vk::letter(b'D'))),
        p!("Volume up", Category::Media, ["Vol+"], Action::key(&[], vk::VOLUME_UP)),
        p!("Volume down", Category::Media, ["Vol\u{2212}"], Action::key(&[], vk::VOLUME_DOWN)),
        p!("Mute", Category::Media, ["Mute"], Action::key(&[], vk::VOLUME_MUTE)),
        p!("Play / pause", Category::Media, ["Play"], Action::key(&[], vk::MEDIA_PLAY_PAUSE)),
        p!("Next track", Category::Media, ["Next"], Action::key(&[], vk::MEDIA_NEXT)),
        p!("Previous track", Category::Media, ["Prev"], Action::key(&[], vk::MEDIA_PREV)),
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    PassThrough,
    Disabled,
    Preset,
    Custom,
}

/// How an action renders in a chip or picker row.
pub struct Display {
    pub label: String,
    pub keys: Vec<String>,
    pub kind: Kind,
}

/// Human name for a virtual-key code, for custom combos.
pub fn key_name(code: u16) -> String {
    match code {
        vk::TAB => "Tab".into(),
        vk::ENTER => "Enter".into(),
        vk::ESCAPE => "Esc".into(),
        vk::LEFT => "\u{2190}".into(),
        vk::RIGHT => "\u{2192}".into(),
        vk::UP => "\u{2191}".into(),
        vk::DOWN => "\u{2193}".into(),
        vk::VOLUME_UP => "Vol+".into(),
        vk::VOLUME_DOWN => "Vol\u{2212}".into(),
        vk::VOLUME_MUTE => "Mute".into(),
        vk::MEDIA_PLAY_PAUSE => "Play".into(),
        vk::MEDIA_NEXT => "Next".into(),
        vk::MEDIA_PREV => "Prev".into(),
        vk::MEDIA_STOP => "Stop".into(),
        0x20 => "Space".into(),
        c @ 0x41..=0x5A => ((c as u8) as char).to_string(),
        c @ 0x70..=0x7B => format!("F{}", c - 0x70 + 1),
        c => format!("{c:#04x}"),
    }
}

/// Render an action the way the design does: presets show their name plus
/// keycaps; custom combos show only keycaps; pass-through and disabled show
/// only a word.
pub fn display(action: &Action) -> Display {
    match action {
        Action::PassThrough => Display {
            label: "Pass through".into(),
            keys: vec![],
            kind: Kind::PassThrough,
        },
        Action::Disabled => Display {
            label: "Disabled".into(),
            keys: vec![],
            kind: Kind::Disabled,
        },
        Action::Key { .. } => {
            if let Some(p) = presets().into_iter().find(|p| &(p.action)() == action) {
                return Display {
                    label: p.label.into(),
                    keys: p.keys.iter().map(|s| s.to_string()).collect(),
                    kind: Kind::Preset,
                };
            }
            // Not a preset: show the raw combo. Modifier order matches the
            // design's builder (Ctrl, Win, Alt, Shift).
            let Action::Key { mods, vk: code } = action else {
                unreachable!()
            };
            let mut keys = Vec::new();
            for (m, name) in [(Ctrl, "Ctrl"), (Win, "Win"), (Alt, "Alt"), (Shift, "Shift")] {
                if mods.contains(&m) {
                    keys.push(name.to_string());
                }
            }
            keys.push(key_name(*code));
            Display {
                label: String::new(),
                keys,
                kind: Kind::Custom,
            }
        }
    }
}
