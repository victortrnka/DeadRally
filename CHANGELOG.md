# Changelog

## Unreleased

- Race records: the best time of each whole race, by circuit, car and number of laps, on the Hall of Fame's records with Up and Down.
- The Hall of Fame keeps every winner; Up and Down scroll past the tenth.
- The last lap of a player who finishes after the winner counts for the best lap and the lap record.
- Remedy's race records and Arena lap records to beat, from Remedy's lap records.
- "It's a new race record" at the finish line, made of the announcer's own calls; with a lap record on the last lap, the lap's call first, then the race's, then "race over".
- `deadrally-headless render --deadrally` renders the game as DeadRally plays it, not as the Windows version.
- Personal best: each game's own best laps and races, saved with it, after See Current Statistics; the statistics count the records the player holds.
- Up and Down never leave the Hall of Fame's best ten.
- The personal best's pages show the circuit's snapshot between the arrows, as the records do, over the whole screen.
- The Arena's snapshot is made of its own track, at its start line, and its name lettered as the circuits' names are.
- A title font of "TRACK RECORDS"'s own letters, the rest drawn in their style, for the personal best's title.

## 1.0.0 (2026-10-10)

The first release, with the whole single-player game. Each milestone was checked against the original; [docs/verification](docs/verification) has the runs and what still differs.

- **M7, 1.0 polish:**
  - the original's `-smooth` and `-nogl` pictures;
  - options kept in `config.toml`;
  - a folder dialog at the first start;
  - packages for Linux, Windows and macOS;
  - the players' README;
  - the race's last key no longer skips the easy race's results page or the end animation, a bug of the Windows version;
  - the menu's welcome line names one universal version for every system, and the next line credits DeadRally's port;
  - the game asks for fullscreen again once its window exists, for macOS;
  - lap records saved as each race ends, so a closed window no longer loses them;
  - the Arena's own lap records, with their page in the Hall of Fame.
- **M6, the full game:**
  - the Adversary's screen;
  - the race in the Arena;
  - the Adversary's animation after the results that first make the player the leader;
  - the end animation and the Hall of Fame entry;
  - the welcome box in a new game's first race.
- **M5, the race's rest:**
  - the opponents' driving;
  - the results, the money and the statistics;
  - the shop's popups after a race: sponsors, deals, the loan shark, the end of the road;
  - the race's other keys;
  - the effect power-up;
  - the shop's cheat words.
- **M4, the race:**
  - the preview and the start;
  - the driving, the walls, the laps and the knocks;
  - the power-ups, the pedestrians, the guns, the mines, the rocket, the fire and the spikes;
  - the pause, the help and the end of the race.
- **M3, the campaign without racing:**
  - the new game, the licence and the sign-up;
  - the saved games and the shop;
  - the purchases, the Underground Market and the hitman's and the steroids' offers;
  - quick save and load.
- **M2, the menus:** the main menu, Configure (volumes, keys, gamepad), the Hall of Fame and the credits, with `dr.cfg` as the original's.
- **M1, the start:** the game's data, pictures, music and effects; the intro, the logos and the title.
- **M0, the foundations:** the workspace, CI on three systems, and the deterministic core.
