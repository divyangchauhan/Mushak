//! Fixed-angle MX Master 2S artwork with capability-gated button callouts.

use super::{actions, fonts, icons, theme, widgets::Ui2, App, PickerTarget, Section};
use crate::hidpp::device::DeviceStatus;
use ply_engine::prelude::*;

static MOUSE: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s.png",
    data: include_bytes!("../../assets/devices/mx-master-2s.png"),
};

const STAGE_WIDTH: f32 = 600.0;
const STAGE_HEIGHT: f32 = 320.0;

struct Part {
    id: &'static str,
    slot: &'static str,
    label: &'static str,
    // Coordinates in the original 800 x 687 product image.
    point: (f32, f32),
    // Label position in the 600 x 320 stage.
    label_at: (f32, f32),
}

const PARTS: [Part; 5] = [
    Part {
        id: "mx2s_middle",
        slot: "middle",
        label: "Middle click",
        point: (180.0, 340.0),
        label_at: (0.0, 20.0),
    },
    Part {
        id: "mx2s_back",
        slot: "back",
        label: "Back button",
        point: (560.0, 190.0),
        label_at: (435.0, 20.0),
    },
    Part {
        id: "mx2s_forward",
        slot: "forward",
        label: "Forward button",
        point: (530.0, 225.0),
        label_at: (435.0, 115.0),
    },
    Part {
        id: "mx2s_gesture",
        slot: "tap",
        label: "Gesture button",
        point: (615.0, 370.0),
        label_at: (435.0, 225.0),
    },
    Part {
        id: "mx2s_mode",
        slot: "control_00c4",
        label: "Wheel mode button",
        point: (275.0, 220.0),
        label_at: (0.0, 245.0),
    },
];

pub fn matches_device(device: &DeviceStatus) -> bool {
    let name = device.display_name().to_ascii_lowercase();
    let words: Vec<_> = name.split_whitespace().collect();
    words.windows(3).any(|w| w == ["mx", "master", "2s"])
}

impl Part {
    fn target(&self) -> PickerTarget {
        if self.slot == "tap" {
            PickerTarget::Gesture("tap".into())
        } else {
            PickerTarget::Button(self.slot.into())
        }
    }

    fn point_on_stage(&self) -> (f32, f32) {
        (40.0 + self.point.0 * 0.525, -30.0 + self.point.1 * 0.525)
    }
}

pub fn section(ui: &mut Ui2, app: &mut App) {
    let pal = app.pal;
    let scale = ui
        .bounding_box("mx2s_stage")
        .map(|r| r.width)
        .unwrap_or((screen_width() - 300.0).clamp(240.0, STAGE_WIDTH))
        / STAGE_WIDTH;
    let mut selected = None;

    ui.element()
        .id("mx2s_stage")
        .width(grow!())
        .height(fixed!(STAGE_HEIGHT * scale))
        .children(|ui| {
            ui.element()
                .width(fixed!(420.0 * scale))
                .height(fixed!(360.675 * scale))
                .floating(|f| {
                    f.attach_id("mx2s_stage")
                        .clip_by_parent()
                        .offset((40.0 * scale, -30.0 * scale))
                        .passthrough()
                })
                .background_color(theme::WHITE)
                .image(&MOUSE)
                .accessibility(|a| {
                    a.image("Logitech MX Master 2S, viewed from the front and thumb side")
                })
                .empty();

            for (index, part) in PARTS.iter().enumerate() {
                let target = part.target();
                if !target.available(&app.device_view.device) {
                    continue;
                }
                let action = actions::display(&app.action_for(&target));
                let action_label = if action.label.is_empty() {
                    action.keys.join(" + ")
                } else {
                    action.label
                };
                let active =
                    app.picker.as_ref() == Some(&target) && app.picker_anchor == Some(part.id);
                let label_id: Id = ("mx2s_label", index as u32).into();
                let hovered = ui.pointer_over(part.id) || ui.pointer_over(label_id.clone());
                let ink = if active || hovered {
                    pal.accent
                } else {
                    pal.line_strong
                };
                let (x, y) = part.point_on_stage();
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
                        .id(("mx2s_wire", (index * 3 + segment) as u32))
                        .width(fixed!(((bx - ax).abs() * scale).max(1.0)))
                        .height(fixed!(((by - ay).abs() * scale).max(1.0)))
                        .floating(|f| {
                            f.attach_id("mx2s_stage")
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
                        f.attach_id("mx2s_stage")
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
                        f.attach_id("mx2s_stage")
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
                        let label = if part.slot == "tap" && !app.draft.gestures.enabled {
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

    if app.device_view.device.connected && app.device_view.device.capabilities.gestures {
        ui.element().height(fixed!(14.0)).empty();
        let mut gestures = false;
        ui.element()
            .id("mx2s_gestures")
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

    #[test]
    fn artwork_only_matches_the_2s() {
        for name in [
            "MX Master 2S",
            "Wireless Mouse MX Master 2S",
            "mx MASTER 2s",
        ] {
            assert!(matches_device(&DeviceStatus {
                name: Some(name.into()),
                ..Default::default()
            }));
        }
        for name in [
            "MX Master 3S",
            "MX Master 2",
            "MX Anywhere 2S",
            "MX Master 2Sx",
            "",
        ] {
            assert!(!matches_device(&DeviceStatus {
                name: Some(name.into()),
                ..Default::default()
            }));
        }
    }

    #[test]
    fn connected_callouts_follow_reported_controls() {
        let mut device = DeviceStatus {
            connected: true,
            ..Default::default()
        };
        device.capabilities.buttons.push(ButtonInfo {
            cid: 0x0052,
            slot: "middle".into(),
            name: "Middle click".into(),
        });
        let visible: Vec<_> = PARTS
            .iter()
            .filter(|p| p.target().available(&device))
            .map(|p| p.slot)
            .collect();
        assert_eq!(visible, ["middle"]);
        device.capabilities.gestures = true;
        assert!(PARTS[3].target().available(&device));
        assert!(!PARTS[4].target().available(&device));
        device.connected = false;
        let visible: Vec<_> = PARTS
            .iter()
            .filter(|p| p.target().available(&device))
            .map(|p| p.slot)
            .collect();
        assert_eq!(visible, ["middle", "back", "forward"]);
    }
}
