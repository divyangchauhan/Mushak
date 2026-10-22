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
        ("mx2s_middle", "middle"),
        ("mx2s_back", "back"),
        ("mx2s_forward", "forward"),
        ("mx2s_mode", "control_00c4"),
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
    click(&mut ply, &mut app, ("mx2s_label", 1)).await;
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

    click(&mut ply, &mut app, "mx2s_back").await;
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

    click(&mut ply, &mut app, "mx2s_gesture").await;
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
    click(&mut ply, &mut app, "mx2s_forward").await;
    assert_panel_fits(&ply);
    snapshot(&output, "07-popover-light");
    app.close_picker();

    chrome::make_frameless(DESIGN_W, 420.0);
    settle(&mut ply, &mut app).await;
    ply.set_scroll_position("settings_content", (0.0, -180.0));
    settle(&mut ply, &mut app).await;
    snapshot(&output, "07-scrolled-short-window");
    ply.set_scroll_position("settings_content", (0.0, 0.0));
    chrome::make_frameless(DESIGN_W, DESIGN_H);
    settle(&mut ply, &mut app).await;

    app.device_view.device.connected = false;
    app.device_view.device.capabilities = Default::default();
    settle(&mut ply, &mut app).await;
    snapshot(&output, "08-disconnected");
    click(&mut ply, &mut app, "mx2s_middle").await;
    assert!(app.picker == Some(PickerTarget::Button("middle".into())));
    app.close_picker();

    chrome::make_frameless(720.0, 540.0);
    settle(&mut ply, &mut app).await;
    click(&mut ply, &mut app, ("btn_chip", 0)).await;
    assert_panel_fits(&ply);
    snapshot(&output, "09-small-window");
    app.close_picker();
    app.device_view.device.name = Some("MX Master 3S".into());
    settle(&mut ply, &mut app).await;
    snapshot(&output, "10-other-mouse-list");

    std::fs::write(output.join("result.txt"), "PASS: marker and label selection, correct mappings, anchored layout, close and outside dismissal, profile isolation and persistence, custom shortcuts, search reset, gestures, themes, disconnected editing, small-window bounds.\n").unwrap();
    println!("UI smoke checks passed. Screenshots: {}", output.display());
    std::process::exit(0);
}
