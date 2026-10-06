//! The shop (spec M3b §3): `postLoadedOrLicense` (0x4387D0) with its screen
//! (`drawShopAnimationAndRightSide` 0x429100), the six items' boxes (`reloadCarAnimation2`
//! 0x420250, `reloadEngineAnimation2` 0x4204F0, `reloadTireAnimation2` 0x4207E0,
//! `reloadArmourAnimation2` 0x420A80, `reloadRepairAnimation` 0x420D20,
//! `reloadContinueAnimation` 0x428FD0) and the moves between them (0x421D90, 0x421DF0,
//! 0x42D8C0, 0x42DAB0, the car's turn `sub_42D780`). DreeRally `ui/shopScreen.c`,
//! `ui/util/anim.c`; dRally `___25330h.c`.

use super::draw::Focus;
use super::hall_of_fame::Wipe;
use super::licence::draw_price;
use super::{Menu, State};
use crate::canvas::{Canvas, at};
use crate::keys;

/// The items, `menuOptionSelected_463DF0`.
pub(crate) const CAR: usize = 0;
pub(crate) const ENGINE: usize = 1;
pub(crate) const TIRES: usize = 2;
pub(crate) const ARMOUR: usize = 3;
pub(crate) const REPAIR: usize = 4;
pub(crate) const CONTINUE: usize = 5;
/// The item boxes' left edges along the bottom row, engine to continue.
const ITEM_X: [usize; 5] = [16, 120, 224, 328, 432];
const ITEM_BOX_Y: usize = 253;
const ITEM_Y: usize = 269;
const ITEM_PRICE_Y: usize = 335;
/// Their borders, and the car's.
const ITEM_BORDER_X: [usize; 5] = [10, 114, 218, 322, 426];
const ITEM_BORDER: (usize, usize, usize) = (243, 108, 114);
const CAR_BORDER: (usize, usize, usize, usize) = (0, 115, 128, 114);
/// The car box, its turning car, its price and arrows.
const CAR_BOX: (usize, usize) = (16, 125);
const CAR_TURN: (usize, usize) = (16, 141);
const CAR_PRICE_Y: usize = 207;
const ARROWS: [(usize, usize); 2] = [(0, 141), (112, 141)];
/// The description popup and its six lines.
const INFO: (usize, usize, usize, usize) = (144, 114, 384, 119);
const INFO_TEXT: (usize, usize) = (170, 124);
/// The repair box's damage, right-aligned in the medium font.
const REPAIR_TEXT: (usize, usize) = (410, 255);
/// Effect 26 sounds as the selection moves.
const STEP_SOUND: u8 = 26;
/// The turning loops: engine 24 frames, tires 12, armour 16 back and forth, repair 24 (not
/// 23 as DreeRally has it: 0x43963A), continue 23, the car 64.
const ENGINE_FRAMES: usize = 24;
const TIRE_FRAMES: usize = 12;
const ARMOUR_LAST: usize = 15;
const REPAIR_FRAMES: usize = 24;
const CONTINUE_FRAMES: usize = 23;
const CAR_FRAMES: usize = 64;

/// The shop's selection and its animations' frames (globals of the original).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Shop {
    pub(crate) selected: usize,
    /// The car the car box shows (`actualCarSelected` 0x45FC30): 1 after `initDrivers`.
    pub(crate) car: usize,
    engine_frame: usize,
    tire_frame: usize,
    armour_frame: usize,
    armour_back: bool,
    repair_frame: usize,
    continue_frame: usize,
}

impl Default for Shop {
    fn default() -> Shop {
        Shop {
            selected: CONTINUE,
            car: 1,
            engine_frame: 0,
            tire_frame: 0,
            armour_frame: 0,
            armour_back: false,
            repair_frame: 0,
            continue_frame: 0,
        }
    }
}

impl Shop {
    /// What `initDrivers` resets: the car box on car 1, every loop at its first frame but
    /// the repair's.
    pub(crate) fn reset(&mut self) {
        let repair_frame = self.repair_frame;
        *self = Shop {
            selected: self.selected,
            repair_frame,
            ..Shop::default()
        };
    }
}

/// `n` as the shop prints a price.
fn dollars(n: i32) -> Vec<u8> {
    format!("${n}").into_bytes()
}

impl Menu {
    /// `postLoadedOrLicense` for a game in progress: the shop drawn over a copy of the
    /// screen and wiped in, the continue item selected.
    pub(super) fn open_shop(&mut self) -> State {
        self.shop.selected = CONTINUE;
        let mut back = self.screen.clone();
        back.restore(&self.graphics.background, at(0, 96), 640, 267);
        self.draw_shop(&mut back);
        self.back = back;
        self.palette.set_player_ramp(self.player_copper());
        State::Wipe {
            wipe: Wipe::Shop,
            step: 0,
        }
    }

    /// `drawShopAnimationAndRightSide`: the title, the side panel, the continue item's
    /// border, then every item's box, each drawing its description over the last.
    pub(super) fn draw_shop(&mut self, canvas: &mut Canvas) {
        canvas.draw(&self.assets.menu.shop_title, at(0, 92), true);
        self.draw_side_panel(canvas);
        self.item_border(canvas, CONTINUE);
        for item in [CAR, ENGINE, TIRES, ARMOUR, REPAIR, CONTINUE] {
            self.draw_item(canvas, item);
        }
    }

    fn item_border(&self, canvas: &mut Canvas, item: usize) {
        if item == CAR {
            let (x, y, w, h) = CAR_BORDER;
            self.border(canvas, x, y, w, h);
        } else {
            let (y, w, h) = ITEM_BORDER;
            self.border(canvas, ITEM_BORDER_X[item - 1], y, w, h);
        }
    }

    /// `removeBorder` around an item.
    fn remove_item_border(&mut self, item: usize) {
        let (x, y, w, h) = if item == CAR {
            CAR_BORDER
        } else {
            let (y, w, h) = ITEM_BORDER;
            (ITEM_BORDER_X[item - 1], y, w, h)
        };
        let background = &self.graphics.background;
        self.screen.restore(background, at(x, y), w, 5);
        self.screen.restore(background, at(x, y + h - 5), w, 5);
        self.screen.restore(background, at(x, y), 5, h);
        self.screen.restore(background, at(x + w - 5, y), 5, h);
    }

    /// An item's box with its description in the popup above.
    fn draw_item(&mut self, canvas: &mut Canvas, item: usize) {
        let player = *self.campaign.player();
        let menu = &self.assets.menu;
        let shop = &menu.texts.shop;
        let weapons = usize::from(self.campaign.use_weapons);
        let car = player.car as usize;
        let spec = menu.texts.campaign.cars[car];
        let mut price = None;
        let info = match item {
            CAR => {
                let shown = self.shop.car;
                canvas.draw(&menu.car_box, at(CAR_BOX.0, CAR_BOX.1), false);
                canvas.draw(&menu.car_names[shown], at(CAR_BOX.0, CAR_BOX.1), false);
                canvas.draw(
                    &menu.car_turning[shown][self.car_frame],
                    at(CAR_TURN.0, CAR_TURN.1),
                    false,
                );
                let cost = menu.texts.campaign.cars[shown].price;
                draw_price(canvas, menu, &dollars(cost), CAR_BOX.0, CAR_PRICE_Y);
                shop.cars[shown][weapons].clone()
            }
            ENGINE | TIRES | ARMOUR => {
                let kind = item - ENGINE;
                let x = ITEM_X[kind];
                canvas.draw(&menu.item_boxes[kind], at(x, ITEM_BOX_Y), false);
                let level = [player.engine, player.tires, player.armour][kind].max(0) as usize;
                let count = spec.upgrades[kind].max(1) as usize;
                if level >= count {
                    let maxed = &menu.maxed[4 * kind + count - 1];
                    canvas.draw(maxed, at(x, ITEM_Y), false);
                    [&shop.engine_max, &shop.tire_max, &shop.armour_max][kind].clone()
                } else {
                    let (frames, frame) = match kind {
                        0 => (&menu.engines[level], self.shop.engine_frame),
                        1 => (&menu.tires[level], self.shop.tire_frame),
                        _ => (&menu.armours[level], self.shop.armour_frame),
                    };
                    canvas.draw(&frames[frame], at(x, ITEM_Y), false);
                    price = Some((dollars(spec.upgrade_prices[kind][level]), x));
                    match kind {
                        0 => shop.engines[car][level].clone(),
                        1 => shop.tires[level].clone(),
                        _ => shop.armours[level].clone(),
                    }
                }
            }
            REPAIR => {
                let x = ITEM_X[3];
                canvas.draw(&menu.item_boxes[3], at(x, ITEM_BOX_Y), false);
                canvas.draw(&menu.repair[self.shop.repair_frame], at(x, ITEM_Y), false);
                let full = spec.repair_price;
                let (text, cost) = if player.damage >= 10 {
                    let cost = if self.campaign.use_weapons {
                        full >> 1
                    } else {
                        full
                    };
                    (shop.repair_ten.clone(), cost)
                } else {
                    let step = if self.campaign.use_weapons {
                        full / 10 / 2
                    } else {
                        full / 10
                    };
                    (player.damage.to_string().into_bytes(), player.damage * step)
                };
                let medium = &self.graphics.medium;
                let right = REPAIR_TEXT.0 - medium.width(&text).min(REPAIR_TEXT.0);
                medium.draw(canvas, &text, at(right, REPAIR_TEXT.1));
                price = Some((dollars(cost), x));
                let step = match player.damage {
                    d if d >= 100 => 0,
                    0 => 11,
                    d if d < 10 => 10,
                    d => (10 - d / 10) as usize,
                };
                shop.repairs[step].clone()
            }
            _ => {
                let x = ITEM_X[4];
                canvas.draw(&menu.item_boxes[4], at(x, ITEM_BOX_Y), false);
                canvas.draw(
                    &menu.continue_flag[self.shop.continue_frame],
                    at(x, ITEM_Y),
                    false,
                );
                shop.continues[weapons].clone()
            }
        };
        if let Some((text, x)) = price {
            draw_price(canvas, menu, &text, x, ITEM_PRICE_Y);
        }
        let (x, y, w, h) = INFO;
        self.graphics.popup(canvas, x, y, w, h, Focus::Focused);
        for (line, text) in info.iter().enumerate() {
            self.graphics
                .write_text(canvas, text, at(INFO_TEXT.0, INFO_TEXT.1 + 16 * line));
        }
        if item == CAR {
            let arrows = &self.assets.menu.car_arrows;
            canvas.draw(&arrows[0], at(ARROWS[0].0, ARROWS[0].1), true);
            canvas.draw(&arrows[1], at(ARROWS[1].0, ARROWS[1].1), true);
        }
    }

    /// An item's box redrawn on the screen.
    fn redraw_item(&mut self, item: usize) {
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_item(&mut screen, item);
        self.screen = screen;
    }

    /// After the wipe: the shop's first pass.
    pub(super) fn shop_shown(&mut self) -> State {
        self.shown = self.screen.clone();
        State::Shop { second: false }
    }

    /// A wait of the shop's loop; after the second, the selected item turns a frame and the
    /// key is read.
    pub(super) fn shop_tick(&mut self, second: bool) -> State {
        self.palette.after_wait();
        if !second {
            return State::Shop { second: true };
        }
        self.turn_selected();
        match self.keys.take() {
            keys::UP | keys::PAD_UP => {
                if self.shop.selected == ENGINE {
                    self.sound(STEP_SOUND);
                    self.select(CAR);
                }
            }
            keys::DOWN | keys::PAD_DOWN => {
                if self.shop.selected == CAR {
                    self.sound(STEP_SOUND);
                    self.select(ENGINE);
                }
            }
            keys::LEFT | keys::PAD_LEFT => {
                self.sound(STEP_SOUND);
                match self.shop.selected {
                    CAR => {
                        self.shop.car = (self.shop.car + 5) % 6;
                        return self.turn_car(false);
                    }
                    item => self.select(item - 1),
                }
            }
            keys::RIGHT | keys::PAD_RIGHT => {
                if self.shop.selected < CONTINUE {
                    self.sound(STEP_SOUND);
                }
                match self.shop.selected {
                    CAR => {
                        self.shop.car = (self.shop.car + 1) % 6;
                        return self.turn_car(true);
                    }
                    CONTINUE => {}
                    item => self.select(item + 1),
                }
            }
            keys::ENTER | 0x9C => {
                // Buying comes with M3c; the continue item leads on to the sign-up.
                if self.shop.selected == CONTINUE {
                    return self.open_sign_up();
                }
            }
            keys::ESCAPE => return self.leave_shop(),
            _ => {}
        }
        State::Shop { second: false }
    }

    /// The selection moves to `item`: its box and description drawn, the border moved.
    fn select(&mut self, item: usize) {
        let from = self.shop.selected;
        self.shop.selected = item;
        self.redraw_item(item);
        self.remove_item_border(from);
        let mut screen = std::mem::take(&mut self.screen);
        self.item_border(&mut screen, item);
        self.screen = screen;
        self.shown = self.screen.clone();
    }

    /// The selected item's loop turns a frame, shown.
    fn turn_selected(&mut self) {
        let menu = &self.assets.menu;
        let player = *self.campaign.player();
        let spec = menu.texts.campaign.cars[player.car as usize];
        let (image, x) = match self.shop.selected {
            CAR => {
                let image = &menu.car_turning[self.shop.car][self.car_frame];
                self.car_frame = (self.car_frame + 1) % CAR_FRAMES;
                (image, CAR_TURN.0)
            }
            ENGINE if player.engine < spec.upgrades[0] => {
                let image = &menu.engines[player.engine as usize][self.shop.engine_frame];
                self.shop.engine_frame = (self.shop.engine_frame + 1) % ENGINE_FRAMES;
                (image, ITEM_X[0])
            }
            TIRES if player.tires < spec.upgrades[1] => {
                let image = &menu.tires[player.tires as usize][self.shop.tire_frame];
                self.shop.tire_frame = (self.shop.tire_frame + 1) % TIRE_FRAMES;
                (image, ITEM_X[1])
            }
            ARMOUR if player.armour < spec.upgrades[2] => {
                let image = &menu.armours[player.armour as usize][self.shop.armour_frame];
                let shop = &mut self.shop;
                if shop.armour_back {
                    shop.armour_frame -= 1;
                    if shop.armour_frame < 1 {
                        shop.armour_back = false;
                    }
                } else {
                    shop.armour_frame += 1;
                    if shop.armour_frame >= ARMOUR_LAST {
                        shop.armour_back = true;
                    }
                }
                (image, ITEM_X[2])
            }
            REPAIR => {
                let image = &menu.repair[self.shop.repair_frame];
                self.shop.repair_frame = (self.shop.repair_frame + 1) % REPAIR_FRAMES;
                (image, ITEM_X[3])
            }
            CONTINUE => {
                let image = &menu.continue_flag[self.shop.continue_frame];
                self.shop.continue_frame = (self.shop.continue_frame + 1) % CONTINUE_FRAMES;
                (image, ITEM_X[4])
            }
            _ => return,
        };
        let y = if self.shop.selected == CAR {
            CAR_TURN.1
        } else {
            ITEM_Y
        };
        let image = image.clone();
        self.screen.draw(&image, at(x, y), false);
        self.shown.copy_from(&self.screen, at(x, y), 96, 64);
    }

    /// Left or Right on the car box: the next car shown with its arrow lit, then four turns
    /// of two waits each (drawn, not shown) before the arrow goes out.
    fn turn_car(&mut self, right: bool) -> State {
        self.redraw_item(CAR);
        let (frame, (x, y)) = if right {
            (3, ARROWS[1])
        } else {
            (2, ARROWS[0])
        };
        let lit = self.assets.menu.car_arrows[frame].clone();
        self.screen.draw(&lit, at(x, y), true);
        self.shown = self.screen.clone();
        if !right {
            self.redraw_item(CAR);
        }
        State::CarTurn { right, waits: 0 }
    }

    /// A wait of the car's turn (`sub_42D780`).
    pub(super) fn car_turn_tick(&mut self, right: bool, waits: u32) -> State {
        self.palette.after_wait();
        let waits = waits + 1;
        if waits.is_multiple_of(2) {
            let image = self.assets.menu.car_turning[self.shop.car][self.car_frame].clone();
            self.screen.draw(&image, at(CAR_TURN.0, CAR_TURN.1), false);
            self.car_frame = (self.car_frame + 1) % CAR_FRAMES;
        }
        if waits < 8 {
            return State::CarTurn { right, waits };
        }
        let (frame, (x, y)) = if right {
            (1, ARROWS[1])
        } else {
            (0, ARROWS[0])
        };
        let arrow = self.assets.menu.car_arrows[frame].clone();
        self.screen.draw(&arrow, at(x, y), true);
        self.shown = self.screen.clone();
        State::Shop { second: false }
    }

    /// Escape: the shop's screen gives way to the background and the panel, the Start
    /// Racing menu wipes in over it.
    fn leave_shop(&mut self) -> State {
        let mut back = std::mem::take(&mut self.back);
        back.copy_all(&self.graphics.background);
        self.graphics.panel_frame(&mut back, 0, 371, 639, 109);
        self.graphics.panel_text(&mut back, &self.panel);
        self.back = back;
        self.start_wipe()
    }
}
