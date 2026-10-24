//! MX Master family artwork with capability-gated button callouts.

use super::{actions, fonts, icons, theme, widgets::Ui2, App, PickerTarget, Section};
use crate::hidpp::device::DeviceStatus;
use ply_engine::prelude::*;

static CLASSIC: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s.png",
    data: include_bytes!("../../assets/devices/mx-master-2s.png"),
};
static MASTER_3: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-3s.png",
    data: include_bytes!("../../assets/devices/mx-master-3s.png"),
};
static MASTER_4: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-4.png",
    data: include_bytes!("../../assets/devices/mx-master-4.png"),
};

const STAGE_WIDTH: f32 = 600.0;
const STAGE_HEIGHT: f32 = 320.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Original,
    Master2S,
    Master3,
    Master3S,
    Master4,
}

impl Model {
    pub fn for_device(device: &DeviceStatus) -> Option<Self> {
        let name = device.display_name().to_ascii_lowercase();
        let words: Vec<_> = name
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let index = words.windows(2).position(|w| w == ["mx", "master"])? + 2;
        match words.get(index).copied() {
            Some("2s") => Some(Self::Master2S),
            Some("3") => Some(Self::Master3),
            Some("3s") => Some(Self::Master3S),
            Some("4") => Some(Self::Master4),
            // Original mice may report a descriptive suffix without a generation.
            None | Some("mouse" | "wireless" | "for" | "business" | "bluetooth") => {
                Some(Self::Original)
            }
            _ => None,
        }
    }

    fn artwork(self) -> &'static GraphicAsset {
        match self {
            Self::Original | Self::Master2S => &CLASSIC,
            Self::Master3 | Self::Master3S => &MASTER_3,
            Self::Master4 => &MASTER_4,
        }
    }

    fn placement(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Original | Self::Master2S => (40.0, -30.0, 420.0, 360.675),
            _ => (0.0, -90.0, 600.0, 518.4),
        }
    }

    fn parts(self) -> &'static [Part] {
        match self {
            Self::Original | Self::Master2S => &CLASSIC_PARTS,
            Self::Master3 | Self::Master3S => &MASTER_3_PARTS,
            Self::Master4 => &MASTER_4_PARTS,
        }
    }
}

#[derive(Clone, Copy)]
struct Part {
    id: &'static str,
    slot: &'static str,
    label: &'static str,
    // Coordinates in the embedded product image, before scaling.
    point: (f32, f32),
    // Label position in the 600 x 320 stage.
    label_at: (f32, f32),
}

const CLASSIC_PARTS: [Part; 5] = [
    Part {
        id: "mouse_middle",
        slot: "middle",
        label: "Middle click",
        point: (180.0, 340.0),
        label_at: (0.0, 20.0),
    },
    Part {
        id: "mouse_back",
        slot: "back",
        label: "Back button",
        point: (560.0, 190.0),
        label_at: (435.0, 20.0),
    },
    Part {
        id: "mouse_forward",
        slot: "forward",
        label: "Forward button",
        point: (530.0, 225.0),
        label_at: (435.0, 115.0),
    },
    Part {
        id: "mouse_gesture",
        slot: "tap",
        label: "Gesture button",
        point: (615.0, 370.0),
        label_at: (435.0, 225.0),
    },
    Part {
        id: "mouse_mode",
        slot: "control_00c4",
        label: "Wheel mode button",
        point: (275.0, 220.0),
        label_at: (0.0, 245.0),
    },
];

const MASTER_3_PARTS: [Part; 5] = [
    Part {
        point: (300.0, 420.0),
        ..CLASSIC_PARTS[0]
    },
    Part {
        point: (625.0, 415.0),
        ..CLASSIC_PARTS[1]
    },
    Part {
        point: (565.0, 465.0),
        ..CLASSIC_PARTS[2]
    },
    Part {
        point: (700.0, 565.0),
        ..CLASSIC_PARTS[3]
    },
    Part {
        point: (420.0, 320.0),
        ..CLASSIC_PARTS[4]
    },
];

const MASTER_4_PARTS: [Part; 6] = [
    Part {
        point: (300.0, 410.0),
        ..CLASSIC_PARTS[0]
    },
    Part {
        point: (650.0, 350.0),
        ..CLASSIC_PARTS[1]
    },
    Part {
        point: (600.0, 400.0),
        ..CLASSIC_PARTS[2]
    },
    Part {
        point: (535.0, 460.0),
        label_at: (0.0, 135.0),
        ..CLASSIC_PARTS[3]
    },
    Part {
        point: (450.0, 290.0),
        ..CLASSIC_PARTS[4]
    },
    Part {
        id: "mouse_haptic",
        slot: "control_01a0",
        label: "Haptic Sense Panel",
        point: (680.0, 500.0),
        label_at: (435.0, 245.0),
    },
];

pub fn matches_device(device: &DeviceStatus) -> bool {
    Model::for_device(device).is_some()
}

impl Part {
    fn target(&self, device: &DeviceStatus) -> Option<PickerTarget> {
        let haptic_gesture = device.capabilities.gesture_control == Some(0x01a0);
        let target = if device.capabilities.gestures
            && ((self.slot == "tap" && !haptic_gesture)
                || (self.slot == "control_01a0" && haptic_gesture))
        {
            PickerTarget::Gesture("tap".into())
        } else if self.slot == "tap" {
            // Some firmware exposes the thumb button as an ordinary control.
            let button = device
                .capabilities
                .buttons
                .iter()
                .find(|button| matches!(button.cid, 0x00c3 | 0x00d0))?;
            PickerTarget::Button(button.slot.clone())
        } else {
            PickerTarget::Button(self.slot.into())
        };
        target.available(device).then_some(target)
    }

    fn point_on_stage(&self, model: Model) -> (f32, f32) {
        let (x, y, width, _) = model.placement();
        let source_width = match model {
            Model::Original | Model::Master2S => 800.0,
            _ => 1000.0,
        };
        (
            x + self.point.0 * width / source_width,
            y + self.point.1 * width / source_width,
        )
    }
}

fn extra_buttons<'a>(
    model: Model,
    device: &'a DeviceStatus,
) -> Vec<&'a crate::hidpp::features::reprog::ButtonInfo> {
    if !device.connected {
        return Vec::new();
    }
    device
        .capabilities
        .buttons
        .iter()
        .filter(|button| {
            !model
                .parts()
                .iter()
                .any(|part| part.target(device) == Some(PickerTarget::Button(button.slot.clone())))
        })
        .collect()
}

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let Some(model) = Model::for_device(&app.device_view.device) else {
        return;
    };
    let (image_x, image_y, image_w, image_h) = model.placement();
    let scale = ui
        .bounding_box("mouse_stage")
        .map(|r| r.width)
        .unwrap_or((screen_width() - 300.0).clamp(240.0, STAGE_WIDTH))
        / STAGE_WIDTH;
    let mut selected = None;

    ui.element()
        .id("mouse_stage")
        .width(grow!())
        .height(fixed!(STAGE_HEIGHT * scale))
        .children(|ui| {
            ui.element()
                .width(fixed!(image_w * scale))
                .height(fixed!(image_h * scale))
                .floating(|f| {
                    f.attach_id("mouse_stage")
                        .clip_by_parent()
                        .offset((image_x * scale, image_y * scale))
                        .passthrough()
                })
                .background_color(theme::WHITE)
                .image(model.artwork())
                .accessibility(|a| {
                    a.image(&format!(
                        "{}, front and thumb side button layout",
                        app.device_view.device.display_name()
                    ))
                })
                .empty();

            for (index, part) in model.parts().iter().enumerate() {
                let Some(target) = part.target(&app.device_view.device) else {
                    continue;
                };
                let action = actions::display(&app.action_for(&target));
                let action_label = if action.label.is_empty() {
                    action.keys.join(" + ")
                } else {
                    action.label
                };
                let active =
                    app.picker.as_ref() == Some(&target) && app.picker_anchor == Some(part.id);
                let label_id: Id = ("mouse_label", index as u32).into();
                let hovered = ui.pointer_over(part.id) || ui.pointer_over(label_id.clone());
                let ink = if active || hovered {
                    pal.accent
                } else {
                    pal.line_strong
                };
                let (x, y) = part.point_on_stage(model);
                let (lx, ly) = part.label_at;
                let label_edge = if lx < x { lx + 155.0 } else { lx };
                let elbow = (x + label_edge) / 2.0;
                let label_y = ly + 25.0;

                // The callout wires use the same coordinates as the image and markers.
                for (segment, (ax, ay, bx, by)) in [
                    (x, y, elbow, y),
                    (elbow, y, elbow, label_y),
                    (elbow, label_y, label_edge, label_y),
                ]
                .into_iter()
                .enumerate()
                {
                    ui.element()
                        .id(("mouse_wire", (index * 3 + segment) as u32))
                        .width(fixed!(((bx - ax).abs() * scale).max(1.0)))
                        .height(fixed!(((by - ay).abs() * scale).max(1.0)))
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .clip_by_parent()
                                .offset((ax.min(bx) * scale, ay.min(by) * scale))
                                .z_index(1)
                                .passthrough()
                        })
                        .background_color(ink)
                        .empty();
                }

                ui.element()
                    .id(part.id)
                    .width(fixed!(24.0 * scale))
                    .height(fixed!(24.0 * scale))
                    .floating(|f| {
                        f.attach_id("mouse_stage")
                            .clip_by_parent()
                            .offset(((x - 12.0) * scale, (y - 12.0) * scale))
                            .z_index(2)
                    })
                    .corner_radius(99.0)
                    .background_color(if active || hovered {
                        pal.accent
                    } else {
                        pal.pop
                    })
                    .border(|b| {
                        b.all(2).color(if active || hovered {
                            pal.accent
                        } else {
                            pal.muted
                        })
                    })
                    .accessibility(|a| {
                        if app.picker.is_none() {
                            a.button(&format!("Configure {}", part.label))
                        } else {
                            a.static_text(part.label)
                        }
                    })
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        if ui.just_pressed() {
                            selected = Some((target.clone(), part.id));
                        }
                        ui.element()
                            .width(fixed!(6.0 * scale))
                            .height(fixed!(6.0 * scale))
                            .corner_radius(99.0)
                            .background_color(if active || hovered {
                                pal.accent_ink
                            } else {
                                pal.text
                            })
                            .empty();
                    });

                ui.element()
                    .id(label_id)
                    .width(fixed!(155.0 * scale))
                    .height(fixed!(52.0 * scale))
                    .floating(|f| {
                        f.attach_id("mouse_stage")
                            .clip_by_parent()
                            .offset((lx * scale, ly * scale))
                            .z_index(2)
                    })
                    .corner_radius(10.0)
                    .background_color(if active || hovered {
                        pal.s2
                    } else {
                        pal.surface
                    })
                    .border(|b| {
                        b.all(1).color(if active || hovered {
                            pal.accent
                        } else {
                            pal.line
                        })
                    })
                    .accessibility(|a| {
                        if app.picker.is_none() {
                            a.button(&format!("{}, {}", part.label, action_label))
                        } else {
                            a.static_text(part.label)
                        }
                    })
                    .layout(|l| l.direction(TopToBottom).gap(3).padding((9, 11, 9, 11)))
                    .children(|ui| {
                        if ui.just_pressed() {
                            selected = Some((target.clone(), part.id));
                        }
                        ui.text(part.label, |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(12).color(pal.text)
                        });
                        let label = if matches!(target, PickerTarget::Gesture(_))
                            && !app.draft.gestures.enabled
                        {
                            "Gestures off"
                        } else {
                            &action_label
                        };
                        ui.text(label, |t| {
                            t.font(&fonts::SANS).font_size(12).color(
                                if app.prefs.theme == theme::Theme::Light {
                                    pal.muted
                                } else {
                                    pal.accent
                                },
                            )
                        });
                    });
            }
        });

    ui.text(
        "Click a button marker or its label to change the action.",
        |t| t.font(&fonts::SANS).font_size(13).color(pal.muted),
    );
    ui.element().height(fixed!(8.0)).empty();
    let note = if app.mouse_awake() {
        "Button mappings use the selected profile. Gesture actions apply to every profile."
    } else {
        "Mouse disconnected. You can still edit the saved Back, Forward, and Middle mappings."
    };
    ui.text(note, |t| {
        t.font(&fonts::SANS).font_size(12).color(pal.faint)
    });

    // Keep newly discovered controls editable even without an artwork marker.
    let extras: Vec<_> = extra_buttons(model, &app.device_view.device)
        .into_iter()
        .map(|button| (button.slot.clone(), button.name.clone()))
        .collect();
    for (slot, name) in extras {
        ui.element().height(fixed!(10.0)).empty();
        let target = PickerTarget::Button(slot.clone());
        let display = actions::display(&app.action_for(&target));
        super::widgets::card(ui, &pal)
            .layout(|l| {
                l.direction(LeftToRight)
                    .align(Left, CenterY)
                    .gap(12)
                    .padding(12)
            })
            .children(|ui| {
                ui.element().width(grow!()).height(fit!()).children(|ui| {
                    ui.text(&name, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                    });
                });
                let index = super::buttons::slot_index(&slot);
                if super::widgets::action_chip(ui, &pal, ("mouse_extra", index).into(), &display) {
                    app.open_picker(target);
                }
            });
    }

    if app.device_view.device.connected && app.device_view.device.capabilities.gestures {
        ui.element().height(fixed!(14.0)).empty();
        let mut gestures = false;
        ui.element()
            .id("mouse_gestures")
            .width(fit!())
            .height(fixed!(34.0))
            .corner_radius(8.0)
            .background_color(pal.surface)
            .border(|b| b.all(1).color(pal.line_strong))
            .accessibility(|a| {
                if app.picker.is_none() {
                    a.button("Configure gestures")
                } else {
                    a.static_text("Configure gestures")
                }
            })
            .layout(|l| l.direction(LeftToRight).gap(8).padding((8, 12, 8, 12)))
            .children(|ui| {
                gestures = ui.just_pressed();
                super::widgets::icon(ui, &icons::NAV_GESTURES, 16.0, pal.muted);
                ui.text("Configure gestures", |t| {
                    t.font(&fonts::SANS_MEDIUM).font_size(12).color(pal.text)
                });
            });
        if gestures {
            app.section = Section::Gestures;
            app.close_picker();
        }
    }
    if let Some((target, anchor)) = selected {
        app.open_button_popover(target, anchor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hidpp::features::reprog::ButtonInfo;

    fn device(name: &str) -> DeviceStatus {
        DeviceStatus {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    #[test]
    fn identifies_every_master_generation_and_variant() {
        for (name, model) in [
            ("MX Master", Model::Original),
            ("MX Master Wireless Mouse", Model::Original),
            ("Wireless Mouse MX Master 2S", Model::Master2S),
            ("mx MASTER 2s Bluetooth Edition", Model::Master2S),
            ("MX Master 3 for Mac", Model::Master3),
            ("Logitech MX Master 3 for Business", Model::Master3),
            ("MX Master 3S", Model::Master3S),
            ("MX Master 3S B", Model::Master3S),
            ("MX Master 3S Bluetooth Edition", Model::Master3S),
            ("MX Master 4 for Mac", Model::Master4),
            ("MX Master 4 for Business", Model::Master4),
        ] {
            assert_eq!(Model::for_device(&device(name)), Some(model), "{name}");
        }
        for name in [
            "MX Master 2",
            "MX Master 2Sx",
            "MX Master 5",
            "MX Master 4S",
            "MX Anywhere 3S",
            "MX Vertical",
            "MX Masterpiece",
            "MX Master Keyboard",
            "",
        ] {
            assert_eq!(Model::for_device(&device(name)), None, "{name}");
        }
    }

    fn add_button(device: &mut DeviceStatus, cid: u16, slot: &str) {
        device.capabilities.buttons.push(ButtonInfo {
            cid,
            slot: slot.into(),
            name: slot.into(),
        });
    }

    #[test]
    fn every_layout_preserves_capability_gating_and_offline_editing() {
        for model in [
            Model::Original,
            Model::Master2S,
            Model::Master3,
            Model::Master3S,
            Model::Master4,
        ] {
            let mut device = DeviceStatus {
                connected: true,
                ..Default::default()
            };
            add_button(&mut device, 0x0052, "middle");
            let slots: Vec<_> = model
                .parts()
                .iter()
                .filter(|p| p.target(&device).is_some())
                .map(|p| p.slot)
                .collect();
            assert_eq!(slots, ["middle"]);
            device.connected = false;
            let slots: Vec<_> = model
                .parts()
                .iter()
                .filter(|p| p.target(&device).is_some())
                .map(|p| p.slot)
                .collect();
            assert_eq!(slots, ["middle", "back", "forward"]);
            for part in model.parts() {
                let (x, y) = part.point_on_stage(model);
                assert!((12.0..STAGE_WIDTH - 12.0).contains(&x));
                assert!((12.0..STAGE_HEIGHT - 12.0).contains(&y));
            }
        }
    }

    #[test]
    fn gesture_and_haptic_targets_follow_the_reported_gesture_control() {
        let mut device = DeviceStatus {
            connected: true,
            ..Default::default()
        };
        device.capabilities.gestures = true;
        device.capabilities.gesture_control = Some(0x00c3);
        add_button(&mut device, 0x01a0, "control_01a0");
        assert!(MASTER_4_PARTS[3].target(&device) == Some(PickerTarget::Gesture("tap".into())));
        assert!(
            MASTER_4_PARTS[5].target(&device) == Some(PickerTarget::Button("control_01a0".into()))
        );
        device.capabilities.gesture_control = Some(0x01a0);
        device.capabilities.buttons.clear();
        add_button(&mut device, 0x00c3, "control_00c3");
        assert!(MASTER_4_PARTS[5].target(&device) == Some(PickerTarget::Gesture("tap".into())));
        assert!(
            MASTER_4_PARTS[3].target(&device) == Some(PickerTarget::Button("control_00c3".into()))
        );
        device.capabilities.gestures = false;
        assert!(
            MASTER_4_PARTS[3].target(&device) == Some(PickerTarget::Button("control_00c3".into()))
        );
        assert!(MASTER_4_PARTS[5].target(&device).is_none());
    }

    #[test]
    fn additional_reported_buttons_are_never_lost() {
        let mut device = DeviceStatus {
            connected: true,
            ..Default::default()
        };
        add_button(&mut device, 0x0052, "middle");
        add_button(&mut device, 0x00c4, "control_00c4");
        add_button(&mut device, 0x01a0, "control_01a0");
        add_button(&mut device, 0x01ff, "control_01ff");
        for model in [
            Model::Original,
            Model::Master2S,
            Model::Master3,
            Model::Master3S,
            Model::Master4,
        ] {
            let extras = extra_buttons(model, &device);
            assert_eq!(
                extras.iter().filter(|b| b.slot == "control_01ff").count(),
                1
            );
            assert_eq!(extras.len(), if model == Model::Master4 { 1 } else { 2 });
        }
        device.connected = false;
        assert!(extra_buttons(Model::Master4, &device).is_empty());
    }
}
