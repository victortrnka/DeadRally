# M3b: verification against the original

The checks of the M3b spec (section 5), run against the original `dr.exe` under Wine as in [M3a](m3a.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE`: the run's copy of the game starts with FILE as its saved game `DR.SG0`. FILE is a game made for the test (seed 1's drivers, the player at $23456 in a Dervish with one engine and one tire upgrade and 37 % damage); the data test builds the same file from the game's data. Screenshots, recordings and saved games stay under `captures/`.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| Loading, the shop, saving | `saved-games`, seed 1, `--save 0:` the test game | **Pass.** All 31 shots equal one of our frames: the slots with the saved game's name, the confirmation, the shop wiping in, every item's box and description (Left through all five items, Up to the car, Right and Left on the car box with the lit arrows), Down and Right, Escape to the menus, the save slots, the name prompt, the typed name and its confirmation. |
| The saved file | the same run | **Pass.** Our slot 1 file equals the original's `DR.SG1` byte for byte; its key is the seed's first `rand()` modulo 255, as in the original. |

## What the original does that the decompilations do not say

- **The repair box turns through 24 frames** (the counter wraps after 23, 0x43963A); DreeRally's shop loop wraps it after 22.
- **The armour box's "no more upgrades" check** compares the armour level with the armour's upgrade count (0x420AB2); DreeRally compares it with the engine's.
- **The shop's descriptions live in `dr.exe`'s tables:** the car's and its engine levels' in the car's own record, the others in tables of 240 bytes a level (`reloadCarAnimation2` 0x420250 and the functions after it).
- **`mainMenu` runs `initDrivers` at start-up**, so a game loaded first thing shows car 1 in the shop's car box.
- **Left and Right on the car box** show the next car with its arrow lit, then the car turns four frames over eight waits without being shown; only the arrow going out shows the screen again.

## Key ticks

The keys were put on a line of 14 ms a tick from the menu's idle shot, as in M3a; the confirmation's key and eight of the shop's keys then had to move one or two ticks (none further) for their shots to match, the shop's loops showing the pass each key was read on.

## The manifest

`crates/headless/tests/saved-games-run.sha256` holds our frames at the 31 shots' ticks, the run's sound, the game saved into slot 1 and the last `dr.cfg` written.
