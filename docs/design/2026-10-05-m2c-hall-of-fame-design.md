# M2c — The Hall of Fame: design

- **Date:** 2026-10-05
- **Status:** written without the owner, who asked for M2 to go on without questions and for the usage limit to be spared; section 2 lists the decisions taken on their behalf; [docs/verification/m2c.md](../../verification/m2c.md) records the checks against the original.
- **Scope:** the main menu's "Hall of Fame" row: the best ten and the records by circuit. It completes milestone M2.
- **Builds on:** [M2a](2026-10-05-m2a-main-menu-design.md), [M2b](2026-10-05-m2b-configure-design.md) (`dr.cfg` already carries the records and the Hall of Fame).

## 1. Goal

Choosing the Hall of Fame shows the original's two screens with their wipes and music, matching the original pixel for pixel, from the player's `dr.cfg`.

## 2. Decisions taken on the owner's behalf

1. **Names are upper-cased in `dr.cfg` as the original does:** showing the best ten upper-cases their names in the configuration itself (`_strupr` in place), so the next save writes them so. *Cost if wrong:* none; the original's own file changes the same way.
2. **No new-record highlighting yet:** the original draws a record set in the race just run in another font; races come with M3, so every record is drawn in the medium font. *Cost if wrong:* none until M3.

## 3. Facts about the original

`seeHallOfFame` (0x431510), `drawRecordByCircuit` (0x41E490), the wipes `sub_42C560` and `sub_42C4A0`, the mask blit 0x43B080; DreeRally `ui/hallOfFame.c`, `dr.c` (read-only).

- **Best ten:** built in a second buffer from the current screen: rows 105–366 restored from `MENUBG5`, `FAMETXT` (640 × 54) at row 84; for rank k (0–9) at y = 144 + 22k in the medium font (`F-MED1A`, 9 × 12): "k+1." at x 36 (x 28 for "10."), the name upper-cased at x 137, the races right-aligned by digit count (x 344, 336, 328), the difficulty's name upper-cased at x 429. The entries are `dr.cfg`'s 10 of 20 bytes: name, races (+12), difficulty (+16); difficulty names at 0x447340 + 24d.
- **The wipe** (`sub_42C560`): 43 steps, one wait each; step i puts the new screen over the old through 22 × 10 tiles of `15X150` masks, tile column k with mask frame k, at linear offset 48000 + 15i + 9600 row + 15k, and copies 150 × 330 from offset 48000 + 15i to the screen. Entering, the music's volume mask falls from 65532 by 1524 a step; then the music jumps to order 81 and the mask is 0x10000.
- **Any key** moves on to the records (F1 too, outside a network game).
- **Records:** rows 84–366 restored, `RECOTXT` (640 × 16) at row 92, the circuit's records (`drawRecordByCircuit`: two areas restored, `RECOBAR` (640 × 68) at row 132, the circuit's name in big A centred on x 413 at y 136, six rows from car 5 down to car 0 at y 208 + 22r: car name at x 228, driver upper-cased at x 360, time `MM:SS.CC` at x 514), the circuit's snapshot (`TRSNAP2M`, 128 × 98) at (40, 214), arrows (`TRARR1`) at (24, 228) and (168, 228), a border (15, 204, 178 × 117); wiped in the same way without the music.
- **Left/Right** step through the 18 circuits in `circuitOrder` (0x45673C), with effect 26, the arrow lit for 8 waits; Enter, keypad Enter or Escape leave.
- **Back to the menu:** the main menu is drawn in the second buffer and wiped in with the music mask falling; the menu music restarts at the order it had and the mask is 0x10000.
- Circuit names at 0x44D148 + 15c, car names at 0x4501B0 − 1760 (5 − k).

## 4. Architecture

- **`deadrally-gamedata`:** `text` gains circuit, car and difficulty names and the circuit order; `Assets` the six pictures and the medium font's glyphs; `dr_cfg` the records' and the best ten's fields.
- **`deadrally-core`:** the menu scene's Hall of Fame states; `Sound` gains jumping to an order and restarting the music at one.

## 5. Verification

A scenario through both screens, Left and Right, and back: every shot equals one of our frames (`find`); a recording of it compares as in M1b.

## 6. Tests

Without data: the wipe's tiles and window; the best ten's layout and upper-casing; the records' time format and car order; Left/Right wrapping and the arrow; leaving; the music's mask and jump. With data: the names and pictures load; a manifest of the run.
