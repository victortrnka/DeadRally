//! The original's menu strings and font metrics, read from the player's `dr.exe` at the known
//! release's addresses (spec M2a §3.1). DeadRally ships none of the game's text.

use std::fmt;

use crate::exe::{Exe, ExeError};

/// Menus in the text table, and rows per menu.
pub const MENUS: usize = 9;
pub const MENU_ROWS: usize = 9;
/// The byte that draws nothing and moves the pen one pixel.
pub const GAP: u8 = 0xFA;

/// The menu text table: 50 bytes per row (`dr.exe` 0x446368).
const MENU_TABLE: u32 = 0x44_6368;
const MENU_ROW_BYTES: u32 = 50;
/// The bottom panel's start-up lines, in the order `mainMenu` (0x43A020) adds them; an empty
/// line comes between the third and the fourth.
const PANEL_LINES: [u32; 4] = [0x44_4370, 0x44_433C, 0x44_4300, 0x44_42C0];
const EXIT_QUESTION: u32 = 0x44_42B0;
const YES: u32 = 0x44_3CDC;
const NO: u32 = 0x44_3CD8;
/// Font descriptors: width, height, then one advance per glyph from character 32.
const BIG_METRICS: u32 = 0x44_5848;
const SMALL_METRICS: u32 = 0x44_58B0;
const MEDIUM_METRICS: u32 = 0x44_5928;
/// The longest string read anywhere but the menu table.
const MAX_LINE: usize = 150;
/// The fonts' cell sizes in the known release: big, small and medium.
const BIG_SIZE: (u8, u8) = (32, 32);
const SMALL_SIZE: (u8, u8) = (16, 16);
const MEDIUM_SIZE: (u8, u8) = (9, 12);
/// The main menu (0) and the start submenu (1) show six rows each, all of them text.
const SHOWN_MENUS: usize = 2;
const SHOWN_ROWS: usize = 6;

/// Configure's popups (spec M2b §3.2): the volume captions, the gamepad switch's two texts and
/// the popup when no gamepad is found.
const ADJUST_MUSIC: u32 = 0x44_3F90;
const ADJUST_EFFECTS: u32 = 0x44_3F6C;
const GAMEPAD_ON: u32 = 0x44_3F50;
const GAMEPAD_OFF: u32 = 0x44_3F34;
const NOT_DETECTED: u32 = 0x44_3170;
const PRESS_ANY_KEY: u32 = 0x44_29A4;
/// The eight controls' names (accelerate, brake, left, right, turbo, gun, mine, horn), with
/// the prompts for a key and, but for the horn, for a gamepad input.
const CONTROLS: [u32; 8] = [
    0x44_2AD0, 0x44_2A60, 0x44_2A4C, 0x44_2A34, 0x44_2A18, 0x44_29FC, 0x44_29E0, 0x44_29C0,
];
const KEY_PROMPTS: [u32; 8] = [
    0x44_3E34, 0x44_3E18, 0x44_3DF8, 0x44_3DD8, 0x44_3DB8, 0x44_3D98, 0x44_3D78, 0x44_3D60,
];
const PAD_PROMPTS: [u32; 7] = [
    0x44_3F14, 0x44_3EF8, 0x44_3ED8, 0x44_3EB8, 0x44_3E98, 0x44_3E74, 0x44_3E54,
];
/// The names of the gamepad inputs 0 (none) to 8, 16 bytes apart going down.
const PAD_NAMES: u32 = 0x44_3160;
pub const PAD_INPUTS: usize = 9;
/// Key names, 16 bytes apart going down from here in scancode order: 0x01..=0x54, then
/// `MORE_NAMED_KEYS`; one slot after 0xCB's holds a control's name. Other keys are
/// "unavailable".
const KEY_NAMES: u32 = 0x44_30C0;
const MORE_NAMED_KEYS: [u8; 17] = [
    0x57, 0x58, 0x9C, 0x9D, 0xB5, 0xB7, 0xB8, 0xC7, 0xC8, 0xC9, 0xCB, 0xCD, 0xCF, 0xD0, 0xD1, 0xD2,
    0xD3,
];
const UNAVAILABLE_KEY: u32 = 0x44_30D0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    Exe(ExeError),
    /// A byte that is neither printable ASCII nor the gap.
    Unprintable {
        address: u32,
    },
    /// An empty string where the menus show text, or a font of another size.
    Unexpected {
        address: u32,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::Exe(error) => write!(f, "dr.exe: {error}"),
            TextError::Unprintable { address } => write!(
                f,
                "dr.exe: the text at address {address:#x} is not the original's; is this the known release?"
            ),
            TextError::Unexpected { address } => write!(
                f,
                "dr.exe: the data at address {address:#x} is not where the known release keeps it; is this the known release?"
            ),
        }
    }
}

impl std::error::Error for TextError {}

impl From<ExeError> for TextError {
    fn from(error: ExeError) -> TextError {
        TextError::Exe(error)
    }
}

/// The address of scancode `code`'s name.
fn key_name(code: u8) -> u32 {
    match code {
        0x01..=0x54 => KEY_NAMES - 16 * (u32::from(code) - 1),
        _ => match MORE_NAMED_KEYS.iter().position(|&named| named == code) {
            Some(k) => {
                let skip = if code > 0xCB { 16 } else { 0 };
                KEY_NAMES - 16 * (0x54 + k as u32) - skip
            }
            None => UNAVAILABLE_KEY,
        },
    }
}

/// Configure's texts (spec M2b §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigureTexts {
    pub adjust_music: Vec<u8>,
    pub adjust_effects: Vec<u8>,
    pub gamepad_on: Vec<u8>,
    pub gamepad_off: Vec<u8>,
    pub not_detected: Vec<u8>,
    pub press_any_key: Vec<u8>,
    /// The eight controls, each padded so its key's name lines up.
    pub controls: Vec<Vec<u8>>,
    pub key_prompts: Vec<Vec<u8>>,
    pub pad_prompts: Vec<Vec<u8>>,
    /// `key_names[scancode]`, all 256.
    pub key_names: Vec<Vec<u8>>,
    /// `pad_names[input]`, 0 (none) to 8.
    pub pad_names: Vec<Vec<u8>>,
}

/// A font's cell size and the pen advance of each glyph, character 32 first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub width: u8,
    pub height: u8,
    pub advances: Vec<u8>,
}

/// Everything M2a reads from `dr.exe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texts {
    /// `menus[m][r]`: row `r` of menu `m`, empty where the menu has no such row.
    pub menus: Vec<Vec<Vec<u8>>>,
    /// The bottom panel's four start-up lines.
    pub panel: Vec<Vec<u8>>,
    pub exit_question: Vec<u8>,
    pub yes: Vec<u8>,
    pub no: Vec<u8>,
    pub big: Metrics,
    pub small: Metrics,
    pub medium: Metrics,
    pub configure: ConfigureTexts,
}

impl Texts {
    /// # Errors
    ///
    /// [`TextError`] when a string is missing, does not end, or holds a byte the original's
    /// strings never do, when a string the menus show is empty, or when a font's size is not
    /// the known release's: the executable is not the known release.
    pub fn read(exe: &Exe) -> Result<Texts, TextError> {
        let text = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = exe.string_at(address, max)?;
            if bytes.iter().all(|&b| (32..127).contains(&b) || b == GAP) {
                Ok(bytes.to_vec())
            } else {
                Err(TextError::Unprintable { address })
            }
        };
        let shown = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = text(address, max)?;
            if bytes.is_empty() {
                Err(TextError::Unexpected { address })
            } else {
                Ok(bytes)
            }
        };
        let all = |addresses: &[u32]| -> Result<Vec<Vec<u8>>, TextError> {
            addresses
                .iter()
                .map(|&address| shown(address, MAX_LINE))
                .collect()
        };
        let menus = (0..MENUS)
            .map(|menu| {
                (0..MENU_ROWS)
                    .map(|row| {
                        let address = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS * menu + row) as u32;
                        if menu < SHOWN_MENUS && row < SHOWN_ROWS {
                            shown(address, MENU_ROW_BYTES as usize - 1)
                        } else {
                            text(address, MENU_ROW_BYTES as usize - 1)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metrics = |address: u32, glyphs: usize, size: (u8, u8)| -> Result<Metrics, TextError> {
            let bytes = exe.bytes_at(address, 2 + glyphs)?;
            if (bytes[0], bytes[1]) != size {
                return Err(TextError::Unexpected { address });
            }
            Ok(Metrics {
                width: bytes[0],
                height: bytes[1],
                advances: bytes[2..].to_vec(),
            })
        };
        Ok(Texts {
            menus,
            panel: PANEL_LINES
                .iter()
                .map(|&address| shown(address, MAX_LINE))
                .collect::<Result<Vec<_>, _>>()?,
            exit_question: shown(EXIT_QUESTION, MAX_LINE)?,
            yes: shown(YES, MAX_LINE)?,
            no: shown(NO, MAX_LINE)?,
            configure: ConfigureTexts {
                adjust_music: shown(ADJUST_MUSIC, MAX_LINE)?,
                adjust_effects: shown(ADJUST_EFFECTS, MAX_LINE)?,
                gamepad_on: shown(GAMEPAD_ON, MAX_LINE)?,
                gamepad_off: shown(GAMEPAD_OFF, MAX_LINE)?,
                not_detected: shown(NOT_DETECTED, MAX_LINE)?,
                press_any_key: shown(PRESS_ANY_KEY, MAX_LINE)?,
                controls: all(&CONTROLS)?,
                key_prompts: all(&KEY_PROMPTS)?,
                pad_prompts: all(&PAD_PROMPTS)?,
                key_names: (0..=255)
                    .map(|code| shown(key_name(code), MAX_LINE))
                    .collect::<Result<_, _>>()?,
                pad_names: (0..PAD_INPUTS as u32)
                    .map(|input| shown(PAD_NAMES - 16 * input, MAX_LINE))
                    .collect::<Result<_, _>>()?,
            },
            big: metrics(BIG_METRICS, 96, BIG_SIZE)?,
            small: metrics(SMALL_METRICS, 96, SMALL_SIZE)?,
            medium: metrics(MEDIUM_METRICS, 62, MEDIUM_SIZE)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::{build, build_at};

    /// A section from 0x442000 to 0x448000 with every string and table where the known release
    /// keeps it: menu `m` row `r` reads "m.r", the other strings made-up words.
    fn known_layout() -> Vec<u8> {
        let mut data = vec![0u8; 0x6000];
        let mut put = |address: u32, bytes: &[u8]| {
            let at = (address - 0x44_2000) as usize;
            data[at..at + bytes.len()].copy_from_slice(bytes);
        };
        for menu in 0..MENUS as u32 {
            for row in 0..MENU_ROWS as u32 {
                let address = MENU_TABLE + MENU_ROW_BYTES * (9 * menu + row);
                put(address, format!("{menu}.{row}").as_bytes());
            }
        }
        for (index, &address) in PANEL_LINES.iter().enumerate() {
            put(address, format!("line {index}").as_bytes());
        }
        put(EXIT_QUESTION, b"Quit?");
        put(YES, b"Y");
        put(NO, b"N");
        put(BIG_METRICS, &[32, 32, 20, 9]);
        put(SMALL_METRICS, &[16, 16, 10, 5]);
        put(MEDIUM_METRICS, &[9, 12, 9, 9]);
        for address in [
            ADJUST_MUSIC,
            ADJUST_EFFECTS,
            GAMEPAD_ON,
            GAMEPAD_OFF,
            NOT_DETECTED,
            PRESS_ANY_KEY,
        ] {
            put(address, format!("text {address:x}").as_bytes());
        }
        for address in CONTROLS.iter().chain(&KEY_PROMPTS).chain(&PAD_PROMPTS) {
            put(*address, format!("text {address:x}").as_bytes());
        }
        for code in 0..=255 {
            put(key_name(code), format!("key {code:02x}").as_bytes());
        }
        for input in 0..PAD_INPUTS as u32 {
            put(PAD_NAMES - 16 * input, format!("pad {input}").as_bytes());
        }
        build_at(0x4_2000, 0x6000, &data)
    }

    #[test]
    fn menu_rows_are_fifty_bytes_apart_and_menus_nine_rows() {
        // A wrong stride shows another menu's text, or half of two rows.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.menus[0][0], b"0.0");
        assert_eq!(texts.menus[3][4], b"3.4");
        assert_eq!(texts.menus[8][8], b"8.8");
        assert_eq!(texts.panel[3], b"line 3");
        assert_eq!(
            (texts.exit_question.as_slice(), texts.yes.as_slice()),
            (b"Quit?".as_slice(), b"Y".as_slice())
        );
        assert_eq!(
            (texts.big.width, texts.big.height, &texts.big.advances[..2]),
            (32, 32, [20, 9].as_slice())
        );
        assert_eq!(texts.small.advances.len(), 96);
        assert_eq!(
            texts.medium.advances.len(),
            62,
            "the medium font has 62 glyphs"
        );
    }

    #[test]
    fn a_byte_the_original_never_uses_is_refused_with_its_address() {
        let mut bytes = known_layout();
        let at = offset(&bytes, YES);
        bytes[at] = 0x01;
        assert_eq!(
            Texts::read(&Exe::parse(bytes).unwrap()),
            Err(TextError::Unprintable { address: YES })
        );
    }

    #[test]
    fn each_key_has_its_name_and_keys_without_one_are_unavailable() {
        // Define Keyboard shows these names; one slot off names every key after it wrongly.
        // The slot after 0xCB's holds the accelerate control's name, so 0xCD skips it.
        assert_eq!(key_name(0x01), 0x44_30C0);
        assert_eq!(key_name(0x54), 0x44_30C0 - 16 * 0x53);
        assert_eq!(key_name(0x57), 0x44_2B80);
        assert_eq!(key_name(0xCB), 0x44_2AE0);
        assert_eq!(key_name(0xCD), 0x44_2AC0);
        assert_eq!(key_name(0xD3), 0x44_2A70);
        for code in [0x00, 0x55, 0x56, 0x59, 0xCC, 0xD4, 0xFF] {
            assert_eq!(key_name(code), UNAVAILABLE_KEY, "{code:#x}");
        }
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.configure.key_names[0x1E], b"key 1e");
        assert_eq!(texts.configure.pad_names[8], b"pad 8");
        assert_eq!(texts.configure.controls.len(), 8);
        assert_eq!(texts.configure.pad_prompts.len(), 7);
    }

    /// Where `address` lies in the bytes of [`known_layout`]'s file.
    fn offset(bytes: &[u8], address: u32) -> usize {
        bytes.len() - 0x6000 + (address - 0x44_2000) as usize
    }

    #[test]
    fn a_layout_moved_by_a_few_bytes_is_refused() {
        // Another build of dr.exe may keep the same strings a little further on. Every read
        // would then land on blanks, other strings' tails or other bytes that pass as text,
        // and the menus would show fragments: the rows the menus show and the fonts' sizes
        // must be where the known release keeps them.
        let mut moved = known_layout();
        let start = offset(&moved, 0x44_2000);
        moved[start..].rotate_right(3);
        let error = Texts::read(&Exe::parse(moved).unwrap()).unwrap_err();
        assert!(matches!(error, TextError::Unexpected { .. }), "{error}");
    }

    #[test]
    fn an_empty_shown_string_or_another_font_size_is_refused_with_its_address() {
        let mut no_yes = known_layout();
        let at = offset(&no_yes, YES);
        no_yes[at] = 0;
        assert_eq!(
            Texts::read(&Exe::parse(no_yes).unwrap()),
            Err(TextError::Unexpected { address: YES })
        );
        let mut empty_row = known_layout();
        let row = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS as u32 + 5);
        let at = offset(&empty_row, row);
        empty_row[at] = 0;
        assert_eq!(
            Texts::read(&Exe::parse(empty_row).unwrap()),
            Err(TextError::Unexpected { address: row }),
            "the start submenu's last row"
        );
        let mut narrow = known_layout();
        let at = offset(&narrow, SMALL_METRICS);
        narrow[at] = 15;
        assert_eq!(
            Texts::read(&Exe::parse(narrow).unwrap()),
            Err(TextError::Unexpected {
                address: SMALL_METRICS
            })
        );
    }

    #[test]
    fn a_file_that_is_not_the_known_release_is_refused_with_the_address() {
        // Another executable keeps different bytes at these addresses; drawing them would fill
        // the menus with garbage instead of saying what is wrong.
        let exe = Exe::parse(build(&[0x7F; 0x200])).unwrap();
        let error = Texts::read(&exe).unwrap_err();
        assert!(
            matches!(error, TextError::Exe(ExeError::Address(_))),
            "{error}"
        );
    }
}
