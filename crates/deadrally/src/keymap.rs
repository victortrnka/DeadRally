use deadrally_core::{Key, PadButton};
use sdl3::gamepad::Button;
use sdl3::keyboard::Scancode;

/// Maps an SDL scancode (a physical key position) to the core's key, if the core knows it.
pub fn key(scancode: Scancode) -> Option<Key> {
    Some(match scancode {
        Scancode::A => Key::A,
        Scancode::B => Key::B,
        Scancode::C => Key::C,
        Scancode::D => Key::D,
        Scancode::E => Key::E,
        Scancode::F => Key::F,
        Scancode::G => Key::G,
        Scancode::H => Key::H,
        Scancode::I => Key::I,
        Scancode::J => Key::J,
        Scancode::K => Key::K,
        Scancode::L => Key::L,
        Scancode::M => Key::M,
        Scancode::N => Key::N,
        Scancode::O => Key::O,
        Scancode::P => Key::P,
        Scancode::Q => Key::Q,
        Scancode::R => Key::R,
        Scancode::S => Key::S,
        Scancode::T => Key::T,
        Scancode::U => Key::U,
        Scancode::V => Key::V,
        Scancode::W => Key::W,
        Scancode::X => Key::X,
        Scancode::Y => Key::Y,
        Scancode::Z => Key::Z,
        Scancode::_0 => Key::Digit0,
        Scancode::_1 => Key::Digit1,
        Scancode::_2 => Key::Digit2,
        Scancode::_3 => Key::Digit3,
        Scancode::_4 => Key::Digit4,
        Scancode::_5 => Key::Digit5,
        Scancode::_6 => Key::Digit6,
        Scancode::_7 => Key::Digit7,
        Scancode::_8 => Key::Digit8,
        Scancode::_9 => Key::Digit9,
        Scancode::F1 => Key::F1,
        Scancode::F2 => Key::F2,
        Scancode::F3 => Key::F3,
        Scancode::F4 => Key::F4,
        Scancode::F5 => Key::F5,
        Scancode::F6 => Key::F6,
        Scancode::F7 => Key::F7,
        Scancode::F8 => Key::F8,
        Scancode::F9 => Key::F9,
        Scancode::F10 => Key::F10,
        Scancode::F11 => Key::F11,
        Scancode::F12 => Key::F12,
        Scancode::Up => Key::Up,
        Scancode::Down => Key::Down,
        Scancode::Left => Key::Left,
        Scancode::Right => Key::Right,
        Scancode::Return => Key::Enter,
        Scancode::Escape => Key::Escape,
        Scancode::Space => Key::Space,
        Scancode::Backspace => Key::Backspace,
        Scancode::Tab => Key::Tab,
        Scancode::LShift => Key::LeftShift,
        Scancode::RShift => Key::RightShift,
        Scancode::LCtrl => Key::LeftCtrl,
        Scancode::RCtrl => Key::RightCtrl,
        Scancode::LAlt => Key::LeftAlt,
        Scancode::RAlt => Key::RightAlt,
        Scancode::Kp0 => Key::Kp0,
        Scancode::Kp1 => Key::Kp1,
        Scancode::Kp2 => Key::Kp2,
        Scancode::Kp3 => Key::Kp3,
        Scancode::Kp4 => Key::Kp4,
        Scancode::Kp5 => Key::Kp5,
        Scancode::Kp6 => Key::Kp6,
        Scancode::Kp7 => Key::Kp7,
        Scancode::Kp8 => Key::Kp8,
        Scancode::Kp9 => Key::Kp9,
        Scancode::KpPlus => Key::KpPlus,
        Scancode::KpMinus => Key::KpMinus,
        Scancode::KpMultiply => Key::KpMultiply,
        Scancode::KpDivide => Key::KpDivide,
        Scancode::KpEnter => Key::KpEnter,
        Scancode::KpPeriod => Key::KpPeriod,
        _ => return None,
    })
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
    fn every_core_key_is_reachable() {
        // A key the frontend cannot produce could never be bound in the controls menu.
        let mut reached: Vec<Key> = (0..512)
            .filter_map(Scancode::from_i32)
            .filter_map(key)
            .collect();
        reached.sort();
        reached.dedup();
        assert_eq!(reached, Key::ALL);
    }
}
