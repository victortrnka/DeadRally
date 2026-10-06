//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::dr_cfg::{DrCfg, HEADER_BYTES, PAYLOAD_BYTES};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{
    CampaignTexts, CarSpec, ConfigureTexts, HallOfFameTexts, Metrics, ShopTexts, Texts,
};
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
/// The licence's pictures from 203 on, the faces from 76, the turning car's frames from 231
/// (eight colours round), the sign-up's pictures from 239.
pub const LICENCE: u8 = 203;
pub const FACE: u8 = 76;
pub const TURNING: u8 = 231;
pub const SIGN_UP: u8 = 239;
/// The shop's pictures from 53 on.
pub const SHOP: u8 = 53;
/// The drug dealer's and the hitman's pictures in their offers.
pub const DRUG_DEALER: u8 = SHOP + 30;
pub const HITMAN: u8 = SHOP + 31;
/// The race's preview: its banner, the grid's frame, the circuit's picture.
pub const PREVIEW: u8 = 110;

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
        campaign: CampaignTexts {
            driver_names: (0..20)
                .map(|face| format!("N{face}").into_bytes())
                .collect(),
            cars: (0..6)
                .map(|k| CarSpec {
                    price: 500 * (k + 1),
                    upgrades: [1 + k % 4, 2, 4],
                    upgrade_prices: [[100; 4]; 3],
                    repair_price: 10,
                })
                .collect(),
            new_game_row: b"M".to_vec(),
            start_racing_row: b"M".to_vec(),
            enter_shop_row: b"S".to_vec(),
            continue_racing_row: b"R".to_vec(),
            text_cursor: vec![0x7F],
            select_difficulty: b"D".to_vec(),
            name_characters: (0..256).map(|c| (32..127).contains(&c)).collect(),
            price: Metrics {
                width: 16,
                height: 13,
                advances: vec![14, 13, 9, 13, 13, 13, 13, 13, 12, 13, 13],
            },
            race_prices: vec![b"$1".to_vec(), b"$2".to_vec(), b"$3".to_vec()],
            welcome: vec![b"w".to_vec(); 10],
            continue_word: b"C".to_vec(),
            end_game: b"E".to_vec(),
            empty_slot: b"-".to_vec(),
            quicksave_slot: b"Q".to_vec(),
            game_loaded: b"L".to_vec(),
            game_saved: b"S".to_vec(),
            save_prompt: b"?".to_vec(),
            no_sign_up: b"0".to_vec(),
            race_warnings: vec![vec![b"x".to_vec(); 5]; 2],
            speeds: vec![[55, 60, 65, 70, 75]; 6],
            sabotage: vec![b"s".to_vec(); 8],
            game_not_found: b"?".to_vec(),
            drug_offer: vec![b"d".to_vec(); 11],
            hitman_offer: vec![b"h".to_vec(); 11],
            laps: b"L".to_vec(),
            prize: b"P".to_vec(),
        },
        shop: shop_texts(),
    }
}

/// Every shop description one line of one letter.
pub fn shop_texts() -> ShopTexts {
    let info = || vec![b"i".to_vec(); 6];
    ShopTexts {
        cars: (0..6).map(|_| [info(), info()]).collect(),
        engines: (0..6).map(|_| (0..4).map(|_| info()).collect()).collect(),
        engine_max: info(),
        tires: (0..4).map(|_| info()).collect(),
        tire_max: info(),
        armours: (0..4).map(|_| info()).collect(),
        armour_max: info(),
        repairs: (0..12).map(|_| info()).collect(),
        repair_ten: b"10".to_vec(),
        continues: [info(), info()],
        bought: (0..3).map(|_| (0..4).map(|_| info()).collect()).collect(),
        short: [b"<".to_vec(), b">".to_vec(), b"^".to_vec(), b"v".to_vec()],
        wrecked: vec![b"w".to_vec(); 5],
        offer: (0..8).map(|k| vec![b'a' + k]).collect(),
        paint: vec![b"p".to_vec(); 3],
        car_bought: (0..6).map(|_| info()).collect(),
        weapons: (0..4).map(|_| info()).collect(),
        weapons_bought: (0..4).map(|_| info()).collect(),
        out_of_stock: info(),
        shareware: info(),
        market_on: info(),
        market_wrecked: vec![b"w".to_vec(); 5],
        loan_offers: (0..5).map(|_| info()).collect(),
        loans_granted: (0..5).map(|_| info()).collect(),
        loan_owed: info(),
        loan_refused: info(),
        loan_paid: info(),
        market_welcome: vec![b"m".to_vec(); 10],
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
        licence: solid(530, 249, LICENCE),
        face_frame: solid(68, 102, LICENCE + 1),
        faces: (0..20).map(|face| solid(64, 64, FACE + face)).collect(),
        face_arrows: (0..4).map(|k| solid(68, 16, LICENCE + 2 + k)).collect(),
        car_box: solid(96, 96, LICENCE + 6),
        car_names: (0..6).map(|k| solid(96, 16, LICENCE + 7 + k)).collect(),
        car_turning: (0..6)
            .map(|_| (0..64).map(|k| solid(96, 64, TURNING + k % 8)).collect())
            .collect(),
        colour_slider: solid(294, 16, LICENCE + 13),
        colour_knob: solid(10, 24, LICENCE + 14),
        price_digits: (0..11).map(|k| solid(16, 13, LICENCE + 15 + k)).collect(),
        sign_up_title: solid(640, 32, SIGN_UP),
        side_panel: solid(96, 224, SIGN_UP + 1),
        side_cars: (0..6).map(|k| solid(96, 64, SIGN_UP + 2 + k)).collect(),
        upgrade_lamps: (0..6).map(|k| solid(20, 10, SIGN_UP + 8 + k)).collect(),
        sign_line: solid(136, 2, SIGN_UP + 14),
        shop_title: solid(640, 16, SHOP),
        car_arrows: (0..4).map(|k| solid(16, 64, SHOP + 1 + k)).collect(),
        item_boxes: (0..5).map(|k| solid(96, 96, SHOP + 5 + k)).collect(),
        engines: (0..4)
            .map(|_| (0..24).map(|_| solid(96, 64, SHOP + 10)).collect())
            .collect(),
        tires: (0..4)
            .map(|_| (0..12).map(|_| solid(96, 64, SHOP + 11)).collect())
            .collect(),
        armours: (0..4)
            .map(|_| (0..16).map(|_| solid(96, 64, SHOP + 12)).collect())
            .collect(),
        repair: (0..24).map(|_| solid(96, 64, SHOP + 13)).collect(),
        continue_flag: (0..23).map(|_| solid(96, 64, SHOP + 14)).collect(),
        maxed: (0..12).map(|_| solid(96, 64, SHOP + 15)).collect(),
        market_title: solid(640, 16, SHOP + 16),
        loan_shark: solid(96, 96, SHOP + 17),
        weapons: (0..12).map(|k| solid(96, 96, SHOP + 18 + k)).collect(),
        market_prices: vec![[150, 200, 275, 250]; 6],
        car_colours: Palette::BLACK,
        drug_dealer: solid(104, 128, DRUG_DEALER),
        hitman: solid(104, 128, HITMAN),
        preview_banner: solid(640, 54, PREVIEW),
        preview_grid: solid(225, 274, PREVIEW + 1),
        track_shapes: (0..19).map(|_| solid(360, 274, PREVIEW + 2)).collect(),
    }
}

/// No tracks, cars or boards: the races' archives empty.
pub fn race_archives() -> deadrally_gamedata::race::RaceArchives {
    let empty = |name: &str| {
        deadrally_gamedata::bpa::Archive::from_bytes(
            name.into(),
            vec![0; deadrally_gamedata::bpa::DATA_START],
        )
        .expect("an empty archive")
    };
    deadrally_gamedata::race::RaceArchives {
        tracks: (0..10).map(|n| empty(&format!("TR{n}.BPA"))).collect(),
        engine: empty("ENGINE.BPA"),
        ib_files: empty("IBFILES.BPA"),
    }
}
