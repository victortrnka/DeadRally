# M3a — A new game, up to signing up for the first race: design

- **Date:** 2026-10-06
- **Status:** written without the owner, who asked to go on milestone after milestone without stopping while everything works, and to get the game finished; section 2 lists the decisions taken on their behalf; [docs/verification/m3a.md](../../verification/m3a.md) records the checks against the original.
- **Scope:** the first part of milestone M3 (brief §7, "Campaign without racing"): the original's random numbers, "Start A New Game" in the Start Racing menu, the driver's licence, the twenty drivers, and the sign-up screen with its three races. M3b brings saved games and the shop, M3c the shop's purchases, the Underground Market and the loan shark.
- **Builds on:** [M2a](2026-10-05-m2a-main-menu-design.md), [M2b](2026-10-05-m2b-configure-design.md), [M2c](2026-10-05-m2c-hall-of-fame-design.md).

## 1. Goal

From the main menu, "Start Racing" → "Start A New Game" runs the original's licence screen (nickname, face, car colour, weapons, difficulty), sets up the twenty drivers as the original does, and shows the sign-up screen with the same three circuits and the same drivers signing up as the original for the same random seed and the same keys, pixel for pixel.

## 2. Decisions taken on the owner's behalf

1. **M3 is split in three** (M3a new game and sign-up; M3b saved games and the shop; M3c purchases, the Underground Market and the loan shark), as M2 was. The original only reaches the shop after a race or a loaded game, so saved games come before the shop. *Cost if wrong:* none; the order of work only.
2. **Until races exist, signing up ends the race at once:** where the original shows the race preview and runs the race (`previewRaceScreen`, M4), DeadRally goes back to the Start Racing menu with nothing changed. *Cost if wrong:* none; M4 replaces the stand-in.
3. **The seed comes from the host:** the original seeds `rand()` with `SDL_GetTicks()` as the main menu starts; DeadRally's core takes the seed from its frontend (the milliseconds since start, as the original) and the headless tool takes `--seed`. *Cost if wrong:* none for players; runs repeat only with the same seed and keys.
4. **Sponsors move to M5:** their popups follow a race's results (`ui/util/popup.c`), so they come with results and money. *Cost if wrong:* none; the roadmap line moves.
5. **Until the shop exists, "Enter The Shop" opens the sign-up**, the shop's way on; after the stand-in race the welcome counts as shown (the shop's welcome would clear it). *Cost if wrong:* none; M3b replaces it.
6. **The hitman's offer waits for M3c:** after a sign-up the original draws `rand() % 100` against the hitman's chance; when he would come, DeadRally takes that draw, resets his chance as the original does and goes on without his screen (its further draws, and his price, come with the market). *Cost if wrong:* from the first visit on, the random numbers differ from the original's until M3c.
7. **Signing up for no race** shows the original's popup and fade, then returns to the Start Racing menu where the original shows the race's results (M5). *Cost if wrong:* none.

## 3. Facts about the original

DreeRally (read-only): `ui/menu.c` `startRacingMenu` (0x439CD0), `ui/licenseScreen.c` `licenseScreen` (0x434800), `ui/util/input.c` `readKeyboard` (0x42E7F0), `drivers.c` `initDrivers` (0x428930), `ui/selectRaceScreen.c` (`calculateNextRaces` 0x4240B0, `selectRaceScreen` 0x4357F0, `addParticipantToRace` 0x423A20).

- **Random numbers:** the MSVC runtime's `rand()`: state × 214013 + 2531011, bits 16–30. `mainMenu` (0x43A020) calls `srand(SDL_GetTicks())` at 0x43A191, after it wrote `dr.cfg` once; every later `saveConfiguration` (0x4264E0) takes one `rand()` for its random byte.
- **The drivers:** 20 records of 108 bytes (name 12, damage, engine, tires, armour, car, 3 unused, colour, money, loan, loan races, car's price, face, points, rank, wins, races, last income, total income, mines, spikes, rocket, sabotage). `initDrivers` makes the player driver 19; drivers 0–18 get cars 5,5,5,4,4,4,4,3,3,3,2,2,2,2,1,1,1,0,0, points `trunc((100 − trunc(77 log10(i+1)) + 5.5 (18 − i) + 2) / 2)` (driver 19 too), and in order: money `rand() % 100000`, engine, tires and armour `rand() %` their car's upgrade counts, the car's price, rank i + 1, and the first face from i on that no driver took and is not the player's, which is also its colour and picks its name. The player gets $495, car 0, rank 20, no loan (−1, −1).
- **The licence:** nickname (up to 10 letters, 300 pixels), Up/Down change the face (20), Left/Right the colour slider (0–252, step 2), Enter ends; weapons yes/no; difficulty popup with three rows; Escape at the nickname leaves without a game.
- **Sign-up:** three circuits drawn from the circuit order with `rand()` (never the last one offered in that column, never the one beside it), prices $750, $3000, $12000; while the player chooses, each menu pass (two waits) a driver may sign up (`rand() % 75 == 0`) for a random race whose car class fits; Left/Right move the border; Enter signs the player up (unless the race is full or a warning applies), then the other places fill a driver at a time (`rand() % 3 == 0` each pass) and the four of each race are sorted by driver index.

## 4. Architecture

- `deadrally-core::campaign`: the random numbers (`Rand`), the drivers (`Driver`, `Drivers`) and `initDrivers`, `calculateNextRaces`, `addParticipantToRace` as plain functions over them: no assets, unit-tested.
- `deadrally-core::menu`: new states for the licence and the sign-up screen in `menu/licence.rs` and `menu/sign_up.rs`, drawn from new `MenuAssets` pictures; texts read from `dr.exe` as in M2 (`gamedata::text`).
- The frontend passes a seed; `deadrally-headless` gets `--seed` for `run`, `render` and `find`.

## 5. Verification

`scripts/reference-run.sh --seed N` patches the run's copy of `dr.exe` (the call at 0x43A191 becomes `mov eax, N`). A scenario with seed 1 goes through a new game (nickname, a face, a colour, weapons, a difficulty) to the sign-up screen and signs up; every shot must equal one of our frames (`find` with `--seed 1` and the same keys). The menu's sound with the licence's effects is compared as in M2.

## 6. Tests

Unit tests: the `rand()` sequence for a seed; `initDrivers`' points table and the player's start; the circuits never repeat a column's last one and keep to their part of the circuit order; drivers sign up only for races their car fits; the nickname's key table; the licence's shading. A data test runs the seed-1 keys and checks our frames' hashes at the shots' ticks (manifest `new-game-run.sha256`); the M2 manifests run with seed 1, so their `dr.cfg` carries the original's random byte.
