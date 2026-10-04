//! The startup sequence's timeline on synthetic assets (spec M1a §5.2). Each test pins down
//! something a player of the original would notice: a logo that holds too long, a fade that
//! ends at the wrong brightness, a key that does not skip.

use deadrally_core::{AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, Game, InputEvent, Key, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use std::path::PathBuf;

use deadrally_gamedata::haf::{Animation, FRAME_PIXELS, HafFrame};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// Intro delays: frame 0 at tick 4, frame 1 at tick 6, the last frame at tick 9.
const DELAYS: [u8; 3] = [4, 2, 3];
const INTRO_END: u32 = 9;
/// Pixel values that tell the pictures apart; each picture's palette makes its own colour
/// full red, green or blue.
const APOGEE: u8 = 1;
const REMEDY: u8 = 2;
const TITLE: u8 = 3;
/// A logo takes 25 fade-in ticks, 180 hold ticks and 26 fade-out ticks.
const FADE_IN: u32 = 25;
const HOLD: u32 = 180;
const LOGO: u32 = FADE_IN + HOLD + 26;

fn palette(entries: &[(usize, [u8; 3])]) -> Palette {
    let mut palette = Palette::BLACK;
    for &(index, rgb) in entries {
        palette.0[index] = rgb;
    }
    palette
}

fn picture(pixel: u8, rgb: [u8; 3]) -> Picture {
    Picture {
        image: Image::new(4, 3, vec![pixel; 12]),
        palette: palette(&[(usize::from(pixel), rgb)]),
    }
}

/// Frame `k` is all pixel `16 + k`, coloured grey level `10 + k`.
fn intro_frame(k: u8) -> HafFrame {
    HafFrame {
        // Entries below 16 belong to the letterbox; the intro must not take them from frames.
        palette: palette(&[(0, [63, 63, 63]), (usize::from(16 + k), [10 + k; 3])]),
        pixels: vec![16 + k; FRAME_PIXELS],
    }
}

fn assets() -> Assets {
    let mut letterbox = vec![0u8; 320 * 200];
    letterbox[..320].fill(5);
    Assets {
        intro: Animation::from_frames(DELAYS.to_vec(), (0..3).map(intro_frame).collect()),
        letterbox: Picture {
            image: Image::new(320, 200, letterbox),
            // Entries from 16 on must stay black until the first frame sets them.
            palette: palette(&[(5, [20, 30, 40]), (16, [63, 63, 63])]),
        },
        apogee: picture(APOGEE, [63, 0, 0]),
        remedy: picture(REMEDY, [0, 63, 0]),
        title: picture(TITLE, [0, 0, 63]),
        intro_music: music(false),
        intro_effects: effects(),
        menu_music: music(false),
    }
}

/// A module that is silent, or plays one endless tone from its first row.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let mut pattern = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    pattern.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone { vec![0] } else { Vec::new() },
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
        patterns: vec![pattern],
    }
}

/// Effect 1 is an endless constant tone.
fn effects() -> Bank {
    Bank {
        linear_frequencies: true,
        instruments: vec![Some(Instrument {
            name: "Hum".into(),
            data: vec![1000; 1000],
            looping: Looping::Forward {
                start: 0,
                length: 1000,
            },
            volume: 64,
            finetune: 0,
            relative_note: 0,
            panning: 128,
            fadeout: 0,
        })],
    }
}

fn press(game: &mut Game) {
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: true,
    });
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: false,
    });
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// Which picture is on screen and how bright its colour is (0..=63).
fn shown(game: &Game) -> (u8, u8) {
    let frame = game.frame();
    let pixel = frame.pixels[0];
    let brightness = frame.palette[usize::from(pixel)].into_iter().max().unwrap();
    (pixel, brightness)
}

/// The intro row shown at row 40 (the first animation row) and its colour.
fn intro_row(game: &Game) -> (u8, [u8; 3]) {
    let frame = game.frame();
    assert_eq!((frame.width, frame.height), (320, 200));
    let pixel = frame.pixels[40 * 320];
    (pixel, frame.palette[usize::from(pixel)])
}

#[test]
fn the_intro_starts_with_the_letterbox_and_black_animation_colours() {
    let game = Game::new(assets());
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (320, 200, (4, 3))
    );
    assert_eq!(frame.pixels[0], 5);
    assert_eq!(frame.palette[5], [20, 30, 40]);
    assert_eq!(frame.palette[16], [0, 0, 0]);
}

#[test]
fn each_intro_frame_appears_its_delay_after_the_previous_one() {
    // The intro is cut to its music (M1b); a frame early or late drifts out of sync.
    let mut game = Game::new(assets());
    run(&mut game, 3);
    assert_eq!(intro_row(&game), (0, [0, 0, 0]), "nothing before tick 4");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (16, [10, 10, 10]), "frame 0 at tick 4");
    assert_eq!(
        game.frame().palette[0],
        [0, 0, 0],
        "entries below 16 stay the letterbox's"
    );
    run(&mut game, 1);
    assert_eq!(intro_row(&game).0, 16, "frame 0 still at tick 5");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (17, [11, 11, 11]), "frame 1 at tick 6");
}

#[test]
fn the_last_intro_frame_is_never_shown() {
    // openAnimation blacks the palette as soon as the last frame is drawn, before it is shown.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END - 1);
    assert_eq!(intro_row(&game).0, 17);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn a_key_ends_the_intro_when_the_next_frame_is_due() {
    // The original checks for a key once per frame, so the intro runs on until the next frame
    // would have been shown.
    let mut game = Game::new(assets());
    run(&mut game, 1);
    press(&mut game);
    run(&mut game, 2);
    assert_eq!(intro_row(&game).0, 0, "still waiting for frame 0");
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0), "frame 0 is skipped too");
}

#[test]
fn a_corrupt_intro_frame_ends_the_intro_instead_of_crashing() {
    // Only data of an unknown version can hold one, and the player was warned at start-up;
    // the game should still reach its menus. The broken frame comes first: the last frame is
    // never decoded, so it could not show the problem.
    let mut record = vec![0u8; 768];
    record.extend([8, 2, 0xFF, 0xFF, 0, 0x3B]);
    let mut haf = vec![2, 0, 0, 0, 1, 1];
    for _ in 0..2 {
        haf.extend(u16::try_from(record.len()).unwrap().to_le_bytes());
        haf.extend(&record);
    }
    let mut broken = assets();
    broken.intro = Animation::from_bytes(PathBuf::from("BROKEN.HAF"), haf).unwrap();
    assert!(broken.intro.frame(0).is_err());
    let mut game = Game::new(broken);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn an_empty_intro_goes_straight_to_the_logos() {
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    let mut game = Game::new(empty);
    assert_eq!(shown(&game), (APOGEE, 0));
    run(&mut game, FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
}

#[test]
fn a_pad_button_skips_like_a_key() {
    let mut game = Game::new(assets());
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    assert_eq!(shown(&game).0, APOGEE);
}

/// Red brightness of the Apogee logo for each tick after the intro.
fn apogee_brightness(game: &mut Game, ticks: u32) -> Vec<u8> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let (pixel, brightness) = shown(game);
            assert_eq!(pixel, APOGEE);
            brightness
        })
        .collect()
}

#[test]
fn a_logo_fades_in_holds_and_fades_out_like_the_original() {
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END);
    let brightness = apogee_brightness(&mut game, LOGO - 1);
    // The fade-in climbs one 4 % step per tick from black and stops at 96 %: 63 shows as 60.
    assert_eq!(&brightness[..4], [0, 3, 5, 8]);
    assert_eq!(brightness[24], 60);
    // The hold lasts 180 ticks when nobody presses a key.
    assert!(brightness[25..205].iter().all(|&level| level == 60));
    // The fade-out starts at 100 %, a visible flash from 60 to 63, and steps down to black.
    assert_eq!(&brightness[205..208], [63, 60, 58]);
    assert_eq!(brightness[229], 3);
    // Its last, black step already has Remedy drawn under it: the same black screen.
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 0));
}

#[test]
fn a_key_during_the_fade_in_ends_the_hold_after_one_tick() {
    // The original remembers the press until the hold asks, so impatient players see the logo
    // at full fade for a single tick.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + 5);
    press(&mut game);
    let brightness = apogee_brightness(&mut game, FADE_IN - 5 + 1);
    assert_eq!(brightness.last(), Some(&60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63), "the fade-out starts");
}

#[test]
fn a_key_during_a_fade_out_ends_the_next_logos_hold_after_one_tick() {
    // The remembered press survives the change of screen, as in the original: a player who
    // presses while Apogee fades out sees the Remedy logo for a single hold tick.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + FADE_IN + HOLD + 10);
    assert_eq!(shown(&game).0, APOGEE, "Apogee is fading out");
    press(&mut game);
    run(&mut game, LOGO - FADE_IN - HOLD - 10 + FADE_IN);
    assert_eq!(shown(&game), (REMEDY, 60), "Remedy's fade-in is done");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 63), "the fade-out starts");
}

#[test]
fn a_key_with_the_last_intro_frame_carries_into_the_apogee_hold() {
    // openAnimation stops after its last frame without checking for a key, so the press waits
    // for the Apogee hold.
    let mut game = Game::new(assets());
    run(&mut game, 7);
    press(&mut game);
    run(&mut game, INTRO_END - 7 + FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn a_key_during_the_hold_starts_the_fade_out_on_the_next_tick() {
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + FADE_IN + 50);
    press(&mut game);
    game.tick();
    assert_eq!(
        shown(&game),
        (APOGEE, 60),
        "the hold tick that reads the key"
    );
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn the_title_fades_in_after_both_logos_and_stays_at_92_percent() {
    // Measured on the original: it sets the last fade step and then loads the main menu
    // without showing another frame, so the title never gets brighter than 92 % (63 as 58).
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + 2 * LOGO);
    assert_eq!(shown(&game), (TITLE, 0));
    run(&mut game, FADE_IN - 1);
    assert_eq!(shown(&game), (TITLE, 58));
    // M2's main menu continues from here; until then nothing else happens, keys included.
    press(&mut game);
    run(&mut game, 1_000);
    assert_eq!(shown(&game), (TITLE, 58));
}

#[test]
fn the_startup_sequence_is_silent_but_keeps_the_audio_stream_full() {
    // The frontend paces itself on the audio queue; missing samples would stall or drift it.
    let mut game = Game::new(assets());
    run(&mut game, 30);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), 30 * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
    assert!(audio.iter().all(|&sample| sample == 0));
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), 30 * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
}
