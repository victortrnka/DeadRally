/// Something the player did. Frontends forward state changes only (no OS key repeat) and keep
/// presentation keys (Alt+Enter, F12) to themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Key {
        key: Key,
        pressed: bool,
    },
    PadButton {
        button: PadButton,
        pressed: bool,
    },
    /// Stick position from -32768 (left or up) to 32767 (right or down).
    PadAxis {
        axis: PadAxis,
        value: i16,
    },
}

/// The four face buttons of "one stick, four buttons" (brief §1), named by position with Xbox
/// labels: A bottom, B right, X left, Y top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PadButton {
    A,
    B,
    X,
    Y,
}

impl PadButton {
    /// Every button; `PadButton::ALL[b as usize] == b`.
    pub const ALL: [PadButton; 4] = [PadButton::A, PadButton::B, PadButton::X, PadButton::Y];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PadAxis {
    StickX,
    StickY,
}

macro_rules! keys {
    ($($key:ident),+ $(,)?) => {
        /// A physical key position named after the US layout, independent of the active
        /// keyboard layout. M2 adds the mapping to the original's scancodes.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Key {
            $($key),+
        }

        impl Key {
            /// Every key in declaration order; `Key::ALL[k as usize] == k`.
            pub const ALL: &'static [Key] = &[$(Key::$key),+];
        }
    };
}

keys![
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Digit0, Digit1,
    Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, F1, F2, F3, F4, F5, F6, F7, F8,
    F9, F10, F11, F12, Up, Down, Left, Right, Enter, Escape, Space, Backspace, Tab, LeftShift,
    RightShift, LeftCtrl, RightCtrl, LeftAlt, RightAlt, Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7,
    Kp8, Kp9, KpPlus, KpMinus, KpMultiply, KpDivide, KpEnter, KpPeriod,
];
