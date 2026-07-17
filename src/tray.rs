//! System tray icon and menu.

use anyhow::Result;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Owns the tray icon (kept alive) and the menu items we update at runtime.
pub struct Tray {
    _icon: TrayIcon,
    open: MenuItem,
    quit: MenuItem,
    pause: CheckMenuItem,
    startup: CheckMenuItem,
    battery: MenuItem,
}

impl Tray {
    pub fn build(start_with_windows: bool) -> Result<Tray> {
        let menu = Menu::new();
        let open = MenuItem::new("Open Settings", true, None);
        let pause = CheckMenuItem::new("Pause remapping", true, false, None);
        // Disabled item used purely as a status label.
        let battery = MenuItem::new("Battery: n/a", false, None);
        let startup = CheckMenuItem::new("Start with Windows", true, start_with_windows, None);
        let quit = MenuItem::new("Quit", true, None);

        menu.append_items(&[
            &open,
            &PredefinedMenuItem::separator(),
            &pause,
            &battery,
            &startup,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        let icon = make_icon()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Mushak")
            .with_icon(icon)
            .build()?;

        Ok(Tray {
            _icon: tray,
            open,
            quit,
            pause,
            startup,
            battery,
        })
    }

    pub fn open_id(&self) -> &MenuId {
        self.open.id()
    }

    pub fn quit_id(&self) -> &MenuId {
        self.quit.id()
    }

    pub fn pause_id(&self) -> &MenuId {
        self.pause.id()
    }

    pub fn startup_id(&self) -> &MenuId {
        self.startup.id()
    }

    pub fn set_paused(&self, paused: bool) {
        self.pause.set_checked(paused);
        // The icon carries the paused state too, so the tray reads correctly
        // without opening the menu.
        match icon_for(paused) {
            Ok(icon) => {
                if let Err(e) = self._icon.set_icon(Some(icon)) {
                    tracing::warn!("swapping tray icon failed: {e}");
                }
            }
            Err(e) => tracing::warn!("building tray icon failed: {e:#}"),
        }
    }

    pub fn set_startup(&self, on: bool) {
        self.startup.set_checked(on);
    }

    pub fn set_battery(&self, text: &str) {
        self.battery.set_text(text);
    }
}

/// A simple 32x32 accent-colored dot icon (no external asset needed).
/// The Modak app icon, rasterised from SVG by `build.rs` as raw 32x32 RGBA.
/// Raw pixels rather than PNG because the resident links no image decoder.
const MODAK_ACTIVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons/modak_active.rgba"));
const MODAK_PAUSED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons/modak_paused.rgba"));
const ICON_PX: u32 = 32;

fn make_icon() -> Result<Icon> {
    icon_for(false)
}

/// Tray icon for the active / paused state. The design ships a distinct paused
/// mark so the tray shows at a glance that remapping is off.
fn icon_for(paused: bool) -> Result<Icon> {
    let rgba = if paused { MODAK_PAUSED } else { MODAK_ACTIVE };
    Ok(Icon::from_rgba(rgba.to_vec(), ICON_PX, ICON_PX)?)
}

