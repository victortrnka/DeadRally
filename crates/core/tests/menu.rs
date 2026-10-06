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
        race: common::race_archives(),
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
/// sign-up's welcome, Escape at the sign-up and its "no race" popup, the shop after the
/// stand-in race, and Escape back to the Start Racing menu with a game on.
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
    // The fade to black, then the shop wiped in.
    run(game, 120);
    step(game, Key::Escape);
    run(game, 60);
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

/// A saved game: the player (driver 19) in a Vagabond, everyone else zero, named "s".
fn saved_game() -> Vec<u8> {
    let mut drivers = vec![0u8; 0x870];
    // Driver 19's name and rank 20.
    drivers[19 * 108] = b'p';
    drivers[19 * 108 + 72] = 20;
    let mut name = [0; 15];
    name[0] = b's';
    deadrally_gamedata::save_game::SaveGame {
        driver_id: 19,
        use_weapons: 1,
        difficulty: 1,
        name,
        drivers,
    }
    .encode(3)
}

/// From the main menu to the Start Racing menu's "load game" row and its slots.
fn to_the_slots(game: &mut Game) {
    step(game, Key::Enter);
    step(game, Key::Down);
    step(game, Key::Enter);
}

#[test]
fn a_loaded_game_opens_the_shop_and_escape_comes_back_to_the_start_menu() {
    // Loading ends in the shop, as the original's loadGame and postLoadedOrLicense do; Escape
    // there returns to the menu with the game's rows active.
    let mut game = Game::new(assets(), common::config());
    game.set_saved_games(vec![Some(saved_game())]);
    run(&mut game, MENU_SHOWN);
    to_the_slots(&mut game);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Space);
    // The wipe, then a pass of the shop.
    run(&mut game, 60);
    assert_eq!(
        pixel(&game, (300, 95)),
        common::SHOP,
        "the shop's title is shown"
    );
    step(&mut game, Key::Escape);
    run(&mut game, 60);
    assert_eq!(
        row_fonts(&game, START),
        [BIG_B, BIG_B, BIG_B, BIG_A, BIG_B, BIG_B],
        "back on the load row, with the game's rows active"
    );
}

#[test]
fn an_empty_slot_loads_nothing() {
    // Choosing an empty slot only sounds; the slots stay.
    let mut game = in_menu(assets());
    to_the_slots(&mut game);
    step(&mut game, Key::Enter);
    run(&mut game, 60);
    assert_ne!(pixel(&game, (300, 95)), common::SHOP, "no shop");
}

#[test]
fn saving_writes_the_game_under_the_typed_name_into_the_chosen_slot() {
    // The file must be the original's format: driver 19, the drivers' records, the name the
    // player typed, lower-cased as the entry types it.
    let mut game = Game::new(assets(), common::config());
    game.set_saved_games(vec![Some(saved_game())]);
    run(&mut game, MENU_SHOWN);
    to_the_slots(&mut game);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Space);
    run(&mut game, 60);
    step(&mut game, Key::Escape);
    run(&mut game, 60);
    // From the load row down to "save game", its slots, slot 2.
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Q);
    step(&mut game, Key::Enter);
    let (slot, file) = game.take_saved_game().expect("a game was saved");
    assert_eq!(slot, 2);
    let saved = deadrally_gamedata::save_game::SaveGame::decode(&file);
    assert_eq!(saved.driver_id, 19);
    assert_eq!(&saved.name[..2], b"q\0");
    assert_eq!(
        saved.drivers,
        deadrally_gamedata::save_game::SaveGame::decode(&saved_game()).drivers
    );
}

#[test]
fn a_damaged_saved_game_is_refused_like_an_empty_slot() {
    // A save from another program or cut short decrypts into numbers no game holds (a car
    // past the sixth, a colour past COPPER.PAL, a name wider than the side panel); loading it
    // must not stop the game, so the slot counts as empty.
    for (offset, value) in [
        (28, 6),
        (44, 300),
        (48, -20_000_000),
        (20, -1),
        (12, i32::MIN),
        // The licence and the paint only make even colours; the paint steps by 2 to 0.
        (44, 1),
    ] {
        let mut file = deadrally_gamedata::save_game::SaveGame::decode(&saved_game());
        let at = 19 * 108 + offset;
        file.drivers[at..at + 4].copy_from_slice(&value.to_le_bytes());
        let mut game = Game::new(assets(), common::config());
        game.set_saved_games(vec![Some(file.encode(3))]);
        run(&mut game, MENU_SHOWN);
        to_the_slots(&mut game);
        step(&mut game, Key::Enter);
        step(&mut game, Key::Space);
        run(&mut game, 60);
        assert_ne!(
            pixel(&game, (300, 95)),
            common::SHOP,
            "offset {offset}: no shop"
        );
    }
}

/// [`saved_game`] with the player holding `money` dollars.
fn saved_game_with_money(money: i32) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(&saved_game());
    let at = 19 * 108 + 48;
    game.drivers[at..at + 4].copy_from_slice(&money.to_le_bytes());
    let price = 19 * 108 + 60;
    game.drivers[price..price + 4].copy_from_slice(&500i32.to_le_bytes());
    game.encode(3)
}

/// The player's record in the game saved into slot 1 after `keys` in the shop of
/// [`saved_game_with_money`].
fn player_after_shopping(money: i32, keys: &[Key]) -> Vec<u8> {
    let mut game = in_shop(saved_game_with_money(money));
    for &key in keys {
        step(&mut game, key);
    }
    saved_player(game)
}

/// `file` loaded from slot 0, the shop shown.
fn in_shop(file: Vec<u8>) -> Game {
    let mut game = Game::new(assets(), common::config());
    game.set_saved_games(vec![Some(file)]);
    run(&mut game, MENU_SHOWN);
    to_the_slots(&mut game);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Space);
    run(&mut game, 60);
    game
}

/// From the shop, the game saved into slot 1; the player's record in it.
fn saved_player(mut game: Game) -> Vec<u8> {
    step(&mut game, Key::Escape);
    run(&mut game, 60);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Enter);
    step(&mut game, Key::Q);
    step(&mut game, Key::Enter);
    let (_, file) = game.take_saved_game().expect("a game was saved");
    deadrally_gamedata::save_game::SaveGame::decode(&file).drivers[19 * 108..20 * 108].to_vec()
}

fn field(record: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(record[offset..offset + 4].try_into().unwrap())
}

#[test]
fn an_engine_upgrade_is_paid_for_and_adds_to_the_cars_worth() {
    // The money goes, the engine level rises, and the car's worth grows by the price, which
    // the dealer's refund is a quarter of (enterShop 0x43805F).
    let left = [Key::Left; 4];
    let record = player_after_shopping(10_000, &[&left[..], &[Key::Enter]].concat());
    assert_eq!(field(&record, 16), 1, "engine level 1");
    assert_eq!(field(&record, 48), 10_000 - 100, "money");
    assert_eq!(field(&record, 60), 500 + 100, "the car's worth");
}

#[test]
fn without_the_money_nothing_is_bought() {
    // Short of the price, the shop says so and keeps everything as it was.
    let left = [Key::Left; 4];
    let record = player_after_shopping(50, &[&left[..], &[Key::Enter]].concat());
    assert_eq!((field(&record, 16), field(&record, 48)), (0, 50));
}

/// The shop's fades into the Underground Market and back each take 101 waits.
const MARKET_FADES: u32 = 110;

/// [`saved_game_with_money`] with the player's record changed at `fields` (offset, value).
fn saved_game_with(money: i32, fields: &[(usize, i32)]) -> Vec<u8> {
    let mut game = deadrally_gamedata::save_game::SaveGame::decode(&saved_game_with_money(money));
    for &(offset, value) in fields {
        let at = 19 * 108 + offset;
        game.drivers[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    game.encode(3)
}

/// The player's record saved after going on from the shop into the Underground Market,
/// pressing `keys` there and leaving it with Escape.
fn player_after_market(file: Vec<u8>, keys: &[Key]) -> Vec<u8> {
    let mut game = in_shop(file);
    step(&mut game, Key::Enter);
    run(&mut game, MARKET_FADES);
    for &key in keys {
        step(&mut game, key);
    }
    step(&mut game, Key::Escape);
    run(&mut game, MARKET_FADES);
    saved_player(game)
}

const MONEY: usize = 48;
const LOAN: usize = 52;
const LOAN_RACES: usize = 56;
const MINES: usize = 92;

#[test]
fn mines_are_paid_for_fitted_and_then_sold_out() {
    // underGroundMenuEnter (0x43636F): the price goes, the car carries 8 mines, and the
    // market has no more of them, so a second Enter buys nothing.
    let to_mines = [Key::Left; 4];
    let record = player_after_market(
        saved_game_with_money(1_000),
        &[&to_mines[..], &[Key::Enter, Key::Enter]].concat(),
    );
    assert_eq!(field(&record, MINES), 8, "mines fitted");
    assert_eq!(field(&record, MONEY), 1_000 - 150, "paid once");
}

#[test]
fn a_car_full_of_mines_finds_them_sold_out() {
    // A loaded game's market sells only what the car is not full of (0x42F6A1).
    let to_mines = [Key::Left; 4];
    let record = player_after_market(
        saved_game_with(1_000, &[(MINES, 8)]),
        &[&to_mines[..], &[Key::Enter]].concat(),
    );
    assert_eq!(field(&record, MONEY), 1_000, "nothing bought");
}

#[test]
fn the_loan_shark_lends_by_the_car_and_is_paid_back() {
    // With the cheapest car after the Vagabond he lends $1500 (0x4361A7); paid back in the
    // same race it costs the loan itself, and the loan is gone.
    let to_shark = [Key::Left, Key::Left, Key::Left, Key::Left, Key::Up];
    let no_loan = [(28, 1), (LOAN, -1), (LOAN_RACES, -1)];
    let lent = player_after_market(
        saved_game_with(1_000, &no_loan),
        &[&to_shark[..], &[Key::Enter]].concat(),
    );
    assert_eq!(
        (
            field(&lent, MONEY),
            field(&lent, LOAN),
            field(&lent, LOAN_RACES)
        ),
        (2_500, 4, 1),
        "lent"
    );
    let repaid = player_after_market(
        saved_game_with(1_000, &no_loan),
        &[&to_shark[..], &[Key::Enter, Key::Enter]].concat(),
    );
    assert_eq!(
        (
            field(&repaid, MONEY),
            field(&repaid, LOAN),
            field(&repaid, LOAN_RACES)
        ),
        (1_000, -1, -1),
        "paid back"
    );
}

#[test]
fn the_loan_shark_lends_nothing_for_a_vagabond() {
    // 0x4361BB: the Vagabond's driver is refused.
    let to_shark = [Key::Left, Key::Left, Key::Left, Key::Left, Key::Up];
    let record = player_after_market(
        saved_game_with(1_000, &[(LOAN, -1), (LOAN_RACES, -1)]),
        &[&to_shark[..], &[Key::Enter]].concat(),
    );
    assert_eq!(
        (field(&record, MONEY), field(&record, LOAN_RACES)),
        (1_000, -1)
    );
}

/// A new game (see [`through_a_new_game`]) and its shop wiped in from the Start Racing menu.
fn new_game_in_shop() -> Game {
    let mut game = in_menu(assets());
    through_a_new_game(&mut game);
    step(&mut game, Key::Enter);
    run(&mut game, 60);
    game
}

/// From the shop through the Underground Market (`keys` there; its popup closed on the
/// `first_visit`) to the sign-up, a race chosen; waits until its other places are filled.
fn to_a_race(game: &mut Game, first_visit: bool, keys: &[Key]) {
    step(game, Key::Enter);
    run(game, MARKET_FADES);
    if first_visit {
        // The popup takes no key in its first eleven passes.
        run(game, 30);
        step(game, Key::Enter);
    }
    for &key in keys {
        step(game, key);
    }
    step(game, Key::Enter);
    run(game, 80);
    step(game, Key::Enter);
    run(game, 300);
}

/// The drivers' records of the game saved into slot 1 from the Start Racing menu with its
/// first row highlighted.
fn saved_drivers(game: &mut Game) -> Vec<u8> {
    for _ in 0..4 {
        step(game, Key::Down);
    }
    step(game, Key::Enter);
    step(game, Key::Down);
    step(game, Key::Enter);
    step(game, Key::Q);
    step(game, Key::Enter);
    let (_, file) = game.take_saved_game().expect("a game was saved");
    deadrally_gamedata::save_game::SaveGame::decode(&file).drivers
}

#[test]
fn the_sabotage_damages_one_rival_by_25_to_49_percent() {
    // sabotageScreen (0x42DD10): with the sabotage bought and the player not leading, after
    // the sign-up every other car is repaired and the best-ranked rival in the player's race
    // starts it 25 to 49 % damaged; its popup waits for a key, then the race comes.
    let mut game = new_game_in_shop();
    to_a_race(&mut game, true, &[Key::Left, Key::Enter, Key::Right]);
    step(&mut game, Key::Enter);
    // The race's preview wipes in, stays while the race loads, fades out; then the shop.
    run(&mut game, 330);
    assert_eq!(
        pixel(&game, (300, 95)),
        common::SHOP,
        "the shop after the race"
    );
    step(&mut game, Key::Escape);
    run(&mut game, 60);
    let drivers = saved_drivers(&mut game);
    let damages: Vec<i32> = (0..19).map(|d| field(&drivers[108 * d..], 12)).collect();
    let hit: Vec<i32> = damages.iter().copied().filter(|&d| d != 0).collect();
    assert_eq!(hit.len(), 1, "one rival sabotaged: {damages:?}");
    assert!((25..50).contains(&hit[0]), "{damages:?}");
}

#[test]
fn an_offer_after_a_sign_up_waits_for_its_answer_and_escape_does_not_give_one() {
    // 0x431B30: at the hitman's chance (5 %, 2 % more each sign-up he does not come) the drug
    // dealer or the hitman offers a deal after the sign-up; its question ignores Escape
    // (drawYesNoMenu with 0), and either answer leads to the race.
    let mut game = new_game_in_shop();
    let offered = |game: &Game| {
        let at = pixel(game, (60, 200));
        at == common::DRUG_DEALER || at == common::HITMAN
    };
    to_a_race(&mut game, true, &[]);
    let mut sign_ups = 1;
    while !offered(&game) {
        sign_ups += 1;
        assert!(sign_ups < 40, "no offer in 40 sign-ups");
        // The screen lingers, the stand-in race, the shop again.
        run(&mut game, 300);
        run(&mut game, 60);
        to_a_race(&mut game, false, &[]);
    }
    run(&mut game, 80);
    step(&mut game, Key::Escape);
    run(&mut game, 20);
    assert!(offered(&game), "Escape does not answer");
    step(&mut game, Key::Enter);
    run(&mut game, 330);
    assert_eq!(
        pixel(&game, (300, 95)),
        common::SHOP,
        "the race, then the shop"
    );
}

/// Holds `key` down for `ticks` ticks, then lets it go.
fn hold(game: &mut Game, key: Key, ticks: u32) {
    game.input(InputEvent::Key { key, pressed: true });
    run(game, ticks);
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

#[test]
fn f2_held_in_the_shop_saves_into_the_quicksave_slot_and_f3_loads_it_back() {
    // sub_4221A0: each pass of the shop looks at the keys held; F2 writes DR.SG7 under the
    // quicksave's name and says so, F3 reads it back. A tap shorter than a pass is missed,
    // as in the original, so the keys are held.
    let mut game = in_shop(saved_game_with_money(1_000));
    hold(&mut game, Key::F2, 4);
    let (slot, file) = game.take_saved_game().expect("a quick save");
    assert_eq!(slot, 7);
    let saved = deadrally_gamedata::save_game::SaveGame::decode(&file);
    assert_eq!(&saved.name[..2], b"Q\0", "the quicksave's name");
    step(&mut game, Key::Space);
    // An engine bought after the quick save is gone once the game is loaded back.
    for key in [Key::Left, Key::Left, Key::Left, Key::Left, Key::Enter] {
        step(&mut game, key);
    }
    hold(&mut game, Key::F3, 4);
    step(&mut game, Key::Space);
    let record = saved_player(game);
    assert_eq!((field(&record, 16), field(&record, MONEY)), (0, 1_000));
}

#[test]
fn f3_without_a_quicksave_loads_nothing() {
    // No DR.SG7: the original says the game is not found and keeps the one on.
    let mut game = in_shop(saved_game_with_money(1_000));
    for key in [Key::Left, Key::Left, Key::Left, Key::Left, Key::Enter] {
        step(&mut game, key);
    }
    hold(&mut game, Key::F3, 4);
    // The popup's fill (draw.rs POPUP_FILL) where the engine box was.
    assert_eq!(pixel(&game, (120, 255)), 0xC4, "the confirmation is up");
    step(&mut game, Key::Space);
    let record = saved_player(game);
    assert_eq!(field(&record, 16), 1, "the engine bought stays");
}

#[test]
fn a_quick_save_happens_once_however_long_f2_is_held() {
    // confirmationPopup (0x42DC70) lets go of F2 and F3 when a key ends it: holding F2
    // through "game saved" would otherwise save again, with another rand() for the file's key.
    let mut game = in_shop(saved_game_with_money(1_000));
    game.input(InputEvent::Key {
        key: Key::F2,
        pressed: true,
    });
    run(&mut game, 10);
    assert!(game.take_saved_game().is_some(), "saved");
    step(&mut game, Key::Space);
    run(&mut game, 10);
    game.input(InputEvent::Key {
        key: Key::F2,
        pressed: false,
    });
    assert!(game.take_saved_game().is_none(), "saved once");
}

#[test]
fn a_hand_made_loan_or_car_worth_does_not_stop_the_game() {
    // A save may hold any loan count or car worth; the debt and the car's worth wrap as the
    // original's ints do instead of stopping the game.
    let to_shark = [
        Key::Left,
        Key::Left,
        Key::Left,
        Key::Left,
        Key::Up,
        Key::Enter,
    ];
    player_after_market(
        saved_game_with(1_000, &[(28, 1), (LOAN, 0), (LOAN_RACES, i32::MIN)]),
        &to_shark,
    );
    let left = [Key::Left; 4];
    player_after_shopping_from(
        saved_game_with(10_000, &[(60, i32::MAX - 10)]),
        &[&left[..], &[Key::Enter]].concat(),
    );
}

/// [`player_after_shopping`] from `file`.
fn player_after_shopping_from(file: Vec<u8>, keys: &[Key]) -> Vec<u8> {
    let mut game = in_shop(file);
    for &key in keys {
        step(&mut game, key);
    }
    saved_player(game)
}

#[test]
fn the_race_s_preview_wipes_in_after_the_sign_up() {
    // previewRaceScreen (0x4321B0): before the race the player sees the grid's four drivers
    // and the circuit, its banner across the bottom; they wipe in over the sign-up.
    let mut game = new_game_in_shop();
    to_a_race(&mut game, true, &[]);
    step(&mut game, Key::Space);
    run(&mut game, 20);
    assert_eq!(
        pixel(&game, (5, 450)),
        common::PREVIEW,
        "the banner's left on its way"
    );
    assert_ne!(
        pixel(&game, (600, 450)),
        common::PREVIEW,
        "its right still to come"
    );
    run(&mut game, 30);
    assert_eq!(pixel(&game, (600, 450)), common::PREVIEW, "the banner");
    assert_eq!(pixel(&game, (400, 200)), common::PREVIEW + 2, "the circuit");
}

#[test]
fn the_shop_after_a_race_is_the_menus_own_again() {
    // After a race (here the stand-in for one whose data does not load) the shop opens on the
    // menus' screen with none of the preview left on it, and its sounds are heard again: the
    // race's fade to black had silenced them, and the original brings the menus' music and
    // sounds back as the race ends (0x434617).
    let mut game = new_game_in_shop();
    to_a_race(&mut game, true, &[]);
    step(&mut game, Key::Space);
    run(&mut game, 44 + 143 + 41 + 60);
    assert_ne!(
        pixel(&game, (5, 450)),
        common::PREVIEW,
        "the preview's banner gone"
    );
    assert_ne!(
        pixel(&game, (400, 200)),
        common::PREVIEW + 2,
        "its circuit gone"
    );
    press(&mut game, Key::Left);
    let mut audio = Vec::new();
    for _ in 0..10 {
        game.tick();
        game.take_audio(&mut audio);
    }
    assert!(
        audio.iter().any(|&sample| sample != 0),
        "the shop's step heard"
    );
}

#[test]
fn an_opponent_s_face_past_the_pictures_does_not_stop_the_preview() {
    // A save's check covers the player's record only: an edited one may give the opponents
    // faces past the pictures. The preview leaves such a face out instead of stopping the game.
    let mut file = deadrally_gamedata::save_game::SaveGame::decode(&saved_game_with_money(5_000));
    for driver in 0..19 {
        let at = driver * 108 + 64;
        file.drivers[at..at + 4].copy_from_slice(&1_000i32.to_le_bytes());
    }
    let mut game = in_shop(file.encode(3));
    to_a_race(&mut game, true, &[]);
    step(&mut game, Key::Space);
    run(&mut game, 50);
    assert_eq!(
        pixel(&game, (600, 450)),
        common::PREVIEW,
        "the preview shown"
    );
}
