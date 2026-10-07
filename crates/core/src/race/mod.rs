//! The race (spec M4): `startRace` (0x415710) from its load to its end. The race draws into
//! a buffer of rows 512 bytes apart ([`buffer`]), which the window shows doubled from row 40;
//! the track's view is 256 wide right of the HUD's 64.

mod ai;
mod buffer;
mod cars;
mod collisions;
mod driving;
mod flag;
mod guns;
mod help;
mod hud;
mod intro;
mod laps;
mod marks;
mod mines;
mod outro;
mod pause;
mod pedestrians;
mod power_ups;
mod raster;
mod scene;
mod semaphore;
mod waver;

use deadrally_gamedata::image::Palette;
use deadrally_gamedata::race::{RaceArchives, RaceError, Track};
use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::sound;
use deadrally_gamedata::text::HelpTexts;
use deadrally_gamedata::xm::Bank;

use crate::audio::Sound;
use crate::campaign::Rand;
use crate::keys::Keys;
use crate::trig::{cos, sin};

use self::buffer::{Buffer, LEFT, STRIDE};
use self::driving::Car;
use self::raster::ftol;

/// The race buffer's shown size, and the window rows it is doubled into from.
pub(crate) const VIEW_WIDTH: usize = 320;
pub(crate) const VIEW_HEIGHT: usize = 200;
pub(crate) const WINDOW_TOP: usize = 40;
/// The HUD's width once slid in (`leftMenuInRaceWidth` 0x456AA0), and the track's view right
/// of it (`raceEffectiveWidth` 256 from `initRaceValues` 0x409AB9, `raceEffectiveHeight` 200
/// and its half 100). The status bar sliding away (TAB) widens the view to 320.
const HUD_WIDTH: i64 = 64;
const TRACK_VIEW_WIDTH: i32 = 256;
const HALF_HEIGHT: i32 = 100;

/// A driver in the race as the preview hands them over, in their place on the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Driver {
    /// The name, upper-cased.
    pub(crate) name: Vec<u8>,
    pub(crate) car: usize,
    /// The level the car is set up for: the game's difficulty (0 to 2, 0x456738) for an
    /// opponent, 3 for the player (0x4330A5).
    pub(crate) level: usize,
    /// The engine, tires and armour upgrades.
    pub(crate) engine: i32,
    pub(crate) tires: i32,
    pub(crate) armour: i32,
    pub(crate) damage: i32,
    pub(crate) rocket: i32,
    pub(crate) mines: i32,
    /// Spiked wheels, which have sprites of their own.
    pub(crate) spikes: bool,
    /// The car's colour: `COPPER.PAL`'s for the player, `CARCOL.PAL`'s for the others.
    pub(crate) colour: [u8; 3],
}

/// The car colours' ramps: ten entries a driver from 15 (`initRaceValues_409F90`).
const RAMPS: [usize; 4] = [15, 25, 35, 45];

/// A help page's picture (`NAME.BPK`, 320x200) and palette (`NAME.PAL`) in `ENGINE.BPA`.
fn page(
    engine: &deadrally_gamedata::bpa::Archive,
    name: &str,
) -> Result<(Vec<u8>, Palette), RaceError> {
    let picture = hud::decoded(engine, &format!("{name}.BPK"))?;
    let bytes = engine.read(&format!("{name}.PAL"))?;
    let palette = Palette::from_bytes(bytes).map_err(|error| RaceError::Track {
        name: format!("{name}.PAL"),
        error: deadrally_gamedata::track::TrackError::Palette(error),
    })?;
    Ok((picture, palette))
}

/// `setCircuitPaletteValues` (0x409E50): five entries from a tenth of the colour up to it,
/// five from the colour towards 63; the blue's first step kept as a float, as the original
/// stores it.
fn car_ramp(palette: &mut Palette, first: usize, [r, g, b]: [u8; 3]) {
    let channel = |c: u8| {
        let c = f64::from(c);
        let tenth = c * 0.1;
        (tenth, (c - tenth) * 0.2)
    };
    let (r_tenth, r_step) = channel(r);
    let (g_tenth, g_step) = channel(g);
    let (b_tenth, b_step) = channel(b);
    let b_step = f64::from(b_step as f32);
    for i in 0..5 {
        let i64_ = f64::from(i);
        palette.0[first + i as usize] = [
            (i64_ * r_step + r_tenth) as i64 as u8,
            (i64_ * g_step + g_tenth) as i64 as u8,
            (i64_ * b_step + b_tenth) as i64 as u8,
        ];
    }
    let fifth = f64::from(0.2f32);
    let up = |c: u8| (63.0 - f64::from(c)) * fifth;
    for i in 0..5 {
        let i64_ = f64::from(i);
        palette.0[first + 5 + i as usize] = [
            (i64_ * up(r) + f64::from(r)) as i64 as u8,
            (i64_ * up(g) + f64::from(g)) as i64 as u8,
            (i64_ * up(b) + f64::from(b)) as i64 as u8,
        ];
    }
}

/// The cars' ramps in the race's palette (`initRaceValues` 0x409FCF on), the four places'
/// always: each from its driver's colour, those past the drivers from `spare` (left as the
/// track has them without one), but the first left as the track has it when its car is the
/// Adversary's.
fn car_ramps(palette: &mut Palette, drivers: &[Driver], spare: Option<[u8; 3]>) {
    for (place, &first) in RAMPS.iter().enumerate() {
        let colour = match drivers.get(place) {
            Some(driver) if place == 0 && driver.car == driving::ADVERSARY_CAR => None,
            Some(driver) => Some(driver.colour),
            None => spare,
        };
        if let Some(colour) = colour {
            car_ramp(palette, first, colour);
        }
    }
}

/// Whether a car's headlights shine (0x40DAB9, 0x40DD5D): not once it is wrecked or has
/// finished.
fn lit(car: &Car) -> bool {
    car.handling.damage > 0 && !car.finished
}

/// Whether `drawShadows` draws a shadow with these corners in a view `half_width` across from
/// its middle: one corner across the view and one (maybe another) down it; a shadow whose
/// corners all lie outside is left out even where it would cover the view.
fn shadow_in_view(points: [(i32, i32); 3], half_width: i32) -> bool {
    let near = |value: i32, half: i32| (value - half).abs() < half;
    points.iter().any(|&(x, _)| near(x, half_width))
        && points.iter().any(|&(_, y)| near(y, HALF_HEIGHT))
}

#[derive(Debug)]
pub(crate) struct Race {
    track: Track,
    drivers: Vec<Driver>,
    cars: Vec<Car>,
    /// The player's place on the grid.
    player: usize,
    weapons: bool,
    laps: i32,
    hud: hud::HudImages,
    /// Every car's sprites (0x5034FC).
    sprites: Vec<u8>,
    /// What shadows turn each colour into (`ENGINE.BPA`'s `VARJO.TAB`, 0x466F00).
    shade: [u8; 256],
    /// The scene's lights and pictures, worked out once.
    scene: scene::Setup,
    /// Which track: the first leaves its scene's far objects in.
    number: usize,
    buffer: Buffer,
    /// The race's palette (0x4A9BA0): the track's with the cars' ramps.
    palette: Palette,
    /// The palette and the 320x200 screen as shown.
    shown: Palette,
    screen: Vec<u8>,
    stage: Stage,
    semaphore: semaphore::Semaphore,
    pedestrians: pedestrians::Pedestrians,
    clock: Clock,
    /// The track's music and the race's sounds (`GEN-EFE.CMF`).
    music: Module,
    effects: Bank,
    /// The pause's box (`GEN-MES.BPK`, 0x479688) and its nine lines.
    pause_box: Vec<u8>,
    pause_lines: Vec<Vec<u8>>,
    /// The box's lines at the race's end, and the ticks the end has come nearer by
    /// (0x4AA508): the race ends past 300.
    race_over_lines: Vec<Vec<u8>>,
    over_ticks: i32,
    /// The box's lines when P pauses the game.
    paused_lines: Vec<Vec<u8>>,
    /// The music's and the effects' volumes in `dr.cfg`, which F2 and F3 turn back on.
    volumes: (u32, u32),
    /// The help's pages and texts, the gamepad's inputs for the controls (`dr.cfg`), and
    /// whether the pause asked for the help (F1 left held, 0x4069BA).
    help_pages: help::Pages,
    help_texts: HelpTexts,
    pads: [u32; 7],
    help_asked: bool,
    /// The opponents kept still.
    still: bool,
    /// The scancodes of the eight controls in `dr.cfg`.
    controls: [u32; 8],
    /// The player's keys as the timer samples them each tick (0x4A7D60), and where the next
    /// goes (0x4A7DA0).
    samples: [u32; 16],
    sampled: usize,
    /// The ticks the player's rocket has burned (0x456AAC).
    rocket_ticks: i32,
    /// The view's lead ahead of the player's car.
    lead: Lead,
    /// The view's corner on the track for the frame being drawn (0x456ABC, 0x456AC0), and its
    /// width (0x445010), 256 right of the HUD and up to 320 with the status bar slid away.
    view: (usize, usize),
    view_width: i32,
    /// The smoke puffs' pictures (`SMOKE.BPK`).
    smoke: Vec<u8>,
    power_ups: power_ups::PowerUps,
    /// How loud the player's tires squeal (0x4AA92C) and whether the squeal plays
    /// (0x456AD0).
    squeal: i32,
    squealing: bool,
    /// Whether the start has been given (`0x456AD4` at 2).
    started: bool,
    /// The laps' clocks, times and calls, and the wrecks in the order they were wrecked.
    laps_state: laps::Laps,
    wrecks: Vec<usize>,
    /// The car of the driver whose armour counts 2.2 times, who has a call when winning;
    /// whether it is the player's, who has a call when wrecked.
    tough: Option<usize>,
    player_tough: bool,
    /// The HUD's calls on the player's damage made this race.
    damage_calls: hud::DamageCalls,
    /// What a money power-up is worth (0x4A7AB0), and whether the player took the bonus
    /// power-up (0x4A7AAC).
    pickup_money: i32,
    bonus: bool,
    /// The race chosen at the sign-up, and the balance's fractions.
    race: usize,
    balance: Vec<f32>,
    /// The machine guns' muzzle flashes by kind (`FLAME1.BPK` to `FLAME6.BPK`), their hits'
    /// sparks (`SHOTS.BPK`), what a hit takes off by the shooter's car, and the last hit.
    flashes: Vec<Vec<u8>>,
    sparks: Vec<u8>,
    gun_damage: Vec<f32>,
    gun_hits: guns::Shared,
    /// The mines dropped (`MINES1A.BPK`) and their blasts (`BLOWI.BPK`).
    mines: mines::Mines,
    /// A wreck's fire (`BURN1A.BPK`).
    fire: Vec<u8>,
    /// The chequered flag once a car has finished.
    flag: flag::Flag,
    /// The HUD's medals of the places.
    medals: hud::Medals,
    /// The rocket's flames (`ROCKET1.BPK`, `ROCKET2.BPK`); the one shown is the session's.
    rocket_flames: [Vec<u8>; 2],
    /// The effect power-up's waves.
    waves: waver::Waves,
    /// What the original keeps from race to race.
    session: Session,
}

/// What the original keeps in its globals from race to race, never set back: the switches
/// the race's keys turn (TAB the status bar 0x445028 and its press 0x46F200, F2 the music
/// 0x445020, F3 the effects 0x445024, F4 the scene's pictures 0x44502C, F5 the shadows
/// 0x445030), all on at the game's start; the effect power-up's waves' phase (0x456AF4); and
/// the rocket flames' picture (0x456AFC), which only a flame's turn writes (0x40F651).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Session {
    status_bar: bool,
    tab_held: bool,
    music: bool,
    effects: bool,
    pictures: bool,
    shadows: bool,
    waves: i32,
    flame_phase: usize,
}

impl Session {
    /// The race's switch keys in a pass (0x416D13). The music's and the effects' switches
    /// that F2 and F3 turned.
    fn keys(&mut self, keys: &mut Keys, wrecked: bool) -> [Option<bool>; 2] {
        let tab = keys.held(TAB);
        if tab && !self.tab_held {
            self.status_bar = !self.status_bar;
        }
        self.tab_held = tab;
        if wrecked {
            self.status_bar = true;
        }
        let mut turn = |switch: &mut bool, code: u8| {
            let held = keys.held(code);
            if held {
                *switch = !*switch;
                keys.release(code);
            }
            held.then_some(*switch)
        };
        let music = turn(&mut self.music, MUSIC_KEY);
        let effects = turn(&mut self.effects, EFFECTS_KEY);
        turn(&mut self.pictures, PICTURES_KEY);
        turn(&mut self.shadows, SHADOWS_KEY);
        [music, effects]
    }
}

impl Default for Session {
    fn default() -> Session {
        Session {
            status_bar: true,
            tab_held: false,
            music: true,
            effects: true,
            pictures: true,
            shadows: true,
            waves: 0,
            flame_phase: 0,
        }
    }
}

/// How a race is set up: the track (`TRn`, 0x45EA50) and whether it is turned round
/// (0x4A7AA8) and its laps, the player's place on the grid and whether the race has weapons,
/// the pause box's lines, the eight controls' scancodes in `dr.cfg`, what a money power-up is
/// worth, and the circuit's lap record for the player's car (minutes, seconds, hundredths,
/// from `dr.cfg`).
pub(crate) struct Setup {
    pub(crate) track: usize,
    pub(crate) reversed: bool,
    /// The colour of the cars' ramps past the drivers (in the Arena, the third and fourth
    /// places', `CARCOL.PAL`'s entry 10: 0x4332EE); none leaves the track's colours there.
    pub(crate) spare_ramps: Option<[u8; 3]>,
    /// The race chosen at the sign-up, 0 to 2 (0x456B88).
    pub(crate) race: usize,
    pub(crate) laps: i32,
    pub(crate) player: usize,
    pub(crate) weapons: bool,
    pub(crate) pause_lines: Vec<Vec<u8>>,
    pub(crate) race_over_lines: Vec<Vec<u8>>,
    pub(crate) paused_lines: Vec<Vec<u8>>,
    pub(crate) help: HelpTexts,
    pub(crate) controls: [u32; 8],
    pub(crate) pads: [u32; 7],
    /// The opponents kept still (the reference runner's `--no-ai`).
    pub(crate) still: bool,
    pub(crate) pickup_money: i32,
    pub(crate) lap_record: [i32; 3],
    /// What the last race left in the original's globals.
    pub(crate) session: Session,
}

/// `recalculateCircuitImageOffset`'s lead (0x40D560): the view runs ahead of a moving car,
/// a fifth of the way to where its speed points each frame the target changes, then
/// closing in over the frames after.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Lead {
    target: [i32; 2],
    at: [i32; 2],
    steps: i32,
}

/// The sounds' channels and pitches (16.16) in the race's calls of `loadMenuSoundEffect`.
const ENGINE_CHANNEL: usize = 1;
const LIGHTS_CHANNEL: usize = 2;
const START_CHANNEL: usize = 5;
/// The engine's sound for car 0; the cars' follow it.
const ENGINE_SOUND: u8 = 25;
const ENGINE_PITCH: u32 = 0x2_8000;
const READY_SOUND: (u8, u32) = (3, 0x5_0000);
const SET_SOUND: (u8, u32) = (44, 0x2_0000);
const GO_SOUND: (u8, u32) = (44, 0x2_8000);
const FULL: u32 = 0x1_0000;
/// The channels of the crash sounds and of the fire.
const CRASH_CHANNEL: usize = 5;
const FIRE_CHANNEL: usize = 6;
/// A car-car knock's sound by the timer's tick (0x4167C9), and a wall's of kind 0.
const KNOCK_SOUNDS: [(u8, u32); 3] = [(11, 0x2_1000), (13, 0x2_3000), (16, 0x2_3000)];
const WALL_SOUNDS: [u8; 3] = [10, 15, 16];
/// The most a knock takes off a car at once.
const MAX_HURT: i32 = 10_000;
/// The countdown's frames: the cars move from the next.
const START_FRAME: i32 = 190;
/// A pedestrian's scream (`sub_410FA0`).
const SCREAM_PITCH: u32 = 0x5_0000;
/// A power-up picked up (`sub_410B90`), and the effect power-up's call.
const PICKUP_SOUND: u8 = 18;
const PICKUP_PITCH: u32 = 0x2_1000;
const EFFECT_CALL: u8 = 6;
/// The race's calls: the last lap, a record, being lapped (`laps`).
const CALL_CHANNEL: usize = 2;
const CALL_PITCH: u32 = 0x5_0000;
/// The tires' squeal (on the fire's channel).
const SQUEAL_SOUND: u8 = 37;

/// The race's clocks: its frame count (`raceFrame` 0x481E14, the countdown under 190), the
/// ticks the timer has counted (0x503500, and at the HUD's last frame 0x4A7CFC), the ticks
/// between the HUD's last two frames (0x4A9EA4), and the ticks since the loop last looked
/// (0x4A9EAC), which the timer counts to 14 and back to 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Clock {
    /// The timer's ticks since the game started (`sub_43C760`), which pedestrians step by.
    timer: u32,
    frame: i32,
    ticks: i32,
    seen: i32,
    between: i32,
    waiting: i32,
}

impl Clock {
    /// A tick of the timer (`sub_4138A0`).
    fn tick(&mut self) {
        self.timer = self.timer.wrapping_add(1);
        self.waiting = if self.waiting + 1 >= 15 {
            0
        } else {
            self.waiting + 1
        };
        self.ticks += 1;
    }

    /// The clocks from 0 (0x4053E0, and again at the start 0x415079).
    fn restart(&mut self) {
        self.waiting = 0;
        self.ticks = 0;
        self.seen = 0;
    }
}

/// Where the race is: at the race loop's wait for the next tick (`0x4173A0`), the frame drawn
/// into the buffer but not yet onto the screen; or in the intro, which runs over the loop's
/// first frame.
#[derive(Debug)]
enum Stage {
    Loop {
        first: bool,
    },
    /// The intro; whether the race was abandoned before it, and ends once it is over.
    Intro {
        intro: Box<intro::Intro>,
        ending: bool,
    },
    /// The pause; whether it came before the intro, or is the box at the race's end.
    Pause {
        pause: Box<pause::Pause>,
        first: bool,
        ending: bool,
    },
    /// The help (F1), the music's order it interrupted, and whether it came in the loop's
    /// first pass, before the intro.
    Help {
        help: Box<help::Help>,
        order: usize,
        first: bool,
    },
    /// The game paused (P): the box, the music's order it interrupted, and whether it came in
    /// the loop's first pass.
    Paused {
        pause: Box<pause::Pause>,
        order: usize,
        first: bool,
    },
    /// The race ended, abandoned or over: the loop's last frame shown, the view tilting away
    /// from the next tick.
    Ended(Outcome),
    /// The view tilting away, and how the race came to its end.
    Outro {
        outro: Box<outro::Outro>,
        outcome: Outcome,
    },
}

/// What a tick of the race came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Racing,
    /// The player answered Y to the pause's question, and the view has tilted away.
    Aborted,
    /// The race over, its box answered, and the view tilted away.
    Over,
}

/// What follows the pause once its box has flown apart (from 0x4176F8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AfterPause {
    /// The race goes on.
    Race,
    /// The race goes on, from the intro the pause came before.
    Intro,
    /// The race is abandoned: its last frame, then the view tilting away.
    End,
    /// The race is abandoned, but the intro the pause came before still runs first.
    IntroThenEnd,
}

/// The pause's `answer`, the pause having come before the intro when `first`: abandoning the
/// race (0x41770D) goes on to 0x417862 like the other answers, where the loop's first pass
/// runs the intro, and only then leaves the loop.
fn after_pause(answer: pause::Answer, first: bool) -> AfterPause {
    match (answer, first) {
        (pause::Answer::Abort, true) => AfterPause::IntroThenEnd,
        (pause::Answer::Abort, false) => AfterPause::End,
        (_, true) => AfterPause::Intro,
        (_, false) => AfterPause::Race,
    }
}

/// Escape's scancode, which pauses the race.
const ESCAPE: u8 = 0x01;
/// The channels the pauses, the help and the race's end silence (1 to 14).
const CHANNELS: usize = 14;
/// The race's end: past these ticks of its counter, and its call.
const OVER_TICKS: i32 = 300;
const END_CALL: u8 = 5;
/// F1's scancode, the music's orders the help plays by the track (`TR0` to `TR9`, 0x416B6E),
/// and the sound's masks during the help and after.
const HELP_KEY: u8 = 0x3B;
const HELP_ORDERS: [usize; 10] = [0x1E, 0x37, 0x2D, 0x32, 0x2D, 0x37, 0x32, 0x32, 0x32, 0x32];
const HELP_MASK: u32 = 0x8000 >> 8;
const FULL_MASK: u32 = 0x1_0000 >> 8;
/// The race's other keys (0x416D13): TAB the status bar, F2 the music, F3 the effects, F4 the
/// scene's pictures, F5 the shadows, P the game paused.
const TAB: u8 = 0x0F;
const MUSIC_KEY: u8 = 0x3C;
const EFFECTS_KEY: u8 = 0x3D;
const PICTURES_KEY: u8 = 0x3E;
const SHADOWS_KEY: u8 = 0x3F;
const PAUSE_KEY: u8 = 0x19;
/// The status bar slides away 2 pixels a tick and back 4 (0x417497).
const SLIDE_OUT: i32 = 2;
const SLIDE_IN: i32 = 4;
const PAUSE_CHANNEL: usize = 5;
const PAUSE_PITCH: u32 = 0x2_8000;

impl Race {
    /// The race on track `TRn` (n = `track`) over `laps` laps, the drivers in their places,
    /// the player in place `player`; `pause_lines` the pause box's.
    pub(crate) fn new(
        archives: &RaceArchives,
        setup: Setup,
        drivers: Vec<Driver>,
        rand: &mut Rand,
    ) -> Result<Race, RaceError> {
        let Setup {
            track: number,
            reversed,
            spare_ramps,
            race,
            laps,
            player,
            weapons,
            pause_lines,
            race_over_lines,
            paused_lines,
            help,
            controls,
            pads,
            still,
            pickup_money,
            lap_record,
            session,
        } = setup;
        let mut track = Track::load(&archives.tracks[number], number)?;
        // The scene's lights and pictures are worked out before the track is turned round
        // (0x4161EC and 0x416206 come before 0x416304).
        let scene = scene::Setup::new(&track.scene);
        if reversed {
            let name = format!("TR{number}-FLIP.PAL");
            let flip =
                Palette::from_bytes(archives.tracks[number].read(&name)?).map_err(|error| {
                    RaceError::Track {
                        name,
                        error: deadrally_gamedata::track::TrackError::Palette(error),
                    }
                })?;
            track = track.reversed(flip);
        }
        let obstacles = hud::decoded(
            &archives.engine,
            if reversed {
                "OBST_REV.BPK"
            } else {
                "OBSTACLE.BPK"
            },
        )?;
        let spots = track.info.power_ups;
        let power_ups = power_ups::PowerUps::new(&mut track.image, &spots, obstacles, rand);
        let cars = drivers
            .iter()
            .enumerate()
            .map(|(slot, driver)| {
                let [mut x, mut y, mut rotation] = track.info.starts[slot];
                if reversed {
                    // 0x40ACC0: across the track, and half round (one step short of it from
                    // the first half of the directions, as the original counts).
                    x = track.info.width as i32 - x - 1;
                    y = track.info.height as i32 - y - 1;
                    rotation = if rotation < 48 {
                        rotation + 47
                    } else {
                        rotation - 48
                    };
                }
                let handling = match drivers.get(1) {
                    // 0x401FBC: the Adversary's car, first on the grid, is set up apart.
                    Some(second) if slot == 0 && driver.car == driving::ADVERSARY_CAR => {
                        driving::Handling::adversary(&archives.handling, driver, second, weapons)
                    }
                    _ => {
                        driving::Handling::new(&archives.handling, driver, slot == player, weapons)
                    }
                };
                Car::new(
                    (x as f32, y as f32, rotation),
                    slot,
                    handling,
                    driver.engine,
                )
            })
            .collect();
        let hud = hud::HudImages::load(&archives.ib_files, player, drivers[player].car, weapons)?;
        let looks: Vec<(usize, bool)> = drivers.iter().map(|d| (d.car, d.spikes)).collect();
        let sprites = cars::sprites(&archives.engine, &looks, reversed)?;
        let mut shade = [0; 256];
        let table = archives.engine.read("VARJO.TAB")?;
        shade[..table.len().min(256)].copy_from_slice(&table[..table.len().min(256)]);
        let music = sound::load_music(&archives.musics, &format!("TR{number}-MUS.CMF"))
            .map_err(RaceError::Sound)?;
        let effects =
            sound::load_effects(&archives.musics, "GEN-EFE.CMF").map_err(RaceError::Sound)?;
        let pedestrians = pedestrians::Pedestrians::new(
            &track.info,
            reversed,
            hud::decoded(&archives.engine, "PEDESTR.BPK")?,
            [
                hud::decoded(&archives.engine, "SPLAT3.BPK")?,
                hud::decoded(&archives.engine, "SPLAT4.BPK")?,
            ],
        );
        let is_tough = |driver: &Driver| {
            let tough = &archives.handling.tough;
            let mut name = driver.name.clone();
            name.push(0);
            name.get(..tough.len()) == Some(tough.as_slice())
        };
        let tough = drivers.iter().position(is_tough);
        let player_tough = is_tough(&drivers[player]);
        let mut palette = track.palette.clone();
        car_ramps(&mut palette, &drivers, spare_ramps);
        let mut race = Race {
            track,
            drivers,
            cars,
            player,
            weapons,
            laps,
            hud,
            sprites,
            shade,
            scene,
            number,
            buffer: Buffer::default(),
            palette,
            // `setCircuitPalette` (0x4049F0) blacks every entry out before the loop starts.
            shown: Palette::BLACK,
            screen: vec![0; VIEW_WIDTH * VIEW_HEIGHT],
            stage: Stage::Loop { first: true },
            semaphore: semaphore::Semaphore::new(hud::decoded(&archives.engine, "GEN-LAM.BPK")?),
            pedestrians,
            // The timer has run for minutes by any race; what counts is that it is past the
            // pedestrians' first step, which comes on the first frame as in the original.
            // Loading the race leaves the timer a tick behind the clock, which the first
            // frame's pedestrians catch up before its HUD counts the ticks since the set-up
            // (0x4022A0): the first frame's `between` is 1, as the power-ups' wait shows.
            clock: Clock {
                timer: 1000,
                ticks: 1,
                ..Clock::default()
            },
            music,
            effects,
            pause_box: hud::decoded(&archives.engine, "GEN-MES.BPK")?,
            pause_lines,
            race_over_lines,
            over_ticks: 0,
            paused_lines,
            volumes: (0, 0),
            help_pages: help::Pages {
                keys: page(&archives.engine, "KEYCOM3")?,
                info: page(&archives.engine, "INFO2")?,
            },
            help_texts: help,
            pads,
            still,
            help_asked: false,
            controls,
            samples: [0; 16],
            sampled: 0,
            rocket_ticks: 0,
            lead: Lead::default(),
            view: (0, 0),
            view_width: TRACK_VIEW_WIDTH,
            smoke: hud::decoded(&archives.engine, "SMOKE.BPK")?,
            power_ups,
            squeal: 0,
            squealing: false,
            started: false,
            laps_state: laps::Laps {
                record: lap_record,
                ..laps::Laps::default()
            },
            pickup_money,
            bonus: false,
            race,
            balance: archives.handling.balance.clone(),
            flashes: (1..=6)
                .map(|kind| hud::decoded(&archives.engine, &format!("FLAME{kind}.BPK")))
                .collect::<Result<_, _>>()?,
            sparks: hud::decoded(&archives.engine, "SHOTS.BPK")?,
            gun_damage: archives.handling.gun_damage.clone(),
            gun_hits: guns::Shared::default(),
            mines: mines::Mines::new(
                hud::decoded(&archives.engine, "MINES1A.BPK")?,
                hud::decoded(&archives.engine, "BLOWI.BPK")?,
            ),
            fire: hud::decoded(&archives.engine, "BURN1A.BPK")?,
            flag: flag::Flag::new(hud::decoded(&archives.engine, "GEN-FLA.BPK")?),
            medals: hud::Medals::new(&[]),
            rocket_flames: [
                hud::decoded(&archives.engine, "ROCKET1.BPK")?,
                hud::decoded(&archives.engine, "ROCKET2.BPK")?,
            ],
            wrecks: Vec::new(),
            tough,
            player_tough,
            damage_calls: hud::DamageCalls::default(),
            waves: waver::Waves::default(),
            session,
        };
        let places: Vec<i32> = race
            .board_order()
            .iter()
            .map(|&slot| race.cars[slot].place)
            .collect();
        race.medals = hud::Medals::new(&places);
        Ok(race)
    }

    /// The race's sound set up (0x416215: the menu's stopped, the track's music started at
    /// the configured volumes but silent until the intro raises it, the player's engine), then
    /// the race loop's first pass, its keys checked as every pass's are, up to its wait.
    pub(crate) fn begin(
        &mut self,
        sound: &mut Sound,
        (music_volume, effects_volume): (u32, u32),
        keys: &mut Keys,
        rand: &mut Rand,
    ) {
        sound.stop();
        sound.load_effects(&self.effects);
        sound.set_mask(0);
        sound.play_music(&self.music, 0, music_volume);
        sound.set_effects_volume(effects_volume);
        // 0x4162D1: the music or the effects the player turned off stay off.
        self.volumes = (music_volume, effects_volume);
        if !self.session.music {
            sound.set_music_volume(0);
        }
        if !self.session.effects {
            sound.set_effects_volume(0);
        }
        let car = self.drivers[self.player].car as u8;
        sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
        self.frame(sound, rand);
        self.keys_and_draw(sound, keys, rand);
    }

    /// A pass of the race loop up to its wait (0x416390): the player's keys of the ticks
    /// waited, a step of the race for each, then the frame drawn into the buffer.
    fn frame(&mut self, sound: &mut Sound, rand: &mut Rand) {
        let steps = self.clock.waiting as usize;
        self.clock.waiting = 0;
        // 0x4163AD: the lap's call when its wait is over.
        let laps = &mut self.laps_state;
        if laps.call > 0 {
            laps.call -= self.clock.between;
            if laps.call <= 0 {
                sound.trigger_at(CALL_CHANNEL, laps::RECORD, FULL, CALL_PITCH);
                laps.call = 0;
            }
        }
        // The samples since the last pass, oldest first (0x41641F).
        let mut at = self.sampled;
        for k in 0..steps {
            at = (at + 15) % 16;
            self.cars[self.player].keys[steps - 1 - k] = self.samples[at];
        }
        if self.clock.frame > START_FRAME {
            for horn in horns(&mut self.cars, steps, self.player) {
                match horn {
                    Horn::Start {
                        channel,
                        effect,
                        volume,
                        pitch,
                    } => sound.trigger_at(channel, effect, volume, pitch),
                    Horn::Stop(channel) => sound.stop_channel(channel),
                }
            }
        }
        self.power_ups.step(
            &mut self.track.image,
            self.clock.between,
            self.weapons,
            rand,
            self.laps_state.over,
        );
        for tick in 0..steps {
            self.clock.frame += 1;
            if self.clock.frame > START_FRAME {
                self.drive(tick, sound, rand);
            }
            self.after_tick(tick, sound, rand);
        }
        // 0x416AF7: not in the Adversary's race.
        if self.drivers[0].car != 6 {
            self.balance();
        }
    }

    /// `balanceIAEngineInRace` (0x40B920) once a pass: an opponent a zone or two behind the
    /// player gets more engine, one ahead less (not in the third race), by the game's
    /// difficulty; every other car runs on its own engine.
    fn balance(&mut self) {
        let level = self.drivers[usize::from(self.player == 0)].level;
        let fraction = |index: usize| f64::from(self.balance.get(index).copied().unwrap_or(0.0));
        let zones = self.track.info.zones;
        let progress = |car: &Car| (car.lap & 0xFF) * zones + car.zone;
        let mine = progress(&self.cars[self.player]);
        let mut factors = Vec::with_capacity(self.cars.len());
        for (slot, car) in self.cars.iter().enumerate() {
            let mut factor = 1.0;
            if slot != self.player && car.handling.damage > 0 {
                let theirs = progress(car);
                if theirs == mine - 1 {
                    factor = fraction(2 * level) + 1.0;
                }
                if theirs <= mine - 2 {
                    factor = fraction(2 * level + 1) + 1.0;
                }
                if self.race != 2 {
                    if theirs == mine + 1 {
                        factor = 1.0 - fraction(6 + 2 * level);
                    }
                    if theirs >= mine + 2 {
                        factor = 1.0 - fraction(7 + 2 * level);
                    }
                }
            }
            factors.push(factor);
        }
        for (car, factor) in self.cars.iter_mut().zip(factors) {
            car.handling.engine = (factor * f64::from(car.handling.engine_backup)) as f32;
        }
    }

    /// The cars' step of a tick once the race is on (0x4164B6): the opponents' driving comes
    /// with M5, so they hold no keys.
    fn drive(&mut self, tick: usize, sound: &mut Sound, rand: &mut Rand) {
        // 0x4164C1: the opponents decide their keys.
        if !self.still {
            let mines = self.mines.places();
            let guide = ai::Guide {
                guide: &self.track.guide.pixels,
                zones: &self.track.zones.pixels,
                width: self.track.zones.width as i32,
                speed: &self.track.zone_speed,
                steering: &self.track.zone_steering,
                offset: &self.track.zone_offset,
                track: (self.track.info.width as i32, self.track.info.height as i32),
                mines: &mines,
                sprites: &self.sprites,
            };
            for slot in (0..self.cars.len()).filter(|&slot| slot != self.player) {
                ai::steer(&mut self.cars, slot, tick, &guide, rand);
            }
        }
        for car in &mut self.cars {
            car.wall = 0;
        }
        let ground = driving::Ground {
            mask: &self.track.mask.pixels,
            width: self.track.info.width as i32,
            height: self.track.info.height as i32,
        };
        for (slot, car) in self.cars.iter_mut().enumerate() {
            let keys = car.keys[tick];
            let rocket = (slot == self.player).then_some(&mut self.rocket_ticks);
            car.drive(keys, slot, &ground, &self.sprites, rand, rocket);
        }
        for car in &mut self.cars {
            car.knocks = [0, 0];
        }
        let spikes: Vec<bool> = self.drivers.iter().map(|driver| driver.spikes).collect();
        collisions::collide(&mut self.cars, &self.sprites, &spikes);
        // `sub_40F6A0` for each car: the mines it drops and sets off.
        let width = self.track.info.width as i32;
        for slot in 0..self.cars.len() {
            let keys = self.cars[slot].keys[tick];
            let mut track = mines::Track {
                image: &mut self.track.image.pixels,
                width,
            };
            let blasts = self.mines.step(
                &mut self.cars,
                slot,
                keys,
                self.clock.frame,
                &self.sprites,
                &mut track,
                self.player,
                rand,
            );
            for (channel, effect, volume) in blasts {
                sound.trigger_at(channel, effect, volume, PICKUP_PITCH);
            }
        }
        // `sub_410FA0` for each car: the pedestrians it runs over.
        for slot in 0..self.cars.len() {
            let player = &self.cars[self.player];
            let near = (slot != self.player).then_some((player.x, player.y));
            let screams = self
                .pedestrians
                .hit(&mut self.cars[slot], &self.sprites, near, rand);
            for (channel, effect, volume) in screams {
                sound.trigger_at(channel, effect, volume, SCREAM_PITCH);
            }
        }
        // `sub_410B90` for each car: the power-ups it drives over.
        let player = &self.cars[self.player];
        let player_at = (player.x, player.y);
        for slot in 0..self.cars.len() {
            let near = (slot != self.player).then_some(player_at);
            let taken = self.power_ups.pick_up(
                &mut self.track.image,
                &mut self.cars[slot],
                &self.sprites,
                near,
                rand,
            );
            self.bonus |= taken.bonus;
            for picked in taken.sounds {
                match picked {
                    power_ups::PickupSound::Picked { channel, volume } => {
                        sound.trigger_at(channel, PICKUP_SOUND, volume, PICKUP_PITCH);
                    }
                    power_ups::PickupSound::Effect => {
                        sound.trigger_at(CALL_CHANNEL, EFFECT_CALL, FULL, CALL_PITCH);
                    }
                }
            }
        }
        // `sub_40E180` for each car: its machine guns.
        let ground = (
            self.track.mask.pixels.as_slice(),
            self.track.info.width as i32,
            self.track.info.height as i32,
        );
        for slot in 0..self.cars.len() {
            let keys = self.cars[slot].keys[tick];
            let shots = guns::fire(
                &mut self.cars,
                slot,
                keys,
                self.clock.frame,
                &self.sprites,
                ground,
                &mut self.pedestrians,
                self.player,
                &mut self.gun_hits,
                &self.gun_damage,
                rand,
            );
            for (channel, effect, volume) in shots {
                let pitch = if channel == 3 {
                    SCREAM_PITCH
                } else {
                    PICKUP_PITCH
                };
                sound.trigger_at(channel, effect, volume, pitch);
            }
        }
    }

    /// What every tick does after the cars' steps (0x41661A): the counters of walls and
    /// knocks, the knocks' damage, the player's crash sounds and engine.
    fn after_tick(&mut self, tick: usize, sound: &mut Sound, rand: &mut Rand) {
        for car in &mut self.cars {
            if car.stuck > 0 {
                car.stuck -= 1;
            }
            if car.knocked > 0 {
                car.knocked -= 1;
            }
            if car.wall == 1 {
                car.stuck += 2;
            }
            if car.knocks[0] == collisions::KNOCKED_BACK {
                car.x = car.previous[0];
                car.knocked += 2;
            }
            if car.knocks[1] == collisions::KNOCKED_BACK {
                car.y = car.previous[1];
                car.knocked += 2;
            }
        }
        for car in &mut self.cars {
            car.previous = [car.x, car.y, car.angle];
        }
        // 0x4166E0: the push back from a wall or a car hurts as its square, less the armour.
        for car in &mut self.cars {
            if (car.stuck == 2 || car.knocked == 2) && !car.finished {
                let [x, y] = car.push.map(f64::from);
                let hurt =
                    ftol((x * x + y * y) * f64::from(0x400 - car.handling.armour)).min(MAX_HURT);
                car.handling.damage = (car.handling.damage - hurt).max(0);
            }
        }
        self.crash_sounds(sound, rand);
        // 0x4168E6: the tires' squeal from the tick before, once the start is given.
        let player = &self.cars[self.player];
        let alive = player.handling.damage > 0 && !player.finished;
        if self.squeal > 0 && !self.squealing && alive && self.started {
            sound.trigger_at(FIRE_CHANNEL, SQUEAL_SOUND, self.squeal as u32, ENGINE_PITCH);
            self.squealing = true;
        }
        if self.squeal == 0 && self.squealing {
            sound.stop_channel(FIRE_CHANNEL);
            self.squealing = false;
        }
        self.squeal = 0;
        if player.handling.damage <= 0 || player.finished {
            sound.stop_channel(ENGINE_CHANNEL);
            sound.stop_channel(FIRE_CHANNEL);
        }
        // 0x4169A2: the engine's pitch from the speed.
        let speed = (f64::from(player.speed) / f64::from(player.handling.engine)).abs();
        let [idle, rise] = player.note;
        let pitch =
            ftol(f64::from(rise.wrapping_mul(5)) * speed + f64::from(idle.wrapping_add(0x2_8000)));
        sound.set_channel(ENGINE_CHANNEL, FULL, pitch as u32);
        let mut track = marks::Track {
            mask: &self.track.mask.pixels,
            image: &mut self.track.image.pixels,
            width: self.track.info.width as i32,
            skid: &self.track.skid,
            blood: &self.track.blood,
        };
        for (slot, car) in self.cars.iter_mut().enumerate() {
            let keys = car.keys[tick];
            let squeal = (slot == self.player).then_some(&mut self.squeal);
            marks::roll(car, keys, &mut track, rand, squeal, self.weapons);
        }
        let zones = laps::Zones {
            map: &self.track.zones.pixels,
            width: self.track.zones.width as i32,
            count: self.track.info.zones,
        };
        let race = laps::Race {
            player: self.player,
            laps: self.laps,
            intro_track: self.number == 0,
            // 0x413274 reads the first driver's car: the Adversary's in the last race.
            special: self.drivers[0].car == 6,
            tough: self.tough,
        };
        for laps::Call(effect) in laps::check(&mut self.cars, &zones, &mut self.laps_state, &race) {
            sound.trigger_at(CALL_CHANNEL, effect, FULL, CALL_PITCH);
        }
        laps::place_wrecks(&mut self.cars, &mut self.wrecks);
        self.over_ticks += laps::ending(&self.cars, self.player);
    }

    /// 0x41674F: the player's car against a wall or a car, its sounds as loud as the push
    /// back; `rand()` drawn for the wall's pitch.
    fn crash_sounds(&mut self, sound: &mut Sound, rand: &mut Rand) {
        let player = &self.cars[self.player];
        if player.wall == 0 && player.knocks == [0, 0] {
            return;
        }
        let [x, y] = player.push.map(f64::from);
        let volume = ftol((x * x + y * y).sqrt() * 25000.0).min(FULL as i32) as u32;
        let third = self.clock.ticks.rem_euclid(3);
        if player.knocks != [0, 0] {
            let (effect, pitch) = KNOCK_SOUNDS[third as usize];
            sound.trigger_at(CRASH_CHANNEL, effect, volume, pitch);
        }
        let pitch = (rand.next() % 0x6000 + 0x2_2000) as u32;
        let effect = match player.hit {
            0 => Some(WALL_SOUNDS[third as usize]),
            1 => Some(11),
            2 => Some(15),
            3 => Some(16),
            _ => None,
        };
        if let Some(effect) = effect {
            sound.trigger_at(CRASH_CHANNEL, effect, volume, pitch);
        }
    }

    /// From this wait to the next: the frame drawn onto the screen and, on the first, the
    /// intro; or the intro's next step, and once it is over the loop's next frame.
    pub(crate) fn tick(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand) -> Outcome {
        self.samples[self.sampled] = sample(keys, &self.controls, &self.pads);
        self.sampled = (self.sampled + 1) % 16;
        self.clock.tick();
        match &mut self.stage {
            Stage::Loop { first } => {
                let first = *first;
                self.show_frame();
                // 0x417517: Escape pauses the race.
                if keys.held(ESCAPE) {
                    self.pause(sound, keys, rand, first);
                    return Outcome::Racing;
                }
                if first {
                    self.start_intro(sound, false);
                    return Outcome::Racing;
                }
            }
            Stage::Intro { intro, ending } => {
                let going = intro.wait();
                sound.set_mask(intro.volume() >> 8);
                self.screen.copy_from_slice(intro.screen());
                self.shown = intro.palette().clone();
                if going {
                    return Outcome::Racing;
                }
                if *ending {
                    // 0x41795D: the race abandoned before the intro leaves the loop after it.
                    self.start_outro(sound, Outcome::Aborted);
                    return Outcome::Racing;
                }
                self.clock.restart();
            }
            Stage::Pause {
                pause,
                first,
                ending,
            } => {
                let (first, ending) = (*first, *ending);
                let mut asked = Vec::new();
                let step = pause.wait(|code| keys.held(code), rand, &mut asked);
                self.screen.copy_from_slice(pause.screen());
                Self::pause_sounds(sound, &asked);
                let answer = match step {
                    pause::Step::Waiting => return Outcome::Racing,
                    pause::Step::Leaving => {
                        keys.release_all();
                        return Outcome::Racing;
                    }
                    pause::Step::Over(answer) => answer,
                };
                if ending {
                    // 0x417387: the loop's last frame, then the end.
                    self.show_frame();
                    self.stage = Stage::Ended(Outcome::Over);
                    return Outcome::Racing;
                }
                self.clock.restart();
                let after = after_pause(answer, first);
                if matches!(after, AfterPause::End | AfterPause::IntroThenEnd) {
                    // 0x4176F8: the player drops behind the cars racing, and the race ends.
                    laps::abandon(&mut self.cars, self.player);
                }
                match after {
                    AfterPause::End => {
                        self.show_buffer();
                        self.stage = Stage::Ended(Outcome::Aborted);
                        return Outcome::Racing;
                    }
                    AfterPause::IntroThenEnd => {
                        // The intro runs on the same frame, then the race ends.
                        self.start_intro(sound, true);
                        return Outcome::Racing;
                    }
                    AfterPause::Intro | AfterPause::Race => {}
                }
                // 0x41771D: the engine again; F1 left held for the race's pass, which opens the
                // help.
                self.help_asked = answer == pause::Answer::Help;
                let car = self.drivers[self.player].car as u8;
                sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
                if after == AfterPause::Intro {
                    // The pause came before the intro, which now runs on the same frame.
                    self.start_intro(sound, false);
                    return Outcome::Racing;
                }
            }
            Stage::Help { help, order, first } => {
                let waiting = help.waiting();
                let pressed = waiting && (0..=255).any(|code| keys.held(code));
                let going = help.wait(pressed);
                self.screen.copy_from_slice(help.screen());
                self.shown = help.palette().clone();
                if !waiting && help.waiting() {
                    keys.release_all();
                }
                if going {
                    return Outcome::Racing;
                }
                // 0x416CC2: the music back where it was, at full volume, and the engine; then
                // the pass the help came in is drawn.
                let (order, first) = (*order, *first);
                keys.release_all();
                sound.set_music_order(order);
                sound.set_mask(FULL_MASK);
                let car = self.drivers[self.player].car as u8;
                sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
                self.stage = Stage::Loop { first };
                self.keys_then_draw(sound, keys, rand);
                return Outcome::Racing;
            }
            Stage::Paused {
                pause,
                order,
                first,
            } => {
                let (order, first) = (*order, *first);
                let mut asked = Vec::new();
                let step = pause.wait(|code| keys.held(code), rand, &mut asked);
                self.screen.copy_from_slice(pause.screen());
                Self::pause_sounds(sound, &asked);
                let answer = match step {
                    pause::Step::Waiting => return Outcome::Racing,
                    pause::Step::Leaving => {
                        keys.release_all();
                        return Outcome::Racing;
                    }
                    pause::Step::Over(answer) => answer,
                };
                // 0x417063: Y does not abort here; the music back where it was at full volume,
                // the engine again, F1 left held for the next pass; then the pass is drawn.
                self.clock.restart();
                self.help_asked = answer == pause::Answer::Help;
                sound.set_music_order(order);
                sound.set_mask(FULL_MASK);
                let car = self.drivers[self.player].car as u8;
                sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
                self.stage = Stage::Loop { first };
                self.draw_pass(sound, keys, rand);
                return Outcome::Racing;
            }
            Stage::Ended(outcome) => {
                let outcome = *outcome;
                self.start_outro(sound, outcome);
                return Outcome::Racing;
            }
            Stage::Outro { outro, outcome } => {
                let going = outro.wait();
                if let Some(volume) = outro.volume() {
                    sound.set_mask(volume >> 8);
                }
                self.screen.copy_from_slice(outro.screen());
                self.shown = outro.palette().clone();
                if going {
                    return Outcome::Racing;
                }
                // 0x417969: the screen cleared once the view has tilted away.
                let outcome = *outcome;
                self.screen.fill(0);
                return outcome;
            }
        }
        self.stage = Stage::Loop { first: false };
        self.frame(sound, rand);
        self.keys_and_draw(sound, keys, rand);
        Outcome::Racing
    }

    /// A pass after its logic (0x416B21): F1 opens the help before the pass is drawn, else
    /// the race's other keys, then the drawing.
    fn keys_and_draw(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand) {
        if keys.held(HELP_KEY) || self.help_asked {
            self.help_asked = false;
            self.start_help(sound);
            return;
        }
        self.keys_then_draw(sound, keys, rand);
    }

    /// Whether the pass under way is the loop's first, before the intro.
    fn first_pass(&self) -> bool {
        matches!(self.stage, Stage::Loop { first: true })
    }

    /// The race's other keys before the pass is drawn (0x416D13): TAB turns the status bar
    /// away or back once a press, and a wrecked player always has it; F2 and F3 turn the music
    /// and the effects off or back to `dr.cfg`'s volumes, F4 and F5 the scene's pictures and
    /// the shadows, each once and the key let go; P pauses the game, and the pass is drawn
    /// after it.
    fn keys_then_draw(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand) {
        let wrecked = self.cars[self.player].handling.damage <= 0;
        let [music, effects] = self.session.keys(keys, wrecked);
        if let Some(on) = music {
            sound.set_music_volume(if on { self.volumes.0 } else { 0 });
        }
        if let Some(on) = effects {
            sound.set_effects_volume(if on { self.volumes.1 } else { 0 });
        }
        if keys.held(PAUSE_KEY) {
            self.game_paused(sound, keys, rand);
            return;
        }
        self.draw_pass(sound, keys, rand);
    }

    /// The pass drawn, then the race over (0x417283) its box.
    fn draw_pass(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand) {
        self.draw(sound);
        if self.over_ticks > OVER_TICKS {
            self.end_box(sound, keys, rand);
        }
    }

    /// The help (0x416B30): every channel but the last two silenced, the track's music on to
    /// a calmer order at half volume, up to the help's first wait.
    fn start_help(&mut self, sound: &mut Sound) {
        for channel in 1..=CHANNELS {
            sound.stop_channel(channel);
        }
        let order = sound.music_order();
        if let Some(&calm) = HELP_ORDERS.get(self.number) {
            sound.set_music_order(calm);
        }
        sound.set_mask(HELP_MASK);
        let help = help::Help::new(
            (&self.screen, &self.palette),
            &self.help_pages,
            &self.hud.small_font,
            &self.help_texts,
            (&self.controls, &self.pads),
        );
        self.stage = Stage::Help {
            help: Box::new(help),
            order,
            first: self.first_pass(),
        };
    }

    /// The view tilting away (`sub_4055A0`, 0x417963) from the loop's last frame in the
    /// buffer and the race's palette (0x4A9BA0, the intro's too), over the screen as shown, up
    /// to its first wait; or, the status bar not all in (TAB), the frame spinning away.
    fn start_outro(&mut self, sound: &mut Sound, outcome: Outcome) {
        let frame: Vec<u8> = (0..VIEW_HEIGHT)
            .flat_map(|y| (0..VIEW_WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| self.buffer.pixel(x, y))
            .collect();
        let shown = (self.screen.as_slice(), &self.shown);
        let left = i64::from(self.left());
        let outro = outro::Outro::new(&self.palette, shown, &frame, left);
        if let Some(volume) = outro.volume() {
            sound.set_mask(volume >> 8);
        }
        self.screen.copy_from_slice(outro.screen());
        self.shown = outro.palette().clone();
        self.stage = Stage::Outro {
            outro: Box::new(outro),
            outcome,
        };
    }

    /// The race over (0x4172A7): every channel but the last two silenced, the end's call, and
    /// the box saying so, up to its first wait.
    fn end_box(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand) {
        for channel in 1..=CHANNELS {
            sound.stop_channel(channel);
        }
        sound.trigger_at(CALL_CHANNEL, END_CALL, FULL, CALL_PITCH);
        let lines = self.race_over_lines.clone();
        let (pause, asked) = self.open_box(&lines, keys, rand);
        self.stage = Stage::Pause {
            pause,
            first: false,
            ending: true,
        };
        Self::pause_sounds(sound, &asked);
    }

    /// The buffer's track view (200 rows of 256) and HUD (200 rows of 64).
    fn view_and_hud(&self) -> (Vec<u8>, Vec<u8>) {
        let mut view = Vec::with_capacity(VIEW_HEIGHT * TRACK_VIEW_WIDTH as usize);
        let mut hud = Vec::with_capacity(VIEW_HEIGHT * HUD_WIDTH as usize);
        for y in 0..VIEW_HEIGHT {
            let columns = HUD_WIDTH as usize..VIEW_WIDTH;
            view.extend(columns.map(|x| self.buffer.pixel(x, y)));
            hud.extend((0..HUD_WIDTH as usize).map(|x| self.buffer.pixel(x, y)));
        }
        (view, hud)
    }

    /// The intro (0x41787C), over the loop's first frame, up to its first wait; `ending` when
    /// the race was abandoned before it.
    fn start_intro(&mut self, sound: &mut Sound, ending: bool) {
        let (view, hud) = self.view_and_hud();
        let intro = intro::Intro::new(&self.palette, self.player, &view, &hud);
        sound.set_mask(intro.volume() >> 8);
        self.show_intro(&intro);
        self.stage = Stage::Intro {
            intro: Box::new(intro),
            ending,
        };
    }

    /// The pause (0x417544): every channel silenced, the box with its lines, up to its first
    /// wait.
    fn pause(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand, first: bool) {
        for channel in 1..=CHANNELS {
            sound.stop_channel(channel);
        }
        let lines = self.pause_lines.clone();
        let (pause, asked) = self.open_box(&lines, keys, rand);
        self.stage = Stage::Pause {
            pause,
            first,
            ending: false,
        };
        Self::pause_sounds(sound, &asked);
    }

    /// The game paused (P, 0x416E24): every channel silenced, the track's music on to the
    /// help's calmer order at half volume, and the box saying so, up to its first wait.
    fn game_paused(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand) {
        for channel in 1..=CHANNELS {
            sound.stop_channel(channel);
        }
        let order = sound.music_order();
        if let Some(&calm) = HELP_ORDERS.get(self.number) {
            sound.set_music_order(calm);
        }
        sound.set_mask(HELP_MASK);
        let lines = self.paused_lines.clone();
        let first = self.first_pass();
        let (pause, asked) = self.open_box(&lines, keys, rand);
        self.stage = Stage::Paused {
            pause,
            order,
            first,
        };
        Self::pause_sounds(sound, &asked);
    }

    /// `racePauseMenu` (0x4064A0) with the box's nine `lines`, over the screen as shown; and
    /// the sounds it asks for at its start.
    fn open_box(
        &mut self,
        lines: &[Vec<u8>],
        keys: &mut Keys,
        rand: &mut Rand,
    ) -> (Box<pause::Pause>, Vec<pause::Sound>) {
        let mut picture = self.pause_box.clone();
        picture.resize(204 * 76, 0);
        for (line, text) in lines.iter().enumerate() {
            for (column, &c) in text.iter().enumerate() {
                let start = 36 * usize::from(c.saturating_sub(32));
                let glyph = self.hud.small_font.get(start..start + 36).unwrap_or(&[]);
                let at = 6 * (272 * line + column + 205);
                for (row, pixels) in glyph.chunks(6).enumerate() {
                    for (x, &pixel) in pixels.iter().enumerate() {
                        if pixel != 0
                            && let Some(slot) = picture.get_mut(at + row * 204 + x)
                        {
                            *slot = pixel;
                        }
                    }
                }
            }
        }
        keys.release_all();
        // 0x406580: the box flies in over the buffer's view as the pass left it, which at the
        // race's end is the last frame drawn and never shown.
        let mut frame = vec![0; VIEW_WIDTH * VIEW_HEIGHT];
        for (y, row) in frame.chunks_mut(VIEW_WIDTH).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = self.buffer.pixel(x, y);
            }
        }
        let (pause, asked) = pause::Pause::new(&frame, picture, self.left(), rand);
        self.screen.copy_from_slice(pause.screen());
        (Box::new(pause), asked)
    }

    fn pause_sounds(sound: &mut Sound, asked: &[pause::Sound]) {
        for &ask in asked {
            match ask {
                pause::Sound::Play(effect) => {
                    sound.trigger_at(PAUSE_CHANNEL, effect, FULL, PAUSE_PITCH);
                }
                pause::Sound::Stop => sound.stop_channel(PAUSE_CHANNEL),
            }
        }
    }

    /// How the race ended (`sub_402240` at its end): each car's place, its damage in percent
    /// (100 less the bar's 1024ths rounded up) and the money power-ups it picked up; whether
    /// the player was lapped and took the bonus power-up; the laps, the race's time and the
    /// best lap.
    pub(crate) fn outcome(&self) -> crate::books::Outcome {
        crate::books::Outcome {
            finishes: self
                .cars
                .iter()
                .map(|car| crate::books::Finish {
                    place: car.place,
                    damage: ftol(100.0 - (f64::from(car.handling.damage) * 0.0009765625).ceil()),
                    money: car.handling.money,
                })
                .collect(),
            lapped: self.laps_state.lapped,
            bonus: self.bonus,
            laps: self.laps,
            race_time: laps::time(self.laps_state.race_clock),
            best_lap: self.laps_state.best,
        }
    }

    /// The race's state for comparing with the original's memory (`scripts/reference-watch.py`):
    /// the frame, the rocket flames' picture (`fp`, 0x456AFC), the ticks between the last two
    /// frames (`bt`, 0x4A9EA4) and before the next power-up (`pw`, 0x456AC4), `rand()`'s state
    /// (`rs`), then for each car its numbers in the original's layout, floats as their bits.
    pub(crate) fn trace(&self, rand: &Rand) -> String {
        let mut line = format!(
            "{} fp{} bt{} pw{} rs{}",
            self.clock.frame,
            self.session.flame_phase,
            self.clock.between,
            self.power_ups.wait(),
            rand.state()
        );
        for car in &self.cars {
            let h = &car.handling;
            line += &format!(
                " | z{} d{} s{} w{} k{},{} t{:08x} a{:08x} v{:08x} x{:08x} y{:08x} sl{:08x} \
                 g{:08x} px{:08x} py{:08x} sp{:08x} l{} p{} f{} dx{:08x} dy{:08x} st{} kn{} \
                 e{:08x} dm{} tb{} mc{} hn{} mn{} fi{} at{} bo{} av{} mw{} ho{} ef{} ag{}",
                car.zone,
                car.direction,
                car.sprite,
                car.wall,
                car.knocks[0],
                car.knocks[1],
                car.turn.to_bits(),
                car.angle.to_bits(),
                car.speed.to_bits(),
                car.x.to_bits(),
                car.y.to_bits(),
                car.slide.to_bits(),
                car.grip.to_bits(),
                car.push[0].to_bits(),
                car.push[1].to_bits(),
                car.spin.to_bits(),
                car.lap,
                car.place,
                i32::from(car.finished),
                car.step[0].to_bits(),
                car.step[1].to_bits(),
                car.stuck,
                car.knocked,
                h.engine.to_bits(),
                h.damage,
                h.turbo,
                car.mine_cooldown,
                i32::from(car.horn),
                h.mines,
                car.fire,
                car.ai.turning,
                car.ai.back_off,
                car.ai.avoid,
                car.ai.mine_wait,
                i32::from(car.ai.horn),
                car.effect,
                car.gunfire.active,
            );
        }
        line
    }

    /// The palette as shown.
    pub(crate) fn shown(&self) -> &Palette {
        &self.shown
    }

    /// The pass's frame onto the screen after the wait (0x4173AD): wavering while the
    /// player's effect power-up lasts, else as it is, the player's count then cleared; then
    /// every car's count run down by the ticks between the last two frames (0x41746C).
    fn show_frame(&mut self) {
        if self.cars[self.player].effect > 0 {
            let left = self.left();
            waver::show(
                &mut self.screen,
                &self.buffer,
                &self.waves,
                &mut self.session.waves,
                left,
            );
        } else {
            self.show_buffer();
            self.cars[self.player].effect = 0;
        }
        for car in &mut self.cars {
            if car.effect > 0 {
                car.effect -= self.clock.between;
            }
        }
        self.view_width = slide(self.view_width, self.session.status_bar, self.clock.between);
    }

    /// What the race leaves in the original's globals for the next.
    pub(crate) fn session(&self) -> Session {
        self.session
    }

    /// The frame in the buffer copied onto the screen (0x4173E0).
    fn show_buffer(&mut self) {
        for y in 0..VIEW_HEIGHT {
            for x in 0..VIEW_WIDTH {
                self.screen[y * VIEW_WIDTH + x] = self.buffer.pixel(x, y);
            }
        }
    }

    fn show_intro(&mut self, intro: &intro::Intro) {
        self.screen.copy_from_slice(intro.screen());
        self.shown = intro.palette().clone();
    }

    /// `recalculateCircuitImageOffset` (0x40D560), once a frame: the view centred on the
    /// player plus its lead, kept inside the track.
    fn move_view(&mut self) {
        let car = &self.cars[self.player];
        let r = (f64::from(car.angle) + 180.0) * RADIANS;
        let speed = f64::from(car.speed);
        let target = [
            ftol(sin(r) * speed * 16.0),
            ftol(cos(r) * speed * 10.666_666_666_666_666),
        ];
        let lead = &mut self.lead;
        let steps = if target == lead.target { lead.steps } else { 5 };
        lead.target = target;
        if steps > 0 {
            for (at, target) in lead.at.iter_mut().zip(target) {
                *at += (target - *at) / steps;
            }
            lead.steps = steps - 1;
        }
        let info = &self.track.info;
        let max_x = info.width as i32 - self.view_width;
        let max_y = info.height as i32 - VIEW_HEIGHT as i32;
        let x = (ftol(f64::from(car.x)) - (self.view_width >> 1) + lead.at[0])
            .min(max_x)
            .max(0);
        let y = (ftol(f64::from(car.y)) - HALF_HEIGHT + lead.at[1])
            .min(max_y)
            .max(0);
        self.view = (x as usize, y as usize);
    }

    /// The view's corner on the track for this frame.
    fn camera(&self) -> (usize, usize) {
        self.view
    }

    /// The HUD's width (0x456AA0): the track's view starts this far from the screen's left.
    fn left(&self) -> i32 {
        VIEW_WIDTH as i32 - self.view_width
    }

    /// The HUD's drivers: the player, then the others in their places.
    /// Their medals roll on a frame's worth.
    fn boards(&mut self) -> Vec<hud::Board> {
        let order = self.board_order();
        let places: Vec<i32> = order.iter().map(|&slot| self.cars[slot].place).collect();
        let medals = self.medals.roll(&places, self.clock.between);
        order
            .into_iter()
            .zip(medals)
            .map(|(slot, medal)| hud::Board {
                name: self.drivers[slot].name.clone(),
                lap: self.cars[slot].lap,
                medal,
                damage_bar: self.cars[slot].handling.damage,
                finished: self.cars[slot].finished,
            })
            .collect()
    }

    /// The HUD's boards' cars: the player, then the others by their place on the grid.
    fn board_order(&self) -> Vec<usize> {
        let mut order = vec![self.player];
        order.extend((0..self.cars.len()).filter(|&slot| slot != self.player));
        order
    }

    /// A frame: the track under the camera copied right of the HUD (0x4170C1), each row's
    /// `(width >> 2) + 1` dwords; what is on the track; the HUD.
    fn draw(&mut self, sound: &mut Sound) {
        self.move_view();
        let (x, y) = self.camera();
        let width = self.track.info.width as usize;
        let image = &self.track.image.pixels;
        let left = self.left();
        let copied = (((self.view_width >> 2) + 1) * 4) as usize;
        for row in 0..VIEW_HEIGHT {
            let from = (y + row) * width + x;
            let end = (from + copied).min(image.len());
            let at = (row * STRIDE + LEFT) as i64 + i64::from(left);
            self.buffer.copy(at, &image[from..end]);
        }
        let car = (self.cars[self.player].x, self.cars[self.player].y);
        let view = (x as i32, y as i32);
        let now = self.clock.timer;
        self.pedestrians
            .draw(&mut self.buffer, now, car, view, left);
        for car in &mut self.cars {
            marks::draw_puffs(
                &mut self.buffer,
                car,
                &self.smoke,
                view,
                left,
                self.clock.between,
            );
        }
        self.draw_cars();
        self.mines.draw(&mut self.buffer, view, left, now);
        if self.session.shadows {
            self.draw_shadows();
        }
        for car in &mut self.cars {
            guns::draw_flash(&mut self.buffer, car, &self.flashes, view, left);
        }
        for car in &mut self.cars {
            cars::draw_flame(
                &mut self.buffer,
                car,
                &self.rocket_flames,
                &mut self.session.flame_phase,
                now,
            );
        }
        let (x, y) = self.camera();
        let camera = (x as i32, y as i32);
        let cull = self.number != 0;
        scene::draw(
            &mut self.buffer,
            &self.track.scene,
            &self.scene,
            camera,
            cull,
            (left, self.view_width),
            self.session.pictures,
        );
        let camera = (self.view.0 as i32, self.view.1 as i32);
        for car in &mut self.cars {
            guns::draw_sparks(&mut self.buffer, car, &self.sparks, camera, left);
        }
        self.power_ups.draw_notes(
            &mut self.buffer,
            &self.hud.small_font,
            self.pickup_money,
            camera,
            left,
            self.clock.between,
        );
        if self.clock.frame < 290 {
            let event = self
                .semaphore
                .draw(&mut self.buffer, self.clock.frame, self.clock.between);
            let (channel, (effect, pitch)) = match event {
                Some(semaphore::Event::Ready) => (LIGHTS_CHANNEL, READY_SOUND),
                Some(semaphore::Event::Set) => (START_CHANNEL, SET_SOUND),
                Some(semaphore::Event::Go) => {
                    // 0x415079: the clocks and the player's samples from the start.
                    self.clock.restart();
                    self.sampled = 0;
                    self.started = true;
                    (START_CHANNEL, GO_SOUND)
                }
                None => (0, (0, 0)),
            };
            if channel != 0 {
                sound.trigger_at(channel, effect, FULL, pitch);
            }
        }
        self.clock.between = self.clock.ticks - self.clock.seen;
        self.clock.seen = self.clock.ticks;
        // 0x41425D: the race's and the lap's clocks run from the start to the finish.
        if self.clock.frame > START_FRAME && !self.cars[self.player].finished {
            self.laps_state.race_clock += self.clock.between;
            self.laps_state.lap_clock += self.clock.between;
        }
        self.draw_hud(sound);
        // 0x417240: the flag for each car in first place once the race is over for the cars.
        if self.laps_state.over {
            for _ in self.cars.iter().filter(|car| car.place == 1) {
                self.flag.draw(&mut self.buffer, self.clock.between);
            }
        }
    }

    /// The HUD (0x414220) or, with the status bar away, the small board alone (0x414110).
    fn draw_hud(&mut self, sound: &mut Sound) {
        let left = self.left();
        if self.view_width == VIEW_WIDTH as i32 {
            // 0x414110: the status bar away, the last lap's time counted down a first time,
            // and only the small board.
            self.laps_state.count_down(self.clock.between);
            self.draw_small_board(sound);
            return;
        }
        // 0x41430A: the status bar sliding, the small board under it.
        if left < HUD_WIDTH as i32 {
            self.draw_small_board(sound);
        }
        let gauge = self.gauge();
        let boards = self.boards();
        hud::draw(
            &mut self.buffer,
            &self.hud,
            i64::from(left),
            &boards,
            &gauge,
            self.laps,
        );
        self.damage_calls(sound);
        if !self.weapons {
            self.laps_state.count_down(self.clock.between);
        }
    }

    /// The calls on the player's damage after the HUD's or the small board's damage
    /// (0x414E28, 0x414028), on the race's calls' channel.
    fn damage_calls(&mut self, sound: &mut Sound) {
        let damage = self.cars[self.player].handling.damage;
        for effect in self.damage_calls.check(damage, self.player_tough) {
            sound.trigger_at(CALL_CHANNEL, effect, FULL, CALL_PITCH);
        }
    }

    /// What the HUD shows of the player's car.
    fn gauge(&self) -> hud::Player {
        let player = &self.cars[self.player];
        hud::Player {
            speed: player.speed,
            engine: player.handling.engine,
            weapons: self.weapons,
            weapons_bar: player.handling.weapons_bar,
            turbo_bar: player.handling.turbo,
            mines: player.handling.mines,
            time: self.laps_state.time_shown(),
        }
    }

    /// The small board (0x413C90), the last lap's time counted down as it shows it, and the
    /// calls on the player's damage.
    fn draw_small_board(&mut self, sound: &mut Sound) {
        let gauge = self.gauge();
        let damage = self.cars[self.player].handling.damage;
        hud::draw_small(&mut self.buffer, &self.hud, &gauge, damage);
        self.damage_calls(sound);
        if !self.weapons {
            self.laps_state.count_down(self.clock.between);
        }
    }

    /// Where the player's car is on the screen (0x40D929): where the view's lead puts it,
    /// unless the view has stopped at the track's edge.
    fn player_on_screen(&self) -> (i32, i32) {
        let car = &self.cars[self.player];
        let info = &self.track.info;
        let axis = |position: f32, lead: i32, half: i32, size: i32, view: i32| {
            let at = f64::from(lead) + f64::from(position);
            if at < f64::from(half) {
                ftol(f64::from(position))
            } else if at > f64::from(size - half) {
                ftol(f64::from(position) - f64::from(size - view))
            } else {
                half - lead
            }
        };
        let width = info.width as i32;
        let height = info.height as i32;
        let view_width = self.view_width;
        (
            axis(car.x, self.lead.at[0], view_width >> 1, width, view_width) + self.left(),
            axis(
                car.y,
                self.lead.at[1],
                HALF_HEIGHT,
                height,
                VIEW_HEIGHT as i32,
            ),
        )
    }

    /// `drawCarInRace` (0x40D920): every car's headlights, the player's first, then the
    /// player's sprite and the others' over it, those far off the view left out.
    fn draw_cars(&mut self) {
        let (camera_x, camera_y) = self.camera();
        let left = self.left();
        let on_screen: Vec<(i32, i32)> = (0..self.cars.len())
            .map(|slot| {
                if slot == self.player {
                    return self.player_on_screen();
                }
                let car = &self.cars[slot];
                (
                    ((f64::from(car.x) - camera_x as f64) + f64::from(left)) as i32,
                    (f64::from(car.y) - camera_y as f64) as i32,
                )
            })
            .collect();
        for (car, &(x, y)) in self.cars.iter_mut().zip(&on_screen) {
            car.screen = [x, y];
        }
        let others = (0..self.cars.len()).filter(|&slot| slot != self.player);
        let lit = &self.track.lit;
        let player = &self.cars[self.player];
        if self::lit(player) {
            cars::headlights(&mut self.buffer, on_screen[self.player], player.angle, lit);
        }
        for slot in others.clone() {
            let (x, y) = on_screen[slot];
            let near = x > left - 40 && x < 360 && y > -40 && y < VIEW_HEIGHT as i32 + 40;
            if near && self::lit(&self.cars[slot]) {
                cars::headlights(&mut self.buffer, (x, y), self.cars[slot].angle, lit);
            }
        }
        cars::draw_sprite(
            &mut self.buffer,
            &self.sprites,
            on_screen[self.player],
            player.sprite,
        );
        // A wreck burns over its sprite: the player's wherever it is, another's near the view.
        let now = self.clock.timer;
        if player.handling.damage <= 0 {
            cars::draw_fire(
                &mut self.buffer,
                &mut self.cars[self.player],
                &self.fire,
                now,
            );
        }
        for slot in others {
            let (x, y) = on_screen[slot];
            if x > left - 20 && x < 340 && y > -20 && y < VIEW_HEIGHT as i32 + 20 {
                cars::draw_sprite(
                    &mut self.buffer,
                    &self.sprites,
                    (x, y),
                    self.cars[slot].sprite,
                );
            }
            let near = x > left - 8 && x < 328 && y > -8 && y < VIEW_HEIGHT as i32 + 8;
            if near && self.cars[slot].handling.damage <= 0 {
                cars::draw_fire(&mut self.buffer, &mut self.cars[slot], &self.fire, now);
            }
        }
    }

    /// `drawShadows` (0x40D7B0): the track's shadow triangles near the view, every colour
    /// under them turned through `VARJO.TAB`, which darkens only the cars' colours: the track's
    /// own picture has its shadows drawn in.
    fn draw_shadows(&mut self) {
        let (camera_x, camera_y) = self.camera();
        let shadows = &self.track.shadows;
        for corners in &shadows.triangles {
            let points = corners.map(|corner| {
                let (x, y) = shadows.points[corner];
                (x - camera_x as i32, y - camera_y as i32)
            });
            if shadow_in_view(points, self.view_width >> 1) {
                let on_screen = points.map(|(x, y)| (x + self.left(), y));
                raster::light_triangle(&mut self.buffer, on_screen, &self.shade);
            }
        }
    }

    /// The screen doubled into the 640x480 window from row 40.
    pub(crate) fn present(&self, window: &mut [u8]) {
        window.fill(0);
        for row in 0..VIEW_HEIGHT {
            for column in 0..VIEW_WIDTH {
                let pixel = self.screen[row * VIEW_WIDTH + column];
                for dy in 0..2 {
                    let at = (WINDOW_TOP + 2 * row + dy) * 2 * VIEW_WIDTH + 2 * column;
                    window[at] = pixel;
                    window[at + 1] = pixel;
                }
            }
        }
    }
}

/// The track's view's width a frame `between` ticks on (0x417497): with the status bar wanted
/// back it narrows 4 a tick to 256, else it widens 2 a tick to 320.
fn slide(width: i32, status_bar: bool, between: i32) -> i32 {
    if status_bar {
        let width = if width > TRACK_VIEW_WIDTH {
            width - SLIDE_IN * between
        } else {
            width
        };
        width.max(TRACK_VIEW_WIDTH)
    } else {
        let width = if width < VIEW_WIDTH as i32 {
            width + SLIDE_OUT * between
        } else {
            width
        };
        width.min(VIEW_WIDTH as i32)
    }
}

/// Degrees to radians as the original has it (0x4412B0).
const RADIANS: f64 = 0.017_453_292_519_944_444;

/// `sub_4138A0`: the race's keys as the timer samples them each tick, from the eight
/// controls' scancodes in `dr.cfg` (accelerate, brake, left, right, turbo, gun, mine and horn)
/// and the arrows, which always drive; the arrows count for the controls set to their
/// extended codes. The mine control's key is let go once seen; the horn holds the brake and
/// the mine bits together. Then, with the gamepad on, the seven controls' gamepad inputs
/// (`pads`, no horn among them).
fn sample(keys: &mut Keys, controls: &[u32; 8], pads: &[u32; 7]) -> u32 {
    let held = |keys: &Keys, code: u32| u8::try_from(code).is_ok_and(|code| keys.held(code));
    let mut bits = 0;
    let arrows = [
        (0xC8, 0x48, 1),
        (0xD0, 0x50, 2),
        (0xCB, 0x4B, 4),
        (0xCD, 0x4D, 8),
    ];
    for (&control, (extended, arrow, bit)) in controls.iter().zip(arrows) {
        if control == extended && keys.held(arrow) {
            bits |= bit;
        }
    }
    for (code, bit) in [(0xC8, 1), (0x48, 1), (controls[0], 1)] {
        if held(keys, code) {
            bits |= bit;
        }
    }
    for (code, bit) in [(0xD0, 2), (0x50, 2), (controls[1], 2)] {
        if held(keys, code) {
            bits |= bit;
        }
    }
    for (control, bit) in [(2, 4), (3, 8), (4, 0x10), (5, 0x20)] {
        if held(keys, controls[control]) {
            bits |= bit;
        }
    }
    if held(keys, controls[6]) {
        bits |= driving::MINE;
        if let Ok(code) = u8::try_from(controls[6]) {
            keys.release(code);
        }
    }
    if bits & driving::BRAKE != 0 && bits & driving::MINE != 0 {
        bits &= !driving::BRAKE;
    }
    if held(keys, controls[7]) {
        bits |= driving::MINE | driving::BRAKE;
    }
    // 0x413A3E: the gamepad, after the mine key has taken the brake away; the pad's mine is
    // seen every tick it is held (its letting the mine key go again changes nothing: the key
    // was let go above when held).
    let pad_bits = [
        driving::ACCELERATE,
        driving::BRAKE,
        driving::LEFT,
        driving::RIGHT,
        driving::TURBO,
        guns::GUN,
        driving::MINE,
    ];
    for (&input, bit) in pads.iter().zip(pad_bits) {
        if keys.pad_held(input) {
            bits |= bit;
        }
    }
    if bits & driving::TURBO != 0 {
        bits |= driving::ACCELERATE;
    }
    bits
}

/// The horn's keys (the brake and the mine), the channel of car 0's horn (the others' follow
/// it), and its sound for the first two cars (the others' is the next).
const HORN: u32 = driving::BRAKE | driving::MINE;
const HORN_CHANNEL: usize = 11;
const HORN_SOUND: u8 = 33;

/// What the horns ask of the sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Horn {
    Start {
        channel: usize,
        effect: u8,
        volume: u32,
        pitch: u32,
    },
    Stop(usize),
}

/// `sub_413500`, once a pass from the start: a car holding the brake and the mine key
/// together in a tick of the pass sounds its horn, and those keys do nothing else; it starts
/// once while held, the player's at full volume and another's 88 less a pixel away, and stops
/// when let go or the car has finished.
fn horns(cars: &mut [Car], steps: usize, player: usize) -> Vec<Horn> {
    let mut asked = Vec::new();
    for slot in 0..cars.len() {
        let car = &mut cars[slot];
        let mut pressed = false;
        for keys in &mut car.keys[..steps] {
            if *keys & HORN == HORN {
                *keys &= 0xBD;
                pressed = true;
            }
        }
        let channel = HORN_CHANNEL + slot;
        if pressed && !car.finished {
            if car.horn {
                continue;
            }
            car.horn = true;
            let effect = HORN_SOUND + u8::from(car.handling.car > 1);
            let volume = if slot == player {
                FULL as i32
            } else {
                0x9500 - 88 * cars[slot].distance(&cars[player])
            };
            if volume > 0 {
                asked.push(Horn::Start {
                    channel,
                    effect,
                    volume: volume as u32,
                    pitch: (slot as u32 + 0x21) << 12,
                });
            }
        } else if car.horn {
            car.horn = false;
            asked.push(Horn::Stop(channel));
        }
    }
    asked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputEvent, Key};

    /// The defaults `dr.cfg` gets: A, Z, the arrows' extended codes, left shift, left
    /// control, left alt and space.
    const DEFAULT_CONTROLS: [u32; 8] = [0x1E, 0x2C, 0xCB, 0xCD, 0x2A, 0x1D, 0x38, 0x39];
    /// No gamepad input for any control.
    const NO_PADS: [u32; 7] = [0; 7];

    fn holding(held: &[Key]) -> Keys {
        let mut keys = Keys::default();
        for &key in held {
            keys.event(InputEvent::Key { key, pressed: true });
        }
        keys
    }

    /// The arrows drive whatever `dr.cfg` sets, and the turbo key also accelerates: a player
    /// on the default keys steers with the arrows and boosts with shift alone.
    #[test]
    fn the_arrows_always_drive_and_the_turbo_accelerates() {
        let mut keys = holding(&[Key::Up, Key::Left]);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &NO_PADS),
            driving::ACCELERATE | driving::LEFT
        );
        let mut keys = holding(&[Key::Down, Key::Right, Key::LeftShift]);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &NO_PADS),
            driving::BRAKE | driving::RIGHT | driving::TURBO | driving::ACCELERATE
        );
    }

    /// The mine control is seen once a press and cancels the brake held with it; the horn
    /// holds both bits, which `horns` takes for the horn.
    #[test]
    fn a_mine_key_counts_once_and_takes_the_brake() {
        let mut keys = holding(&[Key::Z, Key::LeftAlt]);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &NO_PADS),
            driving::MINE
        );
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &NO_PADS),
            driving::BRAKE
        );
        let mut keys = holding(&[Key::Space]);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &NO_PADS),
            driving::MINE | driving::BRAKE
        );
    }

    /// Gamepad inputs for the seven controls: the stick up, down, left and right, buttons 1
    /// to 3 (Define Gamepad's numbers).
    const PADS: [u32; 7] = [3, 4, 1, 2, 5, 6, 7];

    fn pad(keys: &mut Keys, events: &[InputEvent]) {
        for &event in events {
            keys.event(event);
        }
    }

    /// With the gamepad switched on in Configure, each control also answers to its gamepad
    /// input in `dr.cfg`; switched off, a pad in the player's hands does nothing in a race.
    #[test]
    fn the_gamepad_drives_through_the_inputs_dr_cfg_gives_the_controls() {
        use crate::input::{PadAxis, PadButton};
        let mut keys = Keys::default();
        let events = [
            InputEvent::PadConnected { connected: true },
            InputEvent::PadAxis {
                axis: PadAxis::StickX,
                value: -20_000,
            },
            InputEvent::PadButton {
                button: PadButton::B,
                pressed: true,
            },
        ];
        pad(&mut keys, &events);
        assert_eq!(sample(&mut keys, &DEFAULT_CONTROLS, &PADS), 0);
        keys.set_pad_on(true);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &PADS),
            driving::LEFT | guns::GUN
        );
        pad(
            &mut keys,
            &[InputEvent::PadButton {
                button: PadButton::A,
                pressed: true,
            }],
        );
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS, &PADS),
            driving::LEFT | guns::GUN | driving::TURBO | driving::ACCELERATE
        );
    }

    /// The gamepad is read after the keyboard's mine has taken the brake away: unlike the
    /// mine key, the pad's mine keeps a brake held with it (the horn's two bits), and counts
    /// every tick it is held rather than once a press.
    #[test]
    fn the_gamepads_mine_counts_every_tick_and_keeps_the_brake() {
        use crate::input::PadButton;
        let mut keys = holding(&[Key::Z]);
        keys.set_pad_on(true);
        let press = |pressed| InputEvent::PadButton {
            button: PadButton::X,
            pressed,
        };
        pad(
            &mut keys,
            &[InputEvent::PadConnected { connected: true }, press(true)],
        );
        let both = driving::BRAKE | driving::MINE;
        assert_eq!(sample(&mut keys, &DEFAULT_CONTROLS, &PADS), both);
        assert_eq!(sample(&mut keys, &DEFAULT_CONTROLS, &PADS), both);
        pad(&mut keys, &[press(false)]);
        assert_eq!(sample(&mut keys, &DEFAULT_CONTROLS, &PADS), driving::BRAKE);
    }

    fn horn_car(slot: usize, model: usize, x: f32) -> Car {
        let handling = driving::Handling {
            car: model,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: 0x1_0000,
            armour: 400,
            rocket: 0,
            weapons_bar: 102_400,
            turbo: 102_400,
            rocket_used: false,
            mines: 3,
            money: 0,
            weapons: true,
            guns: Default::default(),
        };
        Car::new((x, 100.0, 0), slot, handling, 0)
    }

    /// Brake and mine held together are the horn: it starts on the car's own channel at its
    /// own pitch, the player's at full volume and another's fainter with distance, and the
    /// two keys do nothing else in the pass; it sounds once while held, stops when let go,
    /// and a finished car's horn stops.
    #[test]
    fn brake_and_mine_together_sound_the_horn_until_let_go() {
        let both = driving::BRAKE | driving::MINE;
        let mut cars = vec![horn_car(0, 1, 100.0), horn_car(1, 4, 200.0)];
        cars[0].keys[1] = both | driving::ACCELERATE;
        cars[1].keys[0] = both;
        let asked = horns(&mut cars, 2, 0);
        assert_eq!(
            asked,
            vec![
                Horn::Start {
                    channel: 11,
                    effect: 33,
                    volume: 0x1_0000,
                    pitch: 0x2_1000
                },
                Horn::Start {
                    channel: 12,
                    effect: 34,
                    volume: 0x9500 - 88 * 100,
                    pitch: 0x2_2000
                },
            ]
        );
        assert_eq!(cars[0].keys[1], driving::ACCELERATE);
        assert_eq!(cars[1].keys[0], 0);
        cars[0].keys[0] = both;
        cars[1].finished = true;
        cars[1].keys[0] = both;
        assert_eq!(horns(&mut cars, 1, 0), vec![Horn::Stop(12)]);
        cars[0].keys[0] = 0;
        assert_eq!(horns(&mut cars, 1, 0), vec![Horn::Stop(11)]);
        assert!(horns(&mut cars, 1, 0).is_empty());
    }

    /// TAB turns the status bar away or back once a press however long it is held, and a
    /// wrecked player always gets it back; F2 to F5 turn their switch once and let the key go,
    /// so a key held on does not flicker it. A wrong count would leave the player without
    /// the HUD, or with the music off, after one press.
    #[test]
    fn the_switch_keys_turn_once_a_press() {
        let mut session = Session::default();
        let mut keys = holding(&[Key::Tab]);
        assert_eq!(session.keys(&mut keys, false), [None, None]);
        assert!(!session.status_bar, "TAB took the status bar away");
        session.keys(&mut keys, false);
        assert!(!session.status_bar, "TAB still held");
        keys.event(InputEvent::Key {
            key: Key::Tab,
            pressed: false,
        });
        session.keys(&mut keys, false);
        keys.event(InputEvent::Key {
            key: Key::Tab,
            pressed: true,
        });
        session.keys(&mut keys, false);
        assert!(session.status_bar, "TAB pressed again");
        keys.event(InputEvent::Key {
            key: Key::Tab,
            pressed: false,
        });
        session.status_bar = false;
        session.keys(&mut keys, true);
        assert!(session.status_bar, "a wreck has the status bar");
        let mut keys = holding(&[Key::F2, Key::F3, Key::F4, Key::F5]);
        assert_eq!(session.keys(&mut keys, false), [Some(false), Some(false)]);
        assert!(!session.pictures && !session.shadows);
        assert_eq!(
            session.keys(&mut keys, false),
            [None, None],
            "the keys were let go"
        );
        let mut keys = holding(&[Key::F2]);
        assert_eq!(session.keys(&mut keys, false), [Some(true), None]);
    }

    /// The status bar slides away 2 pixels a tick of the frame and back 4, from and to the
    /// track's view's 256 and the screen's 320.
    #[test]
    fn the_status_bar_slides_away_slowly_and_back_quickly() {
        assert_eq!(slide(256, false, 1), 258);
        assert_eq!(slide(318, false, 2), 320);
        assert_eq!(slide(320, false, 3), 320);
        assert_eq!(slide(320, true, 1), 316);
        assert_eq!(slide(258, true, 1), 256);
        assert_eq!(slide(256, true, 4), 256);
        assert_eq!(slide(300, true, 0), 300);
    }

    /// A race abandoned in a pause before its intro (Escape held while the race loads) still
    /// runs the intro, the view tilting up and the colours coming back, before the view tilts
    /// away; the player must not see the race end on a black screen.
    #[test]
    fn a_race_abandoned_before_its_intro_still_shows_the_intro() {
        use pause::Answer::{Abort, Help, Resume};
        assert_eq!(after_pause(Abort, true), AfterPause::IntroThenEnd);
        assert_eq!(after_pause(Abort, false), AfterPause::End);
        assert_eq!(after_pause(Resume, true), AfterPause::Intro);
        assert_eq!(after_pause(Help, true), AfterPause::Intro);
        assert_eq!(after_pause(Resume, false), AfterPause::Race);
    }

    /// The original checks a shadow's corners, not its area: one stretched across the whole
    /// view with its corners outside is never drawn, and cars under it stay lit.
    #[test]
    fn a_shadow_with_every_corner_off_the_view_is_left_out() {
        assert!(!shadow_in_view([(-10, -10), (300, -10), (-10, 250)], 128));
        assert!(shadow_in_view([(5, -10), (300, 300), (-10, 199)], 128));
        assert!(!shadow_in_view([(5, -10), (300, 300), (-10, 200)], 128));
    }

    fn racer(car: usize, colour: [u8; 3]) -> Driver {
        Driver {
            name: b"R".to_vec(),
            car,
            level: 3,
            engine: 0,
            tires: 0,
            armour: 0,
            damage: 0,
            rocket: 0,
            mines: 0,
            spikes: false,
            colour,
        }
    }

    /// In the Arena the Adversary's car keeps the track's own colours (0x409FCF skips its
    /// ramp), the player's second car takes the player's colour, and the two empty places'
    /// ramps take the spare colour; a ramp set for the Adversary would paint its car the
    /// player's colour.
    #[test]
    fn the_adversarys_car_keeps_the_tracks_colours() {
        let track = Palette([[7, 7, 7]; 256]);
        let mut palette = track.clone();
        let drivers = [racer(6, [40, 0, 0]), racer(1, [0, 40, 0])];
        car_ramps(&mut palette, &drivers, Some([0, 0, 40]));
        assert_eq!(&palette.0[15..25], &track.0[15..25], "the Adversary's");
        assert_eq!(palette.0[30], [0, 40, 0], "the player's own colour");
        assert_eq!(palette.0[40], [0, 0, 40], "the third place's");
        assert_eq!(palette.0[50], [0, 0, 40], "the fourth place's");
        let mut palette = track.clone();
        let drivers = [racer(5, [40, 0, 0]), racer(1, [0, 40, 0])];
        car_ramps(&mut palette, &drivers, None);
        assert_eq!(palette.0[20], [40, 0, 0], "any other car's colour");
        assert_eq!(
            &palette.0[35..55],
            &track.0[35..55],
            "no spare: the track's"
        );
    }

    /// A car's headlights go out with its wreck and once it has finished (0x40DAB9,
    /// 0x40DD5D): the winner rolling on after the line lights nothing ahead of it.
    #[test]
    fn a_finished_or_wrecked_car_has_no_headlights() {
        let mut car = horn_car(0, 1, 100.0);
        assert!(lit(&car));
        car.finished = true;
        assert!(!lit(&car), "finished");
        car.finished = false;
        car.handling.damage = 0;
        assert!(!lit(&car), "wrecked");
    }
}
