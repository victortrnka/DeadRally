# M3b — Saved games and the shop: design

- **Date:** 2026-10-06
- **Status:** written without the owner, who asked to go on milestone after milestone without stopping; section 2 lists the decisions taken on their behalf; [docs/verification/m3b.md](../../verification/m3b.md) records the checks against the original.
- **Scope:** the second part of milestone M3: loading and saving games in `DR.SG0`..`DR.SG7` from the Start Racing menu, and the shop's screen with its six items, the moves between them and the way out. Purchases, the Underground Market, the loan shark and the hitman's offer are M3c.
- **Builds on:** [M3a](2026-10-06-m3a-new-game-design.md).

## 1. Goal

A game saved by the original loads in DeadRally and the other way round; loading shows the original's slots, confirmation and the shop as the original draws them; the shop's items, descriptions, prices and animations match the original pixel for pixel, and a game saved from DeadRally is byte for byte the file the original writes for the same game, name and seed.

## 2. Decisions taken on the owner's behalf

1. **Saved games live with DeadRally's own `dr.cfg`** (the configuration folder); a slot DeadRally has no file for is read from the game folder, so games saved with the original go on, and nothing is ever written there. *Cost if wrong:* none; a player who wants their saves elsewhere moves the files.
2. **Only single-player games load:** a saved game whose player is not driver 19 (none the Windows version writes) counts as an empty slot. *Cost if wrong:* none known.
3. **Until purchases exist (M3c), Enter on a shop item other than "continue" does nothing**, and "continue" leads straight to the sign-up (the Underground Market comes in between in M3c). After the stand-in race the shop wipes in as from the menu; the original's return from a race (its fade-in and its popups) comes with races. *Cost if wrong:* none; M3c and M4 replace them.
4. **The shop's cheat words and the quick save and load keys** follow in M3c with the rank recalculation they need. *Cost if wrong:* none until then.

## 3. Facts about the original

- **Files:** 2179 bytes: a key (`rand() % 255` when saving), then driver index, weapons, difficulty, a 15-byte name and the 20 drivers' 108-byte records; from the second byte on each byte has the key subtracted, 17 × its offset added and is rotated right by its offset modulo 6 (`decryptEntireSavegame` 0x41C910; dRally `drencryption.c` agrees).
- **Slots** are menu 5 (eight rows at (231, 114)): a saved game's name over the empty slot's text, the last row the quicksave's; loading an empty slot sounds effect 29. Loading renames the menus as a new game does, clears the new game's warnings, takes the difficulty into `dr.cfg`, shows "game loaded" (`confirmationPopup` 0x42DC70) and enters the shop. Saving asks for a name (`readKeyboard` at (130, 298), 15 characters, 320 pixels, no face or colour, the slot's name filled in), then writes and confirms.
- **`mainMenu` runs `initDrivers` at start-up** (0x43A062), so the shop's car box starts on car 1 and its loops at frame 0.
- **The shop** (`postLoadedOrLicense` 0x4387D0): rows 96–362 of a copy of the screen restored, the title `SHOPTXT1`, the side panel, the continue item's border, the six items each with its description in the popup at (144, 114), wiped in; each pass two waits, then the selected item's loop turns a frame (car 64, engine 24, tires 12, armour 16 back and forth, repair and continue 23). The descriptions sit in `dr.exe`: the car's and its engine levels' in the car's record, the others in tables of 240 bytes a level. Arrows move between items; Left and Right on the car box show the next car with its arrow lit through four turns of two waits. Escape leaves for the Start Racing menu, wiped in over the background.
- **DreeRally's armour box** compares the armour level with the engine's upgrade count; the original compares it with the armour's (0x420AB2).

## 4. Architecture

- `deadrally-gamedata::save_game`: the file format and the slots' files (own folder first, game folder read-only).
- `deadrally-core::campaign::Driver::{to_bytes, from_bytes}`; `deadrally-core::menu::{slots, shop}`; `Game::set_saved_games` and `Game::take_saved_game` carry the files to and from the host.
- The headless tool takes `--save SLOT:FILE`; the reference runner `--save SLOT:FILE`.

## 5. Verification

A saved game made for the test (seed 1's drivers, the player's fields set) is loaded by the original (`reference-run.sh --save 0:FILE --seed 1`), shown in the shop through every item and both car arrows, then saved into slot 1 under a typed name. Every shot must equal one of our frames, and our slot-1 file must equal the original's byte for byte.

## 6. Tests

Unit tests: the file format both ways and its encryption, the slots' files (own before the game folder's, never written there), the drivers' byte layout. A data test runs the scenario's keys and checks our frames' hashes and the saved file's (manifest `saved-games-run.sha256`).
