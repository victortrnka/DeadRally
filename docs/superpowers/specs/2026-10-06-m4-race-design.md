# M4 — The race, drawn and driven: design

- **Date:** 2026-10-06
- **Status:** written without the owner, who asked to go on milestone after milestone without stopping; section 2 lists the decisions taken on their behalf; `docs/verification/m4*.md` records the checks against the original.
- **Scope:** milestone M4 of the brief: from the sign-up to the race and back: the race's preview, the race itself with its track, cars, HUD, camera, start and end, and the player's car driven as the original drives it.
- **Builds on:** [M3a](2026-10-06-m3a-new-game-design.md), [M3b](2026-10-06-m3b-saves-shop-design.md), [M3c](2026-10-06-m3c-market-design.md).

## 1. Goal

After a sign-up the player sees the race's preview, then races: the track scrolls under the player's car as in the original, the HUD shows the same numbers, and for the same keys held the car ends where the original's does, tick for tick and pixel for pixel.

## 2. Decisions taken on the owner's behalf

1. **M4 comes in parts, each merged on its own:** M4a the race's preview ("prepare to race"); M4b the race's scene and start (the track, the cars on the grid, the HUD, the camera, the lights, the pause menu); M4c the player driving (the car's physics, the walls, laps, the race's end). *Cost if wrong:* none; the order of work only.
2. **The opponents stand still until M5 brings their driving.** The reference runner patches the original's opponents out of its runs (`--no-ai`), so its screenshots show them where ours stay. *Cost if wrong:* none for a player until M5; the runs check only what M4 builds.
3. **The results come with M5.** Until then the race's end leads back to the shop as the stand-in race does now, nothing won or lost. *Cost if wrong:* none until M5.

## 3. Facts about the original

- **The preview** (`previewRaceScreen` 0x4321B0, after the sign-up's linger, its sabotage popup or its offer): drawn into the second buffer over the background: the banner `PREP4` at (0, 426), the grid's frame `PREPW1` at (13, 100), the circuit's `TSHAPE` picture at (264, 100), the laps (4, 5 or 6 by race) and the prize in the medium font, and for the four drivers in their places their rank (right-aligned), upper-cased name and face. It wipes in (`sub_42C670`), then the opponents' weapons are drawn and the race starts (`startRace` 0x415710).

## 4. Verification

Each part has a scenario from the test game of M3b through the sign-up, a shot after every key, and its manifest; M4c's scenarios hold keys (`keydown`, `keyup`) and compare every frame of a lap.
