# M2c: verification against the original

The checks of the M2c spec (section 5), run against the original `dr.exe` under Wine as in [M2a](m2a.md). Screenshots and recordings stay under `captures/`.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| The Hall of Fame's screens | `menu-hall-of-fame` | **Pass.** All 78 shots equal one of our frames: the best ten wiping in over the menu every 100 ms, held, the records wiping in, Right and Left with the arrow lit, and the main menu wiping back. |
| Its sound | `menu-hall-of-fame`, recorded | **Pass.** `compare-audio` over the whole run: loudness per second within 0.34 dB (median) and 1.34 dB (largest), bands within 0.6 dB, tempo +0.080 % over 10 pieces, pitch +0 cents, balance +0.11 dB. Every second of the Hall of Fame, the music's fall, its jump to order 81 and the menu music starting again, is within 1.13 dB. |

All of it was repeated with fresh runs while the implementation plan was carried out: the 79 shots of the full scenario matched again (Space and Escape each a tick later than the line put them), and the recording compared within 0.37 dB (median) and 1.93 dB (largest), balance −0.03 dB, tempo +0.069 %.

## What the original does that the spec did not say

- **The difficulty's name is upper-cased too** before it is drawn: the medium font has no small letters.
- **Frame 0 of the wipe's masks covers its whole tile:** the band's leading tile column shows the least of the new screen, and every column ends covered.
- **Notes keep sounding across the jump to order 81**, as in a tracker; the Hall of Fame's own notes start over them.

## Key ticks

The screenshot run's keys were put on a line of 14 ms a tick from the shot before the first key. Two keys, Space on the best ten and Escape on the records, had to move two ticks later: the wipes that follow them only match when the highlights' pulse is at the original's phase.

## The manifest

`crates/headless/tests/hall-of-fame-run.sha256` holds our frames at the 78 shots' ticks, the run's sound and the last `dr.cfg` written.

## Runs

```
$ deadrally-headless find --ticks 7700 --key-at 6455:down --key-at 6491:down --key-at 6527:enter --key-at 6814:space --key-at 7027:right --key-at 7134:left --key-at 7205:left --key-at 7314:escape captures/hof/*.png
(78 lines, each "ticks ..."; exit 0)
$ deadrally-headless compare-audio captures/hof-sound/sound.wav captures/hof-sound/ours.wav --min-overlap 100
lag: 1910 ms (envelope correlation 0.900)
loudness per second, ours - original: median 0.34 dB, largest 1.34 dB
tempo, ours - original: +0.080 % (over 10 pieces of 10 s)
stereo balance (left - right), ours - original: largest +0.11 dB (over 10 pieces of 10 s)
result: PASS
```

The sound run's scenario has one more shot at 108 s than the screenshot run's, so the recording covers the menu music starting again.
