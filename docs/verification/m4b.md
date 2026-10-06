# M4b: verification against the original

The checks of the M4 spec (section 4) for its second part, the race's scene and start, run against the original `dr.exe` under Wine as in [M4a](m4a.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE` and M3b's test game. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The race's start | `race-start`, seed 1, the test game | **Pass (pictures).** All 55 shots, one every 100 ms, equal one of our frames 7 ticks apart: the preview held, its fade to black, the black while the race loads, the race's intro (the view tilting up from edge-on as it fades in, the HUD sliding in, the player's car flashing, the colours coming back) and the countdown: the track, the 3D scene, the cars with their headlights, the shadows, a pedestrian stepping, the HUD and the first light sliding in. The run's sound is not yet checked: the race's music and sounds come later in M4b. |

## What the original does that the decompilations do not say

- **The race's intro** (`sub_404C30`, run on the race loop's first frame) tilts the view from 1 to 90 degrees, the step growing by 1.02 a tick from 0.9, the colours at the tilt's 90th; then 20 waits, the player's car flashing to 1.7 times its colours and back (6 and 26 steps), 50 waits, and the other colours coming back from grey in 32 steps of 8/256. The last steps stop at 248/256 (250/256 for the car), so the race's palette ends a little off the track's: (32, 49, 8) shows as (33, 50, 8).
- **Shadows darken only the cars:** `VARJO.TAB` maps entries 0 to 63, the cars' colours, two steps down a ramp and leaves the track's colours alone; the track's picture has its shadows drawn in.
- **The 3D scene's tables for kinds 0x81 to 0x83** are never filled; those triangles come out black. A second picture made for a triangle lands on the triangle before (`calculateSceTextureStructure` writes the slot before).
- **The pedestrians step by the timer**, every 5 ticks, not by the race's frames: they move during the countdown.
