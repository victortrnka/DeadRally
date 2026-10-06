//! The race's books (spec M5): what `previewRaceScreen` does after `startRace` returns, in a
//! game against the computer (0x4334F7 to 0x434512): the deals settled, the weapons gone, the
//! money picked up and the prizes paid, the wins, races and damage carried over, and a
//! headline for the bottom panel.

use crate::campaign::Campaign;

/// How a car finished the race (`sub_402240`): its place, its damage in percent (100 for a
/// wreck) and the money power-ups it picked up, in the cars' places on the grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Finish {
    pub(crate) place: i32,
    pub(crate) damage: i32,
    pub(crate) money: i32,
}

/// How the race ended: each car's finish, whether the player was lapped (0x456BC0) and took
/// the bonus power-up (0x4A7AAC); the race's laps (0x463CA0), the player's race time
/// (0x45EEC0, 0x45EEBC, 0x462D74) and best lap (0x463CAC, 0x45EB48, 0x461FEC) in minutes,
/// seconds and hundredths.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) finishes: Vec<Finish>,
    pub(crate) lapped: bool,
    pub(crate) bonus: bool,
    pub(crate) laps: i32,
    pub(crate) race_time: [i32; 3],
    pub(crate) best_lap: [i32; 3],
}

/// What the results show of the player's race: the place (0x456B50), the money picked up
/// (0x456BE0) and the prize (0x456BDC).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Books {
    pub(crate) place: i32,
    pub(crate) picked_up: i32,
    pub(crate) prize: i32,
}

/// The prizes for the first three places of the easy, medium and hard race (0x433C28,
/// 0x433B7D, 0x433AD2).
const PRIZES: [[i32; 3]; 3] = [[750, 375, 187], [3000, 1500, 375], [12000, 6000, 1500]];
/// What a money power-up is worth in the easy race, and in the medium and hard races by the
/// player's rank (1 to 5, 6 to 10, 11 to 15, 16 to 20); in the Adversary's race.
const PICKUP_EASY: i32 = 50;
const PICKUP_BY_RANK: [[i32; 2]; 4] = [[260, 500], [200, 300], [120, 150], [60, 80]];
const PICKUP_LEADER: i32 = 400;
/// A wreck's damage, and the damage under which a race counts as clean.
const WRECKED: i32 = 100;
const CLEAN: i32 = 3;
/// The last place, which takes nobody's money power-ups.
const LAST: i32 = 4;

impl Campaign {
    /// The books of the race the player was in, the racers in their places on the grid as
    /// [`Campaign::racers`].
    pub(crate) fn settle(&mut self, outcome: &Outcome) -> Books {
        let finish = |racer: usize| outcome.finishes.get(racer).copied().unwrap_or_default();
        let me = self.player_racer();
        let mine = finish(me);
        let race = self.entered_race.unwrap_or(0);
        let mut books = Books {
            place: mine.place,
            ..Books::default()
        };
        let leading = self.player_leads();
        if !leading {
            // 0x4335C4: everyone else wrecked, a clean race, a win in a row.
            let wrecked = (0..self.racers.len())
                .filter(|&racer| racer != me && finish(racer).damage == WRECKED)
                .count();
            if wrecked + 1 == self.racers.len() && mine.damage < WRECKED {
                self.all_wrecked = true;
            }
            if mine.place != LAST && mine.damage < CLEAN && self.use_weapons {
                self.clean_race = true;
            }
            self.win_streak = if mine.place == 1 {
                self.win_streak + 1
            } else {
                0
            };
            // 0x43367D: the drug run pays only for the bonus taken and a win in one piece.
            if self.drug_deal > 0 && !(outcome.bonus && mine.place == 1 && mine.damage < WRECKED) {
                self.drug_deal = -self.drug_deal;
            }
            // 0x4336AC: the hit pays only for the victim wrecked; a victim not in the race
            // counts as the racer at the place of the count of wrecks.
            if self.hit > 0 {
                let victim = self
                    .racers
                    .iter()
                    .rposition(|racer| racer.driver == self.hit_victim)
                    .unwrap_or(wrecked);
                if finish(victim).damage != WRECKED {
                    self.hit = -self.hit;
                }
            }
        }
        // 0x433720: the loan's races; every driver's weapons gone; the market restocked.
        let player = self.player_mut();
        if player.loan != -1 {
            player.loan_races += 1;
        }
        for record in &mut self.drivers {
            record.mines = 0;
            record.spikes = 0;
            record.rocket = 0;
            record.sabotage = 0;
        }
        self.stock = [1, 1, 1, i32::from(!leading)];
        for racer in &self.racers {
            self.drivers[racer.driver].last_income = 0;
        }
        let best = self.best_other_points();
        if self.player().points > best || outcome.lapped {
            // 0x433D0F: the Adversary's race pays the money power-ups only.
            if self.player().points > best {
                books.picked_up = mine.money * PICKUP_LEADER;
                self.player_mut().last_income = books.picked_up;
            }
        } else {
            let rank = self.player().rank;
            let worth = match rank {
                1..=5 => Some(PICKUP_BY_RANK[0]),
                6..=10 => Some(PICKUP_BY_RANK[1]),
                11..=15 => Some(PICKUP_BY_RANK[2]),
                16..=20 => Some(PICKUP_BY_RANK[3]),
                _ => None,
            };
            for (racer, entry) in self.racers.clone().iter().enumerate() {
                let it = finish(racer);
                let record = &mut self.drivers[entry.driver];
                if mine.place != LAST {
                    let each = match (race, worth) {
                        (0, _) => Some(PICKUP_EASY),
                        (1, Some([medium, _])) => Some(medium),
                        (2, Some([_, hard])) => Some(hard),
                        _ => None,
                    };
                    if let Some(each) = each {
                        record.last_income = it.money * each;
                        if racer == me {
                            books.picked_up = record.last_income;
                        }
                    }
                }
                if (1..=3).contains(&it.place) && it.damage != WRECKED {
                    let prize = PRIZES[race.min(2)][(it.place - 1) as usize];
                    record.last_income += prize;
                    if racer == me {
                        books.prize = prize;
                    }
                }
                record.money += record.last_income;
                record.total_income += record.last_income;
                if it.place == 1 {
                    record.wins += 1;
                }
            }
        }
        // 0x43443C: the races and the damage carried over.
        if self.player_leads() {
            let record = self.player_mut();
            record.races += 1;
            record.damage = finish(1).damage;
        } else {
            for (racer, entry) in self.racers.clone().iter().enumerate() {
                let record = &mut self.drivers[entry.driver];
                record.races += 1;
                record.damage = finish(racer).damage;
            }
        }
        books
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::{PLAYER, Racer};

    /// A game in the medium race: the player (driver 19, rank 7) in place 1 on the grid
    /// between drivers 3, 5 and 8, each with a little of everything to lose.
    fn campaign() -> Campaign {
        let mut campaign = Campaign::new(1);
        for (driver, record) in campaign.drivers.iter_mut().enumerate() {
            record.points = driver as i32;
            record.money = 1000;
            record.mines = 2;
            record.spikes = 1;
            record.rocket = 1;
            record.sabotage = 1;
            record.loan = -1;
        }
        campaign.drivers[PLAYER].points = 5;
        campaign.drivers[PLAYER].rank = 7;
        campaign.entered_race = Some(1);
        campaign.racers = [3, PLAYER, 5, 8]
            .map(|driver| Racer {
                driver,
                rocket: 0,
                spikes: 0,
                mines: 0,
                adversary: false,
            })
            .to_vec();
        campaign
    }

    /// The Arena, the player leading with 150 points: the Adversary first on the grid, the
    /// player second (both the player's record).
    fn arena() -> Campaign {
        let mut campaign = campaign();
        campaign.drivers[PLAYER].points = 150;
        campaign.drivers[PLAYER].damage = 37;
        campaign.entered_race = Some(crate::campaign::ARENA);
        let racer = Racer {
            driver: PLAYER,
            rocket: 0,
            spikes: 0,
            mines: 0,
            adversary: false,
        };
        campaign.racers = vec![
            Racer {
                adversary: true,
                spikes: 1,
                ..racer
            },
            racer,
        ];
        campaign
    }

    /// In the Arena the player's books are their own car's, the second (0x45FC20 is 1): its
    /// place, its money power-ups at $400 each and no prize (0x433D0F), its damage carried
    /// over (0x4344CF); the Adversary's car, first on the grid, counts for nothing, and no
    /// sponsor hears of the race.
    #[test]
    fn the_arena_pays_the_players_own_car_its_money_power_ups_only() {
        let mut campaign = arena();
        let outcome = Outcome {
            finishes: vec![finish(1, 20, 5), finish(2, 40, 2)],
            ..Outcome::default()
        };
        let books = campaign.settle(&outcome);
        assert_eq!(
            books,
            Books {
                place: 2,
                picked_up: 800,
                prize: 0
            }
        );
        let player = campaign.drivers[PLAYER];
        assert_eq!((player.damage, player.races, player.wins), (40, 1, 0));
        assert_eq!(player.last_income, 800);
        assert_eq!(player.money, 1000, "the money power-ups are not paid out");
        assert_eq!(campaign.win_streak, 0);
    }

    fn finish(place: i32, damage: i32, money: i32) -> Finish {
        Finish {
            place,
            damage,
            money,
        }
    }

    /// The medium race pays 3000, 1500 and 375 for the first three places, nothing to a
    /// wreck, and each money power-up 200 to a racer while the player is ranked 6 to 10; a
    /// win counts; every driver's weapons are gone after; the damage carries over.
    #[test]
    fn the_medium_race_pays_its_prizes_and_money_power_ups() {
        let mut campaign = campaign();
        let outcome = Outcome {
            finishes: vec![
                finish(2, 40, 0),
                finish(1, 10, 2),
                finish(3, 100, 1),
                finish(4, 0, 3),
            ],
            ..Outcome::default()
        };
        let books = campaign.settle(&outcome);
        assert_eq!(
            books,
            Books {
                place: 1,
                picked_up: 400,
                prize: 3000
            }
        );
        let money = |driver: usize| campaign.drivers[driver].money;
        assert_eq!(
            [money(3), money(PLAYER), money(5), money(8)],
            [2500, 4400, 1200, 1600]
        );
        assert_eq!(campaign.drivers[PLAYER].wins, 1);
        assert_eq!(campaign.drivers[5].damage, 100);
        assert_eq!(campaign.drivers[PLAYER].races, 1);
        assert!(
            campaign
                .drivers
                .iter()
                .all(|d| d.mines + d.spikes + d.rocket + d.sabotage == 0)
        );
        assert_eq!(campaign.win_streak, 1);
    }

    /// A player in the last place takes nobody's money power-ups, the prizes still go out; a
    /// player lapped gets nothing at all, nor does anyone else.
    #[test]
    fn last_place_or_lapped_cuts_the_money() {
        let mut campaign = campaign();
        let outcome = Outcome {
            finishes: vec![
                finish(1, 0, 5),
                finish(4, 0, 5),
                finish(2, 0, 5),
                finish(3, 0, 5),
            ],
            ..Outcome::default()
        };
        campaign.settle(&outcome);
        assert_eq!(campaign.drivers[3].money, 1000 + 3000);
        assert_eq!(campaign.drivers[PLAYER].money, 1000);
        let mut campaign = self::campaign();
        let lapped = Outcome {
            lapped: true,
            ..outcome
        };
        campaign.settle(&lapped);
        assert_eq!(campaign.drivers[3].money, 1000);
    }

    /// The drug run pays only for the bonus taken and a win in one piece; the hit only for its
    /// victim wrecked: otherwise they turn negative, failed.
    #[test]
    fn the_deals_are_settled_by_the_race() {
        let outcome = Outcome {
            finishes: vec![
                finish(2, 100, 0),
                finish(1, 0, 0),
                finish(3, 0, 0),
                finish(4, 0, 0),
            ],
            bonus: true,
            ..Outcome::default()
        };
        let mut campaign = campaign();
        campaign.drug_deal = 3;
        campaign.hit = 2;
        campaign.hit_victim = 3;
        campaign.settle(&outcome);
        assert_eq!((campaign.drug_deal, campaign.hit), (3, 2));
        let mut campaign = self::campaign();
        campaign.drug_deal = 3;
        campaign.hit = 2;
        campaign.hit_victim = 5;
        campaign.settle(&Outcome {
            bonus: false,
            ..outcome
        });
        assert_eq!((campaign.drug_deal, campaign.hit), (-3, -2));
    }
}
