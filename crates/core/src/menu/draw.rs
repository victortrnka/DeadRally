//! The menu screen's drawing, as `dr.exe` does it into its screen buffer and copies to the
//! shown buffer (spec M2a §3.3, §3.4): popups (`createPopup` 0x41A530), menus (`drawMenu`
//! 0x41A880), the cursor (`updateCursor` 0x41AB50), the highlight's moves (`refreshMenuUp`
//! 0x41AF40, `refreshMenuDown` 0x41B1A0, 0x41ACF0) and the bottom panel (0x41A7A0, 0x41E810).

use deadrally_gamedata::assets::MenuAssets;
use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::Texts;

use crate::canvas::{Canvas, at};
use crate::font::Font;

/// The fill colour of popups and of the cursor's box.
pub(crate) const POPUP_FILL: u8 = 0xC4;
/// Popup lines: a focused popup's, an unfocused one's.
const LINE_FOCUSED: u8 = 7;
const LINE_UNFOCUSED: u8 = 4;
/// Corner pictures are 32x20, the cursor 20x20, a big glyph 32 high.
const CORNER_WIDTH: usize = 32;
const CORNER_HEIGHT: usize = 20;
const CURSOR_SIZE: usize = 20;
pub(crate) const CURSOR_FRAMES: usize = 50;

/// One menu of the table at 0x4456F0 and its active rows (0x4457F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuTable {
    /// Which menu of `dr.exe`'s text table its rows are.
    pub(crate) text: usize,
    pub(crate) rows: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) row_height: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) selected: usize,
    pub(crate) active: [bool; 9],
}

/// The main menu: start, multiplayer (inactive), configure, hall of fame, credits, exit.
pub(crate) const MAIN_MENU: MenuTable = MenuTable {
    text: 0,
    rows: 6,
    x: 145,
    y: 124,
    row_height: 28,
    width: 349,
    height: 192,
    selected: 0,
    active: [true, false, true, true, true, true, false, false, false],
};

/// The start submenu at the first start: rows 0, 3 and 5 active.
pub(crate) const START_MENU: MenuTable = MenuTable {
    text: 1,
    rows: 6,
    x: 109,
    y: 171,
    row_height: 28,
    width: 421,
    height: 192,
    selected: 0,
    active: [true, false, false, true, false, true, false, false, false],
};

/// Configure (menu 3): music volume, effect volume, define keyboard, define gamepad, the
/// gamepad switch, previous menu.
pub(crate) const CONFIGURE_MENU: MenuTable = MenuTable {
    text: 3,
    rows: 6,
    x: 95,
    y: 146,
    row_height: 28,
    width: 485,
    height: 192,
    selected: 0,
    active: [true, true, true, true, true, true, false, false, false],
};

/// Define Keyboard (menu 6): the eight controls and previous menu.
pub(crate) const KEYBOARD_MENU: MenuTable = MenuTable {
    text: 6,
    rows: 9,
    x: 50,
    y: 93,
    row_height: 28,
    width: 532,
    height: 278,
    selected: 0,
    active: [true; 9],
};

/// Define Gamepad (menu 8): seven controls (no horn) and previous menu.
pub(crate) const PAD_MENU: MenuTable = MenuTable {
    text: 8,
    rows: 8,
    x: 50,
    y: 113,
    row_height: 28,
    width: 532,
    height: 250,
    selected: 0,
    active: [true, true, true, true, true, true, true, true, false],
};

/// The saved games' slots (menu 5): eight rows, the last the quicksave.
pub(crate) const SLOTS_MENU: MenuTable = MenuTable {
    text: 5,
    rows: 8,
    x: 231,
    y: 114,
    row_height: 28,
    width: 383,
    height: 250,
    selected: 0,
    active: [true, true, true, true, true, true, true, true, false],
};

/// How a menu is drawn: unfocused (mode 0) or focused (mode 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Unfocused,
    Focused,
}

/// The menu's pictures, fonts and rows.
#[derive(Clone, Debug)]
pub(crate) struct Graphics {
    pub(crate) background: Image,
    panel_line: Image,
    corners_focused: Vec<Image>,
    corners_unfocused: Vec<Image>,
    cursor: Vec<Image>,
    pub(crate) big_a: Font,
    pub(crate) big_b: Font,
    pub(crate) big_d: Font,
    pub(crate) small: [Font; 3],
    /// The Hall of Fame's font.
    pub(crate) medium: Font,
    /// `dr.exe`'s menu text table: `menus[m][r]` is row `r` of menu `m`. The original rewrites
    /// some rows as settings change.
    menus: Vec<Vec<Vec<u8>>>,
    /// The volume popups' slider and its knob.
    pub(crate) slider: Image,
    pub(crate) knob: Image,
}

impl Graphics {
    pub(crate) fn new(assets: &MenuAssets) -> Graphics {
        let texts = &assets.texts;
        Graphics {
            background: assets.background.clone(),
            panel_line: assets.panel_line.clone(),
            corners_focused: assets.corners_focused.clone(),
            corners_unfocused: assets.corners_unfocused.clone(),
            cursor: assets.cursor.clone(),
            big_a: Font::new(assets.big_a.clone(), &texts.big),
            big_b: Font::new(assets.big_b.clone(), &texts.big),
            big_d: Font::new(assets.big_d.clone(), &texts.big),
            small: [
                Font::new(assets.small_a.clone(), &texts.small),
                Font::new(assets.small_b.clone(), &texts.small),
                Font::new(assets.small_c.clone(), &texts.small),
            ],
            medium: Font::new(assets.medium.clone(), &texts.medium),
            menus: texts.menus.clone(),
            slider: assets.slider.clone(),
            knob: assets.knob.clone(),
        }
    }

    /// Rewrites row `row` of menu `menu`, as the original copies a setting's text into its
    /// table.
    pub(crate) fn row(&self, menu: usize, row: usize) -> &[u8] {
        &self.menus[menu][row]
    }

    pub(crate) fn set_row(&mut self, menu: usize, row: usize, text: Vec<u8>) {
        self.menus[menu][row] = text;
    }

    pub(crate) fn cursor(&self, frame: usize) -> &Image {
        &self.cursor[frame % self.cursor.len()]
    }

    /// `createPopup(x, y, w, h, focus)`: fill, corners, then lines; nothing outside is
    /// cleared, so a popup drawn over another blends their corners.
    pub(crate) fn popup(
        &self,
        screen: &mut Canvas,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        focus: Focus,
    ) {
        if h > 8 {
            screen.fill(at(x + 2, y + 2), w - 6, h - 8, POPUP_FILL);
        }
        let (corners, line) = match focus {
            Focus::Unfocused => (&self.corners_unfocused, LINE_UNFOCUSED),
            Focus::Focused => (&self.corners_focused, LINE_FOCUSED),
        };
        let right = x + w - CORNER_WIDTH;
        let bottom = y + h - CORNER_HEIGHT;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            screen.draw(corner, offset, true);
        }
        if w > 64 {
            screen.fill(at(x + 32, y + 1), w - 64, 1, line);
            screen.fill(at(x + 32, y + h - 7), w - 64, 1, line);
        }
        if h > 40 {
            screen.fill(at(x + 1, y + 20), 1, h - 40, line);
            screen.fill(at(x + w - 5, y + 20), 1, h - 40, line);
        }
    }

    /// `drawMenu(menu, focus)`: its popup and rows; the selected row with the cursor when the
    /// menu has focus. Nothing reaches the shown buffer.
    pub(crate) fn menu(&self, screen: &mut Canvas, menu: &MenuTable, focus: Focus, cursor: usize) {
        self.popup(screen, menu.x, menu.y, menu.width, menu.height, focus);
        for row in 0..menu.rows {
            let text = &self.menus[menu.text][row];
            let at_text = at(menu.x + 32, menu.y + 5 + row * menu.row_height);
            let font = if row == menu.selected {
                if focus == Focus::Focused {
                    screen.draw(self.cursor(cursor), self.cursor_at(menu), true);
                    &self.big_a
                } else {
                    &self.big_d
                }
            } else if menu.active[row] && focus == Focus::Focused {
                &self.big_b
            } else {
                &self.big_d
            };
            font.draw(screen, text, at_text);
        }
    }

    /// Where the selected row's cursor goes.
    fn cursor_at(&self, menu: &MenuTable) -> usize {
        at(menu.x + 9, menu.y + 11 + menu.selected * menu.row_height)
    }

    /// `updateCursor`: the cursor's box refilled, frame `frame` drawn, the box copied to the
    /// shown buffer.
    pub(crate) fn update_cursor(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &MenuTable,
        frame: usize,
    ) {
        let offset = self.cursor_at(menu);
        screen.fill(offset, CURSOR_SIZE, CURSOR_SIZE, POPUP_FILL);
        screen.draw(self.cursor(frame), offset, true);
        shown.copy_from(screen, offset, CURSOR_SIZE, CURSOR_SIZE);
    }

    /// Moves the highlight to row `to` as `refreshMenuUp`/`refreshMenuDown` and 0x41ACF0 do:
    /// both rows' areas refilled and redrawn, the cursor drawn with frame `frame`, both
    /// copied to the shown buffer. `base` is 6 for Up and the jump to the last row, 5 for Down.
    pub(crate) fn move_highlight(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &mut MenuTable,
        to: usize,
        base: usize,
        frame: usize,
    ) {
        let rows_at = |row: usize| menu.y + base + row * menu.row_height;
        let text_at = |row: usize| at(menu.x + 32, menu.y + 5 + row * menu.row_height);
        let old = menu.selected;
        screen.fill(
            at(menu.x + 9, rows_at(old) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_b
            .draw(screen, &self.menus[menu.text][old], text_at(old));
        menu.selected = to;
        screen.fill(
            at(menu.x + 9, rows_at(to) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_a
            .draw(screen, &self.menus[menu.text][to], text_at(to));
        screen.draw(self.cursor(frame), self.cursor_at(menu), true);
        shown.copy_from(screen, at(menu.x + 7, rows_at(old)), menu.width - 10, 32);
        shown.copy_from(screen, at(menu.x + 7, rows_at(to)), menu.width - 10, 32);
    }

    /// `drawTransparentBlock(x, y, w, h)`: the background restored, then the panel's two
    /// frame lines.
    pub(crate) fn panel_frame(&self, screen: &mut Canvas, x: usize, y: usize, w: usize, h: usize) {
        screen.restore(&self.background, at(x + 2, y - 4), w - 6, h);
        screen.draw(&self.panel_line, at(0, y + 1), true);
        screen.draw(&self.panel_line, at(0, y + h - 9), true);
    }

    /// `writeTextInScreen` (0x41A430): small B; "}" switches to small C, "[" to small A, "{"
    /// back to small B, the codes taking no room.
    pub(crate) fn write_text(&self, screen: &mut Canvas, text: &[u8], offset: usize) {
        let mut font = 1;
        let mut pen = offset;
        for &c in text {
            match c {
                b'[' => font = 0,
                b'{' => font = 1,
                b'}' => font = 2,
                _ => pen = self.small[font].draw(screen, &[c], pen),
            }
        }
    }

    /// `drawBottomMenuText`: rows 380..=468 restored, then the panel's last six lines at
    /// (12, 378 + 15k), each in its own small font.
    pub(crate) fn panel_text(&self, screen: &mut Canvas, panel: &Panel) {
        screen.copy_rows(&self.background, 380, 89);
        for (k, line) in panel.lines[16..].iter().enumerate() {
            if let Some(font) = self.small.get(usize::from(line.font)) {
                font.draw(screen, &line.text, at(12, 378 + 15 * k));
            }
        }
    }
}

/// One line of the bottom panel and its font (0, 1, 2: small A, B, C).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: Vec<u8>,
    pub(crate) font: u8,
}

/// The bottom message panel: 22 lines, new ones pushed in at the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    lines: Vec<PanelLine>,
}

impl Panel {
    /// The panel as `mainMenu` fills it at start-up: the four start-up lines in small B, an
    /// empty line before the last.
    pub(crate) fn startup(texts: &Texts) -> Panel {
        let mut panel = Panel {
            lines: vec![PanelLine::default(); 22],
        };
        let [first, second, third, last] = [0, 1, 2, 3].map(|i| texts.panel[i].clone());
        for text in [first, second, third, Vec::new(), last] {
            panel.push(text, 1);
        }
        panel
    }

    fn push(&mut self, text: Vec<u8>, font: u8) {
        self.lines.remove(0);
        self.lines.push(PanelLine { text, font });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::{HEIGHT, WIDTH};

    /// Graphics where every picture has its own colour: corners 11..=14 (focused) and 21..=24,
    /// cursor frame k colour 100 + k with a transparent top-left pixel, glyphs as in
    /// `font::tests::font` but 32x32 (big) or 16x16 (small) and colour 50 + font.
    pub(crate) fn graphics() -> Graphics {
        let solid = |w: u32, h: u32, colour: u8| Image::new(w, h, vec![colour; (w * h) as usize]);
        let glyphs = |size: u32, colour: u8| {
            (0..96)
                .map(|_| solid(size, size, colour))
                .collect::<Vec<_>>()
        };
        let metrics = |size: u8| deadrally_gamedata::text::Metrics {
            width: size,
            height: size,
            advances: vec![size; 96],
        };
        Graphics {
            background: Image::new(
                640,
                480,
                (0..WIDTH * HEIGHT).map(|i| (i % 7) as u8 + 1).collect(),
            ),
            panel_line: solid(640, 10, 99),
            corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
            corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
            cursor: (0..50)
                .map(|k| {
                    let mut frame = solid(20, 20, 100 + k);
                    frame.pixels[0] = 0;
                    frame
                })
                .collect(),
            big_a: Font::new(glyphs(32, 50), &metrics(32)),
            big_b: Font::new(glyphs(32, 51), &metrics(32)),
            big_d: Font::new(glyphs(32, 52), &metrics(32)),
            small: [
                Font::new(glyphs(16, 60), &metrics(16)),
                Font::new(glyphs(16, 61), &metrics(16)),
                Font::new(glyphs(16, 62), &metrics(16)),
            ],
            medium: Font::new(glyphs(9, 63), &metrics(9)),
            menus: texts().menus,
            slider: solid(172, 24, 70),
            knob: solid(10, 24, 71),
        }
    }

    pub(crate) fn texts() -> Texts {
        let metrics = deadrally_gamedata::text::Metrics {
            width: 32,
            height: 32,
            advances: vec![32; 96],
        };
        Texts {
            menus: (0..9)
                .map(|m| {
                    (0..9)
                        .map(|r| {
                            if r < 6 {
                                vec![b'A' + m as u8]
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
            panel: (0..4).map(|i| vec![b'a' + i]).collect(),
            exit_question: b"?".to_vec(),
            yes: b"Y".to_vec(),
            no: b"N".to_vec(),
            big: metrics.clone(),
            small: metrics.clone(),
            medium: metrics,
            configure: configure_texts(),
            hall_of_fame: deadrally_gamedata::text::HallOfFameTexts {
                circuits: vec![b"C".to_vec(); 18],
                cars: vec![b"V".to_vec(); 6],
                difficulties: vec![b"D".to_vec(); 4],
                circuit_order: (0..18).collect(),
            },
            campaign: deadrally_gamedata::text::CampaignTexts {
                driver_names: (0..20)
                    .map(|face| format!("N{face}").into_bytes())
                    .collect(),
                cars: (0..6)
                    .map(|k| deadrally_gamedata::text::CarSpec {
                        price: 500 * (k + 1),
                        upgrades: [1 + k % 4, 2, 4],
                        upgrade_prices: [[100; 4]; 3],
                        repair_price: 10,
                    })
                    .collect(),
                new_game_row: b"A".to_vec(),
                start_racing_row: b"A".to_vec(),
                enter_shop_row: b"S".to_vec(),
                continue_racing_row: b"R".to_vec(),
                text_cursor: vec![0x7F],
                select_difficulty: b"D".to_vec(),
                name_characters: (0..256).map(|c| (32..127).contains(&c)).collect(),
                price: deadrally_gamedata::text::Metrics {
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
            },
            shop: shop_texts(),
        }
    }

    /// Every shop description one line of one letter.
    pub(crate) fn shop_texts() -> deadrally_gamedata::text::ShopTexts {
        let info = || vec![b"i".to_vec(); 6];
        deadrally_gamedata::text::ShopTexts {
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

    /// Configure's texts: one letter each, `k` for key and pad names.
    pub(crate) fn configure_texts() -> deadrally_gamedata::text::ConfigureTexts {
        let one = |c: u8| vec![c];
        deadrally_gamedata::text::ConfigureTexts {
            adjust_music: one(b'm'),
            adjust_effects: one(b'e'),
            gamepad_on: one(b'+'),
            gamepad_off: one(b'-'),
            not_detected: one(b'!'),
            press_any_key: one(b'.'),
            controls: (0..8).map(|i| one(b'0' + i)).collect(),
            key_prompts: (0..8).map(|_| one(b'k')).collect(),
            pad_prompts: (0..7).map(|_| one(b'p')).collect(),
            key_names: (0..256).map(|_| one(b'k')).collect(),
            pad_names: (0..9).map(|_| one(b'p')).collect(),
        }
    }

    #[test]
    fn a_popup_fills_exactly_w_minus_6_columns() {
        // DreeRally fills two columns short on the main menu; the original fills x+2..=x+w-5.
        let mut screen = Canvas::default();
        graphics().popup(&mut screen, 145, 124, 349, 192, Focus::Focused);
        let p = screen.pixels();
        assert_eq!(p[at(147, 200)], POPUP_FILL);
        assert_eq!(
            p[at(489 - 1, 200)],
            POPUP_FILL,
            "x + w - 5 is the right line"
        );
        assert_eq!(p[at(489, 200)], LINE_FOCUSED);
        assert_eq!(p[at(146, 200)], LINE_FOCUSED, "left line at x + 1");
        assert_eq!(p[at(490, 200)], 0, "the shadow columns are left alone");
        assert_eq!(p[at(177, 125)], LINE_FOCUSED, "top line from x + 32");
        assert_eq!(
            p[at(461, 309)],
            LINE_FOCUSED,
            "bottom line at y + h - 7 to x + w - 33"
        );
        assert_eq!(p[at(145, 124)], 11, "top-left corner");
        assert_eq!(p[at(462, 124)], 12, "top-right corner");
        assert_eq!(p[at(145, 296)], 13, "bottom-left corner");
        assert_eq!(p[at(462 + 31, 296 + 19)], 14, "bottom-right corner");
    }

    #[test]
    fn a_focused_menu_shows_the_cursor_and_its_rows_in_three_fonts() {
        let mut screen = Canvas::default();
        let mut menu = MAIN_MENU;
        menu.selected = 2;
        graphics().menu(&mut screen, &menu, Focus::Focused, 7);
        let p = screen.pixels();
        let text_row = |row: usize| p[at(177, 129 + 28 * row)];
        assert_eq!(text_row(2), 50, "selected: big A");
        assert_eq!(text_row(0), 51, "active: big B");
        assert_eq!(text_row(1), 52, "inactive: big D");
        assert_eq!(
            p[at(155, 135 + 56)],
            107,
            "cursor frame 7 at (x + 9, y + 11 + 28 * 2)"
        );
        let mut dim = Canvas::default();
        graphics().menu(&mut dim, &menu, Focus::Unfocused, 7);
        assert_eq!(
            dim.pixels()[at(177, 129 + 56)],
            52,
            "unfocused: everything big D"
        );
        assert_eq!(dim.pixels()[at(155, 191)], POPUP_FILL, "no cursor");
        assert_eq!(dim.pixels()[at(145, 124)], 21, "unfocused corners");
    }

    #[test]
    fn the_cursor_update_copies_its_box_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        graphics().update_cursor(&mut screen, &mut shown, &MAIN_MENU, 3);
        assert_eq!(
            shown.pixels()[at(154, 135)],
            POPUP_FILL,
            "transparent pixel over the fill"
        );
        assert_eq!(shown.pixels()[at(155, 135)], 103);
        assert_eq!(shown.pixels()[at(174, 135)], 0, "only the 20x20 box");
    }

    #[test]
    fn moving_the_highlight_redraws_both_rows_and_copies_both_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        let mut menu = MAIN_MENU;
        graphics().move_highlight(&mut screen, &mut shown, &mut menu, 2, 5, 9);
        assert_eq!(menu.selected, 2);
        let s = shown.pixels();
        assert_eq!(s[at(177, 129)], 51, "the old row in big B");
        assert_eq!(s[at(177, 129 + 56)], 50, "the new row in big A");
        assert_eq!(s[at(155, 135 + 56)], 109, "cursor frame 9");
        assert_eq!(s[at(152, 129 + 28)], 0, "row 1 is not copied");
    }

    #[test]
    fn the_panel_shows_its_last_six_lines_with_the_startup_text_in_small_b() {
        let panel = Panel::startup(&texts());
        let mut screen = Canvas::default();
        graphics().panel_text(&mut screen, &panel);
        let p = screen.pixels();
        assert_eq!(
            p[at(12, 380)],
            graphics().background.pixels[at(12, 380)],
            "line 16 is empty; the restore starts at row 380"
        );
        for k in [1, 2, 3, 5] {
            assert_eq!(p[at(12, 378 + 15 * k)], 61, "line {}: small B", 16 + k);
        }
        assert_eq!(
            p[at(12, 445)],
            graphics().background.pixels[at(12, 445)],
            "line 20 is empty (the glyphs above reach row 438)"
        );
    }
}
