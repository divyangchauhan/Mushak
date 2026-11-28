//! System tray icon and menu.

use anyhow::Result;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Owns the tray icon (kept alive) and the menu items we update at runtime.
pub struct Tray {
    icon: TrayIcon,
    last_status: std::cell::RefCell<String>,
    items: TrayMenu,
}

struct TrayMenu {
    menu: Menu,
    open: MenuItem,
    quit: MenuItem,
    pause: MenuItem,
    startup: CheckMenuItem,
}

impl TrayMenu {
    fn set_paused(&self, paused: bool) {
        self.pause.set_text(if paused {
            "Resume remapping"
        } else {
            "Pause remapping"
        });
    }

    fn new(start_with_windows: bool) -> Result<Self> {
        let menu = Menu::new();
        let open = MenuItem::new("Open mouse settings", true, None);
        let pause = MenuItem::new("Pause remapping", true, None);
        let startup = CheckMenuItem::new("Start with Windows", true, start_with_windows, None);
        let quit = MenuItem::new("Quit Mushak", true, None);

        menu.append_items(&[
            &open,
            &PredefinedMenuItem::separator(),
            &pause,
            &startup,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        Ok(Self {
            menu,
            open,
            pause,
            startup,
            quit,
        })
    }
}

#[cfg(debug_assertions)]
pub fn preview_menu(hwnd: isize, paused: bool) {
    use tray_icon::menu::ContextMenu;
    let items = TrayMenu::new(true).unwrap();
    items.set_paused(paused);
    assert_eq!(
        items.pause.text(),
        if paused {
            "Resume remapping"
        } else {
            "Pause remapping"
        }
    );
    // The smoke window owns this popup. It has no resident handler.
    unsafe {
        items.menu.show_context_menu_for_hwnd(
            hwnd,
            Some(tray_icon::dpi::LogicalPosition::new(30.0, 70.0).into()),
        )
    };
}

#[cfg(debug_assertions)]
pub async fn preview_tooltip() {
    let tray = Tray::build(false).unwrap();
    tray.set_battery("Battery: 50%");
    let start = std::time::Instant::now();
    let mut reported = false;
    while start.elapsed().as_secs() < 12 {
        if !reported {
            if let Some(rect) = tray.icon.rect() {
                println!(
                    "ICON {} {} {} {}",
                    rect.position.x, rect.position.y, rect.size.width, rect.size.height
                );
                reported = true;
            }
        }
        ply_engine::prelude::next_frame().await;
    }
}

impl Tray {
    pub fn build(start_with_windows: bool) -> Result<Tray> {
        let items = TrayMenu::new(start_with_windows)?;
        let icon = make_icon()?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(items.menu.clone()))
            .with_tooltip("Mushak")
            .with_icon(icon)
            .build()?;

        Ok(Tray {
            icon: tray,
            last_status: Default::default(),
            items,
        })
    }

    pub fn open_id(&self) -> &MenuId {
        self.items.open.id()
    }

    pub fn quit_id(&self) -> &MenuId {
        self.items.quit.id()
    }

    pub fn pause_id(&self) -> &MenuId {
        self.items.pause.id()
    }

    pub fn startup_id(&self) -> &MenuId {
        self.items.startup.id()
    }

    pub fn set_paused(&self, paused: bool) {
        self.items.set_paused(paused);
        // The icon carries the paused state too, so the tray reads correctly
        // without opening the menu.
        match icon_for(paused) {
            Ok(icon) => {
                if let Err(e) = self.icon.set_icon(Some(icon)) {
                    tracing::warn!("swapping tray icon failed: {e}");
                }
            }
            Err(e) => tracing::warn!("building tray icon failed: {e:#}"),
        }
    }

    pub fn set_startup(&self, on: bool) {
        self.items.startup.set_checked(on);
    }

    pub fn set_battery(&self, text: &str) {
        if self.last_status.borrow().as_str() != text {
            let tooltip = format!("Mushak\n{text}");
            if self.icon.set_tooltip(Some(tooltip)).is_ok() {
                self.last_status.replace(text.to_string());
            }
        }
    }
}

/// The Modak app icon, rasterised from SVG by `build.rs` as raw 32x32 RGBA.
/// Raw pixels rather than PNG because the resident links no image decoder.
const MODAK_ACTIVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons/modak_active_32.rgba"));
const MODAK_PAUSED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons/modak_paused_32.rgba"));
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
