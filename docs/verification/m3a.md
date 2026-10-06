# M3a: verification against the original

The checks of the M3a spec (section 5), run against the original `dr.exe` under Wine as in [M2a](m2a.md), with `scripts/reference-run.sh --seed 1`: the run's copy of `dr.exe` seeds `rand()` with 1 where it would take `SDL_GetTicks()` (0x43A191). Screenshots and recordings stay under `captures/`.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| A new game up to the sign-up | `new-game`, seed 1 | **Pass.** All 36 shots equal one of our frames with seed 1: the menu after the intro is skipped, the licence (typing, Backspace, a face change with its lit arrow, the colour slider), the weapons question, the difficulty popup, the sign-up's wipe and welcome popup, the medium race's warning, the player signing up for the easy race and the other places filling. The circuits offered and every driver who signs up, in order and place, match: `initDrivers` and the sign-up take `rand()` as the original does. |
| `dr.cfg` with its random byte | `menu-configure`, seed 1 | **Pass.** The file written when Configure is left equals the original's byte for byte, now including the byte `saveConfiguration` takes from `rand()` (it was the one byte M2b could not match). |
| The sound of the new game | `new-game`, seed 1, recorded | **Loudness, bands, pitch and balance pass; the tempo figure is not meaningful at this length.** `compare-audio` over 43.7 s: loudness per second within 0.10 dB (median) and 0.73 dB (largest), bands within 0.7 dB, pitch +0 cents, balance −0.04 dB, so the licence's effects and voices sound where and as loud as the original's. Its tempo estimate rests on 4 pieces of 10 s and swings with the intro's skipped length, which no shot pins: +0.24 % (FAIL against the 0.15 % limit) for the run as pinned, +0.09 % to +0.24 % with the whole run moved by up to 20 ticks. The music player is M1b's, measured over 10 pieces in M2c (+0.08 %). |

## What the original does that the decompilations do not say

- **DreeRally's side panel computes the top speed** as 55 + 5 × engine + 5 × car; the original reads a table of 6 × 5 speeds at 0x44DED8 (a Sentinel starts at 80, not 65), as dRally's DOS code does.
- **The mirrored circuit is offered on an even `rand()`** in the Windows version; dRally's DOS code adds 9 on an odd one.
- **A race found on the 50th try is not used** by `addParticipantToRace` in the Windows version (the DOS version uses it).
- **`initDrivers` ends by composing the menu palette** for the player's colour (`sub_4224E0` at 0x428F8C); the licence's "copy palette 1" right after shows it, and the sign-up's wipe is drawn under that palette.
- **The welcome popup's wait reads a key before its two waits** and ends on Enter only at the start of the next pass, two waits after the key was read.
- **The other drivers' damage is repaired** every time the player signs up (`sabotageScreen`, 0x42DD10, before it checks for a sabotage).
- **The hitman's chance** starts at 5 % and rises by 2 % (up to 97 %) after every sign-up he does not come.

## Key ticks

The intro was skipped with Space 2 s after the window opened; our run presses it at tick 130, where the menu's idle shot falls on the same frame. The other keys were put on a line of 14 ms a tick from that shot; every shot matched without moving a key.

## The manifest

`crates/headless/tests/new-game-run.sha256` holds our frames at the 36 shots' ticks, the run's sound and the last `dr.cfg` written. The M2 manifests now run with seed 1 too: only their `dr.cfg` line changed (the random byte), their frames and sound did not.

## Runs

```
$ scripts/reference-run.sh --seed 1 scripts/reference/new-game.scenario captures/new-game
done: 36 shots in captures/new-game
$ deadrally-headless find --ticks 3400 --seed 1 --key-at 130:space --key-at 1727:enter --key-at 1799:enter --key-at 1885:a --key-at 1906:b --key-at 1927:c --key-at 1985:backspace --key-at 2042:c --key-at 2077:down --key-at 2160:right --key-at 2175:right --key-at 2189:right --key-at 2239:enter --key-at 2325:enter --key-at 2410:down --key-at 2482:enter --key-at 2660:enter --key-at 2746:right --key-at 2817:enter --key-at 2903:space --key-at 2975:left --key-at 3046:enter captures/new-game/*.png
(36 lines, each "ticks ..."; exit 0)
$ scripts/reference-run.sh --seed 1 scripts/reference/menu-configure.scenario captures/configure-seeded
$ cmp ours.cfg captures/configure-seeded/dr.cfg   # ours from the configure manifest run, seed 1
(no output: equal)
$ deadrally-headless compare-audio captures/new-game-sound/sound.wav captures/new-game-sound/ours.wav --min-overlap 40
lag: 2390 ms (envelope correlation 0.860)
loudness per second, ours - original: median 0.10 dB, largest 0.73 dB
tempo, ours - original: +0.240 % (over 4 pieces of 10 s)
result: FAIL (tempo off by more than 0.15 %)
```

## Review

A fresh review of the branch found two Important problems, both fixed with a test that failed first: ending a game left the Start Racing menu's highlight on its now inactive second row, so Enter would set the drivers up again (76 draws the original never makes); and a difficulty past 2 in a damaged `dr.cfg` crashed the difficulty popup (now taken as 2). Fixed too: Escape at the sign-up fills the races with a do-while as the original does (51 draws even when every race is already full), the circuit order is refused when mirroring one of its first nine would pass the last circuit, the text cursor is the one string allowed outside printable ASCII, and comments that quoted game text or misstated a count. Rulings kept: when the hitman would come, the screen still lingers its 280 waits (timing inside the stand-in only); the first `dr.cfg` written at start-up keeps its file's random byte (the original's comes from a `rand()` state seeded before the main menu); the statistics and save rows are active but do nothing until M3b.
