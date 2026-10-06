# M4a: verification against the original

The checks of the M4 spec (section 4) for its first part, the race's preview, run against the original `dr.exe` under Wine as in [M3c](m3c.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE` and M3b's test game. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The race's preview | `preview`, seed 1, the test game | **Pass.** All 24 shots equal one of our frames: through the Underground Market to the sign-up, the easy race chosen, the places filling, a key ending the screen's linger, ten shots of the preview wiping in over the sign-up and three of it shown: the grid's four drivers with their ranks, names and faces, the circuit's picture, its laps and prize. |

## What the original does that the decompilations do not say

- **The preview's wipe** (`sub_42C670`) is the menus' wipe with 27 tile rows from row 73 instead of 22 from row 75, the music falling over its 43 steps, and one more wait after it; then the music starts at order 0x28 at full volume.
- **The preview stays while the race loads.** The original shows it about three seconds, then the screen goes black and the race fades in by itself; no key is read. Until the race exists (M4b) DeadRally goes from the shown preview to the stand-in race at once, so frames after it differ from the original's.
- **The opponents' weapons are drawn after the preview:** a rocket, spikes and 8 mines at one chance in five each, in that order, three `rand()` calls for every opponent whether weapons are on or not (0x433102; dRally `___33010h.c` the same).

## Key ticks

As in M3c: the keys on the 14 ms line, the space before the shop four ticks early and the later keys two (the pulse's turn, M3c); the key ending the linger one tick late.

## The manifest

`crates/headless/tests/preview-run.sha256` holds our frames at the 24 shots' ticks, the run's sound and the last `dr.cfg` written.
