# DeadRally: project brief

> **One sentence:** build a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) that plays exactly like the original on Windows, macOS and Linux. Players supply the original game data. The code is our own. Two existing reverse-engineering projects serve as the reference.

This document is the starting point for whoever builds DeadRally. It covers:
- what we are building;
- what we already have;
- how to work so that the result really matches the original;
- the first steps.

Decisions that still belong to the project owner are listed at the end.

---

## 1. Goal and definition of "done" for 1.0

**1.0 is done when:**

- **The single-player game is complete:** new game, licence, sign-up, races, shop, Underground Market, loans and sponsors, the hitman and steroid offers, save/load (8 slots), Hall of Fame, the final race against The Adversary, and the end animation.
- **It plays like the original:**
  - same track offers, opponents and grid for the same random seed;
  - same car physics, AI, weapons, mines, power-ups, damage and money;
  - same menus and texts, glyph for glyph.
- **Native 64-bit builds** for Windows, macOS (Apple Silicon and Intel) and Linux, from one codebase, built by CI.
- **Features the 2009 Windows version already had:** fullscreen or window (Alt+Enter), OpenGL-style scaling to the desktop resolution, optional bilinear smoothing, keyboard and gamepad (one stick, four buttons), redefinable controls.
- **No game data in the repository.** The player points the game at an installed copy: Steam "Death Rally (Classic)" or Remedy's 2009 freeware download.

**Not in 1.0**, but planned:
- multiplayer: the Windows release removed it; the DOS version had IPX and modem play;
- widescreen and HD;
- mods;
- the owner's own additions, e.g. a "ROLEPLAY" mode, still to be specified.

---

## 2. Ground rules

1. **Faithfulness first, improvements second.** Anything that changes how the game plays (timings, physics, prices, AI) must match the original first. Improvements come afterwards, as options that default to the original behaviour.
2. **Never commit game assets.** That means BPA, HAF, the original exe, sound or music files, or screenshots that are mostly original art. Tests that need assets read them from a path set by the developer (an environment variable or a config file).
3. **Licence: GPL-3.0** (already in the repo). Know where each piece of code comes from:
   - **New code:** written by us, GPL-3.0.
   - **Ideas, facts, file formats and constants** from the reference projects: fine to use. Describe them in your own words and code, and credit them in the docs.
   - **Copied code from dRally (MIT):** allowed. Keep its copyright notice; MIT is GPL-compatible.
   - **Copied code from DreeRally (CC BY-SA 4.0):** CC BY-SA 4.0 is one-way compatible with GPL-3.0, so adapted material may go into a GPL-3.0 project with attribution. **But** DreeRally is a decompilation of Remedy's binary. Prefer re-implementing from understanding over pasting decompiled code.
   - This is not legal advice. If in doubt, ask the owner before copying anything.
4. **Every gameplay claim needs evidence:** a parity test, a side-by-side screenshot, or a reference to the original's code. See section 6.

---

## 3. What we start from

| Source | What it is | Use it for | Watch out |
|---|---|---|---|
| **The original `dr.exe`** (Steam / Remedy freeware, 2009) | Remedy's official Windows port. Jari Komppa made it from the original source code; it uses SDL 1.2, OpenGL and FMOD. Its readme is in the game folder (`readme/index.html`). | **Ground truth.** Run it for side-by-side comparison. Read its disassembly when in doubt. | 32-bit Windows only. On macOS/Linux use CrossOver/Wine. Never modify the installed copy. |
| **DreeRally** — [victortrnka/DreeRally](https://github.com/victortrnka/DreeRally/tree/foundation) | A readable C decompilation of that `dr.exe`, kept 1:1 function-for-function and **playable**. Its `foundation` branch (to be merged into `0.3.x`) has over 200 `fix:` commits, each verified against the original. | **Executable specification.** When you need to know exactly what the game does, read the corresponding DreeRally function (original address in its `//----- (00XXXXXX)` marker), or run it and log state. | Still decompiled code: ints used as pointers, Hex-Rays names (`dword_45EB50`), 32-bit only. Read `doc/FINDINGS.md` before trusting any odd-looking expression. |
| **dRally** — [urxp/dRally](https://github.com/urxp/dRally) | An independent decompilation of the **DOS** version (1996). It runs natively, 64-bit, on Linux with SDL2. MIT. | A second opinion on game logic: the same source, a different compiler. Its struct layouts with offsets (`drally_structs_fixed.h`); how it solved 64-bit and the sound playback; DOS multiplayer (IPX). | Its addresses do **not** map to the Windows exe; match functions by strings and constants. The DOS version differs in video, sound, input, the CD check and multiplayer. |

The game data is identical between the DOS and Windows versions: dRally needs the same BPA files.

### DreeRally material worth reading first

- `doc/FINDINGS.md`: every class of decompilation bug found so far, with examples, and the methods that worked.
- `doc/KNOWN-ISSUES.md`: what is still open, plus the deliberate deviations.
- `doc/DEVELOPMENT.md`: how to build, run and compare with the original. On macOS: clang-cl and xwin, then CrossOver. On Windows: the Visual Studio solution.
- Tools in DreeRally:

| Tool | What it does |
|---|---|
| `make docker-test` (`ORIG=1`) | Headless Wine runner: plays the port or the original with scripted keys and takes screenshots. |
| `make orighook` | A hook DLL injected into a copy of the original exe, to log what the original does at any address. |
| `make check-equiv` | Machine-code equivalence of two builds. |
| `make verify-tables` | Compares data tables with the original's bytes. |
| `make signcheck`, `make calldiff` | Signedness and missing-call diffs against the original. |

---

## 4. Recommended approach: new code, DreeRally as the oracle

Write DeadRally as a **new, clean codebase** with its own architecture, types and names. Port it **module by module**. Prove each module behaves like the original by comparing it with DreeRally or with `dr.exe` (parity testing, section 6).

Why this approach:
- **Starting from nothing usually fails on the details:** the car "feels wrong", the AI is off, the prices drift. A working, verified reference removes the guesswork.
- **Forking the decompilation and refactoring it** (the OpenRCT2 / DevilutionX path) is the other proven route. It reaches "playable" sooner but drags along decompiled code and its provenance.
- **New code:** the owner chose a fresh project. With an oracle beside it, new code is both clean and accurate.

How it works in practice:
1. **Pick a module** (see the order in section 7).
2. **Specify it** from the DreeRally source, the original's behaviour and dRally:
   - inputs, outputs and state;
   - exact constants and tables, copied as data with their original address noted;
   - corner cases.
3. **Implement it** in DeadRally style, with tests.
4. **Compare with the oracle.** It passes when the logs match: same seed, same inputs, same state per frame or the same screen.
5. **Merge.** Record anything that intentionally differs.

---

## 5. Architecture sketch (proposal, owner decides)

Language and libraries are open decisions (section 11). The recommendation is **C++20, SDL3 and CMake**: SDL covers window, input, gamepad and audio on all three platforms, and C++ allows clean types while staying close to the reference C code.

```
platform/   window, fullscreen toggle, input (keyboard/gamepad), timing, file paths
assets/     BPA archive reader, BPK image decoder, palettes, HAF animations,
            XM/sample loading, original-data locator + validator
gfx/        8-bit indexed framebuffers (640x480 menus, 320x200 race),
            palette fades, sprite/font blitting, GPU presentation + scaling
ui/         menu system (flat text table semantics), popups, fonts, screens
game/       campaign state, drivers, economy (shop, market, loans, sponsors),
            race selection, save/load (DR.SGn compatible), config
race/       track loading, physics, AI, weapons/mines/power-ups, damage,
            HUD, 3D elements, smoke/skid marks, race intro/outro
audio/      XM music + effects mixer (FMOD/minifmod behaviour), volumes
tools/      asset dumpers, parity harness, screenshot diff
```

Design rules that come from the original:
- **The simulation runs in fixed ticks of 1/70 s**; lap times are stored in 1/70 s. Rendering is decoupled from it. Keep a deterministic core: no wall-clock reads inside game logic.
- **Use a deterministic RNG that reproduces MSVC's `rand()`:** `seed = seed * 214013 + 2531011; return (seed >> 16) & 0x7FFF`. Call it **in the same order as the original**; otherwise parity is impossible, because opponents, tracks and events all depend on it.
- **Render into indexed 8-bit buffers**, as the original does, so palette effects (fades, flashes, the greyscale race intro) work, and convert to RGBA only at presentation. Menus are 640×480; the race view is 320×200, drawn into a buffer with a row stride of 512.
- **Physics fields are floats wherever the original uses floats.** Several old bugs came from ints where the original had floats.
- **Read the original's own file formats directly.** Do not convert assets in a build step that would need redistributable output.

---

## 6. Verification: parity testing

This is what keeps "from scratch" honest.

- **Seeded runs.** Force the same seed (the DreeRally work used a temporary `srand(9)` at the race-selection function, 0x4240B0, and an orighook hook for the same in the original exe) and feed the same scripted inputs. DreeRally's key injector supports held keys (`+up`/`-up`).
- **State logs.** Dump per-tick state from the oracle (DreeRally instrumentation, or orighook in the original exe): car positions, velocities, damage, money, rand() calls with callers, the offered tracks and opponents. Dump the same from DeadRally and diff.
- **Screens.** Compare screenshots of the same screen pixel by pixel, or by glyph mask for text: menus, shop, HUD, results.
- **Audio, where it matters.** Capture to WAV and compare levels and timing. Sound code only runs when a real audio device exists.
- **Tolerances.** The original uses x87 80-bit floats, so ±1 differences in some palette or position values are expected. Document every tolerance you accept.
- **Automate it.** Put parity checks in CI wherever the assets are available (a self-hosted runner or a developer's machine). Unit tests without assets run everywhere.

---

## 7. Roadmap

Each milestone ends with a parity check against the oracle.

| # | Milestone | Done when |
|---|---|---|
| M0 | **Repo foundations** | CMake builds on Win/macOS/Linux in CI; the asset path is configurable; code style is agreed; the CLAUDE.md/CONTRIBUTING are written; the game data is detected and validated. |
| M1 | **Assets** | BPA/BPK/palette/HAF/XM loaders. A tool dumps every image to PNG locally (never committed). The intro animation plays. |
| M2 | **Menus and text** | The main menu, Configure, Define Keyboard/Gamepad and Hall of Fame render glyph-identical to the original. The bottom message panel works. |
| M3 | **Campaign without racing** | New game, licence, sign-up screen with the same three tracks per seed, shop, Underground Market, loans and sponsors, save/load compatible with `DR.SG0..DR.SG7`. |
| M4 | **Race: draw and drive** | Tracks, cars, HUD, camera and the race intro/outro. Player physics matches the oracle tick by tick for scripted inputs. |
| M5 | **Race: everything else** | AI, collisions, weapons, mines (32 slots), power-ups, pedestrians, damage, smoke and skid marks, results and money. Opponents match per seed. |
| M6 | **Full game** | The whole campaign to The Adversary, end animation, Hall of Fame, music and effects, gamepad. |
| M7 | **1.0 polish** | Fullscreen/window/scaling options, smoothing, settings UI, installers or packages, docs. |
| M8+ | **Beyond the original** | Widescreen, online multiplayer (DOS IPX knowledge from dRally), mods, ROLEPLAY, and so on. Every one is optional, and the original behaviour stays available. |

---

## 8. Traps we already fell into

From DreeRally's `doc/FINDINGS.md`. Expect them whenever you read decompiled code from either reference.

- **Strings built through stack "walks":** Hex-Rays shows `strcpy` into neighbouring locals as pointer walks. Reimplement them as plain string building, or they overflow.
- **1-byte stand-ins:** `_UNKNOWN unk_XXXXXX` is really a table or a buffer. Find the real size in the original's data.
- **Strided tables declared as one row,** and hand-typed tables with typos. Always take tables byte-exact from the original's data.
- **Signedness:** `sar` versus `shr`, `movsx` versus `movzx`, signed versus unsigned compares, signed `char` colours. Negative values break everything.
- **int versus float:** physics fields, positions and the nose probe were ints in the decompilation but floats in the original.
- **Reader index offsets:** the font advance tables are read as `table[c - 30]`. A correct table read with the wrong offset is still wrong.
- **Code the previous port author commented out or "tuned":** volume shifts, frequency factors, clamps, skipped calls. Compare with the original before keeping anything that looks odd.
- **The random sequence matters:** one extra or missing `rand()` changes tracks and opponents.
- **Wrong per-car stride or wrong car index:** a corner computed with the *player's* Y put every AI car's smoke hundreds of pixels off.
- **Restored `free()` calls exposed old heap overruns:** an XM loader that loaded two instruments too many. Run memory checks (ASan/UBSan) from day one.

---

## 9. Key facts

| Thing | Value |
|---|---|
| Original exe | `dr.exe`, 32-bit, MSVC 7.1 runtime (`msvcr71.dll`), SDL 1.2, OpenGL, FMOD (`fmod.dll`) |
| Data files | `ENGINE.BPA`, `IBFILES.BPA`, `MENU.BPA`, `MUSICS.BPA`, `TR0.BPA`..`TR9.BPA`, `SANIM.haf`, `ENDANI.haf`, `ENDANI0.HAF`, `end.bmp`, `rmd.bmp` |
| Saves / config | `DR.SG0`..`DR.SG7` (encrypted; slot 7 = Quicksave), `dr.cfg` |
| Resolutions | menus 640×480, race view 320×200 (8-bit indexed, VGA-style 6-bit palette) |
| Time base | 70 ticks per second |
| Command line (original) | `-window`, `-nogl`, `-smooth` (F12 toggles), `-nosound`; fullscreen by default, Alt+Enter toggles |
| Window caption | `DR` |
| Menu text | one flat table: 50-byte rows, 9 rows per menu; rows are rewritten in place (e.g. "Continue Racing") |
| Mines | 32 slots per race |
| Multiplayer | not available in the Windows release |

Exact addresses for all of these are in the DreeRally source and docs.

---

## 10. First week

1. Get the game: install *Death Rally (Classic)* from Steam, or Remedy's 2009 freeware. Play a few races of the original.
2. Clone **DreeRally** and build and run it (see its `doc/DEVELOPMENT.md`). Read `FINDINGS.md` and `KNOWN-ISSUES.md`.
3. Clone **dRally**. Skim `drally_structs_fixed.h` and its asset loaders.
4. Set up the DeadRally skeleton for M0: CMake, SDL, CI matrix (windows-latest, macos-latest, ubuntu-latest), formatting/lint, ASan/UBSan in debug, an asset-path setting, and a `CLAUDE.md` / `CONTRIBUTING.md` with the rules from section 2.
5. Write the BPA reader and an asset dump tool (M1) and check the output against DreeRally's loaders (`asset/bpaUtil.c`, `decryptTexture`).
6. Open a GitHub issue per milestone, then split each into module issues.

---

## 11. Working with AI coding agents (optional, recommended)

This setup worked well for DreeRally:
- **Small, self-contained tasks.** Give each one a written brief: goal, files, evidence required, commit rules.
- **Isolation.** One git worktree per task. Merge only after an independent review.
- **"Done" means verified:** a parity log or screenshot, not "it compiles".
- **Watch out for silent failures.** Agents sometimes report success on checks that never ran; for example, "sound tests" ran with sound disabled. Reviewers should re-run the key checks.
- **Rate limits.** Long parallel runs hit API usage limits. Commit often, so a stopped agent loses nothing.

---

## 12. Decisions for the owner

| Decision | Recommendation |
|---|---|
| Language | C++20 (alternatives: C11, closest to the references; Rust, safest but furthest away) |
| Platform layer | SDL3 (SDL2 if older Linux distros matter) |
| Renderer | indexed framebuffers uploaded as textures through SDL's GPU renderer |
| Music | own XM player matching FMOD, or libxmp, checked against the original's output |
| Reuse policy | re-implement; copy from dRally (MIT) only with notice; avoid pasting decompiled DreeRally code |
| Name and branding | "Death Rally" is Remedy's; check the project name and logo use before any public release |
| Contact Remedy? | optional, but goodwill helps a public fan project |

---

## Links

- DeadRally: https://github.com/victortrnka/DeadRally
- DreeRally (Windows decompilation, oracle): https://github.com/victortrnka/DreeRally
- dRally (DOS decompilation, MIT): https://github.com/urxp/dRally
- Original game: Death Rally (Classic) on Steam, or Remedy's 2009 freeware release
