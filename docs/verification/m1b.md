# M1b: verification against the original

The checks of the M1b spec (section 5), run against the original `dr.exe` under Wine. Recordings stay under `captures/` and are never committed; this file keeps only what they showed.

## Setup

- **Original:** `dr.exe` of the Steam release, SHA-256 `54fe789faca583d67b8e73e7c58908f3f1468c5c8f75942239a60483ae9be58c`, with the known data release `check-data` reports.
- **Runner:** `scripts/reference-run.sh --sound` (Wine 9.0 in a 32-bit prefix, Xvfb, xdotool), original started as `dr.exe -window -nogl` from a fresh copy, so it writes a fresh `dr.cfg` (music 50 %, effects 75 %) unless `--cfg` gives one. The game plays into a PulseAudio null sink; `pactl` confirms its stream is there before anything is recorded, and `parec` records the sink at 44.1 kHz. Nothing reaches the speakers.
- **Ours:** `deadrally-headless render-audio --startup`, which runs the same `Game::tick` and `Game::take_audio` as the game.
- **Comparison:** `deadrally-headless compare-audio`. It aligns the two files by their 10 ms loudness envelopes and compares loudness per second, octave bands, tempo, pitch and stereo balance, never waveforms: Wine resamples, and FMOD's mixer is not ours.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| The startup: intro, its end, the menu music | `startup-sound` | **Pass.** Loudness per second within 0.29 dB (median) and 2.55 dB (largest); bands within 0.9 dB; tempo +0.018 % over 12 pieces; pitch +0 cents; balance +0.20 dB. Around the intro's end (78 to 89 s) every second is within 0.7 dB. |
| The music volume law | `menu-sound`, default `dr.cfg` and music at 100 % | **Pass.** The menu music at 100 % is 6.17 dB louder (median), against 6.1 dB for FMOD master volume 127 against 63; the seconds of intro before the key are equally loud in both (within 0.3 dB). |
| Effect pitch and loudness | `startup-sound` | **Pass,** through the intro's bands and loudness: its effects are louder than its music. |
| By ear | `startup-sound` | Pending: the owner has the A/B files. |

## The game itself

`render-audio` runs the same `Game::tick` and `Game::take_audio` as the game, which spec section 7 accepts as the check. The game binary was also run on its own Xvfb display with SDL's disk audio driver (`SDL_AUDIO_DRIVER=disk`, the sound servers and ALSA unreachable), which writes what the frontend plays to a file and never reaches a device:

- 92 s from start-up: the file holds our `render-audio --startup` samples bit for bit, apart from the audio gate's corrections (`deadrally_core::host::AudioGate`): 20 underruns padded with silence and a repeated frame in most ticks, which the disk driver's clock provoked. 4523 of the 9499 10 ms pieces of the render appear verbatim; every other piece holds one of those single repeated frames.
- No stream appeared on the sound server and the game never opened `/dev/snd`, checked every second.

An earlier attempt routed the game to a PulseAudio null sink with `PULSE_SINK`. SDL3 ignores that variable: the game's stream went to the real output, and about 3 s of the intro may have reached the speakers before the check stopped the run. `scripts/spike-check.sh` and `scripts/fullscreen-check.sh` relied on the same variable and now use the disk driver too.

## What the original does that the spec did not say

Found by the checks above and the research for M2, and now part of the spec (section 3.3) and the code (`crates/core/src/audio/`, `crates/core/src/startup.rs`):

- **The menu music starts as the intro ends.** `mainMenu` stops the intro's sound, starts `MEN-MUS` at order 45 and only then shows the logos and the title. The first recordings ended 1 s after the intro and held quiet sound there with unequal channels; it was first taken for an artefact of the recording path, and is the menu music starting.
- **The intro always plays at full volume;** `dr.cfg`'s volumes apply from the menu music on. A fresh `dr.cfg` has the music at 50 %, FMOD master volume 63 against the intro's 127.
- **The module's master volume applies:** the intro's music was 2.5 dB too loud until it was scaled by the modules' master volume, 48/64.
- **A stereo module plays at twice a mono module's level:** at master volume 63, the stereo menu music is as loud as our player made it at 127 before this was known.
- **FMOD's ticks are whole samples at 44.1 kHz:** the menu music (tempo 141) ran 0.1 % slower than the recording until each tick was rounded down as FMOD does (781 samples, not 781.9).
- **Stereo modules play their channels fully on one side:** with linear pans the menu music's left-to-right balance was 2.1 to 5.1 dB off the recording; with full sides it is within 0.05 dB. `compare-audio` has measured the balance since then.

`MUSICS.BPA` also showed that every order list holds several sections separated by 255, which the game starts by number (spec section 3.2); the menu music starts in its fourth section, which jumps back into the first.

## Observed, not reproduced

Spec section 3.4: effects sound about 45 ms late against the music at the start of the intro and 70–100 ms late at its end: the effects' stream, plus the recording's clock running about 0.1 % fast against the game's under Wine.

## Scenarios

- `scripts/reference/startup-sound.scenario`: no keys; the whole intro and 40 s of the menu music, recorded until 122 s.
- `scripts/reference/menu-sound.scenario`: keys skip the intro and both logos; then 45 s of the menu music. Recorded twice: as it is, and with `--cfg` and a `dr.cfg` whose music volume is 100 %, made from the one the default run leaves behind:

  ```
  python3 -c "import struct, sys; d = bytearray(open(sys.argv[1], 'rb').read()); struct.pack_into('<i', d, 8, 0x10000); open(sys.argv[2], 'wb').write(d)" ~/.cache/deadrally/reference/run/dr.cfg captures/music-100.cfg
  ```

## Runs

The `compare-audio` output this record is based on. The lag moves by some tens of milliseconds from run to run, with Wine's start-up.

### startup-sound

```
$ deadrally-headless render-audio --startup --seconds 122 --out captures/startup-sound/ours.wav
$ deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115
lag: 1990 ms (envelope correlation 0.946)
overlap: 120.2 s
loudness per second, ours - original: median 0.29 dB, largest 2.55 dB
octave bands, ours - original: 63 Hz +0.9 dB, 125 Hz +0.5 dB, 250 Hz +0.2 dB, 500 Hz +0.2 dB, 1000 Hz +0.2 dB, 2000 Hz +0.0 dB, 4000 Hz +0.1 dB, 8000 Hz +0.4 dB
tempo, ours - original: +0.018 % (over 12 pieces of 10 s)
pitch, ours - original: +0 cents
stereo balance (left - right), ours - original: largest +0.20 dB (over 12 pieces of 10 s)
result: PASS
```

### menu-sound at two music volumes

The second recording is "ours" here; the comparison fails on purpose, since the two differ in volume. The median is the measure.

```
$ deadrally-headless compare-audio captures/menu-sound/sound.wav captures/menu-sound-100/sound.wav --min-overlap 40
lag: 20 ms (envelope correlation 0.979)
overlap: 50.1 s
loudness per second, ours - original: median 6.17 dB, largest 6.34 dB
```

Second by second: seconds 2 to 4, still the intro, differ by +0.31, +0.26 and −0.26 dB, since the intro ignores `dr.cfg`'s music volume; second 5, where the menu music starts, by 4.5 dB; from second 6 on, the menu music, by 5.9 to 6.6 dB.

The menu music with linear pans, before the stereo fix, against a default recording:

```
stereo balance (left - right), ours - original: largest -5.08 dB (over 4 pieces of 10 s)
result: FAIL (stereo balance off by more than 1 dB)
```
