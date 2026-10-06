# M5 — The race, everything else: design

- **Date:** 2026-10-06
- **Status:** written without the owner, who asked to go on milestone after milestone; section 2 lists the decisions taken on their behalf; `docs/verification/m5.md` records the checks against the original.
- **Scope:** milestone M5 of the brief, less what M4c already did (collisions, weapons, mines, power-ups, pedestrians, damage, smoke and skid marks): the opponents' driving, a whole race to its finish, the results and the money, and the sponsors' popups after them; and what M4c left for later.
- **Builds on:** [M4](2026-10-06-m4-race-design.md).

## 1. Goal

A race runs from the start to the results as the original's does: the opponents drive tick for tick as the original's, the race ends, the results give the same places, points and money, and the game goes on from them.

## 2. Decisions taken on the owner's behalf

1. **M5 comes in parts:** M5a the opponents' driving (and with it laps and a whole race checked against the original); M5b the results, the money and the sponsors' popups; M5c what M4c left (the effect power-up's view, the race's other keys, the HUD's lap time without weapons, the statistics row and the shop's cheats). *Cost if wrong:* none; the order of work only.
2. **The facts come from the binary.** DreeRally and dRally are hints; where they disagree with `dr.exe`, the binary and the reference runs decide. *Cost if wrong:* none.
3. **The reference runs keep `--no-ai` for what they check of the player,** and DeadRally keeps the opponents still in those runs too (`Game::keep_opponents_still`, `--no-ai` in `find`, `render` and `trace`), so M4's manifests stay as they were checked. *Cost if wrong:* none for a player; a test option.

## 3. Facts about the original

- **The opponents' driving** (`calculateIAMovements` 0x40AFC0, for each opponent each logic tick, before the cars move): the keys it holds come from the track's guide (`-LR1.BPK`, a byte for each 4x4 pixels, 16 on the line) under two feelers 40 pixels out, 26 degrees either side of ahead; the zone's tables (`-DRV.DAT`: the share of its engine to drive at and of its steering to turn with; `-OHI.DAT`: how far off the line to keep while getting round a car); a car or a mine ahead (it gets round them, with the turbo), walls and knocks; the effect power-up's count; cars behind (a mine) and ahead (the guns); now and then the horn. It draws `rand()` once a tick, and once more on the tick it may sound its horn.

## 4. Verification

Each part has a scenario from the test game of M3b, its state watched in the original's memory frame by frame against ours, a shot every 500 ms, and its manifest.
