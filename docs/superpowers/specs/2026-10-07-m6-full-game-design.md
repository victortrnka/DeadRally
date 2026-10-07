# M6 — The full game: design

- **Date:** 2026-10-07
- **Status:** written without the owner, who asked to go on milestone after milestone; section 2 lists the decisions taken on their behalf; `docs/verification/m6.md` records the checks against the original.
- **Scope:** milestone M6 of the brief: the campaign to its end. A player who leads every other driver on points meets the Adversary: the results' turn to the leader, the Adversary's screen, the final race in the Arena, and the end with its animation and the Hall of Fame. Music, effects and the gamepad, which the brief also lists, are in already (M1b, M2b, M4c, M5); what this milestone finds missing of them is added on the way.
- **Builds on:** [M5](2026-10-06-m5-race-rest-design.md).

## 1. Goal

A game goes on from the first race to the last as the original's does: the results that make the player the leader show the Adversary's animation, the shop leads to the Adversary instead of the sign-up, the race in the Arena is the player against the Adversary, and winning it ends the game with the end animation and the Hall of Fame.

## 2. Decisions taken on the owner's behalf

1. **M6 comes in parts:** M6a the Adversary's screen and its Escape (the other races' results); M6b the race in the Arena and its results; M6c the results' turn to the leader with the Adversary's animation; M6d the end: the end animation, the Hall of Fame entry and the way back to the main menu. *Cost if wrong:* none; the order of work only.
2. **The facts come from the binary**; DreeRally and dRally are hints. *Cost if wrong:* none.
3. **Reference runs reach the end from crafted saves** (a test game whose player leads, or nearly), as M3b–M5 crafted theirs. *Cost if wrong:* none; the saves are test inputs, never committed.

## 3. Facts about the original (decoded so far)

- **The way to the Adversary:** with the player ahead of every other driver on points, the Underground Market's way on (0x4366A0) and the shop's continue without weapons (0x438607) open the Adversary's screen (0x435320) instead of the sign-up.
- **The Adversary's screen** (0x435320): the copper entries at 100 %, the menus' background with three pictures and a border wiped in (0x42C4A0), then up to 480 waits for Enter or Escape. Enter (or the waits running out) starts the race in the Arena: race 3, two cars (`previewRaceScreen(2)`), the player's record on both places of the grid. Escape fades out, fills every race with the others, draws four places for the cars and shows `postRaceMain(1)`; the sabotage is then on sale again unless the player leads.
- **After a race, the leader** (0x434728): the next races drawn and filled at once, four places drawn; a won race in the Arena (the player's place 1) leads to the end (0x4312D0), any other to the results.
- **The results' turn to the leader** (0x463DF8, set when the results make the player the leader for the first time): the way out fades all but entries 96 to 127, then plays `endani0.haf` with `tr0-mus.cmf` and `endani0e.cmf` (`openAnimation` 0x4185B0, already ported for the startup), the menu music back.
- **The end** (0x4312D0): `endani.haf` with `tr0-mus.cmf` and `endani-e.cmf`, the shop's pictures loaded again, the Hall of Fame entry (0x427300), 0x430FA0, the palette composed and faded in with the music, the menu music back.

## 4. Verification

Each part has a scenario from a crafted test game, screenshots every 200–500 ms matched with `find`, and its manifest; the race in the Arena also its state in the original's memory frame by frame (`--watch`).
