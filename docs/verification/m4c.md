# M4c: verification against the original

The checks of the M4 spec (section 4) for its third part, the player driving, run against the original `dr.exe` under Wine as in [M4b](m4b.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE --no-ai` and M3b's test game. The scenarios hold keys (`keydown`, `keyup`); our runs hold them the ticks fitted to each run. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The player driving | `drive` with `--no-ai`, seed 1, the test game: Up held from before the start, Left and Right held 200 ms each on the way, Up let go and the car rolling out | **Pass.** All 61 shots, one every 100 ms, equal our frames: the countdown, the car pulling away (its wheels spinning, the smoke puffs and tire marks behind it), the view running ahead of it, the speed gauge, the turns, the grass slowing it, the power-ups' draws of `rand()` from the 350th tick on, rolling out. Keys fitted: Up at tick 3359 for 324 ticks, Left at 3540 and Right at 3583 for 14. |
| The second pause | `pause` with `--no-ai` (M4b's run) | **Pass.** With the power-ups' draws of `rand()` in, the second pause's tiles start where the original's do: all 5 shots tried equal our frames (Escape fitted at tick 3895). |

## What the original does that the decompilations do not say

- **The cars' handling comes from six tables** that `initParticipantValues` (0x401060) fills on its stack: engine, tires, steering, armour and armour upgrades by car and level, and each car's slew; the opponents are set up for the race's level, the player for a level of their own (3). DeadRally reads them by running that code over the player's `dr.exe`.
- **The movement (0x40BAB0) in the x87's 53 bits:** the speed eases by 2 % a tick and the accelerator, the brake, the turbo and the rocket push it as the original's constants say; the steering turns the car by its steering capacity a tick and slews it sideways; seven points round the car read the ground under it (walls below 4, slowing grounds, spinning ones, a boost, a random jolt); a wall under the car's sprite turns it 20 degrees away, then pushes it back, then shakes it loose with `rand()`. Degrees become radians through the original's slightly-off constant (0x4412B0).
- **The wheels (`sub_411D10`, every tick, every car):** at low speed with the accelerator held the wheels spin and shake the car by up to half a degree, two draws of `rand()` a tick; a skidding car paints marks through the track's `-SKI.TAB` into the track's picture (red ones through `-BLO.TAB` for a while after a pedestrian), throws a smoke puff off each rear wheel every 6 ticks (`SMOKE.BPK`), and the player's tires squeal on channel 6 once the start is given.
- **Every logic tick eases each car's push back and spin** (by 0.8696 and 0.8333, at the start of `sub_40CD10`).
- **The view runs ahead of the moving car** (`recalculateCircuitImageOffset` 0x40D560): 16 and 10.67 times its speed along its direction, a fifth of the way there each frame the target changes.
- **The keys are sampled by the timer** (`sub_4138A0`) into a ring of 16, and each pass of the race loop hands the player's car the samples since the last, oldest first; the arrows drive whatever `dr.cfg` sets, the turbo key also accelerates, and the first mine control counts once a press and takes the brake away.
- **The power-ups (`sub_410220`, once a pass):** after 350 ticks every pass tries the first 12 places in a random order, drawing `rand()` until each place has come up, and lays a power-up (by `rand() % 100`) where the place's own wait (100 to 149 ticks, then 300 to 499) is over, at most four at once; they blink out from their 1530th tick and go at the 2000th.
- **The first frame counts a tick.** Loading the race leaves the timer a tick behind, which the first frame catches up before its HUD, so the first `between` is 1: the power-ups' wait ends a pass earlier than 350 ticks after the loop's start, and the run's wheelspin only matches with that.
- **Only the player's car moves off the grid's sprite:** every car is stepped every tick, but the cars standing keep their direction (72 times 3.75 times the original's 0.2666... is 72 again in 53 bits).
