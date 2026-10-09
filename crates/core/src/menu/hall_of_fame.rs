//! The Hall of Fame (spec M2c §3): the best ten (`seeHallOfFame`, 0x431510), the records by
//! circuit (`drawRecordByCircuit`, 0x41E490) and the wipes between them and the main menu
//! (`sub_42C560`, `sub_42C4A0`).

use deadrally_gamedata::dr_cfg::ARENA;
use deadrally_gamedata::image::Image;

use super::draw::Focus;
use super::{Menu, State};
use crate::canvas::{Canvas, at};
use crate::keys;

/// The wipe: 43 steps of 15 pixels, a band of 10 tile columns and 22 tile rows of 15 x 15,
/// copied to the screen as a 150 x 330 window from row 75; the race's preview's (0x42C670)
/// 27 tile rows from row 73.
const WIPE_STEPS: u32 = 43;
const WIPE_TOP: usize = 75;
const TILE: usize = 15;
const WIPE_ROWS: usize = 22;
const PREVIEW_TOP: usize = 73;
const PREVIEW_ROWS: usize = 27;
const WIPE_COLUMNS: usize = 10;
/// The music's volume mask falls from here by this much a step while the menu wipes away.
const WIPE_VOLUME: u32 = 65_532;
const WIPE_VOLUME_STEP: u32 = 1524;
/// The Hall of Fame's music starts at this order; the volume mask is then 0x10000 >> 8.
const FAME_ORDER: usize = 81;
const FULL_MASK: u32 = 0x1_0000 >> 8;
/// Effect 26 sounds as Left or Right change the circuit; the arrow stays lit for 8 waits.
const STEP_SOUND: u8 = 26;
const ARROW_WAITS: u32 = 8;
/// The snapshots are drawn 98 rows high.
const SNAPSHOT_ROWS: u32 = 98;

/// What a wipe brings in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Wipe {
    Fame,
    Records,
    Menu,
    /// The sign-up screen, and the Start Racing menu after a game's screens (`sub_42C4A0`,
    /// no music).
    SignUp,
    StartMenu,
    /// The shop, drawn over a copy of the screen.
    Shop,
    /// The race's preview (`sub_42C670`), the music falling; one more wait after it.
    Preview,
    /// The Adversary's screen (`sub_42C4A0`).
    Adversary,
}

impl Wipe {
    /// The band's top row and its rows of tiles.
    fn band(self) -> (usize, usize) {
        if self == Wipe::Preview {
            (PREVIEW_TOP, PREVIEW_ROWS)
        } else {
            (WIPE_TOP, WIPE_ROWS)
        }
    }
}

fn upper(text: &[u8]) -> Vec<u8> {
    text.to_ascii_uppercase()
}

impl Menu {
    /// The best ten drawn into the second buffer, then wiped in with the music falling.
    pub(super) fn open_hall_of_fame(&mut self) -> State {
        self.config.upper_case_hall_of_fame();
        self.back = self.screen.clone();
        self.back.copy_rows(&self.graphics.background, 105, 262);
        let menu = &self.assets.menu;
        self.back.draw(&menu.fame_title, at(0, 84), true);
        let mut back = std::mem::take(&mut self.back);
        self.draw_best_ten(&mut back);
        self.back = back;
        State::Wipe {
            wipe: Wipe::Fame,
            step: 0,
        }
    }

    /// One step of a wipe after its wait: the band of masked tiles from the second buffer,
    /// its window shown, the music falling when the menu goes or comes back.
    pub(super) fn wipe_tick(&mut self, wipe: Wipe, step: u32) -> State {
        self.palette.after_wait();
        let (top, rows) = wipe.band();
        let base = at(0, top) + TILE * step as usize;
        for row in 0..rows {
            for column in 0..WIPE_COLUMNS {
                let offset = base + TILE * (row * crate::canvas::WIDTH + column);
                self.screen
                    .blit_mask(&self.assets.menu.wipe[column], &self.back, offset);
            }
        }
        self.shown.copy_from(&self.screen, base, 150, TILE * rows);
        if matches!(wipe, Wipe::Fame | Wipe::Menu | Wipe::Preview) {
            self.sound
                .set_mask((WIPE_VOLUME - WIPE_VOLUME_STEP * step) >> 8);
        }
        if step + 1 < WIPE_STEPS {
            return State::Wipe {
                wipe,
                step: step + 1,
            };
        }
        self.keys.take();
        self.keys.take();
        match wipe {
            Wipe::Fame => {
                self.music_order = self.sound.music_order();
                self.sound.set_music_order(FAME_ORDER);
                self.sound.set_mask(FULL_MASK);
                self.keys.take();
                self.keys.take();
                State::FameWait
            }
            Wipe::Records => State::Records { index: 0 },
            Wipe::SignUp => self.sign_up_shown(),
            Wipe::Shop => self.shop_shown(),
            Wipe::Preview => State::PreviewWait,
            Wipe::Adversary => self.adversary_shown(),
            Wipe::StartMenu => {
                self.shown = self.screen.clone();
                State::Submenu {
                    menu: super::Submenu::Start,
                    second: false,
                }
            }
            Wipe::Menu => {
                let volume = self.config.music_volume();
                self.sound
                    .play_music(&self.assets.menu_music, self.music_order, volume);
                self.sound.set_mask(FULL_MASK);
                self.shown = self.screen.clone();
                State::Main { second: false }
            }
        }
    }

    /// The best ten wait for any key.
    pub(super) fn fame_wait(&mut self) -> State {
        self.palette.after_wait();
        if self.keys.take() == 0 {
            return State::FameWait;
        }
        self.keys.take();
        self.keys.take();
        self.back = self.screen.clone();
        self.back.copy_rows(&self.graphics.background, 84, 283);
        let menu = &self.assets.menu;
        self.back.draw(&menu.records_title, at(0, 92), true);
        let circuit = usize::from(menu.texts.hall_of_fame.circuit_order[0]);
        let mut back = std::mem::take(&mut self.back);
        self.draw_records(&mut back, circuit);
        back.draw(&self.snapshot(circuit), at(40, 214), false);
        back.draw(&self.assets.menu.arrows[0], at(24, 228), false);
        back.draw(&self.assets.menu.arrows[1], at(168, 228), false);
        self.border(&mut back, 15, 204, 178, 117);
        self.back = back;
        State::Wipe {
            wipe: Wipe::Records,
            step: 0,
        }
    }

    /// The records screen's key after its wait: Left and Right step through the circuits,
    /// Enter, keypad Enter and Escape leave.
    pub(super) fn records_tick(&mut self, index: usize) -> State {
        self.palette.after_wait();
        let pages = self.records_pages();
        let (index, right) = match self.keys.take() {
            keys::LEFT | keys::PAD_LEFT => ((index + pages - 1) % pages, false),
            keys::RIGHT | keys::PAD_RIGHT => ((index + 1) % pages, true),
            keys::ENTER | keys::ESCAPE | 0x9C => return self.leave_hall_of_fame(),
            // F1 opens the chat in a network game; nothing else does anything here.
            _ => return State::Records { index },
        };
        self.sound(STEP_SOUND);
        let (frame, x) = if right { (3, 168) } else { (2, 24) };
        let mut screen = std::mem::take(&mut self.screen);
        screen.draw(&self.assets.menu.arrows[frame], at(x, 228), false);
        let circuit = self.records_page(index);
        screen.draw(&self.snapshot(circuit), at(40, 214), false);
        self.draw_records(&mut screen, circuit);
        self.screen = screen;
        self.shown = self.screen.clone();
        State::RecordsArrow {
            index,
            right,
            waits: 0,
        }
    }

    /// The lit arrow's waits; after the eighth it goes dark again.
    pub(super) fn records_arrow(&mut self, index: usize, right: bool, waits: u32) -> State {
        self.palette.after_wait();
        if waits + 1 < ARROW_WAITS {
            return State::RecordsArrow {
                index,
                right,
                waits: waits + 1,
            };
        }
        let (frame, x) = if right { (1, 168) } else { (0, 24) };
        self.screen
            .draw(&self.assets.menu.arrows[frame], at(x, 228), false);
        self.shown = self.screen.clone();
        State::Records { index }
    }

    /// Back to the main menu: drawn into the second buffer and wiped in with the music
    /// falling; the menu music then starts again at the order it had.
    fn leave_hall_of_fame(&mut self) -> State {
        self.back = self.screen.clone();
        self.back.copy_rows(&self.graphics.background, 84, 283);
        self.graphics
            .menu(&mut self.back, &self.main, Focus::Focused, self.cursor);
        State::Wipe {
            wipe: Wipe::Menu,
            step: 0,
        }
    }

    /// The records' pages: the circuits in the Hall of Fame's order, then in DeadRally the
    /// Arena's own records, which the Windows version has none of.
    fn records_pages(&self) -> usize {
        let circuits = self.assets.menu.texts.hall_of_fame.circuit_order.len();
        circuits + usize::from(!self.campaign.windows_version)
    }

    /// Page `index`'s circuit, or [`ARENA`].
    fn records_page(&self, index: usize) -> usize {
        let order = &self.assets.menu.texts.hall_of_fame.circuit_order;
        order
            .get(index)
            .map_or(ARENA, |&circuit| usize::from(circuit))
    }

    /// Circuit `circuit`'s records (0x41E490): two areas restored, the bar, the circuit's name
    /// centred, and car 5 down to car 0 with their record's driver and time.
    fn draw_records(&self, canvas: &mut Canvas, circuit: usize) {
        let background = &self.graphics.background;
        canvas.restore(background, at(225, 133), 381, 29);
        canvas.restore(background, at(224, 206), 368, 130);
        let menu = &self.assets.menu;
        canvas.draw(&menu.records_bar, at(0, 132), true);
        let hall = &menu.texts.hall_of_fame;
        let name = match hall.circuits.get(circuit) {
            Some(name) => name,
            None => arena_name(&menu.texts.campaign.race_kinds[crate::campaign::ARENA]),
        };
        let width = self.graphics.big_a.width(name);
        self.graphics
            .big_a
            .draw(canvas, name, at(413, 136) - width / 2);
        for row in 0..6 {
            let car = 5 - row;
            let y = 208 + 22 * row;
            let medium = &self.graphics.medium;
            medium.draw(canvas, &upper(&hall.cars[car]), at(228, y));
            let (driver, [minutes, seconds, hundredths]) = self.config.record(circuit, car);
            medium.draw(canvas, &upper(driver), at(360, y));
            let time = format!("{minutes:02}:{seconds:02}.{hundredths:02}");
            medium.draw(canvas, time.as_bytes(), at(514, y));
        }
    }

    /// Circuit `circuit`'s snapshot, 98 rows of its frame; the Arena, which has none, its
    /// picture on the race's preview shrunk to a snapshot's size.
    fn snapshot(&self, circuit: usize) -> Image {
        let menu = &self.assets.menu;
        if circuit == ARENA {
            let width = menu.snapshots.first().map_or(0, |frame| frame.width);
            let shape = menu.track_shapes.get(super::preview::ARENA_SHAPE);
            return shape.map_or_else(
                || Image::new(0, 0, Vec::new()),
                |shape| shrink(shape, width, SNAPSHOT_ROWS),
            );
        }
        let frame = &self.assets.menu.snapshots[circuit];
        let rows = (frame.width * SNAPSHOT_ROWS) as usize;
        Image::new(frame.width, SNAPSHOT_ROWS, frame.pixels[..rows].to_vec())
    }

    /// The best ten's rows: the rank, the name, the races and the difficulty in the medium
    /// font, 22 lines apart from line 144.
    pub(super) fn draw_best_ten(&self, canvas: &mut Canvas) {
        let medium = &self.graphics.medium;
        let difficulties = &self.assets.menu.texts.hall_of_fame.difficulties;
        for rank in 0..10 {
            let y = 144 + 22 * rank;
            let rank_at = if rank == 9 { at(28, y) } else { at(36, y) };
            medium.draw(canvas, format!("{}.", rank + 1).as_bytes(), rank_at);
            let (name, races, difficulty) = self.config.hall_of_fame(rank);
            medium.draw(canvas, name, at(137, y));
            if races >= 0 {
                let x = match races {
                    0..10 => 344,
                    10..100 => 336,
                    _ => 328,
                };
                medium.draw(canvas, races.to_string().as_bytes(), at(x, y));
            }
            let level = difficulties
                .get(difficulty as usize)
                .map_or(&[][..], Vec::as_slice);
            medium.draw(canvas, &upper(level), at(429, y));
        }
    }

    /// `drawBorder_421980`: the border with `CHOO2`'s corners cut to their outer halves (12
    /// rows), for a single row.
    pub(super) fn thin_border(
        &self,
        canvas: &mut Canvas,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) {
        const CORNER: usize = 24;
        const HALF: usize = 12;
        const LINE: u8 = 0x16;
        let corners = &self.assets.menu.border_corners;
        let half = |corner: usize, bottom: bool| {
            let image = &corners[corner];
            let w = image.width as usize;
            let first = if bottom {
                image.height as usize - HALF
            } else {
                0
            };
            Image::new(
                image.width,
                HALF as u32,
                image.pixels[first * w..(first + HALF) * w].to_vec(),
            )
        };
        let right = x + width - CORNER;
        let bottom = y + height - HALF;
        canvas.draw(&half(0, false), at(x, y), true);
        canvas.draw(&half(1, false), at(right, y), true);
        canvas.draw(&half(2, true), at(x, bottom), true);
        canvas.draw(&half(3, true), at(right, bottom), true);
        let across = width.saturating_sub(2 * CORNER);
        let down = height.saturating_sub(2 * CORNER);
        canvas.fill(at(x + CORNER, y + 2), across, 1, LINE);
        canvas.fill(at(x + CORNER, y + height - 3), across, 1, LINE);
        canvas.fill(at(x + 2, y + CORNER), 1, down, LINE);
        canvas.fill(at(x + width - 3, y + CORNER), 1, down, LINE);
    }

    /// `drawBorder` (0x421AE0): `CHOO2`'s four corners and lines of colour 22 between them.
    pub(super) fn border(
        &self,
        canvas: &mut Canvas,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) {
        const CORNER: usize = 24;
        const LINE: u8 = 0x16;
        let corners = &self.assets.menu.border_corners;
        let right = x + width - CORNER;
        let bottom = y + height - CORNER;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            canvas.draw(corner, offset, true);
        }
        let across = width - 2 * CORNER;
        let down = height - 2 * CORNER;
        canvas.fill(at(x + CORNER, y + 2), across, 1, LINE);
        canvas.fill(at(x + CORNER, y + height - 3), across, 1, LINE);
        canvas.fill(at(x + 2, y + CORNER), 1, down, LINE);
        canvas.fill(at(x + width - 3, y + CORNER), 1, down, LINE);
    }
}

/// The Arena's name as the statistics' heading for its race has it, without the colon.
fn arena_name(heading: &[u8]) -> &[u8] {
    let heading = heading.trim_ascii();
    heading.strip_suffix(b":").unwrap_or(heading)
}

/// `image` shrunk to `width` x `height`, each pixel the nearest of the image's.
fn shrink(image: &Image, width: u32, height: u32) -> Image {
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let (from_x, from_y) = (x * image.width / width, y * image.height / height);
                image.pixels[(from_y * image.width + from_x) as usize]
            })
        })
        .collect();
    Image::new(width, height, pixels)
}
