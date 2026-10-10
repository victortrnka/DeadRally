# DeadRally

Original Death Rally reincarnation for modern systems: a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) for Windows, macOS and Linux, written in Rust.

**Status:** the whole single-player game, from the intro to the race against the Adversary and the end, played as the original plays it. Every part was checked against the original, screen by screen and often frame by frame ([docs/verification](https://github.com/victortrnka/DeadRally/blob/master/docs/verification)). Version 1.0.0 is the first release. [The project brief](https://github.com/victortrnka/DeadRally/blob/master/docs/PROJECT_BRIEF.md) covers the goal, the approach and the roadmap.

DeadRally contains no game data. You need your own copy of the game: *Death Rally (Classic)* on Steam (free) or Remedy's 2009 freeware release for Windows. DeadRally reads its files and never changes them.

## Playing

1. **Get the game's files.**
   - Steam: install *Death Rally (Classic)*. On Linux or macOS, `steamcmd` can fetch its Windows files (see [CONTRIBUTING.md](https://github.com/victortrnka/DeadRally/blob/master/CONTRIBUTING.md)).
   - Or unpack Remedy's 2009 freeware release.
2. **Get DeadRally** for your system from the [releases](https://github.com/victortrnka/DeadRally/releases) and unpack it:
   - Windows: `deadrally.exe`. It is not signed, so the first time Windows may warn: choose "More info", then "Run anyway".
   - Linux: `deadrally`.
   - macOS: `DeadRally.app`. It is not signed by a known developer, so macOS refuses it the first time. Then open System Settings, Privacy & Security, and choose "Open Anyway" there (on macOS 14 and older, right-clicking the app and choosing Open also works). Or run `xattr -dr com.apple.quarantine DeadRally.app` once.
3. **Start it.** The first time, it asks for the folder of your copy of the game, the one with `MENU.BPA` in it; Steam's `Death Rally` folder above it works too. It remembers the folder.

### Options

The 2009 version's command-line options work as they did:

| Option | What it does |
|---|---|
| `-window` | starts in a window instead of fullscreen; Alt+Enter switches at any time |
| `-smooth` | smooths the race and the animations when they are scaled up; F12 switches at any time |
| `-nogl` | the original's software picture: 640x480, the race doubled, shown at a whole scale |
| `--data <dir>` | the game's folder, for this start only |
| `-novsync` | does not wait for the screen's refresh (for measuring) |

To keep options for every start, write them into DeadRally's `config.toml`. As in the original, the command line can only switch an option on, so one written there stays on for every start until it is taken out of the file.

```toml
data_path = "/home/me/games/DeathRally"
window = true
smooth = true
nogl = false
vsync = true
```

### Where DeadRally keeps its files

DeadRally keeps `config.toml` in its own folder. Next to it are `dr.cfg`, with the volumes, the keys and the Hall of Fame, and the saved games `DR.SG0` to `DR.SG7`.

| System | Folder |
|---|---|
| Linux | `~/.config/deadrally` |
| macOS | `~/Library/Application Support/DeadRally` |
| Windows | `%APPDATA%\DeadRally\config` |

Where it has no `dr.cfg` or saved game of its own yet, DeadRally reads the game folder's, so a game saved with the original goes on in DeadRally. It never writes into the game's folder.

### Keys

These are the original's keys:
- **Driving:** the arrows drive, Left Shift is the turbo, Ctrl shoots, Left Alt drops a mine and Space sounds the horn. Configure in the main menu changes them and sets up a gamepad (one stick, four buttons).
- **In a race:**
  - F1 shows the info screen and P pauses.
  - TAB hides the status bar; F2 and F3 switch the music and the effects; F4 and F5 the scene's pictures and the shadows.
  - Esc asks whether to abandon the race; Y does.
- **In the shop:** F2 saves the game to the quicksave slot and F3 loads it.

### Where DeadRally differs on purpose

The 2009 Windows version keeps the key that ends a race for what follows it. The Enter that closes the race-over box therefore skips the easy race's results page before it can be read, and ends the end animation at once. Its DOS version did not do this, and DeadRally does not either: each results page waits for a key of its own.

The original saves its lap records only when the player leaves through the main menu's Quit, so closing the window loses them, and it counts the Arena's laps as Suburbia's records. DeadRally saves each record as the race that set it ends, and the Arena has lap records of its own, with a page in the Hall of Fame.

DeadRally runs on every system, so the menu's welcome line names one "Universal Version 1.0" instead of the Windows version, and the line under it credits DeadRally's port instead of the Windows one.

## Building from source

```
scripts/install-linux-deps.sh               # Linux; see CONTRIBUTING.md for macOS and Windows
export DEADRALLY_DATA=~/games/DeathRally    # your copy of the game
cargo run -p deadrally-headless -- check-data
cargo run --release -p deadrally -- -window
```

[CONTRIBUTING.md](https://github.com/victortrnka/DeadRally/blob/master/CONTRIBUTING.md) has the full setup, the data configuration and the rules.

Licence: GPL-3.0-or-later, see [LICENSE](https://github.com/victortrnka/DeadRally/blob/master/LICENSE).
