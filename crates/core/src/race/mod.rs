//! The race (spec M4): `startRace` (0x415710) from its load to its end. The race draws into
//! a buffer of rows 512 bytes apart ([`buffer`]), which the window shows doubled from row 40;
//! the track's view is 256 wide right of the HUD's 64.

mod buffer;
mod cars;
mod collisions;
mod driving;
mod hud;
mod intro;
mod laps;
mod marks;
mod pause;
mod pedestrians;
mod power_ups;
mod raster;
mod scene;
mod semaphore;

use deadrally_gamedata::image::Palette;
use deadrally_gamedata::race::{RaceArchives, RaceError, Track};
use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::sound;
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
/// of it (`raceEffectiveWidth` 256 and its half 128 from `initRaceValues` 0x409AB9,
/// `raceEffectiveHeight` 200 and its half 100).
const HUD_WIDTH: i64 = 64;
const TRACK_VIEW_WIDTH: i32 = 256;
const HALF_WIDTH: i32 = 128;
const HALF_HEIGHT: i32 = 100;
/// Each row copies 4 bytes past the view (`(width >> 2) + 1` dwords).
const ROW_COPY: usize = TRACK_VIEW_WIDTH as usize + 4;

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

/// Whether `drawShadows` draws a shadow with these corners in the view: one corner across
/// the view and one (maybe another) down it; a shadow whose corners all lie outside is left
/// out even where it would cover the view.
fn shadow_in_view(points: [(i32, i32); 3]) -> bool {
    let near = |value: i32, half: i32| (value - half).abs() < half;
    points.iter().any(|&(x, _)| near(x, HALF_WIDTH))
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
    /// The view's corner on the track for the frame being drawn (0x456ABC, 0x456AC0).
    view: (usize, usize),
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
    /// The car of the driver whose armour counts 2.2 times, who has a call when winning.
    tough: Option<usize>,
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
    Intro(Box<intro::Intro>),
    /// The pause; whether it came before the intro.
    Pause {
        pause: Box<pause::Pause>,
        first: bool,
    },
}

/// What a tick of the race came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Racing,
    /// The player answered Y to the pause's question.
    Aborted,
}

/// Escape's scancode, which pauses the race.
const ESCAPE: u8 = 0x01;
/// The channels the pause silences first (1 to 13), and the one its own sounds play on.
const CHANNELS: usize = 13;
const PAUSE_CHANNEL: usize = 5;
const PAUSE_PITCH: u32 = 0x2_8000;

impl Race {
    /// The race on circuit `circuit` (`TRn` with n = circuit % 9 + 1) over `laps` laps, the
    /// drivers in their places, the player in place `player`; `pause_lines` the pause box's.
    pub(crate) fn new(
        archives: &RaceArchives,
        (circuit, laps): (usize, i32),
        drivers: Vec<Driver>,
        (player, weapons): (usize, bool),
        (pause_lines, controls): (Vec<Vec<u8>>, [u32; 8]),
        rand: &mut Rand,
    ) -> Result<Race, RaceError> {
        let number = circuit % 9 + 1;
        // The second half's circuits run their tracks the other way round (0x432532).
        let reversed = circuit > 8;
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
                let handling = driving::Handling::new(&archives.handling, driver, slot == player);
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
        let tough = drivers.iter().position(|driver| {
            let tough = &archives.handling.tough;
            let mut name = driver.name.clone();
            name.push(0);
            name.get(..tough.len()) == Some(tough.as_slice())
        });
        let mut palette = track.palette.clone();
        for (driver, &first) in drivers.iter().zip(&RAMPS) {
            car_ramp(&mut palette, first, driver.colour);
        }
        Ok(Race {
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
            controls,
            samples: [0; 16],
            sampled: 0,
            rocket_ticks: 0,
            lead: Lead::default(),
            view: (0, 0),
            smoke: hud::decoded(&archives.engine, "SMOKE.BPK")?,
            power_ups,
            squeal: 0,
            squealing: false,
            started: false,
            laps_state: laps::Laps::default(),
            wrecks: Vec::new(),
            tough,
        })
    }

    /// The race's sound set up (0x416215: the menu's stopped, the track's music started at
    /// the configured volumes but silent until the intro raises it, the player's engine), then
    /// the race loop's first frame, up to its wait.
    pub(crate) fn begin(
        &mut self,
        sound: &mut Sound,
        (music_volume, effects_volume): (u32, u32),
        rand: &mut Rand,
    ) {
        sound.stop();
        sound.load_effects(&self.effects);
        sound.set_mask(0);
        sound.play_music(&self.music, 0, music_volume);
        sound.set_effects_volume(effects_volume);
        let car = self.drivers[self.player].car as u8;
        sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
        self.frame(sound, rand);
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
        self.power_ups.step(
            &mut self.track.image,
            self.clock.between,
            self.weapons,
            rand,
        );
        for tick in 0..steps {
            self.clock.frame += 1;
            if self.clock.frame > START_FRAME {
                self.drive(tick, rand);
            }
            self.after_tick(tick, sound, rand);
        }
        self.draw(sound);
    }

    /// The cars' step of a tick once the race is on (0x4164B6): the opponents' driving comes
    /// with M5, so they hold no keys.
    fn drive(&mut self, tick: usize, rand: &mut Rand) {
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
        self.samples[self.sampled] = sample(keys, &self.controls);
        self.sampled = (self.sampled + 1) % 16;
        self.clock.tick();
        match &mut self.stage {
            Stage::Loop { first } => {
                let first = *first;
                self.show_buffer();
                // 0x417517: Escape pauses the race.
                if keys.held(ESCAPE) {
                    self.pause(sound, keys, rand, first);
                    return Outcome::Racing;
                }
                if first {
                    self.start_intro(sound);
                    return Outcome::Racing;
                }
            }
            Stage::Intro(intro) => {
                let going = intro.wait();
                sound.set_mask(intro.volume() >> 8);
                self.screen.copy_from_slice(intro.screen());
                self.shown = intro.palette().clone();
                if going {
                    return Outcome::Racing;
                }
                self.clock.restart();
            }
            Stage::Pause { pause, first } => {
                let first = *first;
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
                self.clock.restart();
                if answer == pause::Answer::Abort {
                    return Outcome::Aborted;
                }
                // 0x41771D: the engine again; the help F1 asks for comes with the race's keys.
                let car = self.drivers[self.player].car as u8;
                sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
                if first {
                    // The pause came before the intro, which now runs on the same frame.
                    self.start_intro(sound);
                    return Outcome::Racing;
                }
            }
        }
        self.stage = Stage::Loop { first: false };
        self.frame(sound, rand);
        Outcome::Racing
    }

    /// The intro (0x41787C), over the loop's first frame, up to its first wait.
    fn start_intro(&mut self, sound: &mut Sound) {
        let mut view = Vec::with_capacity(VIEW_HEIGHT * TRACK_VIEW_WIDTH as usize);
        let mut hud = Vec::with_capacity(VIEW_HEIGHT * HUD_WIDTH as usize);
        for y in 0..VIEW_HEIGHT {
            let columns = HUD_WIDTH as usize..VIEW_WIDTH;
            view.extend(columns.map(|x| self.buffer.pixel(x, y)));
            hud.extend((0..HUD_WIDTH as usize).map(|x| self.buffer.pixel(x, y)));
        }
        let intro = intro::Intro::new(&self.palette, self.player, &view, &hud);
        sound.set_mask(intro.volume() >> 8);
        self.show_intro(&intro);
        self.stage = Stage::Intro(Box::new(intro));
    }

    /// The pause (0x417544): every channel silenced, the box with its lines, up to its first
    /// wait.
    fn pause(&mut self, sound: &mut Sound, keys: &mut Keys, rand: &mut Rand, first: bool) {
        for channel in 1..=CHANNELS {
            sound.stop_channel(channel);
        }
        let mut picture = self.pause_box.clone();
        picture.resize(204 * 76, 0);
        for (line, text) in self.pause_lines.iter().enumerate() {
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
        let (pause, asked) = pause::Pause::new(&self.screen, picture, rand);
        Self::pause_sounds(sound, &asked);
        self.screen.copy_from_slice(pause.screen());
        self.stage = Stage::Pause {
            pause: Box::new(pause),
            first,
        };
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

    /// The race's state for comparing with the original's memory (`scripts/reference-watch.py`):
    /// the frame, then for each car its numbers in the original's layout, floats as their
    /// bits.
    pub(crate) fn trace(&self) -> String {
        let mut line = format!("{}", self.clock.frame);
        for car in &self.cars {
            let h = &car.handling;
            line += &format!(
                " | z{} d{} s{} w{} k{},{} t{:08x} a{:08x} v{:08x} x{:08x} y{:08x} sl{:08x} \
                 g{:08x} px{:08x} py{:08x} sp{:08x} l{} p{} f{} dx{:08x} dy{:08x} st{} kn{} \
                 e{:08x} dm{} tb{}",
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
            );
        }
        line
    }

    /// The palette as shown.
    pub(crate) fn shown(&self) -> &Palette {
        &self.shown
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
        let max_x = info.width as i32 - TRACK_VIEW_WIDTH;
        let max_y = info.height as i32 - VIEW_HEIGHT as i32;
        let x = (ftol(f64::from(car.x)) - HALF_WIDTH + lead.at[0])
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

    /// The HUD's drivers: the player, then the others in their places.
    fn boards(&self) -> Vec<hud::Board> {
        let mut order = vec![self.player];
        order.extend((0..self.cars.len()).filter(|&slot| slot != self.player));
        order
            .into_iter()
            .map(|slot| hud::Board {
                name: self.drivers[slot].name.clone(),
                lap: self.cars[slot].lap,
                place: self.cars[slot].place,
                damage_bar: self.cars[slot].handling.damage,
                finished: self.cars[slot].finished,
            })
            .collect()
    }

    /// A frame: the track under the camera copied right of the HUD (0x4170C1), the HUD.
    fn draw(&mut self, sound: &mut Sound) {
        self.move_view();
        let (x, y) = self.camera();
        let width = self.track.info.width as usize;
        let image = &self.track.image.pixels;
        for row in 0..VIEW_HEIGHT {
            let from = (y + row) * width + x;
            let end = (from + ROW_COPY).min(image.len());
            let at = (row * STRIDE + LEFT) as i64 + HUD_WIDTH;
            self.buffer.copy(at, &image[from..end]);
        }
        let car = (self.cars[self.player].x, self.cars[self.player].y);
        let view = (x as i32, y as i32);
        let left = HUD_WIDTH as i32;
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
        self.draw_shadows();
        let (x, y) = self.camera();
        let camera = (x as i32, y as i32);
        let cull = self.number != 0;
        let left = HUD_WIDTH as i32;
        scene::draw(
            &mut self.buffer,
            &self.track.scene,
            &self.scene,
            camera,
            cull,
            left,
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
        let player = &self.cars[self.player];
        let gauge = hud::Player {
            speed: player.speed,
            engine: player.handling.engine,
            weapons: self.weapons,
            weapons_bar: hud::FULL_BAR,
            turbo_bar: player.handling.turbo,
            mines: self.drivers[self.player].mines,
        };
        let boards = self.boards();
        hud::draw(
            &mut self.buffer,
            &self.hud,
            HUD_WIDTH,
            &boards,
            &gauge,
            self.laps,
        );
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
        (
            axis(car.x, self.lead.at[0], HALF_WIDTH, width, TRACK_VIEW_WIDTH) + HUD_WIDTH as i32,
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
        let left = HUD_WIDTH as i32;
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
        let others = (0..self.cars.len()).filter(|&slot| slot != self.player);
        let lit = &self.track.lit;
        let player = &self.cars[self.player];
        if player.handling.damage > 0 {
            cars::headlights(&mut self.buffer, on_screen[self.player], player.angle, lit);
        }
        for slot in others.clone() {
            let (x, y) = on_screen[slot];
            let near = x > left - 40 && x < 360 && y > -40 && y < VIEW_HEIGHT as i32 + 40;
            if near && self.cars[slot].handling.damage > 0 {
                cars::headlights(&mut self.buffer, (x, y), self.cars[slot].angle, lit);
            }
        }
        cars::draw_sprite(
            &mut self.buffer,
            &self.sprites,
            on_screen[self.player],
            player.sprite,
        );
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
            if shadow_in_view(points) {
                let on_screen = points.map(|(x, y)| (x + HUD_WIDTH as i32, y));
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

/// Degrees to radians as the original has it (0x4412B0).
const RADIANS: f64 = 0.017_453_292_519_944_444;

/// `sub_4138A0`: the race's keys as the timer samples them each tick, from the eight
/// controls' scancodes in `dr.cfg` (accelerate, brake, left, right, turbo, gun, and the two
/// that drop a mine) and the arrows, which always drive; the arrows count for the
/// controls set to their extended codes. The first mine control's key is let go once seen.
fn sample(keys: &mut Keys, controls: &[u32; 8]) -> u32 {
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
    if bits & driving::TURBO != 0 {
        bits |= driving::ACCELERATE;
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputEvent, Key};

    /// The defaults `dr.cfg` gets: A, Z, the arrows' extended codes, left shift, left
    /// control, left alt and space.
    const DEFAULT_CONTROLS: [u32; 8] = [0x1E, 0x2C, 0xCB, 0xCD, 0x2A, 0x1D, 0x38, 0x39];

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
            sample(&mut keys, &DEFAULT_CONTROLS),
            driving::ACCELERATE | driving::LEFT
        );
        let mut keys = holding(&[Key::Down, Key::Right, Key::LeftShift]);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS),
            driving::BRAKE | driving::RIGHT | driving::TURBO | driving::ACCELERATE
        );
    }

    /// The first mine control is seen once a press and cancels the brake held with it; the
    /// second holds both bits, which drops a mine.
    #[test]
    fn a_mine_key_counts_once_and_takes_the_brake() {
        let mut keys = holding(&[Key::Z, Key::LeftAlt]);
        assert_eq!(sample(&mut keys, &DEFAULT_CONTROLS), driving::MINE);
        assert_eq!(sample(&mut keys, &DEFAULT_CONTROLS), driving::BRAKE);
        let mut keys = holding(&[Key::Space]);
        assert_eq!(
            sample(&mut keys, &DEFAULT_CONTROLS),
            driving::MINE | driving::BRAKE
        );
    }

    /// The original checks a shadow's corners, not its area: one stretched across the whole
    /// view with its corners outside is never drawn, and cars under it stay lit.
    #[test]
    fn a_shadow_with_every_corner_off_the_view_is_left_out() {
        assert!(!shadow_in_view([(-10, -10), (300, -10), (-10, 250)]));
        assert!(shadow_in_view([(5, -10), (300, 300), (-10, 199)]));
        assert!(!shadow_in_view([(5, -10), (300, 300), (-10, 200)]));
    }
}
