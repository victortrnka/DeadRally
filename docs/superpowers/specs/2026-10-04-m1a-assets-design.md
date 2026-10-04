# M1a — Game data and pictures: design

- **Date:** 2026-10-04
- **Status:** agreed in brainstorming with the owner; this written spec awaits review.
- **Scope:** the first half of milestone M1 in [PROJECT_BRIEF.md](../../PROJECT_BRIEF.md) §7. M1b (sound) gets its own spec.
- **Builds on:** [M0 foundations](2026-10-03-m0-foundations-design.md) and [ADR 0001](../../adr/0001-platform-layer.md) (SDL3).

## 1. Goal

When M1a is done:

- **DeadRally reads every picture in the original data:** archives, compressed images, palettes, BMPs, track images and the HAF animations.
- **The game starts like the original, but silently:** the intro, the Apogee logo, the Remedy logo and the title screen, with the original's fades and timing.
- **Every image can be dumped to PNG locally.**
- **Each of these screens is shown to match the original pixel for pixel.** The original runs under Wine without a monitor, purely as a test tool.

## 2. Owner decisions (brainstorming, 2026-10-04)

| Decision | Choice |
|---|---|
| M1 split | **M1a** (data and pictures) now; **M1b** (music and effects, CMF/S3M/XM, mixer, intro with sound) later, with its own spec |
| What the game shows at the end of M1a | The original's startup sequence without sound; the M0 test scene only behind `-testscene` |
| Tick | **14 ms**, like the Windows version (`SDL_GetTicks()/14`), not DOS's 70 Hz. Section 5. |
| Palette display | **`v << 2`**, like the Windows version (6-bit 63 → 252) |
| Oracle | The original `dr.exe` run under Wine without a monitor; DreeRally and dRally are read-only documentation |

## 3. Facts about the data

These facts come from reading DreeRally (Windows decompilation, function addresses in its `//----- (00XXXXXX)` markers) and dRally (DOS decompilation), and were confirmed by parsing the owner's copy. All multi-byte values are little-endian. The DOS and Windows data files are identical; Windows adds only `rmd.bmp` and `end.bmp`.

### 3.1 BPA archives

- **Header:**
  - `u32 count` (≤ 255), then a fixed table of 255 entries of 17 bytes each (`name[13]`, `u32 size`).
  - Unused entries are zero.
  - Data starts at byte **4339**; entry *i* starts at `4339 + Σ size[0..i)`. No offsets are stored.
  - The sizes sum exactly to `file size − 4339`.
- **Names:**
  - 8.3 format, NUL-terminated, `name[12] = 0`.
  - Each byte before the NUL is stored as `c + (117 − 3·i) mod 256` (*i* = position); padding stays 0.
  - Lookup upper-cases ASCII letters and compares exactly.
- **Entry counts:** ENGINE 39, IBFILES 17, MENU 167, MUSICS 16, TR0 12, TR1–TR9 13 each.
- **No archive-level compression.** The extension says what an entry is: `.BPK` image, `.PAL` palette, `.TAB` 256-byte table, `.DAT`/`.BIN` data, `.CMF` music (M1b).

### 3.2 BPK images

- **LZW** with LSB-first bit packing and 9- to 12-bit codes:
  - 256 = end, 257 = clear (the reverse of GIF); new codes start at 258.
  - After a clear: width 9, next code 258, skip repeated clears, stop on end. Otherwise the next code is a literal that becomes "previous" without adding a dictionary entry.
  - Normal step:
    - Output the entry, or for the KwKwK case `entry[prev] + first(entry[prev])`.
    - If next < 4096, add `entry[prev] + first(current)`.
    - Widen the code when next reaches 2^width and width < 12.
- **Output transform:** every output byte is rotated right by 3, `(b >> 3) | (b << 5)`. The dictionary holds untransformed bytes.
- **Partial decode:** "skip *s* output bytes, return *n*". The original slices some images this way, e.g. TRSNAP2M into 20 × 14336.
- **Multi-frame entries:** shop and car animations are concatenated streams.
  - After a stream's end code, align to the next byte. If the next 9 bits are not 257, skip one more byte.
  - This reproduces the frame tables in `dr.exe`; they are not needed.
- **No header.** Pixels are 8-bit indexed and row-major; dimensions come from the code (the catalogue, 4.3).

### 3.3 Palettes and BMPs

- **`.PAL`:** 768 bytes, 256 × RGB with 6-bit components (0–63). `BGCOP.PAL` is 1536 bytes: two palettes.
- **`FRAMES.BPK`:** decodes to a 768-byte palette followed by a 320×200 image (the intro letterbox).
- **`rmd.bmp` and `end.bmp`:**
  - Uncompressed 8-bit BMPs, 640×480, rows stored bottom-up.
  - Colours are converted with `(c & 0xFC) >> 2`.
  - `end.bmp` defines only 223 colours; the rest are black.

### 3.4 Tracks

- **`TRn-INF.BIN`:** 127 × `i32`: width, height, zone count, 4 × (x, y, direction) start slots, 16 × (x, y) power-up spots, 20 × (x, y, id, flag) pedestrians.
- **`TRn-IMA/MAS.BPK`** (full size) and **`TRn-VAI/LR1.BPK`** (quarter size) decode to:
  - a 10-byte RIX3 header: `"RIX3"`, `u16 w`, `u16 h`, `0xAF`, `0x00`;
  - a 768-byte palette;
  - `w × h` pixels.
- **Checks:** the header dimensions must equal INF.BIN's. `TRn-FLIP.PAL` replaces the palette for a reversed track.
- **Later milestones:** `SCE`, `SHA`, `.TAB`, `DRV.DAT` and `OHI.DAT` are only extracted in M1a. They are decoded in M4.

### 3.5 HAF animations

- **Header:**
  - `u16 n`, then `u8 sfx[n]` (0 = none, *k* = effect instrument *k*, used in M1b) and `u8 delay[n]` in ticks.
  - Then *n* records, each `u16 len` followed by `len` bytes. The file ends exactly after the last record.
- **Frame record:** a 768-byte palette, the LZW minimum code size (8), GIF sub-blocks (a length byte 1–255 followed by data; 0 ends them), then the trailer `0x3B`.
- **LZW:** GIF-standard: clear 256, end 257, 9–12 bits, no rotation.
- **Each frame** is a full 320×120 picture (38400 pixels, values ≥ 16).
- **Stop rule:** stop at 38400 pixels, at the end code, or at the end of the data, whichever comes first. `ENDANI.haf` frame 200 writes its end code at the wrong width.
- **Contents:**

| File | Frames | Σ delay (ticks) |
|---|---|---|
| `SANIM.haf` (intro) | 1626 | 5732 |
| `ENDANI.haf` | 368 | 1620 |
| `ENDANI0.HAF` | 383 | 1915 |

### 3.6 Startup sequence of the Windows version

From DreeRally `ui/menu.c` (main), `asset/haf.c`, `ui/startGameScreen.c` and `dr.c`:

1. **Intro** (`SANIM.haf`, skippable).
2. **Apogee** (`apogeeScreen`, 0x427380):
   - black palette, then `APOGEE.PAL` and `APOGEE.BPK` (640×480);
   - fade in, hold up to 180 ticks or until a key, fade out.
3. **Remedy:** `rmd.bmp`, with the same fade-in, hold and fade-out.
4. **Title** (`showStartScreen`, 0x427880): black palette, then `STARTSCR.PAL` and `STARTSCR.BPK`, fade in; then the main menu (M2).

All the images are in `MENU.BPA`.

- **Fades:**
  - `transitionToCurrentImage` (0x427280) raises the brightness from 0 to 100 in steps of 4, one step per tick (26 ticks).
  - `transitionToBlack` (0x427300) lowers it from 100 to 0 the same way.
  - Each component is `(component · 65536/100 · level + 0x8000) >> 16` in the original's fixed point.
  - DreeRally shows an odd `− 4` on the red component during fade-out. Whether the original does this is checked against it (section 9).
- **Intro playback** (`openAnimation`, 0x4185B0):
  - Palette entries 0..254 go black. The `FRAMES.BPK` palette is set into entries 0–15 and its 320×200 image drawn.
  - For each frame: decode it, wait until `delay[i]` ticks after the previous frame, set palette entries 16–255 from the frame, and copy its 320×120 pixels to row 40.
  - A key press before a frame ends the intro. At the end the palette goes black.
- **Tick:** `SDL_GetTicks()/14` drives the whole Windows version: the same per-tick callback runs menus and races (`setBackgroundRefreshFunction`, `dr.c:10273`).

## 4. Architecture

### 4.1 Crates

| Crate | Change in M1a |
|---|---|
| `deadrally-gamedata` | New decoders (`bpa`, `bpk`, `palette`, `bmp`, `catalog`, `track`, `haf`) and an `Assets` loader that opens the validated data directory. Stays free of platform code. |
| `deadrally-core` | Depends on `deadrally-gamedata` (planned since M0). Tick, palette and audio constants change (section 5). New scenes. |
| `deadrally` (SDL frontend) | Loads the data, reports failures in a dialog, and adds `-testscene`. |
| `deadrally-headless` | `run` keeps using the test scene; new commands `dump-assets`, `render` and `compare`. |

### 4.2 `deadrally-gamedata` modules

- **`bpa`:**
  - `Archive::open(path)` reads the whole file (the largest archive is 5.7 MB) and validates the header (count ≤ 255, `name[12] = 0`, sizes sum to the file length).
  - `entry(name) -> Option<&[u8]>` matches case-insensitively.
  - Errors name the archive and the entry.
- **`bpk`:**
  - `decode(stream) -> Result<Vec<u8>, BpkError>` and `decode_range(stream, skip, len)`.
  - `split_streams(entry) -> Vec<&[u8]>` splits a multi-frame entry.
  - Errors: a code beyond the dictionary, or data ending before the end code.
- **`palette`:** `Palette` from 768 bytes; components above 63 are an error (the data has none).
- **`bmp`:** a minimal reader for 8-bit uncompressed BMPs (header fields, `biClrUsed`, bottom-up rows), returning `Palette` and pixels.
- **`catalog`:**
  - A static table of every image entry.
  - Each entry has: archive, entry name, width, height, frame count, layout (`Single` / `Streams` / `Slices { count }` / `Rix3Track` / `PaletteThenImage`), palette (`Named(entry)` / `Embedded` / `FirstBytes`), transparency (colour 0) and a `source` note (the DreeRally function and address the fact comes from).
  - Section 3 lists the facts known now; the remaining entries are filled in during implementation by reading DreeRally.
  - **Completeness rule:** every `.BPK` entry of every archive is either catalogued or listed in `catalog::NOT_IMAGES` with a reason.
- **`track`:** parses `INF.BIN` into a struct and decodes RIX3 track images, checking the header dimensions against `INF.BIN`.
- **`haf`:** `Animation::open(path)` reads the header and frame offsets; `frame(i) -> Result<HafFrame { palette, pixels: Box<[u8; 38400]> }>` decodes on demand.
- **`assets`:** `Assets::load(&Validation)` opens `MENU.BPA`, `SANIM.haf` and `rmd.bmp` and decodes what the startup sequence needs. Other archives open lazily (the dump tool).

## 5. Core changes

### 5.1 Time and colour

- **Tick:**
  - `TICK_NANOS = 14_000_000` replaces `TICKS_PER_SECOND`.
  - `Pacer` keeps its backlog in nanoseconds: `due = backlog / TICK_NANOS`.
  - The catch-up cap (5) stays.
- **Audio:**
  - Output becomes **48 kHz**, so a tick is exactly **672** stereo frames (`AUDIO_FRAMES_PER_TICK`).
  - The test-scene tone step is recomputed for 48 kHz.
  - The audio gate, drift correction and stats line are unchanged apart from their constants.
  - M1b may revisit the rate when real music arrives.
- **Palette:** `expand_6bit(v) = (v & 0x3F) << 2`, as in the Windows version (63 → 252, not 255). The test changes accordingly, and its comment says why.
- **The determinism hash changes;** that is expected and recorded.

### 5.2 Scenes

The `Game` holds a scene state machine and advances it once per tick. Frames are produced as in M0: indexed pixels, a 6-bit palette and the aspect ratio.

| Scene | Frame | Behaviour (ticks of 14 ms) |
|---|---|---|
| Intro | 320×200, 4:3 | Starts black with the letterbox (`FRAMES.BPK`, palette entries 0–15). Frame *i* appears `delay[i]` ticks after frame *i − 1* (the first after `delay[0]`), with its palette in 16–255 and its pixels at row 40. A key press ends the intro. The end turns black. |
| Apogee | 640×480, 4:3 | Fade in (26 ticks: brightness 0, 4, …, 100), hold up to 180 ticks or until a key, fade out (26 ticks). |
| Remedy | 640×480, 4:3 | As Apogee, with `rmd.bmp`. |
| Title | 640×480, 4:3 | Fade in (26 ticks), then stays. M2 continues from here. |

- **API:**
  - `Game::new(&Assets)` starts the sequence.
  - `Game::test_scene()` gives the M0 scene, for `-testscene`, headless runs without data, and CI.
- **Keys:** "a key" means any key or pad button press forwarded by the frontend. Whether a press during a fade-in skips the following hold is decided by the reference check (section 9). The scene code isolates that rule in one place.
- **No decoding at tick time beyond one HAF frame per shown frame.** Decoding a frame takes well under a tick.

## 6. Frontend

- **Finding the data:** the same sources as `check-data` (`--data`, `DEADRALLY_DATA`, config).
- **Failures:**
  - If no source is given, the data is unusable, or a decoder fails, the game shows an SDL message box and writes the same text to stderr, then exits with status 1.
  - The message says where it looked, what is missing or broken, and how to set the path.
  - An unknown data version shows a warning dialog, and the game then starts.
- **Flags:** `-window`, `-novsync` and `-testscene`. Unknown flags are an error, as in M0.

## 7. Headless

- **`run --ticks N`:** unchanged; it uses the test scene, so CI's cross-OS determinism job keeps working without game data.
- **`dump-assets [--data PATH] --out DIR`:**
  - Writes every catalogued image as PNG, with its palette expanded `v << 2`.
  - Multi-frame and sliced images become one PNG with the frames side by side; track images are written whole.
  - It refuses to write inside the game data directory.
  - The default `DIR` is `dumps/` in the working directory, which `.gitignore` covers.
- **`render [--data PATH] --tick T --out FILE.png`:** runs the startup sequence for `T` ticks without input and writes that frame. `--key-at TICK` (repeatable) injects key presses.
- **`compare A.png B.png`:**
  - Reports the size, the number of differing pixels and the largest channel difference. It exits 0 only for identical images.
  - `--search` takes a set of candidate frames instead (all intro frames, or all 26 fade levels) and reports which one matches exactly.
- **New dependency:** the `png` crate, in `deadrally-headless` only. `core` stays free of image formats.

## 8. Reference runner (the oracle)

`scripts/reference-run.sh <scenario> <out-dir>`:

1. **Copies the validated game files** into `~/.cache/deadrally/reference/run/`. The original writes `dr.cfg` next to itself, so the install is never used directly.
2. **Creates once a 32-bit Wine prefix** at `~/.cache/deadrally/reference/wineprefix` (`WINEARCH=win32`, `WINEDLLOVERRIDES="mscoree,mshtml="` so Wine does not ask for Mono or Gecko). The prefix only hosts the 32-bit original; DeadRally itself is 64-bit native.
3. **Starts the original:** Xvfb on a free display (`-displayfd`), then `dr.exe -window -nogl -nosound`. Software rendering gives exact pixels, and nothing reaches the speakers.
4. **Runs the scenario:** a text file with lines `at <ms> key <name>` and `at <ms> shot <label>`. Keys go through xdotool and shots through `import -window`.
5. **Writes** the PNGs and a log to `<out-dir>`, which lives under `captures/` (ignored by Git). Screenshots of original art are never committed.

Scenarios live in `scripts/reference/*.scenario`. The tools come from `scripts/install-linux-deps.sh --local` (Wine 9 with `wine32`, Xvfb, xdotool, ImageMagick).

## 9. Verification against the original

Because the original's timing under Wine jitters, comparisons **search for a match** instead of comparing at a fixed time.

| Check | How | Passes when |
|---|---|---|
| Intro frames (HAF decoder, palettes, letterbox) | about 8 screenshots spread over the intro; `compare --search` over all 1626 of our rendered intro frames | each screenshot equals one of our frames exactly |
| Apogee, Remedy, title (BPK, BMP, palettes) | screenshots during the holds and after the title fade | pixel-identical to our render |
| Fade formula, including the red `− 4` question | screenshots during a fade-in and a fade-out; `--search` over all 26 levels | an exact match at some level; otherwise the formula is corrected until it matches |
| Key during fade-in | a scenario presses a key during the Apogee fade-in | our rule reproduces whether the hold was skipped |
| Timing | a screenshot every 200 ms; intro length and hold lengths measured from frame changes | within ±5 % of `Σ delay × 14 ms` and `(26 + 180 + 26) × 14 ms` |

- **First unknown to settle:** how the original shows a 320×200 screen in a 640×480 window (for example doubled to 640×400 and centred). `compare` takes an `--scale` and offset once this is known.
- **Records:** `docs/verification/m1a.md` keeps the results (pass/fail, which frame or level matched, image hashes) and the scenario files used, never the images.
- **Regression manifest:** after the visual checks pass, a SHA-256 manifest of every decoded catalogue image is committed (`crates/gamedata/tests/decoded-images.sha256`). Hashes are facts about the data, not the data.

## 10. Tests

- **Without data** (everywhere, CI included):
  - hand-built bit streams for both LZW variants (clear, end, KwKwK, a code beyond the dictionary, truncated input, width growth to 12, the HAF stop rule);
  - synthetic BPA archives (name codec, count > 255, `name[12] ≠ 0`, sizes not summing);
  - a synthetic BMP (bottom-up rows, a short palette);
  - the scene timeline on synthetic assets (fade levels per tick, a hold ended by a key, intro frame timing from delays);
  - the pacer at 14 ms and the 6-bit expansion `v << 2`.
- **With data** (`cargo test-data`):
  - entry counts and size sums for every archive;
  - catalogue completeness (every `.BPK` is covered);
  - every catalogued image decodes to exactly `w × h × frames` bytes;
  - multi-frame splits give the documented frame counts (ENGIx 24, TIREx 12, ARMORx 16, CONTANI 23, REPAANI 24, car spinners 64);
  - RIX3 headers match `INF.BIN` on all ten tracks;
  - the HAF frame counts and delay sums of 3.5;
  - the decoded-images manifest.
- **Tests encode why** (owner rule 9). The name or a comment says what a player would see go wrong.

## 11. Brief updates

Done in M1a's documentation task:

- **§5:** the design rule "fixed ticks of 1/70 s" becomes "fixed ticks of 14 ms, as the Windows version (`SDL_GetTicks()/14`); DOS used 70 Hz". Note that lap times will be checked against the Windows display in M4.
- **§7:** M1 is split into M1a (this spec) and M1b (sound).
- **§12:** "Tick" is recorded as decided. The "Music" row stays open for M1b.

## 12. Done criteria

1. The decoders, catalogue, `Assets`, scenes, frontend changes and headless commands are merged, and CI is green on the merge commit to `master`.
2. `cargo test` passes everywhere. `cargo test-data` passes locally against the Steam data, including catalogue completeness and the manifest.
3. `deadrally-headless dump-assets` writes every catalogued image. The owner has looked through the PNGs and found nothing obviously broken.
4. Every check in section 9 passes and is recorded in `docs/verification/m1a.md`, or any deviation is explained there and accepted by the owner.
5. The game starts like the original without sound: intro (skippable), Apogee, Remedy, title. It is checked by the owner on a real display, or by `scripts/fullscreen-check.sh` plus screenshots if no display is available.
6. The brief is updated (section 11).

## 13. Risks

| Risk | Mitigation |
|---|---|
| The 320×200 presentation in the original's window is unknown | the first verification task measures it; `compare` takes a scale and offset |
| The original under Wine behaves differently from Windows (timing, rendering) | software rendering (`-nogl`) for pixels; ±5 % tolerance for timing; anything odd is recorded |
| Catalogue facts are wrong (dimensions come from code) | the completeness and size tests catch wrong products of `w × h`; the PNG dump and the reference screenshots catch wrong `w` with the right product |
| Wine prompts or crashes on first start | a prefix created once with prompts disabled; the runner reports a failed start loudly |
| `ENDANI` and other HAF edge cases | the stop rule plus a test over all frames of all three files |
