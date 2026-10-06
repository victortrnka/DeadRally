# M4b: verification against the original

The checks of the M4 spec (section 4) for its second part, the race's scene and start, run against the original `dr.exe` under Wine as in [M4a](m4a.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE` and M3b's test game. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The race's start | `race-start`, seed 1, the test game | **Pass.** All 55 shots, one every 100 ms, equal one of our frames 7 ticks apart: the preview held, its fade to black, the black while the race loads, the race's intro (the view tilting up from edge-on as it fades in, the HUD sliding in, the player's car flashing, the colours coming back) and the countdown: the track, the 3D scene, the cars with their headlights, the shadows, a pedestrian stepping, the HUD and the first light sliding in. |
| The race's sound | `race-sound` with `--no-ai`, seed 1, the test game: the start, then two minutes of the race standing | **Pass.** `compare-audio` over the whole run (the menus, the preview, the race): lag 3.01 s, overlap 167.5 s, loudness median +0.45 dB (largest 2.35 dB), octave bands within +1.3 dB, tempo +0.054 %, pitch 0 cents, balance 0.17 dB. The race's two minutes alone: loudness median +0.49 dB (largest 0.96 dB), bands within +1.4 dB, pitch 0 cents; its tempo cannot be measured (the track's music has too even an envelope to align over 10 s pieces). The track's music starts silent and rises with the intro's zoom; the engine idles on channel 1; the lights' sounds play on channels 2 and 5. |

## What the original does that the decompilations do not say

- **The race's intro** (`sub_404C30`, run on the race loop's first frame) tilts the view from 1 to 90 degrees, the step growing by 1.02 a tick from 0.9, the colours at the tilt's 90th; then 20 waits, the player's car flashing to 1.7 times its colours and back (6 and 26 steps), 50 waits, and the other colours coming back from grey in 32 steps of 8/256. The last steps stop at 248/256 (250/256 for the car), so the race's palette ends a little off the track's: (32, 49, 8) shows as (33, 50, 8).
- **Shadows darken only the cars:** `VARJO.TAB` maps entries 0 to 63, the cars' colours, two steps down a ramp and leaves the track's colours alone; the track's picture has its shadows drawn in.
- **The 3D scene's tables for kinds 0x81 to 0x83** are never filled; those triangles come out black. A second picture made for a triangle lands on the triangle before (`calculateSceTextureStructure` writes the slot before).
- **The race's sound** (0x416215): the menu's music stops, the track's `TRn-MUS.CMF` starts at the configured volume under a volume mask of 0, which the intro raises to the zoom's factor times 728 each frame; `GEN-EFE.CMF` holds the race's sounds: the engine (25 plus the car) at pitch 0x28000, the first light's sound (3, pitch 0x50000), the second's and the start's (44, pitches 0x20000 and 0x28000). The engine's pitch is set again every frame from the car's speed, unchanged while it stands.
- **The pedestrians step by the timer**, every 5 ticks, not by the race's frames: they move during the countdown.
