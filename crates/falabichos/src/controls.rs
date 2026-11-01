// SPDX-License-Identifier: Apache-2.0
//! Keyboard, gamepad and touch input -> `falabichos_game::Input`.

use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::input::keyboard::KeyCode;
use falabichos_game::Input;

/// Map held keys to game buttons. While the game wants text (a typed answer), letter keys are
/// *not* buttons: they are text. Only arrows, Enter, Esc and Backspace keep working.
pub fn map_keys(held: &dyn Fn(KeyCode) -> bool, wants_text: bool) -> Input {
    let any = |keys: &[KeyCode]| keys.iter().any(|k| held(*k));
    let mut i = Input {
        up: any(&[KeyCode::ArrowUp]),
        down: any(&[KeyCode::ArrowDown]),
        left: any(&[KeyCode::ArrowLeft]),
        right: any(&[KeyCode::ArrowRight]),
        start: any(&[KeyCode::Escape]),
        ..Input::default()
    };
    if wants_text {
        i.enter = any(&[KeyCode::Enter, KeyCode::NumpadEnter]);
        i.backspace = any(&[KeyCode::Backspace]);
    } else {
        i.up |= any(&[KeyCode::KeyW]);
        i.down |= any(&[KeyCode::KeyS]);
        i.left |= any(&[KeyCode::KeyA]);
        i.right |= any(&[KeyCode::KeyD]);
        i.a = any(&[
            KeyCode::Enter,
            KeyCode::NumpadEnter,
            KeyCode::Space,
            KeyCode::KeyZ,
            KeyCode::KeyJ,
        ]);
        i.b = any(&[
            KeyCode::KeyX,
            KeyCode::Backspace,
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::KeyK,
        ]);
        i.start |= any(&[KeyCode::KeyM]);
        i.select = any(&[KeyCode::Tab, KeyCode::KeyG, KeyCode::KeyH]);
    }
    i
}

pub fn merge(a: &mut Input, b: &Input) {
    a.up |= b.up;
    a.down |= b.down;
    a.left |= b.left;
    a.right |= b.right;
    a.a |= b.a;
    a.b |= b.b;
    a.start |= b.start;
    a.select |= b.select;
    a.enter |= b.enter;
    a.backspace |= b.backspace;
    a.typed.push_str(&b.typed);
}

pub fn read_gamepad(gp: &Gamepad) -> Input {
    let stick_x = gp.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
    let stick_y = gp.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
    Input {
        up: gp.pressed(GamepadButton::DPadUp) || stick_y > 0.5,
        down: gp.pressed(GamepadButton::DPadDown) || stick_y < -0.5,
        left: gp.pressed(GamepadButton::DPadLeft) || stick_x < -0.5,
        right: gp.pressed(GamepadButton::DPadRight) || stick_x > 0.5,
        a: gp.pressed(GamepadButton::South),
        b: gp.pressed(GamepadButton::East),
        start: gp.pressed(GamepadButton::Start),
        select: gp.pressed(GamepadButton::Select) || gp.pressed(GamepadButton::North),
        ..Input::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_text_not_buttons_while_typing() {
        let held = |k: KeyCode| k == KeyCode::KeyZ || k == KeyCode::KeyA || k == KeyCode::Enter;
        let walking = map_keys(&held, false);
        assert!(walking.a && walking.left);
        let typing = map_keys(&held, true);
        assert!(!typing.a && !typing.left, "z and a are letters while typing");
        assert!(typing.enter, "enter still submits");
    }

    #[test]
    fn arrows_always_work_and_merge_ors() {
        let held = |k: KeyCode| k == KeyCode::ArrowUp;
        assert!(map_keys(&held, true).up && map_keys(&held, false).up);
        let mut a = Input::default();
        merge(
            &mut a,
            &Input {
                b: true,
                typed: "x".into(),
                ..Input::default()
            },
        );
        merge(
            &mut a,
            &Input {
                a: true,
                typed: "y".into(),
                ..Input::default()
            },
        );
        assert!(a.a && a.b);
        assert_eq!(a.typed, "xy");
    }
}
