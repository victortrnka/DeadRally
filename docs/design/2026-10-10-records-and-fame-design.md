# Race records and a growing Hall of Fame: design

- **Date:** 2026-10-10
- **Status:** approved in conversation by the owner on 2026-10-10; written down here.
- **Scope:** after 1.0.0, beyond the original: records of whole races beside the lap records, and a best ten that keeps every entry.
- **Builds on:** [M7](2026-10-07-m7-polish-design.md) and the lap record fixes of 2026-10-09 (`docs/verification/m7.md`, "Lap records as DeadRally keeps them").

## 1. Goal

A player sees their best whole-race times as well as their best laps, and winning a game never pushes a name out of the Hall of Fame. Nothing the original shows is lost, and the runs checked against the original (`Game::as_the_windows_version`) still play as the original.

## 2. The owner's decisions

1. **Race records by circuit, car and laps.** A circuit can be raced over 4, 5 or 6 laps (the easy, medium and hard races take their circuits from overlapping parts of the order), the Arena over 9. Each count of laps has records of its own, so only races of the same length are compared.
2. **Up and Down on the records screen.** Left and Right still change the circuit. Up and Down step through the kind of record: the lap, then the races of 4, 5 and 6 laps (the Arena: the lap, then 9 laps). The layout stays the original's, with the drivers' names.
3. **The Hall of Fame grows.** Its entries stay sorted by races, fewest first, as in the original. A won game is put before the first entry with more races, and no entry drops out. Ten rows fit on the screen; Up and Down scroll.
4. **Lap records are kept and saved** (2026-10-09): every record is saved as the race that set it ends, and where the original is wrong, DeadRally fixes it.

## 3. Decisions taken on the owner's behalf

1. **What counts as a whole race:**
   - the player's car crosses the finish line on its last lap;
   - an abandoned race, a wrecked car, or a lapped player who finishes a lap short sets no race record.

   The time is the race time the statistics show, which stops when the player's car finishes. *Cost if wrong:* a rule changed in one place.
2. **The last lap after the winner counts.** When another car has finished, the original ends the player's race at the line without looking at the lap just driven (0x412DF0's `laps.over` branch), so a best lap set on the last lap of a race the player did not win is lost. DeadRally times that lap as any other: its best lap, the lap record, and the record's call. *Cost if wrong:* that last lap's call sound when it is a record.
3. **Where the new data lives:** in DeadRally's own `dr.cfg`, after the original's bytes, in this order:
   - the Arena's lap records, 6 × 24 bytes, as 1.0.0 writes them;
   - the race records: for each count of laps (4, 5, 6), for each car, for each of the 18 circuits, 24 bytes as a lap record has them; then the Arena's 9-lap records, one for each car. That makes 330 records, 7920 bytes;
   - the best ten's entries past the tenth: a 32-bit count, then 20 bytes each, as the original's entries.

   A block is written only when it or a later one has something in it. Blocks before it are written empty (zeros), which reads as no record. The first ten entries of the Hall of Fame stay in the original's ten slots, so the original can still read the file. A 1.0.0 file reads as before. *Cost if wrong:* a converter for files written in between.
4. **The records screen's headings:** the title stays the circuit's name. On a race page the third column's heading reads its laps ("4 LAPS:"), drawn over the bar's own "LAP TIME:". A title with the laps ("Hell Mountain - 6 laps") did not fit its box. Checked by eye in the game. *Cost if wrong:* a different heading.
5. **The Hall of Fame's screens:**
   - The best ten from the main menu starts at the top. Up and Down move it a row while there are rows beyond the screen; any other key goes on as now.
   - The entry after a won game shows the ten rows ending with the new one when it is below the tenth, with the border round it; a key ends it as now.
   - Ranks past 99 move left to fit.
   - There is no limit to the entries.

   *Cost if wrong:* scrolling added to the entry screen later.
6. **The Windows version plays as the original:**
   - the best ten keeps ten entries and drops the last;
   - there are no race records, no Up and Down on the records, and no Arena page;
   - the last lap after the winner is not timed.

## 4. Tests

- **`dr.cfg`:**
  - the race records and the extra entries are written and read back;
  - a 1.0.0 file and an original file read as before;
  - an empty block is written only before a filled one.
- **Race records:**
  - a whole race keeps its time in the record of its circuit, car and laps, and `dr.cfg` is handed over as the race ends;
  - an abandoned race and a lapped finish keep none;
  - the Arena's go to its 9-lap records.
- **The last lap after the winner** is timed: a race where the player finishes second with their best lap last.
- **The Hall of Fame:**
  - a won game with more races than the tenth entry still enters, as the eleventh, and nobody drops out;
  - Up and Down scroll;
  - the entry screen shows the new row.
- **The screens** are checked by eye in the game, then pinned with manifests of DeadRally's own runs.
- **The runs checked against the original** keep their manifests unchanged.
