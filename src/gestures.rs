//! Software gesture detection for the thumb button.
//!
//! Fed by diverted REPROG_CONTROLS_V4 events (button hold + raw XY) from the
//! HID device thread. On button-down we start accumulating raw motion; on
//! button-up we classify the accumulated vector into tap / up / down / left /
//! right and inject the configured action.

use crate::config::Action;
use crate::state;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Tap,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default)]
struct GestureDetector {
    active: bool,
    dx: i64,
    dy: i64,
}

impl GestureDetector {
    fn on_button(&mut self, gesture_held: bool) -> Option<Outcome> {
        if gesture_held && !self.active {
            self.active = true;
            self.dx = 0;
            self.dy = 0;
            tracing::debug!("gesture start");
            None
        } else if !gesture_held && self.active {
            self.active = false;
            let threshold = state::config().gestures.tap_threshold as i64;
            let outcome = classify(self.dx, self.dy, threshold);
            tracing::info!(
                "gesture end: dx={} dy={} -> {:?}",
                self.dx,
                self.dy,
                outcome
            );
            Some(outcome)
        } else {
            None
        }
    }

    fn on_raw_xy(&mut self, dx: i32, dy: i32) {
        if self.active {
            self.dx += dx as i64;
            self.dy += dy as i64;
        }
    }
}

/// Classify an accumulated motion vector. Screen coordinates: +y is downward.
fn classify(dx: i64, dy: i64, threshold: i64) -> Outcome {
    if dx.abs() < threshold && dy.abs() < threshold {
        return Outcome::Tap;
    }
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            Outcome::Right
        } else {
            Outcome::Left
        }
    } else if dy >= 0 {
        Outcome::Down
    } else {
        Outcome::Up
    }
}

fn action_for(outcome: Outcome) -> Action {
    let cfg = state::config();
    let g = &cfg.gestures;
    match outcome {
        Outcome::Tap => g.tap.clone(),
        Outcome::Up => g.up.clone(),
        Outcome::Down => g.down.clone(),
        Outcome::Left => g.left.clone(),
        Outcome::Right => g.right.clone(),
    }
}

static DETECTOR: OnceLock<Mutex<GestureDetector>> = OnceLock::new();

fn detector() -> &'static Mutex<GestureDetector> {
    DETECTOR.get_or_init(|| Mutex::new(GestureDetector::default()))
}

/// Handle a diverted-buttons event (whether the gesture button is currently
/// held). Injects the configured action when a gesture completes.
pub fn handle_buttons(gesture_held: bool) {
    let outcome = detector().lock().unwrap().on_button(gesture_held);
    if let Some(outcome) = outcome {
        let action = action_for(outcome);
        if action.intercepts() {
            state::inject(action);
        }
    }
}

/// Handle a diverted raw-XY event. While the gesture button is held the motion
/// is accumulated for classification; otherwise (global raw-XY divert) it is
/// re-injected as cursor movement so the pointer is never frozen.
pub fn handle_raw_xy(dx: i32, dy: i32) {
    let active = {
        let mut d = detector().lock().unwrap();
        if d.active {
            d.on_raw_xy(dx, dy);
            true
        } else {
            false
        }
    };
    if !active {
        crate::injector::move_cursor(dx, dy);
    }
}
