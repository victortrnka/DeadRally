//! The player's input as the original reads it (spec M2a §3.3): one remembered key, its PC
//! set-1 scancode as SDL 1.2's `windib` driver reports it, kept until `eventDetected`
//! (0x417EB0) reads and clears it; SDL 1.2's key repeat (`SDL_EnableKeyRepeat(500, 30)`); and
//! the joystick, polled by `eventDetected` itself.

use crate::input::{InputEvent, Key, PadAxis, PadButton};

/// Scancodes the menus act on.
pub(crate) const ESCAPE: u8 = 0x01;
pub(crate) const ENTER: u8 = 0x1C;
pub(crate) const SPACE: u8 = 0x39;
pub(crate) const UP: u8 = 0x48;
pub(crate) const DOWN: u8 = 0x50;
pub(crate) const LEFT: u8 = 0x4B;
pub(crate) const RIGHT: u8 = 0x4D;
pub(crate) const Y: u8 = 0x15;
pub(crate) const N: u8 = 0x31;
/// The joystick's codes are DirectInput's: set-1 with the extended bit.
pub(crate) const PAD_LEFT: u8 = 0xCB;
pub(crate) const PAD_RIGHT: u8 = 0xCD;
pub(crate) const PAD_UP: u8 = 0xC8;
pub(crate) const PAD_DOWN: u8 = 0xD0;

/// One tick, in SDL milliseconds.
const TICK_MS: i64 = 14;
/// SDL 1.2 key repeat: the delay before the first repeat and the interval after it.
const REPEAT_DELAY_MS: i64 = 500;
const REPEAT_INTERVAL_MS: i64 = 30;
/// `eventDetected`'s joystick: stick positions past ±50 (raw / 256) count; a first push holds
/// the repeat off for 700 ms; 400 ms tell a fresh push from a held one.
const STICK_THRESHOLD: i32 = 50;
const HOLD_OFF_MS: i64 = 700;
const FRESH_MS: i64 = 400;
const UNSET_MS: i64 = 250;

/// The PC set-1 scancode SDL 1.2's `windib` driver reports for a key (no 0xE0 prefix, so the
/// arrows share the keypad's codes).
pub(crate) fn scancode(key: Key) -> u8 {
    match key {
        Key::Escape => 0x01,
        Key::Digit1 => 0x02,
        Key::Digit2 => 0x03,
        Key::Digit3 => 0x04,
        Key::Digit4 => 0x05,
        Key::Digit5 => 0x06,
        Key::Digit6 => 0x07,
        Key::Digit7 => 0x08,
        Key::Digit8 => 0x09,
        Key::Digit9 => 0x0A,
        Key::Digit0 => 0x0B,
        Key::Backspace => 0x0E,
        Key::Tab => 0x0F,
        Key::Q => 0x10,
        Key::W => 0x11,
        Key::E => 0x12,
        Key::R => 0x13,
        Key::T => 0x14,
        Key::Y => 0x15,
        Key::U => 0x16,
        Key::I => 0x17,
        Key::O => 0x18,
        Key::P => 0x19,
        Key::Enter | Key::KpEnter => 0x1C,
        Key::LeftCtrl | Key::RightCtrl => 0x1D,
        Key::A => 0x1E,
        Key::S => 0x1F,
        Key::D => 0x20,
        Key::F => 0x21,
        Key::G => 0x22,
        Key::H => 0x23,
        Key::J => 0x24,
        Key::K => 0x25,
        Key::L => 0x26,
        Key::LeftShift => 0x2A,
        Key::Z => 0x2C,
        Key::X => 0x2D,
        Key::C => 0x2E,
        Key::V => 0x2F,
        Key::B => 0x30,
        Key::N => 0x31,
        Key::M => 0x32,
        Key::KpDivide => 0x35,
        Key::RightShift => 0x36,
        Key::KpMultiply => 0x37,
        Key::LeftAlt | Key::RightAlt => 0x38,
        Key::Space => 0x39,
        Key::F1 => 0x3B,
        Key::F2 => 0x3C,
        Key::F3 => 0x3D,
        Key::F4 => 0x3E,
        Key::F5 => 0x3F,
        Key::F6 => 0x40,
        Key::F7 => 0x41,
        Key::F8 => 0x42,
        Key::F9 => 0x43,
        Key::F10 => 0x44,
        Key::Kp7 => 0x47,
        Key::Up | Key::Kp8 => 0x48,
        Key::Kp9 => 0x49,
        Key::KpMinus => 0x4A,
        Key::Left | Key::Kp4 => 0x4B,
        Key::Kp5 => 0x4C,
        Key::Right | Key::Kp6 => 0x4D,
        Key::KpPlus => 0x4E,
        Key::Kp1 => 0x4F,
        Key::Down | Key::Kp2 => 0x50,
        Key::Kp3 => 0x51,
        Key::Kp0 => 0x52,
        Key::KpPeriod => 0x53,
        Key::F11 => 0x57,
        Key::F12 => 0x58,
    }
}

/// SDL 1.2 repeats every key but the modifiers.
fn repeats(key: Key) -> bool {
    !matches!(
        key,
        Key::LeftShift
            | Key::RightShift
            | Key::LeftCtrl
            | Key::RightCtrl
            | Key::LeftAlt
            | Key::RightAlt
    )
}

/// The key SDL repeats while it is held.
#[derive(Clone, Copy, Debug)]
struct Repeat {
    key: Key,
    /// Still waiting for the first delay.
    first: bool,
    /// Milliseconds since SDL's repeat timestamp.
    elapsed: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Keys {
    /// `[0x456BF8]`: the last key-down's scancode, 0 when read.
    remembered: u8,
    repeat: Option<Repeat>,
    /// Ticks polled so far: SDL's clock is `14 * ticks` milliseconds.
    ticks: i64,
    stick: [i32; 2],
    buttons: [bool; 4],
    /// `eventDetected`'s joystick timestamps: 0x456B28, 0x456B2C and the hold-off 0x456B1C.
    pad_pushed_ms: Option<i64>,
    pad_called_ms: Option<i64>,
    hold_off_until_ms: i64,
}

impl Keys {
    /// An input event from the frontend. Key-downs become the remembered key at once, as the
    /// next poll of the original's event loop would make them.
    pub(crate) fn event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed: true } => {
                self.remembered = scancode(key);
                if repeats(key) {
                    self.repeat = Some(Repeat {
                        key,
                        first: true,
                        elapsed: 0,
                    });
                }
            }
            InputEvent::Key {
                key,
                pressed: false,
            } => {
                if self.repeat.is_some_and(|repeat| repeat.key == key) {
                    self.repeat = None;
                }
            }
            InputEvent::PadButton { button, pressed } => {
                self.buttons[PadButton::ALL
                    .iter()
                    .position(|&b| b == button)
                    .expect("every button is listed")] = pressed;
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[usize::from(axis == PadAxis::StickY)] = i32::from(value) / 256;
            }
        }
    }

    /// One poll of the event loop, once per tick: SDL 1.2 repeats the held key.
    pub(crate) fn tick(&mut self) {
        self.ticks += 1;
        if let Some(repeat) = &mut self.repeat {
            repeat.elapsed += TICK_MS;
            if repeat.first {
                if repeat.elapsed > REPEAT_DELAY_MS {
                    repeat.first = false;
                    repeat.elapsed = 0;
                }
            } else if repeat.elapsed > REPEAT_INTERVAL_MS {
                repeat.elapsed = 0;
                self.remembered = scancode(repeat.key);
            }
        }
    }

    /// `eventDetected`: the remembered key, cleared, unless the joystick says something.
    pub(crate) fn take(&mut self) -> u8 {
        let key = std::mem::take(&mut self.remembered);
        let now = TICK_MS * self.ticks;
        let pushed = self.pad_pushed_ms.unwrap_or(now - UNSET_MS);
        let called = *self.pad_called_ms.get_or_insert(now - UNSET_MS);
        let pad = self.pad_code();
        if now >= self.hold_off_until_ms {
            if pad != 0 {
                if now - pushed >= FRESH_MS && now - called < FRESH_MS {
                    self.hold_off_until_ms = now + HOLD_OFF_MS;
                }
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
                pad
            } else {
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
                key
            }
        } else if pad != 0 {
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
            0
        } else {
            self.hold_off_until_ms = now;
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
            key
        }
    }

    /// The joystick's code, later checks winning as in the original: buttons over the stick,
    /// the vertical axis over the horizontal one.
    fn pad_code(&self) -> u8 {
        let [x, y] = self.stick;
        let mut code = 0;
        if x < -STICK_THRESHOLD {
            code = PAD_LEFT;
        }
        if x > STICK_THRESHOLD {
            code = PAD_RIGHT;
        }
        if y < -STICK_THRESHOLD {
            code = PAD_UP;
        }
        if y > STICK_THRESHOLD {
            code = PAD_DOWN;
        }
        for (pressed, button_code) in self.buttons.iter().zip([ENTER, ESCAPE, ENTER, ESCAPE]) {
            if *pressed {
                code = button_code;
            }
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool) -> InputEvent {
        InputEvent::Key { key, pressed }
    }

    #[test]
    fn the_last_key_down_is_remembered_until_it_is_read() {
        // One byte in the original: a second press replaces the first, and reading clears it.
        let mut keys = Keys::default();
        keys.event(key(Key::Up, true));
        keys.event(key(Key::Up, false));
        keys.event(key(Key::Escape, true));
        keys.event(key(Key::Escape, false));
        assert_eq!(keys.take(), ESCAPE);
        assert_eq!(keys.take(), 0);
    }

    #[test]
    fn arrows_report_the_keypads_codes() {
        // windib drops the 0xE0 prefix, so the menus see 0x48 for both Up and keypad 8.
        assert_eq!(scancode(Key::Up), scancode(Key::Kp8));
        assert_eq!(scancode(Key::KpEnter), ENTER);
        assert_eq!(scancode(Key::Y), Y);
        assert_eq!(scancode(Key::N), N);
    }

    #[test]
    fn a_held_key_repeats_after_half_a_second_then_every_third_tick() {
        // SDL 1.2's repeat, polled once a tick: the delay passes at the 36th poll (504 ms),
        // the first repeat comes 3 polls later (42 ms > 30 ms), then every 3 polls.
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        assert_eq!(keys.take(), DOWN);
        let polls: Vec<u32> = (1..=48)
            .filter(|_| {
                keys.tick();
                keys.take() == DOWN
            })
            .collect();
        assert_eq!(polls, [39, 42, 45, 48]);
    }

    #[test]
    fn releasing_the_key_or_holding_a_modifier_does_not_repeat() {
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        keys.event(key(Key::Down, false));
        keys.take();
        keys.event(key(Key::LeftShift, true));
        assert_eq!(keys.take(), 0x2A, "a modifier is still a key press");
        for _ in 0..100 {
            keys.tick();
            assert_eq!(keys.take(), 0);
        }
    }

    #[test]
    fn a_fresh_push_of_the_stick_holds_its_repeat_off_for_700_ms() {
        // The menus read every 2 ticks: a held push moves once, waits 700 ms, then moves on
        // every read.
        let mut keys = Keys::default();
        let reads: Vec<u8> = (0..60)
            .map(|read| {
                if read == 1 {
                    keys.event(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value: 20_000,
                    });
                }
                keys.tick();
                keys.tick();
                keys.take()
            })
            .collect();
        assert_eq!(reads[1], PAD_DOWN);
        // 700 ms is 25 reads of 28 ms; the hold-off ends at read 1 + 25.
        assert!(reads[2..26].iter().all(|&code| code == 0), "{reads:?}");
        assert!(
            reads[26..].iter().all(|&code| code == PAD_DOWN),
            "{reads:?}"
        );
    }

    #[test]
    fn pad_buttons_confirm_and_go_back() {
        // Buttons 0 and 2 answer like Enter, 1 and 3 like Escape.
        let mut keys = Keys::default();
        let push = |keys: &mut Keys, button, pressed| {
            keys.event(InputEvent::PadButton { button, pressed });
        };
        push(&mut keys, PadButton::A, true);
        assert_eq!(keys.take(), ENTER);
        push(&mut keys, PadButton::A, false);
        keys.tick();
        assert_eq!(keys.take(), 0);
        push(&mut keys, PadButton::B, true);
        keys.tick();
        assert_eq!(keys.take(), ESCAPE);
    }
}
