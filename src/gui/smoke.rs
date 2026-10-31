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
    get_screen_data().export_png(output.join(format!("{name}.png")).to_str().unwrap());
}

fn assert_panel_fits(ply: &Ply<()>) {
    let p = ply.bounding_box("picker_panel").expect("picker panel");
    assert!(
        p.x >= 0.0
            && p.y >= 0.0
            && p.x + p.width <= screen_width()
            && p.y + p.height <= screen_height()
    );
    for id in ["picker_search", "capture", "assign", "picker_close"] {
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
        assert_panel_fits(&ply);
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
    app.custom_mods = vec![Modifier::Alt, Modifier::Shift];
    app.custom_key = Some(0x4B);
    settle(&mut ply, &mut app).await;
    snapshot(&output, "04-custom-shortcut");
    click(&mut ply, &mut app, "assign").await;
    assert!(app.picker.is_none());
    let saved = Config::load_strict().unwrap();
    assert_eq!(
        saved.profiles[0].buttons.back,
        Action::key(&[Modifier::Alt, Modifier::Shift], 0x4B)
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
    assert_panel_fits(&ply);
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
                assert_panel_fits(&ply);
                click(&mut ply, &mut app, "picker_close").await;
            }
            if master4 {
                click(&mut ply, &mut app, "mouse_haptic").await;
                assert!(app.picker == Some(PickerTarget::Button("control_01a0".into())));
                assert_panel_fits(&ply);
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
        assert_panel_fits(&ply);
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
    assert_panel_fits(&ply);
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
    assert_panel_fits(&ply);
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
        assert_panel_fits(&ply);
        ply.set_text_value("picker_search", "Copy");
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("pk", 0)).await;
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
                cid,
                slot: slot.clone(),
                name: crate::hidpp::features::reprog::control_name(cid),
            });
        chrome::make_frameless(width, 1000.0);
        app.selected_profile = 1;
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("btn_chip", buttons::slot_index(&slot))).await;
        assert!(app.picker == Some(PickerTarget::Button(slot.clone())));
        assert_panel_fits(&ply);
        ply.set_text_value("picker_search", "Copy");
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("pk", 0)).await;
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
        click(&mut ply, &mut app, ("pk", 0)).await;
        assert_eq!(
            Config::load_strict().unwrap().profiles[0]
                .buttons
                .control_action(cid),
            Action::Disabled
        );
        click(&mut ply, &mut app, ("btn_chip", buttons::slot_index(&slot))).await;
        ply.set_text_value("picker_search", "Pass through");
        settle(&mut ply, &mut app).await;
        click(&mut ply, &mut app, ("pk", 0)).await;
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
    click(&mut ply, &mut app, ("pk", 0)).await;
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
    assert_panel_fits(&ply);
    snapshot(&output, "15-device-actions");
    click(&mut ply, &mut app, "picker_wheel_action").await;
    assert_eq!(
        app.action_for(&PickerTarget::Button("control_00c4".into())),
        Action::ShiftWheelMode
    );
    app.open_picker(PickerTarget::Button("control_00c4".into()));
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, "picker_speed_setup").await;
    assert!(app.picker_speed_editor);
    snapshot(&output, "16-pointer-speed-editor");
    click(&mut ply, &mut app, "picker_dpi_action").await;
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
    click(&mut ply, &mut app, ("pk", 0)).await;
    assert_eq!(
        app.action_for(&PickerTarget::Button("control_00c4".into())),
        Action::PassThrough
    );

    std::fs::write(output.join("result.txt"), "PASS: all MX Master generations and Mac/Business variants; markers, popovers, both themes, profile isolation, gesture markers offline, gesture actions in generic/narrow lists, all five gesture actions saved while offline and disabled, DPI Change/Switch and wheel-mode remapping, offline saved controls, pass-through reset, capability gating for connected mice, scrolling and window bounds.\n").unwrap();
    println!("UI smoke checks passed. Screenshots: {}", output.display());
    std::process::exit(0);
}
