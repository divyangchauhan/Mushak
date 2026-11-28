fn review_frame(dir: &std::path::Path, name: &str) {
    use std::io::Write;
    let image = get_screen_data();
    let mut f = std::fs::File::create(dir.join(format!("{name}.rgba"))).unwrap();
    f.write_all(&image.width.to_le_bytes()).unwrap();
    f.write_all(&image.height.to_le_bytes()).unwrap();
    f.write_all(&image.bytes).unwrap();
}

async fn controls_review(ply: &mut Ply<()>, app: &mut App, output: &std::path::Path) {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/validation/mx-master-2s-bluetooth.json"
    ))
    .unwrap();
    let device: DeviceStatus = serde_json::from_value(fixture["status"].clone()).unwrap();
    let sample = app.draft.clone();
    for theme in [Theme::Dark, Theme::Light] {
        let dir = output.join(format!("{theme:?}"));
        std::fs::create_dir_all(&dir).unwrap();
        app.draft = sample.clone();
        assert!(app.commit_config());
        app.prefs.theme = theme;
        app.pal = Palette::resolve(theme, app.prefs.accent);
        app.selected_profile = 0;
        app.device_view.device = device.clone();
        app.device_view.device.capabilities.smartshift_max_threshold = 50;
        app.device_view.device.capabilities.wheel_multiplier = Some(8);
        app.device_view.device.dpi_list = vec![400, 800, 1000, 1600, 3200];
        app.compact_diagram = false;
        app.section = Section::Buttons;
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        settle(ply, app).await;
        review_frame(&dir, "A1");
        click(ply, app, "profile_btn").await;
        review_frame(&dir, "A2");
        click(ply, app, ("profile_opt", 1)).await;
        chrome::make_frameless(720.0, 540.0);
        settle(ply, app).await;
        review_frame(&dir, "A3");
        click(ply, app, "profile_btn").await;
        let trigger = ply.bounding_box("profile_btn").unwrap();
        let menu = ply.bounding_box("profile_menu").unwrap();
        assert!((menu.y - trigger.y - trigger.height).abs() < 6.0);
        review_frame(&dir, "A5");
        click(ply, app, ("profile_opt", 1)).await;
        app.device_view.device.connected = false;
        settle(ply, app).await;
        review_frame(&dir, "A4");
        app.device_view.device.connected = true;
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        settle(ply, app).await;
        click(ply, app, "wheel_hit").await;
        assert!(app.scrolling_tab);
        review_frame(&dir, "B1");
        let initial = app.wheel_draft.as_ref().unwrap().invert_scroll;
        click(ply, app, ("sw_invert", 1u32)).await;
        assert_ne!(app.wheel_draft.as_ref().unwrap().invert_scroll, initial);
        click(ply, app, "sw_invert").await;
        assert_eq!(app.wheel_draft.as_ref().unwrap().invert_scroll, initial);
        click(ply, app, ("mode", 0)).await;
        review_frame(&dir, "B2");
        click(ply, app, ("wheel_switch_marker", 1u32)).await;
        assert!(app.wheel_switch.is_some());
        review_frame(&dir, "B5");
        click(ply, app, "keep_wheel").await;
        assert!(app.wheel_switch.is_none() && app.scrolling_tab);
        let original = Config::load_strict().unwrap();
        let save = ply.bounding_box("save_wheel").unwrap();
        for _ in 0..3 {
            click(ply, app, "middle_click_tab").await;
            assert!(!app.scrolling_tab);
            let other = ply.bounding_box("save_action").unwrap();
            assert!((other.y - save.y).abs() < 1.1);
            click(ply, app, "scrolling_tab").await;
        }
        click(ply, app, "middle_click_tab").await;
        review_frame(&dir, "B3");
        chrome::make_frameless(720.0, 360.0);
        settle(ply, app).await;
        click(ply, app, "scrolling_tab").await;
        review_frame(&dir, "B4");
        let save = ply.bounding_box("save_wheel").unwrap();
        assert!(save.y + save.height <= screen_height());
        click(ply, app, "cancel_wheel").await;
        assert_eq!(Config::load_strict().unwrap(), original);
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        settle(ply, app).await;
        click(ply, app, "mouse_back").await;
        review_frame(&dir, "C1");
        ply.set_text_value("picker_search", "Copy");
        settle(ply, app).await;
        review_frame(&dir, "C2");
        click(ply, app, "shortcut_tab").await;
        click(ply, app, "capture").await;
        assert!(app.capturing_key);
        unsafe {
            let hwnd = windows::Win32::Foundation::HWND(miniquad::window::windows_hwnd() as _);
            use windows::Win32::{
                Foundation::{LPARAM, WPARAM},
                UI::WindowsAndMessaging::{PostMessageW, WM_KEYDOWN, WM_KEYUP},
            };
            PostMessageW(hwnd, WM_KEYDOWN, WPARAM(0x4b), LPARAM(0x250001)).unwrap();
            PostMessageW(hwnd, WM_KEYUP, WPARAM(0x4b), LPARAM(0xc0250001u32 as isize)).unwrap();
        }
        settle(ply, app).await;
        assert_eq!(app.custom_key, Some(75));
        review_frame(&dir, "C3");
        click(ply, app, ("mod", 0u32)).await;
        assert!(app.custom_mods.contains(&crate::config::Modifier::Ctrl));
        review_frame(&dir, "C5");
        click(ply, app, ("mod", 0u32)).await;
        assert!(app.custom_mods.is_empty());
        chrome::make_frameless(720.0, 360.0);
        settle(ply, app).await;
        review_frame(&dir, "C4");
        click(ply, app, "assign").await;
        let mut expected = original.clone();
        expected.profiles[0].buttons.back = Action::key(&[], 75);
        assert_eq!(Config::load_strict().unwrap(), expected);
        chrome::make_frameless(DESIGN_W, DESIGN_H);
        settle(ply, app).await;
        click(ply, app, "mouse_back").await;
        assert_eq!(app.custom_key, Some(75));
        click(ply, app, "picker_close").await;
        click(ply, app, "mouse_mode").await;
        review_frame(&dir, "D1");
        click(ply, app, "picker_speed_setup").await;
        chrome::make_frameless(720.0, 360.0);
        settle(ply, app).await;
        review_frame(&dir, "D2");
        click(ply, app, "cancel_action").await;
    }
    std::fs::write(output.join("verified.txt"),"Both themes passed profile selection, wheel cycling, resize, cancel, shortcut capture, save and reopen.").unwrap();
}
