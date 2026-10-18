//! HID++ 2.0 mouse control over receivers, Bluetooth, and wired USB.

pub mod device;
pub mod features;
pub mod history;
pub mod protocol;
pub mod transport;

#[cfg(test)]
mod tests;
