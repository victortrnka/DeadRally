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
/// `MORE_NAMED_KEYS`; one slot after 0xCB's holds a control's name. Other keys share one
/// fallback name.
const KEY_NAMES: u32 = 0x44_30C0;
const MORE_NAMED_KEYS: [u8; 17] = [
    0x57, 0x58, 0x9C, 0x9D, 0xB5, 0xB7, 0xB8, 0xC7, 0xC8, 0xC9, 0xCB, 0xCD, 0xCF, 0xD0, 0xD1, 0xD2,
    0xD3,
];
const NAMELESS_KEY: u32 = 0x44_30D0;

/// The Hall of Fame (spec M2c §3): the 18 circuits' names (15 bytes apart), the six cars'
/// names (in the car table, 1760 bytes apart, car 5 last), the difficulties' names (24 bytes
/// apart) and the order the records screen steps through the circuits.
const CIRCUIT_NAMES: u32 = 0x44_D148;
pub const CIRCUITS: usize = 18;
const CAR_NAMES: u32 = 0x45_01B0;
pub const CARS: usize = 6;
const DIFFICULTY_NAMES: u32 = 0x44_7340;
const DIFFICULTIES: usize = 4;
const CIRCUIT_ORDER: u32 = 0x45_673C;

/// The twenty drivers' names as `initDrivers` (0x428930) copies them, by face.
const DRIVER_NAMES: [u32; DRIVERS] = [
    0x44_3AE8, 0x44_36E8, 0x44_1250, 0x44_36DC, 0x44_36D0, 0x44_3AE0, 0x44_36B8, 0x44_36AC,
    0x44_3AD4, 0x44_3694, 0x44_3AC8, 0x44_3ABC, 0x44_3AB0, 0x44_3AA4, 0x44_3A98, 0x44_3A8C,
    0x44_3A80, 0x44_3A74, 0x44_3A68, 0x44_3A5C,
];
pub const DRIVERS: usize = 20;
const DRIVER_NAME_MAX: usize = 10;
/// The cars' records, 0x6E0 bytes each from car 0: price at +0xC, the engine, tire and armour
/// upgrade counts at +0x6A0, their prices at +0x6AC (4 each), the repair price at +0x6DC.
const CAR_TABLE: u32 = 0x44_DF50;
const CAR_BYTES: u32 = 0x6E0;
/// The rows the Start Racing menu and the main menu get when a game starts or ends
/// (`startRacingMenu`, 0x439CD0).
const NEW_GAME_ROW: u32 = 0x44_3B04;
const START_RACING_ROW: u32 = 0x44_3AF4;
const ENTER_SHOP_ROW: u32 = 0x44_3D00;
const CONTINUE_RACING_ROW: u32 = 0x44_3CF0;
/// The licence (`licenseScreen` 0x434800, `readKeyboard` 0x42E7F0): the text cursor's glyph,
/// the difficulty popup's title, which characters a nickname may hold (one flag a byte), and
/// the price digits' cell and advances ("$", then 0 to 9).
const TEXT_CURSOR: u32 = 0x44_3C40;
const SELECT_DIFFICULTY: u32 = 0x44_40B4;
const NAME_CHARACTERS: u32 = 0x44_55B0;
const PRICE_METRICS: u32 = 0x44_5914;
const PRICE_SIZE: (u8, u8) = (16, 13);
/// The sign-up (`selectRaceScreen` 0x4357F0, `drawSelectRaceScreen` 0x423F40): the races'
/// prices, the welcome popup's lines (`welcomePopup` 0x41C840, 80 bytes apart) and its
/// "continue", the popup when the player signs up for no race, the warnings for a race too
/// hard for the car (`selectRaceWarningPopup` 0x42B1B0: five lines of 60 bytes each), and the
/// cars' top speeds by engine level (`drawCarRightSide` 0x41FC20).
const RACE_PRICES: [u32; 3] = [0x44_3500, 0x44_34F8, 0x44_34F0];
const WELCOME: u32 = 0x44_C1A8;
const WELCOME_LINES: u32 = 10;
const CONTINUE: u32 = 0x44_292C;
const NO_SIGN_UP: u32 = 0x44_40D8;
const RACE_WARNINGS: u32 = 0x45_0890;
const SPEEDS: u32 = 0x44_DED8;
/// The Start Racing menu's question before it ends a game (`startRacingMenu`, 0x439E97).
const END_GAME: u32 = 0x44_4280;

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
            None => NAMELESS_KEY,
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

/// The Hall of Fame's texts (spec M2c §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HallOfFameTexts {
    pub circuits: Vec<Vec<u8>>,
    /// `cars[k]`: car `k`, 0 to 5.
    pub cars: Vec<Vec<u8>>,
    /// `difficulties[d]`; the last is empty.
    pub difficulties: Vec<Vec<u8>>,
    /// The circuits in the order Left and Right step through them.
    pub circuit_order: Vec<u8>,
}

/// A car's prices and upgrades, as the original's car table holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CarSpec {
    pub price: i32,
    /// How many engine, tire and armour upgrades the car takes (1 to 4).
    pub upgrades: [i32; 3],
    pub upgrade_prices: [[i32; 4]; 3],
    pub repair_price: i32,
}

/// What the campaign reads from `dr.exe` (spec M3a §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignTexts {
    /// `driver_names[face]`.
    pub driver_names: Vec<Vec<u8>>,
    /// `cars[k]`, car 0 to 5.
    pub cars: Vec<CarSpec>,
    pub new_game_row: Vec<u8>,
    pub start_racing_row: Vec<u8>,
    pub enter_shop_row: Vec<u8>,
    pub continue_racing_row: Vec<u8>,
    pub text_cursor: Vec<u8>,
    pub select_difficulty: Vec<u8>,
    /// `name_characters[c]`: whether a nickname may hold character `c`.
    pub name_characters: Vec<bool>,
    /// The price digits' cell, and the advances of "$" and of 0 to 9.
    pub price: Metrics,
    pub race_prices: Vec<Vec<u8>>,
    /// The welcome popup's ten lines (some empty), with `writeTextInScreen`'s font codes.
    pub welcome: Vec<Vec<u8>>,
    pub continue_word: Vec<u8>,
    pub no_sign_up: Vec<u8>,
    /// `race_warnings[w][line]`: the medium race's warning (0) and the hard race's (1).
    pub race_warnings: Vec<Vec<Vec<u8>>>,
    /// `speeds[car][engine]`: the top speed the side panel shows.
    pub speeds: Vec<[i32; 5]>,
    pub end_game: Vec<u8>,
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
    pub hall_of_fame: HallOfFameTexts,
    pub campaign: CampaignTexts,
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
            if bytes.iter().all(|&b| (32..=127).contains(&b) || b == GAP) {
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
            hall_of_fame: HallOfFameTexts {
                circuits: (0..CIRCUITS as u32)
                    .map(|c| shown(CIRCUIT_NAMES + 15 * c, MAX_LINE))
                    .collect::<Result<_, _>>()?,
                cars: (0..CARS as u32)
                    .map(|k| shown(CAR_NAMES - 1760 * (CARS as u32 - 1 - k), MAX_LINE))
                    .collect::<Result<_, _>>()?,
                difficulties: (0..DIFFICULTIES as u32)
                    .map(|d| text(DIFFICULTY_NAMES + 24 * d, MAX_LINE))
                    .collect::<Result<_, _>>()?,
                circuit_order: {
                    let order = exe.bytes_at(CIRCUIT_ORDER, CIRCUITS)?.to_vec();
                    if order.iter().any(|&c| usize::from(c) >= CIRCUITS) {
                        return Err(TextError::Unexpected {
                            address: CIRCUIT_ORDER,
                        });
                    }
                    order
                },
            },
            campaign: CampaignTexts {
                driver_names: DRIVER_NAMES
                    .iter()
                    .map(|&address| shown(address, DRIVER_NAME_MAX))
                    .collect::<Result<_, _>>()?,
                cars: (0..CARS as u32)
                    .map(|k| car_spec(exe, CAR_TABLE + CAR_BYTES * k))
                    .collect::<Result<_, _>>()?,
                new_game_row: shown(NEW_GAME_ROW, MENU_ROW_BYTES as usize - 1)?,
                start_racing_row: shown(START_RACING_ROW, MENU_ROW_BYTES as usize - 1)?,
                enter_shop_row: shown(ENTER_SHOP_ROW, MENU_ROW_BYTES as usize - 1)?,
                continue_racing_row: shown(CONTINUE_RACING_ROW, MENU_ROW_BYTES as usize - 1)?,
                text_cursor: shown(TEXT_CURSOR, 1)?,
                select_difficulty: shown(SELECT_DIFFICULTY, MAX_LINE)?,
                name_characters: exe
                    .bytes_at(NAME_CHARACTERS, 256)?
                    .iter()
                    .map(|&flag| flag == 1)
                    .collect(),
                price: metrics(PRICE_METRICS, 11, PRICE_SIZE)?,
                race_prices: RACE_PRICES
                    .iter()
                    .map(|&address| shown(address, MAX_LINE))
                    .collect::<Result<_, _>>()?,
                welcome: (0..WELCOME_LINES)
                    .map(|line| text(WELCOME + 80 * line, 79))
                    .collect::<Result<_, _>>()?,
                continue_word: shown(CONTINUE, MAX_LINE)?,
                end_game: shown(END_GAME, MAX_LINE)?,
                no_sign_up: shown(NO_SIGN_UP, MAX_LINE)?,
                race_warnings: (0..2)
                    .map(|warning| {
                        (0..5)
                            .map(|line| text(RACE_WARNINGS + 300 * warning + 60 * line, 59))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<_, _>>()?,
                speeds: (0..CARS as u32)
                    .map(|car| -> Result<[i32; 5], TextError> {
                        let b = exe.bytes_at(SPEEDS + 20 * car, 20)?;
                        Ok(std::array::from_fn(|level| {
                            i32::from_le_bytes([
                                b[4 * level],
                                b[4 * level + 1],
                                b[4 * level + 2],
                                b[4 * level + 3],
                            ])
                        }))
                    })
                    .collect::<Result<_, _>>()?,
            },
            big: metrics(BIG_METRICS, 96, BIG_SIZE)?,
            small: metrics(SMALL_METRICS, 96, SMALL_SIZE)?,
            medium: metrics(MEDIUM_METRICS, 62, MEDIUM_SIZE)?,
        })
    }
}

/// Car record `base`; refused when its upgrade counts are not 1 to 4 or a price is negative.
fn car_spec(exe: &Exe, base: u32) -> Result<CarSpec, TextError> {
    let value = |offset: u32| -> Result<i32, TextError> {
        let b = exe.bytes_at(base + offset, 4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut prices = [[0; 4]; 3];
    for (kind, row) in prices.iter_mut().enumerate() {
        for (level, price) in row.iter_mut().enumerate() {
            *price = value(0x6AC + 16 * kind as u32 + 4 * level as u32)?;
        }
    }
    let spec = CarSpec {
        price: value(0xC)?,
        upgrades: [value(0x6A0)?, value(0x6A4)?, value(0x6A8)?],
        upgrade_prices: prices,
        repair_price: value(0x6DC)?,
    };
    if spec.upgrades.iter().any(|count| !(1..=4).contains(count))
        || spec.price < 0
        || spec.repair_price < 0
        || prices.iter().flatten().any(|&price| price < 0)
    {
        return Err(TextError::Unexpected { address: base });
    }
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::{build, build_at};

    /// A section from 0x441000 to 0x457000 with every string and table where the known release
    /// keeps it: menu `m` row `r` reads "m.r", the other strings made-up words.
    fn known_layout() -> Vec<u8> {
        let mut data = vec![0u8; 0x1_6000];
        let mut put = |address: u32, bytes: &[u8]| {
            let at = (address - 0x44_1000) as usize;
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
        for c in 0..CIRCUITS as u32 {
            put(CIRCUIT_NAMES + 15 * c, format!("circuit {c}").as_bytes());
        }
        for k in 0..CARS as u32 {
            put(CAR_NAMES - 1760 * k, format!("car {k}").as_bytes());
        }
        for d in 0..3 {
            put(DIFFICULTY_NAMES + 24 * d, format!("level {d}").as_bytes());
        }
        put(
            CIRCUIT_ORDER,
            &[0, 7, 5, 3, 4, 2, 8, 1, 6, 9, 16, 14, 12, 13, 11, 17, 10, 15],
        );
        for (face, &address) in DRIVER_NAMES.iter().enumerate() {
            put(address, format!("d{face}").as_bytes());
        }
        for k in 0..CARS as u32 {
            let base = CAR_TABLE + CAR_BYTES * k;
            let k = k as i32;
            put(base + 0xC, &(1000 * (k + 1)).to_le_bytes());
            for (kind, count) in [(0, 1 + k % 4), (1, 2), (2, 1 + (k + 1) % 4)] {
                put(base + 0x6A0 + 4 * kind, &count.to_le_bytes());
                for level in 0..4 {
                    let price = 100 * (k + 1) + 10 * kind as i32 + level;
                    put(
                        base + 0x6AC + 16 * kind + 4 * level as u32,
                        &price.to_le_bytes(),
                    );
                }
            }
            put(base + 0x6DC, &(7 * (k + 1)).to_le_bytes());
        }
        put(NEW_GAME_ROW, b"new");
        put(START_RACING_ROW, b"start");
        put(ENTER_SHOP_ROW, b"shop");
        put(CONTINUE_RACING_ROW, b"continue");
        put(TEXT_CURSOR, &[0x7F]);
        put(SELECT_DIFFICULTY, b"select");
        let mut characters = [0u8; 256];
        characters[32..127].fill(1);
        put(NAME_CHARACTERS, &characters);
        put(
            PRICE_METRICS,
            &[16, 13, 14, 13, 9, 13, 13, 13, 13, 13, 12, 13, 13],
        );
        for (race, &address) in RACE_PRICES.iter().enumerate() {
            put(address, format!("${race}").as_bytes());
        }
        for line in 0..WELCOME_LINES {
            put(WELCOME + 80 * line, format!("w{line}").as_bytes());
        }
        put(CONTINUE, b"go");
        put(END_GAME, b"end?");
        put(NO_SIGN_UP, b"none");
        for warning in 0..2u32 {
            for line in 1..5u32 {
                put(
                    RACE_WARNINGS + 300 * warning + 60 * line,
                    format!("r{warning}{line}").as_bytes(),
                );
            }
        }
        for car in 0..CARS as u32 {
            for level in 0..5u32 {
                put(
                    SPEEDS + 20 * car + 4 * level,
                    &(10 * car + level).to_le_bytes(),
                );
            }
        }
        build_at(0x4_1000, 0x1_6000, &data)
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
    fn each_key_has_its_name_and_keys_without_one_share_a_fallback() {
        // Define Keyboard shows these names; one slot off names every key after it wrongly.
        // The slot after 0xCB's holds the accelerate control's name, so 0xCD skips it.
        assert_eq!(key_name(0x01), 0x44_30C0);
        assert_eq!(key_name(0x54), 0x44_30C0 - 16 * 0x53);
        assert_eq!(key_name(0x57), 0x44_2B80);
        assert_eq!(key_name(0xCB), 0x44_2AE0);
        assert_eq!(key_name(0xCD), 0x44_2AC0);
        assert_eq!(key_name(0xD3), 0x44_2A70);
        for code in [0x00, 0x55, 0x56, 0x59, 0xCC, 0xD4, 0xFF] {
            assert_eq!(key_name(code), NAMELESS_KEY, "{code:#x}");
        }
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.configure.key_names[0x1E], b"key 1e");
        assert_eq!(texts.configure.pad_names[8], b"pad 8");
        assert_eq!(texts.configure.controls.len(), 8);
        assert_eq!(texts.configure.pad_prompts.len(), 7);
    }

    #[test]
    fn the_hall_of_fames_names_and_circuit_order_come_from_their_tables() {
        // Car 5 is the first in the table's order of reading: a wrong stride names every row's
        // car wrongly; a circuit order naming a circuit past 17 is not the known release.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        let hall = &texts.hall_of_fame;
        assert_eq!(hall.circuits[17], b"circuit 17");
        assert_eq!(
            (hall.cars[5].as_slice(), hall.cars[0].as_slice()),
            (b"car 0".as_slice(), b"car 5".as_slice())
        );
        assert_eq!(hall.difficulties[1], b"level 1");
        assert!(hall.difficulties[3].is_empty());
        assert_eq!(hall.circuit_order[1], 7);
        let mut bad = known_layout();
        let at = offset(&bad, CIRCUIT_ORDER);
        bad[at] = 18;
        assert_eq!(
            Texts::read(&Exe::parse(bad).unwrap()),
            Err(TextError::Unexpected {
                address: CIRCUIT_ORDER
            })
        );
    }

    /// Where `address` lies in the bytes of [`known_layout`]'s file.
    fn offset(bytes: &[u8], address: u32) -> usize {
        bytes.len() - 0x1_6000 + (address - 0x44_1000) as usize
    }

    #[test]
    fn the_drivers_names_and_the_cars_prices_come_from_their_tables() {
        // Each driver is named by face; a wrong address names a driver after another one's
        // tail. A car read with the wrong stride or offset prices every purchase wrongly.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        let campaign = &texts.campaign;
        assert_eq!(campaign.driver_names[0], b"d0");
        assert_eq!(campaign.driver_names[19], b"d19");
        assert_eq!(
            campaign.cars[5],
            CarSpec {
                price: 6000,
                upgrades: [2, 2, 3],
                upgrade_prices: [
                    [600, 601, 602, 603],
                    [610, 611, 612, 613],
                    [620, 621, 622, 623]
                ],
                repair_price: 42,
            }
        );
        assert_eq!(campaign.enter_shop_row, b"shop");
        assert_eq!(campaign.text_cursor, [0x7F], "the cursor is glyph 127");
        assert_eq!(campaign.speeds[3][2], 32, "car 3 at engine level 2");
        assert_eq!(campaign.race_warnings[1][4], b"r14");
        assert!(campaign.race_warnings[0][0].is_empty());
        assert!(campaign.name_characters[usize::from(b'a')]);
        assert!(!campaign.name_characters[0x7F]);
        assert_eq!(
            (campaign.price.advances[0], campaign.price.advances[2]),
            (14, 9),
            "the dollar sign, then the digits from 0"
        );
    }

    #[test]
    fn a_car_with_impossible_upgrades_is_refused() {
        // Upgrade counts index the shop's rows; a count past 4 is not the known release.
        let mut bytes = known_layout();
        let at = offset(&bytes, CAR_TABLE + CAR_BYTES * 2 + 0x6A4);
        bytes[at] = 5;
        assert_eq!(
            Texts::read(&Exe::parse(bytes).unwrap()),
            Err(TextError::Unexpected {
                address: CAR_TABLE + CAR_BYTES * 2
            })
        );
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
