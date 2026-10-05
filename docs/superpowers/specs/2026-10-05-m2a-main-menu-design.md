# M2a — Text and the main menu: design

- **Date:** 2026-10-05
- **Status:** written without the owner, who asked for the work to go on from M1b into M2 without questions (2026-10-04). Section 2 lists every decision taken on their behalf; section 3 marks facts from the original's code that the reference runs must still confirm (*to verify*).
- **Scope:** the first half of milestone M2 in [PROJECT_BRIEF.md](../../PROJECT_BRIEF.md) §7. M2b (Configure, Define Keyboard, Define Gamepad, Hall of Fame and `dr.cfg`) gets its own spec.
- **Builds on:** [M1a](2026-10-04-m1a-assets-design.md) (the picture catalogue, the startup sequence, `find`), [M1b](2026-10-04-m1b-sound-design.md) (the players, the menu music).

## 1. Goal

When M2a is done:

- **The title screen gives way to the main menu as in the original:** the fade to black, the menu fading in, its animated palette and cursor, the bottom message panel, the menu music going on.
- **The main menu works with the keyboard and a gamepad:** moving the highlight, the start submenu, the exit question and the end screen, the credits, each with the original's sounds.
- **Text renders glyph for glyph:** the original's fonts, metrics and strings, read from the player's own data, never copied into DeadRally.
- **Each screen is shown to match the original pixel for pixel,** and the menu's sounds to sound the same, measured against the original under Wine as in M1.

## 2. Decisions taken on the owner's behalf

Each with what it costs if it is wrong.

1. **M2 is split** like M1: **M2a** is text and the main menu; **M2b** is Configure, Define Keyboard, Define Gamepad, Hall of Fame and `dr.cfg`. Until M2b, those two main-menu items do nothing. *Cost if wrong:* none to the result; a later merge for the rest of M2.
2. **The original's strings and font metrics are read from the player's `dr.exe`,** the only place they exist (there is no text file). `dr.exe` joins the required data files and the known release; it is read, never run. DeadRally ships no game text. *Cost if wrong:* DeadRally needs the Windows version's executable among the data, which the Steam release has; a release without it (the DOS one) cannot show menus until another source is found.
3. **Items that lead to M3 do nothing yet:** the start submenu opens, highlights and closes, but its entries (new game, load and the rest) wait for M3. *Cost if wrong:* none; a player sees a menu whose entries do not respond.
4. **Volumes are the original's defaults** (music 50 %, effects 75 %) until M2b reads and writes `dr.cfg`. *Cost if wrong:* as in M1b.
5. **Loading takes no time:** where the original loads graphics between screens and its clock runs on, DeadRally loads everything at start-up and moves straight on, as M1a did for the logos. *Cost if wrong:* a fade step or two that the original drops after a slow load is shown by DeadRally.
6. **The game can end:** the exit item's "yes" shows the end screen and then asks the frontend to quit, as the original exits. *Cost if wrong:* none.

## 3. Facts about the original

From the Windows `dr.exe` (addresses below are its functions), read through DreeRally (`ui/menu.c`, `ui/util/popup.c`, `asset/imageUtil.c`, `graphics.c`, `dr.c`; read-only) and checked in the disassembly where DreeRally is known to differ (its `doc/FINDINGS.md`, `doc/KNOWN-ISSUES.md`).

### 3.1 Text

- **Strings live in `dr.exe` only,** NUL-terminated, at fixed addresses of the known release. M2a needs: the menu text table (9 menus × 9 rows × 50 bytes at 0x446368; a row may hold byte 0xFA, drawn as a 1-pixel gap), the four bottom-panel start-up lines, the exit question, and "yes" and "no".
- **Fonts** are `MENU.BPA` images cut into glyph cells, glyph *k* for character `k + 32`, colour 0 transparent; the colours, outlines and shading are in the pictures:

| Font | Cell | Glyphs | Use |
|---|---|---|---|
| `F-BIG3A` | 32 × 32 | 96 | selected item, popup text |
| `F-BIG3B` | 32 × 32 | 96 | active item |
| `F-BIG3D` | 32 × 32 | 96 | dim item, unfocused menu |
| `F-SMA3A`, `-B`, `-C` | 16 × 16 | 96 | captions, panel text (three colours) |
| `F-MED1A` | 9 × 12 | 62 | Hall of Fame (M2b) |

- **Metrics** are byte tables in `dr.exe` (`{width, height, advance[96]}`): big at 0x445848, small at 0x4458B0, medium at 0x445928. The advance of character *c* is `table[c − 30]` (the two-byte header, then glyph `c − 32`).
- **Drawing** (`drawTextWithFont`, 0x41A2D0): each byte draws its glyph transparently at the pen and advances it; 0xFA advances 1 pixel without drawing; no clipping, no colour parameter.

### 3.2 From the title to the main menu (`mainMenu`, 0x43A020)

1. The menu music has been playing since the intro ended (M1b). The title has faded in to 92 % (M1a).
2. The graphics load (instant here, decision 5); `transitionToBlack` (0x427300) fades the title from 100 % to 0 % over 26 ticks; its first frame shows the pending 96 % step (M1a).
3. The menu palette is set up (`loadPaletteMenu` 0x419EA0, `sub_418B00`, `sub_4224E0` 0x4224E0): `MENU.PAL`, the player colour's ramp (entries 64–95), the copper ramp (176–182), the background copper rows (192–223) and the pulsing entries (16–31).
4. `MENUBG5` is drawn whole; the bottom panel is framed (`drawTransparentBlock(0, 371, 639, 109)`: background restored, `CHATLIN1` at rows 372 and 471) and its lines drawn (`drawBottomMenuText`, 0x41E810); the main menu's popup and items are drawn (`drawMenu`, 0x41A880).
5. The palette fades in from 0 % to 98 % in steps of 2 over 50 ticks, the cursor turning every other tick. It stays at 98 % (*to verify:* entry 15 shows as 240, not 244).

### 3.3 The main menu

- **Layout** (`dr.exe` 0x4456F0, 7 values per menu: count, x, y, row height, width, height, selection): main menu 6 rows at (145, 124), 28 high, 349 × 192; start submenu 6 rows at (109, 171), 421 × 192.
- **Active rows** (0x4457F0): main menu rows 0, 2, 3, 4, 5 (row 1, multiplayer, is always inactive); start submenu rows 0, 3, 5.
- **Popup** (`createPopup`, 0x41A530): fill colour 196 over rows Y+2..Y+H−7 and columns X+2..X+W−5; 32 × 20 corners from `CORN3A` (focused) or `CORN3B`; border lines in colour 7 (focused) or 4.
- **Rows** (`drawMenu`): text at (X+32, Y+5+28i); the selected row in `F-BIG3A` with the 20 × 20 cursor (`CURSOR`, 50 frames) at (X+9, Y+11+28i) when the menu has focus; other active rows in `F-BIG3B` (focused menu) or `F-BIG3D`; inactive rows in `F-BIG3D`.
- **The loop** (`readEventInMenu`, 0x42E0B0): every pass takes two ticks, turns the cursor one frame and reads the one remembered key (M1a's `eventDetected`).
  - Up and Down move the highlight, wrapping and skipping inactive rows (`refreshMenuUp` 0x41AF40, `refreshMenuDown` 0x41B1A0), with effect 25.
  - Enter, Space and keypad Enter choose the row, with effect 28.
  - Escape in the main menu moves the highlight to the last row (exit) with effect 25 if it is not there already; in a submenu it closes it with effect 22.
- **Keys repeat** while held, as SDL 1.2's `SDL_EnableKeyRepeat(500, 30)` set up by the original: first after 500 ms, then every 30 ms.
- **A gamepad** steers the menus through `eventDetected`: the stick as the arrow keys, buttons as Enter and Escape, repeating after 400 and then every 700 ms (*to verify* the order).
- **Items:** 0 opens the start submenu (`startRacingMenu`, 0x439CD0: the main menu dims, the submenu gets focus); 1 is inactive; 2 and 3 wait for M2b; 4 shows the credits (`showCredits`, 0x4274E0); 5 asks whether to exit.
- **Exit question:** the main menu dims; a popup (170, 200, 300 × 80) with the question in `F-SMA3A` at (253, 208); "yes" and "no" in big letters (`drawYesNoMenu`, 0x42E310), "no" first selected; Left/Y and Right/N move, Enter/Escape confirm. "Yes" shows `END.BMP` (`showEndScreen`) for at most 560 ticks or until a key, then the game ends.
- **The palette moves while the menu waits:** entries 16–31 pulse every tick (100 % down to 49 % and back over 34 ticks, `sub_4220D0` 0x4220D0); the background copper rows step every 70 ticks (0x42A570).

### 3.4 The bottom message panel

- 22 rows of up to 150 bytes, each with a font (small A, B or C); new lines push the old ones up (`bottomMenuText` 0x462000, fonts 0x461EC0).
- At start-up it holds four lines from `dr.exe` and one empty line, in small B, in rows 17–21.
- `drawBottomMenuText` restores rows 380–468 from `MENUBG5` and draws rows 16–21 at (12, 378 + 15k). It is redrawn only when a screen is rebuilt.

### 3.5 Sound

- `MEN-SAM` is loaded with `MEN-MUS` when the intro ends; its effects play on channel 1 at the configured effects volume and pitch `0x28000` (`loadMenuSoundEffect`, 0x43C380).
- At the default 75 % the volume reaches the sound twice: as its volume byte (`(volume · 64 >> 16) + 16`) and as the effects stream's volume (`255 · 192 >> 8 = 191` of 255) (*to verify* by recording).

## 4. Architecture

### 4.1 `deadrally-gamedata`

- **`exe`:** reads a PE file's sections and gives the bytes and NUL-terminated strings at virtual addresses.
- **`text`:** the strings and font metrics M2a needs, by address, checked to be printable and terminated; an error names the address.
- **Required files:** `DR.EXE` joins `REQUIRED_FILES` and the known release (size and SHA-256 of the Steam executable).
- **`Assets`** gains the menu's pictures (fonts, `CORN3A`/`B`, `CURSOR`, `MENUBG5`, `CHATLIN1`, `CREDIT1`/`2`), palettes (`MENU.PAL`, `COPPER.PAL`, `BGCOP.PAL`, the credits'), `END.BMP`, `MEN-SAM` and the texts.

### 4.2 `deadrally-core`

- **`canvas`:** an indexed picture with the original's operations: copy a region, blit with colour 0 transparent, fill a rectangle.
- **`font`:** a font's glyphs and advances; draw and measure a byte string.
- **`menu`:** the main menu scene: the fade from the title, the palette, the popups, the cursor, navigation, the submenu, the exit question, the end screen and the credits; the bottom panel.
- **`keys`:** the one remembered key of the original (scancodes), key repeat, and the gamepad's mapping, shared by the startup and the menus.
- **`Game`:** the startup hands over to the menu scene when the title is done; `Game::quit_requested()` tells the frontend the player chose to exit.
- **Sound:** `Sound` loads `MEN-SAM` when the intro ends and triggers effects at a given volume and pitch; a new bank fades the old one's effects out.

### 4.3 Frontend and headless

- The frontend shows 640 × 480 frames as it shows the startup's, and quits when the game asks.
- `find`, `render` and `render-audio --startup` run on through the menus with `--key-at` keys; `find` gains the keys a scenario presses (arrows, Enter, Escape, Y, N).

## 5. Verification against the original

| Check | How | Passes when |
|---|---|---|
| Title to menu | a scenario with shots through the fade to black, the menu's fade-in and its idle palette | every shot equals one of our frames exactly (`find`) |
| Text and popups | shots of the main menu, the start submenu and the exit question | equal exactly |
| Navigation | keys moving the highlight, wrapping, Escape to the last row | each shot after a key equals one of our frames |
| Credits, end screen | shots through both | equal exactly |
| Sound | a recording of the menu while keys move the highlight, against `render-audio --startup` with the same keys | `compare-audio` as in M1b over the menu part |

Records go to `docs/verification/m2a.md`; shots and recordings stay under `captures/`.

## 6. Tests

- **Without data:** PE sections and strings on a synthetic executable; text rendering (advances, the 0xFA gap, transparency); popups and rows on a synthetic canvas; navigation (wrapping, skipping inactive rows, Escape to the last row); key repeat timing; the fade and palette steps; the exit question; quitting.
- **With data:** `DR.EXE` is the known release; every string M2a reads is printable and terminated; the menu assets load; a manifest of our menu frames at fixed ticks (hashes), written after the shots match.

## 7. Done criteria

1. Merged to `master` with CI green on the merge commit.
2. `cargo test` and `cargo test-data` pass.
3. Every check of section 5 passes and is recorded in `docs/verification/m2a.md`.
4. The game shows the main menu after the title and quits from it.

## 8. Risks

| Risk | Mitigation |
|---|---|
| DreeRally's menu code has known errors (popup fill width, palette loops that run once, the key switch) | each is checked in the disassembly; the screenshots decide |
| Palette animation makes screenshots time-dependent | `find` searches every tick, as for the fades in M1a |
| Key timing under Wine moves a scenario's keys by a few ticks | the shots compare pictures, not times; `find` reports where each falls |
| An unknown `dr.exe` has its strings elsewhere | texts are checked when loaded; an error names the address |
