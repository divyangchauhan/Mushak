//! MX Master family artwork with capability-gated button callouts.

use super::{
    actions, fonts, icons, theme,
    widgets::{self, Ui2},
    App, PickerTarget, Section,
};
use crate::hidpp::device::DeviceStatus;
use ply_engine::prelude::*;

static CLASSIC: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s.png",
    data: include_bytes!(concat!(env!("OUT_DIR"), "/mx-master-2s-diagram.png")),
};
static CLASSIC_DARK: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s-dark.png",
    data: include_bytes!(concat!(env!("OUT_DIR"), "/mx-master-2s-dark.png")),
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
    data: include_bytes!(concat!(env!("OUT_DIR"), "/mx-master-4-diagram.png")),
};

static MASTER_4_THUMB: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-4-thumb.png",
    data: include_bytes!("../../assets/devices/mx-master-4-thumb.png"),
};

static CLASSIC_WHEEL: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-2s-wheel.png",
    data: include_bytes!(concat!(env!("OUT_DIR"), "/mx-master-2s-wheel.png")),
};
static MASTER_3_WHEEL: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-3s-wheel.png",
    data: include_bytes!(concat!(env!("OUT_DIR"), "/mx-master-3s-wheel.png")),
};
static MASTER_4_WHEEL: GraphicAsset = GraphicAsset::Bytes {
    file_name: "mx-master-4-wheel.png",
    data: include_bytes!(concat!(env!("OUT_DIR"), "/mx-master-4-wheel.png")),
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

    pub(super) fn has_detail(self, anchor: Option<&str>) -> bool {
        anchor == Some("mouse_mode")
            || matches!(
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
        label: "Gesture button tap",
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

/// Look up the diagram number for a mapping opened from the compact list.
pub(super) fn target_number(device: &DeviceStatus, target: &PickerTarget) -> Option<usize> {
    Model::for_device(device)?
        .parts()
        .iter()
        .position(|part| part.target(device).as_ref() == Some(target))
        .map(|index| index + 1)
}

impl Part {
    fn target(&self, device: &DeviceStatus) -> Option<PickerTarget> {
        if !device.connected && self.slot.starts_with("control_") && self.slot != "control_00c4" {
            return None;
        }
        let haptic_gesture = device.capabilities.gesture_control == Some(0x01a0);
        let target = if (self.slot == "tap" && !device.connected)
            || device.capabilities.gestures
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
        target.can_open(device).then_some(target)
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

fn extra_buttons(
    model: Model,
    device: &DeviceStatus,
    saved: &crate::config::Config,
) -> Vec<crate::hidpp::features::reprog::ButtonInfo> {
    super::buttons::editable_buttons(device, saved)
        .into_iter()
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
fn button_detail(ui: &mut Ui2, app: &App, model: Model, part: &Part, scale: f32) {
    let pal = app.pal;
    let width = 211.0 * scale;
    let image_height = width * 285.0 / 348.0;
    let (image, left, top, title, caption) = if part.id == "mouse_mode" {
        let (image, left, top) = match model {
            Model::Original | Model::Master2S => (&CLASSIC_WHEEL, 84.0, 106.0),
            Model::Master3 | Model::Master3S => (&MASTER_3_WHEEL, 230.0, 192.0),
            Model::Master4 => (&MASTER_4_WHEEL, 250.0, 163.0),
        };
        (image, left, top, "Wheel controls", "Behind the main wheel")
    } else if part.id == "mouse_haptic" {
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

/// Forward marker clicks through the anchored editor's modal backdrop.
pub(super) fn wheel_marker_input(ui: &mut Ui2, app: &App) -> Option<(PickerTarget, &'static str)> {
    let device = &app.device_view.device;
    let model = Model::for_device(device)?;
    let mut next = None;
    for (index, part) in model.parts().iter().enumerate() {
        if Some(part.id) == app.picker_anchor {
            continue;
        }
        let Some(target) = part.target(device) else {
            continue;
        };
        let Some(r) = ui.bounding_box(part.id) else {
            continue;
        };
        ui.element()
            .id(("wheel_switch_marker", index as u32))
            .width(fixed!(r.width))
            .height(fixed!(r.height))
            .floating(|f| f.attach_root().offset((r.x, r.y)).z_index(103))
            .accessibility(|a| a.button(part.label))
            .children(|ui| {
                if ui.just_pressed() {
                    next = Some((target, part.id));
                }
            });
    }
    next
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
    let extras: Vec<_> = extra_buttons(model, &app.device_view.device, &app.draft)
        .into_iter()
        .map(|button| (button.slot.clone(), button.name.clone(), button.cid))
        .collect();
    let editing = app.picker_anchor.is_some() && screen_width() >= 840.0;
    // Reserve the right column for the editor while keeping every marker on the artwork.
    let wheel_editor = app.wheel_draft.is_some();
    let reserve_editor_space =
        screen_width() >= 840.0 && (editing || super::scroll::available(&app.device_view.device));
    let factor = if reserve_editor_space {
        if model == Model::Master4 {
            0.70
        } else {
            0.77
        }
    } else {
        1.0
    };
    let art_scale = scale * factor;
    let art_y = ((1.0 - factor) * 120.0 - if reserve_editor_space { 0.0 } else { 14.0 }) * scale;
    // Leave room for the separate row when firmware reports an unmapped location.
    let count = model
        .parts()
        .iter()
        .filter(|p| p.target(&app.device_view.device).is_some())
        .count();
    let list_left = if reserve_editor_space { 324.0 } else { 379.0 };
    let list_width = STAGE_WIDTH - list_left - 4.0;
    let row_pitch = if model == Model::Master4 { 48.0 } else { 55.0 };
    let extra_heading = if extras.is_empty() { 0.0 } else { 21.0 };
    let list_height = (count + extras.len()) as f32 * row_pitch + extra_heading;
    let list_top = ((STAGE_HEIGHT - count as f32 * row_pitch) / 2.0).max(5.0);
    let stage_height = STAGE_HEIGHT.max(list_top + list_height + 5.0);
    let mut selected = None;

    ui.element()
        .id("mouse_stage")
        .width(grow!())
        .height(fixed!(stage_height * scale))
        .children(|ui| {
            if editing && !wheel_editor && model.has_detail(app.picker_anchor) {
                if let Some(part) = model
                    .parts()
                    .iter()
                    .find(|p| Some(p.id) == app.picker_anchor)
                {
                    button_detail(ui, app, model, part, scale);
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
                .image(
                    if matches!(model, Model::Original | Model::Master2S)
                        && app.prefs.theme == super::Theme::Dark
                    {
                        &CLASSIC_DARK
                    } else {
                        model.artwork()
                    },
                )
                .accessibility(|a| {
                    a.image(&format!(
                        "{}, front and thumb side button layout",
                        app.device_view.device.display_name()
                    ))
                })
                .empty();

            let mut row = 0;
            for (index, part) in model.parts().iter().enumerate() {
                let Some(target) = part.target(&app.device_view.device) else {
                    continue;
                };
                let wheel = target.opens_scroll(&app.device_view.device);
                let label = if wheel || (part.slot == "middle" && !app.device_view.device.connected)
                {
                    "Scroll wheel"
                } else {
                    part.label
                };
                let action = actions::assignment(app, &target);
                let action_label = if action.label.is_empty() {
                    action.keys.join(" + ")
                } else {
                    action.label
                };
                let action_label = if wheel && !target.available(&app.device_view.device) {
                    "Scrolling settings".to_owned()
                } else if wheel && matches!(action.kind, actions::Kind::PassThrough) {
                    "Standard middle click".to_owned()
                } else if wheel {
                    format!("Middle click: {action_label}")
                } else {
                    action_label
                };
                let active =
                    app.picker.as_ref() == Some(&target) && app.picker_anchor == Some(part.id);
                // Only the active marker has a corresponding control while the editor is open.
                if editing && !wheel_editor && !active {
                    continue;
                }
                let label_id: Id = ("mouse_label", index as u32).into();
                let hovered = ui.pointer_over(part.id) || ui.pointer_over(label_id.clone());
                let number = (index + 1).to_string();
                let (x, y) = part.point_on_stage(model);
                // Label the two adjacent thumb buttons on opposite sides of their contact points.
                let leader = if matches!(
                    model,
                    Model::Original | Model::Master2S | Model::Master3 | Model::Master3S
                ) {
                    match part.id {
                        "mouse_back" => -19.0,
                        "mouse_forward" => 19.0,
                        _ => 0.0,
                    }
                } else if model == Model::Master4 {
                    match part.id {
                        "mouse_forward" => 32.0,
                        "mouse_gesture" => 22.0,
                        _ => 0.0,
                    }
                } else {
                    0.0_f32
                };
                // A vertical line from Back crosses the 4's polished thumb wheel.
                let horizontal = if model == Model::Master4 && part.id == "mouse_back" {
                    65.0
                } else {
                    0.0
                };
                let has_leader = leader != 0.0 || horizontal != 0.0;
                if has_leader && ui.pointer_over(part.id) && !editing {
                    ui.element()
                        .width(fit!())
                        .height(fixed!(27.0))
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .offset((
                                    (x - 44.0) * art_scale + horizontal * scale,
                                    (y + leader) * art_scale
                                        + art_y
                                        + if leader < 0.0 { -43.0 } else { 15.0 },
                                ))
                                .z_index(4)
                                .passthrough()
                        })
                        .background_color(pal.pop)
                        .border(|b| b.all(1).color(pal.line_strong))
                        .corner_radius(6.0)
                        .layout(|l| l.align(CenterX, CenterY).padding((0, 9, 0, 9)))
                        .children(|ui| {
                            ui.text(label, |t| {
                                t.font(&fonts::SANS_SEMIBOLD).font_size(13).color(pal.text)
                            });
                        });
                }
                if has_leader {
                    ui.element()
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
                        .children(|ui| {
                            if ui.just_pressed() {
                                selected = Some((target.clone(), part.id));
                            }
                        });
                    ui.element()
                        .width(fixed!(if horizontal > 0.0 {
                            horizontal * scale
                        } else {
                            3.0
                        }))
                        .height(fixed!(if horizontal > 0.0 {
                            3.0
                        } else {
                            leader.abs() * scale
                        }))
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .clip_by_parent()
                                .offset((
                                    x * art_scale - 1.0,
                                    y * art_scale + art_y + leader.min(0.0) * scale
                                        - if horizontal > 0.0 { 1.0 } else { 0.0 },
                                ))
                                .z_index(1)
                        })
                        .background_color(theme::WHITE)
                        .border(|b| b.all(1).color(Color::rgb(39.0, 43.0, 47.0)))
                        .children(|ui| {
                            if ui.just_pressed() {
                                selected = Some((target.clone(), part.id));
                            }
                        });
                    ui.element()
                        .width(fixed!(6.0))
                        .height(fixed!(6.0))
                        .corner_radius(3.0)
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .clip_by_parent()
                                .offset((x * art_scale - 3.0, y * art_scale + art_y - 3.0))
                                .z_index(2)
                                .passthrough()
                        })
                        .background_color(theme::WHITE)
                        .border(|b| b.all(1).color(Color::rgb(39.0, 43.0, 47.0)))
                        .empty();
                }
                if wheel && !editing {
                    ui.element()
                        .id("wheel_hit")
                        .width(fixed!(36.0 * art_scale))
                        .height(fixed!(48.0 * art_scale))
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .clip_by_parent()
                                .offset(((x - 18.0) * art_scale, (y - 24.0) * art_scale + art_y))
                                .z_index(1)
                        })
                        .accessibility(|a| a.button("Edit scroll wheel"))
                        .children(|ui| {
                            if ui.just_pressed() {
                                selected = Some((target.clone(), part.id));
                            }
                        });
                }
                let lx = list_left;
                let ly = list_top + row as f32 * row_pitch;
                row += 1;
                ui.element()
                    .id(part.id)
                    .width(fixed!(24.0 * scale))
                    .height(fixed!(24.0 * scale))
                    .floating(|f| {
                        f.attach_id("mouse_stage")
                            .clip_by_parent()
                            .offset((
                                x * art_scale + (horizontal - 12.0) * scale,
                                y * art_scale + art_y + (leader - 12.0) * scale,
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
                            a.button(&format!("Configure {}", label))
                        } else {
                            a.static_text(label)
                        }
                    })
                    .layout(|l| l.align(CenterX, CenterY))
                    .children(|ui| {
                        if ui.just_pressed() && !active {
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

                if editing && wheel_editor {
                    continue;
                }
                if editing {
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
                    .width(fixed!(list_width * scale))
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
                            a.button(&format!("{}, {}", label, action_label))
                        } else {
                            a.static_text(label)
                        }
                    })
                    .layout(|l| {
                        l.direction(LeftToRight)
                            .align(Left, CenterY)
                            .gap(6)
                            .padding((5, 12, 5, 12))
                    })
                    .children(|ui| {
                        if ui.just_pressed() {
                            selected = Some((target.clone(), part.id));
                        }
                        ui.element()
                            .width(fixed!(22.0))
                            .height(fixed!(22.0))
                            .corner_radius(11.0)
                            .background_color(pal.win)
                            .border(|b| b.all(1).color(pal.line_strong))
                            .layout(|l| l.align(CenterX, CenterY))
                            .children(|ui| {
                                ui.text(&number, |t| {
                                    t.font(&fonts::MONO_BOLD).font_size(12).color(pal.text)
                                });
                            });
                        ui.element()
                            .width(grow!())
                            .height(fit!())
                            .layout(|l| l.direction(TopToBottom).gap(3))
                            .children(|ui| {
                                ui.text(label, |t| {
                                    t.font(&fonts::SANS_SEMIBOLD).font_size(15).color(pal.text)
                                });
                                let label = if matches!(target, PickerTarget::Gesture(_))
                                    && !app.selected_gestures().enabled
                                {
                                    "Gestures off"
                                } else {
                                    &action_label
                                };
                                if screen_width() >= 840.0 {
                                    ui.text(label, |t| {
                                        t.font(&fonts::SANS_MEDIUM).font_size(14).color(pal.accent)
                                    });
                                }
                            });
                        widgets::icon(
                            ui,
                            &icons::CHEVRON_RIGHT,
                            18.0,
                            if hovered { pal.accent } else { pal.muted },
                        );
                    });
            }
            if !editing && !extras.is_empty() {
                let x = list_left * scale;
                let y = list_top + count as f32 * row_pitch;
                ui.element()
                    .width(fixed!(list_width * scale))
                    .height(fixed!(extra_heading * scale))
                    .floating(|f| {
                        f.attach_id("mouse_stage")
                            .offset((x, y * scale))
                            .passthrough()
                    })
                    .layout(|l| l.align(Left, CenterY))
                    .children(|ui| {
                        ui.text("Outside the diagram", |t| {
                            t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.muted)
                        });
                    });
                for (i, (slot, name, cid)) in extras.iter().enumerate() {
                    let target = PickerTarget::Button(slot.clone());
                    let action = actions::assignment(app, &target);
                    let assignment = if action.label.is_empty() {
                        action.keys.join(" + ")
                    } else {
                        action.label
                    };
                    let label = if name == "Additional button" || name.starts_with("Control 0x") {
                        format!("Button 0x{cid:04X}")
                    } else {
                        name.clone()
                    };
                    let alias_key = format!("{}/{}", app.device_view.device.display_name(), slot);
                    let displayed = app
                        .prefs
                        .button_names
                        .get(&alias_key)
                        .unwrap_or(&label)
                        .clone();
                    let id: Id = ("mouse_extra", super::buttons::slot_index(slot)).into();
                    let hover = ui.pointer_over(id.clone());
                    ui.element()
                        .id(id)
                        .width(fixed!(list_width * scale))
                        .height(fixed!((row_pitch - 4.0) * scale))
                        .floating(|f| {
                            f.attach_id("mouse_stage")
                                .offset((x, (y + extra_heading + i as f32 * row_pitch) * scale))
                                .z_index(2)
                        })
                        .background_color(if hover { pal.s2 } else { pal.surface })
                        .corner_radius(10.0)
                        .border(|b| b.all(1).color(if hover { pal.accent } else { pal.line }))
                        .accessibility(|a| {
                            a.button(&format!(
                                "Edit {displayed}, control {cid:#06X}, {assignment}"
                            ))
                        })
                        .layout(|l| {
                            l.direction(LeftToRight)
                                .align(Left, CenterY)
                                .padding((4, 12, 4, 12))
                                .gap(6)
                        })
                        .children(|ui| {
                            widgets::icon(ui, &icons::NAV_BUTTONS, 22.0, pal.muted);
                            ui.element()
                                .width(grow!())
                                .height(fit!())
                                .layout(|l| l.direction(TopToBottom).gap(3))
                                .children(|ui| {
                                    ui.text(&displayed, |t| {
                                        t.font(&fonts::SANS_SEMIBOLD).font_size(14).color(pal.text)
                                    });
                                    ui.text(&assignment, |t| {
                                        t.font(&fonts::SANS_MEDIUM).font_size(13).color(pal.accent)
                                    });
                                });
                            widgets::icon(
                                ui,
                                &icons::CAT_EDIT,
                                15.0,
                                if hover { pal.accent } else { pal.muted },
                            );
                            if ui.just_pressed() {
                                app.open_picker(target);
                                app.picker_row_label = Some(label);
                            }
                        });
                }
            }
        });

    if editing {
        return;
    }
    if Section::Gestures.available(&app.device_view.device) {
        if !extras.is_empty() {
            ui.element().height(fixed!(12.0)).empty();
        }
        gesture_footer(
            ui,
            app,
            "Gesture actions follow the selected profile.",
            "Configure gestures",
        );
    }
    if let Some((target, anchor)) = selected {
        app.open_button_popover(target, anchor);
    }
}

pub(super) fn gesture_footer(ui: &mut Ui2, app: &mut App, hint: &str, label: &str) {
    let pal = app.pal;
    let mut gestures = false;
    ui.element()
        .width(grow!())
        .height(fit!())
        .layout(|l| l.direction(LeftToRight).align(Left, CenterY).gap(10))
        .children(|ui| {
            ui.element().width(grow!()).children(|ui| {
                ui.text(hint, |t| {
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
                        a.button(label)
                    } else {
                        a.static_text(label)
                    }
                })
                .layout(|l| l.direction(LeftToRight).gap(8).padding((8, 12, 8, 12)))
                .children(|ui| {
                    gestures = ui.just_pressed();
                    widgets::icon(ui, &icons::NAV_GESTURES, 16.0, pal.muted);
                    ui.text(label, |t| {
                        t.font(&fonts::SANS_MEDIUM).font_size(14).color(pal.text)
                    });
                });
        });
    if gestures {
        app.section = Section::Gestures;
        app.close_picker();
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
            default_task_id: None,
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
            assert_eq!(slots, ["middle", "back", "forward", "tap", "control_00c4"]);
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
            let extras = extra_buttons(model, &device, &crate::config::Config::default());
            assert_eq!(
                extras.iter().filter(|b| b.slot == "control_01ff").count(),
                1
            );
            assert_eq!(extras.len(), if model == Model::Master4 { 1 } else { 2 });
        }
        device.connected = false;
        assert!(
            extra_buttons(Model::Master4, &device, &crate::config::Config::default()).is_empty()
        );
    }
}
