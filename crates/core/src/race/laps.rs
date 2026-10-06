//! Laps and places (spec M4c), `checkVaiZones` (0x412DF0) each tick: the track's zone map
//! (`-VAI.BPK`, a byte for each 4x4 pixels) numbers the zones round the track; a car moves on
//! a zone when either front wheel reaches the next, finishes a lap when it reaches the last,
//! and goes back to none when it stands on the last zone without having come round to it. A
//! car ahead by zone and lap takes a place from one behind it; the player hears the last lap,
//! a lap record and being lapped. Wrecks drop to the last places (`sub_413380`).

use super::driving::Car;
use super::raster::ftol;

/// The zone map and the number of zones (`INF.BIN`'s third number).
pub(super) struct Zones<'a> {
    pub(super) map: &'a [u8],
    /// The map's width, the track's quarter.
    pub(super) width: i32,
    pub(super) count: i32,
}

impl Zones<'_> {
    /// The zone under (`x`, `y`); the original reads past the map unchecked, nothing out there
    /// is a zone here.
    fn at(&self, x: f32, y: f32) -> i32 {
        let row = ftol(f64::from(y)) >> 2;
        let column = ftol(f64::from(x)) >> 2;
        usize::try_from(row.wrapping_mul(self.width).wrapping_add(column))
            .ok()
            .and_then(|at| self.map.get(at))
            .map_or(0, |&zone| i32::from(zone))
    }
}

/// A lap's time: minutes, seconds and hundredths.
pub(super) type Time = [i32; 3];

/// The time in `ticks` as the HUD counts it (0x414282): 70 ticks a second, the hundredths
/// from the ticks left over times 1.42.
pub(super) fn time(ticks: i32) -> Time {
    let seconds = ticks / 70;
    [
        seconds / 60,
        seconds % 60,
        ftol(f64::from(ticks % 70) * 1.42),
    ]
}

fn hundredths([minutes, seconds, hundredths]: Time) -> i32 {
    (minutes * 60 + seconds) * 100 + hundredths
}

/// What the race keeps of laps beyond the cars' own counts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Laps {
    /// The race's and the player's lap's ticks (0x50A164, 0x481E0C).
    pub(super) race_clock: i32,
    pub(super) lap_clock: i32,
    /// The player's best lap (0x463CAC) and the record a lap must beat for its sound
    /// (0x50A160).
    pub(super) best: Time,
    pub(super) record: Time,
    /// The ticks the last lap's time stays shown (0x4A9EB0) and that time (0x503224).
    pub(super) shown: i32,
    pub(super) last_lap: i32,
    /// The ticks until the lap's call (0x456AE4).
    pub(super) call: i32,
    /// Whether a car has finished, which finishes every car at its line (0x456AC8), and
    /// whether the player has heard they are lapped (0x456BC0).
    pub(super) over: bool,
    pub(super) lapped: bool,
}

impl Laps {
    /// The time a race without weapons shows (0x414B82): the last lap's while it is shown
    /// (0x4A9EB0 above 0), else the lap's clock.
    pub(super) fn time_shown(&self) -> i32 {
        if self.shown > 0 {
            self.last_lap
        } else {
            self.lap_clock
        }
    }

    /// The last lap's time shown for `between` ticks less (0x414C61), down to 0.
    pub(super) fn count_down(&mut self, between: i32) {
        if self.shown > 0 {
            self.shown = (self.shown - between).max(0);
        }
    }
}

/// The race's calls on channel 2 (pitch 0x50000): the last lap, a lap record, the player
/// lapped, the tough driver winning.
pub(super) const LAST_LAP: u8 = 2;
pub(super) const RECORD: u8 = 4;
pub(super) const LAPPED: u8 = 22;
pub(super) const TOUGH_WINS: u8 = 31;
/// The ticks the last lap's call waits, and a lap's time stays shown.
const CALL_WAIT: i32 = 210;

/// What the zones' check asks the race to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Call(pub(super) u8);

/// The race's facts the check needs: the player's place on the grid, the race's laps,
/// whether the track is the intro's (no record call there), whether the player's car is the
/// Adversary's (no lapped call), and which car belongs to the tough driver.
pub(super) struct Race {
    pub(super) player: usize,
    pub(super) laps: i32,
    pub(super) intro_track: bool,
    pub(super) special: bool,
    pub(super) tough: Option<usize>,
}

/// `checkVaiZones` for every car; the calls it makes, in order.
pub(super) fn check(cars: &mut [Car], zones: &Zones, laps: &mut Laps, race: &Race) -> Vec<Call> {
    let mut calls = Vec::new();
    for car in cars.iter_mut() {
        let next = car.zone + 1;
        let [front_left, front_right, ..] = car.wheels;
        if zones.at(front_left[0], front_left[1]) == next
            || zones.at(front_right[0], front_right[1]) == next
        {
            car.zone += 1;
        }
        if zones.at(car.x, car.y) == zones.count && car.zone < zones.count {
            car.zone = 0;
        }
    }
    let n = cars.len();
    for i in 0..n {
        for j in 0..n {
            let (a, b) = (&cars[i], &cars[j]);
            if !b.finished
                && i != j
                && a.zone > b.zone
                && a.lap as u8 >= b.lap as u8
                && a.place as u8 > b.place as u8
            {
                let (pa, pb) = (cars[i].place, cars[j].place);
                cars[i].place = pb;
                cars[j].place = pa;
            }
        }
    }
    for (slot, car) in cars.iter_mut().enumerate() {
        if car.zone != zones.count {
            continue;
        }
        if laps.over {
            car.finished = true;
            car.handling.engine = 0.0;
        } else {
            car.lap = (car.lap + 1) & 0xFF;
            if slot == race.player {
                if car.lap == race.laps {
                    calls.push(Call(LAST_LAP));
                    laps.call = CALL_WAIT;
                }
                lap_times(laps, race, &mut calls);
            }
        }
        car.zone = 0;
        if car.lap > race.laps {
            car.lap = race.laps;
            car.finished = true;
            if slot == race.player {
                laps.shown = 9999;
                car.effect = 0;
            }
            car.handling.engine = 0.0;
            laps.over = true;
            if race.tough == Some(slot) && car.place == 1 {
                calls.push(Call(TOUGH_WINS));
            }
        }
    }
    let leader = (0..n)
        .filter(|&slot| slot != race.player)
        .map(|slot| cars[slot].lap * zones.count + cars[slot].zone)
        .fold(0, i32::max);
    let player = &cars[race.player];
    let mine = (player.lap + 1) * zones.count + player.zone;
    if !race.special {
        if leader > mine && !laps.lapped {
            laps.lapped = true;
            calls.push(Call(LAPPED));
        }
        if leader < mine && laps.lapped {
            laps.lapped = false;
        }
    }
    calls
}

/// The player's lap done: the best lap kept, a lap better than the record called and kept,
/// the lap's time shown for 210 ticks and its clock started again.
fn lap_times(laps: &mut Laps, race: &Race, calls: &mut Vec<Call>) {
    let lap = time(laps.lap_clock);
    if hundredths(lap) < hundredths(laps.best) || laps.best == [0; 3] {
        laps.best = lap;
    }
    if hundredths(lap) >= hundredths(laps.record) && laps.record != [0; 3] {
        laps.call = 0;
    } else {
        if !race.intro_track && laps.call == 0 {
            calls.push(Call(RECORD));
        }
        laps.record = lap;
    }
    laps.last_lap = laps.lap_clock;
    laps.shown = CALL_WAIT;
    laps.lap_clock = 0;
}

/// `sub_413380`: a wreck gives its place to any running car behind it, and the wrecks, in
/// the order they were wrecked, take the last places (`wrecks` the list, 0x4A7CE0).
pub(super) fn place_wrecks(cars: &mut [Car], wrecks: &mut Vec<usize>) {
    let n = cars.len();
    for i in 0..n {
        for j in 0..n {
            if i != j
                && cars[i].place as u8 > cars[j].place as u8
                && cars[i].handling.damage > 0
                && cars[j].handling.damage <= 0
            {
                let (pi, pj) = (cars[i].place, cars[j].place);
                cars[i].place = pj;
                cars[j].place = pi;
            }
        }
    }
    for (slot, car) in cars.iter().enumerate() {
        let listed = wrecks.contains(&slot);
        if car.handling.damage == 0 && !listed {
            wrecks.push(slot);
        }
        if car.handling.damage > 0 && listed {
            wrecks.retain(|&wreck| wreck != slot);
        }
    }
    for (index, &slot) in wrecks.iter().enumerate() {
        cars[slot].place = n as i32 - index as i32;
    }
}

/// The ticks the race's end comes nearer this tick (0x416A38, counted in 0x4AA508; the race
/// ends past 300): one once the player has finished or is wrecked, one more while every car
/// but one stands (0.5 or slower) finished or wrecked.
pub(super) fn ending(cars: &[Car], player: usize) -> i32 {
    let done = |car: &Car| car.finished || car.handling.damage <= 0;
    let mut nearer = i32::from(done(&cars[player]));
    let standing = cars
        .iter()
        .filter(|car| f64::from(car.speed) <= 0.5 && done(car))
        .count();
    if standing + 1 >= cars.len() {
        nearer += 1;
    }
    nearer
}

/// `sub_413300`, when the player abandons the race: while racing, the player swaps places
/// with each car still racing behind them, in the cars' order, and so ends behind them all.
pub(super) fn abandon(cars: &mut [Car], player: usize) {
    for slot in 0..cars.len() {
        if slot == player || cars[slot].finished || cars[player].finished {
            continue;
        }
        let (mine, theirs) = (cars[player].place, cars[slot].place);
        if (mine as u8) < (theirs as u8) {
            cars[slot].place = mine;
            cars[player].place = theirs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::driving::Handling;
    use deadrally_gamedata::handling::Guns;

    /// A 4-zone loop on a 40x8 pixel track: zones 1 to 4 along a row, 4 the last.
    const MAP: [u8; 10 * 2] = [1, 1, 2, 2, 3, 3, 4, 4, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 0];

    fn zones() -> Zones<'static> {
        Zones {
            map: &MAP,
            width: 10,
            count: 4,
        }
    }

    fn car(slot: usize) -> Car {
        let handling = Handling {
            car: 0,
            engine: 2.5,
            engine_backup: 2.5,
            tires: 0.5,
            size: 9.0,
            steering: 2.5,
            damage: 102_400,
            armour: 300,
            rocket: 0,
            weapons_bar: 102_400,
            turbo: 102_400,
            rocket_used: false,
            mines: 0,
            money: 0,
            weapons: true,
            guns: Guns::default(),
        };
        Car::new((2.0, 2.0, 72), slot, handling, 0)
    }

    /// The car's front wheels and middle at pixel column `x`.
    fn put(car: &mut Car, x: f32) {
        car.x = x;
        car.wheels = [[x, 1.0], [x, 5.0], [x, 1.0], [x, 5.0]];
    }

    fn race(laps: i32) -> Race {
        Race {
            player: 0,
            laps,
            intro_track: false,
            special: false,
            tough: None,
        }
    }

    /// Zones count only in order; the last zone is a lap, and the line can only be crossed by
    /// coming round to it: a car starting behind its line does not gain a lap.
    #[test]
    fn a_lap_takes_every_zone_in_order() {
        let mut cars = vec![car(0)];
        let mut laps = Laps::default();
        put(&mut cars[0], 28.0);
        check(&mut cars, &zones(), &mut laps, &race(4));
        assert_eq!(
            (cars[0].zone, cars[0].lap),
            (0, 1),
            "the last zone from nowhere"
        );
        put(&mut cars[0], 2.0);
        check(&mut cars, &zones(), &mut laps, &race(4));
        put(&mut cars[0], 20.0);
        check(&mut cars, &zones(), &mut laps, &race(4));
        assert_eq!(cars[0].zone, 1, "zone 3 jumped to does not count");
        put(&mut cars[0], 9.0);
        check(&mut cars, &zones(), &mut laps, &race(4));
        put(&mut cars[0], 20.0);
        check(&mut cars, &zones(), &mut laps, &race(4));
        put(&mut cars[0], 28.0);
        let calls = check(&mut cars, &zones(), &mut laps, &race(4));
        assert_eq!((cars[0].zone, cars[0].lap), (0, 2));
        assert_eq!(calls, [Call(RECORD)], "a first lap is a record");
    }

    /// Past the race's laps the car finishes, and every other car finishes at its own line.
    #[test]
    fn the_first_car_home_finishes_the_race_for_all() {
        let mut cars = vec![car(0), car(1)];
        let mut laps = Laps::default();
        cars[1].lap = 2;
        cars[1].zone = 3;
        put(&mut cars[1], 28.0);
        let calls = check(&mut cars, &zones(), &mut laps, &race(1));
        assert!(cars[1].finished && laps.over);
        assert_eq!(cars[1].lap, 1);
        assert_eq!(cars[1].handling.engine, 0.0);
        assert!(calls.is_empty());
        cars[0].zone = 3;
        put(&mut cars[0], 28.0);
        check(&mut cars, &zones(), &mut laps, &race(1));
        assert!(cars[0].finished);
        assert_eq!(cars[0].lap, 1, "no lap counted once the race is over");
    }

    /// A car further round on the same lap takes the place of one behind it.
    #[test]
    fn a_car_ahead_by_zone_takes_the_place() {
        let mut cars = vec![car(0), car(1)];
        let mut laps = Laps::default();
        put(&mut cars[0], 36.0);
        put(&mut cars[1], 36.0);
        cars[1].zone = 2;
        check(&mut cars, &zones(), &mut laps, &race(4));
        assert_eq!((cars[0].place, cars[1].place), (2, 1));
    }

    /// Wrecks drop behind every running car, the first wrecked last.
    #[test]
    fn wrecks_take_the_last_places_in_order() {
        let mut cars = vec![car(0), car(1), car(2)];
        let mut wrecks = Vec::new();
        cars[0].handling.damage = 0;
        place_wrecks(&mut cars, &mut wrecks);
        cars[1].handling.damage = 0;
        place_wrecks(&mut cars, &mut wrecks);
        assert_eq!(wrecks, [0, 1]);
        assert_eq!((cars[0].place, cars[1].place, cars[2].place), (3, 2, 1));
    }

    /// The HUD's clock: 70 ticks a second, the hundredths 1.42 a tick.
    #[test]
    fn lap_times_count_seventy_ticks_a_second() {
        assert_eq!(time(70 * 61 + 35), [1, 1, 49]);
    }

    /// After each of the player's laps a race without weapons shows that lap's time for 210
    /// ticks of the frames, then the new lap's clock again. A time left up too long, or never
    /// shown, misleads the player about the lap.
    #[test]
    fn a_lap_s_time_shows_for_210_ticks_then_the_clock_again() {
        let mut cars = vec![car(0)];
        let mut laps = Laps {
            lap_clock: 70 * 33,
            ..Laps::default()
        };
        assert_eq!(laps.time_shown(), 70 * 33, "the clock before the first lap");
        cars[0].zone = 3;
        put(&mut cars[0], 28.0);
        check(&mut cars, &zones(), &mut laps, &race(4));
        laps.lap_clock = 5;
        assert_eq!(laps.time_shown(), 70 * 33, "the lap's time");
        for _ in 0..104 {
            laps.count_down(2);
        }
        assert_eq!(laps.time_shown(), 70 * 33, "2 ticks still to go");
        laps.count_down(3);
        assert_eq!((laps.shown, laps.time_shown()), (0, 5), "the clock again");
        laps.count_down(3);
        assert_eq!(laps.shown, 0);
    }

    /// The race ends 300 ticks after the player is done: a tick counts once the player has
    /// finished or is wrecked, and once more while every car but one stands (0.5 or slower)
    /// finished or wrecked; cars standing still in the race count for nothing.
    #[test]
    fn the_end_comes_nearer_once_the_player_is_done() {
        let mut cars = vec![car(0), car(1), car(2)];
        assert_eq!(ending(&cars, 0), 0);
        cars[0].handling.damage = 0;
        assert_eq!(ending(&cars, 0), 1);
        cars[1].finished = true;
        assert_eq!(ending(&cars, 0), 2);
        cars[1].speed = 0.6;
        assert_eq!(ending(&cars, 0), 1);
        cars[0].handling.damage = 5;
        cars[0].finished = true;
        cars[1].speed = -3.0;
        assert_eq!(ending(&cars, 0), 2);
    }

    /// The player who abandons the race drops behind every car still racing; a car that has
    /// finished keeps its place.
    #[test]
    fn an_abandoning_player_drops_behind_the_cars_still_racing() {
        let mut cars = vec![car(0), car(1), car(2), car(3)];
        for (slot, place) in [(0, 2), (1, 1), (2, 3), (3, 4)] {
            cars[slot].place = place;
        }
        cars[1].finished = true;
        abandon(&mut cars, 0);
        let places: Vec<i32> = cars.iter().map(|car| car.place).collect();
        assert_eq!(places, [4, 1, 2, 3]);
    }
}
