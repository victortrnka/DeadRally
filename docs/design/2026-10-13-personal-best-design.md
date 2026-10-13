# Personal best: design

- **Date:** 2026-10-13
- **Status:** approved in conversation by the owner on 2026-10-13 (the game menu, the Arena included); written down here.
- **Scope:** beyond the original, after [race records and a growing Hall of Fame](2026-10-10-records-and-fame-design.md): the player's own best times in one game, to look at.

## 1. Goal

While playing a game, the player can look at their own best lap and best whole race on every circuit with every car, kept with the saved game. Nothing is called, and nothing compares the times with anyone else's.

## 2. The owner's decisions

1. **A row in the game menu.** "Personal best" comes under "See current statistics", and like it, it can be chosen only while a game is on.
2. **The Arena too.** Its lap and its 9-lap race are kept as the circuits' are.
3. **Only the player's times, without calls.** A time is kept when it is better than the player's own, whatever the records are.

## 3. Decisions taken on the owner's behalf

1. **What is kept:**
   - for each of the 18 circuits and each car, the best lap and the best whole race over 4, 5 and 6 laps;
   - for the Arena and each car, the best lap and the best whole race over 9 laps.

   A lap counts from any race where the player drove one; a race counts only when driven to its end, as for the race records. The times are kept as the race ends, beside the records. *Cost if wrong:* a rule changed in one place.
2. **With the game:**
   - A new game starts with none, and ending a game drops them.
   - Saving a game (a slot or F2) writes them into its file after the original's 2179 bytes, as `DRPB` and 444 times of 4 bytes in hundredths (0 for none). The original reads only its 2179 bytes, so it still loads the game.
   - Loading a game (a slot or F3) takes them from the file; a game the original saved has none.

   *Cost if wrong:* a converter for files written in between.
3. **The screen:**
   - It wipes in from the menu as the Hall of Fame does, and back to the menu on Enter or Escape.
   - Its title is "PERSONAL BEST". Left and Right step through the circuits in the Hall of Fame's order, then the Arena.
   - The circuit's name is in the records' title box. Under it is a table of the six cars, with columns for the lap and each count of laps (the Arena's: the lap and 9 laps).
   - A time never driven shows as "--:--.--".
   - There is no snapshot or driver's name: it is always the player.

   Checked by eye in the game. *Cost if wrong:* the layout changed.
4. **The menu's seventh row.** The menu's box grows a row and moves up a row, so that it keeps clear of the bottom panel. The rows' actions follow the new order. *Cost if wrong:* the row moved.
5. **The Windows version plays as the original:** six rows in the menu, no personal best, and saved games of the original's 2179 bytes. The runs checked against the original keep their manifests.

## 4. Tests

- **The data:** the block written and read back, its times by circuit, laps and car, and a file of the original's length with none.
- **Keeping times:** a better lap and a better whole race are kept, a worse one or a race not driven to its end is not, and records do not matter.
- **Saving and loading:** with a game, a new game, ending a game, and a game the original saved.
- **The menu:** the row is there as DeadRally, chosen only with a game on, and the other rows still do what they did.
- **The screen:** checked by eye, then pinned with a manifest of DeadRally's own run.
- **The runs checked against the original** keep their manifests unchanged.
