//! The race (spec M4): `startRace` (0x415710) from its load to its end. The race draws into
//! a buffer of rows 512 bytes apart ([`buffer`]), which the window shows doubled from row 40;
//! the track's view is 256 wide right of the HUD's 64.

mod buffer;
mod cars;
mod hud;
mod intro;
mod pedestrians;
mod raster;
mod scene;
mod semaphore;

use deadrally_gamedata::image::Palette;
use deadrally_gamedata::race::{RaceArchives, RaceError, Track};
use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::sound;
use deadrally_gamedata::xm::Bank;

use crate::audio::Sound;

use self::buffer::{Buffer, LEFT, STRIDE};

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
    pub(crate) damage: i32,
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

/// A car in the race.
#[derive(Clone, Debug, PartialEq)]
struct Car {
    x: f32,
    y: f32,
    /// Its direction in steps of 3.75 degrees (`directionRotation` 0x4A7D0C), and in degrees
    /// (`carAngle` 0x4A7DAC).
    rotation: i32,
    angle: f32,
    /// Where its sprite is among all the cars' (`participantBpkOffset` 0x4A7D10).
    sprite: usize,
    lap: i32,
    place: i32,
    /// What is left of the car, 102400 for none of 100 % damage (`damageBar` 0x4A6898).
    damage_bar: i32,
    speed: f32,
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
    Loop { first: bool },
    Intro(Box<intro::Intro>),
}

impl Race {
    /// The race on circuit `circuit` (`TRn` with n = circuit % 9 + 1) over `laps` laps, the
    /// drivers in their places, the player in place `player`.
    pub(crate) fn new(
        archives: &RaceArchives,
        (circuit, laps): (usize, i32),
        drivers: Vec<Driver>,
        player: usize,
        weapons: bool,
    ) -> Result<Race, RaceError> {
        let number = circuit % 9 + 1;
        let track = Track::load(&archives.tracks[number], number)?;
        let scene = scene::Setup::new(&track.scene);
        let cars = drivers
            .iter()
            .enumerate()
            .map(|(slot, driver)| {
                let [x, y, rotation] = track.info.starts[slot];
                Car {
                    x: x as f32,
                    y: y as f32,
                    rotation,
                    angle: rotation as f32 * 3.75,
                    sprite: cars::FRAME * (rotation as usize + cars::FRAMES * slot),
                    lap: 1,
                    place: slot as i32 + 1,
                    damage_bar: (100 - driver.damage) << 10,
                    speed: 0.0,
                }
            })
            .collect();
        let hud = hud::HudImages::load(&archives.ib_files, player, drivers[player].car, weapons)?;
        let looks: Vec<(usize, bool)> = drivers.iter().map(|d| (d.car, d.spikes)).collect();
        let sprites = cars::sprites(&archives.engine, &looks)?;
        let mut shade = [0; 256];
        let table = archives.engine.read("VARJO.TAB")?;
        shade[..table.len().min(256)].copy_from_slice(&table[..table.len().min(256)]);
        let music = sound::load_music(&archives.musics, &format!("TR{number}-MUS.CMF"))
            .map_err(RaceError::Sound)?;
        let effects =
            sound::load_effects(&archives.musics, "GEN-EFE.CMF").map_err(RaceError::Sound)?;
        let pedestrians = pedestrians::Pedestrians::new(
            &track.info,
            hud::decoded(&archives.engine, "PEDESTR.BPK")?,
            [
                hud::decoded(&archives.engine, "SPLAT3.BPK")?,
                hud::decoded(&archives.engine, "SPLAT4.BPK")?,
            ],
        );
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
            clock: Clock {
                timer: 1000,
                ..Clock::default()
            },
            music,
            effects,
        })
    }

    /// The race's sound set up (0x416215: the menu's stopped, the track's music started at
    /// the configured volumes but silent until the intro raises it, the player's engine), then
    /// the race loop's first frame, up to its wait.
    pub(crate) fn begin(&mut self, sound: &mut Sound, (music_volume, effects_volume): (u32, u32)) {
        sound.stop();
        sound.load_effects(&self.effects);
        sound.set_mask(0);
        sound.play_music(&self.music, 0, music_volume);
        sound.set_effects_volume(effects_volume);
        let car = self.drivers[self.player].car as u8;
        sound.trigger_at(ENGINE_CHANNEL, ENGINE_SOUND + car, FULL, ENGINE_PITCH);
        self.frame(sound);
    }

    /// A pass of the race loop up to its wait: a step of the race for each tick waited, then
    /// the frame drawn into the buffer.
    fn frame(&mut self, sound: &mut Sound) {
        let steps = self.clock.waiting;
        self.clock.waiting = 0;
        for _ in 0..steps {
            self.clock.frame += 1;
        }
        self.draw(sound);
    }

    /// From this wait to the next: the frame drawn onto the screen and, on the first, the
    /// intro; or the intro's next step, and once it is over the loop's next frame.
    pub(crate) fn tick(&mut self, sound: &mut Sound) {
        self.clock.tick();
        match &mut self.stage {
            Stage::Loop { first } => {
                let first = *first;
                self.show_buffer();
                if first {
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
                    return;
                }
            }
            Stage::Intro(intro) => {
                let going = intro.wait();
                sound.set_mask(intro.volume() >> 8);
                self.screen.copy_from_slice(intro.screen());
                self.shown = intro.palette().clone();
                if going {
                    return;
                }
                self.clock.restart();
            }
        }
        self.stage = Stage::Loop { first: false };
        self.frame(sound);
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

    /// `recalculateCircuitImageOffset` (0x40D560) without the lead of a moving car: the view
    /// centred on the player, kept inside the track.
    fn camera(&self) -> (usize, usize) {
        let car = &self.cars[self.player];
        let info = &self.track.info;
        let max_x = info.width as i32 - TRACK_VIEW_WIDTH;
        let max_y = info.height as i32 - VIEW_HEIGHT as i32;
        let x = (car.x as i32 - HALF_WIDTH).min(max_x).max(0);
        let y = (car.y as i32 - HALF_HEIGHT).min(max_y).max(0);
        (x as usize, y as usize)
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
                damage_bar: self.cars[slot].damage_bar,
                finished: false,
            })
            .collect()
    }

    /// A frame: the track under the camera copied right of the HUD (0x4170C1), the HUD.
    fn draw(&mut self, sound: &mut Sound) {
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
                    self.clock.restart();
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
        let player = &self.cars[self.player];
        let gauge = hud::Player {
            speed: player.speed,
            engine: 1.0,
            weapons: self.weapons,
            weapons_bar: hud::FULL_BAR,
            turbo_bar: hud::FULL_BAR,
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

    /// Where the player's car is on the screen (0x40D929): in the middle of the view unless
    /// the view has stopped at the track's edge.
    fn player_on_screen(&self) -> (i32, i32) {
        let car = &self.cars[self.player];
        let info = &self.track.info;
        let axis = |position: f32, half: i32, size: i32, view: i32| {
            let at = f64::from(position);
            if at < f64::from(half) {
                position as i32
            } else if at > f64::from(size - half) {
                (at - f64::from(size - view)) as i32
            } else {
                half
            }
        };
        let width = info.width as i32;
        let height = info.height as i32;
        (
            axis(car.x, HALF_WIDTH, width, TRACK_VIEW_WIDTH) + HUD_WIDTH as i32,
            axis(car.y, HALF_HEIGHT, height, VIEW_HEIGHT as i32),
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
        if player.damage_bar > 0 {
            cars::headlights(&mut self.buffer, on_screen[self.player], player.angle, lit);
        }
        for slot in others.clone() {
            let (x, y) = on_screen[slot];
            let near = x > left - 40 && x < 360 && y > -40 && y < VIEW_HEIGHT as i32 + 40;
            if near && self.cars[slot].damage_bar > 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The original checks a shadow's corners, not its area: one stretched across the whole
    /// view with its corners outside is never drawn, and cars under it stay lit.
    #[test]
    fn a_shadow_with_every_corner_off_the_view_is_left_out() {
        assert!(!shadow_in_view([(-10, -10), (300, -10), (-10, 250)]));
        assert!(shadow_in_view([(5, -10), (300, 300), (-10, 199)]));
        assert!(!shadow_in_view([(5, -10), (300, 300), (-10, 200)]));
    }
}
