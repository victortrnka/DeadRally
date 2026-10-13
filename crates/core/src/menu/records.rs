//! The lap and race records as DeadRally keeps them (docs/design/2026-10-10-records-and-fame-design.md):
//! the player's from `dr.cfg`, and where none is set, Remedy's: the original's lap records,
//! and the Arena's laps and every race, which the original has no records of, as Remedy might
//! have driven them.

use deadrally_gamedata::dr_cfg::{ARENA, ARENA_LAPS, DrCfg, RACE_LAPS};

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

    /// How many lap records and race records the player holds.
    pub(super) fn records_held(&self) -> (usize, usize) {
        held(&self.config, self.campaign.player().name())
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

/// How many lap records and race records `config` gives `name`, as the records keep a name
/// (upper-cased, up to 11 letters); none to no name.
fn held(config: &DrCfg, name: &[u8]) -> (usize, usize) {
    let name = name[..name.len().min(11)].to_ascii_uppercase();
    if name.is_empty() {
        return (0, 0);
    }
    let circuits = (0..18).chain([ARENA]);
    let laps = circuits
        .clone()
        .flat_map(|circuit| (0..6).map(move |car| config.record(circuit, car).0))
        .filter(|driver| *driver == name.as_slice())
        .count();
    let races = circuits
        .flat_map(|circuit| {
            let counts: Vec<i32> = if circuit == ARENA {
                vec![ARENA_LAPS]
            } else {
                RACE_LAPS.to_vec()
            };
            counts.into_iter().flat_map(move |laps| {
                (0..6).map(move |car| config.race_record(circuit, laps, car).0)
            })
        })
        .filter(|driver| *driver == name.as_slice())
        .count();
    (laps, races)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_statistics_count_the_records_the_player_holds_by_name() {
        // The records keep a name upper-cased and cut to 11 letters, so the player's own are
        // found that way, the Arena's and the races' too; another driver's are not theirs.
        let mut config = DrCfg::parse(&[0; 8]).unwrap();
        config.set_record(14, 0, b"Victor Trnka", [0, 13, 1]);
        config.set_record(7, 1, b"VICTOR TRNKA", [0, 26, 50]);
        config.set_record(ARENA, 1, b"victor trnka", [0, 14, 80]);
        config.set_record(3, 2, b"Remedy", [0, 11, 0]);
        config.set_race_record(14, 4, 0, b"Victor Trnka", [0, 56, 90]);
        config.set_race_record(ARENA, ARENA_LAPS, 5, b"Someone", [2, 0, 0]);
        assert_eq!(held(&config, b"Victor Trnka"), (3, 1));
        assert_eq!(held(&config, b"Remedy"), (1, 0));
        assert_eq!(held(&config, b""), (0, 0));
    }

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
