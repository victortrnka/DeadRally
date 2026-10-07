//! The shop after a race (spec M5): `postLoadedOrLicense` from 0x4389A6. The shop is drawn
//! afresh with the first popup due over it and faded in; after the fade, that popup's wait,
//! then the others due one by one, each with its wait: the welcome to the shop (0x41C230),
//! the player lapped (0x41B400), the sponsors (three wins in a row 0x41B4F0, a clean race
//! 0x41B6A0, everyone else wrecked 0x41B850), the drug run's and the hit's outcome (0x41BA00,
//! 0x41BDE0), the loan due (0x41C4C0), the end of the road (0x41C300), then a last place
//! (0x42E6F0); and the shop drawn again. DreeRally `ui/shopScreen.c`, `ui/util/popup.c`.

use deadrally_gamedata::image::Image;

use super::draw::Focus;
use super::shop::CONTINUE;
use super::sign_up::PopupThen;
use super::{Menu, State};
use crate::campaign::{Campaign, Driver};
use crate::canvas::at;
use crate::keys;

/// A popup of the shop after a race.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Popup {
    Welcome,
    Lapped,
    WinStreak,
    CleanRace,
    AllWrecked,
    DrugRun,
    Hit,
    LoanDue,
    EndOfRoad,
    TooSlow,
}

/// The popups in the order the shop looks for them (0x4389C8, 0x439020).
const ORDER: [Popup; 9] = [
    Popup::Welcome,
    Popup::Lapped,
    Popup::WinStreak,
    Popup::CleanRace,
    Popup::AllWrecked,
    Popup::DrugRun,
    Popup::Hit,
    Popup::LoanDue,
    Popup::EndOfRoad,
];

/// The wins in a row a sponsor pays for.
const WINS_IN_A_ROW: i32 = 3;
/// The loan's races when it is due (as the market's way on counts them).
const LOAN_DUE: i32 = 4;
/// The last place, which the last popup is for.
const LAST: i32 = 4;
/// Ranks up to this one see the shop drawn again after its fade (0x438E74).
const TOP_RANKS: i32 = 7;
/// The end of the road: less than this with the car's trade-in, and a car more damaged than
/// this (0x438AB6).
const RACE_MONEY: i32 = 1000;
const WRECK_DAMAGE: i32 = 95;
/// What the sponsors pay by the player's car, the Vagabond first: three wins in a row
/// (0x41B53A) and everyone else wrecked (0x41B89A) alike, a clean race (0x41B6EA).
const WIN_STREAK_PAY: [i32; 6] = [600, 1000, 2000, 3000, 4000, 5000];
const ALL_WRECKED_PAY: [i32; 6] = [600, 1000, 2000, 3000, 4000, 5000];
const CLEAN_RACE_PAY: [i32; 6] = [350, 750, 1500, 3000, 4500, 6000];
/// What the deals pay by their level (1, the best car's, to 6, the Vagabond's: the offer's
/// pay, 0x431B30) and what failing them costs (0x41BC9A, 0x41C0E4).
const DRUG_RUN_PAY: [i32; 6] = [12_000, 8000, 6000, 4000, 2000, 1000];
const DRUG_RUN_LOSS: [i32; 6] = [6000, 4000, 3000, 2000, 1000, 500];
const HIT_PAY: [i32; 6] = [6000, 4000, 3000, 2000, 1000, 500];
const HIT_LOSS: [i32; 6] = [3000, 2000, 1000, 750, 500, 250];
/// What the loan shark takes back by loan (0x41C4C0), the biggest first: what the market
/// asks for it in its fourth race.
const LOAN_DUE_DEBT: [i32; 5] = [18_000, 13_500, 9000, 4500, 2250];
/// The popups' frames: the welcome's (0x41C243), the others' (0x41B413 and on), the last
/// place's a little wider (0x42E703).
const WELCOME_FRAME: (usize, usize, usize, usize) = (45, 131, 458, 230);
const FRAME: (usize, usize, usize, usize) = (33, 131, 482, 230);
const TOO_SLOW_FRAME: (usize, usize, usize, usize) = (33, 131, 497, 230);
/// The pictures beside the lines (0x41B431), the loan shark's lower down and drawn whole
/// (0x41C569).
const PICTURE: (usize, usize) = (45, 168);
const LOAN_SHARK: (usize, usize) = (45, 184);
/// The lines, 16 apart: from (60, 141) without a picture, from (161, 136) beside one, from
/// (161, 168) for the deals' outcomes and from (128, 136) beside the loan shark.
const LINE_STEP: usize = 16;
const PLAIN_LINES: (usize, usize) = (60, 141);
const PICTURE_LINES: (usize, usize) = (161, 136);
const DEAL_LINES: (usize, usize) = (161, 168);
const LOAN_LINES: (usize, usize) = (128, 136);
/// The word under every popup, in big A (0x41C2F6).
const CONTINUE_WORD: (usize, usize) = (192, 316);
/// The deals' lines the hit's victim and the pay go on (0x41C02E, 0x41BC64, 0x41C0AE).
const VICTIM_LINE: usize = 6;
const PAY_LINE: usize = 7;
/// What the shop draws again after the popups (0x4392B4): its rows 96 to 362.
const SHOP_TOP: usize = 96;
const SHOP_ROWS: usize = 267;

/// A popup's picture: where it goes and whether colour 0 leaves the popup under it.
type Picture<'a> = (&'a Image, (usize, usize), bool);

/// Whether the player is at the end of the road (0x438AB6): less than 1000 with `trade_in`
/// (the car's worth when the shop was entered), less than a repair costs (`repair`), and the
/// car more than 95 % damaged.
pub(crate) fn broke(player: &Driver, trade_in: i32, repair: i32) -> bool {
    // Added as 32 bits, as the original adds them (0x438AC0).
    player.money.wrapping_add(trade_in) < RACE_MONEY
        && player.money < repair
        && player.damage > WRECK_DAMAGE
}

/// Whether the shop, its fade done, looks for popups at all (0x438E1F): one is due, the player
/// ranks in the top seven, or came last.
pub(crate) fn looks_for_popups(due: bool, rank: i32, place: i32) -> bool {
    due || rank <= TOP_RANKS || place == LAST
}

/// Whether the last place gets its popup (0x439294): not when the player heard they were
/// lapped or at the end of the road on this visit.
pub(crate) fn too_slow(place: i32, told: bool) -> bool {
    place == LAST && !told
}

/// A failed deal (0x41BD8F, 0x41C1D9) or a loan unpaid (0x41C71B, without the first step):
/// the loss paid when the money is there, else an engine level taken (the car's worth kept),
/// else half the money, rounded toward zero.
fn punish(player: &mut Driver, loss: Option<i32>) {
    if let Some(loss) = loss
        && player.money >= loss
    {
        player.money -= loss;
    } else if player.engine > 0 {
        player.engine -= 1;
    } else {
        player.money /= 2;
    }
}

/// What a deal of `level` (1 to 6) pays or costs by `table`; a level past them (never
/// offered) nothing.
fn by_level(table: &[i32; 6], level: i32) -> Option<i32> {
    let k = usize::try_from(level.checked_sub(1)?).ok()?;
    table.get(k).copied()
}

/// A deal's outcome: its pay by its level when done (positive), its loss when failed
/// (negative).
fn settle_deal(player: &mut Driver, level: i32, pay: &[i32; 6], loss: &[i32; 6]) {
    if level > 0 {
        if let Some(pay) = by_level(pay, level) {
            player.money = player.money.wrapping_add(pay);
        }
    } else if let Some(loss) = level.checked_neg().and_then(|level| by_level(loss, level)) {
        punish(player, Some(loss));
    }
}

/// What paying the loan due back costs (0x41C4C0), none for a loan past the Vagabond's (only
/// from an edited save).
fn loan_debt(player: &Driver) -> Option<i32> {
    let loan = usize::try_from(player.loan).ok()?;
    LOAN_DUE_DEBT.get(loan).copied()
}

impl Campaign {
    /// The popup due next (0x4389C8, then 0x439020 after each popup's wait), the player
    /// `lapped` (0x456BC0) or `broke`.
    pub(crate) fn popup_due(&self, lapped: bool, broke: bool) -> Option<Popup> {
        ORDER.into_iter().find(|&popup| match popup {
            Popup::Welcome => self.welcome,
            Popup::Lapped => lapped,
            Popup::WinStreak => self.win_streak == WINS_IN_A_ROW,
            Popup::CleanRace => self.clean_race,
            Popup::AllWrecked => self.all_wrecked,
            Popup::DrugRun => self.drug_deal != 0,
            Popup::Hit => self.hit != 0,
            Popup::LoanDue => self.player().loan_races == LOAN_DUE,
            Popup::EndOfRoad => broke,
            Popup::TooSlow => false,
        })
    }

    /// Whether one of the popups the campaign itself flags is due (0x42914B, where the shop's
    /// drawing leaves the continue item's border out then): the welcome, a sponsor's, a
    /// deal's outcome or a loan due; not the player lapped nor the end of the road.
    pub(crate) fn shop_popup_flagged(&self) -> bool {
        self.popup_due(false, false)
            .is_some_and(|popup| popup != Popup::EndOfRoad)
    }

    /// What `popup` pays or takes as it is drawn: the sponsors' money by the player's car,
    /// the deals' pay or loss by their level, the loan's debt.
    pub(crate) fn settle_popup(&mut self, popup: Popup) {
        let (drug_deal, hit) = (self.drug_deal, self.hit);
        let player = self.player_mut();
        // A car past the Lotus (only from an edited save) gets no sponsor's money.
        let car = usize::try_from(player.car).ok().filter(|&car| car < 6);
        let sponsor = |pay: &[i32; 6]| car.map_or(0, |car| pay[car]);
        match popup {
            Popup::WinStreak => player.money = player.money.wrapping_add(sponsor(&WIN_STREAK_PAY)),
            Popup::CleanRace => player.money = player.money.wrapping_add(sponsor(&CLEAN_RACE_PAY)),
            Popup::AllWrecked => {
                player.money = player.money.wrapping_add(sponsor(&ALL_WRECKED_PAY));
            }
            Popup::DrugRun => settle_deal(player, drug_deal, &DRUG_RUN_PAY, &DRUG_RUN_LOSS),
            Popup::Hit => settle_deal(player, hit, &HIT_PAY, &HIT_LOSS),
            Popup::LoanDue => match loan_debt(player) {
                Some(debt) if player.money < debt => punish(player, None),
                Some(debt) => player.money -= debt,
                None => {}
            },
            Popup::Welcome | Popup::Lapped | Popup::EndOfRoad | Popup::TooSlow => {}
        }
    }

    /// What is cleared once `popup` has been told (0x438F4B, 0x439048 and on); the player
    /// lapped and the end of the road are the menu's.
    pub(crate) fn popup_told(&mut self, popup: Popup) {
        match popup {
            Popup::Welcome => self.welcome = false,
            Popup::WinStreak => self.win_streak = 0,
            Popup::CleanRace => self.clean_race = false,
            Popup::AllWrecked => self.all_wrecked = false,
            Popup::DrugRun => self.drug_deal = 0,
            Popup::Hit => self.hit = 0,
            Popup::LoanDue => {
                let player = self.player_mut();
                player.loan_races = -1;
                player.loan = -1;
            }
            Popup::Lapped | Popup::EndOfRoad | Popup::TooSlow => {}
        }
    }
}

impl Menu {
    /// Whether the player is at the end of the road, by the trade-in counted when the shop
    /// was entered.
    fn at_end_of_road(&self) -> bool {
        broke(
            self.campaign.player(),
            self.shop.trade_in,
            self.repair_price(),
        )
    }

    /// The popup due next, the player lapped as the last race left it.
    pub(super) fn shop_popup_due(&self) -> Option<Popup> {
        self.campaign
            .popup_due(self.outcome.lapped, self.at_end_of_road())
    }

    /// 0x4389C8: the first popup due drawn over the shop, settled; none told yet.
    pub(super) fn first_shop_popup(&mut self) {
        self.shop.told = false;
        self.shop.popup = self.shop_popup_due();
        if let Some(popup) = self.shop.popup {
            self.draw_shop_popup(popup);
        }
    }

    /// The shop faded in (0x438E1F): its loop at once when it has nothing to tell; else the
    /// first popup's wait, or the next popup.
    pub(super) fn after_shop_fade(&mut self) -> State {
        let due = self.shop_popup_due().is_some();
        let rank = self.campaign.player().rank;
        if !looks_for_popups(due, rank, self.campaign.place) {
            self.shop.market_escaped = false;
            return State::Shop { second: false };
        }
        if self.shop.popup.is_some() {
            return self.popup_wait_start(PopupThen::Shop);
        }
        self.next_shop_popup()
    }

    /// A popup's wait is over: the side panel shows what it paid or took and its flag is
    /// cleared (the end of the road ends the game after the shop's next pass), then the
    /// next popup (0x438EDF, 0x439043 and on); after the last place's, the shop again.
    pub(super) fn shop_popup_told(&mut self) -> State {
        let Some(popup) = self.shop.popup.take() else {
            return self.next_shop_popup();
        };
        if popup == Popup::TooSlow {
            // 0x4392AE.
            self.campaign.place = 0;
            return self.shop_drawn_again();
        }
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_side_panel(&mut screen);
        self.screen = screen;
        self.shown = self.screen.clone();
        match popup {
            Popup::Lapped => self.outcome.lapped = false,
            Popup::EndOfRoad => self.shop.game_over = true,
            _ => self.campaign.popup_told(popup),
        }
        self.next_shop_popup()
    }

    /// The next popup due drawn, shown and waited for (0x439020); with none left, or the
    /// game over, the last place's popup unless the player heard worse (0x439294), then the
    /// shop again.
    fn next_shop_popup(&mut self) -> State {
        let next = if self.shop.game_over {
            None
        } else {
            self.shop_popup_due()
        };
        let next = next
            .or_else(|| too_slow(self.campaign.place, self.shop.told).then_some(Popup::TooSlow));
        if let Some(popup) = next {
            self.draw_shop_popup(popup);
            self.shown = self.screen.clone();
            self.shop.popup = Some(popup);
            return self.popup_wait_start(PopupThen::Shop);
        }
        if self.campaign.place == LAST {
            self.campaign.place = 0;
        }
        self.shop_drawn_again()
    }

    /// 0x4392B4: the shop drawn again where the popups were, with the bottom panel and the
    /// continue item's border, shown; then its loop.
    fn shop_drawn_again(&mut self) -> State {
        self.screen
            .restore(&self.graphics.background, at(0, SHOP_TOP), 640, SHOP_ROWS);
        let mut screen = std::mem::take(&mut self.screen);
        self.draw_shop(&mut screen);
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut screen, &self.panel);
        self.item_border(&mut screen, CONTINUE);
        self.screen = screen;
        self.shown = self.screen.clone();
        self.shop.market_escaped = false;
        State::Shop { second: false }
    }

    /// The shop's pass after the end of the road (0x438960 to 0x439B39): the game ends
    /// (`endGame` 0x4291D0) and the Start Racing menu comes back as after Escape. The original
    /// acts on a key read in that pass first, the whole Underground Market and race on Enter
    /// at the way on; DeadRally lets that key go instead, so a game that is over plays on in
    /// no way.
    pub(super) fn shop_game_over(&mut self) -> State {
        self.keys.take();
        self.shop.game_over = false;
        self.end_game();
        self.leave_shop()
    }

    /// What `popup` says: the sponsors' by the player's car, the deals' by their outcome
    /// with the pay (and the hit's victim) on their lines, the loan's by whether it can be
    /// paid back.
    fn popup_lines(&self, popup: Popup) -> Vec<Vec<u8>> {
        let texts = &self.assets.menu.texts.shop_popups;
        let campaign = &self.campaign;
        let player = campaign.player();
        // The original's text for a car past the Lotus (only from an edited save) is the
        // Vagabond's.
        let car = usize::try_from(player.car)
            .ok()
            .filter(|&car| car < 6)
            .unwrap_or(0);
        let pay = |table: &[i32; 6], level: i32| {
            by_level(table, level).map_or_else(Vec::new, |pay| pay.to_string().into_bytes())
        };
        match popup {
            Popup::Welcome => texts.welcome.clone(),
            Popup::Lapped => texts.lapped.clone(),
            Popup::WinStreak => texts.win_streak[car].clone(),
            Popup::CleanRace => texts.clean_race[car].clone(),
            Popup::AllWrecked => texts.all_wrecked[car].clone(),
            Popup::DrugRun if campaign.drug_deal > 0 => {
                // 0x41BC00: the pay is the amount the offer named.
                let mut lines = texts.drug_run.clone();
                lines[PAY_LINE].extend(pay(&DRUG_RUN_PAY, campaign.drug_deal));
                lines[PAY_LINE].extend(&texts.drug_run_end);
                lines
            }
            Popup::DrugRun => texts.drug_run_failed.clone(),
            Popup::Hit if campaign.hit > 0 => {
                let mut lines = texts.hit.clone();
                lines[VICTIM_LINE].extend(&campaign.hit_victim_name);
                lines[VICTIM_LINE].extend(&texts.hit_victim_end);
                lines[PAY_LINE].extend(pay(&HIT_PAY, campaign.hit));
                lines[PAY_LINE].extend(&texts.hit_end);
                lines
            }
            Popup::Hit => texts.hit_failed.clone(),
            Popup::LoanDue => match loan_debt(player) {
                Some(debt) if player.money < debt => texts.loan_unpaid.clone(),
                _ => texts.loan_repaid.clone(),
            },
            Popup::EndOfRoad => texts.end_of_road.clone(),
            Popup::TooSlow => texts.too_slow.clone(),
        }
    }

    /// `popup` drawn over the shop, settled as it is drawn: its money is in the record before
    /// the side panel shows it after the wait. The end of the road and the last place also
    /// let go of Enter (0x41C4AB, 0x42E7DD).
    fn draw_shop_popup(&mut self, popup: Popup) {
        self.shop.told |= matches!(popup, Popup::Lapped | Popup::EndOfRoad);
        let lines = self.popup_lines(popup);
        self.campaign.settle_popup(popup);
        let menu = &self.assets.menu;
        let (frame, picture, origin): (_, Option<Picture>, _) = match popup {
            Popup::Welcome => (WELCOME_FRAME, None, PLAIN_LINES),
            Popup::Lapped | Popup::WinStreak => {
                (FRAME, Some((&menu.sponsor, PICTURE, true)), PICTURE_LINES)
            }
            Popup::CleanRace => (FRAME, Some((&menu.hitman, PICTURE, true)), PICTURE_LINES),
            Popup::AllWrecked => (FRAME, Some((&menu.reaper, PICTURE, true)), PICTURE_LINES),
            Popup::DrugRun => (FRAME, Some((&menu.drug_dealer, PICTURE, true)), DEAL_LINES),
            Popup::Hit => (FRAME, Some((&menu.hitman, PICTURE, true)), DEAL_LINES),
            Popup::LoanDue => (
                FRAME,
                Some((&menu.loan_shark, LOAN_SHARK, false)),
                LOAN_LINES,
            ),
            Popup::EndOfRoad => (FRAME, None, PLAIN_LINES),
            Popup::TooSlow => (
                TOO_SLOW_FRAME,
                Some((&menu.sponsor, PICTURE, true)),
                PICTURE_LINES,
            ),
        };
        let mut screen = std::mem::take(&mut self.screen);
        let (x, y, w, h) = frame;
        self.graphics.popup(&mut screen, x, y, w, h, Focus::Focused);
        if let Some((image, (px, py), transparent)) = picture {
            screen.draw(image, at(px, py), transparent);
        }
        for (line, text) in lines.iter().enumerate() {
            let at_line = at(origin.0, origin.1 + LINE_STEP * line);
            self.graphics.write_text(&mut screen, text, at_line);
        }
        let word = &menu.texts.campaign.continue_word;
        self.graphics
            .big_a
            .draw(&mut screen, word, at(CONTINUE_WORD.0, CONTINUE_WORD.1));
        self.screen = screen;
        if matches!(popup, Popup::EndOfRoad | Popup::TooSlow) {
            self.keys.release(keys::ENTER);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::PLAYER;

    /// A game in progress: the player (driver 19) in car `car` with 10000, no loan.
    fn campaign(car: i32) -> Campaign {
        let mut campaign = Campaign::new(1);
        let player = &mut campaign.drivers[PLAYER];
        player.car = car;
        player.money = 10_000;
        player.engine = 2;
        player.loan = -1;
        player.loan_races = -1;
        campaign
    }

    fn money(campaign: &Campaign) -> i32 {
        campaign.player().money
    }

    #[test]
    fn every_popup_due_is_told_once_in_the_originals_order() {
        // A popup out of order or told twice would show the player another screen after the
        // race than the original, and pay a sponsor twice.
        let mut campaign = campaign(3);
        campaign.welcome = true;
        campaign.win_streak = 3;
        campaign.clean_race = true;
        campaign.all_wrecked = true;
        campaign.drug_deal = 2;
        campaign.hit = 3;
        campaign.player_mut().loan = 1;
        campaign.player_mut().loan_races = 4;
        let mut lapped = true;
        let mut broke = true;
        let mut told = Vec::new();
        // At most one more round than there are popups, so a flag left set fails here.
        for _ in 0..=ORDER.len() {
            let Some(popup) = campaign.popup_due(lapped, broke) else {
                break;
            };
            campaign.popup_told(popup);
            match popup {
                Popup::Lapped => lapped = false,
                Popup::EndOfRoad => broke = false,
                _ => {}
            }
            told.push(popup);
        }
        assert_eq!(
            told,
            [
                Popup::Welcome,
                Popup::Lapped,
                Popup::WinStreak,
                Popup::CleanRace,
                Popup::AllWrecked,
                Popup::DrugRun,
                Popup::Hit,
                Popup::LoanDue,
                Popup::EndOfRoad,
            ]
        );
    }

    #[test]
    fn two_wins_in_a_row_are_no_streak_yet() {
        let mut campaign = campaign(3);
        campaign.win_streak = 2;
        assert_eq!(campaign.popup_due(false, false), None);
        campaign.win_streak = 3;
        assert_eq!(campaign.popup_due(false, false), Some(Popup::WinStreak));
        campaign.popup_told(Popup::WinStreak);
        assert_eq!(campaign.win_streak, 0, "the next streak counts from 0");
    }

    #[test]
    fn the_sponsors_pay_by_the_players_car() {
        // The popups' texts name these sums; another sum would leave the side panel's money
        // off the original's after the popup.
        for car in 0..6 {
            let paid = |popup: Popup| {
                let mut campaign = campaign(car);
                campaign.settle_popup(popup);
                money(&campaign) - 10_000
            };
            let k = car as usize;
            assert_eq!(
                paid(Popup::WinStreak),
                [600, 1000, 2000, 3000, 4000, 5000][k]
            );
            assert_eq!(
                paid(Popup::CleanRace),
                [350, 750, 1500, 3000, 4500, 6000][k]
            );
            assert_eq!(
                paid(Popup::AllWrecked),
                [600, 1000, 2000, 3000, 4000, 5000][k]
            );
        }
        let mut lotus = campaign(5);
        lotus.settle_popup(Popup::CleanRace);
        assert_eq!(money(&lotus), 16_000);
        let mut vagabond = campaign(0);
        vagabond.settle_popup(Popup::WinStreak);
        assert_eq!(money(&vagabond), 10_600);
    }

    #[test]
    fn a_deal_done_pays_what_the_offer_promised() {
        // The drug run of the best car (level 1) pays 12000, the Vagabond's (6) 1000; the hit
        // half of that.
        let paid = |drug_deal: i32, hit: i32, popup: Popup| {
            let mut campaign = campaign(2);
            campaign.drug_deal = drug_deal;
            campaign.hit = hit;
            campaign.settle_popup(popup);
            money(&campaign) - 10_000
        };
        assert_eq!(paid(1, 0, Popup::DrugRun), 12_000);
        assert_eq!(paid(4, 0, Popup::DrugRun), 4000);
        assert_eq!(paid(6, 0, Popup::DrugRun), 1000);
        assert_eq!(paid(0, 1, Popup::Hit), 6000);
        assert_eq!(paid(0, 3, Popup::Hit), 3000);
        assert_eq!(paid(0, 6, Popup::Hit), 500);
    }

    #[test]
    fn a_failed_deal_takes_its_loss_else_an_engine_level_else_half_the_money() {
        // 0x41BD8F: the loss when the money is there; else an engine level (the car's worth
        // kept); with no level left, half the money, rounded toward zero.
        let after = |drug_deal: i32, money: i32, engine: i32| {
            let mut campaign = campaign(2);
            campaign.drug_deal = drug_deal;
            campaign.player_mut().money = money;
            campaign.player_mut().engine = engine;
            campaign.settle_popup(Popup::DrugRun);
            (campaign.player().money, campaign.player().engine)
        };
        assert_eq!(after(-1, 7000, 2), (1000, 2));
        assert_eq!(after(-1, 6000, 2), (0, 2), "exactly the loss is enough");
        assert_eq!(after(-6, 499, 2), (499, 1));
        assert_eq!(after(-3, 2999, 0), (1499, 0));
        assert_eq!(after(-3, -5, 0), (-2, 0));
        let mut campaign = campaign(2);
        campaign.hit = -4;
        campaign.settle_popup(Popup::Hit);
        assert_eq!(money(&campaign), 9250);
    }

    #[test]
    fn a_loan_due_is_paid_back_or_costs_an_engine_level_and_is_gone_either_way() {
        let mut paid = campaign(2);
        paid.player_mut().money = 20_000;
        paid.player_mut().loan = 0;
        paid.player_mut().loan_races = 4;
        paid.settle_popup(Popup::LoanDue);
        assert_eq!(money(&paid), 2000);
        paid.popup_told(Popup::LoanDue);
        assert_eq!((paid.player().loan, paid.player().loan_races), (-1, -1));
        let mut short = campaign(2);
        short.player_mut().money = 4499;
        short.player_mut().loan = 3;
        short.player_mut().loan_races = 4;
        short.settle_popup(Popup::LoanDue);
        assert_eq!((money(&short), short.player().engine), (4499, 1));
        short.player_mut().engine = 0;
        short.settle_popup(Popup::LoanDue);
        assert_eq!(money(&short), 2249);
    }

    #[test]
    fn a_told_popup_is_not_told_after_the_next_race() {
        let mut campaign = campaign(1);
        campaign.welcome = true;
        campaign.clean_race = true;
        campaign.all_wrecked = true;
        campaign.drug_deal = -2;
        campaign.hit = 5;
        for popup in [
            Popup::Welcome,
            Popup::CleanRace,
            Popup::AllWrecked,
            Popup::DrugRun,
            Popup::Hit,
        ] {
            campaign.popup_told(popup);
        }
        assert_eq!(campaign.popup_due(false, false), None);
        assert_eq!((campaign.drug_deal, campaign.hit), (0, 0));
    }

    #[test]
    fn only_a_near_wreck_without_money_for_a_race_or_a_repair_is_the_end_of_the_road() {
        // Any of the three conditions failing keeps the game going.
        let player = |money: i32, damage: i32| Driver {
            money,
            damage,
            ..Driver::default()
        };
        assert!(broke(&player(900, 96), 99, 950));
        assert!(!broke(&player(900, 96), 100, 950), "1000 with the trade-in");
        assert!(!broke(&player(950, 96), 0, 950), "the money for a repair");
        assert!(!broke(&player(900, 95), 0, 950), "the car not near wrecked");
    }

    #[test]
    fn the_top_seven_and_the_last_place_see_the_shop_after_its_fade() {
        assert!(!looks_for_popups(false, 8, 3));
        assert!(looks_for_popups(true, 8, 3));
        assert!(
            looks_for_popups(false, 7, 3),
            "drawn again in the top seven"
        );
        assert!(looks_for_popups(false, 8, 4), "the last place");
    }

    #[test]
    fn a_last_place_is_told_unless_the_player_heard_worse() {
        assert!(too_slow(4, false));
        assert!(!too_slow(4, true), "lapped or at the end of the road");
        assert!(!too_slow(3, false));
    }
}
