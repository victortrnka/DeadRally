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

/// Remedy's race over `laps` laps from its lap record: each lap 5 % slower than the best
/// (the owner's choice: a clean race of his was 8.9 % slower), and a second more for the
/// standing start.
fn remedy_race([minutes, seconds, hundredths]: [u32; 3], laps: i32) -> [u32; 3] {
    let lap = u64::from(minutes) * 6000 + u64::from(seconds) * 100 + u64::from(hundredths);
    let laps = u64::try_from(laps).unwrap_or(0);
    let total = lap * laps * 105 / 100 + 100;
    [total / 6000, total / 100 % 60, total % 100].map(|part| part as u32)
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
        (self.remedy(), remedy_race(lap, laps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remedy_s_races_are_its_laps_5_percent_slower_and_a_standing_start() {
        // A race record a player can beat by driving every lap near the lap record: Palm Side
        // with the Vagabond (lap record 13.36) over 4 laps, which a clean race of the owner's
        // drove in 58.21.
        assert_eq!(
            remedy_race([0, 13, 36], 4),
            [0, 57, 11],
            "4 x 13.36 x 1.05 + 1"
        );
        assert_eq!(
            remedy_race([0, 26, 96], 6),
            [2, 50, 84],
            "6 x 26.96 x 1.05 + 1"
        );
    }
}
