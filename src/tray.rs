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
    }

    pub fn set_startup(&self, on: bool) {
        self.startup.set_checked(on);
    }

    pub fn set_battery(&self, text: &str) {
        self.battery.set_text(text);
    }
}

/// A simple 32x32 accent-colored dot icon (no external asset needed).
fn make_icon() -> Result<Icon> {
    const N: i32 = 32;
    let mut rgba = vec![0u8; (N * N * 4) as usize];
    let cx = 15.5f32;
    let cy = 15.5f32;
    let r = 14.0f32;
    let (ar, ag, ab) = (0x2Du8, 0x7Du8, 0xD2u8); // accent blue
    for y in 0..N {
        for x in 0..N {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * N + x) * 4) as usize;
            if dist <= r {
                // Slight radial highlight toward the top-left.
                let hl = (1.0 - (dist / r) * 0.5).clamp(0.6, 1.0);
                rgba[idx] = (ar as f32 * hl).min(255.0) as u8;
                rgba[idx + 1] = (ag as f32 * hl).min(255.0) as u8;
                rgba[idx + 2] = (ab as f32 * hl).min(255.0) as u8;
                rgba[idx + 3] = 255;
            } else {
                rgba[idx + 3] = 0; // transparent
            }
        }
    }
    Ok(Icon::from_rgba(rgba, N as u32, N as u32)?)
}
