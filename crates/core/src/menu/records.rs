//! The lap and race records as DeadRally keeps them (docs/design/2026-10-10-records-and-fame-design.md):
//! the player's from `dr.cfg`, and where none is set, Remedy's: the original's lap records,
//! and the Arena's laps and every race, which the original has no records of, as Remedy might
//! have driven them.

use deadrally_gamedata::dr_cfg::ARENA;

use super::Menu;

/// Remedy's Arena lap records, by car: a Dervish's lap of the Arena measured at 17.25 s in
/// DeadRally's tests, at the pace Remedy's Holocaust record for the Dervish (26.96 s) keeps over
/// the same driving there (31.07 s), and each other car at the pace of its records against the
/// Dervish's, on average over the eighteen circuits.
const REMEDY_ARENA_LAPS: [[u32; 3]; 6] = [
    [0, 15, 76],
    [0, 14, 97],
    [0, 14, 25],
    [0, 13, 62],
    [0, 12, 71],
    [0, 11, 57],
];

/// Remedy's race over `laps` laps from its lap record: each lap 1.5 % to 3.5 % slower than
/// the best, by a spread fixed for each record so that they do not look made by a rule, and a
/// second more for the standing start.
fn remedy_race([minutes, seconds, hundredths]: [u32; 3], laps: i32, spread: u32) -> [u32; 3] {
    let lap = u64::from(minutes) * 6000 + u64::from(seconds) * 100 + u64::from(hundredths);
    let laps = u64::try_from(laps).unwrap_or(0);
    let total = lap * laps * (1015 + u64::from(spread % 21)) / 1000 + 100;
    [total / 6000, total / 100 % 60, total % 100].map(|part| part as u32)
}

/// The spread of the record of `car` over `laps` laps of `circuit`: 0 to 20 thousandths.
fn spread(circuit: usize, laps: i32, car: usize) -> u32 {
    let key = (circuit as u32) * 7 + (car as u32) * 13 + (laps as u32) * 31;
    key.wrapping_mul(2_654_435_761) >> 27
}

impl Menu {
    /// Remedy's name, as its records in the original's `dr.cfg` have it.
    fn remedy(&self) -> Vec<u8> {
        self.assets.menu.default_config.record(0, 0).0.to_vec()
    }

    /// The lap record of `car` on `circuit` (or [`ARENA`]): the driver and the time.
    pub(super) fn lap_record(&self, circuit: usize, car: usize) -> (Vec<u8>, [u32; 3]) {
        let (driver, time) = self.config.record(circuit, car);
        if circuit == ARENA && time == [0; 3] {
            return (self.remedy(), REMEDY_ARENA_LAPS[car]);
        }
        (driver.to_vec(), time)
    }

    /// The race record of `car` over `laps` laps of `circuit` (or [`ARENA`]): the driver and
    /// the time; Remedy's from its lap record there while none is set.
    pub(super) fn race_record(&self, circuit: usize, laps: i32, car: usize) -> (Vec<u8>, [u32; 3]) {
        let (driver, time) = self.config.race_record(circuit, laps, car);
        if time != [0; 3] {
            return (driver.to_vec(), time);
        }
        let lap = if circuit == ARENA {
            REMEDY_ARENA_LAPS[car]
        } else {
            self.assets.menu.default_config.record(circuit, car).1
        };
        (
            self.remedy(),
            remedy_race(lap, laps, spread(circuit, laps, car)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remedy_s_races_are_its_laps_a_little_slower_and_a_standing_start() {
        // A race record a player can beat by driving every lap near the lap record, and never
        // faster than the lap record allows.
        let lap = [0, 11, 36];
        assert_eq!(remedy_race(lap, 4, 0), [0, 47, 12], "4 x 11.36 x 1.015 + 1");
        assert_eq!(remedy_race(lap, 4, 20), [0, 48, 3], "4 x 11.36 x 1.035 + 1");
        assert_eq!(
            remedy_race([1, 0, 0], 6, 21),
            remedy_race([1, 0, 0], 6, 0),
            "0 to 20"
        );
        for circuit in 0..19 {
            for laps in [4, 5, 6, 9] {
                for car in 0..6 {
                    assert!(spread(circuit, laps, car) < 32);
                }
            }
        }
    }
}
