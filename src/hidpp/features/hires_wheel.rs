//! Feature 0x2121 HIRES_WHEEL (smooth/high-resolution scrolling + invert).
//!
//! function 0 getWheelCapability -> multiplier, flags
//! function 1 getMode -> mode byte
//! function 2 setMode <- mode byte
//! event 0    wheelMovement: periods byte, then int16 BE delta
//!
//! mode bits: 0x01 target (divert to HID++), 0x02 high-resolution, 0x04 invert.
//!
//! # Why high-resolution means diverting
//!
//! Turning on bit 0x02 alone makes the wheel report `multiplier` increments per
//! detent (8 on the MX Master 2S) straight to Windows on the ordinary mouse
//! interface. Windows is never told the resolution changed — that is negotiated
//! through the HID Resolution Multiplier in the report descriptor, which
//! Logitech's own driver drives and we do not — so it reads all 8 increments as
//! 8 whole notches and scrolling comes out 8x too fast and jittery.
//!
//! So high-resolution also sets the target bit, which routes wheel movement to
//! us as HID++ events instead of to Windows. We scale them back to Windows'
//! units and re-inject, which is what makes the scroll smooth rather than fast.
//! The cost is that while diverted the wheel sends *nothing* to Windows on its
//! own: if this process dies without restoring the mode, scrolling stops until
//! the mouse power-cycles. `restore_wheel` exists for that reason and is called
//! on shutdown alongside the control diverts.

use crate::hidpp::device::Device;
use crate::injector;
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicI32, Ordering};

const MODE_TARGET: u8 = 0x01;
const MODE_HIRES: u8 = 0x02;
const MODE_INVERT: u8 = 0x04;

/// One wheel notch, in Windows' wheel units.
const WHEEL_DELTA: i32 = 120;

/// Left-over hi-res units not yet worth a whole Windows unit. Keeping the
/// remainder is what stops slow scrolling from being quantised away.
static REMAINDER: AtomicI32 = AtomicI32::new(0);

impl Device {
    fn hires_index(&self) -> Result<u8> {
        self.features
            .hires_wheel
            .ok_or_else(|| anyhow!("no HIRES_WHEEL feature"))
    }

    pub(crate) fn get_wheel_mode(&self) -> Result<u8> {
        let idx = self.hires_index()?;
        let rep = self.request(idx, 1, &[])?;
        Ok(rep.param(0))
    }

    /// function 0 getWheelCapability -> multiplier, flags.
    ///
    /// The multiplier is how many increments the wheel reports per detent once
    /// high-resolution mode is on.
    pub(crate) fn get_wheel_capability(&self) -> Result<(u8, u8)> {
        let idx = self.hires_index()?;
        let rep = self.request(idx, 0, &[])?;
        Ok((rep.param(0), rep.param(1)))
    }

    fn set_wheel_mode(&self, mode: u8) -> Result<()> {
        let idx = self.hires_index()?;
        self.request(idx, 2, &[mode])?;
        Ok(())
    }

    /// The wheel's hi-res multiplier, or 8 if the device will not say.
    fn wheel_multiplier(&self) -> i32 {
        match self.get_wheel_capability() {
            Ok((m, _)) if m > 0 => m as i32,
            _ => 8,
        }
    }

    /// Apply high-resolution + invert.
    ///
    /// High-resolution implies the target bit: see the module comment. Without
    /// it, Windows misreads every increment as a whole notch.
    pub(crate) fn apply_hires(&self, hires: bool, invert: bool) -> Result<()> {
        if self.features.hires_wheel.is_none() {
            return Ok(());
        }
        if let Ok((mult, flags)) = self.get_wheel_capability() {
            tracing::debug!("hires wheel capability: multiplier={mult} flags={flags:#04x}");
        }
        let mut mode = 0u8;
        if hires {
            mode |= MODE_HIRES | MODE_TARGET;
        }
        if invert {
            mode |= MODE_INVERT;
        }
        REMAINDER.store(0, Ordering::Relaxed);
        tracing::info!(
            "hires wheel -> mode={mode:#04x} (hires={hires} invert={invert} diverted={hires})"
        );
        self.set_wheel_mode(mode)
    }

    /// Hand the wheel back to Windows. Called on shutdown: a wheel left
    /// diverted reports to nobody once we are gone.
    pub(crate) fn restore_wheel(&self) {
        if self.features.hires_wheel.is_none() {
            return;
        }
        // Keep invert, drop hi-res and the divert.
        let invert = self.get_wheel_mode().unwrap_or(0) & MODE_INVERT;
        match self.set_wheel_mode(invert) {
            Ok(()) => tracing::info!("wheel restored to native reporting"),
            Err(e) => tracing::warn!("restoring wheel failed: {e:#}"),
        }
    }

    /// Handle a diverted wheelMovement event: `periods`, then int16 BE delta in
    /// hi-res increments.
    pub(crate) fn on_wheel_event(&self, rep: &crate::hidpp::protocol::Report) {
        // 0x2121 also raises event 1 (ratchetSwitch) when the wheel changes
        // between ratchet and freespin; only event 0 carries movement.
        if rep.function() != 0 {
            tracing::debug!("wheel ratchet event: {}", crate::logging::hex(&rep.raw));
            return;
        }
        let delta = (((rep.param(1) as u16) << 8) | rep.param(2) as u16) as i16 as i32;
        if delta == 0 {
            return;
        }
        let mult = self.wheel_multiplier();

        // hi-res increments -> Windows units, carrying the remainder so slow
        // scrolls are not rounded to nothing.
        let scaled = delta * WHEEL_DELTA + REMAINDER.load(Ordering::Relaxed);
        let out = scaled / mult;
        REMAINDER.store(scaled % mult, Ordering::Relaxed);

        tracing::trace!("wheel event: delta={delta} -> {out} (mult={mult})");
        injector::wheel(out);
    }
}
