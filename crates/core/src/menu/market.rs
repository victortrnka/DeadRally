//! The Underground Market (spec M3c §3): `enterBlackMarketScreen` (0x436700) with its fades,
//! its screen (`drawBlackMarketScreen` 0x4237A0), the loan shark (`drawLoanShark` 0x422A30),
//! the four weapons (0x422E00, 0x422F10, 0x423020, 0x423130), the way on (0x423350), the
//! moves (0x423410, 0x423590) and Enter (`underGroundMenuEnter` 0x4360B0); and the shop's
//! fade back in after it (`postLoadedOrLicense` from 0x438960). DreeRally
//! `ui/blackMarketScreen.c`; dRally `___3d1f8h.c`.

use super::draw::Focus;
use super::licence::draw_price;
use super::shop::{CONTINUE, CONTINUE_FRAMES};
use super::sign_up::PopupThen;
use super::{Menu, State};
use crate::canvas::{Canvas, at};
use crate::keys;

/// The market's items (0x461278): the loan shark, the four weapons, the way on.
const LOAN_SHARK: usize = 0;
const WAY_ON: usize = 5;
/// The boxes along the bottom row, mines to the way on, their prices, and the borders.
const BOX_X: [usize; 5] = [16, 120, 224, 328, 432];
const BOX_Y: usize = 253;
const PRICE_Y: usize = 335;
const SHARK_BORDER: (usize, usize) = (10, 115);
const BORDER_X: [usize; 5] = [10, 114, 218, 322, 426];
const BORDER_Y: usize = 243;
const BORDER_SIZE: (usize, usize) = (108, 114);
/// The loan shark's picture and the way on's flag.
const SHARK: (usize, usize) = (16, 125);
const FLAG: (usize, usize) = (432, 269);
/// The first visit's popup's cursor.
const WELCOME_CURSOR: (usize, usize) = (164, 321);
/// The fades: 51 waits out from 100 % and volume 65500, 50 in from 0, 2 % and 1310 a wait;
/// the volume mask is the volume shifted down by 8.
const FADE_OUT_STEPS: u32 = 51;
const FADE_IN_STEPS: u32 = 50;
const VOLUME_TOP: u32 = 65_500;
const VOLUME_STEP: u32 = 1310;
/// The market's music starts at this order of the menu module (`musicSetOrder(0x3100)`).
const MARKET_ORDER: usize = 0x31;
/// Sounds: a step 26, a weapon 28, the loan shark's deals 29, the way on 24; a refusal is
/// effect 23 on channel 2.
const STEP_SOUND: u8 = 26;
const BUY_SOUND: u8 = 28;
const LOAN_SOUND: u8 = 29;
const ON_SOUND: u8 = 24;
const REFUSE_CHANNEL: usize = 2;
const REFUSE_SOUND: u8 = 23;
const REFUSE_PITCH: u32 = 0x2_5500;
/// After a deal the item's description comes back 310 passes later.
const MESSAGE_PASSES: u32 = 310;
/// The loan shark lends by loan (5 − car; the Vagabond gets none), and the debt grows by a
/// third of half the loan a race (0x4361FB: doubles, truncated).
const LOANS: [f64; 5] = [12_000.0, 9_000.0, 6_000.0, 3_000.0, 1_500.0];
const DEBT_GROWTH: [f64; 5] = [6_000.0, 4_500.0, 3_000.0, 1_500.0, 750.0];
const THIRD: f64 = 1.0 / 3.0;
/// A weapon bought: mines fill to 8, the others are fitted.
const WEAPON_FILL: [i32; 4] = [8, 1, 1, 1];
/// The shop's fade back in turns the flag only when the player can race (0x438C13): no loan
/// due, 1000 with the trade-in value, the money for a repair, the car not near wrecked.
const RACE_MONEY: i64 = 1000;
const WRECK_DAMAGE: i32 = 95;
const LOAN_DUE: i32 = 4;

/// The loan the player's car gets.
fn loan_for(car: i32) -> usize {
    if car == 0 {
        4
    } else {
        (5 - car).clamp(0, 4) as usize
    }
}

/// What is owed on `loan` taken `races` races ago (counting from 1). The count wraps and an
/// amount past an int becomes `_ftol`'s 0x80000000, as the original's arithmetic does.
fn debt(loan: usize, races: i32) -> i32 {
    let owed = f64::from(races.wrapping_sub(1)) * THIRD * DEBT_GROWTH[loan] + LOANS[loan];
    if (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&owed) {
        owed as i32
    } else {
        i32::MIN
    }
}

impl Menu {
    /// The shop's way on with weapons: the shop fades out with its music, the sabotage
    /// sold out while the player leads.
    pub(super) fn open_market(&mut self) -> State {
        if self.campaign.player_leads() {
            self.campaign.stock[3] = 0;
        }
        self.compose_palette();
        State::MarketFadeOut { step: 0 }
    }

    /// The way on's flag turns a frame, shown.
    fn turn_flag(&mut self) {
        let frame = self.assets.menu.continue_flag[self.shop.continue_frame].clone();
        self.screen.draw(&frame, at(FLAG.0, FLAG.1), false);
        self.shown
            .copy_from(&self.screen, at(FLAG.0, FLAG.1), 96, 64);
        self.shop.continue_frame = (self.shop.continue_frame + 1) % CONTINUE_FRAMES;
    }

    fn fade_volume(&mut self, volume: u32) {
        self.sound.set_mask(volume >> 8);
    }

    /// A wait of the shop's fade out (`waitWithRefresh`, no pulse): the volume down, the
    /// flag turning every other wait; then the market drawn under a black palette, its music
    /// started.
    pub(super) fn market_fade_out(&mut self, step: u32) -> State {
        self.fade_volume(VOLUME_TOP - VOLUME_STEP * step);
        if (50 - step) % 2 == 1 {
            self.turn_flag();
        }
        self.palette.fade_market(100 - 2 * i64::from(step));
        if step + 1 < FADE_OUT_STEPS {
            return State::MarketFadeOut { step: step + 1 };
        }
        self.music_order = self.sound.music_order();
        self.sound.set_music_order(MARKET_ORDER);
        self.shop.market = WAY_ON;
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_market(&mut screen);
        if self.campaign.underground_popup {
            self.market_welcome(&mut screen);
        } else {
            self.market_border(&mut screen, WAY_ON);
        }
        self.screen = screen;
        self.shown = self.screen.clone();
        self.compose_palette();
        State::MarketFadeIn { step: 0 }
    }

    /// A wait of the market's fade in; every other wait the flag turns, or on the first
    /// visit the popup's cursor.
    pub(super) fn market_fade_in(&mut self, step: u32) -> State {
        self.fade_volume(VOLUME_STEP * step);
        if step % 2 == 1 {
            if self.campaign.underground_popup {
                self.draw_cursor_at(WELCOME_CURSOR.0, WELCOME_CURSOR.1);
            } else {
                self.turn_flag();
            }
        }
        self.palette.fade_market(2 * i64::from(step));
        if step + 1 < FADE_IN_STEPS {
            return State::MarketFadeIn { step: step + 1 };
        }
        if self.campaign.underground_popup {
            return self.popup_wait_start(PopupThen::Market);
        }
        State::Market { second: false }
    }

    /// The first visit's popup is gone: the market again with its border.
    pub(super) fn after_market_welcome(&mut self) -> State {
        self.campaign.underground_popup = false;
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_market(&mut screen);
        self.market_border(&mut screen, WAY_ON);
        self.screen = screen;
        self.shown = self.screen.clone();
        State::Market { second: false }
    }

    /// `drawBlackMarketScreen`: the title, the side panel, the loan shark, the weapons and
    /// the way on, each drawing its description over the last.
    fn draw_market(&mut self, canvas: &mut Canvas) {
        canvas.restore(&self.graphics.background, at(0, 92), 640, 271);
        canvas.draw(&self.assets.menu.market_title, at(0, 92), true);
        self.draw_side_panel(canvas);
        let menu = &self.assets.menu;
        canvas.draw(&menu.loan_shark, at(SHARK.0, SHARK.1), false);
        canvas.draw(&menu.item_boxes[4], at(BOX_X[4], BOX_Y), false);
        for weapon in 0..4 {
            self.draw_weapon(canvas, weapon);
        }
        self.draw_way_on(canvas);
    }

    /// The first visit's popup (`undergroundMarketPopup`, 0x41C770).
    fn market_welcome(&self, canvas: &mut Canvas) {
        self.graphics
            .popup(canvas, 45, 131, 458, 230, Focus::Focused);
        let texts = &self.assets.menu.texts;
        for (line, text) in texts.shop.market_welcome.iter().enumerate() {
            self.graphics
                .write_text(canvas, text, at(60, 141 + 16 * line));
        }
        self.graphics
            .big_a
            .draw(canvas, &texts.campaign.continue_word, at(192, 316));
    }

    fn border_of(item: usize) -> (usize, usize) {
        if item == LOAN_SHARK {
            SHARK_BORDER
        } else {
            (BORDER_X[item - 1], BORDER_Y)
        }
    }

    fn market_border(&self, canvas: &mut Canvas, item: usize) {
        let (x, y) = Self::border_of(item);
        self.border(canvas, x, y, BORDER_SIZE.0, BORDER_SIZE.1);
    }

    /// `removeBorder` (0x421C40).
    fn remove_market_border(&mut self, item: usize) {
        let (x, y) = Self::border_of(item);
        let (w, h) = BORDER_SIZE;
        let background = &self.graphics.background;
        self.screen.restore(background, at(x, y), w, 5);
        self.screen.restore(background, at(x, y + h - 5), w, 5);
        self.screen.restore(background, at(x, y), 5, h);
        self.screen.restore(background, at(x + w - 5, y), 5, h);
    }

    /// A weapon's box: on sale with its description and price, sold out, or locked.
    fn draw_weapon(&self, canvas: &mut Canvas, weapon: usize) {
        let menu = &self.assets.menu;
        let shop = &menu.texts.shop;
        let x = BOX_X[weapon];
        let (frame, info) = match self.campaign.stock[weapon] {
            1 => (weapon, &shop.weapons[weapon]),
            0 => (8 + weapon, &shop.out_of_stock),
            _ => (4 + weapon, &shop.shareware),
        };
        canvas.draw(&menu.weapons[frame], at(x, BOX_Y), false);
        self.info_popup(canvas, info);
        if self.campaign.stock[weapon] == 1 {
            let car = self.campaign.player().car as usize;
            let price = format!("${}", menu.market_prices[car][weapon]).into_bytes();
            draw_price(canvas, menu, &price, x, PRICE_Y);
        }
    }

    /// The way on's box, its description and its flag.
    fn draw_way_on(&self, canvas: &mut Canvas) {
        let menu = &self.assets.menu;
        canvas.draw(&menu.item_boxes[4], at(BOX_X[4], BOX_Y), false);
        self.info_popup(canvas, &menu.texts.shop.market_on);
        canvas.draw(
            &menu.continue_flag[self.shop.continue_frame],
            at(FLAG.0, FLAG.1),
            false,
        );
    }

    /// The loan shark's popup: his offer, or what is owed.
    fn draw_loan_shark(&self, canvas: &mut Canvas) {
        let shop = &self.assets.menu.texts.shop;
        let player = self.campaign.player();
        if player.loan_races == -1 {
            self.info_popup(canvas, &shop.loan_offers[loan_for(player.car)]);
            return;
        }
        let owed = debt(player.loan.clamp(0, 4) as usize, player.loan_races);
        let mut lines = shop.loan_owed.clone();
        lines[0].extend(owed.to_string().bytes());
        lines[0].push(b'.');
        self.info_popup(canvas, &lines);
    }

    /// An item's box drawn on the screen.
    fn draw_market_item(&mut self, item: usize) {
        let mut screen = std::mem::take(&mut self.screen);
        match item {
            LOAN_SHARK => self.draw_loan_shark(&mut screen),
            WAY_ON => self.draw_way_on(&mut screen),
            weapon => self.draw_weapon(&mut screen, weapon - 1),
        }
        self.screen = screen;
    }

    /// A wait of the market's loop (two a pass); after the second, a deal's message gives
    /// way to the description, the flag turns while selected, and the key.
    pub(super) fn market_tick(&mut self, second: bool) -> State {
        self.palette.after_wait();
        if !second {
            return State::Market { second: true };
        }
        let next = self.market_pass();
        if next == (State::Market { second: false })
            && let Some(quick) = self.quick_keys()
        {
            return self.market_after_quick(quick);
        }
        next
    }

    /// After a quick save or load (0x436E00): the market drawn afresh at full brightness, the
    /// selected item's border and box, the panel, then the confirmation.
    fn market_after_quick(&mut self, quick: super::slots::Quick) -> State {
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_market(&mut screen);
        self.screen = screen;
        self.compose_palette();
        self.palette.show_composed(32..256);
        for item in [WAY_ON, LOAN_SHARK, 1, 2, 3, 4] {
            self.remove_market_border(item);
        }
        let selected = self.shop.market;
        let mut screen = std::mem::take(&mut self.screen);
        self.market_border(&mut screen, selected);
        self.screen = screen;
        self.draw_market_item(selected);
        let mut screen = std::mem::take(&mut self.screen);
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut screen, &self.panel);
        self.screen = screen;
        self.shown = self.screen.clone();
        self.quick_confirm(quick, true)
    }

    /// The rest of a pass: a message giving way, the flag, the key.
    fn market_pass(&mut self) -> State {
        self.shop.message_passes = self.shop.message_passes.saturating_sub(1);
        let selected = self.shop.market;
        if selected == WAY_ON {
            self.turn_flag();
        } else if self.shop.message_passes == 1 {
            self.draw_market_item(selected);
            self.shown = self.screen.clone();
        }
        match self.keys.take() {
            keys::UP | keys::PAD_UP if selected == 1 => {
                self.sound(STEP_SOUND);
                self.market_select(LOAN_SHARK);
            }
            keys::DOWN | keys::PAD_DOWN if selected == LOAN_SHARK => {
                self.sound(STEP_SOUND);
                self.market_select(1);
            }
            keys::LEFT | keys::PAD_LEFT if selected > LOAN_SHARK => {
                self.sound(STEP_SOUND);
                self.market_select(selected - 1);
            }
            keys::RIGHT | keys::PAD_RIGHT if selected < WAY_ON => {
                self.sound(STEP_SOUND);
                if selected > LOAN_SHARK {
                    self.market_select(selected + 1);
                }
            }
            keys::ENTER | 0x9C => return self.market_enter(),
            keys::ESCAPE => {
                // 0x436DE3: left by Escape, the music's order and volume come back with it.
                self.shop.market_escaped = true;
                self.compose_palette();
                return State::MarketLeave { step: 0 };
            }
            _ => {}
        }
        State::Market { second: false }
    }

    /// The selection moves to `item`: its box drawn, the border moved.
    fn market_select(&mut self, item: usize) {
        let from = self.shop.market;
        self.shop.market = item;
        self.draw_market_item(item);
        self.remove_market_border(from);
        let mut screen = std::mem::take(&mut self.screen);
        self.market_border(&mut screen, item);
        self.screen = screen;
        self.shown = self.screen.clone();
    }

    fn refuse(&mut self) {
        let volume = self.config.effects_volume();
        self.sound
            .trigger_at(REFUSE_CHANNEL, REFUSE_SOUND, volume, REFUSE_PITCH);
    }

    /// `underGroundMenuEnter` on the selected item.
    fn market_enter(&mut self) -> State {
        match self.shop.market {
            LOAN_SHARK => self.deal_with_shark(),
            WAY_ON => return self.market_on(),
            item => self.buy_weapon(item - 1),
        }
        State::Market { second: false }
    }

    /// A deal's message in the popup, the side panel after it when the money changed; the
    /// description comes back later.
    fn market_message(&mut self, lines: &[Vec<u8>], panel: bool) {
        self.shop.message_passes = MESSAGE_PASSES;
        let mut screen = std::mem::take(&mut self.screen);
        self.info_popup(&mut screen, lines);
        if panel {
            self.draw_side_panel(&mut screen);
        }
        self.screen = screen;
        self.shown = self.screen.clone();
    }

    /// The loan shark: a loan when there is none (none for a Vagabond), else paying it back
    /// when the money is there.
    fn deal_with_shark(&mut self) {
        let player = *self.campaign.player();
        let shop = &self.assets.menu.texts.shop;
        if player.loan_races == -1 {
            if player.car == 0 {
                let lines = shop.loan_refused.clone();
                self.refuse();
                self.market_message(&lines, false);
                return;
            }
            let loan = loan_for(player.car);
            let lines = shop.loans_granted[loan].clone();
            self.sound(LOAN_SOUND);
            let player = self.campaign.player_mut();
            player.loan_races = 1;
            player.loan = loan as i32;
            player.money = player.money.wrapping_add(LOANS[loan] as i32);
            self.market_message(&lines, true);
            return;
        }
        let owed = debt(player.loan.clamp(0, 4) as usize, player.loan_races);
        if player.money < owed {
            self.refuse();
            return;
        }
        let lines = shop.loan_paid.clone();
        self.sound(LOAN_SOUND);
        let player = self.campaign.player_mut();
        player.money = player.money.wrapping_sub(owed);
        player.loan_races = -1;
        player.loan = -1;
        self.market_message(&lines, true);
    }

    /// A weapon in stock and affordable: paid for, sold out, fitted.
    fn buy_weapon(&mut self, weapon: usize) {
        let car = self.campaign.player().car as usize;
        let price = self.assets.menu.market_prices[car][weapon];
        if self.campaign.stock[weapon] != 1 || self.short_of(price) {
            self.refuse();
            return;
        }
        self.sound(BUY_SOUND);
        self.campaign.player_mut().money -= price;
        self.campaign.stock[weapon] = 0;
        self.draw_market_item(weapon + 1);
        let player = self.campaign.player_mut();
        match weapon {
            0 => player.mines = WEAPON_FILL[0],
            1 => player.spikes = WEAPON_FILL[1],
            2 => player.rocket = WEAPON_FILL[2],
            _ => player.sabotage = WEAPON_FILL[3],
        }
        let lines = self.assets.menu.texts.shop.weapons_bought[weapon].clone();
        self.market_message(&lines, true);
    }

    /// The way on: a wreck must be repaired first; else effect 24 and the sign-up (the
    /// Adversary's race when the player leads comes with M6).
    fn market_on(&mut self) -> State {
        if self.campaign.player().damage == 100 {
            let lines = self.assets.menu.texts.shop.market_wrecked.clone();
            let mut screen = std::mem::take(&mut self.screen);
            self.info_popup(&mut screen, &lines);
            self.screen = screen;
            self.shown = self.screen.clone();
            self.refuse();
            return State::Market { second: false };
        }
        self.sound(ON_SOUND);
        // 0x4366CA: the selection left on the sabotage, as the original leaves it.
        self.shop.market = 4;
        self.open_sign_up()
    }

    /// A wait of the fade out after Escape: the flag turning every other wait while
    /// selected, the volume down; then the shop drawn afresh under a black palette and its
    /// music's order back (`postLoadedOrLicense`, 0x4389A6).
    pub(super) fn market_leave(&mut self, step: u32) -> State {
        // 0x43710E: the flag and the volume only when left by Escape, not after a race.
        let escaped = self.shop.market_escaped;
        if escaped && (50 - step) % 2 == 1 && self.shop.market == WAY_ON {
            self.turn_flag();
        }
        if escaped {
            self.fade_volume(VOLUME_TOP - VOLUME_STEP * step);
        }
        self.palette.fade_market(100 - 2 * i64::from(step));
        if step + 1 < FADE_OUT_STEPS {
            return State::MarketLeave { step: step + 1 };
        }
        self.shop_again()
    }

    /// The shop again after the market or a race's results (`postLoadedOrLicense` from
    /// 0x4389A6): drawn afresh on the menu's background with the continue item selected and
    /// the bottom panel under it, shown under a black palette; the music's order back when the
    /// market was left by Escape (0x438B58); then faded in.
    pub(super) fn shop_again(&mut self) -> State {
        self.campaign.welcome = false;
        self.screen.copy_all(&self.graphics.background);
        self.shop.selected = CONTINUE;
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_shop(&mut screen);
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut screen, &self.panel);
        self.screen = screen;
        self.shown = self.screen.clone();
        if self.shop.market_escaped {
            self.sound.set_music_order(self.music_order);
        }
        self.compose_palette();
        State::ShopFadeIn { step: 0 }
    }

    /// A wait of the shop's fade in after the market: the volume up, the flag turning every
    /// other wait when the player can race; then the continue item's border.
    pub(super) fn shop_fade_in(&mut self, step: u32) -> State {
        if self.shop.market_escaped {
            self.fade_volume(VOLUME_STEP * step);
        }
        if step % 2 == 1 && self.can_race() {
            self.turn_flag();
        }
        self.palette.fade_market(2 * i64::from(step));
        if step + 1 < FADE_IN_STEPS {
            return State::ShopFadeIn { step: step + 1 };
        }
        let mut screen = std::mem::take(&mut self.screen);
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut screen, &self.panel);
        self.border(
            &mut screen,
            BORDER_X[4],
            BORDER_Y,
            BORDER_SIZE.0,
            BORDER_SIZE.1,
        );
        self.screen = screen;
        self.shown = self.screen.clone();
        self.shop.market_escaped = false;
        State::Shop { second: false }
    }

    /// No loan due, 1000 or more with the car's trade-in value, the money for a repair, the
    /// car not near wrecked (0x438C13).
    fn can_race(&self) -> bool {
        let player = self.campaign.player();
        let full = self.assets.menu.texts.campaign.cars[player.car as usize].repair_price;
        let repair = if self.campaign.use_weapons {
            full / 2
        } else {
            full
        };
        player.loan_races != LOAN_DUE
            && i64::from(player.money) + i64::from(self.trade_in()) >= RACE_MONEY
            && player.money >= repair
            && player.damage <= WRECK_DAMAGE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loan_grows_by_a_sixth_of_it_each_race() {
        // 0x4361FB: the debt in the race the loan was taken is the loan; each race after
        // adds a third of half of it. Another sum would change what paying back costs.
        assert_eq!(debt(0, 1), 12_000);
        assert_eq!(debt(0, 2), 14_000);
        assert_eq!(debt(4, 2), 1_750);
        assert_eq!(debt(1, 4), 13_500);
        assert_eq!(debt(3, 3), 4_000);
    }

    #[test]
    fn a_better_car_gets_a_bigger_loan() {
        assert_eq!(LOANS[loan_for(1)], 1_500.0);
        assert_eq!(LOANS[loan_for(5)], 12_000.0);
    }
}
