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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    Exe(ExeError),
    /// A byte that is neither printable ASCII nor the gap.
    Unprintable {
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
        }
    }
}

impl std::error::Error for TextError {}

impl From<ExeError> for TextError {
    fn from(error: ExeError) -> TextError {
        TextError::Exe(error)
    }
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
}

impl Texts {
    /// # Errors
    ///
    /// [`TextError`] when a string is missing, does not end, or holds a byte the original's
    /// strings never do: the executable is not the known release.
    pub fn read(exe: &Exe) -> Result<Texts, TextError> {
        let text = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = exe.string_at(address, max)?;
            if bytes.iter().all(|&b| (32..127).contains(&b) || b == GAP) {
                Ok(bytes.to_vec())
            } else {
                Err(TextError::Unprintable { address })
            }
        };
        let menus = (0..MENUS as u32)
            .map(|menu| {
                (0..MENU_ROWS as u32)
                    .map(|row| {
                        let address = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS as u32 * menu + row);
                        text(address, MENU_ROW_BYTES as usize - 1)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metrics = |address: u32, glyphs: usize| -> Result<Metrics, TextError> {
            let bytes = exe.bytes_at(address, 2 + glyphs)?;
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
                .map(|&address| text(address, MAX_LINE))
                .collect::<Result<Vec<_>, _>>()?,
            exit_question: text(EXIT_QUESTION, MAX_LINE)?,
            yes: text(YES, MAX_LINE)?,
            no: text(NO, MAX_LINE)?,
            big: metrics(BIG_METRICS, 96)?,
            small: metrics(SMALL_METRICS, 96)?,
            medium: metrics(MEDIUM_METRICS, 62)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::{build, build_at};

    /// A section from 0x443000 to 0x448000 with every string and table where the known release
    /// keeps it: menu `m` row `r` reads "m.r", the other strings their address's last digit.
    fn known_layout() -> Vec<u8> {
        let mut data = vec![0u8; 0x5000];
        let mut put = |address: u32, bytes: &[u8]| {
            let at = (address - 0x44_3000) as usize;
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
        build_at(0x4_3000, 0x5000, &data)
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
        let at = bytes.len() - 0x5000 + (YES - 0x44_3000) as usize;
        bytes[at] = 0x01;
        assert_eq!(
            Texts::read(&Exe::parse(bytes).unwrap()),
            Err(TextError::Unprintable { address: YES })
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
