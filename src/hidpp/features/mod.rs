//! HID++ 2.0 feature implementations. Each submodule adds `impl Device` methods
//! for one feature id.

pub mod battery; // 0x1000 BATTERY_STATUS
pub mod device_info; // 0x0003 DEVICE_INFO (firmware)
pub mod feature_set; // 0x0001 IFeatureSet
pub mod root; // 0x0000 IRoot

pub mod dpi; // 0x2201 ADJUSTABLE_DPI
pub mod hires_wheel; // 0x2121 HIRES_WHEEL
pub mod reprog; // 0x1B04 REPROG_CONTROLS_V4
pub mod smartshift; // 0x2110 SMART_SHIFT
