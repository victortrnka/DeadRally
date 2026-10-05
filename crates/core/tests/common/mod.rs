//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{ConfigureTexts, Metrics, Texts};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// `MENUBG5`, white in `MENU.PAL`.
pub const BACKGROUND: u8 = 1;
/// The big fonts: selected row (A), active row (B), dim row (D).
pub const BIG_A: u8 = 50;
pub const BIG_B: u8 = 51;
pub const BIG_D: u8 = 52;
/// The small fonts A, B and C.
pub const SMALL: [u8; 3] = [60, 61, 62];
/// Cursor frame k is colour `CURSOR + k`.
pub const CURSOR: u8 = 100;
/// The two credits screens and the end screen, each full red, green or blue in its palette.
pub const CREDITS: [u8; 2] = [200, 201];
pub const END: u8 = 202;
/// The menu's effects (`MEN-SAM`): the back sound plays on the left only, the move sound on
/// the right only, the choose sound on both sides.
pub const BACK_SOUND: usize = 22;
pub const MOVE_SOUND: usize = 25;
pub const CHOOSE_SOUND: usize = 28;
/// The volume popups' slider and knob.
pub const SLIDER: u8 = 70;
pub const KNOB: u8 = 71;

fn solid(width: u32, height: u32, colour: u8) -> Image {
    Image::new(width, height, vec![colour; (width * height) as usize])
}

fn glyphs(size: u32, colour: u8) -> Vec<Image> {
    (0..96).map(|_| solid(size, size, colour)).collect()
}

fn full_screen(colour: u8, rgb: [u8; 3]) -> Picture {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(colour)] = rgb;
    Picture {
        image: solid(640, 480, colour),
        palette,
    }
}

/// A short tone, panned (255 left, 0 right, 128 both).
fn sound(panning: u8) -> Instrument {
    Instrument {
        name: "Beep".into(),
        data: vec![2000; 2000],
        looping: Looping::None,
        volume: 64,
        finetune: 0,
        relative_note: 0,
        panning,
        fadeout: 0,
    }
}

fn texts() -> Texts {
    let metrics = |size: u8| Metrics {
        width: size,
        height: size,
        advances: vec![size; 96],
    };
    Texts {
        // Every menu has six one-letter rows.
        menus: vec![
            (0..9)
                .map(|row| if row < 6 { b"M".to_vec() } else { Vec::new() })
                .collect();
            9
        ],
        panel: (0..4).map(|line| vec![b'a' + line]).collect(),
        exit_question: b"?".to_vec(),
        yes: b"Y".to_vec(),
        no: b"N".to_vec(),
        big: metrics(32),
        small: metrics(16),
        medium: metrics(9),
        configure: ConfigureTexts {
            adjust_music: b"m".to_vec(),
            adjust_effects: b"e".to_vec(),
            gamepad_on: b"+".to_vec(),
            gamepad_off: b"-".to_vec(),
            not_detected: b"!".to_vec(),
            press_any_key: b".".to_vec(),
            controls: vec![b"c".to_vec(); 8],
            key_prompts: vec![b"k".to_vec(); 8],
            pad_prompts: vec![b"p".to_vec(); 7],
            key_names: vec![b"K".to_vec(); 256],
            pad_names: vec![b"P".to_vec(); 9],
        },
    }
}

pub fn menu_assets() -> MenuAssets {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(BACKGROUND)] = [63, 63, 63];
    // The pulsing entries.
    palette.0[16..32].fill([63, 63, 63]);
    let mut copper = Palette::BLACK;
    copper.0[0] = [63, 0, 32];
    let mut effects = vec![None; CHOOSE_SOUND];
    effects[BACK_SOUND - 1] = Some(sound(255));
    effects[MOVE_SOUND - 1] = Some(sound(0));
    effects[CHOOSE_SOUND - 1] = Some(sound(128));
    MenuAssets {
        background: solid(640, 480, BACKGROUND),
        panel_line: solid(640, 10, 99),
        corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
        corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
        cursor: (0..50).map(|k| solid(20, 20, CURSOR + k)).collect(),
        big_a: glyphs(32, BIG_A),
        big_b: glyphs(32, BIG_B),
        big_d: glyphs(32, BIG_D),
        small_a: glyphs(16, SMALL[0]),
        small_b: glyphs(16, SMALL[1]),
        small_c: glyphs(16, SMALL[2]),
        palette,
        copper,
        background_copper: (0..512).map(|row| [(row % 64) as u8, 0, 0]).collect(),
        credits: vec![
            full_screen(CREDITS[0], [63, 0, 0]),
            full_screen(CREDITS[1], [0, 63, 0]),
        ],
        end: full_screen(END, [0, 0, 63]),
        effects: Bank {
            linear_frequencies: true,
            instruments: effects,
        },
        texts: texts(),
        slider: solid(172, 24, SLIDER),
        knob: solid(10, 24, KNOB),
        default_config: DrCfg::parse(&[0; HEADER_BYTES + PAYLOAD_BYTES]).unwrap(),
    }
}
