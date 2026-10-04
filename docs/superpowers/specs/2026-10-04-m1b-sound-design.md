# M1b — Sound: design

- **Date:** 2026-10-04
- **Status:** sections 1–4 agreed with the owner in brainstorming (2026-10-04); the owner then asked for the rest (section 5 onwards, the written spec and its plan) to be done without further questions.
- **Scope:** the second half of milestone M1 in [PROJECT_BRIEF.md](../../PROJECT_BRIEF.md) §7, after [M1a](2026-10-04-m1a-assets-design.md).
- **Builds on:** M1a (decoders, `Assets`, the startup scene, `scripts/reference-run.sh`, `deadrally-headless find`).

## 1. Goal

When M1b is done:

- **The intro sounds like the original's:** music `TR0-MUS` from its first order and the intro's effects from `SANIM-E`, each at the frame the HAF table names. Everything stops when the intro ends; the logos and the title stay silent, as in the original.
- **DeadRally can play every sound of the original data:** the 11 S3M music modules and the 5 XM effect banks, so M2 (menus) and M4 (races) only connect them.
- **Every module, effect and the whole intro can be rendered to WAV locally,** for listening.
- **The intro and the main-menu music are shown to sound the same as the original,** measured against recordings of the original made under Wine, where nothing reaches the speakers.

## 2. Owner decisions (brainstorming, 2026-10-04)

| Decision | Choice |
|---|---|
| Fidelity | **The same by ear:** tempo, pitch, loudness, panning and timing of effects match the original; measured against recordings. Not bit-exact: FMOD 3's music mixer is closed source. |
| Player | **Our own, in Rust, in `deadrally-core`:** deterministic on every OS, tuned to the original by measurement. Not `xmrsplayer` (floats, unknown distance from FMOD) and not libopenmpt (C++ FFI, tuned to the trackers rather than to FMOD). |
| Output | 48 kHz stereo `i16` as since M1a. The mix is kept in 32 bits and **clipped to 16 bits only at the output**, as the original does (minifmod's `FSOUND_MixerClipCopy`); no limiter, no float output. |
| Scope | Sections 1–4 below as presented; the title stays silent until M2 starts the menu music. |
| Process | "Work and do not ask until the limit": the written spec, the plan and the execution follow without further owner gates; decisions are recorded and reported. M2 follows M1b. |

## 3. Facts about the data and the original

From DreeRally (`sfx/sound.c`, `sfx/minifmod/soundSystem.c`, `asset/haf.c`; read-only) and confirmed by parsing the owner's copy.

### 3.1 CMF files

- All sound is in `MUSICS.BPA`: 16 entries named `*.CMF`.
- **Encoding** (`getMusicStream`): byte *b* at position *p* becomes `rotl8(b, p mod 7) − 109 − 17·p` (mod 256). The result is a plain module file.
- **Contents:**

| Files | Format | Use |
|---|---|---|
| `MEN-MUS` | S3M, stereo, 16 channels, 94 orders | menu music (M2) |
| `TR0-MUS` … `TR9-MUS` | S3M, mono, 7–10 channels | race music (M4); `TR0-MUS` also plays during the intro and the end animations |
| `SANIM-E` | XM, 40 instruments (13 empty), linear frequencies | intro effects |
| `ENDANI-E`, `ENDANI0E` | XM, 40 and 5 instruments | end-animation effects (M6) |
| `GEN-EFE` | XM, 44 instruments | race effects (M4) |
| `MEN-SAM` | XM, 31 instruments, Amiga frequencies | menu effects (M2) |

### 3.2 What the modules use

- **S3M** (format 0x1320, samples unsigned):
  - 13 effect commands only: A (speed), B (jump), C (break), D (volume slide), E/F (porta down/up), G (tone porta), H (vibrato), K (vibrato + volume slide), O (sample offset), Q (retrigger), SDx (note delay), T (tempo).
  - Samples: 8-bit, mono, unpacked; some looped.
  - Initial speed 3, 6 or 8; tempo 125 (141 for `MEN-MUS`); global volume 21–64; master volume 48; all but `MEN-MUS` are flagged mono.
- **XM effect banks** (version 0x104):
  - One sample per instrument; no volume or panning envelopes; no auto-vibrato.
  - Samples 8-bit (a few 16-bit), delta-encoded; loops none, forward or ping-pong.
  - Each bank has one 64-row pattern; it is never read (3.3).
  - Every file parses exactly to its end.

### 3.3 How the original plays them

- **Music** goes through FMOD 3 (`fmod.dll`, `FMUSIC_LoadSongEx`). Its master volume is `255 · 255 >> 9 = 127` of 256 at the default settings, so music plays at about half volume.
- **Effects** go through a modified minifmod linked into `dr.exe`, mixed at 44 100 Hz into an FMOD stream at volume `255 · 255 >> 8 = 254` of 255.
  - The bank is "played" as a song, but its update (`FMUSIC_UpdateXM`, 0x43EF50) only processes effects and envelopes. **It never reads the pattern**, so the bank makes no sound by itself.
  - **Triggering** (`loadMenuSoundEffect` → `FMUSIC_UpdateXMNote`, 0x43EC40): effect *k* on channel *c* plays instrument *k − 1* with:
    - XM linear period `7680 − (6 · pitch / 65536 + 46 + relative note) · 64 − finetune / 2`; pitch `0x10000` is the normal one, note 52 + relative;
    - volume byte `(volume · 64 >> 16) + 16`; `0x10000` gives 64;
    - the sample's own panning.
  - **Final volume** (0x43E8C0): `64 · volume · fadeout · global volume · 255 / (64 · 64 · 65536 · 64) · ½`, i.e. 127 of 255 at full volume.
  - **Frequency:** `8363 · 2^((4608 − period) / 768)`.
  - **Retriggering a busy channel** moves the old voice to a spare channel and ramps it to silence instead of cutting it.
  - **Mixer:** linear interpolation between neighbouring samples; volume ramps over `128 · rate / 44100` samples on every volume change and at sample start and end.
- **The intro** (`openAnimation`, 0x4185B0):
  1. Loads `TR0-MUS` as music and `SANIM-E` as effects.
  2. Starts the music just before the first frame.
  3. After drawing each frame whose effect byte is not 0, triggers that effect at full volume and normal pitch on the next channel of 1, 2, …, 6, 1, …
  4. When it ends, stops effect channels 1–6; its caller then stops the music.

  The logos and the title play nothing. The menu music (`MEN-MUS` with `MEN-SAM`) starts when the main menu appears (M2).

## 4. Architecture

### 4.1 `deadrally-gamedata`

Parsing only:

- **`cmf`:** `decode(&[u8]) -> Vec<u8>` (3.1).
- **`s3m`:** `Module::parse` reads:
  - the header: title, orders (254 skipped, 255 ends), initial speed and tempo, global volume, master volume and its mono flag, channel settings (enabled, left or right half);
  - samples: C2SPD, volume, loop; 8-bit unsigned data is converted to signed;
  - patterns, unpacked into rows × channels of note, instrument, volume, command, parameter.

  Anything the data does not use (16-bit, stereo or packed samples, AdLib instruments) is an `Unsupported` error naming the file.
- **`xm`:** `Bank::parse` skips the patterns and reads instruments. Each has one sample, decoded from delta encoding to `i16`, with loop type, volume, finetune, relative note, panning and fadeout. Empty instruments are allowed. Envelopes, several samples per instrument and auto-vibrato are `Unsupported`.
- **`sound`:** which `MUSICS.BPA` entry is music and which is an effect bank; `load_music(&Archive, name)` and `load_effects(&Archive, name)`.
- **`Assets`** gains `intro_music: s3m::Module` (`TR0-MUS`) and `intro_effects: xm::Bank` (`SANIM-E`).

### 4.2 `deadrally-core::audio`

- **`mixer`:**
  - **Voices:** a sample (`i16`), a position in 32.32 fixed point, a step `frequency / 48000`, linear interpolation, left and right volumes, and loops (none, forward, ping-pong).
  - **Ramps:** volume changes ramp over 128 · 48000 / 44100 = 139 samples; a retriggered voice moves to a spare voice and ramps out (3.3).
  - **Integers only.** `2^(x/768)` and the ST3 period table come from built-in tables, not `powf` (banned in the core).
- **`s3m`:** the music player.
  - Rows and ticks; a tick lasts 2.5 / tempo s, which is 960 samples at tempo 125. Each tick's length is computed exactly in samples, carrying the remainder.
  - Effects A, B, C, D, E, F, G, H, K, O, Q, SDx, T, with Scream Tracker's semantics. Where FMOD differs, the reference recordings decide.
  - Global volume. Mono modules are centred; stereo channels follow their left or right setting.
- **`sfx`:** the effect player. `trigger(channel, effect, volume, pitch)` does what `FMUSIC_UpdateXMNote` does (3.3). The bank's song tick is kept only as far as it changes sound, which for these banks is not at all: there are no envelopes and fadeout needs a key-off that never comes.
- **Output:** music × 127/256 plus effects × 254/255, summed in `i32` per output sample and clipped to `i16`. Music and effect volume settings stay at the defaults (255) until M2's Configure menu.
- **Timing:** `Game::tick` renders exactly 672 frames, and the players advance by samples, not by game ticks.

### 4.3 The startup scene

- **Intro start:** music from order 0 and the effect bank ready.
- **When the scene shows a frame:** if its effect byte is not 0, trigger it on the next channel of 1–6.
- **When the intro ends** (last frame or key): music and effect channels stop. The intro's last frame and the frame at which a key ends the intro are never shown, so their effects never start; in the original they start and are cut at once, which sounds the same.
- **Logos and title:** silence.
- `Game::take_audio` keeps its contract (M0).

### 4.4 Headless

- **`render-audio [--data PATH] (--music NAME [--seconds S] | --effect BANK K | --intro [--key-at T]...) --out FILE.wav`:** writes a 48 kHz 16-bit stereo WAV.
- **`compare-audio ORIGINAL.wav OURS.wav`:**
  - aligns the two by the cross-correlation of their 10 ms loudness envelopes;
  - reports the lag, the duration of the overlap, the loudness difference per second (median and largest, in dB) and the difference per octave band (in dB);
  - exits 0 only within the section 5 tolerances.

  Sample rates may differ (the original records at 44 100 Hz).
- `run --ticks N` (the test scene's hashes) is unchanged.

### 4.5 Reference runner

`scripts/reference-run.sh --sound` runs `dr.exe -window -nogl` without `-nosound`:

1. It loads a PulseAudio null sink `deadrally_ref` and starts Wine with `PULSE_SINK=deadrally_ref`.
2. It checks with `pactl` that `dr.exe`'s stream is on that sink, and stops at once if not, so nothing reaches the speakers.
3. It records the sink's monitor with `parec` to `OUT_DIR/sound.wav` at 44 100 Hz.
4. It unloads the sink at the end.

Scenarios as in M1a. New ones:

- `intro-sound.scenario`: the whole intro;
- `menu-sound.scenario`: about 30 s idle in the main menu.

## 5. Verification against the original

| Check | How | Passes when |
|---|---|---|
| Intro music and effects | `intro-sound` recording vs `render-audio --intro` | `compare-audio`: lag found; overlap ≥ 75 s; per-second loudness within ±1.5 dB (median) and ±4 dB (largest); octave bands 63 Hz–8 kHz within ±3 dB |
| Tempo | onset lag at the start vs the end of the intro | drift ≤ 30 ms over the intro |
| Menu music | `menu-sound` recording vs `render-audio --music MEN-MUS --seconds 40` | as the intro, over ≥ 25 s |
| Effect pitch and loudness | effects heard in the intro recording | covered by the intro bands and loudness |
| By ear | the owner listens to both files side by side | recorded when the owner has listened; not a merge gate (the owner asked not to be asked) |

- **Records:** `docs/verification/m1b.md` keeps the measured numbers and the scenarios, never the recordings. Recordings stay under `captures/`.
- **If a check fails,** the player is corrected toward the original (pan law, interpolation, volume scaling, S3M effect details) until it passes. The deviations found are recorded in this spec's section 3.
- **Regression manifest:** after the checks pass, the SHA-256 of `render-audio --intro` and of the first 30 s of every music module is pinned in `crates/gamedata/tests/rendered-audio.sha256` (hashes, not audio).

## 6. Tests

- **Without data:**
  - CMF decode of synthetic data (an encoder exists only in the test);
  - synthetic S3M and XM: unpacking, signedness, delta decoding, loops, `Unsupported` cases;
  - the mixer: interpolation, loops (forward, ping-pong), ramps, clipping at the output, retrigger hand-off;
  - S3M timing (960 samples per tick at tempo 125, remainders at other tempi);
  - each of the 13 commands on a hand-built module, and the period and frequency tables at known points;
  - the scene: effects on channels 1–6 in turn, silence after the intro, a key that ends the intro stops the sound;
  - determinism of a rendered synthetic module.
- **With data** (`cargo test-data`):
  - all 16 files decode to their type and parse to the end;
  - the 13 commands are the only ones used;
  - bank instrument counts;
  - `Assets` load the intro's sound;
  - the rendered-audio manifest.

## 7. Done criteria

1. Merged to `master` with CI green on the merge commit.
2. `cargo test` and `cargo test-data` pass.
3. Every check of section 5 passes and is recorded in `docs/verification/m1b.md`.
4. The intro plays with sound in the game. Without a monitor or speakers this is checked by the WAV the frontend's audio path would get, through `render-audio`, which uses the same `Game::take_audio`.
5. The owner has the A/B files for the intro and the menu music. Their listening is recorded when it happens.

## 8. Risks

| Risk | Mitigation |
|---|---|
| FMOD 3's S3M playback differs from Scream Tracker in details | measured against recordings; the tolerances are by ear, not bit-exact |
| Wine's audio path resamples or adds latency | `compare-audio` aligns by cross-correlation and compares bands and loudness, not waveforms |
| The recording reaches the speakers | `PULSE_SINK` plus a `pactl` check that stops the run otherwise |
| Integer mixing sounds different from FMOD's float mixer | 32.32 positions and 32-bit sums keep the difference below audibility; the loudness checks catch scaling errors |
