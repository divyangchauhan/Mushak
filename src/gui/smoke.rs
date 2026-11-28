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
        gestures: None,
    });
    config.save().expect("write isolated smoke settings");
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
    let animating = app.picker_feedback.is_some();
    for _ in 0..120 {
        if app.picker_feedback.is_none() {
            break;
        }
        draw(ply, app, None).await;
    }
    assert!(
        app.picker_feedback.is_none(),
        "Selection animation must finish"
    );
    if animating {
        settle(ply, app).await;
    }
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
    let mut controls = vec!["shortcut_tab", "preset_tab", "picker_close"];
    if app.shortcut_editor {
        controls.extend(["assign", "cancel_shortcut", "capture"]);
    } else {
        controls.push("picker_search");
        controls.extend(["save_action", "cancel_action"]);
    }
    for id in controls {
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

async fn choose_first_action(ply: &mut Ply<()>, app: &mut App) {
    click(ply, app, ("pk", 0)).await;
    if app.pending_action.is_some() {
        click(ply, app, "save_action").await;
    } else {
        click(ply, app, "cancel_action").await;
    }
}

/// Capture real layout bounds across input transitions. A still image cannot catch a
/// field moving away from the pointer after the first character.
async fn check_editor_sizing(ply: &mut Ply<()>, app: &mut App, output: &std::path::Path) {
    let mut failures = Vec::new();
    let prefs = app.prefs.clone();
    for (name, width, height, control) in [
        ("diagram", DESIGN_W, DESIGN_H, "mouse_middle"),
        ("light", DESIGN_W, DESIGN_H, "mouse_middle"),
        ("thumb", DESIGN_W, DESIGN_H, "mouse_back"),
        ("compact", 720.0, 540.0, "btn_chip"),
    ] {
        app.prefs.theme = if name == "light" {
            Theme::Light
        } else {
            Theme::Dark
        };
        app.pal = Palette::resolve(app.prefs.theme, app.prefs.accent);
        chrome::make_frameless(width, height);
        settle(ply, app).await;
        if control == "btn_chip" {
            click(ply, app, ("btn_chip", 2)).await;
        } else {
            click(ply, app, control).await;
        }
        if app.shortcut_editor {
            click(ply, app, "preset_tab").await;
        }
        let mut ids = vec!["picker_panel", "picker_close", "preset_tab", "shortcut_tab"];
        if name == "thumb" {
            ids.push("thumb_picture");
        } else if control == "mouse_middle" {
            ids.push(control);
        }
        let original: Vec<_> = ids
            .iter()
            .map(|id| ply.bounding_box(*id).unwrap())
            .collect();
        let search = ply.bounding_box("picker_search").unwrap();
        for state in ["actions", "search", "shortcut", "record", "back"] {
            match state {
                "search" => {
                    ply.set_text_value("picker_search", "Copy");
                    settle(ply, app).await;
                    assert_eq!(app.picker_query, "Copy");
                }
                "shortcut" => click(ply, app, "shortcut_tab").await,
                "record" => {
                    click(ply, app, "capture").await;
                    assert!(app.capturing_key);
                }
                "back" => {
                    click(ply, app, "preset_tab").await;
                    assert!(!app.capturing_key && !app.shortcut_editor);
                    assert_eq!(app.picker_query, "Copy");
                }
                _ => (),
            }
            assert_panel_fits(ply, app);
            snapshot(output, &format!("sizing-{name}-{state}"));
            for (id, before) in ids.iter().zip(&original) {
                let after = ply.bounding_box(*id).unwrap();
                if (after.x - before.x).abs() > 1.0
                    || (after.y - before.y).abs() > 1.0
                    || (after.width - before.width).abs() > 1.0
                    || ((!(state == "search" || state == "back") || *id != "picker_panel")
                        && (after.height - before.height).abs() > 1.0)
                {
                    failures.push(format!("{name}/{state}/{id}: {before:?} -> {after:?}"));
                }
            }
            if state == "search" {
                let after = ply.bounding_box("picker_search").unwrap();
                if (after.y - search.y).abs() > 1.0 {
                    failures.push(format!(
                        "{name} search moved from {} to {}",
                        search.y, after.y
                    ));
                }
            }
        }
        click(ply, app, "shortcut_tab").await;
        let target = app.picker.clone().unwrap();
        let before = app.action_for(&target);
        click(ply, app, ("mod", 0)).await;
        click(ply, app, "cancel_shortcut").await;
        assert!(app.picker.is_none());
        assert_eq!(app.action_for(&target), before);
    }
    app.pal = Palette::resolve(prefs.theme, prefs.accent);
    app.prefs = prefs;
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(ply, app).await;
    assert!(
        failures.is_empty(),
        "Editor geometry changed during input: {failures:#?}"
    );
}

async fn check_wheel(ply: &mut Ply<()>, app: &mut App, output: &std::path::Path) {
    let config = app.draft.clone();
    let device = app.device_view.device.clone();
    let profile = app.selected_profile;
    let caps = &mut app.device_view.device.capabilities;
    caps.smartshift = true;
    caps.smartshift_max_threshold = 50;
    caps.hires_scroll = true;
    caps.invert_scroll = true;
    caps.wheel_multiplier = Some(8);
    app.selected_profile = 1;
    assert!(app.commit_config());
    let original = app.draft.clone();
    for (theme, w, h) in [
        (Theme::Dark, DESIGN_W, DESIGN_H),
        (Theme::Light, 720.0, 360.0),
    ] {
        app.prefs.theme = theme;
        app.pal = Palette::resolve(theme, app.prefs.accent);
        chrome::make_frameless(w, h);
        settle(ply, app).await;
        if w >= 840.0 {
            click(ply, app, "wheel_hit").await;
        } else {
            click(ply, app, ("btn_chip", 2)).await;
        }
        assert!(app.scrolling_tab);
        let panel = ply.bounding_box("picker_panel").unwrap();
        let save = ply.bounding_box("save_wheel").unwrap();
        let image = ply.bounding_box("mouse_middle");
        assert!(save.y + save.height <= h);
        snapshot(output, &format!("wheel-scrolling-{theme:?}"));
        click(ply, app, ("mode", 0)).await;
        assert_eq!(Config::load_strict().unwrap(), original);
        if w >= 840.0 {
            assert_ne!(
                app.wheel_draft.as_ref().unwrap().smartshift,
                app.draft.device.smartshift
            );
            click(ply, app, "mouse_forward").await;
            assert!(
                app.wheel_switch.is_some(),
                "clicking a different marker preserves the wheel draft"
            );
            click(ply, app, "keep_wheel").await;
        }

        for _ in 0..3 {
            click(ply, app, "middle_click_tab").await;
            assert!(!app.scrolling_tab);
            let action_save = ply.bounding_box("save_action").unwrap();
            assert!((action_save.y - save.y).abs() < 1.0);
            click(ply, app, "scrolling_tab").await;
            assert!(app.scrolling_tab);
            assert_eq!(ply.bounding_box("picker_panel").unwrap().y, panel.y);
            if let Some(old) = image {
                assert_eq!(ply.bounding_box("mouse_middle").unwrap().x, old.x);
            }
        }
        click(ply, app, "middle_click_tab").await;
        snapshot(output, &format!("wheel-middle-click-{theme:?}"));
        click(ply, app, "cancel_action").await;
        assert_eq!(Config::load_strict().unwrap(), original);
    }
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(ply, app).await;
    click(ply, app, ("mouse_label", 0)).await;
    click(ply, app, ("mode", 0)).await;
    click(ply, app, "middle_click_tab").await;
    click(ply, app, "shortcut_tab").await;
    app.custom_key = Some(75);
    app.custom_mods = vec![Modifier::Ctrl];
    settle(ply, app).await;
    click(ply, app, "scrolling_tab").await;
    click(ply, app, "save_wheel").await;
    let mut expected = original;
    expected.device.smartshift = crate::config::SmartShiftMode::AlwaysRatchet;
    expected.profiles.last_mut().unwrap().buttons.middle = Action::key(&[Modifier::Ctrl], 75);
    assert_eq!(Config::load_strict().unwrap(), expected);
    app.close_picker();
    app.selected_profile = 0;
    app.device_view.device.capabilities.buttons.clear();
    app.device_view.device.name = Some("Wheel-only mouse".into());
    settle(ply, app).await;
    click(ply, app, ("btn_chip", 2)).await;
    assert!(
        app.scrolling_tab
            && !app
                .picker
                .as_ref()
                .unwrap()
                .available(&app.device_view.device)
    );
    click(ply, app, "sw_invert").await;
    click(ply, app, "save_wheel").await;
    expected.device.invert_scroll = !expected.device.invert_scroll;
    assert_eq!(
        Config::load_strict().unwrap(),
        expected,
        "Wheel-only save must preserve every mapping"
    );
    click(ply, app, ("btn_chip", 2)).await;
    click(ply, app, "sw_invert").await;
    let mut offline = app.device_view.clone();
    offline.device.connected = false;
    app.accept_status(offline);
    assert!(app.picker.is_none());
    assert_eq!(
        Config::load_strict().unwrap(),
        expected,
        "Disconnect must discard pending wheel changes"
    );
    app.draft = config;
    app.selected_profile = profile;
    app.device_view.device = device;
    app.prefs.theme = Theme::Dark;
    app.pal = Palette::resolve(Theme::Dark, app.prefs.accent);
    assert!(app.commit_config());
    settle(ply, app).await;
}

include!("controls_review.inc.rs");

async fn check(output: PathBuf) {
    // Use explicit input while retaining Ply's normal layout and GPU renderer.
    let mut ply = Ply::<()>::new_headless((968.0, 668.0).into());
    fonts::preload().await;
    {
        let mut fonts = ply_engine::renderer::FONT_MANAGER.lock().unwrap();
        for _ in 0..90 {
            fonts.clean();
        }
    }
    assert!(ply_engine::renderer::FontManager::is_loaded(
        &fonts::MONO_MEDIUM
    ));
    ply.set_measure_text_function(ply_engine::renderer::create_measure_text_function());
    let mut app = App::new();
    app.conflict_dismissed = true;
    app.device_view.device = DeviceStatus {
        name: Some("Wireless Mouse MX Master 2S".into()),
        connected: true,
        ..Default::default()
    };
    app.device_view.device.capabilities.gestures = true;
    for (slot, name, cid) in [
        ("middle", "Middle click", 0x0052),
        ("back", "Back button", 0x0053),
        ("forward", "Forward button", 0x0056),
        ("control_00c4", "Wheel mode shift", 0x00c4),
    ] {
        app.device_view
            .device
            .capabilities
            .buttons
            .push(ButtonInfo {
                default_task_id: None,
                cid,
                slot: slot.into(),
                name: name.into(),
            });
    }
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(&mut ply, &mut app).await;
    snapshot(&output, "01-mouse-dark");

    if std::env::args().any(|arg| arg == "--review-tooltip") {
        crate::tray::preview_tooltip().await;
        std::process::exit(0);
    }
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

    if std::env::args().any(|a| a == "--controls-review") {
        controls_review(&mut ply, &mut app, &output).await;
        std::process::exit(0);
    }
    check_wheel(&mut ply, &mut app, &output).await;
    check_editor_sizing(&mut ply, &mut app, &output).await;

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
    choose_first_action(&mut ply, &mut app).await;
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
                    default_task_id: None,
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
                choose_first_action(&mut ply, &mut app).await;
                let saved = Config::load_strict().unwrap();
                assert_eq!(
                    saved.profiles[0].buttons.controls["01a0"],
                    Action::key(&[Modifier::Ctrl], 0x43)
                );
                assert!(!saved.default_profile.buttons.controls.contains_key("01a0"));
            }
        }
        // Saved software mappings remain editable when a mouse disconnects.
        app.device_view.device.connected = false;
        app.device_view.device.capabilities = Default::default();
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, "mouse_back").await;
        assert!(app.picker == Some(PickerTarget::Button("back".into())));
        app.close_picker();
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, "mouse_gesture").await;
        assert!(
            app.picker == Some(PickerTarget::Gesture("tap".into())),
            "{name}: offline gesture marker"
        );
        app.close_picker();
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, "mouse_mode").await;
        assert!(
            app.picker == Some(PickerTarget::Button("control_00c4".into())),
            "{name}: offline top button mapping"
        );
        assert_panel_fits(&ply, &app);
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
            default_task_id: None,
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

    // Generic and narrow lists must expose the same gesture mapping as artwork.
    for (name, width, connected) in [
        ("Future HID++ Mouse", DESIGN_W, true),
        ("MX Master 2S", 720.0, true),
        ("MX Master 3S", 720.0, true),
        ("MX Master 4", 720.0, true),
        ("Other mouse", DESIGN_W, false),
    ] {
        app.device_view.device.name = Some(name.into());
        app.device_view.device.connected = connected;
        app.device_view.device.capabilities = Default::default();
        app.device_view.device.capabilities.gestures = connected;
        app.device_view.device.capabilities.gesture_control = connected.then_some(0x00c3);
        chrome::make_frameless(width, 850.0);
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, "gesture_chip").await;
        assert!(
            app.picker == Some(PickerTarget::Gesture("tap".into())),
            "{name}: list gesture action"
        );
        app.close_picker();
    }

    // All five saved actions remain editable offline with recognition disabled.
    app.device_view.device = DeviceStatus {
        name: Some("Wireless Mouse MX Master 2S".into()),
        ..Default::default()
    };
    app.draft.gestures.enabled = false;
    app.commit_config();
    chrome::make_frameless(DESIGN_W, 1000.0);
    app.section = Section::Buttons;
    settle(&mut ply, &mut app).await;
    snapshot(&output, "11-offline-gesture-marker");
    click(&mut ply, &mut app, "mouse_gestures").await;
    assert_eq!(app.section, Section::Gestures);
    snapshot(&output, "12-offline-gesture-settings");
    let back_before = app.draft.profiles[0].buttons.back.clone();
    for (index, slot) in ["tap", "up", "down", "left", "right"]
        .into_iter()
        .enumerate()
    {
        click(&mut ply, &mut app, ("g_chip", index as u32)).await;
        assert!(app.picker == Some(PickerTarget::Gesture(slot.into())));
        assert_panel_fits(&ply, &app);
        ply.set_text_value("picker_search", "Copy");
        settle(&mut ply, &mut app).await;
        choose_first_action(&mut ply, &mut app).await;
        assert!(app.picker.is_none());
    }
    let saved = Config::load_strict().unwrap();
    for action in [
        &saved.profiles[0].gestures.as_ref().unwrap().tap,
        &saved.profiles[0].gestures.as_ref().unwrap().up,
        &saved.profiles[0].gestures.as_ref().unwrap().down,
        &saved.profiles[0].gestures.as_ref().unwrap().left,
        &saved.profiles[0].gestures.as_ref().unwrap().right,
    ] {
        assert_eq!(*action, Action::key(&[Modifier::Ctrl], 0x43));
    }
    assert!(
        !saved.gestures.enabled,
        "Editing does not enable gesture input"
    );
    assert_eq!(saved.profiles[0].buttons.back, back_before);

    // DPI Change, DPI Switch and Master wheel-mode controls share the normal
    // profile picker, including on generic mice and narrow Master layouts.
    app.section = Section::Buttons;
    for (name, width, cid) in [
        ("MX Vertical", DESIGN_W, 0x00edu16),
        ("Future HID++ Mouse", DESIGN_W, 0x00fd),
        ("MX Master 2S", 720.0, 0x00c4),
        ("MX Master 3S", 720.0, 0x00c4),
        ("MX Master 4", 720.0, 0x00c4),
    ] {
        app.device_view.device = DeviceStatus {
            name: Some(name.into()),
            connected: true,
            ..Default::default()
        };
        let slot = format!("control_{cid:04x}");
        app.device_view
            .device
            .capabilities
            .buttons
            .push(ButtonInfo {
                default_task_id: None,
                cid,
                slot: slot.clone(),
                name: crate::hidpp::features::reprog::control_name(cid),
            });
        chrome::make_frameless(width, 1000.0);
        app.selected_profile = 1;
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("btn_chip", buttons::slot_index(&slot))).await;
        assert!(app.picker == Some(PickerTarget::Button(slot.clone())));
        assert_panel_fits(&ply, &app);
        ply.set_text_value("picker_search", "Copy");
        settle(&mut ply, &mut app).await;
        choose_first_action(&mut ply, &mut app).await;
        let saved = Config::load_strict().unwrap();
        assert_eq!(
            saved.profiles[0].buttons.control_action(cid),
            Action::key(&[Modifier::Ctrl], 0x43)
        );
        assert_eq!(
            saved.default_profile.buttons.control_action(cid),
            Action::PassThrough
        );
        assert!(!saved.gestures.enabled);

        app.device_view.device.connected = false;
        app.device_view.device.capabilities = Default::default();
        // A control saved in an application profile can also be configured in
        // the previously unmapped default profile while offline.
        app.selected_profile = 0;
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("btn_chip", buttons::slot_index(&slot))).await;
        assert!(app.picker == Some(PickerTarget::Button(slot.clone())));
        assert_eq!(
            app.action_for(app.picker.as_ref().unwrap()),
            Action::PassThrough
        );
        app.close_picker();
        app.selected_profile = 1;
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("btn_chip", buttons::slot_index(&slot))).await;
        assert!(app.picker == Some(PickerTarget::Button(slot.clone())));
        snapshot(&output, &format!("dpi-{cid:04x}-offline-popover"));
        ply.set_text_value("picker_search", "Disabled");
        settle(&mut ply, &mut app).await;
        choose_first_action(&mut ply, &mut app).await;
        assert_eq!(
            Config::load_strict().unwrap().profiles[0]
                .buttons
                .control_action(cid),
            Action::Disabled
        );
        click(&mut ply, &mut app, ("btn_chip", buttons::slot_index(&slot))).await;
        ply.set_text_value("picker_search", "Pass through");
        settle(&mut ply, &mut app).await;
        choose_first_action(&mut ply, &mut app).await;
        let saved = Config::load_strict().unwrap();
        assert_eq!(
            saved.profiles[0].buttons.control_action(cid),
            Action::PassThrough
        );
        assert!(!saved.profiles[0]
            .buttons
            .controls
            .contains_key(&format!("{cid:04x}")));
    }

    // A dedicated DPI button without a marker stays configurable below artwork.
    chrome::make_frameless(DESIGN_W, 1000.0);
    app.device_view.device = DeviceStatus {
        name: Some("MX Master 2S".into()),
        connected: true,
        ..Default::default()
    };
    app.device_view
        .device
        .capabilities
        .buttons
        .push(ButtonInfo {
            default_task_id: None,
            cid: 0x00ed,
            slot: "control_00ed".into(),
            name: "DPI change button".into(),
        });
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, ("mouse_extra", 0x00ed + 3)).await;
    assert!(app.picker == Some(PickerTarget::Button("control_00ed".into())));
    app.close_picker();

    app.device_view.device.connected = false;
    app.device_view.device.capabilities = Default::default();
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, "mouse_mode").await;
    assert!(app.picker == Some(PickerTarget::Button("control_00c4".into())));
    ply.set_text_value("picker_search", "Copy");
    settle(&mut ply, &mut app).await;
    choose_first_action(&mut ply, &mut app).await;
    snapshot(&output, "14-offline-top-button");
    assert_eq!(
        Config::load_strict().unwrap().profiles[0]
            .buttons
            .control_action(0x00c4),
        Action::key(&[Modifier::Ctrl], 0x43)
    );

    app.section = Section::Buttons;
    app.device_view.device.connected = true;
    app.device_view.device.capabilities = Default::default();
    app.device_view
        .device
        .capabilities
        .buttons
        .push(ButtonInfo {
            default_task_id: None,
            cid: 0x0052,
            slot: "middle".into(),
            name: "Middle click".into(),
        });
    settle(&mut ply, &mut app).await;
    snapshot(&output, "13-unsupported-gesture-hidden");
    // Ply retains old bounds for removed elements. Their former positions must
    // no longer open gesture configuration after an unsupported mouse connects.
    click(&mut ply, &mut app, "mouse_gesture").await;
    assert!(app.picker.is_none());
    click(&mut ply, &mut app, "mouse_mode").await;
    assert!(
        app.picker.is_none(),
        "Saved top-button mapping cannot override connected capability gating"
    );
    click(&mut ply, &mut app, "nav_gestures").await;
    assert_ne!(app.section, Section::Gestures);

    app.section = Section::Buttons;
    app.device_view.device.capabilities.smartshift = true;
    app.device_view.device.capabilities.dpi = true;
    app.device_view.device.dpi_list = vec![400, 800, 1000, 1600, 3200];
    app.open_picker(PickerTarget::Button("control_00c4".into()));
    settle(&mut ply, &mut app).await;
    assert_panel_fits(&ply, &app);
    snapshot(&output, "15-device-actions");
    let before_wheel = Config::load_strict().unwrap();
    click(&mut ply, &mut app, "picker_wheel_action").await;
    assert_eq!(Config::load_strict().unwrap(), before_wheel);
    app.device_view.device.capabilities.smartshift = false;
    settle(&mut ply, &mut app).await;
    assert!(app.pending_action.is_none());
    click(&mut ply, &mut app, "save_action").await;
    assert_eq!(Config::load_strict().unwrap(), before_wheel);
    app.device_view.device.capabilities.smartshift = true;
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, "picker_wheel_action").await;
    click(&mut ply, &mut app, "save_action").await;
    assert!(app.failed_assignment.is_none());
    if app.picker_feedback.is_some() {
        snapshot(&output, "17-wheel-action-feedback");
        while app.picker.is_some() {
            draw(&mut ply, &mut app, None).await;
        }
    }
    assert_eq!(
        app.action_for(&PickerTarget::Button("control_00c4".into())),
        Action::ShiftWheelMode
    );
    app.open_picker(PickerTarget::Button("control_00c4".into()));
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, "picker_speed_setup").await;
    assert!(app.picker_speed_editor);
    snapshot(&output, "16-pointer-speed-editor");
    click(&mut ply, &mut app, "save_action").await;
    assert_eq!(
        app.action_for(&PickerTarget::Button("control_00c4".into())),
        Action::ChangePointerSpeed {
            first: 800,
            second: 1600
        }
    );
    app.open_picker(PickerTarget::Button("control_00c4".into()));
    settle(&mut ply, &mut app).await;
    assert_eq!(app.picker_dpi, [800, 1600]);
    ply.set_text_value("picker_search", "Native action");
    settle(&mut ply, &mut app).await;
    choose_first_action(&mut ply, &mut app).await;
    assert_eq!(
        app.action_for(&PickerTarget::Button("control_00c4".into())),
        Action::PassThrough
    );

    std::fs::write(output.join("result.txt"), "PASS: all MX Master generations and Mac/Business variants; markers, popovers, both themes, profile isolation, gesture markers offline, gesture actions in generic/narrow lists, all five gesture actions saved while offline and disabled, DPI Change/Switch and wheel-mode remapping, offline saved controls, pass-through reset, capability gating for connected mice, scrolling and window bounds.\n").unwrap();
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
        app.device_view.active_profile = Some("Preview app".into());
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
        let back = ply.bounding_box("mouse_back").unwrap();
        for _ in 0..3 {
            draw(
                ply,
                app,
                Some((back.x + back.width / 2.0, back.y + back.height / 2.0, false)),
            )
            .await;
        }
        snapshot(&dir, "marker-hover");
        settle(ply, app).await;
        click(ply, app, "profile_btn").await;
        let caption = ply.bounding_box("profile_caption").unwrap();
        let menu = ply.bounding_box("profile_menu").unwrap();
        assert!(
            menu.x >= caption.x + caption.width || menu.y >= caption.y + caption.height,
            "Profile menu covers its caption"
        );
        let mut navigation = String::from("section,y\n");
        for section in Section::ALL {
            let b = ply.bounding_box(section.id()).unwrap();
            navigation.push_str(&format!("{},{}\n", section.label(), b.y + b.height / 2.0));
        }
        std::fs::write(dir.join("navigation.csv"), navigation).unwrap();
        snapshot(&dir, "menu");
        let option = ply.bounding_box(("profile_opt", 1)).unwrap();
        std::fs::write(
            dir.join("profile-option.csv"),
            format!(
                "x,y\n{},{}\n",
                option.x + option.width / 2.0,
                option.y + option.height / 2.0
            ),
        )
        .unwrap();
        click(ply, app, ("profile_opt", 1)).await;
        click(ply, app, "mouse_back").await;
        assert_panel_fits(ply, app);
        snapshot(&dir, "popover");
        let before_action = Config::load_strict().unwrap();
        let mut centers = String::from("id,x,y\n");
        for (label, id) in [
            ("copy", Id::from(("pk", 2))),
            ("factory", Id::from(("pk", 0))),
            ("common", Id::from(("action_category", 0))),
            ("cancel", Id::from("cancel_action")),
            ("save", Id::from("save_action")),
        ] {
            let rect = ply.bounding_box(id).unwrap();
            centers.push_str(&format!(
                "{label},{},{}\n",
                rect.x + rect.width / 2.0,
                rect.y + rect.height / 2.0
            ));
        }
        std::fs::write(dir.join("action-controls.csv"), centers).unwrap();
        let copy = Action::key(&[Modifier::Ctrl], 0x43);
        click(ply, app, ("pk", 2)).await;
        assert_eq!(app.pending_action, Some(copy.clone()));
        assert_eq!(Config::load_strict().unwrap(), before_action);
        snapshot(&dir, "action-draft");
        click(ply, app, "cancel_action").await;
        assert!(app.picker.is_none());
        assert_eq!(Config::load_strict().unwrap(), before_action);
        click(ply, app, "mouse_back").await;
        click(ply, app, ("pk", 2)).await;
        click(ply, app, "save_action").await;
        let mut expected = before_action.clone();
        expected.profiles[0].buttons.back = copy;
        assert_eq!(Config::load_strict().unwrap(), expected);
        snapshot(&dir, "action-saved");
        click(ply, app, "mouse_back").await;
        click(ply, app, ("action_category", 0)).await;
        click(ply, app, ("pk", 0)).await;
        click(ply, app, "save_action").await;
        assert_eq!(Config::load_strict().unwrap(), before_action);
        app.saved_notice = None;
        click(ply, app, "mouse_mode").await;
        snapshot(&dir, "wheel-actions");
        let wheel_before = Config::load_strict().unwrap();
        click(ply, app, "picker_wheel_action").await;
        assert_eq!(app.pending_action, Some(Action::ShiftWheelMode));
        snapshot(&dir, "wheel-draft");
        click(ply, app, "cancel_action").await;
        assert_eq!(Config::load_strict().unwrap(), wheel_before);
        click(ply, app, "mouse_mode").await;
        let original_search = ply.bounding_box("picker_search").unwrap();
        let original_footer = ply.bounding_box("save_action").unwrap();
        click(ply, app, "browse_wheel_actions").await;
        assert!(app.wheel_presets);
        snapshot(&dir, "wheel-presets");
        let preset_back = ply.bounding_box("mouse_actions_back").unwrap();
        click(ply, app, "mouse_actions_back").await;
        assert!(!app.wheel_presets);
        click(ply, app, "picker_speed_setup").await;
        snapshot(&dir, "dpi-editor");
        chrome::make_frameless(720.0, 360.0);
        settle(ply, app).await;
        assert_panel_fits(ply, app);
        let footer = ply.bounding_box("save_action").unwrap();
        for id in [
            Id::from(("dpi_value", 0)),
            Id::from(("dpi_value", 1)),
            Id::from("dpi_outlook"),
        ] {
            let rect = ply.bounding_box(id).unwrap();
            assert!(
                rect.y + rect.height <= footer.y - 4.0,
                "Speed editor clipped above Save: {rect:?}, {footer:?}"
            );
        }
        snapshot(&dir, "dpi-compact");
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        settle(ply, app).await;
        let speed_search = ply.bounding_box("picker_search").unwrap();
        let speed_footer = ply.bounding_box("save_action").unwrap();
        assert_eq!(
            (original_search.x, original_search.y),
            (speed_search.x, speed_search.y)
        );
        assert_eq!(
            (original_footer.x, original_footer.y),
            (speed_footer.x, speed_footer.y)
        );
        ply.set_text_value(("dpi_value", 0), "9999");
        settle(ply, app).await;
        click(ply, app, "save_action").await;
        assert!(app.picker_speed_editor && app.picker.is_some());
        ply.set_text_value(("dpi_value", 0), "1000");
        settle(ply, app).await;
        assert_eq!(app.picker_dpi[0], 1000);
        let speed_back = ply.bounding_box("mouse_actions_back").unwrap();
        assert_eq!(
            (
                preset_back.x,
                preset_back.y,
                preset_back.width,
                preset_back.height
            ),
            (
                speed_back.x,
                speed_back.y,
                speed_back.width,
                speed_back.height
            ),
            "Back moved between wheel subpages"
        );
        click(ply, app, "mouse_actions_back").await;
        click(ply, app, "picker_close").await;
        click(ply, app, "mouse_back").await;
        click(ply, app, ("action_category", 4)).await;
        snapshot(&dir, "category-menu");
        click(ply, app, ("action_category", 1)).await;
        assert_eq!(app.picker_category, 1);
        snapshot(&dir, "category-edit");
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
        review_sequence(ply, app, &dir).await;
        click(ply, app, "mouse_middle").await;
        snapshot(&dir, "scroll");
        ply.set_scroll_position("wheel_form", (0.0, 500.0));
        settle(ply, app).await;
        snapshot(&dir, "scroll-lower");
        click(ply, app, "picker_close").await;
        for (section, name) in [
            (Section::Gestures, "gestures"),
            (Section::Profiles, "profiles"),
            (Section::Device, "device"),
        ] {
            click(ply, app, section.id()).await;
            snapshot(&dir, name);
            if section == Section::Gestures {
                ply.set_scroll_position("settings_content", (0.0, 500.0));
                settle(ply, app).await;
                snapshot(&dir, &format!("{name}-lower"));
                ply.set_scroll_position("settings_content", (0.0, 0.0));
            }
            if section == Section::Gestures {
                assert_eq!(app.selected_profile, 1);
                let inherited = app.draft.gestures.clone();
                click(ply, app, "sw_gestures").await;
                let config = Config::load_strict().unwrap();
                assert_eq!(config.gestures, inherited);
                assert_eq!(
                    config.profiles[0].gestures.as_ref().unwrap().enabled,
                    !inherited.enabled
                );
                click(ply, app, "gesture_inherit").await;
                assert!(Config::load_strict().unwrap().profiles[0]
                    .gestures
                    .is_none());
            }
            if section == Section::Profiles {
                let before = Config::load_strict().unwrap();
                click(ply, app, ("profile_buttons", 1)).await;
                assert_eq!(app.section, Section::Buttons);
                assert_eq!(app.selected_profile, 1);
                assert_eq!(Config::load_strict().unwrap(), before);
                snapshot(&dir, "profile-buttons");
                click(ply, app, "nav_profiles").await;
                click(ply, app, ("p_pick", 1)).await;
                snapshot(&dir, "window-pick");
                let original_deadline = app.pick_deadline.unwrap();
                click(ply, app, "extend_window_pick").await;
                assert_eq!(
                    app.pick_deadline.unwrap(),
                    original_deadline + Duration::from_secs(6)
                );
                click(ply, app, "cancel_window_pick").await;
                assert!(app.pick_deadline.is_none());
            }
        }
        click(ply, app, "hidpp_head").await;
        ply.set_scroll_position("settings_content", (0.0, 0.0));
        settle(ply, app).await;
        for bounds in ["titlebar", "copy_device_report", "copy_feature_ids"] {
            let box_ = ply.bounding_box(bounds).unwrap();
            assert!(
                box_.x + box_.width <= screen_width() + 0.25,
                "Diagnostics clipped {bounds}"
            );
        }
        snapshot(&dir, "diagnostics");
        ply.set_scroll_position("settings_content", (0.0, 10000.0));
        settle(ply, app).await;
        snapshot(&dir, "diagnostics-lower");
        chrome::make_frameless(720.0, 540.0);
        ply.set_scroll_position("settings_content", (0.0, 0.0));
        settle(ply, app).await;
        for id in ["copy_device_report", "copy_feature_ids"] {
            let rect = ply.bounding_box(id).unwrap();
            assert!(
                rect.x >= 0.0
                    && rect.y >= 0.0
                    && rect.x + rect.width <= screen_width() + 0.25
                    && rect.y + rect.height <= screen_height(),
                "Diagnostics copy action outside compact window"
            );
        }
        snapshot(&dir, "diagnostics-compact");
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        ply.set_scroll_position("settings_content", (0.0, 0.0));
        settle(ply, app).await;
        click(ply, app, "diagnostics_back").await;
        ply.set_scroll_position("settings_content", (0.0, 0.0));
        app.device_view.device.connected = false;
        app.device_view.device.capabilities = Default::default();
        settle(ply, app).await;
        snapshot(&dir, "disconnected");
        app.device_view.device = DeviceStatus::default();
        settle(ply, app).await;
        snapshot(&dir, "empty");
        click(ply, app, "offline_mappings").await;
        assert_eq!(app.section, Section::Buttons);
        snapshot(&dir, "button-list");
        click(ply, app, ("btn_chip", 0)).await;
        snapshot(&dir, "dialog");
        ply.set_text_value("picker_search", "zzzz");
        settle(ply, app).await;
        snapshot(&dir, "no-results");
        click(ply, app, "empty_clear").await;
        assert!(app.picker_query.is_empty());
        click(ply, app, "picker_close").await;
        app.device_view.device = connected.clone();
        let resume_event = crate::single_instance::ResumeEvent::create().unwrap();
        app.device_view.resume_pid = Some(std::process::id());
        app.device_view.paused = true;
        app.conflict_dismissed = false;
        app.conflict_names = vec!["Logi Options+".into()];
        settle(ply, app).await;
        let warned = ply.bounding_box(("mouse_label", 0)).unwrap();
        snapshot(&dir, "alert");
        app.kill_outcome = Some(crate::conflicts::KillOutcome {
            killed: 0,
            failed: vec![("Logi Options+".into(), "Access denied".into())],
        });
        settle(ply, app).await;
        snapshot(&dir, "paused-quit-error");
        click(ply, app, "banner_ignore").await;
        assert!(app.conflict_dismissed && app.device_view.paused);
        let dismissed = ply.bounding_box(("mouse_label", 0)).unwrap();
        assert_eq!(
            (warned.x, warned.y),
            (dismissed.x, dismissed.y),
            "Dismissing a driver warning moved the first mapping"
        );
        snapshot(&dir, "paused-dismissed");
        let resume = ply.bounding_box("resume_remapping").unwrap();
        std::fs::write(
            dir.join("resume-button.csv"),
            format!(
                "x,y\n{},{}\n",
                resume.x + resume.width / 2.0,
                resume.y + resume.height / 2.0
            ),
        )
        .unwrap();
        click(ply, app, "resume_remapping").await;
        assert!(resume_event.take());
        assert!(
            app.device_view.paused,
            "Wait for the resident to confirm resume"
        );
        app.accept_status(SharedStatus {
            paused: false,
            ..app.device_view.clone()
        });
        assert!(matches!(app.resume, ResumeState::Confirmed(_)));
        settle(ply, app).await;
        snapshot(&dir, "resumed");
        drop(resume_event);
        app.accept_status(SharedStatus {
            paused: true,
            ..app.device_view.clone()
        });
        settle(ply, app).await;
        click(ply, app, "resume_remapping").await;
        assert!(matches!(app.resume, ResumeState::Failed));
        let help = ply.bounding_box("resume_help").unwrap();
        std::fs::write(
            dir.join("resume-help.csv"),
            format!(
                "x,y,w,h\n{},{},{},{}\n",
                help.x, help.y, help.width, help.height
            ),
        )
        .unwrap();
        snapshot(&dir, "resume-error");
        let retry_event = crate::single_instance::ResumeEvent::create().unwrap();
        click(ply, app, "resume_remapping").await;
        assert!(retry_event.take(), "Retry must reach the replacement event");
        app.accept_status(SharedStatus {
            resume_pid: None,
            ..app.device_view.clone()
        });
        settle(ply, app).await;
        let show = ply.bounding_box("show_resume_steps").unwrap();
        std::fs::write(
            dir.join("tray-help-button.csv"),
            format!(
                "x,y\n{},{}\n",
                show.x + show.width / 2.0,
                show.y + show.height / 2.0
            ),
        )
        .unwrap();
        click(ply, app, "show_resume_steps").await;
        snapshot(&dir, "resume-legacy");
        click(ply, app, "return_from_tray_help").await;
        chrome::make_frameless(720.0, 360.0);
        settle(ply, app).await;
        let card = ply.bounding_box("rail_device_status").unwrap();
        assert!(
            card.y + card.height <= screen_height(),
            "Short rail cropped the device status card"
        );
        snapshot(&dir, "resume-legacy-compact");
        assert!(!app.resume_steps_visible());
        let mapping = ("btn_chip", buttons::slot_index("forward"));
        let before = ply.bounding_box(mapping).unwrap();
        let show = ply.bounding_box("show_resume_steps").unwrap();
        click(ply, app, "show_resume_steps").await;
        let hide = ply.bounding_box("hide_resume_steps").unwrap();
        assert_eq!(
            (show.x, show.y, show.width, show.height),
            (hide.x, hide.y, hide.width, hide.height)
        );
        let help = ply.bounding_box("resume_help").unwrap();
        let title = ply.bounding_box("titlebar").unwrap();
        assert!(
            help.x >= super::rail_width()
                && help.y >= title.y + title.height
                && help.x + help.width <= screen_width() + 0.25
                && help.y + help.height <= screen_height() + 0.25,
            "Tray help must fit inside the compact content pane"
        );
        assert!(app.resume_steps_visible());
        snapshot(&dir, "resume-compact-steps");
        click(ply, app, "hide_resume_steps").await;
        let after = ply.bounding_box(mapping).unwrap();
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (after.x, after.y, after.width, after.height),
            "Returning from tray help moved the compact mapping"
        );
        assert!(!app.resume_steps_visible());
        let target = PickerTarget::Button("back".into());
        let original = app.action_for(&target);
        app.open_picker(target.clone());
        app.shortcut_editor = true;
        app.custom_key = Some(0x4d);
        settle(ply, app).await;
        assert_panel_fits(ply, app);
        snapshot(&dir, "resume-compact-editor");
        click(ply, app, "assign").await;
        let saved = Config::load_strict().unwrap();
        assert!(matches!(
            saved.profiles[0].buttons.back,
            Action::Key { vk: 0x4d, .. }
        ));
        assert!(app.device_view.paused && app.picker.is_none());
        snapshot(&dir, "paused-saved");
        let forward = ("btn_chip", buttons::slot_index("forward"));
        let row = ply.bounding_box(forward).unwrap();
        let caption = ply.bounding_box("profile_caption").unwrap();
        assert!(
            caption.y + caption.height < row.y && row.y + row.height <= screen_height(),
            "Forward row {row:?} / caption {caption:?} / viewport {} x {}",
            screen_width(),
            screen_height()
        );
        assert!(app.saved_notice.is_some());
        click(ply, app, forward).await;
        assert!(app.picker == Some(PickerTarget::Button("forward".into())));
        let viewport = ply.bounding_box(("category_results", 0)).unwrap();
        for i in 0..6 {
            let category = ply.bounding_box(("action_category", i)).unwrap();
            assert!(category.y + category.height <= viewport.y + viewport.height);
            let item = ply.bounding_box(("pk", i)).unwrap();
            assert!(
                item.y >= viewport.y && item.y + item.height <= viewport.y + viewport.height,
                "Short action picker clips choice {i}: {item:?} in {viewport:?}"
            );
        }
        snapshot(&dir, "next-mapping");
        click(ply, app, "picker_close").await;
        let after = ply.bounding_box(forward).unwrap();
        assert_eq!(
            (row.x, row.y, row.width, row.height),
            (after.x, after.y, after.width, after.height)
        );
        click(ply, app, ("btn_chip", buttons::slot_index("back"))).await;
        assert_eq!(app.custom_key, Some(0x4d));
        click(ply, app, "cancel_shortcut").await;
        assert!(app.picker.is_none());
        app.assign_action(&target, original.clone());
        assert_eq!(app.action_for(&target), original);
        app.saved_notice = None;
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        app.device_view.paused = false;
        app.kill_outcome = None;
        settle(ply, app).await;
        click(ply, app, "mouse_back").await;
        let target = app.picker.clone().unwrap();
        let original = app.action_for(&target);
        use std::os::windows::fs::OpenOptionsExt;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(Config::path().unwrap())
            .unwrap();
        ply.set_text_value("picker_search", "Copy");
        settle(ply, app).await;
        choose_first_action(ply, app).await;
        assert!(app.failed_assignment.is_some());
        assert_eq!(app.action_for(&target), original);
        snapshot(&dir, "save-error");
        drop(lock);
        click(ply, app, "retry_assignment").await;
        assert!(app.picker.is_none() && app.failed_assignment.is_none());
        assert_eq!(
            Config::load_strict().unwrap().profiles[0].buttons.back,
            Action::key(&[Modifier::Ctrl], 0x43)
        );
        app.assign_action(&target, original);
    }
    println!(
        "Gallery saved to {} at {}x{} logical pixels, {}x scale",
        gallery.display(),
        screen_width(),
        screen_height(),
        miniquad::window::dpi_scale()
    );
}

/// Follow one binding through tab changes, a live resize, cancellation and save.
async fn review_sequence(ply: &mut Ply<()>, app: &mut App, dir: &std::path::Path) {
    use std::io::Write;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_KEYDOWN, WM_KEYUP};
    let mut bounds = std::fs::File::create(dir.join("bounds.csv")).unwrap();
    writeln!(bounds, "frame,width,height,scale,control,x,y,w,h").unwrap();
    let mut frame = |ply: &Ply<()>, name: &str| {
        let compact = name == "seq-05-resize";
        let closed = matches!(name, "seq-06-cancel" | "seq-08-save");
        assert_eq!(
            (screen_width(), screen_height()),
            if compact {
                (720.0, 360.0)
            } else {
                (DESIGN_W, DESIGN_H)
            }
        );
        snapshot(dir, name);
        for id in [
            "picker_panel",
            "preset_tab",
            "shortcut_tab",
            "thumb_picture",
            "profile_btn",
            "mouse_back",
            "mouse_stage",
            "picker_search",
            "capture",
            "cancel_shortcut",
            "assign",
        ] {
            // Ply retains the last bounds of controls that have left the tree.
            let visible = match id {
                "profile_btn" => true,
                "mouse_back" | "mouse_stage" => !compact,
                "thumb_picture" => !compact && !closed,
                "picker_search" => name == "seq-01-actions",
                "capture" | "assign" | "cancel_shortcut" => !closed && name != "seq-01-actions",
                _ => !closed,
            };
            if !visible {
                continue;
            }
            if let Some(b) = ply.bounding_box(id) {
                writeln!(
                    bounds,
                    "{name},{},{},{},{id},{},{},{},{}",
                    screen_width(),
                    screen_height(),
                    miniquad::window::dpi_scale(),
                    b.x,
                    b.y,
                    b.width,
                    b.height
                )
                .unwrap();
            }
        }
    };
    let target = PickerTarget::Button("back".into());
    let original = app.action_for(&target);
    click(ply, app, "mouse_back").await;
    if app.shortcut_editor {
        click(ply, app, "preset_tab").await;
    }
    frame(ply, "seq-01-actions");
    let original_tab = ply.bounding_box("shortcut_tab").unwrap();
    for _ in 0..3 {
        click(ply, app, "shortcut_tab").await;
        assert!(app.shortcut_editor);
        click(ply, app, "preset_tab").await;
        assert!(!app.shortcut_editor);
    }
    click(ply, app, "shortcut_tab").await;
    frame(ply, "seq-02-shortcut");
    click(ply, app, "capture").await;
    frame(ply, "seq-03-record");
    let key = |vk, scan: isize| unsafe {
        let hwnd = HWND(miniquad::window::windows_hwnd());
        PostMessageW(hwnd, WM_KEYDOWN, WPARAM(vk), LPARAM((scan << 16) | 1)).unwrap();
        PostMessageW(
            hwnd,
            WM_KEYUP,
            WPARAM(vk),
            LPARAM((scan << 16) | 0xc000_0001u32 as isize),
        )
        .unwrap();
    };
    key(0x4d, 0x32);
    settle(ply, app).await;
    assert_eq!(app.custom_key, Some(0x4d));
    frame(ply, "seq-04-draft");
    click(ply, app, "preset_tab").await;
    let draft_tab = ply.bounding_box("shortcut_tab").unwrap();
    assert_eq!(
        (
            original_tab.x,
            original_tab.y,
            original_tab.width,
            original_tab.height
        ),
        (draft_tab.x, draft_tab.y, draft_tab.width, draft_tab.height)
    );
    snapshot(dir, "draft-actions");
    click(ply, app, "shortcut_tab").await;
    assert_eq!(app.custom_key, Some(0x4d));
    assert_eq!(app.action_for(&target), original);
    snapshot(dir, "draft-return");
    chrome::make_frameless(720.0, 360.0);
    settle(ply, app).await;
    frame(ply, "seq-05-resize");
    assert_panel_fits(ply, app);
    click(ply, app, "cancel_shortcut").await;
    assert!(app.picker.is_none());
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(ply, app).await;
    assert!(app.picker.is_none());
    assert_eq!(app.action_for(&target), original);
    frame(ply, "seq-06-cancel");
    click(ply, app, "mouse_back").await;
    if !app.shortcut_editor {
        click(ply, app, "shortcut_tab").await;
    }
    click(ply, app, "capture").await;
    key(0x4b, 0x25);
    settle(ply, app).await;
    assert_eq!(app.custom_key, Some(0x4b));
    frame(ply, "seq-07-ready");
    click(ply, app, "assign").await;
    let saved = Config::load_strict().unwrap();
    assert!(matches!(
        saved.profiles[0].buttons.back,
        Action::Key { vk: 0x4b, .. }
    ));
    frame(ply, "seq-08-save");
    state::set_config(saved);
    app.prefs.save();
    let mut reopened = App::new();
    reopened.device_view = app.device_view.clone();
    reopened.selected_profile = 1;
    reopened.conflict_dismissed = true;
    settle(ply, &mut reopened).await;
    click(ply, &mut reopened, "mouse_back").await;
    assert!(reopened.shortcut_editor && reopened.custom_key == Some(0x4b));
    frame(ply, "seq-09-reopen");
    reopened.custom_key = Some(0x20);
    reopened.custom_mods = vec![
        Modifier::Ctrl,
        Modifier::Win,
        Modifier::Alt,
        Modifier::Shift,
    ];
    chrome::make_frameless(720.0, 360.0);
    settle(ply, &mut reopened).await;
    assert_panel_fits(ply, &reopened);
    snapshot(dir, "compact-chord");
    click(ply, &mut reopened, "cancel_shortcut").await;
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    app.assign_action(&target, original);
    settle(ply, app).await;
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
                    default_task_id: None,
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
            default_task_id: None,
            name: "Additional button".into(),
            cid: 0x01ff,
            slot: "control_01ff".into(),
        });
    settle(ply, app).await;
    let gestures = ply.bounding_box("mouse_gestures").unwrap();
    assert!(
        gestures.y + gestures.height <= screen_height(),
        "Extra button pushed Configure gestures below the window"
    );
    snapshot(dir, "extra-button");
    click(ply, app, ("mouse_extra", 0x01ff + 3)).await;
    assert!(app.picker == Some(PickerTarget::Button("control_01ff".into())));
    snapshot(dir, "extra-editor");
    click(ply, app, "name_button").await;
    assert!(app.naming_button);
    ply.set_text_value("button_name_input", "Spare button");
    settle(ply, app).await;
    snapshot(dir, "name-control");
    click(ply, app, "save_button_name").await;
    let key = "MX Master 4/control_01ff";
    assert_eq!(
        super::UiPrefs::load()
            .button_names
            .get(key)
            .map(String::as_str),
        Some("Spare button")
    );
    click(ply, app, "picker_close").await;
    snapshot(dir, "named-button");
    click(ply, app, ("mouse_extra", 0x01ff + 3)).await;
    click(ply, app, "name_button").await;
    ply.set_text_value("button_name_input", "");
    settle(ply, app).await;
    click(ply, app, "save_button_name").await;
    assert!(!super::UiPrefs::load().button_names.contains_key(key));
    click(ply, app, "picker_close").await;
    // Same synthetic CID, now with a separately reported firmware task.
    app.device_view
        .device
        .capabilities
        .buttons
        .last_mut()
        .unwrap()
        .default_task_id = Some(0x0003);
    settle(ply, app).await;
    snapshot(dir, "factory-button");
    click(ply, app, ("mouse_extra", 0x01ff + 3)).await;
    snapshot(dir, "factory-editor");
    click(ply, app, "picker_close").await;
    app.device_view.device = DeviceStatus {
        name: connected.name.clone(),
        ..Default::default()
    };
    settle(ply, app).await;
    snapshot(dir, "offline-diagram");
    app.device_view.device = connected.clone();
    app.draft.gestures.enabled = true;
    app.section = Section::Gestures;
    settle(ply, app).await;
    let tap = ply.bounding_box(("g_chip", 0)).unwrap();
    click(ply, app, "sw_gestures").await;
    assert!(!app.selected_gestures().enabled);
    let disabled_tap = ply.bounding_box(("g_chip", 0)).unwrap();
    assert_eq!(
        (tap.x, tap.y),
        (disabled_tap.x, disabled_tap.y),
        "Turning off gestures moved the tap assignment"
    );
    snapshot(dir, "gestures-off");
    let saved_before = Config::load_strict().unwrap();
    ply.set_scroll_position("settings_content", (0.0, 10000.0));
    settle(ply, app).await;
    click(ply, app, "threshold").await;
    assert_eq!(Config::load_strict().unwrap(), saved_before);
    snapshot(dir, "gesture-saved-distance");
    ply.set_scroll_position("settings_content", (0.0, 0.0));
    settle(ply, app).await;
    app.draft.gestures.enabled = true;
    app.draft.device.hires_scroll = false;
    app.draft.device.smartshift = crate::config::SmartShiftMode::AlwaysRatchet;
    app.section = Section::Buttons;
    app.open_picker(PickerTarget::Button("middle".into()));
    settle(ply, app).await;
    snapshot(dir, "scroll-limited");
    app.close_picker();
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
        gestures: None,
    });
    settle(ply, app).await;
    snapshot(dir, "profile-order");
    let original = app.draft.clone();
    click(ply, app, ("rename_profile", 2)).await;
    ply.set_text_value("profile_name_input", "Design tools");
    settle(ply, app).await;
    snapshot(dir, "profile-rename");
    click(ply, app, ("p_down", 1)).await;
    assert_eq!(app.renaming_profile, Some(1));
    assert_eq!(ply.get_text_value("profile_name_input"), "Design tools");
    click(ply, app, ("p_up", 2)).await;
    assert_eq!(app.renaming_profile, Some(2));
    click(ply, app, ("cancel_profile_name", 2)).await;
    assert_eq!(app.draft, original);
    click(ply, app, ("rename_profile", 2)).await;
    ply.set_text_value("profile_name_input", " ");
    click(ply, app, ("save_profile_name", 2)).await;
    assert!(app.profile_name_error.is_some());
    assert_eq!(app.draft, original);
    ply.set_text_value("profile_name_input", "Design tools");
    click(ply, app, ("save_profile_name", 2)).await;
    let mut renamed = original.clone();
    renamed.profiles[1].name = "Design tools".into();
    assert_eq!(Config::load_strict().unwrap(), renamed);
    click(ply, app, ("rename_profile", 2)).await;
    ply.set_text_value("profile_name_input", "Photo editor");
    click(ply, app, ("save_profile_name", 2)).await;
    assert_eq!(Config::load_strict().unwrap(), original);
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
    let first_row = ply.bounding_box(("btn_chip", 0)).unwrap();
    click(ply, app, "compact_diagram_toggle").await;
    assert!(app.compact_diagram);
    snapshot(dir, "compact-diagram");
    click(ply, app, "mouse_back").await;
    assert!(app.picker == Some(PickerTarget::Button("back".into())));
    assert_panel_fits(ply, app);
    click(ply, app, "picker_close").await;
    click(ply, app, "compact_diagram_toggle").await;
    let returned = ply.bounding_box(("btn_chip", 0)).unwrap();
    assert_eq!((first_row.x, first_row.y), (returned.x, returned.y));
    click(ply, app, "mouse_gestures").await;
    assert_eq!(app.section, Section::Gestures);
    for slot in 0..5 {
        let bounds = ply.bounding_box(("g_chip", slot)).unwrap();
        assert!(
            bounds.y >= 46.0 && bounds.y + bounds.height <= 540.0,
            "Gesture {slot} is outside the compact viewport: {bounds:?}"
        );
    }
    let sensitivity = ply.bounding_box("threshold").unwrap();
    assert!(
        sensitivity.y + sensitivity.height <= screen_height(),
        "Compact sensitivity slider is below the viewport: {sensitivity:?}"
    );
    click(ply, app, ("g_chip", 4)).await;
    assert!(app.picker == Some(PickerTarget::Gesture("right".into())));
    click(ply, app, "picker_close").await;
    click(ply, app, "nav_buttons").await;
    click(ply, app, ("btn_chip", 0)).await;
    assert_panel_fits(ply, app);
    snapshot(dir, "compact-picker");
    click(ply, app, ("action_category", 2)).await;
    assert_eq!(app.picker_category, 2);
    assert!(
        ply.bounding_box(("pk", 203)).is_some(),
        "Last Tabs & windows action must be available in compact mode"
    );
    app.close_picker();
    let saved_profiles = app.draft.profiles.clone();
    for name in ["Photo editing and color correction", "Audio tools"] {
        app.draft.profiles.push(Profile {
            name: name.into(),
            match_processes: Vec::new(),
            buttons: ButtonMap::default(),
            gestures: None,
        });
    }
    click(ply, app, "nav_profiles").await;
    let title = ply.bounding_box(("rename_profile", 2)).unwrap();
    let move_up = ply.bounding_box(("p_up", 2)).unwrap();
    let delete = ply.bounding_box(("p_del", 2)).unwrap();
    assert!(
        title.x + title.width <= move_up.x + 0.25,
        "Long profile name overlaps priority commands"
    );
    assert!(delete.x + delete.width <= screen_width());
    click(ply, app, ("rename_profile", 2)).await;
    assert_eq!(
        ply.get_text_value("profile_name_input"),
        "Photo editing and color correction"
    );
    click(ply, app, ("cancel_profile_name", 2)).await;
    app.draft.profiles = saved_profiles;
    app.section = Section::Buttons;
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(ply, app).await;
}
