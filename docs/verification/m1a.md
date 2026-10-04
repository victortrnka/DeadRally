# M1a: verification against the original

The checks of the M1a spec (section 9), run against the original `dr.exe` under Wine. Screenshots stay under `captures/` and are never committed; this file keeps only what they showed.

## Setup

- **Original:** `dr.exe` of the Steam release, SHA-256 `54fe789faca583d67b8e73e7c58908f3f1468c5c8f75942239a60483ae9be58c`, with the known data release `check-data` reports.
- **Runner:** `scripts/reference-run.sh` (Wine 9.0 in a 32-bit prefix, Xvfb, xdotool, `xwd`), original started as `dr.exe -window -nogl -nosound`.
- **Comparison:** `deadrally-headless find`, which renders every tick of our startup sequence the way the original's `-nogl` window shows it and reports the ticks that equal a screenshot exactly (every pixel, every channel).

## How the original's window shows a screen

- The window is always 640x480 (`SDL_SetVideoMode(640, 480, 32, ...)`).
- With `-nogl`, `refreshScreen` (0x43B580) copies 640x480 screens as they are and doubles 320x200 screens to 640x400 from row 40, black above and below. Measured: the intro screenshots match only under this mapping.
- Colours are the 6-bit palette shifted left by two (63 shows as 252).
- Without `-nogl` (the default), the OpenGL path stretches 320x200 over the whole 4:3 window, as DeadRally's frontend does. Screenshots use `-nogl` because the OpenGL path filters.

## Results

| Check (spec section 9) | Scenario | Result |
|---|---|---|
| Intro frames: HAF decoder, palettes, letterbox | `startup` | **Pass.** All 8 shots spread over the intro equal one of our intro frames exactly. |
| Apogee, Remedy, title: BPK, BMP, palettes | `startup`, `fades` | **Pass.** Every hold and title shot is pixel-identical. |
| Fade formula | `fades` | **Pass.** Every shot taken during a fade equals one of our fade levels exactly; no `- 4` on red (in DreeRally it is an address offset, `[esi-4]`, misread as a value). |
| Key during a fade-in | `keys` | **Pass.** A key during the Apogee fade-in ends the Apogee hold after one tick; a key during the Apogee fade-out ends the Remedy hold after one tick. |
| Timing | `startup` | **Pass.** Over the intro, shots 10 s apart are 712 to 716 ticks apart (10 000 / 14 = 714.3); the intro lasts 80.2 s (5732 x 14 ms = 80.25 s); fade-in plus hold of a logo 2.86 s (205 ticks = 2.87 s). |

## What the original does that the spec did not say

These were found by the checks above and are now part of the spec (section 3.6) and the code (`crates/core/src/startup.rs`):

- **Fade levels:** a fade-in shows 0, 4, ..., 96 % over 25 ticks and stops one step short of 100 %; a fade-out shows 100, 96, ..., 0 % over 26 ticks, so it starts with a one-tick flash from 96 to 100 %.
- **The title stays at 92 %:** the original sets the title's last fade step and then loads the main menu without showing another frame. About 0.1 s later it fades the title out and shows the main menu; that belongs to M2.
- **The intro checks for a key once per frame,** right after drawing it and before showing it: a key ends the intro at the next frame, which is never shown. The last frame is never shown either: the palette goes black right after it is drawn.
- **One remembered key:** a press waits until the next check (`eventDetected`, 0x417EB0, reads and clears it), even across screens.

## Scenarios

- `scripts/reference/startup.scenario`: no keys; 8 shots over the intro, then one every 500 ms through the logos and the title.
- `scripts/reference/fades.scenario`: a key skips the intro; bursts of shots every 25 ms through the fades; keys end the holds.
- `scripts/reference/keys.scenario`: keys during the Apogee fade-in and fade-out.

Each file says how to check its shots.

## Runs

`find` output of the run that this record is based on; tick numbers move by a few ticks from run to run because Wine's timing jitters.

### startup

```
$ deadrally-headless find captures/startup/*.png
after-79000.png: ticks 5672-5675
after-79500.png: ticks 5711-5717
after-80000.png: ticks 5745, 5951
after-80500.png: ticks 5757-5937, 5939
after-81000.png: ticks 5757-5937, 5939
after-81500.png: ticks 5757-5937, 5939
after-82000.png: ticks 5757-5937, 5939
after-82500.png: ticks 5757-5937, 5939
after-83000.png: ticks 5736, 5960
after-83500.png: ticks 5988-6168, 6170
after-84000.png: ticks 5988-6168, 6170
after-84500.png: ticks 5988-6168, 6170
after-85000.png: ticks 5988-6168, 6170
after-85500.png: ticks 5988-6168, 6170
after-86000.png: ticks 5984, 6174
after-86500.png: ticks 6210
after-87000.png: ticks 6202
intro-05s.png: ticks 388-391
intro-15s.png: ticks 1100-1103
intro-25s.png: ticks 1816-1819
intro-35s.png: ticks 2530-2533
intro-45s.png: ticks 3246-3249
intro-55s.png: ticks 3960-3963
intro-65s.png: ticks 4672-4675
intro-75s.png: ticks 5388-5391
```

### fades

```
$ deadrally-headless find --key-at 0 --ticks 600 captures/fades/*.png
apogee-hold.png: ticks 29-209, 211
apogee-in-3125.png: ticks 10, 230
apogee-in-3150.png: ticks 12, 228
apogee-in-3175.png: ticks 13, 227
apogee-in-3200.png: ticks 15, 225
apogee-in-3225.png: ticks 17, 223
apogee-in-3250.png: ticks 19, 221
apogee-in-3275.png: ticks 20, 220
apogee-in-3300.png: ticks 22, 218
apogee-in-3325.png: ticks 24, 216
apogee-in-3350.png: ticks 26, 214
apogee-in-3375.png: ticks 28, 212
apogee-in-3400.png: ticks 29-209, 211
apogee-in-3425.png: ticks 29-209, 211
apogee-in-3450.png: ticks 29-209, 211
apogee-in-3475.png: ticks 29-209, 211
apogee-in-3500.png: ticks 29-209, 211
apogee-in-3525.png: ticks 29-209, 211
apogee-in-3550.png: ticks 29-209, 211
apogee-in-3575.png: ticks 29-209, 211
apogee-in-3600.png: ticks 29-209, 211
apogee-in-3625.png: ticks 29-209, 211
apogee-in-3650.png: ticks 29-209, 211
apogee-in-3675.png: ticks 29-209, 211
apogee-in-3700.png: ticks 29-209, 211
apogee-out-5000.png: ticks 29-209, 211
apogee-out-5025.png: ticks 29-209, 211
apogee-out-5050.png: ticks 210
apogee-out-5075.png: ticks 28, 212
apogee-out-5100.png: ticks 26, 214
apogee-out-5125.png: ticks 24, 216
apogee-out-5150.png: ticks 22, 218
apogee-out-5175.png: ticks 21, 219
apogee-out-5200.png: ticks 19, 221
apogee-out-5225.png: ticks 17, 223
apogee-out-5250.png: ticks 15, 225
apogee-out-5275.png: ticks 13, 227
apogee-out-5300.png: ticks 12, 228
apogee-out-5325.png: ticks 10, 230
apogee-out-5350.png: ticks 8, 232
apogee-out-5375.png: ticks 6, 234
apogee-out-5400.png: ticks 4-5, 235-236, 466-467
apogee-out-5425.png: ticks 237, 465
apogee-out-5450.png: ticks 239, 463
apogee-out-5475.png: ticks 241, 461
apogee-out-5500.png: ticks 242, 460
apogee-out-5525.png: ticks 244, 458
apogee-out-5550.png: ticks 246, 456
apogee-out-5575.png: ticks 248, 454
apogee-out-5600.png: ticks 249, 453
apogee-out-5625.png: ticks 251, 451
apogee-out-5650.png: ticks 253, 449
apogee-out-5675.png: ticks 255, 447
apogee-out-5700.png: ticks 257, 445
apogee-out-5725.png: ticks 259, 443
apogee-out-5750.png: ticks 260-440, 442
apogee-out-5775.png: ticks 260-440, 442
apogee-out-5800.png: ticks 260-440, 442
apogee-out-5825.png: ticks 260-440, 442
apogee-out-5850.png: ticks 260-440, 442
remedy-hold.png: ticks 260-440, 442
to-title-7500.png: ticks 260-440, 442
to-title-7525.png: ticks 441
to-title-7550.png: ticks 260-440, 442
to-title-7575.png: ticks 258, 444
to-title-7600.png: ticks 256, 446
to-title-7625.png: ticks 255, 447
to-title-7650.png: ticks 253, 449
to-title-7675.png: ticks 251, 451
to-title-7700.png: ticks 249, 453
to-title-7725.png: ticks 248, 454
to-title-7750.png: ticks 246, 456
to-title-7775.png: ticks 244, 458
to-title-7800.png: ticks 242, 460
to-title-7825.png: ticks 240, 462
to-title-7850.png: ticks 239, 463
to-title-7875.png: ticks 237, 465
to-title-7900.png: ticks 4-5, 235-236, 466-467
to-title-7925.png: ticks 469
to-title-7950.png: ticks 470
to-title-7975.png: ticks 472
to-title-8000.png: ticks 474
to-title-8025.png: ticks 476
to-title-8050.png: ticks 477
to-title-8075.png: ticks 480
to-title-8100.png: ticks 481
to-title-8125.png: ticks 483
to-title-8150.png: ticks 485
to-title-8175.png: ticks 486
to-title-8200.png: ticks 488
to-title-8225.png: ticks 490-600
to-title-8250.png: ticks 490-600
to-title-8275.png: ticks 490-600
to-title-8300.png: ticks 490-600
to-title-8325.png: ticks 490-600
to-title-8350.png: ticks 490-600
```

### keys

```
$ deadrally-headless find --key-at 0 --key-at 15 --key-at 45 --ticks 300 captures/keys/*.png
keys-3125.png: ticks 8, 53
keys-3150.png: ticks 10, 51
keys-3175.png: ticks 11, 50
keys-3200.png: ticks 15, 46
keys-3225.png: ticks 16, 45
keys-3250.png: ticks 17, 44
keys-3275.png: ticks 18, 43
keys-3300.png: ticks 20, 41
keys-3325.png: ticks 22, 39
keys-3350.png: ticks 24, 37
keys-3375.png: ticks 26, 35
keys-3400.png: ticks 28, 33
keys-3425.png: ticks 29-30, 32
keys-3450.png: ticks 31
keys-3475.png: ticks 28, 33
keys-3500.png: ticks 26, 35
keys-3525.png: ticks 25, 36
keys-3550.png: ticks 23, 38
keys-3575.png: ticks 21, 40
keys-3600.png: ticks 18, 43
keys-3625.png: ticks 17, 44
keys-3650.png: ticks 16, 45
keys-3675.png: ticks 14, 47
keys-3700.png: ticks 12, 49
keys-3725.png: ticks 10, 51
keys-3750.png: ticks 9, 52
keys-3775.png: ticks 7, 54
keys-3800.png: ticks 4-5, 56-57, 108-109
keys-3825.png: ticks 58, 107
keys-3850.png: ticks 59, 106
keys-3875.png: ticks 62, 103
keys-3900.png: ticks 63, 102
keys-3925.png: ticks 65, 100
keys-3950.png: ticks 67, 98
keys-3975.png: ticks 69, 96
keys-4000.png: ticks 70, 95
keys-4025.png: ticks 72, 93
keys-4050.png: ticks 74, 91
keys-4075.png: ticks 76, 89
keys-4100.png: ticks 77, 88
keys-4125.png: ticks 79, 86
keys-4150.png: ticks 81-82, 84
keys-4175.png: ticks 83
keys-4200.png: ticks 81-82, 84
keys-4225.png: ticks 79, 86
keys-4250.png: ticks 77, 88
keys-4275.png: ticks 75, 90
keys-4300.png: ticks 73, 92
keys-4325.png: ticks 72, 93
keys-4350.png: ticks 70, 95
keys-4375.png: ticks 68, 97
keys-4400.png: ticks 66, 99
keys-4425.png: ticks 64, 101
keys-4450.png: ticks 63, 102
keys-4475.png: ticks 61, 104
keys-4500.png: ticks 59, 106
keys-4525.png: ticks 58, 107
keys-4550.png: ticks 110
keys-4575.png: ticks 112
keys-4600.png: ticks 113
keys-4625.png: ticks 115
keys-4650.png: ticks 117
keys-4675.png: ticks 118
keys-4700.png: ticks 120
keys-4725.png: ticks 122
keys-4750.png: ticks 124
keys-4775.png: ticks 126
keys-4800.png: ticks 127
keys-4825.png: ticks 129
keys-4850.png: ticks 131
keys-4875.png: ticks 132-300
keys-4900.png: ticks 132-300
keys-4925.png: ticks 132-300
keys-4950.png: ticks 132-300
keys-4975.png: ticks 132-300
keys-5000.png: ticks 132-300
```
