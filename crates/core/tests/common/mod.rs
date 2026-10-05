//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{ConfigureTexts, HallOfFameTexts, Metrics, Texts};
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
/// The Hall of Fame: the medium font, the titles, the record bar, circuit c's snapshot
/// (`SNAPSHOT + c`), the four arrows (`ARROW + k`) and the border's corners.
pub const MEDIUM: u8 = 72;
pub const FAME_TITLE: u8 = 73;
pub const RECORDS_TITLE: u8 = 74;
pub const RECORDS_BAR: u8 = 75;
pub const SNAPSHOT: u8 = 150;
pub const ARROW: u8 = 180;
pub const BORDER: u8 = 190;

/// A `dr.cfg` with the original's default volumes, gamepad off.
pub fn config() -> DrCfg {
    let mut config = DrCfg::parse(&[0; HEADER_BYTES + PAYLOAD_BYTES]).unwrap();
    config.set_music_volume(0x8000);
    config.set_effects_volume(0xC000);
    config
}

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
        hall_of_fame: HallOfFameTexts {
            circuits: vec![b"C".to_vec(); 18],
            cars: vec![b"V".to_vec(); 6],
            difficulties: vec![b"D".to_vec(); 4],
            // The original's order starts 0, 7, 5.
            circuit_order: vec![0, 7, 5, 3, 4, 2, 8, 1, 6, 9, 16, 14, 12, 13, 11, 17, 10, 15],
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
        default_config: config(),
        medium: glyphs(9, MEDIUM),
        fame_title: solid(640, 54, FAME_TITLE),
        records_title: solid(640, 16, RECORDS_TITLE),
        records_bar: solid(640, 68, RECORDS_BAR),
        snapshots: (0..20).map(|c| solid(128, 112, SNAPSHOT + c)).collect(),
        arrows: (0..4).map(|k| solid(16, 84, ARROW + k)).collect(),
        // Frame k covers its tile's rows from k on: frame 0 all of it, so every column is
        // covered once the band's first tile column has passed over it.
        wipe: (0..10u32)
            .map(|k| {
                let pixels = (0..225).map(|i| u8::from(i / 15 >= k)).collect();
                Image::new(15, 15, pixels)
            })
            .collect(),
        border_corners: (0..4).map(|_| solid(24, 24, BORDER)).collect(),
    }
}
