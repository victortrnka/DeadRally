use deadrally_core::{Key, PadButton};
use gilrs::Button;
use winit::keyboard::KeyCode;

/// Physical winit key codes and the core keys they stand for.
const KEYS: &[(KeyCode, Key)] = &[
    (KeyCode::KeyA, Key::A),
    (KeyCode::KeyB, Key::B),
    (KeyCode::KeyC, Key::C),
    (KeyCode::KeyD, Key::D),
    (KeyCode::KeyE, Key::E),
    (KeyCode::KeyF, Key::F),
    (KeyCode::KeyG, Key::G),
    (KeyCode::KeyH, Key::H),
    (KeyCode::KeyI, Key::I),
    (KeyCode::KeyJ, Key::J),
    (KeyCode::KeyK, Key::K),
    (KeyCode::KeyL, Key::L),
    (KeyCode::KeyM, Key::M),
    (KeyCode::KeyN, Key::N),
    (KeyCode::KeyO, Key::O),
    (KeyCode::KeyP, Key::P),
    (KeyCode::KeyQ, Key::Q),
    (KeyCode::KeyR, Key::R),
    (KeyCode::KeyS, Key::S),
    (KeyCode::KeyT, Key::T),
    (KeyCode::KeyU, Key::U),
    (KeyCode::KeyV, Key::V),
    (KeyCode::KeyW, Key::W),
    (KeyCode::KeyX, Key::X),
    (KeyCode::KeyY, Key::Y),
    (KeyCode::KeyZ, Key::Z),
    (KeyCode::Digit0, Key::Digit0),
    (KeyCode::Digit1, Key::Digit1),
    (KeyCode::Digit2, Key::Digit2),
    (KeyCode::Digit3, Key::Digit3),
    (KeyCode::Digit4, Key::Digit4),
    (KeyCode::Digit5, Key::Digit5),
    (KeyCode::Digit6, Key::Digit6),
    (KeyCode::Digit7, Key::Digit7),
    (KeyCode::Digit8, Key::Digit8),
    (KeyCode::Digit9, Key::Digit9),
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::ArrowLeft, Key::Left),
    (KeyCode::ArrowRight, Key::Right),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Space, Key::Space),
    (KeyCode::Backspace, Key::Backspace),
    (KeyCode::Tab, Key::Tab),
    (KeyCode::ShiftLeft, Key::LeftShift),
    (KeyCode::ShiftRight, Key::RightShift),
    (KeyCode::ControlLeft, Key::LeftCtrl),
    (KeyCode::ControlRight, Key::RightCtrl),
    (KeyCode::AltLeft, Key::LeftAlt),
    (KeyCode::AltRight, Key::RightAlt),
    (KeyCode::Numpad0, Key::Kp0),
    (KeyCode::Numpad1, Key::Kp1),
    (KeyCode::Numpad2, Key::Kp2),
    (KeyCode::Numpad3, Key::Kp3),
    (KeyCode::Numpad4, Key::Kp4),
    (KeyCode::Numpad5, Key::Kp5),
    (KeyCode::Numpad6, Key::Kp6),
    (KeyCode::Numpad7, Key::Kp7),
    (KeyCode::Numpad8, Key::Kp8),
    (KeyCode::Numpad9, Key::Kp9),
    (KeyCode::NumpadAdd, Key::KpPlus),
    (KeyCode::NumpadSubtract, Key::KpMinus),
    (KeyCode::NumpadMultiply, Key::KpMultiply),
    (KeyCode::NumpadDivide, Key::KpDivide),
    (KeyCode::NumpadEnter, Key::KpEnter),
    (KeyCode::NumpadDecimal, Key::KpPeriod),
];

/// Maps a physical winit key to the core's key, if the core knows it.
pub fn key(code: KeyCode) -> Option<Key> {
    KEYS.iter()
        .find(|(candidate, _)| *candidate == code)
        .map(|&(_, key)| key)
}

/// Maps a gamepad face button, by position, to the core's button.
pub fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::South => PadButton::A,
        Button::East => PadButton::B,
        Button::West => PadButton::X,
        Button::North => PadButton::Y,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_key_is_reachable_exactly_once() {
        // A key the frontend cannot produce could never be bound in the controls menu.
        let mut keys: Vec<Key> = KEYS.iter().map(|&(_, key)| key).collect();
        keys.sort();
        assert_eq!(keys, Key::ALL);
    }
}
