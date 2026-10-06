//! The main menu on synthetic assets (spec M2a §3.2–§3.5): what a player of the original sees
//! and hears while moving through it. Every picture of the fixture has a colour of its own
//! (`common`), so a pixel tells which font, cursor frame or screen is drawn there.

mod common;

use common::{
    ARROW, BACKGROUND, BIG_A, BIG_B, BIG_D, CREDITS, CURSOR, END, FAME_TITLE, KNOB, MEDIUM,
    RECORDS_TITLE, SLIDER, SMALL, SNAPSHOT,
};
use deadrally_core::{Game, InputEvent, Key, PadAxis, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::dr_cfg::DrCfg;
use deadrally_gamedata::haf::Animation;
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::Bank;

/// Without an intro: two logos of 25 + 180 + 26 ticks, the title's 25-tick fade-in, its
/// 26-tick fade to black and the menu's 50-tick fade-in.
const MENU_SHOWN: u32 = 2 * (25 + 180 + 26) + 25 + 26 + 50;
/// Where the main menu and the start submenu stand.
const MAIN: (usize, usize) = (145, 124);
const START: (usize, usize) = (109, 171);
const CONFIGURE: (usize, usize) = (95, 146);
/// Inside the exit question's "yes" and "no".
const YES: (usize, usize) = (212, 241);
const NO: (usize, usize) = (382, 241);

/// Which sides an effect sounded on: the fixture's back sound is left only, the move sound
/// right only, the choose sound both.
const SILENT: (bool, bool) = (false, false);
const BACK: (bool, bool) = (true, false);
const MOVE: (bool, bool) = (false, true);
const CHOOSE: (bool, bool) = (true, true);

fn picture(pixel: u8, width: u32, height: u32) -> Picture {
    Picture {
        image: Image::new(width, height, vec![pixel; (width * height) as usize]),
        palette: Palette::BLACK,
    }
}

/// A module that is silent, or plays one endless tone from order 45 on, where the original
/// starts the menu music.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let silent = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    let mut playing = silent.clone();
    playing.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone {
            [vec![1; 45], vec![0]].concat()
        } else {
            Vec::new()
        },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![playing, silent],
    }
}

fn assets() -> Assets {
    Assets {
        intro: Animation::from_frames(Vec::new(), Vec::new()),
        letterbox: picture(0, 320, 200),
        apogee: picture(1, 4, 3),
        remedy: picture(2, 4, 3),
        title: picture(3, 640, 480),
        intro_music: music(false),
        intro_effects: Bank {
            linear_frequencies: true,
            instruments: Vec::new(),
        },
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// A game in the main menu, its fade-in over.
fn in_menu(assets: Assets) -> Game {
    in_menu_with(assets, common::config())
}

fn in_menu_with(assets: Assets, config: DrCfg) -> Game {
    let mut game = Game::new(assets, config);
    run(&mut game, MENU_SHOWN);
    game
}

fn press(game: &mut Game, key: Key) {
    game.input(InputEvent::Key { key, pressed: true });
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

/// Presses `key` and runs two menu passes: the first reads it, the second shows the result
/// everywhere (the exit question copies its words to the screen a pass later).
fn step(game: &mut Game, key: Key) {
    press(game, key);
    run(game, 4);
}

/// Waits for earlier effects to end, presses `key`, and tells which sides sounded.
fn sound_after(game: &mut Game, key: Key) -> (bool, bool) {
    run(game, 100);
    game.take_audio(&mut Vec::new());
    step(game, key);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    let side = |side: usize| audio.iter().skip(side).step_by(2).any(|&s| s != 0);
    (side(0), side(1))
}

fn pixel(game: &Game, (x, y): (usize, usize)) -> u8 {
    game.frame().pixels[y * 640 + x]
}

/// The font each of a menu's six rows is drawn in, read inside the row's glyph.
fn row_fonts(game: &Game, (x, y): (usize, usize)) -> Vec<u8> {
    (0..6)
        .map(|row| pixel(game, (x + 33, y + 15 + 28 * row)))
        .collect()
}

/// The main menu's highlighted row.
fn selected(game: &Game) -> usize {
    let fonts = row_fonts(game, MAIN);
    fonts
        .iter()
        .position(|&font| font == BIG_A)
        .unwrap_or_else(|| panic!("no row highlighted: {fonts:?}"))
}

/// How the shown palette makes `colour`.
fn colour(game: &Game, colour: u8) -> [u8; 3] {
    game.frame().palette[usize::from(colour)]
}

#[test]
fn the_menu_fades_in_after_the_title_and_stays_just_below_full_brightness() {
    // The original's fade-in stops at 98 %: white shows as 62, never 63.
    let mut game = Game::new(assets(), common::config());
    run(&mut game, MENU_SHOWN - 1);
    let before = colour(&game, BACKGROUND);
    game.tick();
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (640, 480, (4, 3))
    );
    assert_eq!(pixel(&game, (5, 5)), BACKGROUND);
    assert!(before[0] < 62, "{before:?}");
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    run(&mut game, 300);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
}

#[test]
fn the_main_menu_highlights_start_and_dims_multiplayer() {
    let game = in_menu(assets());
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    let cursor = pixel(&game, (MAIN.0 + 15, MAIN.1 + 16));
    assert!((CURSOR..CURSOR + 50).contains(&cursor), "{cursor}");
}

#[test]
fn the_bottom_panel_shows_the_start_up_lines_with_a_gap_before_the_last() {
    // `mainMenu` pushes three lines, an empty one and the last into the 22-line panel, which
    // shows its last six at (12, 378 + 15k) in small B.
    let game = in_menu(assets());
    let line = |k: usize| pixel(&game, (13, 378 + 15 * k + 8));
    assert_eq!(
        (0..6).map(line).collect::<Vec<_>>(),
        [
            BACKGROUND, SMALL[1], SMALL[1], SMALL[1], BACKGROUND, SMALL[1]
        ]
    );
}

#[test]
fn up_and_down_move_the_highlight_past_the_inactive_row_and_around() {
    let mut game = in_menu(assets());
    let rows: Vec<usize> = [Key::Down, Key::Down, Key::Up, Key::Up, Key::Up, Key::Down]
        .into_iter()
        .map(|key| {
            step(&mut game, key);
            selected(&game)
        })
        .collect();
    assert_eq!(rows, [2, 3, 2, 0, 5, 0]);
    let fonts = row_fonts(&game, MAIN);
    assert_eq!(fonts, [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]);
}

#[test]
fn each_move_sounds_and_other_keys_are_ignored() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(sound_after(&mut game, Key::Up), MOVE);
    assert_eq!(sound_after(&mut game, Key::A), SILENT);
    assert_eq!(selected(&game), 0);
}

#[test]
fn escape_jumps_to_exit_once() {
    // Escape in the main menu only moves the highlight to the last row; there it does
    // nothing, so a player cannot leave the game by pressing it twice.
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Escape), MOVE);
    assert_eq!(selected(&game), 5);
    assert_eq!(sound_after(&mut game, Key::Escape), SILENT);
    assert_eq!(selected(&game), 5);
    assert!(!game.quit_requested());
}

#[test]
fn the_cursor_turns_one_frame_every_menu_pass() {
    // A pass waits two ticks; the cursor runs through its 50 frames in 100 ticks.
    let mut game = in_menu(assets());
    let at = (MAIN.0 + 15, MAIN.1 + 16);
    let frames: Vec<u8> = (0..6)
        .map(|_| {
            game.tick();
            pixel(&game, at)
        })
        .collect();
    let k = frames[0];
    assert_eq!(frames, [k, k + 1, k + 1, k + 2, k + 2, k + 3]);
    run(&mut game, 100);
    assert_eq!(pixel(&game, at), k + 3);
}

#[test]
fn the_highlights_pulse_while_the_menu_waits() {
    // Entries 16–31 go down from 100 % to 49 % and back up in 34 ticks.
    let mut game = in_menu(assets());
    let levels: Vec<u8> = (0..35)
        .map(|_| {
            game.tick();
            colour(&game, 16)[0]
        })
        .collect();
    assert_eq!(levels[34], levels[0]);
    let lowest = *levels.iter().min().unwrap();
    assert!((30..=31).contains(&lowest), "{levels:?}");
    assert_eq!(*levels.iter().max().unwrap(), 63, "{levels:?}");
}

#[test]
fn a_held_key_moves_the_highlight_again_after_half_a_second() {
    let mut game = in_menu(assets());
    game.input(InputEvent::Key {
        key: Key::Down,
        pressed: true,
    });
    run(&mut game, 30);
    assert_eq!(selected(&game), 2, "one move in the first 420 ms");
    run(&mut game, 14);
    assert_ne!(selected(&game), 2, "SDL's repeat has started");
}

#[test]
fn the_gamepad_moves_and_chooses_like_the_keys() {
    // With the gamepad switched on in dr.cfg. `eventDetected` treats a push as fresh when it
    // was last called within 400 ms without the stick: a pass of the menu first.
    let mut config = common::config();
    config.set_use_joystick(1);
    let mut game = in_menu_with(assets(), config);
    run(&mut game, 2);
    game.input(InputEvent::PadAxis {
        axis: PadAxis::StickY,
        value: 20_000,
    });
    run(&mut game, 20);
    assert_eq!(
        selected(&game),
        2,
        "a push moves once, then holds off for 700 ms"
    );
    let stick = |game: &mut Game, value: i16| {
        game.input(InputEvent::PadAxis {
            axis: PadAxis::StickY,
            value,
        });
        run(game, 4);
    };
    stick(&mut game, 0);
    stick(&mut game, -20_000);
    stick(&mut game, 0);
    assert_eq!(selected(&game), 0);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: false,
    });
    run(&mut game, 4);
    assert_eq!(
        row_fonts(&game, MAIN)[0],
        BIG_D,
        "the main menu has lost focus"
    );
}

#[test]
fn start_opens_its_submenu_and_escape_closes_it() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(
        row_fonts(&game, START),
        [BIG_A, BIG_D, BIG_D, BIG_B, BIG_D, BIG_B],
        "only new game, load game and back are active at the first start"
    );
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(row_fonts(&game, START)[3], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
}

#[test]
fn the_submenus_last_row_returns_and_starts_it_over_at_the_top() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(row_fonts(&game, START)[5], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Space), CHOOSE);
    assert_eq!(selected(&game), 0, "back in the main menu");
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, START)[0], BIG_A);
}

#[test]
fn the_hall_of_fame_does_nothing_when_chosen_until_m2c() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_A, BIG_B, BIG_B]
    );
}

#[test]
fn the_exit_question_starts_on_no_and_escape_answers_it() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Escape);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(sound_after(&mut game, Key::Left), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_A, BIG_B));
    assert_eq!(sound_after(&mut game, Key::Y), SILENT, "already on yes");
    assert_eq!(sound_after(&mut game, Key::N), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(sound_after(&mut game, Key::Escape), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_B, BIG_B, BIG_A]
    );
    run(&mut game, 1_000);
    assert!(!game.quit_requested());
}

/// Answers the exit question with yes; returns with the key not yet read.
fn answer_yes(game: &mut Game) {
    step(game, Key::Escape);
    step(game, Key::Enter);
    step(game, Key::Left);
    press(game, Key::Enter);
}

#[test]
fn yes_shows_the_end_screen_until_a_key_and_then_the_game_quits() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    // A pass reads the key; the menu takes 26 ticks to black, the end screen 25 to 96 %.
    run(&mut game, 2 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), END);
    assert_eq!(colour(&game, END), [0, 0, 60]);
    run(&mut game, 300);
    assert!(!game.quit_requested());
    press(&mut game, Key::Space);
    // The hold reads the key at once; the fade-out takes 26 ticks.
    run(&mut game, 26);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
    assert_eq!(colour(&game, END), [0, 0, 0]);
}

#[test]
fn the_end_screen_stops_waiting_after_560_ticks() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
}

#[test]
fn the_music_fades_out_with_the_end_screen() {
    let mut with_music = assets();
    with_music.menu_music = music(true);
    let mut game = in_menu(with_music);
    answer_yes(&mut game);
    run(&mut game, 100);
    let loudest = |game: &mut Game| {
        let mut audio = Vec::new();
        game.take_audio(&mut audio);
        audio.iter().map(|&s| s.unsigned_abs()).max().unwrap()
    };
    let held = loudest(&mut game);
    assert!(held > 0);
    press(&mut game, Key::Space);
    run(&mut game, 14);
    let halfway = loudest(&mut game);
    assert!(halfway < held, "{halfway} vs {held}");
    run(&mut game, 13);
    assert!(game.quit_requested());
    // The music's next tick takes the last step to silence.
    run(&mut game, 2);
    loudest(&mut game);
    run(&mut game, 1);
    assert_eq!(loudest(&mut game), 0, "silent once the game has ended");
}

#[test]
fn the_credits_show_two_screens_each_until_a_key_and_return_to_the_menu() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    assert_eq!(selected(&game), 4);
    press(&mut game, Key::Enter);
    // A pass reads the key; the menu fades out in 51 ticks, the first screen in in 25.
    run(&mut game, 2 + 51 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0]);
    assert_eq!(colour(&game, CREDITS[0]), [60, 0, 0]);
    run(&mut game, 500);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "it waits for a key");
    press(&mut game, Key::Space);
    run(&mut game, 1 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
    press(&mut game, Key::Space);
    // Back to the menu as it was, fading in over 50 ticks.
    run(&mut game, 1 + 26 + 50);
    assert_eq!(selected(&game), 4);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    step(&mut game, Key::Down);
    assert_eq!(selected(&game), 5, "the menu works again");
}

#[test]
fn a_key_during_a_credits_fade_in_moves_on_as_soon_as_it_is_done() {
    // The original reads the key before the screen's first wait, so an impatient player does
    // not see it held at all.
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 51 + 10);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "fading in");
    press(&mut game, Key::Space);
    // The rest of the fade-in, the fade to black and the second screen's fade-in.
    run(&mut game, 15 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
}

/// A game in Configure, the file written at start-up taken.
fn in_configure(config: DrCfg) -> Game {
    let mut game = in_menu_with(assets(), config);
    game.take_config();
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    game
}

/// The `dr.cfg` the game hands out now, if it writes one.
fn written(game: &mut Game) -> Option<DrCfg> {
    game.take_config()
        .map(|bytes| DrCfg::parse(&bytes).unwrap())
}

/// Where the volume popup's knob shows level `level`.
fn knob_at(level: usize) -> (usize, usize) {
    (329 + level + 5, 260)
}

#[test]
fn dr_cfg_is_written_at_start_up_counting_the_start() {
    // mainMenu counts every start and writes the file before the intro.
    let mut config = common::config();
    config.set_times_played(4);
    let mut game = Game::new(assets(), config);
    game.tick();
    assert_eq!(written(&mut game).map(|cfg| cfg.times_played()), Some(5));
    run(&mut game, 50);
    assert!(game.take_config().is_none(), "only once");
}

#[test]
fn configure_opens_over_the_dimmed_main_menu_and_escape_returns_writing_dr_cfg() {
    let mut game = in_configure(common::config());
    assert_eq!(
        row_fonts(&game, CONFIGURE),
        [BIG_A, BIG_B, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert!(game.take_config().is_none());
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(selected(&game), 2, "back on the configure row");
    assert!(
        written(&mut game).is_some(),
        "leaving Configure writes dr.cfg"
    );
}

#[test]
fn the_music_volume_moves_by_two_levels_and_is_kept_on_enter() {
    // The level is the volume / 512: 64 for the default 50 %; three steps left make 58.
    let mut game = in_configure(common::config());
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, (320, 255)), SLIDER);
    assert_eq!(pixel(&game, knob_at(64)), KNOB);
    for _ in 0..3 {
        press(&mut game, Key::Left);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(58)), KNOB);
    assert_eq!(sound_after(&mut game, Key::Enter), BACK);
    assert_eq!(row_fonts(&game, CONFIGURE)[0], BIG_A, "back in Configure");
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.music_volume(), 58 * 512);
    assert_eq!(config.effects_volume(), 0xC000, "the effects untouched");
}

#[test]
fn a_volume_stops_at_its_ends() {
    let mut game = in_configure(common::config());
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    for _ in 0..80 {
        press(&mut game, Key::Right);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(128)), KNOB);
    for _ in 0..80 {
        press(&mut game, Key::Left);
        run(&mut game, 1);
    }
    run(&mut game, 1);
    assert_eq!(pixel(&game, knob_at(0)), KNOB);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().effects_volume(), 0);
}

#[test]
fn define_keyboard_takes_the_next_key_for_the_chosen_control() {
    let mut game = in_configure(common::config());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    run(&mut game, 10);
    step(&mut game, Key::Q);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.key(1), 0x10, "brake on Q");
    assert_eq!(config.key(0), 0, "accelerate untouched");
}

#[test]
fn define_gamepad_waits_for_the_pad_to_settle_and_enter_means_none() {
    // While it waits, the original's key reads leave the gamepad alone: a button is taken as
    // an input, not as Enter or Escape.
    let mut config = common::config();
    config.set_use_joystick(1);
    config.set_pad(0, 5);
    config.set_pad(1, 4);
    let mut game = in_configure(config);
    game.input(InputEvent::PadConnected { connected: true });
    for _ in 0..3 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    let button = |game: &mut Game, pressed: bool| {
        game.input(InputEvent::PadButton {
            button: PadButton::Y,
            pressed,
        });
    };
    button(&mut game, true);
    run(&mut game, 30);
    button(&mut game, false);
    run(&mut game, 30);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    run(&mut game, 4);
    step(&mut game, Key::Escape);
    step(&mut game, Key::Escape);
    let config = written(&mut game).unwrap();
    assert_eq!(config.pad(0), 8, "button 4");
    assert_eq!(config.pad(1), 0, "none");
}

#[test]
fn the_gamepad_switch_without_a_gamepad_shows_the_popup_until_a_key() {
    let mut game = in_configure(common::config());
    for _ in 0..4 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, (141, 218)), BIG_A, "not detected");
    run(&mut game, 100);
    assert_eq!(pixel(&game, (141, 218)), BIG_A, "until a key");
    step(&mut game, Key::Space);
    assert_eq!(row_fonts(&game, CONFIGURE)[4], BIG_A, "back in Configure");
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 0);
}

#[test]
fn the_gamepad_switch_turns_a_connected_gamepad_on_and_off() {
    let mut game = in_configure(common::config());
    game.input(InputEvent::PadConnected { connected: true });
    for _ in 0..4 {
        step(&mut game, Key::Down);
    }
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 1);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().use_joystick(), 0);
}

#[test]
fn previous_menu_returns_and_starts_configure_over_at_its_first_row() {
    // Escape keeps Configure's row; its last row also sets it back.
    let mut game = in_configure(common::config());
    for _ in 0..5 {
        step(&mut game, Key::Down);
    }
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(selected(&game), 2);
    assert!(written(&mut game).is_some());
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, CONFIGURE)[0], BIG_A);
}

#[test]
fn dr_cfg_is_written_after_the_end_screen() {
    let mut game = in_menu(assets());
    game.take_config();
    step(&mut game, Key::Escape);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Left);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(game.take_config().is_none());
    game.tick();
    assert!(game.quit_requested());
    assert!(game.take_config().is_some());
}

#[test]
fn a_corrupt_volume_in_dr_cfg_opens_the_popup_at_full_instead_of_crashing() {
    // A damaged or hand-edited file can hold any number; the slider shows the most it can.
    let mut config = common::config();
    config.set_music_volume(0xFB1B_6C00);
    let mut game = in_configure(config);
    step(&mut game, Key::Enter);
    assert_eq!(pixel(&game, knob_at(128)), KNOB);
}

/// The wipe takes 43 ticks.
const WIPE: u32 = 43;

/// A game showing the best ten, its wipe over.
fn in_hall_of_fame(assets: Assets) -> Game {
    let mut game = in_menu(assets);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + WIPE);
    game
}

/// The last left sample of the next tick.
fn level(game: &mut Game) -> i16 {
    game.take_audio(&mut Vec::new());
    game.tick();
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    audio[audio.len() - 2]
}

#[test]
fn the_best_ten_wipe_in_from_the_left() {
    // The band of masks moves 15 pixels a tick from the left edge.
    let mut game = in_menu(assets());
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 20);
    assert_eq!(pixel(&game, (5, 90)), FAME_TITLE, "covered on the left");
    assert_ne!(pixel(&game, (600, 90)), FAME_TITLE, "not yet on the right");
    run(&mut game, WIPE - 20);
    assert_eq!(pixel(&game, (600, 90)), FAME_TITLE);
    assert_eq!(pixel(&game, (38, 146)), MEDIUM, "rank 1");
    assert_eq!(pixel(&game, (30, 344)), MEDIUM, "rank 10, further left");
}

#[test]
fn the_hall_of_fame_plays_its_own_music_and_the_menu_music_comes_back() {
    // The music falls with the wipe and comes back at full mask; back in the menu it starts
    // again at the order it had (45, the tone).
    let mut with_music = assets();
    with_music.menu_music = music(true);
    // Every order plays the tone, so whichever order the music is at when it starts again
    // sounds.
    with_music.menu_music.orders = vec![0; 94];
    let mut game = in_menu(with_music);
    let before = level(&mut game);
    assert!(before > 0);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 30);
    let falling = level(&mut game);
    assert!(falling < before / 2, "{falling} vs {before}");
    run(&mut game, 30);
    assert!(level(&mut game) > falling, "the mask back at 0x10000");
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    press(&mut game, Key::Escape);
    run(&mut game, 2 + WIPE + 10);
    let after = level(&mut game);
    assert!(
        i32::from(after) * 10 >= i32::from(before) * 9,
        "the menu music again at full mask: {after} vs {before}"
    );
}

#[test]
fn the_records_step_through_the_circuits_in_the_originals_order() {
    let mut game = in_hall_of_fame(assets());
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    assert_eq!(pixel(&game, (5, 95)), RECORDS_TITLE);
    let snapshot = |game: &Game| pixel(game, (45, 220));
    assert_eq!(snapshot(&game), SNAPSHOT, "circuit 0 first");
    press(&mut game, Key::Right);
    run(&mut game, 2);
    assert_eq!(snapshot(&game), SNAPSHOT + 7, "then circuit 7");
    assert_eq!(pixel(&game, (170, 230)), ARROW + 3, "the right arrow lit");
    run(&mut game, 5);
    assert_eq!(pixel(&game, (170, 230)), ARROW + 3, "for 8 waits");
    run(&mut game, 4);
    assert_eq!(
        pixel(&game, (170, 230)),
        ARROW + 1,
        "and dark after 8 waits"
    );
    for _ in 0..2 {
        press(&mut game, Key::Left);
        run(&mut game, 12);
    }
    assert_eq!(
        snapshot(&game),
        SNAPSHOT + 15,
        "Left wraps to the last, circuit 15"
    );
}

#[test]
fn leaving_the_records_wipes_the_main_menu_back_with_its_row_kept() {
    let mut game = in_hall_of_fame(assets());
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    assert_eq!(
        sound_after(&mut game, Key::Up),
        SILENT,
        "Up does nothing here"
    );
    press(&mut game, Key::Escape);
    run(&mut game, 2 + WIPE);
    assert_eq!(selected(&game), 3);
    step(&mut game, Key::Down);
    assert_eq!(selected(&game), 4, "the menu works again");
}

#[test]
fn the_best_tens_names_are_written_upper_case_once_shown() {
    // seeHallOfFame upper-cases them in the configuration itself.
    let mut config = common::config();
    let mut bytes = config.to_bytes();
    bytes[8 + 0xA6E..8 + 0xA6E + 3].copy_from_slice(b"ann");
    config = DrCfg::parse(&bytes).unwrap();
    let mut game = in_menu_with(assets(), config);
    game.take_config();
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + WIPE);
    press(&mut game, Key::Space);
    run(&mut game, 2 + WIPE);
    press(&mut game, Key::Escape);
    run(&mut game, 2 + WIPE);
    step(&mut game, Key::Up);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Escape);
    assert_eq!(written(&mut game).unwrap().hall_of_fame(0).0, b"ANN");
}

/// From the main menu through the licence (nickname "A", weapons, the difficulty), the
/// sign-up's welcome, Escape at the sign-up and its "no race" popup, back to the Start Racing
/// menu with a game on.
fn through_a_new_game(game: &mut Game) {
    step(game, Key::Enter);
    step(game, Key::Enter);
    step(game, Key::A);
    step(game, Key::Enter);
    step(game, Key::Enter);
    press(game, Key::Enter);
    // The difficulty popup reads the key before its two waits.
    run(game, 8);
    // The wipe, then the welcome popup, deaf for its first eleven passes.
    run(game, 80);
    step(game, Key::Enter);
    run(game, 4);
    step(game, Key::Escape);
    step(game, Key::Space);
    // The fade to black, then the menus wiped in.
    run(game, 120);
}

#[test]
fn ending_a_game_puts_the_start_menus_highlight_back_on_its_first_row() {
    // The original sets the Start Racing menu's row to 0 after "yes" (0x439F7D). Left on
    // "End Current Game", now inactive, Enter would ask again with no game on and set the
    // drivers up once more, 76 draws of rand() the original never makes.
    let mut game = in_menu(assets());
    through_a_new_game(&mut game);
    assert_eq!(
        row_fonts(&game, START),
        [BIG_A, BIG_B, BIG_B, BIG_B, BIG_B, BIG_B],
        "with a game on, ending it, statistics and saving are active"
    );
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Enter);
    run(&mut game, 4);
    assert_eq!(
        row_fonts(&game, START),
        [BIG_A, BIG_D, BIG_D, BIG_B, BIG_D, BIG_B],
        "the game ended: the first row highlighted, the game's rows inactive again"
    );
}

#[test]
fn a_damaged_difficulty_in_dr_cfg_does_not_crash_the_licence() {
    // dr.cfg is read as it is; a difficulty past the three rows must not stop the game when
    // the player confirms the popup.
    let mut config = common::config();
    config.set_difficulty(7);
    let mut game = in_menu_with(assets(), config);
    through_a_new_game(&mut game);
}
