//! The core's contract with frontends and the parity harness (spec section 5).

use deadrally_core::{
    AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, AUDIO_SAMPLE_RATE, Game, InputEvent, Key, PadAxis,
    PadButton, TICKS_PER_SECOND,
};

fn press(game: &mut Game, key: Key) {
    game.input(InputEvent::Key { key, pressed: true });
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

/// Every frame's pixels and palette, plus all audio.
type Recording = (Vec<(Vec<u8>, Vec<[u8; 3]>)>, Vec<i16>);

/// Runs a scripted session from a fresh game.
fn record(script: &[(u32, InputEvent)], ticks: u32) -> Recording {
    let mut game = Game::new();
    let mut frames = Vec::new();
    let mut audio = Vec::new();
    for tick in 0..ticks {
        for &(at, event) in script {
            if at == tick {
                game.input(event);
            }
        }
        game.tick();
        let frame = game.frame();
        frames.push((frame.pixels.to_vec(), frame.palette.to_vec()));
        game.take_audio(&mut audio);
    }
    (frames, audio)
}

#[test]
fn same_inputs_give_identical_frames_and_audio() {
    // Parity testing compares runs tick by tick; any hidden state (clock, hash order,
    // uninitialised data) would make two identical runs differ.
    let script = [
        (
            3,
            InputEvent::Key {
                key: Key::A,
                pressed: true,
            },
        ),
        (
            5,
            InputEvent::PadAxis {
                axis: PadAxis::StickX,
                value: -20_000,
            },
        ),
        (
            9,
            InputEvent::Key {
                key: Key::Tab,
                pressed: true,
            },
        ),
        (
            10,
            InputEvent::Key {
                key: Key::Tab,
                pressed: false,
            },
        ),
        (
            12,
            InputEvent::PadButton {
                button: PadButton::Y,
                pressed: true,
            },
        ),
        (
            20,
            InputEvent::Key {
                key: Key::A,
                pressed: false,
            },
        ),
    ];
    assert_eq!(record(&script, 300), record(&script, 300));
}

#[test]
fn each_tick_produces_exactly_one_tick_of_stereo_audio() {
    // Frontends and the WAV capture assume 630 frames per tick; 70 ticks must be exactly one
    // second at 44.1 kHz or music drifts against the picture.
    let mut game = Game::new();
    let mut audio = Vec::new();
    game.tick();
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
    for _ in 1..TICKS_PER_SECOND {
        game.tick();
    }
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), AUDIO_SAMPLE_RATE as usize * AUDIO_CHANNELS);
}

#[test]
fn take_audio_returns_each_sample_only_once() {
    // A frontend that queued the same samples twice would play an echo.
    let mut game = Game::new();
    game.tick();
    let mut first = Vec::new();
    game.take_audio(&mut first);
    let mut second = Vec::new();
    game.take_audio(&mut second);
    assert!(second.is_empty());
}

#[test]
fn tab_cycles_the_documented_frame_modes() {
    // The spike checks scaling for each of these sizes and aspects.
    let mut game = Game::new();
    let mut seen = Vec::new();
    for _ in 0..4 {
        let frame = game.frame();
        // Read right after the switch, before any tick: size and pixels must agree.
        assert_eq!(frame.pixels.len(), (frame.width * frame.height) as usize);
        seen.push((frame.width, frame.height, frame.aspect));
        press(&mut game, Key::Tab);
    }
    assert_eq!(
        seen,
        [
            (640, 480, (4, 3)),
            (320, 200, (4, 3)),
            (640, 360, (16, 9)),
            (640, 480, (4, 3))
        ]
    );
}

#[test]
fn palette_changes_every_tick() {
    // Palette effects (fades, flashes) are per tick in the original; frontends must re-upload.
    let mut game = Game::new();
    game.tick();
    let before = *game.frame().palette;
    game.tick();
    assert_ne!(before, *game.frame().palette);
}

#[test]
fn a_held_key_is_visible_and_releasing_it_restores_the_picture() {
    // The manual input check relies on this feedback.
    let mut idle = Game::new();
    let mut pressed = Game::new();
    pressed.input(InputEvent::Key {
        key: Key::Space,
        pressed: true,
    });
    idle.tick();
    pressed.tick();
    assert_ne!(idle.frame().pixels, pressed.frame().pixels);

    pressed.input(InputEvent::Key {
        key: Key::Space,
        pressed: false,
    });
    idle.tick();
    pressed.tick();
    assert_eq!(idle.frame().pixels, pressed.frame().pixels);
}

#[test]
fn a_press_clicks_and_t_silences_the_tone() {
    // The latency check listens for the click; T lets the tester hear the click alone.
    let mut game = Game::new();
    press(&mut game, Key::T);
    let mut audio = Vec::new();
    game.tick();
    game.take_audio(&mut audio);
    assert!(audio.iter().any(|&s| s != 0), "pressing T clicks");

    audio.clear();
    game.tick();
    game.take_audio(&mut audio);
    assert!(
        audio.iter().all(|&s| s == 0),
        "tone off and click over: silence"
    );

    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    audio.clear();
    game.tick();
    game.take_audio(&mut audio);
    assert!(audio.iter().any(|&s| s != 0), "a pad button clicks too");
}

#[test]
fn a_repeated_press_without_release_does_not_click_again() {
    // Frontends should not forward OS key repeat; if one does, it must not machine-gun clicks.
    let mut game = Game::new();
    press(&mut game, Key::T);
    game.tick();
    game.tick();
    game.input(InputEvent::Key {
        key: Key::A,
        pressed: true,
    });
    game.tick();
    game.input(InputEvent::Key {
        key: Key::A,
        pressed: true,
    });
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    game.tick();
    audio.clear();
    game.take_audio(&mut audio);
    assert!(audio.iter().all(|&s| s == 0));
}

#[test]
fn stick_extremes_stay_on_screen_in_every_mode() {
    // The dot is drawn with unsigned coordinates; full deflection must not underflow.
    let mut game = Game::new();
    for _ in 0..3 {
        for (x, y) in [
            (i16::MIN, i16::MIN),
            (i16::MAX, i16::MAX),
            (i16::MIN, i16::MAX),
        ] {
            game.input(InputEvent::PadAxis {
                axis: PadAxis::StickX,
                value: x,
            });
            game.input(InputEvent::PadAxis {
                axis: PadAxis::StickY,
                value: y,
            });
            game.tick();
        }
        press(&mut game, Key::Tab);
    }
}

#[test]
fn grid_cells_look_square_on_screen_in_every_mode() {
    // Testers judge the aspect ratio by eye from these cells: cells that are not square make
    // correct scaling look stretched (the owner saw rectangles in 16:9).
    let mut game = Game::new();
    for _ in 0..3 {
        let frame = game.frame();
        let (width, height) = (frame.width as usize, frame.height as usize);
        let grey = |x: usize, y: usize| frame.pixels[y * width + x] == 2;
        // The top-left grid cell is the first run of at least 8 grey pixels; the ramp's grey
        // diagonal never makes runs longer than one pixel.
        let (x0, y0) = (0..height)
            .flat_map(|y| (0..width - 8).map(move |x| (x, y)))
            .find(|&(x, y)| (0..8).all(|i| grey(x + i, y)))
            .expect("the frame has a grid cell");
        let cell_width = (x0..width).take_while(|&x| grey(x, y0)).count();
        let cell_height = (y0..height).take_while(|&y| grey(x0, y)).count();
        // On screen a frame pixel is aspect.0 / width wide and aspect.1 / height tall.
        let shown_width = cell_width as f64 * f64::from(frame.aspect.0) / width as f64;
        let shown_height = cell_height as f64 * f64::from(frame.aspect.1) / height as f64;
        let ratio = shown_width / shown_height;
        assert!(
            (0.95..=1.05).contains(&ratio),
            "{width}x{height}: a {cell_width}x{cell_height} px cell shows at {ratio:.3}:1"
        );
        press(&mut game, Key::Tab);
    }
}
