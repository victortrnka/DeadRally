//! DeadRally's personal best (docs/design/2026-10-13-personal-best-design.md): the player's
//! best lap and best whole races of the game on every circuit with every car, after the
//! statistics of the Start Racing menu, a page for each circuit and the Arena.

use deadrally_gamedata::dr_cfg::{ARENA, ARENA_LAPS, RACE_LAPS};
use deadrally_gamedata::image::Image;

use super::Menu;
use crate::canvas::{Canvas, at};
use crate::keys;

/// The screen's title, centred on line 96.
const TITLE: &[u8] = b"PERSONAL BEST";
const TITLE_Y: usize = 96;
/// What a page clears: from line 128, 236 lines.
const PAGE_TOP: usize = 128;
const PAGE_LINES: usize = 236;
/// The records bar (`RECOBAR`), drawn from line 132 on the records: its title box (columns
/// 226 to 605, lines 0 to 37), here centred; and its third heading box (lines 47 to 67), whose
/// borders (columns 491 to 493 and 603 to 605) and grid (eight columns from 494) make boxes of
/// any width.
const BAR_TOP: usize = 132;
const TITLE_BOX: (usize, usize, usize) = (226, 380, 38);
const HEADING_LINES: (usize, usize) = (47, 21);
const HEADING_LEFT: usize = 491;
const HEADING_RIGHT: usize = 603;
const HEADING_TILE: usize = 494;
const BORDER: usize = 3;
const GRID: usize = 8;
/// The headings' text, from line 184; the columns: the car's from column 40, 132 wide, and
/// the times' from 186, 100 wide and 106 apart; the cars' rows from line 208, 22 apart.
const HEADING_TEXT_Y: usize = BAR_TOP + 52;
const CAR_COLUMN: (usize, usize) = (40, 132);
const TIME_COLUMNS: usize = 186;
const TIME_WIDTH: usize = 100;
const TIME_STEP: usize = 106;
const ROWS_Y: usize = 208;
const ROW_STEP: usize = 22;
/// A time never driven.
const NONE: &[u8] = b"--:--.--";
/// Effect 26 sounds as Left or Right change the circuit, as on the records.
const STEP_SOUND: u8 = 26;

impl Menu {
    /// The personal best's screen at page `index`, after the statistics: the menus'
    /// background with the title, the page and the bottom panel.
    pub(super) fn draw_personal_best_screen(&self, canvas: &mut Canvas, index: usize) {
        canvas.copy_all(&self.graphics.background);
        let medium = &self.graphics.medium;
        let half = medium.width(TITLE) / 2;
        medium.draw(canvas, TITLE, at(320 - half, TITLE_Y));
        self.draw_personal_best(canvas, index);
        self.graphics.panel_frame(canvas, 0, 371, 639, 109);
        self.graphics.panel_text(canvas, &self.panel);
    }

    /// A key on page `index`: Left and Right step through the circuits and the Arena, the
    /// page drawn afresh; the page it shows, or none when Enter, keypad Enter, Space or
    /// Escape leave.
    pub(super) fn personal_best_key(&mut self, index: usize) -> Option<usize> {
        let pages = self.assets.menu.texts.hall_of_fame.circuit_order.len() + 1;
        let index = match self.keys.take() {
            keys::LEFT | keys::PAD_LEFT => (index + pages - 1) % pages,
            keys::RIGHT | keys::PAD_RIGHT => (index + 1) % pages,
            keys::ENTER | keys::SPACE | keys::ESCAPE | 0x9C => return None,
            _ => return Some(index),
        };
        self.sound(STEP_SOUND);
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_personal_best(&mut screen, index);
        self.screen = screen;
        self.shown = self.screen.clone();
        Some(index)
    }

    /// Page `index`: the circuit's name in the title box, the headings, and each car's best
    /// lap and races, car 5 down to car 0 as on the records.
    fn draw_personal_best(&self, canvas: &mut Canvas, index: usize) {
        let menu = &self.assets.menu;
        let hall = &menu.texts.hall_of_fame;
        let circuit = hall
            .circuit_order
            .get(index)
            .map_or(ARENA, |&circuit| usize::from(circuit));
        canvas.copy_rows(&self.graphics.background, PAGE_TOP, PAGE_LINES);
        let bar = &menu.records_bar;
        let (box_x, box_width, box_lines) = TITLE_BOX;
        let title_box = crop(bar, box_x, 0, box_width, box_lines);
        canvas.draw(&title_box, at(320 - box_width / 2, BAR_TOP), true);
        let name = match hall.circuits.get(circuit) {
            Some(name) => name.as_slice(),
            None => super::hall_of_fame::arena_name(
                &menu.texts.campaign.race_kinds[crate::campaign::ARENA],
            ),
        };
        let width = self.graphics.big_a.width(name);
        self.graphics
            .big_a
            .draw(canvas, name, at(320 - width / 2, BAR_TOP + 4));
        let laps: Vec<Option<i32>> = if circuit == ARENA {
            vec![None, Some(ARENA_LAPS)]
        } else {
            [None].into_iter().chain(RACE_LAPS.map(Some)).collect()
        };
        let mut headings = vec![(CAR_COLUMN.0, CAR_COLUMN.1, b"CAR".to_vec())];
        for (column, kind) in laps.iter().enumerate() {
            let text = match kind {
                None => b"LAP".to_vec(),
                Some(laps) => format!("{laps} LAPS").into_bytes(),
            };
            headings.push((TIME_COLUMNS + TIME_STEP * column, TIME_WIDTH, text));
        }
        let medium = &self.graphics.medium;
        for (x, width, text) in &headings {
            canvas.draw(
                &heading_box(bar, *width),
                at(*x, BAR_TOP + HEADING_LINES.0),
                true,
            );
            let half = medium.width(text) / 2;
            medium.draw(canvas, text, at(x + width / 2 - half, HEADING_TEXT_Y));
        }
        let best = &self.campaign.personal_best;
        for row in 0..6 {
            let car = 5 - row;
            let y = ROWS_Y + ROW_STEP * row;
            medium.draw(
                canvas,
                &hall.cars[car].to_ascii_uppercase(),
                at(CAR_COLUMN.0 + 6, y),
            );
            for (column, kind) in laps.iter().enumerate() {
                let time = match kind {
                    None => best.lap(circuit, car),
                    Some(laps) => best.race(circuit, *laps, car),
                };
                let text = time.map_or(NONE.to_vec(), |[m, s, h]| {
                    format!("{m:02}:{s:02}.{h:02}").into_bytes()
                });
                let centre = TIME_COLUMNS + TIME_STEP * column + TIME_WIDTH / 2;
                let half = medium.width(&text) / 2;
                medium.draw(canvas, &text, at(centre - half, y));
            }
        }
    }
}

/// `image`'s `width` x `height` from (`x`, `y`).
fn crop(image: &Image, x: usize, y: usize, width: usize, height: usize) -> Image {
    let stride = image.width as usize;
    let pixels = (y..y + height)
        .flat_map(|row| image.pixels[row * stride + x..row * stride + x + width].to_vec())
        .collect();
    Image::new(width as u32, height as u32, pixels)
}

/// A heading box `width` wide made of the records bar's third: its borders, and its grid
/// repeated between them.
fn heading_box(bar: &Image, width: usize) -> Image {
    let stride = bar.width as usize;
    let (top, lines) = HEADING_LINES;
    let pixels = (top..top + lines)
        .flat_map(|row| {
            (0..width).map(move |column| {
                let from = if column < BORDER {
                    HEADING_LEFT + column
                } else if column >= width - BORDER {
                    HEADING_RIGHT + column - (width - BORDER)
                } else {
                    HEADING_TILE + (column - BORDER) % GRID
                };
                bar.pixels[row * stride + from]
            })
        })
        .collect();
    Image::new(width as u32, lines as u32, pixels)
}
