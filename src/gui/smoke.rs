//! Debug-only, GPU-rendered interaction checks. No hooks or HID writes.

use super::*;
use crate::config::{Action, ButtonMap, Config, Profile};
use crate::hidpp::{device::DeviceStatus, features::reprog::ButtonInfo};
use std::path::PathBuf;

pub fn run() {
    let output = std::env::current_dir().unwrap().join("target/ui-smoke");
    std::fs::create_dir_all(output.join("appdata")).unwrap();
    // All configuration writes stay inside the test output directory.
    std::env::set_var("APPDATA", output.join("appdata"));
    let mut config = Config::default();
    config.default_profile.buttons.back = Action::key(&[Modifier::Ctrl], 0x57);
    config.default_profile.buttons.forward = Action::key(&[Modifier::Ctrl], 0x54);
    config.profiles.push(Profile {
        name: "Preview app".into(),
        match_processes: vec!["preview.exe".into()],
        buttons: ButtonMap::default(),
    });
    state::init(config);
    macroquad::Window::from_config(
        macroquad::conf::Conf {
            miniquad_conf: miniquad::conf::Conf {
                window_title: "Mushak UI smoke test".into(),
                window_width: 968,
                window_height: 668,
                high_dpi: true,
                ..Default::default()
            },
            ..Default::default()
        },
        check(output),
    );
}

async fn draw(ply: &mut Ply<()>, app: &mut App, input: Option<(f32, f32, bool)>) {
    next_frame().await;
    app.poll_key_capture();
    clear_background(app.pal.win.into());
    let (x, y, down) = input.unwrap_or((-100.0, -100.0, false));
    ply.set_layout_dimensions((screen_width(), screen_height()).into());
    ply.pointer_state((x, y).into(), down);
    let mut ui = ply.begin();
    window(&mut ui, app);
    ui.show(|_| {}).await;
}

async fn settle(ply: &mut Ply<()>, app: &mut App) {
    for _ in 0..3 {
        draw(ply, app, None).await;
    }
}

async fn click(ply: &mut Ply<()>, app: &mut App, id: impl Into<Id>) {
    let id = id.into();
    let r = ply
        .bounding_box(id.clone())
        .expect("click target is laid out");
    let (x, y) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
    draw(ply, app, Some((x, y, false))).await;
    draw(ply, app, Some((x, y, true))).await;
    draw(ply, app, Some((x, y, false))).await;
    settle(ply, app).await;
}

fn snapshot(output: &std::path::Path, name: &str) {
    if let Ok(states) = std::env::var("MUSHAK_CAPTURE_STATES") {
        if !states.split(',').any(|state| state == name) {
            return;
        }
    }
    get_screen_data().export_png(output.join(format!("{name}.png")).to_str().unwrap());
}

fn assert_panel_fits(ply: &Ply<()>, app: &App) {
    let p = ply.bounding_box("picker_panel").expect("picker panel");
    assert!(
        p.x >= 0.0
            && p.y >= 0.0
            && p.x + p.width <= screen_width()
            && p.y + p.height <= screen_height()
    );
    let editor = if app.shortcut_editor {
        "assign"
    } else {
        "picker_search"
    };
    for id in [editor, "shortcut_tab", "preset_tab", "picker_close"] {
        let b = ply.bounding_box(id).unwrap();
        assert!(
            b.x >= p.x
                && b.y >= p.y
                && b.x + b.width <= p.x + p.width + 1.0
                && b.y + b.height <= p.y + p.height + 1.0,
            "{id} must fit in the popover: {b:?} in {p:?}"
        );
    }
}

async fn check(output: PathBuf) {
    // Use explicit input while retaining Ply's normal layout and GPU renderer.
    let mut ply = Ply::<()>::new_headless((968.0, 668.0).into());
    fonts::preload().await;
    ply.set_measure_text_function(ply_engine::renderer::create_measure_text_function());
    let mut app = App::new();
    app.conflict_dismissed = true;
    app.device_view.device = DeviceStatus {
        name: Some("Wireless Mouse MX Master 2S".into()),
        connected: true,
        ..Default::default()
    };
    app.device_view.device.capabilities.gestures = true;
    for (slot, cid) in [
        ("middle", 0x0052),
        ("back", 0x0053),
        ("forward", 0x0056),
        ("control_00c4", 0x00c4),
    ] {
        app.device_view
            .device
            .capabilities
            .buttons
            .push(ButtonInfo {
                cid,
                slot: slot.into(),
                name: slot.into(),
            });
    }
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(&mut ply, &mut app).await;
    snapshot(&output, "01-mouse-dark");

    if std::env::args().any(|arg| arg == "--review-tray") {
        crate::tray::preview_menu(
            miniquad::window::windows_hwnd() as isize,
            std::env::args().any(|arg| arg == "--paused"),
        );
        std::process::exit(0);
    }
    if std::env::args().any(|arg| arg == "--review-gallery") {
        review_gallery(&mut ply, &mut app, output).await;
        std::process::exit(0);
    }

    for (id, slot) in [
        ("mouse_middle", "middle"),
        ("mouse_back", "back"),
        ("mouse_forward", "forward"),
        ("mouse_mode", "control_00c4"),
    ] {
        click(&mut ply, &mut app, id).await;
        assert!(
            app.picker == Some(PickerTarget::Button(slot.into())),
            "{id} opens its own mapping"
        );
        assert_panel_fits(&ply, &app);
        snapshot(&output, &format!("02-popover-{slot}"));
        click(&mut ply, &mut app, "picker_close").await;
        assert!(app.picker.is_none(), "Close dismisses the popover");
    }

    click(&mut ply, &mut app, "profile_btn").await;
    click(&mut ply, &mut app, ("profile_opt", 1)).await;
    assert_eq!(app.selected_profile, 1);
    click(&mut ply, &mut app, ("mouse_label", 1)).await;
    assert!(app.picker == Some(PickerTarget::Button("back".into())));
    ply.set_text_value("picker_search", "Copy");
    settle(&mut ply, &mut app).await;
    snapshot(&output, "03-profile-search");
    click(&mut ply, &mut app, ("pk", 0)).await;
    assert!(app.picker.is_none());
    let saved = Config::load_strict().unwrap();
    assert_eq!(
        saved.profiles[0].buttons.back,
        Action::key(&[Modifier::Ctrl], 0x43)
    );
    assert_eq!(
        saved.default_profile.buttons.back,
        Action::key(&[Modifier::Ctrl], 0x57)
    );

    click(&mut ply, &mut app, "mouse_back").await;
    assert!(
        app.picker_query.is_empty(),
        "Each opening clears the previous search"
    );
    click(&mut ply, &mut app, "shortcut_tab").await;
    click(&mut ply, &mut app, ("mod", 2)).await;
    click(&mut ply, &mut app, ("mod", 3)).await;
    click(&mut ply, &mut app, "capture").await;
    assert!(app.capturing_key);
    // Post K only to this test window, through miniquad's Win32 input handler.
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_KEYDOWN, WM_KEYUP};
    let hwnd = HWND(miniquad::window::windows_hwnd());
    unsafe {
        PostMessageW(hwnd, WM_KEYDOWN, WPARAM(0x4b), LPARAM(0x0025_0001)).unwrap();
        PostMessageW(
            hwnd,
            WM_KEYUP,
            WPARAM(0x4b),
            LPARAM(0xc025_0001_u32 as isize),
        )
        .unwrap();
    }
    settle(&mut ply, &mut app).await;
    assert_eq!(app.custom_key, Some(0x4b));
    settle(&mut ply, &mut app).await;
    snapshot(&output, "04-custom-shortcut");
    click(&mut ply, &mut app, "assign").await;
    assert!(app.picker.is_none());
    let saved = Config::load_strict().unwrap();
    assert_eq!(
        saved.profiles[0].buttons.back,
        Action::key(&[Modifier::Alt, Modifier::Shift], 0x4B)
    );

    // Recreate the settings UI from the persisted file, as settings_main does.
    state::set_config(Config::load_strict().unwrap());
    let mut reopened = App::new();
    reopened.device_view = app.device_view.clone();
    reopened.selected_profile = 1;
    reopened.conflict_dismissed = true;
    settle(&mut ply, &mut reopened).await;
    click(&mut ply, &mut reopened, "mouse_back").await;
    assert!(reopened.shortcut_editor);
    assert_eq!(reopened.custom_key, Some(0x4B));
    let prior = reopened.action_for(&PickerTarget::Button("back".into()));
    reopened.custom_key = Some(0x4D);
    click(&mut ply, &mut reopened, "picker_close").await;
    assert_eq!(
        Config::load_strict().unwrap().profiles[0].buttons.back,
        prior
    );

    click(&mut ply, &mut app, "mouse_gesture").await;
    assert!(app.picker == Some(PickerTarget::Gesture("tap".into())));
    snapshot(&output, "05-gesture-tap");
    // A click outside dismisses without changing the binding.
    draw(&mut ply, &mut app, Some((20.0, 600.0, true))).await;
    draw(&mut ply, &mut app, Some((20.0, 600.0, false))).await;
    assert!(app.picker.is_none());

    app.prefs.theme = Theme::Light;
    app.pal = Palette::resolve(app.prefs.theme, app.prefs.accent);
    settle(&mut ply, &mut app).await;
    snapshot(&output, "06-mouse-light");
    click(&mut ply, &mut app, "mouse_forward").await;
    assert_panel_fits(&ply, &app);
    snapshot(&output, "07-popover-light");
    app.close_picker();

    let connected = app.device_view.device.clone();
    for (name, slug) in [
        ("MX Master", "original"),
        ("MX Master 2S", "2s"),
        ("MX Master 3 for Mac", "3-mac"),
        ("MX Master 3 for Business", "3-business"),
        ("MX Master 3S", "3s"),
        ("MX Master 3S for Mac", "3s-mac"),
        ("MX Master 3S for Business", "3s-business"),
        ("MX Master 4", "4"),
        ("MX Master 4 for Mac", "4-mac"),
        ("MX Master 4 for Business", "4-business"),
    ] {
        app.device_view.device = connected.clone();
        app.device_view.device.name = Some(name.into());
        app.device_view.device.capabilities.gesture_control = Some(0x00c3);
        let master4 = name.starts_with("MX Master 4");
        if master4 {
            app.device_view
                .device
                .capabilities
                .buttons
                .push(ButtonInfo {
                    cid: 0x01a0,
                    slot: "control_01a0".into(),
                    name: "Haptic Sense Panel".into(),
                });
        }
        for theme in [Theme::Dark, Theme::Light] {
            app.prefs.theme = theme;
            app.pal = Palette::resolve(theme, app.prefs.accent);
            settle(&mut ply, &mut app).await;
            snapshot(&output, &format!("family-{slug}-{theme:?}"));
            for (id, target) in [
                ("mouse_middle", PickerTarget::Button("middle".into())),
                ("mouse_back", PickerTarget::Button("back".into())),
                ("mouse_forward", PickerTarget::Button("forward".into())),
                ("mouse_mode", PickerTarget::Button("control_00c4".into())),
                ("mouse_gesture", PickerTarget::Gesture("tap".into())),
            ] {
                click(&mut ply, &mut app, id).await;
                assert!(app.picker == Some(target), "{name}: {id} mapping");
                assert_panel_fits(&ply, &app);
                click(&mut ply, &mut app, "picker_close").await;
            }
            if master4 {
                click(&mut ply, &mut app, "mouse_haptic").await;
                assert!(app.picker == Some(PickerTarget::Button("control_01a0".into())));
                assert_panel_fits(&ply, &app);
                snapshot(&output, &format!("family-{slug}-haptic-{theme:?}"));
                ply.set_text_value("picker_search", "Copy");
                settle(&mut ply, &mut app).await;
                click(&mut ply, &mut app, ("pk", 0)).await;
                let saved = Config::load_strict().unwrap();
                assert_eq!(
                    saved.profiles[0].buttons.controls["01a0"],
                    Action::key(&[Modifier::Ctrl], 0x43)
                );
                assert!(!saved.default_profile.buttons.controls.contains_key("01a0"));
            }
        }
        // Disconnecting removes live-only controls for every model.
        app.device_view.device.connected = false;
        app.device_view.device.capabilities = Default::default();
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, "mouse_back").await;
        assert!(app.picker == Some(PickerTarget::Button("back".into())));
        app.close_picker();
    }

    // Firmware can expose a new control before it has an artwork marker.
    app.device_view.device = connected.clone();
    app.device_view.device.name = Some("MX Master 4".into());
    app.device_view
        .device
        .capabilities
        .buttons
        .push(ButtonInfo {
            cid: 0x01ff,
            slot: "control_01ff".into(),
            name: "Additional reported button".into(),
        });
    chrome::make_frameless(DESIGN_W, 850.0);
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, ("mouse_extra", 0x01ff + 3)).await;
    assert!(app.picker == Some(PickerTarget::Button("control_01ff".into())));
    assert_panel_fits(&ply, &app);
    snapshot(&output, "family-extra-control");
    app.close_picker();
    app.device_view.device = connected;

    chrome::make_frameless(DESIGN_W, 420.0);
    settle(&mut ply, &mut app).await;
    let stage_before_scroll = ply.bounding_box("mouse_stage").unwrap();
    let marker_before_scroll = ply.bounding_box("mouse_mode").unwrap();
    // Ply takes a positive distance from the top, then negates it internally.
    ply.set_scroll_position("settings_content", (0.0, 180.0));
    settle(&mut ply, &mut app).await;
    let stage_after_scroll = ply.bounding_box("mouse_stage").unwrap();
    let marker_after_scroll = ply.bounding_box("mouse_mode").unwrap();
    let scrolled = stage_before_scroll.y - stage_after_scroll.y;
    assert!(scrolled > 100.0, "The short window must actually scroll");
    assert!(
        ((marker_before_scroll.y - marker_after_scroll.y) - scrolled).abs() < 1.0,
        "Floating markers must move with the diagram"
    );
    snapshot(&output, "07-scrolled-short-window");
    ply.set_scroll_position("settings_content", (0.0, 0.0));
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(&mut ply, &mut app).await;

    app.device_view.device.connected = false;
    app.device_view.device.capabilities = Default::default();
    settle(&mut ply, &mut app).await;
    snapshot(&output, "08-disconnected");
    click(&mut ply, &mut app, "mouse_middle").await;
    assert!(app.picker == Some(PickerTarget::Button("middle".into())));
    app.close_picker();

    chrome::make_frameless(720.0, 540.0);
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, ("btn_chip", 0)).await;
    assert_panel_fits(&ply, &app);
    snapshot(&output, "09-small-window");
    app.close_picker();
    app.device_view.device.name = Some("MX Anywhere 3S".into());
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(&mut ply, &mut app).await;
    snapshot(&output, "10-other-mouse-list");

    std::fs::write(output.join("result.txt"), "PASS: all five MX Master generations and Mac/Business variants; marker and label selection, correct mappings, haptic panel mapping and profile isolation, additional reported controls, anchored layout, close and outside dismissal, persistence, custom shortcuts, search reset, gestures, both themes, disconnected editing, small-window bounds and other-mouse fallback.\n").unwrap();
    println!("UI smoke checks passed. Screenshots: {}", output.display());
    std::process::exit(0);
}

/// Capture the same production layouts for visual review, using a recorded
/// device report and isolated settings. This path never starts the resident.
async fn review_gallery(ply: &mut Ply<()>, app: &mut App, output: PathBuf) {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/validation/mx-master-2s-bluetooth.json"
    ))
    .unwrap();
    let connected: DeviceStatus = serde_json::from_value(fixture["status"].clone()).unwrap();
    let gallery = output.join("gallery");
    std::fs::create_dir_all(&gallery).unwrap();
    for theme in [Theme::Dark, Theme::Light] {
        app.prefs.theme = theme;
        app.pal = Palette::resolve(theme, app.prefs.accent);
        let dir = gallery.join(format!("{theme:?}"));
        std::fs::create_dir_all(&dir).unwrap();
        app.device_view.device = connected.clone();
        if std::env::args().any(|arg| arg == "--more-states") {
            let dir = gallery.join(format!("{theme:?}"));
            std::fs::create_dir_all(&dir).unwrap();
            extra_states(ply, app, &connected, &dir).await;
            continue;
        }
        app.section = Section::Buttons;
        app.selected_profile = 0;
        settle(ply, app).await;
        snapshot(&dir, "diagram");
        click(ply, app, "profile_btn").await;
        snapshot(&dir, "menu");
        click(ply, app, ("profile_opt", 1)).await;
        click(ply, app, "mouse_back").await;
        assert_panel_fits(ply, app);
        snapshot(&dir, "popover");
        click(ply, app, "category_menu").await;
        assert!(app.picker_categories_open);
        snapshot(&dir, "category-menu");
        click(ply, app, ("action_category", 1)).await;
        assert_eq!(app.picker_category, 1);
        snapshot(&dir, "category-edit");
        click(ply, app, "category_menu").await;
        click(ply, app, ("action_category", 0)).await;
        ply.set_text_value("picker_search", "Copy");
        settle(ply, app).await;
        snapshot(&dir, "search-results");
        click(ply, app, "shortcut_tab").await;
        app.custom_mods = vec![Modifier::Ctrl];
        app.custom_key = Some(0x4B);
        settle(ply, app).await;
        snapshot(&dir, "shortcut");
        click(ply, app, "capture").await;
        snapshot(&dir, "recording");
        click(ply, app, "capture").await;
        assert!(!app.capturing_key && app.custom_key == Some(0x4B));
        click(ply, app, "picker_close").await;
        for (section, name) in [
            (Section::Scroll, "scroll"),
            (Section::Gestures, "gestures"),
            (Section::Profiles, "profiles"),
            (Section::Device, "device"),
        ] {
            click(ply, app, section.id()).await;
            snapshot(&dir, name);
            if matches!(section, Section::Scroll | Section::Gestures) {
                ply.set_scroll_position("settings_content", (0.0, 500.0));
                settle(ply, app).await;
                snapshot(&dir, &format!("{name}-lower"));
                ply.set_scroll_position("settings_content", (0.0, 0.0));
            }
            if section == Section::Profiles {
                click(ply, app, ("p_pick", 1)).await;
                snapshot(&dir, "window-pick");
                click(ply, app, "cancel_window_pick").await;
                assert!(app.pick_deadline.is_none());
            }
        }
        click(ply, app, "hidpp_head").await;
        ply.set_scroll_position("settings_content", (0.0, 0.0));
        settle(ply, app).await;
        snapshot(&dir, "diagnostics");
        click(ply, app, "diagnostics_back").await;
        ply.set_scroll_position("settings_content", (0.0, 0.0));
        app.device_view.device.connected = false;
        app.device_view.device.capabilities = Default::default();
        settle(ply, app).await;
        snapshot(&dir, "disconnected");
        app.device_view.device = DeviceStatus::default();
        settle(ply, app).await;
        snapshot(&dir, "empty");
        click(ply, app, "nav_buttons").await;
        snapshot(&dir, "button-list");
        click(ply, app, ("btn_chip", 0)).await;
        snapshot(&dir, "dialog");
        ply.set_text_value("picker_search", "zzzz");
        settle(ply, app).await;
        snapshot(&dir, "no-results");
        click(ply, app, "picker_close").await;
        app.device_view.device = connected.clone();
        app.device_view.paused = true;
        app.conflict_dismissed = false;
        app.conflict_names = vec!["Logi Options+".into()];
        settle(ply, app).await;
        snapshot(&dir, "alert");
        app.conflict_dismissed = true;
        app.device_view.paused = false;
    }
    println!(
        "Gallery saved to {} at {}x{} logical pixels, {}x scale",
        gallery.display(),
        screen_width(),
        screen_height(),
        miniquad::window::dpi_scale()
    );
}

async fn extra_states(
    ply: &mut Ply<()>,
    app: &mut App,
    connected: &DeviceStatus,
    dir: &std::path::Path,
) {
    app.section = Section::Buttons;
    app.selected_profile = 0;
    for (model, slug) in [("MX Master 3S", "master-3"), ("MX Master 4", "master-4")] {
        app.device_view.device.name = Some(model.into());
        if slug == "master-4" {
            app.device_view
                .device
                .capabilities
                .buttons
                .push(ButtonInfo {
                    name: "Haptic Sense Panel".into(),
                    cid: 0x01a0,
                    slot: "control_01a0".into(),
                });
        }
        settle(ply, app).await;
        snapshot(dir, slug);
    }
    click(ply, app, "mouse_haptic").await;
    snapshot(dir, "haptic");
    app.close_picker();
    app.device_view
        .device
        .capabilities
        .buttons
        .push(ButtonInfo {
            name: "Additional button".into(),
            cid: 0x01ff,
            slot: "control_01ff".into(),
        });
    settle(ply, app).await;
    snapshot(dir, "extra-button");
    app.device_view.device = DeviceStatus {
        name: connected.name.clone(),
        ..Default::default()
    };
    settle(ply, app).await;
    snapshot(dir, "offline-diagram");
    app.device_view.device = connected.clone();
    app.draft.gestures.enabled = false;
    app.section = Section::Gestures;
    settle(ply, app).await;
    snapshot(dir, "gestures-off");
    app.draft.gestures.enabled = true;
    app.draft.device.hires_scroll = false;
    app.draft.device.smartshift = crate::config::SmartShiftMode::AlwaysRatchet;
    app.section = Section::Scroll;
    settle(ply, app).await;
    snapshot(dir, "scroll-limited");
    app.device_view.device = DeviceStatus {
        connected: true,
        name: Some("USB mouse".into()),
        connection: Some("USB".into()),
        settings_unavailable_reason: Some(
            "This mouse reports standard Windows buttons only.".into(),
        ),
        ..Default::default()
    };
    app.section = Section::Device;
    settle(ply, app).await;
    snapshot(dir, "standard-mouse");
    app.conflict_names = vec!["Logi Options+".into()];
    app.conflict_dismissed = false;
    app.kill_outcome = Some(crate::conflicts::KillOutcome {
        killed: 0,
        failed: vec![("Logi Options+".into(), "Access denied".into())],
    });
    settle(ply, app).await;
    snapshot(dir, "conflict-error");
    app.kill_outcome = None;
    app.conflict_dismissed = true;
    app.section = Section::Profiles;
    let profiles = std::mem::take(&mut app.draft.profiles);
    settle(ply, app).await;
    snapshot(dir, "default-only");
    app.draft.profiles = profiles;
    app.draft.profiles.push(Profile {
        name: "Photo editor".into(),
        match_processes: vec!["photo.exe".into()],
        buttons: ButtonMap::default(),
    });
    settle(ply, app).await;
    snapshot(dir, "profile-order");
    app.selected_profile = 2;
    click(ply, app, ("p_up", 2)).await;
    assert_eq!(app.selected_profile, 1);
    assert_eq!(app.draft.profiles[0].name, "Photo editor");
    click(ply, app, ("p_down", 1)).await;
    assert_eq!(app.draft.profiles[1].name, "Photo editor");
    assert_eq!(app.selected_profile, 2);
    app.selected_profile = 0;
    app.draft.profiles.pop();
    app.device_view.device = connected.clone();
    app.section = Section::Buttons;
    chrome::make_frameless(720.0, 540.0);
    settle(ply, app).await;
    snapshot(dir, "compact");
    click(ply, app, ("btn_chip", 0)).await;
    assert_panel_fits(ply, app);
    snapshot(dir, "compact-picker");
    for page in 1..=3 {
        click(ply, app, "next_actions").await;
        assert_eq!(app.picker_page, page);
    }
    click(ply, app, "next_actions").await;
    assert_eq!(app.picker_page, 3);
    click(ply, app, "prev_actions").await;
    assert_eq!(app.picker_page, 2);
    app.close_picker();
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(ply, app).await;
}
