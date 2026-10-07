# M7 — 1.0 polish: design

- **Date:** 2026-10-07
- **Status:** written without the owner, who asked to go on milestone after milestone. Section 2 lists the decisions taken on their behalf. The 1.0 release itself (a tag and published packages) waits for the owner's word.
- **Scope:** milestone M7 of the brief: what the 2009 Windows version offered around the game (its command-line options), packages for the three systems, and the player's documentation.
- **Builds on:** [M6](2026-10-07-m6-full-game-design.md).

## 1. Goal

A player downloads DeadRally for their system and points it at their copy of the game. They get the original's display options and keep their settings between runs. A README tells them how to do all of this.

## 2. Decisions taken on the owner's behalf

1. **M7 comes in parts:**
   - M7a, the display options;
   - M7b, the settings kept between runs and the first start;
   - M7c, the packages;
   - M7d, the documentation.

   *Cost if wrong:* none; this only sets the order of work.
2. **No settings window of our own.** The original's settings are its menus (Configure) and three command-line options; DeadRally adds the same options and keeps them in its config file. The only new screen is the system's folder dialog when no game data is found. *Cost if wrong:* an options page later, as an M8 improvement.
3. **Packages, not installers:**
   - Windows: a zip with `deadrally.exe`;
   - macOS: a universal `DeadRally.app` (Apple Silicon and Intel) in a zip;
   - Linux: a tar.gz.

   All are built by CI from a `v*` tag. None is signed: the README says how to open an unsigned app. SDL3 is already linked in statically, so each package holds one program. *Cost if wrong:* an installer or signing later; nothing in the game changes.
4. **Smoothing follows the original:** it applies only to the 320x200 screens (the race, the intro and the animations), not to the 640x480 menus. *Cost if wrong:* none; it can become an option.

## 3. Facts about the original

- **Command-line options** (parsed at 0x43AC50; documented in the 2009 readme):
  - `-nogl` clears 0x456A24 and uses the software renderer.
  - `-smooth` clears 0x456A20, so bilinear filtering is on.
  - `-window` clears 0x456A28: windowed instead of fullscreen.
  - Alt+Enter toggles fullscreen.
  - F12 toggles 0x456A20 (0x43BCB9).
- **The OpenGL output** (0x43B5B4):
  - In the 320x200 mode (0x456C14 = 0x13), the frame is uploaded as a 320x200 texture, filtered `GL_NEAREST` while 0x456A20 is set and `GL_LINEAR` otherwise (0x43B6A2).
  - The 640x480 mode has its own path (0x43B7C0), which does not read 0x456A20.
- **The software output** (0x43B9A1):
  - The 320x200 mode is doubled into 640x400 from row 40 while 0x456A20 is set (0x43B9B9).
  - Smoothed (0x43BA99), each pixel goes to the even column of its even row. Each odd column is then the mean of its two neighbours, and each odd row the mean of the rows above and below it. A mean is `((a & 0xFCFCFC) + (b & 0xFCFCFC)) >> 1`, so each channel loses its low two bits first.
  - The last column (639) is never written there, so it keeps what the window last showed in it. The last odd row is a mean with row 440, below the picture.

## 4. Parts

- **M7a, the display options:**
  - `-smooth`, which starts with smoothing on.
  - Smoothing, and F12's toggle, only on the 320x200 screens.
  - `-nogl`, the software renderer's picture: 640x480, the 320x200 screens doubled at row 40 or, with smoothing on, smoothed as 0x43BA99 does; shown at the largest whole scale that fits.
  - `-window`, Alt+Enter and the desktop-resolution scaling are in already (M0).
- **M7b, settings and the first start:**
  - `config.toml` keeps `window`, `smooth`, `nogl` and `vsync` next to `data_path`, and the command line overrides them.
  - When no game data is found, the system's folder dialog (SDL3's) asks for it, the choice is checked as `check-data` does, and it is written to `data_path`.
- **M7c, the packages:** a release workflow on `v*` tags builds the three packages above. Each package holds the README and the licence, and the macOS one also has the app's icon and `Info.plist`.
- **M7d, the documentation:**
  - The README for players: where to get the game data, where DeadRally keeps `dr.cfg`, its saves and its config, the options and keys, and what is known to differ from the original (from `docs/verification/`).
  - A CHANGELOG.
  - The brief's status brought up to date.

## 5. Verification

- **M7a:** a test per option for the option parser and the scaling arithmetic.
  - Screenshots of the original under Wine with `-nogl` (which the reference runner always passes) and `-nogl -smooth`, matched against our software picture.
  - The OpenGL smoothing is checked by eye, since a GPU's filtering is not pixel exact.
- **M7b:** tests of the config file's round trip and of the command line's precedence; the dialog is tried by hand on Linux.
- **M7c:** the workflow also runs by hand (`workflow_dispatch`) without publishing anything. Its artifacts are downloaded, and each program is started headless (`-testscene` with a time limit) on its system's CI runner.
- **M7d:** the README's commands are run as written.
