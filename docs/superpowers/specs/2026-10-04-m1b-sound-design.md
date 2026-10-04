# M1b — Sound: design

- **Date:** 2026-10-04
- **Status:** sections 1–4 agreed with the owner in brainstorming (2026-10-04); the owner then asked for the rest (section 5 onwards, the written spec and its plan) to be done without further questions. Amended the same day with what measuring a working player against the original found (marked *measured*); section 9 lists the decisions taken on the owner's behalf.
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

From DreeRally (`sfx/sound.c`, `sfx/minifmod/soundSystem.c`, `asset/haf.c`; read-only), confirmed by parsing the owner's copy, and, where marked, measured against recordings of the original (section 5).

### 3.1 CMF files

- All sound is in `MUSICS.BPA`: 16 entries named `*.CMF`.
- **Encoding** (`getMusicStream`): byte *b* at position *p* becomes `rotl8(b, p mod 7) − 109 − 17·p` (mod 256). The result is a plain module file.
- **Contents:**

| Files | Format | Use |
|---|---|---|
| `MEN-MUS` | S3M, stereo, 10 channels, 94 orders | menu music (M2) |
| `TR0-MUS` … `TR9-MUS` | S3M, mono, 7–16 channels | race music (M4); `TR0-MUS` also plays during the intro and the end animations |
| `SANIM-E` | XM, 40 instruments (13 empty), linear frequencies | intro effects |
| `ENDANI-E`, `ENDANI0E` | XM, 40 (33 empty) and 5 instruments | end-animation effects (M6) |
| `GEN-EFE` | XM, 44 instruments (9 empty) | race effects (M4) |
| `MEN-SAM` | XM, 31 instruments (16 empty), Amiga frequencies | menu effects (M2) |

### 3.2 What the modules use

- **S3M** (format 0x1320, samples unsigned):
  - 13 effect commands only: A (speed), B (jump), C (break), D (volume slide), E/F (porta down/up), G (tone porta), H (vibrato), K (vibrato + volume slide), O (sample offset), Q (retrigger), SDx (note delay), T (tempo).
  - Samples: 8-bit, mono, unpacked; some looped.
  - Initial speed 3, 6 or 8; tempo 125 (141 for `MEN-MUS`); global volume 21–64; master volume 48; all but `MEN-MUS` are flagged mono.
  - **Sections:** every order list holds two to six sections separated by 255. The game starts later sections with `musicSetOrder` (M2, M4). Every section ends with a B jump back into itself, so playback never reaches a 255.
  - Every module has a default panning table (byte 53 is 252). In `MEN-MUS`, the only stereo module, channels alternate between the left half (5) and the right half (11).
- **XM effect banks** (version 0x104):
  - One sample per instrument; no volume or panning envelopes; no auto-vibrato.
  - Samples 8-bit (a few 16-bit), delta-encoded; loops none, forward or ping-pong.
  - An instrument without a sample, or with a sample of length 0, is empty.
  - Every effect is panned 128 (centre).
  - Each bank has one 64-row pattern; it is never read (3.3).
  - Every file parses exactly to its end.

### 3.3 How the original plays them

- **Music** goes through FMOD 3 (`fmod.dll`, `FMUSIC_LoadSongEx`).
  - Its master volume is `255 · 255 >> 9 = 127` of 256 at the default settings, so music plays at about half volume.
  - *Measured:* the module's own master volume applies on top, as 48/64 (−2.5 dB).
  - *Measured:* **a tick is a whole number of samples at 44.1 kHz,** `floor(44100 · 5 / (2 · tempo))`: 781 instead of 781.9 at tempo 141, so the menu music runs 0.1 % fast. A new tempo already sets the length of the tick that sets it.
  - *Measured:* **a stereo module's channels play fully on their side:** left half fully left, right half fully right. Linear pans (5/15, 11/15) put the menu music's left-to-right balance 1–5 dB off the recording; full sides match it within 0.05 dB.
  - **FMOD drops 254 and 255 from the order list:** the game numbers orders for `FMUSIC_SetOrder` without them (`loadMusic` builds the table, `musicSetOrder` 0x43C320 uses it). Whether a B jump in a later section counts orders with or without the markers is open; M2 measures it when it first starts a later section. Until then B counts them, as in Scream Tracker; in first sections both give the same order.
- **Effects** go through a modified minifmod linked into `dr.exe`, mixed at 44 100 Hz into an FMOD stream at volume `255 · 255 >> 8 = 254` of 255.
  - The bank is "played" as a song, but its update (`FMUSIC_UpdateXM`, 0x43EF50) only processes effects and envelopes. **It never reads the pattern**, so the bank makes no sound by itself.
  - **Triggering** (`loadMenuSoundEffect` → `FMUSIC_UpdateXMNote`, 0x43EC40): effect *k* on channel *c* plays instrument *k − 1* with:
    - XM linear period `7680 − (6 · pitch / 65536 + 46 + relative note) · 64 − finetune / 2`; pitch `0x10000` is the normal one, note 52 + relative;
    - volume byte `(volume · 64 >> 16) + 16`; `0x10000` gives 64;
    - the sample's own panning.
  - **Final volume** (0x43E8C0): `64 · volume · fadeout · global volume · 255 / (64 · 64 · 65536 · 64) · ½`, i.e. 127 of 255 at full volume.
  - **Frequency:** `8363 · 2^((4608 − period) / 768)`, with the linear formula for every bank, `MEN-SAM` included.
  - **Retriggering a busy channel** moves the old voice to a spare channel and ramps it to silence instead of cutting it.
  - **Mixer:** linear interpolation between neighbouring samples; volume ramps over `128 · rate / 44100` samples on every volume change and at sample start and end.
- **The intro** (`openAnimation`, 0x4185B0):
  1. Loads `TR0-MUS` as music and `SANIM-E` as effects.
  2. Starts the music just before the first frame.
  3. After drawing each frame whose effect byte is not 0, triggers that effect at full volume and normal pitch on the next channel of 1, 2, …, 6, 1, …
  4. When it ends, stops effect channels 1–6; its caller then stops the music and the effects' stream.

  The logos and the title play nothing. The menu music (`MEN-MUS` with `MEN-SAM`) starts when the main menu appears (M2).

### 3.4 What the recordings show and DeadRally does not reproduce

- **Effects sound late against the music:** about 45 ms at the start of the intro, 70–100 ms at its end. The constant part is the effects' stream (minifmod fills it in blocks of 1024 samples, 23 ms each); the growing part is the recording's audio clock running about 0.1 % fast against the game's clock under Wine. DeadRally triggers effects on the tick.
- **After the intro ends,** the recording holds about 1 s of quiet sound, 12 to 30 dB below the intro, whose left and right channels differ by up to 12 dB. Everything the intro plays is centred (mono music, effects panned 128), so this is the recording path, not the game.

## 4. Architecture

### 4.1 `deadrally-gamedata`

Parsing only:

- **`cmf`:** `decode(&[u8]) -> Vec<u8>` (3.1).
- **`s3m`:** `Module::parse` reads:
  - the header: title, the order list as stored (with its 254 and 255 markers), initial speed and tempo, global volume, master volume and its mono flag, channel settings (enabled, left or right half) and the default panning table;
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
  - **Integers only.** `2^(x/768)`, the ST3 period table and the vibrato sine come from built-in tables (`tables`), not `powf` (banned in the core).
- **`music`:** the music player.
  - Rows and ticks; a tick lasts FMOD's whole number of 44.1 kHz samples (3.3), converted to 48 kHz with the remainder carried.
  - Effects A, B, C, D, E, F, G, H, K, O, Q, SDx, T, with Scream Tracker's semantics. Where FMOD differs, the reference recordings decide.
  - Orders 254 and 255 are skipped (3.3).
  - Global volume and the module's master volume. Mono modules are centred; a stereo module's channels play fully on their side.
- **`effects`:** the effect player. `trigger(channel, effect, volume, pitch)` does what `FMUSIC_UpdateXMNote` does (3.3). The bank's song tick is kept only as far as it changes sound, which for these banks is not at all: there are no envelopes and fadeout needs a key-off that never comes.
- **Output:** music × 127/256 plus effects × 254/255, summed in 64-bit integers per output sample, so nothing wraps or clips before the output, and clipped to `i16` there. Music and effect volume settings stay at the defaults (255) until M2's Configure menu.
- **Timing:** `Game::tick` renders exactly 672 frames, and the players advance by samples, not by game ticks.
- **Public:** `render_music(&Module, frames)` and `render_effect(&Bank, effect, frames)` render as the game mixes, for the headless tool and the data tests.

### 4.3 The startup scene

- **Intro start:** music from order 0 and the effect bank ready.
- **When the scene shows a frame:** if its effect byte is not 0, trigger it on the next channel of 1–6.
- **When the intro ends** (last frame or key): music and effect channels stop. The intro's last frame and the frame at which a key ends the intro are never shown, so their effects never start; in the original they start and are cut at once, which sounds the same.
- **Logos and title:** silence.
- `Game::take_audio` keeps its contract (M0).

### 4.4 Headless

- **`render-audio [--data PATH] (--intro [--key-at T]... | --music NAME [--seconds S] | --effect BANK --number K) --out FILE.wav`:** writes a 48 kHz 16-bit stereo WAV.
- **`compare-audio ORIGINAL.wav OURS.wav [--min-overlap S]`:**
  - aligns the two by the cross-correlation of their 10 ms loudness envelopes;
  - reports the lag, the duration of the overlap, the loudness difference per second (median and largest, in dB), the difference per octave band (in dB), the tempo difference (the slope of the lag measured in each 10 s piece), the pitch difference (in cents, from the spectra on a logarithmic frequency axis) and the stereo balance difference (left minus right, the largest over 10 s pieces);
  - exits 0 only within the section 5 tolerances; the overlap must be at least `--min-overlap` seconds (default 25), and a measure that cannot be taken fails (a tempo needs three 10 s pieces that line up, the balance a stereo piece that is not silent).

  Sample rates may differ (the original records at 44 100 Hz).
- `run --ticks N` (the test scene's hashes) is unchanged.

### 4.5 Reference runner

`scripts/reference-run.sh --sound` runs `dr.exe -window -nogl` without `-nosound`:

1. It loads a PulseAudio null sink `deadrally_ref_<pid>` and starts Wine with `PULSE_SINK` set to it.
2. It checks with `pactl` that `dr.exe`'s stream is on that sink, and stops at once if not, so nothing reaches the speakers.
3. It records the sink's monitor with `parec` to `OUT_DIR/sound.wav` at 44 100 Hz.
4. It unloads the sink at the end.

Scenarios as in M1a. New ones:

- `intro-sound.scenario`: the whole intro;
- `menu-sound.scenario`: keys skip the intro and the logos, then 45 s in the main menu.

## 5. Verification against the original

| Check | How | Passes when |
|---|---|---|
| Intro music and effects | `intro-sound` recording vs `render-audio --intro` | `compare-audio`: lag found; overlap ≥ 75 s; per-second loudness within ±1.5 dB (median) and ±4 dB (largest); octave bands 63 Hz–8 kHz within ±3 dB; tempo within ±0.15 %; pitch within ±10 cents; stereo balance within ±1 dB |
| Menu music | `menu-sound` recording vs `render-audio --music MEN-MUS --seconds 50` | as the intro, over ≥ 40 s |
| Effect pitch and loudness | effects heard in the intro recording | covered by the intro bands and loudness |
| By ear | the owner listens to both files side by side | recorded when the owner has listened; not a merge gate (the owner asked not to be asked) |

- **Records:** `docs/verification/m1b.md` keeps the measured numbers and the scenarios, never the recordings. Recordings stay under `captures/`.
- **If a check fails,** the player is corrected toward the original (pan law, interpolation, volume scaling, S3M effect details) until it passes. The deviations found are recorded in section 3.
- **Regression manifest:** after the checks pass, the SHA-256 of the whole intro as the game plays it, of the first 30 s of every music module and of every effect of every bank (2 s each) is pinned in `crates/headless/tests/rendered-audio.sha256` (hashes, not audio). `DEADRALLY_BLESS=1 cargo test-data` rewrites it, only after measuring again.

## 6. Tests

- **Without data:**
  - CMF decode of synthetic data (an encoder exists only in the test);
  - synthetic S3M and XM: unpacking, signedness, delta decoding, loops, the order list with its markers, `Unsupported` cases;
  - the mixer: interpolation, loops (forward, ping-pong), ramps, clipping at the output, retrigger hand-off;
  - S3M timing (FMOD's whole ticks at tempo 125 and 141, a new tempo's own tick);
  - each of the 13 commands on a hand-built module, section markers, full-side stereo, and the period and frequency tables at known points;
  - the scene: effects on channels 1–6 in turn, silence after the intro, a key that ends the intro stops the sound;
  - `compare-audio` on synthetic tones: another rate and delay compare equal; loudness, tempo, pitch and balance differences are reported; the exit status fails a quieter render, a short overlap and a tempo it could not measure;
  - determinism of a rendered synthetic module.
- **With data** (`cargo test-data`):
  - all 16 files decode to their type and parse to the end;
  - the 13 commands are the only ones used;
  - bank instrument counts;
  - `Assets` load the intro's sound;
  - the rendered-sound manifest.

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
| Integer mixing sounds different from FMOD's float mixer | 32.32 positions and 64-bit sums keep the difference below audibility; the loudness checks catch scaling errors |
| A mono measure hides a panning error | `compare-audio` measures the stereo balance too |

## 9. Decisions taken on the owner's behalf

Each with what it costs if it is wrong.

1. **The tempo check is a slope, not a drift:** "drift ≤ 30 ms over the intro" became "tempo within ±0.15 %" (the slope of the lag over 10 s pieces). The recording's clock runs about 0.1 % fast against the game's under Wine, so effects and music drift apart by about 80 ms over the intro whatever the player does. *Cost if wrong:* a tempo error below 0.15 % (under 120 ms over the intro) would pass.
2. **Pitch and stereo balance are measured** (±10 cents, ±1 dB): section 2 names both, and mono loudness and bands cannot see them. The balance check found the stereo law of 3.3. *Cost if wrong:* none beyond a stricter gate.
3. **The effects' latency and the tail after the intro are not reproduced** (3.4). *Cost if wrong:* effects about 45 ms earlier against the music than in the original, and no ring-out when the intro ends.
4. **The whole order list is kept** and playback skips 254 and 255 as FMOD does; a B jump counts the markers until M2 measures a later section (3.3). *Cost if wrong:* a later section looping at the wrong order, found as soon as M2 starts one.
5. **The rendered-sound manifest lives in `deadrally-headless`,** not `deadrally-gamedata`: the gamedata crate cannot render, and the core's lints forbid reading `DEADRALLY_DATA`. It also pins every effect of every bank, which nothing else renders yet. *Cost if wrong:* none; the file moves if a better home appears.
