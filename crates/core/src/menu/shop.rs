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
/// The cheat words' scancodes: D, R, A, W; D, R, O, O, L; D, R, I, V, E; D, R, O, P. The
/// shop keeps the last five keys.
const TYPED: usize = 5;
const DRAW: [u8; 4] = [0x20, 0x13, 0x1E, 0x11];
const DROOL: [u8; 5] = [0x20, 0x13, 0x18, 0x18, 0x26];
const DRIVE: [u8; 5] = [0x20, 0x13, 0x17, 0x2F, 0x12];
const DROP: [u8; 4] = [0x20, 0x13, 0x18, 0x19];
/// DROOL's laugh (0x43978B): effect 23 on channel 2 at a fixed volume, lower than the shop's
/// sounds.
const CHEAT_CHANNEL: usize = 2;
const CHEAT_SOUND: u8 = 0x17;
const CHEAT_VOLUME: u32 = 0xF500;
const CHEAT_PITCH: u32 = 0x2_8000 - 0x7000;
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
/// Effect 26 sounds as the selection moves; 28 a purchase, 31 a repair, 24 the way on, and
/// on channel 2 effect 23 when the money is short.
const STEP_SOUND: u8 = 26;
const BUY_SOUND: u8 = 28;
const REPAIR_SOUND: u8 = 31;
const ON_SOUND: u8 = 24;
const SHORT_CHANNEL: usize = 2;
const SHORT_SOUND: u8 = 23;
const SHORT_PITCH: u32 = 0x2_5500;
/// After a purchase or a short-of-money message the description comes back 310 passes later
/// (`framesToWaitAfterBuy` 0x456B70).
const MESSAGE_PASSES: u32 = 310;
/// The voice when a car is bought (channel 5, effect 4).
const CAR_VOICE: u8 = 4;
/// The turning loops: engine 24 frames, tires 12, armour 16 back and forth, repair 24 (not
/// 23 as DreeRally has it: 0x43963A), continue 23, the car 64.
const ENGINE_FRAMES: usize = 24;
const TIRE_FRAMES: usize = 12;
const ARMOUR_LAST: usize = 15;
const REPAIR_FRAMES: usize = 24;
pub(super) const CONTINUE_FRAMES: usize = 23;
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
    /// The continue item's flag (0x4611D0), the shop's and the market's.
    pub(super) continue_frame: usize,
    /// Passes until the selected item's description comes back (0x456B70).
    pub(super) message_passes: u32,
    /// The Underground Market's selection (0x461278).
    pub(super) market: usize,
    /// Whether the continue item has been drawn selected since the game started (0x456B84,
    /// `reloadContinueAnimation` 0x428FD0): the results then do not fade out.
    pub(super) continue_seen: bool,
    /// Whether the market was left by Escape (0x456B60): the shop then brings its music's
    /// order and volume back.
    pub(super) market_escaped: bool,
    /// The last keys typed in the shop, the latest last, for its cheat words (0x4396B0).
    typed: [u8; TYPED],
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
            message_passes: 0,
            market: CONTINUE,
            continue_seen: false,
            market_escaped: false,
            typed: [0; TYPED],
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
            message_passes: self.message_passes,
            market: self.market,
            continue_seen: self.continue_seen,
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
        self.shop.typed = [0; TYPED];
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
        self.shop.continue_seen |= self.shop.selected == CONTINUE;
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
        let next = self.shop_pass();
        if next == (State::Shop { second: false })
            && let Some(quick) = self.quick_keys()
        {
            return self.shop_after_quick(quick);
        }
        next
    }

    /// After a quick save or load (0x439891): the shop drawn afresh at full brightness, the
    /// selected item's border and box, the panel, then the confirmation.
    fn shop_after_quick(&mut self, quick: super::slots::Quick) -> State {
        self.screen
            .restore(&self.graphics.background, at(0, 96), 640, 267);
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_shop(&mut screen);
        self.screen = screen;
        self.compose_palette();
        self.palette.show_composed(32..256);
        for item in [CONTINUE, CAR, ENGINE, TIRES, ARMOUR, REPAIR] {
            self.remove_item_border(item);
        }
        let selected = self.shop.selected;
        let mut screen = std::mem::take(&mut self.screen);
        self.item_border(&mut screen, selected);
        self.screen = screen;
        self.redraw_item(selected);
        let mut screen = std::mem::take(&mut self.screen);
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut screen, &self.panel);
        self.screen = screen;
        self.shown = self.screen.clone();
        self.quick_confirm(quick, false)
    }

    /// The rest of a pass: a message giving way, the selected item's loop, the key.
    fn shop_pass(&mut self) -> State {
        self.shop.message_passes = self.shop.message_passes.saturating_sub(1);
        self.turn_selected();
        if self.shop.message_passes == 1 && self.shop.selected <= ARMOUR {
            self.redraw_item(self.shop.selected);
            self.shown = self.screen.clone();
        }
        let key = self.keys.take();
        if key != 0 {
            self.cheat(key);
        }
        match key {
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
            keys::ENTER | 0x9C => return self.enter_item(),
            keys::ESCAPE => return self.leave_shop(),
            _ => {}
        }
        State::Shop { second: false }
    }

    /// The shop's cheat words (0x4396B0), typed as scancodes and kept with the four keys before:
    /// DRAW gives $1000, DROOL makes the money $500000 with a laugh, DRIVE and DROP give and
    /// take 10 points and sort the standings afresh; the side panel shows the change. The key
    /// then does what it does in the shop.
    fn cheat(&mut self, key: u8) {
        let typed = &mut self.shop.typed;
        typed.rotate_left(1);
        typed[TYPED - 1] = key;
        let typed = *typed;
        let player = self.campaign.player_mut();
        if typed.ends_with(&DRAW) {
            player.money += 1000;
        } else if typed.ends_with(&DROOL) {
            player.money = 500_000;
            self.sound
                .trigger_at(CHEAT_CHANNEL, CHEAT_SOUND, CHEAT_VOLUME, CHEAT_PITCH);
        } else if typed.ends_with(&DRIVE) || typed.ends_with(&DROP) {
            player.points += if typed.ends_with(&DRIVE) { 10 } else { -10 };
            self.campaign.rank_drivers();
        } else {
            return;
        }
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_side_panel(&mut screen);
        self.screen = screen;
        self.shown = self.screen.clone();
    }

    /// `enterShop` (0x4373B0) on the selected item.
    fn enter_item(&mut self) -> State {
        match self.shop.selected {
            ENGINE | TIRES | ARMOUR => self.buy_upgrade(self.shop.selected - ENGINE),
            REPAIR => self.buy_repair(),
            CONTINUE => return self.go_on(),
            _ => return self.offer_car(),
        }
        State::Shop { second: false }
    }

    /// What the dealer gives for the player's car (`enterShop`, 0x4373B0): its trade-in
    /// value rounded down to tens.
    fn refund(&self) -> i32 {
        let refund = i64::from(self.trade_in());
        // `itoa`, its last digit made '0', `atoi`.
        (refund - refund % 10) as i32
    }

    /// A quarter of the car's worth rounded up, less the damage's repair, never below 0.
    pub(super) fn trade_in(&self) -> i32 {
        let player = self.campaign.player();
        let spec = self.assets.menu.texts.campaign.cars[player.car as usize];
        let quarter = (i64::from(player.car_price) + 3) / 4;
        let damage = i64::from(spec.repair_price / 10) * i64::from(player.damage);
        let damage = if self.campaign.use_weapons {
            damage / 2
        } else {
            damage
        };
        (quarter - damage).max(0) as i32
    }

    /// Enter on the car box: the offer in the popup, "yes" and "no" under it.
    fn offer_car(&mut self) -> State {
        let refund = self.refund();
        let car = self.shop.car;
        let price = self.assets.menu.texts.campaign.cars[car].price;
        let due = price - refund;
        if self.short_of(due) {
            return State::Shop { second: false };
        }
        self.sound(BUY_SOUND);
        let texts = &self.assets.menu.texts;
        let offer = &texts.shop.offer;
        let name = texts.hall_of_fame.cars[car].clone();
        let join = |parts: &[&[u8]]| parts.concat();
        let refund_text = refund.to_string().into_bytes();
        let due_text = due.unsigned_abs().to_string().into_bytes();
        let lines = if due >= 0 {
            [
                join(&[&offer[0], &refund_text, &offer[1]]),
                offer[2].clone(),
                join(&[&name, &offer[6], b"$", &due_text]),
                offer[7].clone(),
            ]
        } else {
            [
                join(&[&offer[0], &refund_text, &offer[1]]),
                offer[2].clone(),
                join(&[&offer[3], b"$", &due_text]),
                join(&[&offer[4], &name, &offer[5]]),
            ]
        };
        let (x, y, w, h) = INFO;
        self.graphics
            .popup(&mut self.screen, x, y, w, h, Focus::Focused);
        for (line, text) in lines.iter().enumerate() {
            self.graphics.small[2].draw(
                &mut self.screen,
                text,
                at(INFO_TEXT.0, INFO_TEXT.1 + 16 * line),
            );
        }
        self.draw_offer_answers(true);
        self.shown = self.screen.clone();
        State::CarOffer {
            second: false,
            yes: true,
        }
    }

    /// The offer's "yes" and "no", the selected one in big A.
    fn draw_offer_answers(&mut self, yes: bool) {
        let texts = &self.assets.menu.texts;
        let (yes_font, no_font) = if yes {
            (&self.graphics.big_a, &self.graphics.big_b)
        } else {
            (&self.graphics.big_b, &self.graphics.big_a)
        };
        yes_font.draw(&mut self.screen, &texts.yes, at(240, 185));
        no_font.draw(&mut self.screen, &texts.no, at(410, 185));
    }

    /// The car box's car turning a frame, shown.
    fn turn_car_box(&mut self) {
        let image = self.assets.menu.car_turning[self.shop.car][self.car_frame].clone();
        self.screen.draw(&image, at(CAR_TURN.0, CAR_TURN.1), false);
        self.shown
            .copy_from(&self.screen, at(CAR_TURN.0, CAR_TURN.1), 96, 64);
        self.car_frame = (self.car_frame + 1) % CAR_FRAMES;
    }

    /// A wait of the offer; after the second the car turns, the cursor beside the selected
    /// answer turns, then the key.
    pub(super) fn car_offer_tick(&mut self, second: bool, yes: bool) -> State {
        self.palette.after_wait();
        if !second {
            return State::CarOffer { second: true, yes };
        }
        self.turn_car_box();
        let cursor_at = at(if yes { 217 } else { 387 }, 192);
        self.screen.fill(cursor_at, 20, 20, super::draw::POPUP_FILL);
        let cursor = self.graphics.cursor(self.cursor).clone();
        self.screen.draw(&cursor, cursor_at, true);
        self.shown.copy_from(&self.screen, at(165, 192), 335, 28);
        self.cursor = (self.cursor + 1) % super::draw::CURSOR_FRAMES;
        let key = match self.keys.take() {
            keys::Y => keys::LEFT,
            keys::N => keys::RIGHT,
            key => key,
        };
        match key {
            keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT => {
                let left = matches!(key, keys::LEFT | keys::PAD_LEFT);
                if left != yes {
                    self.sound(super::MOVE_SOUND);
                }
                self.screen
                    .fill(at(168, 192), 335, 25, super::draw::POPUP_FILL);
                self.draw_offer_answers(left);
                State::CarOffer {
                    second: false,
                    yes: left,
                }
            }
            keys::ENTER | 0x9C => {
                self.sound(BUY_SOUND);
                if yes {
                    self.buy_car()
                } else {
                    self.offer_declined()
                }
            }
            keys::ESCAPE => self.offer_declined(),
            _ => State::CarOffer { second: false, yes },
        }
    }

    fn offer_declined(&mut self) -> State {
        self.redraw_item(CAR);
        self.shown = self.screen.clone();
        State::Shop { second: false }
    }

    /// "Yes": the car traded in (its upgrades and damage gone, the refund paid out), every box
    /// redrawn, then the paint.
    fn buy_car(&mut self) -> State {
        self.sound.trigger_at(
            super::licence::VOICE_CHANNEL,
            CAR_VOICE,
            self.config.effects_volume(),
            super::licence::VOICE_PITCH,
        );
        let refund = self.refund();
        let car = self.shop.car;
        let price = self.assets.menu.texts.campaign.cars[car].price;
        let player = self.campaign.player_mut();
        player.car = car as i32;
        player.money += refund - price;
        player.car_price = price;
        player.damage = 0;
        player.engine = 0;
        player.tires = 0;
        player.armour = 0;
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_side_panel(&mut screen);
        for item in [ENGINE, ARMOUR, TIRES, REPAIR, CAR] {
            self.draw_item(&mut screen, item);
        }
        let (x, y, w, h) = INFO;
        self.graphics.popup(&mut screen, x, y, w, h, Focus::Focused);
        screen.draw(&self.assets.menu.colour_slider, at(188, 195), true);
        let paint = self.assets.menu.texts.shop.paint.clone();
        for (line, text) in paint.iter().enumerate() {
            self.graphics.small[2].draw(
                &mut screen,
                text,
                at(INFO_TEXT.0, INFO_TEXT.1 + 16 * line),
            );
        }
        self.screen = screen;
        self.shown = self.screen.clone();
        self.paint_pass()
    }

    /// The paint loop's start: a key read and acted on, the slider and knob drawn.
    fn paint_pass(&mut self) -> State {
        let key = self.keys.take();
        let colour = &mut self.campaign.player_mut().colour;
        match key {
            keys::LEFT | keys::PAD_LEFT if *colour > 0 => *colour -= 2,
            keys::RIGHT | keys::PAD_RIGHT if *colour < 253 => *colour += 2,
            _ => {}
        }
        let colour = self.campaign.player().colour;
        self.palette.set_player_ramp(self.player_copper());
        self.screen
            .fill(at(182, 191), 294, 24, super::draw::POPUP_FILL);
        let menu = &self.assets.menu;
        self.screen.draw(&menu.colour_slider, at(188, 195), true);
        self.screen
            .draw(&menu.colour_knob, at(202 + colour as usize, 191), true);
        State::CarPaint { second: false, key }
    }

    /// A wait of the paint loop; after the second the knob is shown, the car turns, and
    /// Enter ends the paint.
    pub(super) fn car_paint_tick(&mut self, second: bool, key: u8) -> State {
        self.palette.after_wait();
        if !second {
            return State::CarPaint { second: true, key };
        }
        let colour = self.campaign.player().colour as usize;
        self.shown
            .copy_from(&self.screen, at(200 + colour, 191), 14, 24);
        self.turn_car_box();
        if matches!(key, keys::ENTER | 0x9C) {
            return self.car_bought();
        }
        self.paint_pass()
    }

    /// `showCarBought` (0x4210C0): the car box on the next car, the popup saying what was
    /// bought; the description comes back later.
    fn car_bought(&mut self) -> State {
        let bought = self.campaign.player().car as usize;
        self.shop.car = (bought + 1).min(5);
        let menu = &self.assets.menu;
        let car = self.shop.car;
        self.screen
            .draw(&menu.car_box, at(CAR_BOX.0, CAR_BOX.1), false);
        self.screen
            .draw(&menu.car_names[car], at(CAR_BOX.0, CAR_BOX.1), false);
        self.screen.draw(
            &menu.car_turning[car][self.car_frame],
            at(CAR_TURN.0, CAR_TURN.1),
            false,
        );
        let price = dollars(menu.texts.campaign.cars[car].price);
        draw_price(&mut self.screen, menu, &price, CAR_BOX.0, CAR_PRICE_Y);
        let info = menu.texts.shop.car_bought[bought].clone();
        let mut screen = std::mem::take(&mut self.screen);
        self.info_popup(&mut screen, &info);
        let arrows = &self.assets.menu.car_arrows;
        screen.draw(&arrows[0], at(ARROWS[0].0, ARROWS[0].1), true);
        screen.draw(&arrows[1], at(ARROWS[1].0, ARROWS[1].1), true);
        self.screen = screen;
        self.shop.message_passes = MESSAGE_PASSES;
        self.shown = self.screen.clone();
        State::Shop { second: false }
    }

    /// An engine, tire or armour upgrade: the money checked, paid and added to the car's
    /// worth, the next level shown with what was bought, the level raised.
    fn buy_upgrade(&mut self, kind: usize) {
        let player = *self.campaign.player();
        let spec = self.assets.menu.texts.campaign.cars[player.car as usize];
        let level = [player.engine, player.tires, player.armour][kind];
        if level >= spec.upgrades[kind] {
            self.redraw_item(kind + ENGINE);
            self.shown = self.screen.clone();
            return;
        }
        let cost = spec.upgrade_prices[kind][level as usize];
        if self.short_of(cost) {
            return;
        }
        self.sound(BUY_SOUND);
        let player = self.campaign.player_mut();
        player.money -= cost;
        player.car_price = player.car_price.wrapping_add(cost);
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_bought(&mut screen, kind, level as usize);
        let player = self.campaign.player_mut();
        match kind {
            0 => player.engine += 1,
            1 => player.tires += 1,
            _ => player.armour += 1,
        }
        self.shop.message_passes = MESSAGE_PASSES;
        self.draw_side_panel(&mut screen);
        self.screen = screen;
        self.shown = self.screen.clone();
    }

    /// The box after buying from `level`: the next level turning with its price, or the
    /// "no more" picture; the popup says what was bought.
    fn draw_bought(&mut self, canvas: &mut Canvas, kind: usize, level: usize) {
        let menu = &self.assets.menu;
        let player = *self.campaign.player();
        let spec = menu.texts.campaign.cars[player.car as usize];
        let count = spec.upgrades[kind].max(1) as usize;
        let x = ITEM_X[kind];
        canvas.draw(&menu.item_boxes[kind], at(x, ITEM_BOX_Y), false);
        if level + 1 >= count {
            canvas.draw(&menu.maxed[4 * kind + count - 1], at(x, ITEM_Y), false);
        } else {
            let (frames, frame) = match kind {
                0 => (&menu.engines[level + 1], self.shop.engine_frame),
                1 => (&menu.tires[level + 1], self.shop.tire_frame),
                _ => (&menu.armours[level + 1], self.shop.armour_frame),
            };
            canvas.draw(&frames[frame], at(x, ITEM_Y), false);
            let price = dollars(spec.upgrade_prices[kind][level + 1]);
            draw_price(canvas, menu, &price, x, ITEM_PRICE_Y);
        }
        let info = menu.texts.shop.bought[kind][level].clone();
        self.info_popup(canvas, &info);
    }

    pub(super) fn info_popup(&self, canvas: &mut Canvas, info: &[Vec<u8>]) {
        let (x, y, w, h) = INFO;
        self.graphics.popup(canvas, x, y, w, h, Focus::Focused);
        for (line, text) in info.iter().enumerate() {
            self.graphics
                .write_text(canvas, text, at(INFO_TEXT.0, INFO_TEXT.1 + 16 * line));
        }
    }

    /// `hasInsuficientMoneyToBuy` (0x421E50): with less money than `cost`, the popup's lines
    /// under its title say how much is missing, and the description comes back later.
    pub(super) fn short_of(&mut self, cost: i32) -> bool {
        let money = self.campaign.player().money;
        if money >= cost {
            return false;
        }
        self.screen.fill(
            at(INFO_TEXT.0, INFO_TEXT.1 + 16),
            347,
            80,
            super::draw::POPUP_FILL,
        );
        let short = &self.assets.menu.texts.shop.short;
        let mut line = short[0].clone();
        line.extend(
            i64::from(cost)
                .saturating_sub(i64::from(money))
                .to_string()
                .bytes(),
        );
        line.extend(&short[1]);
        let (above, below) = (short[2].clone(), short[3].clone());
        let x = INFO_TEXT.0;
        self.graphics
            .write_text(&mut self.screen, &above, at(x, INFO_TEXT.1 + 32));
        self.graphics
            .write_text(&mut self.screen, &line, at(x, INFO_TEXT.1 + 48));
        self.graphics
            .write_text(&mut self.screen, &below, at(x, INFO_TEXT.1 + 64));
        self.shown = self.screen.clone();
        self.sound.trigger_at(
            SHORT_CHANNEL,
            SHORT_SOUND,
            self.config.effects_volume(),
            SHORT_PITCH,
        );
        self.shop.message_passes = MESSAGE_PASSES;
        true
    }

    /// A repair: ten points of damage (what is left under ten) for the price the box shows.
    fn buy_repair(&mut self) {
        let player = *self.campaign.player();
        let full = self.assets.menu.texts.campaign.cars[player.car as usize].repair_price;
        let weapons = self.campaign.use_weapons;
        let cost = if player.damage < 10 {
            let step = full / 10;
            let cost = player.damage * step;
            if weapons { cost / 2 } else { cost }
        } else if weapons {
            full / 2
        } else {
            full
        };
        if player.damage <= 0 || self.short_of(cost) {
            return;
        }
        self.sound(REPAIR_SOUND);
        let player = self.campaign.player_mut();
        player.damage = if player.damage < 10 {
            0
        } else {
            player.damage - 10
        };
        player.money -= cost;
        player.car_price = player.car_price.wrapping_add(cost);
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_side_panel(&mut screen);
        self.draw_item(&mut screen, REPAIR);
        self.screen = screen;
        self.shown = self.screen.clone();
    }

    /// The continue item: a wreck cannot race without weapons; with weapons the
    /// Underground Market comes first (0x4385FA), without them the sign-up.
    fn go_on(&mut self) -> State {
        let player = *self.campaign.player();
        if player.damage == 100 && !self.campaign.use_weapons {
            let lines = self.assets.menu.texts.shop.wrecked.clone();
            let mut screen = std::mem::take(&mut self.screen);
            self.info_popup(&mut screen, &lines);
            self.screen = screen;
            self.shown = self.screen.clone();
            self.sound.trigger_at(
                SHORT_CHANNEL,
                SHORT_SOUND,
                self.config.effects_volume(),
                SHORT_PITCH,
            );
            return State::Shop { second: false };
        }
        if self.campaign.use_weapons {
            return self.open_market();
        }
        self.sound(ON_SOUND);
        // 0x438643: a leader meets the Adversary instead of signing up.
        if self.campaign.player_leads() {
            return self.open_adversary();
        }
        self.open_sign_up()
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
