# Personal best: design

- **Date:** 2026-10-13
- **Status:** approved in conversation by the owner on 2026-10-13 (the Arena included); then changed by the owner the same day: after the statistics rather than a row of its own, and the statistics counting the player's records.
- **Scope:** beyond the original, after [race records and a growing Hall of Fame](2026-10-10-records-and-fame-design.md): the player's own best times in one game, to look at.

## 1. Goal

While playing a game, the player can look at their own best lap and best whole race on every circuit with every car, kept with the saved game. Nothing is called, and nothing compares the times with anyone else's.

## 2. The owner's decisions

1. **After the statistics.** The personal best follows "See Current Statistics" in the Start Racing menu: a key there goes on to it. There is no menu row of its own, so the menu stays the original's.
2. **The Arena too.** Its lap and its 9-lap race are kept as the circuits' are.
3. **Only the player's times, without calls.** A time is kept when it is better than the player's own, whatever the records are.
4. **The records held.** The statistics' first screen counts the lap records and the race records the player holds, in rows of their own: "Lap records" and "Race records", under "Total money".

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
   - On the statistics, Escape goes back to the menu as before; any other key fades them out and the personal best's first page in, as the statistics fade.
   - Enter, Space or Escape fade it out and go back to the menu the way the statistics did.
   - The menu's bottom panel is not drawn there (the owner's choice), so that the page fits: under the circuit's name the records' snapshot of it with its arrows and border, centred, the arrow pressed lit for 8 waits as on the records; the table under them.
   - Its title is "PERSONAL BEST" in DeadRally's title font, a whole printable set made in the style of the records' "TRACK RECORDS" (the owner's request, "1:1"). That title is a picture whose letters lack the P, N, L and B, and the game's other titles are lettered otherwise. So the font takes its T, R, A, C, K, E, O, D and S 1:1: the first of each, the pixels where `RECOTXT` differs from the menus' background under it, the A and C split at their one touching column. The others are drawn in their style on a sheet the owner made with ChatGPT, after earlier versions failed: the big font's letters shrunk (`F-BIG3A`) were too bold, and glyphs pieced together from the originals' parts looked broken. The sheet was drawn on a template: a grid of 6x6 squares, one for each pixel, with the nine letters in it as a grey mask (how bright each pixel is beside its line's strokes), the cap lines marked, and no colour. The sheet the owner returned keeps that grid exactly and is coloured in "TRACK RECORDS"'s own colours, line by line; its nine letters are the originals pixel for pixel. `crates/gamedata/src/title_font.txt` holds its other glyphs: for each pixel on lines 1 to 14, the rank of its colour among the colours "TRACK RECORDS" has on that line, brightest first (`.` for nothing), so they are drawn 1:1 in colours the menus never change. Its F is E without its lower bar. Small letters are the capitals. All the glyphs are on its 16 lines, a column apart, a space 5 wide. The same letter is not alike in all its places in `RECOTXT`, since its letters were smoothed where each fell. So no font of one glyph a letter sets all of it pixel for pixel, but "TRA" and "CK" it does.
   - Left and Right step through the circuits in the Hall of Fame's order, then the Arena.
   - The headings read "LAP TIME:" and "4 LAPS:" and so on, as the records' "LAP TIME:" does, and the records bar's line joins their boxes.
   - The circuit's name is in the records' title box. Under it is a table of the six cars, with columns for the lap and each count of laps (the Arena's: the lap and 9 laps).
   - A time never driven shows as "--:--.--".
   - There is no driver's name: it is always the player.

   Checked by eye in the game. *Cost if wrong:* the layout changed.
4. **Counting the records held:** a record is the player's when its driver is their name as the records keep a name: upper-cased and cut to 11 letters. Remedy's never count. The Arena's lap records and every race record count too. The rows are only on the statistics from the menu; after a race, that place holds the race's own rows. *Cost if wrong:* another way to count.
5. **The Windows version plays as the original:** a key on the statistics goes back to the menu, there are no records rows and no personal best, and saved games are the original's 2179 bytes. The runs checked against the original keep their manifests.

6. **The Arena's snapshot** (the owner's request of 2026-10-13: "a proper photo"). `TRSNAP2M` has none, so DeadRally makes one when it loads the data:
   - the first circuit's frame, with its name's strip cleared by repeating the strip's first eight columns, which hold no letters;
   - in its picture, a race's view (320 x 202) of the Arena's track round the start line, made smaller as the circuits' pictures are: each pixel the average of those under it, then the nearest colour the circuits' pictures use in the menu's palette, so that none pulses;
   - the Arena's name written in the strip by the menu.

   It shows on the Hall of Fame's records and the personal best.

## 4. Tests

- **The data:** the block written and read back, its times by circuit, laps and car, and a file of the original's length with none.
- **Keeping times:** a better lap and a better whole race are kept, a worse one or a race not driven to its end is not, and records do not matter.
- **Saving and loading:** with a game, a new game, ending a game, and a game the original saved.
- **The statistics:** a key goes on to the personal best and Enter comes back to the menu; Escape there, and any key as the Windows version, go back at once.
- **The records held:** counted by the player's name as the records keep it.
- **The screen:** checked by eye, then pinned with a manifest of DeadRally's own run.
- **The runs checked against the original** keep their manifests unchanged.
