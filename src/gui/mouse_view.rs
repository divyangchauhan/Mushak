//! MX Master family artwork with capability-gated button callouts.

use super::{actions, fonts, icons, theme, widgets::Ui2, App, PickerTarget, Section};
use crate::hidpp::device::DeviceStatus;
use ply_engine::prelude::*;

static CLASSIC: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s.png",
    data: include_bytes!("../../assets/devices/mx-master-2s.png"),
};
// A detail crop of CLASSIC. Source coordinates cover (317, 105) through (665, 390).
static CLASSIC_THUMB: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s-thumb.png",
    data: include_bytes!("../../assets/devices/mx-master-2s-thumb.png"),
};
static MASTER_3: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-3s.png",
    data: include_bytes!("../../assets/devices/mx-master-3s.png"),
};
static MASTER_4: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-4.png",
    data: include_bytes!("../../assets/devices/mx-master-4.png"),
};

static MASTER_4_THUMB: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-4-thumb.png",
    data: include_bytes!("../../assets/devices/mx-master-4-thumb.png"),
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

    pub(super) fn has_thumb_detail(self, anchor: Option<&str>) -> bool {
        matches!(
            (self, anchor),
            (
                Self::Original | Self::Master2S,
                Some("mouse_back" | "mouse_forward")
            ) | (Self::Master4, Some("mouse_haptic"))
        )
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
            Self::Original | Self::Master2S => (1.0, 10.0, 390.0, 334.9125),
            _ => (-67.0, -47.0, 520.0, 449.28),
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
}

const CLASSIC_PARTS: [Part; 5] = [
    Part {
        id: "mouse_middle",
        slot: "middle",
        label: "Middle click",
        point: (180.0, 340.0),
    },
    Part {
        id: "mouse_back",
        slot: "back",
        label: "Back button",
        point: (560.0, 190.0),
    },
    Part {
        id: "mouse_forward",
        slot: "forward",
        label: "Forward button",
        point: (530.0, 225.0),
    },
    Part {
        id: "mouse_gesture",
        slot: "tap",
        label: "Gesture button",
        point: (615.0, 370.0),
    },
    Part {
        id: "mouse_mode",
        slot: "control_00c4",
        label: "Wheel mode button",
        point: (275.0, 220.0),
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
    },
];

pub fn matches_device(device: &DeviceStatus) -> bool {
    Model::for_device(device).is_some()
}

pub(super) fn control_number(device: &DeviceStatus, anchor: &str) -> Option<usize> {
    Model::for_device(device)?
        .parts()
        .iter()
        .position(|part| part.id == anchor)
        .map(|i| i + 1)
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

/// Back and Forward share a narrow strip behind the horizontal wheel on the classic body.
/// The pointer ends at the button, leaving its face visible.
fn thumb_detail(ui: &mut Ui2, app: &App, part: &Part, scale: f32) {
    let pal = app.pal;
    let compact_form = app.shortcut_editor || !app.picker_query.is_empty();
    let width = (if compact_form { 178.0 } else { 211.0 }) * scale;
    let image_height = width * 285.0 / 348.0;
    let (image, left, top, title, caption) = if part.id == "mouse_haptic" {
        (
            &MASTER_4_THUMB,
            486.0,
            303.0,
            "Thumb panel",
            "Below the thumb wheel",
        )
    } else {
        (
            &CLASSIC_THUMB,
            317.0,
            105.0,
            "Thumb buttons",
            "Behind the thumb wheel",
        )
    };
    let point = (
        (part.point.0 - left) * width / 348.0,
        (part.point.1 - top) * width / 348.0,
    );
    let number = control_number(&app.device_view.device, part.id)
        .expect("detail belongs to this model")
        .to_string();
    ui.element()
        .id("thumb_detail")
        .width(fixed!(width))
        .height(fit!())
        .floating(|f| f.attach_id("mouse_stage").offset((9.0, 0.0)))
        .background_color(pal.surface)
        .border(|b| b.all(1).color(pal.line_strong))
        .corner_radius(12.0)
        .layout(|l| l.direction(TopToBottom))
        .children(|ui| {
            ui.element()
                .width(grow!())
                .height(fixed!(45.0))
                .layout(|l| l.align(Left, CenterY).padding((0, 15, 0, 15)))
                .children(|ui| {
                    ui.text(title, |t| {
                        t.font(&fonts::SANS_SEMIBOLD).font_size(17).color(pal.text)
                    });
                });
            ui.element()
                .id("thumb_picture")
                .width(fixed!(width))
                .height(fixed!(image_height))
                .overflow(|o| o.clip())
                .background_color(theme::WHITE)
                .image(image)
                .children(|ui| {
                    ui.element()
                        .width(fixed!(2.0))
                        .height(fixed!(27.0))
                        .floating(|f| {
                            f.attach_id("thumb_picture")
                                .offset((point.0 - 1.0, point.1 - 27.0))
                                .z_index(2)
                        })
                        .background_color(pal.accent)
                        .empty();
                    ui.element()
                        .id(part.id)
                        .width(fixed!(26.0))
                        .height(fixed!(26.0))
                        .floating(|f| {
                            f.attach_id("thumb_picture")
                                .offset((point.0 - 13.0, point.1 - 51.0))
                                .z_index(3)
                        })
                        .accessibility(|a| a.static_text(part.label))
                        .background_color(pal.accent)
                        .corner_radius(13.0)
                        .layout(|l| l.align(CenterX, CenterY))
                        .children(|ui| {
                            ui.text(&number, |t| {
                                t.font(&fonts::MONO_BOLD)
                                    .font_size(14)
                                    .color(pal.accent_ink)
                            });
                        });
                });
            ui.element()
                .width(grow!())
                .height(fixed!(40.0))
                .layout(|l| l.align(Left, CenterY).padding((0, 15, 0, 15)))
                .children(|ui| {
                    ui.text(caption, |t| {
                        t.font(&fonts::SANS).font_size(14).color(pal.muted)
                    });
                });
        });
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
    let extras: Vec<_> = extra_buttons(model, &app.device_view.device)
        .into_iter()
        .map(|button| (button.slot.clone(), button.name.clone(), button.cid))
        .collect();
    let editing = app.picker_anchor.is_some() && screen_width() >= 840.0;
    // Reserve the right column for the editor while keeping every marker on the artwork.
    let factor = if editing && model == Model::Master4 {
        0.70
    } else if editing {
        0.78
    } else if !extras.is_empty() {
        0.92
    } else {
        1.0
    };
    let art_scale = scale * factor;
    let common_actions = editing
        && !app.shortcut_editor
        && app.picker_query.is_empty()
        && app.picker_category == 0
        && !app.picker_categories_open;
    let art_y = (1.0 - factor) * 120.0 * scale + if common_actions { 34.0 } else { 0.0 };
    // Leave room for the separate row when firmware reports an unmapped location.
    let stage_height = if extras.is_empty() {
        STAGE_HEIGHT
    } else {
        284.0
    };
    let row_pitch = if extras.is_empty() { 49.0 } else { 44.0 };
    let mut selected = None;

    ui.element()
        .id("mouse_stage")
        .width(grow!())
        .height(fixed!(stage_height * scale))
        .children(|ui| {
            if editing && model.has_thumb_detail(app.picker_anchor) {
                if let Some(part) = model
                    .parts()
                    .iter()
                    .find(|p| Some(p.id) == app.picker_anchor)
                {
                    thumb_detail(ui, app, part, scale);
                    return;
                }
            }
            ui.element()
                .width(fixed!(image_w * art_scale))
                .height(fixed!(image_h * art_scale))
                .floating(|f| {
                    f.attach_id("mouse_stage")
                        .clip_by_parent()
                        .offset((image_x * art_scale, image_y * art_scale + art_y))
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

            let count = model
                .parts()
                .iter()
                .filter(|p| p.target(&app.device_view.device).is_some())
                .count();
            let mut row = 0;
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
                // Only the active marker has a corresponding control while the editor is open.
                if editing && !active {
                    continue;
                }
                let label_id: Id = ("mouse_label", index as u32).into();
                let hovered = ui.pointer_over(part.id) || ui.pointer_over(label_id.clone());
                let number = (index + 1).to_string();
                let (x, y) = part.point_on_stage(model);
                let lx = 379.0;
                let ly = (stage_height - count as f32 * row_pitch) / 2.0 + row as f32 * row_pitch;
                row += 1;
                ui.element()
                    .id(part.id)
                    .width(fixed!(20.0 * scale))
                    .height(fixed!(20.0 * scale))
                    .floating(|f| {
                        f.attach_id("mouse_stage")
                            .clip_by_parent()
                            .offset((
                                x * art_scale - 10.0 * scale,
                                y * art_scale + art_y - 10.0 * scale,
                            ))
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
                        ui.text(&number, |t| {
                            t.font(&fonts::MONO_BOLD)
                                .font_size(13)
                                .color(if active || hovered {
                                    pal.accent_ink
                                } else {
                                    pal.text
                                })
                        });
                    });

                if editing {
                    // The checked Common action already shows the saved assignment.
                    if !app.shortcut_editor
                        && app.picker_query.is_empty()
                        && app.picker_category == 0
                        && !app.picker_categories_open
                        && matches!(
                            app.action_for(&target),
                            crate::config::Action::PassThrough | crate::config::Action::Disabled
                        )
                    {
                        continue;
                    }
                    ui.element()
                        .width(fixed!(246.0 * scale))
                        .height(fit!())
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .offset((14.0 * scale, 266.0 * scale))
                                .passthrough()
                        })
                        .layout(|l| l.direction(TopToBottom).gap(5).padding((2, 10, 2, 12)))
                        .border(|b| b.left(2).color(pal.accent))
                        .children(|ui| {
                            ui.text("Current assignment", |t| {
                                t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                            });
                            ui.text(&action_label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(16).color(pal.text)
                            });
                        });
                    continue;
                }
                ui.element()
                    .id(label_id)
                    .width(fixed!(217.0 * scale))
                    .height(fixed!((row_pitch - 4.0) * scale))
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
                    .layout(|l| {
                        l.direction(TopToBottom)
                            .align(Left, CenterY)
                            .gap(3)
                            .padding((5, 12, 5, 12))
                    })
                    .children(|ui| {
                        if ui.just_pressed() {
                            selected = Some((target.clone(), part.id));
                        }
                        ui.text(&format!("{number}   {}", part.label), |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                        let label = if matches!(target, PickerTarget::Gesture(_))
                            && !app.draft.gestures.enabled
                        {
                            "Gestures off"
                        } else {
                            &action_label
                        };
                        ui.text(label, |t| {
                            t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.accent)
                        });
                    });
            }
        });

    if editing {
        return;
    }
    if app.device_view.device.connected && app.device_view.device.capabilities.gestures {
        let mut gestures = false;
        ui.element()
            .width(grow!())
            .height(fit!())
            .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(10))
            .children(|ui| {
                ui.element().width(grow!()).children(|ui| {
                    ui.text("Gesture actions apply to all profiles.", |t| {
                        t.font(&fonts::SANS).font_size(13).color(pal.muted)
                    });
                });
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
                            t.font(&fonts::SANS_MEDIUM).font_size(14).color(pal.text)
                        });
                    });
            });
        if gestures {
            app.section = Section::Gestures;
            app.close_picker();
        }
    }
    // Keep newly discovered controls editable even without an artwork marker.
    if !extras.is_empty() {
        ui.element().height(fixed!(10.0)).empty();
        super::widgets::eyebrow(ui, &pal, "Buttons outside the diagram");
    }
    for (slot, name, cid) in extras {
        ui.element().height(fixed!(10.0)).empty();
        let target = PickerTarget::Button(slot.clone());
        let display = actions::display(&app.action_for(&target));
        let index = super::buttons::slot_index(&slot);
        let id: Id = ("mouse_extra", index).into();
        let hovered = ui.pointer_over(id.clone());
        let assignment = if display.label.is_empty() {
            display.keys.join(" + ")
        } else {
            display.label
        };
        ui.element()
            .id(id)
            .width(grow!())
            .height(fixed!(53.0))
            .corner_radius(10.0)
            .background_color(if hovered { pal.s2 } else { pal.surface })
            .border(|b| b.all(1).color(if hovered { pal.accent } else { pal.line }))
            .accessibility(|a| a.button(&format!("Configure {name}, {assignment}")))
            .layout(|l| {
                l.direction(LeftToRight)
                    .align(Left, CenterY)
                    .gap(12)
                    .padding((0, 14, 0, 14))
            })
            .children(|ui| {
                if ui.just_pressed() {
                    app.open_picker(target);
                }
                super::widgets::icon(ui, &icons::NAV_BUTTONS, 19.0, pal.muted);
                ui.element()
                    .width(grow!())
                    .height(fit!())
                    .layout(|l| l.direction(TopToBottom).gap(3))
                    .children(|ui| {
                        ui.text(&format!("{name}   {cid:#06X}"), |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                        });
                        ui.text(&assignment, |t| {
                            t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.accent)
                        });
                    });
                ui.element()
                    .width(fixed!(91.0))
                    .height(fixed!(30.0))
                    .background_color(pal.s2)
                    .corner_radius(7.0)
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        ui.text("Edit action", |t| {
                            t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                        });
                    });
            });
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
