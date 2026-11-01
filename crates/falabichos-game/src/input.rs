// SPDX-License-Identifier: Apache-2.0
//! Raw input (held state, filled by the platform shell) and per-tick edge detection.

/// Held-button state plus text input for this tick. The shell (Bevy client, tests, sim) fills it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Confirm / interact.
    pub a: bool,
    /// Cancel / run.
    pub b: bool,
    /// Menu.
    pub start: bool,
    /// Hint (battle) / English gloss (dialogue).
    pub select: bool,
    /// Characters typed this tick (physical keyboard).
    pub typed: String,
    /// Backspace pressed this tick.
    pub backspace: bool,
    /// Enter pressed this tick (submits typed answers).
    pub enter: bool,
}

/// Edge-detected input for one tick.
#[derive(Debug, Clone, Default)]
pub struct Pad {
    pub held: Input,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
    pub typed: String,
    pub backspace: bool,
    pub enter: bool,
}

#[derive(Debug, Default)]
pub struct PadTracker {
    prev: [bool; 8],
    hold: [u32; 4],
}

const REPEAT_DELAY: u32 = 16;
const REPEAT_RATE: u32 = 5;

impl PadTracker {
    pub fn update(&mut self, i: &Input) -> Pad {
        let now = [i.up, i.down, i.left, i.right, i.a, i.b, i.start, i.select];
        let mut press = [false; 8];
        for k in 0..8 {
            press[k] = now[k] && !self.prev[k];
        }
        // auto-repeat for the d-pad (menus)
        for k in 0..4 {
            if now[k] {
                self.hold[k] += 1;
                if !press[k]
                    && self.hold[k] > REPEAT_DELAY
                    && (self.hold[k] - REPEAT_DELAY).is_multiple_of(REPEAT_RATE)
                {
                    press[k] = true;
                }
            } else {
                self.hold[k] = 0;
            }
        }
        self.prev = now;
        Pad {
            held: i.clone(),
            up: press[0],
            down: press[1],
            left: press[2],
            right: press[3],
            a: press[4],
            b: press[5],
            start: press[6],
            select: press[7],
            typed: i.typed.clone(),
            backspace: i.backspace,
            enter: i.enter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_and_repeat() {
        let mut t = PadTracker::default();
        let held = Input {
            down: true,
            ..Input::default()
        };
        assert!(t.update(&held).down, "first tick presses");
        for _ in 0..REPEAT_DELAY {
            assert!(!t.update(&held).down);
        }
        let mut repeats = 0;
        for _ in 0..20 {
            if t.update(&held).down {
                repeats += 1;
            }
        }
        assert!(repeats >= 3, "auto-repeats while held, got {repeats}");
        t.update(&Input::default());
        assert!(t.update(&held).down, "release then press fires again");
    }

    #[test]
    fn buttons_only_fire_once() {
        let mut t = PadTracker::default();
        let a = Input {
            a: true,
            ..Input::default()
        };
        assert!(t.update(&a).a);
        assert!(!t.update(&a).a);
    }
}
