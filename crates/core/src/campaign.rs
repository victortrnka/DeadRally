//! The campaign's state and the rules that need no pictures (spec M3a §3): the original's
//! random numbers, the twenty drivers, and the races offered at sign-up.

use deadrally_gamedata::text::CarSpec;

/// The original's `rand()`, the MSVC runtime's: a linear congruential generator whose bits
/// 16–30 are the result. Every call the original makes must be made here too, in the same
/// order: one call more or less changes the drivers and the races.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rand {
    state: u32,
}

impl Rand {
    /// `srand(seed)`.
    pub(crate) fn new(seed: u32) -> Rand {
        Rand { state: seed }
    }

    /// `rand()`: 0 to 32767.
    pub(crate) fn next(&mut self) -> i32 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((self.state >> 16) & 0x7FFF) as i32
    }
}

pub(crate) const DRIVERS: usize = 20;
/// The player's driver: `initDrivers` makes it 19.
pub(crate) const PLAYER: usize = 19;
pub(crate) const NAME_BYTES: usize = 12;
/// The money a new driver starts with.
const START_MONEY: i32 = 495;
/// The cars of drivers 0 to 18 at the start (`initDrivers`, 0x428D2C).
const START_CARS: [i32; PLAYER] = [5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 0, 0];
/// Drivers 0 to 18's points at the start: `trunc((100 − trunc(77 log10(i + 1)) + 5.5 (18 − i)
/// + 2) / 2)` (0x428DC0, x87); the player starts with none.
const START_POINTS: [i32; PLAYER] = [
    100, 86, 77, 69, 63, 57, 51, 46, 42, 37, 33, 28, 25, 20, 17, 13, 9, 5, 2,
];

/// One of the twenty drivers, as the original keeps it (108 bytes, saved as they are).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Driver {
    /// NUL-terminated; the bytes after the NUL are whatever an earlier name left there.
    pub(crate) name: [u8; NAME_BYTES],
    pub(crate) damage: i32,
    pub(crate) engine: i32,
    pub(crate) tires: i32,
    pub(crate) armour: i32,
    pub(crate) car: i32,
    pub(crate) unused: [i32; 3],
    pub(crate) colour: i32,
    pub(crate) money: i32,
    /// The loan shark's car (−1 none) and the races since the loan.
    pub(crate) loan: i32,
    pub(crate) loan_races: i32,
    pub(crate) car_price: i32,
    pub(crate) face: i32,
    pub(crate) points: i32,
    pub(crate) rank: i32,
    pub(crate) wins: i32,
    pub(crate) races: i32,
    pub(crate) last_income: i32,
    pub(crate) total_income: i32,
    pub(crate) mines: i32,
    pub(crate) spikes: i32,
    pub(crate) rocket: i32,
    pub(crate) sabotage: i32,
}

/// A driver's record in a saved game.
pub(crate) const DRIVER_BYTES: usize = 108;

impl Driver {
    /// The record's 24 numbers after the name, in the original's order.
    fn numbers(self) -> [i32; 24] {
        let [a, b, c] = self.unused;
        [
            self.damage,
            self.engine,
            self.tires,
            self.armour,
            self.car,
            a,
            b,
            c,
            self.colour,
            self.money,
            self.loan,
            self.loan_races,
            self.car_price,
            self.face,
            self.points,
            self.rank,
            self.wins,
            self.races,
            self.last_income,
            self.total_income,
            self.mines,
            self.spikes,
            self.rocket,
            self.sabotage,
        ]
    }

    /// The 108 bytes the original keeps and saves: the name, then 24 little-endian numbers.
    pub(crate) fn to_bytes(self) -> [u8; DRIVER_BYTES] {
        let mut bytes = [0; DRIVER_BYTES];
        bytes[..NAME_BYTES].copy_from_slice(&self.name);
        for (chunk, number) in bytes[NAME_BYTES..]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.numbers())
        {
            chunk.copy_from_slice(&number.to_le_bytes());
        }
        bytes
    }

    /// A record as [`Driver::to_bytes`] writes it.
    pub(crate) fn from_bytes(bytes: &[u8; DRIVER_BYTES]) -> Driver {
        let mut n = bytes[NAME_BYTES..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&chunk| i32::from_le_bytes(chunk));
        let mut next = || n.next().expect("24 numbers follow the name");
        let mut name = [0; NAME_BYTES];
        name.copy_from_slice(&bytes[..NAME_BYTES]);
        Driver {
            name,
            damage: next(),
            engine: next(),
            tires: next(),
            armour: next(),
            car: next(),
            unused: [next(), next(), next()],
            colour: next(),
            money: next(),
            loan: next(),
            loan_races: next(),
            car_price: next(),
            face: next(),
            points: next(),
            rank: next(),
            wins: next(),
            races: next(),
            last_income: next(),
            total_income: next(),
            mines: next(),
            spikes: next(),
            rocket: next(),
            sabotage: next(),
        }
    }

    /// Whether the record holds numbers a game can have, which the screens index tables
    /// with: a car 0 to 5, upgrade levels 0 to 4, a colour of `COPPER.PAL`'s 256, one of the
    /// 20 faces, damage 0 to 100 and money the side panel can print.
    pub(crate) fn is_playable(&self) -> bool {
        (0..=5).contains(&self.car)
            && [self.engine, self.tires, self.armour]
                .iter()
                .all(|level| (0..=4).contains(level))
            && (0..=255).contains(&self.colour)
            && (0..20).contains(&self.face)
            && (0..=100).contains(&self.damage)
            && (-9_999_999..=9_999_999).contains(&self.money)
    }

    /// The name up to its NUL.
    pub(crate) fn name(&self) -> &[u8] {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(NAME_BYTES);
        &self.name[..end]
    }

    /// `strcpy` into the name: the bytes after the copied NUL stay as they were.
    pub(crate) fn set_name(&mut self, name: &[u8]) {
        let length = name.len().min(NAME_BYTES - 1);
        self.name[..length].copy_from_slice(&name[..length]);
        self.name[length] = 0;
    }
}

/// `initDrivers` (0x428930): the drivers of a new game. The player's name, face and colour
/// are kept; `names[face]` names the others.
pub(crate) fn init_drivers(
    drivers: &mut [Driver; DRIVERS],
    rand: &mut Rand,
    cars: &[CarSpec],
    names: &[Vec<u8>],
) {
    for driver in drivers.iter_mut() {
        driver.mines = 0;
        driver.spikes = 0;
        driver.rocket = 0;
        driver.sabotage = 0;
    }
    for (driver, &car) in drivers.iter_mut().zip(&START_CARS) {
        driver.car = car;
    }
    drivers[PLAYER].car = 0;
    for (driver, &points) in drivers.iter_mut().zip(&START_POINTS) {
        driver.points = points;
    }
    let player_face = drivers[PLAYER].face;
    let mut taken = [false; DRIVERS];
    for index in 0..PLAYER {
        let spec = cars[drivers[index].car as usize];
        let driver = &mut drivers[index];
        driver.wins = 0;
        driver.races = 0;
        driver.last_income = 0;
        driver.total_income = 0;
        driver.money = rand.next() % 100_000;
        driver.damage = 0;
        driver.engine = rand.next() % spec.upgrades[0];
        driver.tires = rand.next() % spec.upgrades[1];
        driver.armour = rand.next() % spec.upgrades[2];
        driver.car_price = spec.price;
        driver.rank = index as i32 + 1;
        // The first face from the driver's index on that no one took and is not the player's.
        let mut face = index;
        while taken[face] || face as i32 == player_face {
            face += 1;
        }
        taken[face] = true;
        driver.set_name(&names[face]);
        driver.face = face as i32;
        driver.colour = face as i32;
    }
    let player = &mut drivers[PLAYER];
    player.money = START_MONEY;
    player.damage = 0;
    player.engine = 0;
    player.armour = 0;
    player.tires = 0;
    player.car = 0;
    player.car_price = cars[0].price;
    player.rank = PLAYER as i32 + 1;
    player.wins = 0;
    player.races = 0;
    player.last_income = 0;
    player.points = 0;
    player.total_income = 0;
    player.loan = -1;
    player.loan_races = -1;
}

/// A game's state between screens: the drivers and the flags `startRacingMenu` (0x439CD0)
/// and the screens after it keep.
#[derive(Clone, Debug)]
pub(crate) struct Campaign {
    pub(crate) rand: Rand,
    pub(crate) drivers: [Driver; DRIVERS],
    /// Whether weapons are on (0x4456E4); the original starts with them on.
    pub(crate) use_weapons: bool,
    /// A game is on: the Start Racing menu's first row leads to the shop.
    pub(crate) started: bool,
    pub(crate) last_circuits: LastCircuits,
    /// The race the sign-up's border is on (`selectedRaceId`), kept between sign-ups.
    pub(crate) selected_race: usize,
    /// The warnings and popups a new game shows once (0x456B80 hard race, 0x456B7C medium
    /// race, 0x456B78 the Underground Market, 0x456B74 welcome).
    pub(crate) warn_hard: bool,
    pub(crate) warn_medium: bool,
    pub(crate) underground_popup: bool,
    pub(crate) welcome: bool,
    /// The sign-up on screen, and the race the player is in.
    pub(crate) sign_up: Option<SignUp>,
    pub(crate) entered_race: Option<usize>,
    /// The hitman's chance in percent (0x45678C): 5 at first, 2 more after each sign-up he
    /// does not come.
    pub(crate) hitman_chance: i32,
    /// The Underground Market's mines, spikes, rocket fuel and sabotage (0x45EFF0..0x45EFFC):
    /// 1 on sale, 0 sold out, −1 locked (the shareware's; the Windows version sets none).
    pub(crate) stock: [i32; 4],
    /// What `SDL_GetTicks()` gave `mainMenu`'s `srand`; the sabotage seeds `rand()` again
    /// from the clock (0x42DEE1), which runs on 14 ms a tick from there.
    pub(crate) clock: u32,
    /// The clock fixed instead, as the reference runner fixes the original's.
    pub(crate) fixed_clock: Option<u32>,
    /// The deals taken after a sign-up: the drug run's level (0x456BB4), and the hitman's
    /// (0x456BB8) with his victim (0x456BBC); 0 for none. Races settle them (M5).
    pub(crate) drug_deal: i32,
    pub(crate) hit: i32,
    pub(crate) hit_victim: usize,
    /// The offer on screen, waiting for its answer.
    pub(crate) offer: Option<Offer>,
}

/// An offer after a sign-up (0x431B30): its level (1 with the best car to 6 with the
/// Vagabond) and, for the hitman, his victim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Offer {
    Drugs { level: i32 },
    Hit { level: i32, victim: usize },
}

impl Campaign {
    /// No game yet; `rand()` seeded with `seed`.
    pub(crate) fn new(seed: u32) -> Campaign {
        Campaign {
            rand: Rand::new(seed),
            drivers: [Driver::default(); DRIVERS],
            use_weapons: true,
            started: false,
            last_circuits: LastCircuits::default(),
            selected_race: 0,
            warn_hard: false,
            warn_medium: false,
            underground_popup: false,
            welcome: false,
            sign_up: None,
            entered_race: None,
            hitman_chance: 5,
            stock: [1; 4],
            clock: seed,
            fixed_clock: None,
            drug_deal: 0,
            hit: 0,
            hit_victim: 0,
            offer: None,
        }
    }

    /// Whether the player has more points than every other driver (the final race against
    /// the Adversary is due).
    pub(crate) fn player_leads(&self) -> bool {
        let best = self
            .drivers
            .iter()
            .enumerate()
            .filter(|&(index, _)| index != PLAYER)
            .map(|(_, driver)| driver.points)
            .fold(0, i32::max);
        self.player().points > best
    }

    /// The market restocked (0x4236D0, from `initDrivers` on): everything on sale but the
    /// sabotage while the player leads.
    pub(crate) fn restock(&mut self) {
        self.stock = [1, 1, 1, i32::from(!self.player_leads())];
    }

    /// A loaded game's stock (0x42F6A1): what the player's car is not already full of.
    pub(crate) fn stock_from_player(&mut self) {
        let player = *self.player();
        self.stock = [
            player.mines != 8,
            player.spikes != 1,
            player.rocket != 1,
            player.sabotage != 1,
        ]
        .map(i32::from);
    }

    pub(crate) fn player(&self) -> &Driver {
        &self.drivers[PLAYER]
    }

    pub(crate) fn player_mut(&mut self) -> &mut Driver {
        &mut self.drivers[PLAYER]
    }
}

/// The three races of a sign-up (`calculateNextRaces` 0x4240B0, `addParticipantToRace`
/// 0x423A20): their circuits, and who signed up for each in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SignUp {
    pub(crate) circuits: [usize; 3],
    pub(crate) entrants: [[usize; 4]; 3],
    pub(crate) counts: [usize; 3],
    taken: [bool; DRIVERS],
}

/// The circuits last offered in each column (0x456780), −1 before the first sign-up; a
/// column never offers the same circuit twice in a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LastCircuits(pub(crate) [i32; 3]);

impl Default for LastCircuits {
    fn default() -> LastCircuits {
        LastCircuits([-1; 3])
    }
}

/// Circuits from `order[first..first + count]`, mirrored (+9) on an even `rand()`.
fn draw_circuit(rand: &mut Rand, order: &[u8], first: usize, count: i32) -> usize {
    let circuit = usize::from(order[first + (rand.next() % count) as usize]);
    if rand.next() % 2 == 0 {
        circuit + 9
    } else {
        circuit
    }
}

impl SignUp {
    /// `calculateNextRaces`: the easy column from the first five circuits of `order`, the
    /// medium from the six from the third on, the hard from the four from the sixth on. The
    /// player is signed up for none yet but counts as taken.
    pub(crate) fn new(rand: &mut Rand, order: &[u8], last: &mut LastCircuits) -> SignUp {
        let mut circuits = [0; 3];
        loop {
            circuits[0] = draw_circuit(rand, order, 0, 5);
            if last.0[0] != circuits[0] as i32 {
                break;
            }
        }
        last.0[0] = circuits[0] as i32;
        loop {
            circuits[1] = draw_circuit(rand, order, 2, 6);
            if last.0[1] != circuits[1] as i32 && circuits[0] != circuits[1] {
                break;
            }
        }
        last.0[1] = circuits[1] as i32;
        loop {
            circuits[2] = draw_circuit(rand, order, 5, 4);
            if last.0[2] != circuits[2] as i32 && circuits[1] != circuits[2] {
                break;
            }
        }
        last.0[2] = circuits[2] as i32;
        let mut taken = [false; DRIVERS];
        taken[PLAYER] = true;
        SignUp {
            circuits,
            entrants: [[0; 4]; 3],
            counts: [0; 3],
            taken,
        }
    }

    pub(crate) fn full(&self) -> bool {
        self.counts.iter().all(|&count| count >= 4)
    }

    /// Signs `driver` up for `race`; the entry's place, from 0.
    pub(crate) fn enter(&mut self, race: usize, driver: usize) -> usize {
        let place = self.counts[race];
        self.taken[driver] = true;
        self.entrants[race][place] = driver;
        self.counts[race] = place + 1;
        place
    }

    /// `addParticipantToRace(chance)`: with a chance of one in `chance`, a driver whose car
    /// fits signs up for a random race that has room; the race and its new entry's place.
    pub(crate) fn add_driver(
        &mut self,
        chance: i32,
        rand: &mut Rand,
        drivers: &[Driver; DRIVERS],
    ) -> Option<(usize, usize)> {
        if rand.next() % chance != 0 {
            return None;
        }
        let mut tries = 0;
        let race = loop {
            tries += 1;
            let race = (rand.next() % 3) as usize;
            if self.counts[race] <= 3 {
                break race;
            }
            if tries >= 50 {
                return None;
            }
        };
        // A race found on the fiftieth try is not used.
        if tries >= 50 {
            return None;
        }
        let car = |driver: usize| drivers[driver].car;
        let player_car = drivers[PLAYER].car;
        let driver = loop {
            let mut driver = 0;
            for _ in 0..100 {
                driver = (rand.next() % 20) as usize;
                let fits = match race {
                    0 => (0..=2).contains(&car(driver)),
                    1 => {
                        ((0..=2).contains(&player_car) && (1..=3).contains(&car(driver)))
                            || ((3..=5).contains(&player_car) && (2..=4).contains(&car(driver)))
                    }
                    _ => (3..=5).contains(&car(driver)),
                };
                if fits {
                    break;
                }
            }
            if !self.taken[driver] {
                break driver;
            }
        };
        Some((race, self.enter(race, driver)))
    }

    /// Escape at the sign-up (0x435B20): drivers sign up with no chance against them until
    /// every race is full, at least one call even when they already are. The new entries,
    /// race and place, in order.
    pub(crate) fn fill_at_once(
        &mut self,
        rand: &mut Rand,
        drivers: &[Driver; DRIVERS],
    ) -> Vec<(usize, usize)> {
        let mut entries = Vec::new();
        loop {
            entries.extend(self.add_driver(1, rand, drivers));
            if self.full() {
                return entries;
            }
        }
    }

    /// The order the entrants of each race line up in (`selectRaceScreen`, after the
    /// sign-up): by driver index, highest first.
    pub(crate) fn sort_entrants(&mut self) {
        for race in &mut self.entrants {
            race.sort_unstable_by(|a, b| b.cmp(a));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cars() -> Vec<CarSpec> {
        (0..6)
            .map(|k| CarSpec {
                price: 500 * (k + 1),
                upgrades: [1 + k % 4, 2, 4],
                upgrade_prices: [[0; 4]; 3],
                repair_price: 10,
            })
            .collect()
    }

    fn names() -> Vec<Vec<u8>> {
        (0..DRIVERS)
            .map(|face| format!("N{face}").into_bytes())
            .collect()
    }

    #[test]
    fn rand_follows_the_msvc_runtime() {
        // The original's drivers and races come from this sequence; any other generator
        // offers other circuits and other opponents for the same seed.
        let mut rand = Rand::new(1);
        let first: Vec<i32> = (0..5).map(|_| rand.next()).collect();
        assert_eq!(first, [41, 18467, 6334, 26500, 19169]);
        let mut zero = Rand::new(0);
        assert_eq!(zero.next(), 38);
    }

    #[test]
    fn a_new_game_starts_the_player_last_with_495_dollars_and_a_vagabond() {
        // The first race's prices, the shop and the standings all start from these.
        let mut drivers = [Driver::default(); DRIVERS];
        drivers[PLAYER].set_name(b"me");
        drivers[PLAYER].face = 3;
        drivers[PLAYER].colour = 40;
        init_drivers(&mut drivers, &mut Rand::new(1), &cars(), &names());
        let player = &drivers[PLAYER];
        assert_eq!(
            (player.money, player.car, player.rank, player.points),
            (495, 0, 20, 0)
        );
        assert_eq!(
            (player.loan, player.loan_races, player.car_price),
            (-1, -1, 500)
        );
        assert_eq!(
            (player.name(), player.face, player.colour),
            (b"me".as_slice(), 3, 40),
            "the licence's choices stay"
        );
    }

    #[test]
    fn the_other_drivers_take_the_faces_in_order_skipping_the_players() {
        // The player's face is theirs alone; the driver who would have had it takes the next
        // free one, and so does every driver after it.
        let mut drivers = [Driver::default(); DRIVERS];
        drivers[PLAYER].face = 2;
        init_drivers(&mut drivers, &mut Rand::new(7), &cars(), &names());
        let faces: Vec<i32> = drivers[..PLAYER].iter().map(|d| d.face).collect();
        let expected: Vec<i32> = (0..20).filter(|&f| f != 2).collect();
        assert_eq!(faces, expected);
        assert_eq!(drivers[2].name(), b"N3");
        assert_eq!(drivers[2].colour, 3, "a driver's colour is its face");
    }

    #[test]
    fn the_drivers_are_ranked_by_their_car_and_points() {
        // The best cars go to the top of the standings; the sign-up screen's classes and
        // the standings rely on it.
        let mut drivers = [Driver::default(); DRIVERS];
        init_drivers(&mut drivers, &mut Rand::new(7), &cars(), &names());
        assert_eq!(
            (drivers[0].car, drivers[0].points, drivers[0].rank),
            (5, 100, 1)
        );
        assert_eq!(
            (drivers[18].car, drivers[18].points, drivers[18].rank),
            (0, 2, 19)
        );
        assert!(
            drivers
                .windows(2)
                .all(|pair| pair[0].points >= pair[1].points)
        );
    }

    #[test]
    fn money_and_upgrades_take_four_rand_calls_a_driver_in_order() {
        // Money first, then engine, tires, armour: another order gives every driver another
        // car and leaves the generator elsewhere for the races.
        let mut drivers = [Driver::default(); DRIVERS];
        let mut rand = Rand::new(5);
        init_drivers(&mut drivers, &mut rand, &cars(), &names());
        let mut check = Rand::new(5);
        for (index, driver) in drivers[..PLAYER].iter().enumerate() {
            let spec = cars()[START_CARS[index] as usize];
            assert_eq!(driver.money, check.next() % 100_000);
            assert_eq!(driver.engine, check.next() % spec.upgrades[0]);
            assert_eq!(driver.tires, check.next() % spec.upgrades[1]);
            assert_eq!(driver.armour, check.next() % spec.upgrades[2]);
        }
        assert_eq!(rand, check, "76 calls, no more");
    }

    fn order() -> Vec<u8> {
        vec![0, 7, 5, 3, 4, 2, 8, 1, 6, 9, 16, 14, 12, 13, 11, 17, 10, 15]
    }

    #[test]
    fn no_column_offers_its_last_circuit_again_nor_its_neighbours() {
        // The original never offers a circuit twice in a row in a column, and never the
        // same circuit in two neighbouring columns.
        let mut rand = Rand::new(3);
        let mut last = LastCircuits::default();
        let mut previous = SignUp::new(&mut rand, &order(), &mut last);
        for _ in 0..200 {
            let next = SignUp::new(&mut rand, &order(), &mut last);
            for column in 0..3 {
                assert_ne!(next.circuits[column], previous.circuits[column]);
            }
            assert_ne!(next.circuits[0], next.circuits[1]);
            assert_ne!(next.circuits[1], next.circuits[2]);
            assert!(next.circuits.iter().all(|&c| c < 18));
            previous = next;
        }
    }

    #[test]
    fn the_columns_draw_from_their_own_part_of_the_circuit_order() {
        // Easy races come from the first five circuits, hard ones from the sixth to ninth;
        // a wrong range offers hard circuits as easy ones.
        let mut rand = Rand::new(11);
        let mut last = LastCircuits::default();
        let order = order();
        let base = |c: usize| if c >= 9 { c - 9 } else { c };
        for _ in 0..200 {
            let sign_up = SignUp::new(&mut rand, &order, &mut last);
            let position = |c: usize| order.iter().position(|&o| usize::from(o) == base(c));
            assert!(position(sign_up.circuits[0]).unwrap() < 5);
            assert!((2..8).contains(&position(sign_up.circuits[1]).unwrap()));
            assert!((5..9).contains(&position(sign_up.circuits[2]).unwrap()));
        }
    }

    #[test]
    fn drivers_sign_up_only_for_races_their_car_fits() {
        // With a Vagabond the player meets cars 0–2 in the easy race, 1–3 in the medium
        // one and 3–5 in the hard one; the player is never signed up by chance.
        let mut drivers = [Driver::default(); DRIVERS];
        init_drivers(&mut drivers, &mut Rand::new(2), &cars(), &names());
        let mut rand = Rand::new(9);
        let mut sign_up = SignUp::new(&mut rand, &order(), &mut LastCircuits::default());
        while !sign_up.full() {
            sign_up.add_driver(1, &mut rand, &drivers);
        }
        let mut seen = Vec::new();
        for (race, entrants) in sign_up.entrants.iter().enumerate() {
            for &driver in entrants {
                let car = drivers[driver].car;
                let range = [0..=2, 1..=3, 3..=5][race].clone();
                assert!(
                    range.contains(&car),
                    "race {race} driver {driver} car {car}"
                );
                assert_ne!(driver, PLAYER);
                assert!(!seen.contains(&driver), "a driver signs up once");
                seen.push(driver);
            }
        }
    }

    #[test]
    fn escape_draws_once_more_even_when_every_race_is_already_full() {
        // The original fills the races with a do-while (0x435B20): one call of
        // addParticipantToRace(1) even with no place left, which draws rand() once for its
        // chance and fifty times looking for a race with room. Skipping it moves every later
        // draw.
        let mut drivers = [Driver::default(); DRIVERS];
        init_drivers(&mut drivers, &mut Rand::new(2), &cars(), &names());
        let mut rand = Rand::new(9);
        let mut sign_up = SignUp::new(&mut rand, &order(), &mut LastCircuits::default());
        while !sign_up.full() {
            sign_up.add_driver(1, &mut rand, &drivers);
        }
        let mut expected = rand.clone();
        for _ in 0..51 {
            expected.next();
        }
        sign_up.fill_at_once(&mut rand, &drivers);
        assert_eq!(rand, expected);
    }

    #[test]
    fn a_driver_keeps_the_originals_byte_layout() {
        // Saved games carry the records as the original keeps them: money at byte 48, the
        // face at 64, the rank at 72 (0x460870, 0x460880 and 0x460888 from 0x460840).
        let mut drivers = [Driver::default(); DRIVERS];
        init_drivers(&mut drivers, &mut Rand::new(4), &cars(), &names());
        let player = drivers[PLAYER];
        let bytes = player.to_bytes();
        assert_eq!(&bytes[48..52], &495i32.to_le_bytes());
        assert_eq!(&bytes[72..76], &20i32.to_le_bytes());
        assert_eq!(&bytes[56..60], &(-1i32).to_le_bytes(), "no loan: races -1");
        for driver in drivers {
            assert_eq!(Driver::from_bytes(&driver.to_bytes()), driver);
        }
    }

    #[test]
    fn a_shorter_name_keeps_the_tail_of_the_longer_one_before_it() {
        // Saved games carry the whole 12 bytes; the original copies with strcpy.
        let mut driver = Driver::default();
        driver.set_name(b"ABCDEFGH");
        driver.set_name(b"XY");
        assert_eq!(&driver.name[..8], b"XY\0DEFGH");
        assert_eq!(driver.name(), b"XY");
    }
}
