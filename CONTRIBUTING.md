# Contributing to DeadRally

Thank you for helping. Read [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) first; this file covers the practical side.

## Rules

1. **Faithfulness first.** Gameplay must match the original before anything is improved; improvements are options that default to the original behaviour.
2. **No game data in the repository**, ever: no BPA or HAF files, no executables or DLLs, no saves, sound, music, or screenshots of original art. CI rejects tracked files that match `.gitignore`.
3. **Know where code comes from.** Our code is GPL-3.0-or-later. Facts and formats from [DreeRally](https://github.com/victortrnka/DreeRally/tree/0.4.x) and [dRally](https://github.com/urxp/dRally) are welcome with credit; copied dRally code keeps its MIT notice; do not paste decompiled DreeRally code.
4. **Evidence:** every gameplay change comes with a parity log, a side-by-side screenshot, or a reference to the original's code.

## You need the original game

DeadRally ships no game data. Install *Death Rally (Classic)* from Steam (free, appid 358270), or use Remedy's 2009 freeware release. On Linux or macOS you can fetch the Windows files with steamcmd:

```
steamcmd +@sSteamCmdForcePlatformType windows +force_install_dir ~/games/DeathRally +login <steam-user> +app_update 358270 validate +quit
```

Tell DeadRally where the data is. The first of these that is set wins:

1. `--data <dir>` on the command line;
2. the `DEADRALLY_DATA` environment variable;
3. `data_path = "<dir>"` in `config.toml` in your config directory (on Linux `~/.config/deadrally/config.toml`; `check-data` prints the path on every system).

DeadRally keeps the original's settings, records and Hall of Fame in its own `dr.cfg` next to `config.toml`. When it has none, it reads the game folder's `dr.cfg` once, if there is one; it never writes into the game folder.

`<dir>` may be the folder holding `ENGINE.BPA` or Steam's `Death Rally` folder above it. Check your setup:

```
cargo run -p deadrally-headless -- check-data
```

Exit status 0 means a known release, 2 an unknown release (usable, but parity checks may differ), 1 unusable.

## Setting up

- **Rust:** install [rustup](https://rustup.rs). The toolchain version is pinned in `rust-toolchain.toml` and installs itself.
- **Linux (Debian, Ubuntu, Mint):** `scripts/install-linux-deps.sh`; add `--local` for Xvfb, the screenshot tools and Wine (for reference runs of the original).
- **macOS:** Xcode command line tools (`xcode-select --install`) and CMake (`brew install cmake`).
- **Windows:** Visual Studio Build Tools with the "Desktop development with C++" workload, and CMake.

## Build, run, test

```
cargo build --workspace
cargo run --release -p deadrally -- -window     # the game: intro, logos, title, with sound
cargo test --workspace
DEADRALLY_DATA=~/games/DeathRally cargo test-data
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

In the game: `-window` starts windowed, `-testscene` shows the M0 test scene instead (no game data needed), `--data <dir>` names the data directory; Alt+Enter toggles fullscreen, F12 toggles smoothing.

## Looking at the pictures

```
cargo run --release -p deadrally-headless -- dump-assets    # every image as PNG under dumps/
```

`dumps/` is ignored by Git. Never commit what is in it.

## Listening to the sound

```
cargo build --release -p deadrally-headless
target/release/deadrally-headless render-audio --startup --seconds 100 --out captures/startup.wav
target/release/deadrally-headless render-audio --music MEN-MUS --seconds 60 --out captures/menu.wav
target/release/deadrally-headless render-audio --effect SANIM-E --number 29 --out captures/effect.wav
```

The files are 48 kHz WAVs of the original's music and effects: keep them under `captures/`, which Git ignores.

## Checking against the original

The original `dr.exe` is the reference. On Linux, `scripts/reference-run.sh` runs it under Wine on a virtual display (no window appears, nothing reaches the speakers), presses keys and takes screenshots as a scenario file says:

```
scripts/reference-run.sh scripts/reference/startup.scenario captures/startup
cargo build --release -p deadrally-headless
target/release/deadrally-headless find captures/startup/*.png
```

`find` reports, for each screenshot, the ticks of DeadRally's startup sequence that show exactly the same picture. `docs/verification/m1a.md` lists the scenarios and what they must show. Screenshots stay under `captures/`, which Git ignores: they show the original's art.

With `--sound`, the runner also records what the original plays, from a PulseAudio null sink, and stops if the game's sound is not on that sink. `compare-audio` then says whether our render sounds the same:

```
scripts/reference-run.sh --sound scripts/reference/startup-sound.scenario captures/startup-sound
target/release/deadrally-headless render-audio --startup --seconds 122 --out captures/startup-sound/ours.wav
target/release/deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115
```

`docs/verification/m1b.md` has the scenarios and the numbers they gave.

`find` and `render-audio` press keys with `--key-at TICK:KEY` (space when no key is named), so they follow a scenario through the menus; `docs/verification/m2a.md` says how to read the ticks off a run's `run.log`.

CI does not run `cargo test-data`, because GitHub has no game data. Run it yourself when you touch data code.

## Builds from CI

GitHub artifacts lose the executable bit, so on Linux and macOS make the binary runnable first. CI builds are not signed, so macOS also blocks them until you remove the quarantine flag:

```
chmod +x deadrally
xattr -d com.apple.quarantine deadrally     # macOS only
```

## Commits and pull requests

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`; subject at most 50 characters, imperative mood.
- A body only when several things changed, as a `- ` list.
- One topic per pull request; CI must be green.
