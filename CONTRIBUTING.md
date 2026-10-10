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

In the game: `-window` starts windowed, `-smooth` and `-nogl` work as the original's (see the README), `-testscene` shows the M0 test scene instead (no game data needed), `--data <dir>` names the data directory; Alt+Enter toggles fullscreen, F12 toggles smoothing.

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

## Determinism (`crates/core`)

- No clocks, threads, environment reads, `HashMap`/`HashSet` or libm transcendental functions: `crates/core/clippy.toml` bans them. Frontends pace ticks with `deadrally_core::host::Pacer`.
- Overflow checks are on in every profile. Write intentional wrap-around as `wrapping_*`.
- `unsafe` is forbidden in the whole workspace.
- `deadrally-core` must not depend on platform crates; CI's `core-purity` job checks it.
- Everything a frontend shares (pacing, audio gate, letterbox, stats) belongs in `deadrally_core::host`, not in a frontend.

## Tests

- Tests encode **why**: the name or a comment says what goes wrong for a player if the behaviour changes.
- `#[ignore]` is only for tests that need game data: `#[ignore = "needs game data (DEADRALLY_DATA)"]`. They read the data through `DEADRALLY_DATA` and fail when it is unset.
- Fixtures are generated by the tests in temporary directories. Never commit files derived from game data; hashes of decoded data are facts and may be committed.
- "Done" means verified. Say which checks ran, and say so when one could not run (CI never runs `cargo test-data`).

## Every command

| Command | What it does |
|---|---|
| `cargo fmt --all` | format |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint; CI denies warnings |
| `cargo test --workspace` | tests that need no game data |
| `DEADRALLY_DATA=~/games/DeathRally cargo test-data` | tests that need the original data; they fail when it is unset |
| `cargo run -p deadrally-headless -- check-data` | where the data was found and whether it is a known release |
| `cargo run --release -p deadrally-headless -- run --ticks 7000` | determinism hashes; CI compares them across OSes |
| `cargo run --release -p deadrally -- -window` | the game: the original's startup sequence with its sound, then the main menu (`-testscene`: the M0 test scene); `-smooth` and `-nogl` as the original's (spec M7) |
| `cargo run --release -p deadrally-headless -- dump-assets` | every catalogued image as PNG under `dumps/` (ignored) |
| `scripts/reference-run.sh scripts/reference/startup.scenario captures/startup` | screenshots of the original under Wine on a virtual display |
| `target/release/deadrally-headless find captures/startup/*.png` | the ticks of our startup sequence that match each screenshot exactly; `--key-at TICK:KEY` presses keys, `--ticks N` runs on into the menus; `--smooth` smooths our picture as the original's `-smooth` (`reference-run.sh --smooth`); `render --deadrally` plays as DeadRally rather than as the Windows version, for DeadRally's own screens |
| `target/release/deadrally-headless render-audio --startup --seconds 100 --out captures/startup.wav` | the startup's sound as the game plays it, the intro and then the menu music; also `--music NAME`, `--effect BANK --number K` |
| `scripts/reference-run.sh --sound scripts/reference/startup-sound.scenario captures/startup-sound` | the original's sound, recorded from a null sink (nothing reaches the speakers); `--cfg FILE` starts it with another `dr.cfg` |
| `target/release/deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115` | does our render sound like the recording; PASS or FAIL against spec M1b §5 |
| `DEADRALLY_BLESS=1 cargo test-data` | rewrite the manifests `crates/gamedata/tests/decoded-images.sha256`, `crates/headless/tests/rendered-audio.sha256`, `menu-run.sha256`, `configure-run.sha256`, `hall-of-fame-run.sha256`, `new-game-run.sha256`, `saved-games-run.sha256`, `shop-purchases-run.sha256`, `market-run.sha256`, `sabotage-run.sha256`, `offer-run.sha256`, `quick-save-run.sha256`, `preview-run.sha256`, `race-start-run.sha256`, `pause-run.sha256`, `abort-run.sha256`, `reversed-run.sha256`, `drive-run.sha256`, `collide-run.sha256`, `pickup-run.sha256`, `pedestrian-run.sha256`, `guns-run.sha256`, `mines-run.sha256`, `rocket-run.sha256`, `wreck-run.sha256`, `spikes-run.sha256`, `help-run.sha256`, `opponents-run.sha256`, `abort-early-run.sha256`, `race-effects-sound.sha256`, `results-run.sha256`, `effect-run.sha256`, `race-keys-run.sha256`, `lap-run.sha256`, `tab-abort-run.sha256`, `statistics-run.sha256`, `cheats-run.sha256`, `no-sign-up-run.sha256`, `damage-calls-sound.sha256`, `pause-early-run.sha256`, `help-early-run.sha256`, `shop-welcome-run.sha256`, `adversary-run.sha256`, `arena-run.sha256`, `arena-won-run.sha256`, `leader-turn-run.sha256` `leader-turn-market-run.sha256`, `tunnel-guns-run.sha256`, `records-all-run.sha256`, `slow-records-run.sha256`, `arena-records-run.sha256`, `fame-grows-run.sha256` and `race-records-run.sha256`, only after checking the pictures and the sound against the original again |
| `scripts/reference-run.sh --seed 1 scripts/reference/new-game.scenario captures/new-game` | the original with `rand()` seeded (its drivers and races repeat); pass the same `--seed` to `find`, `render` and `render-audio`; `--save SLOT:FILE` gives either a saved game (`find`, `render` and `render-audio --startup`); `--sabotage-clock MS` fixes the clock the sabotage reads in both; `--key-at T:KEY+N` holds a key N ticks (scenarios: `keydown`, `keyup`); `--no-ai` keeps the opponents still in a race (spec M4) |
| `scripts/reference-run.sh --no-ai --watch SCENARIO captures/x` then `target/release/deadrally-headless trace --tick T --key-at ... > ours.txt` and `scripts/compare-watch.py captures/x/watch.log ours.txt` | the race's state in the original's memory each frame against ours (spec M4c); `compare-watch.py WATCH --keys` gives the player's keys the original sampled; `trace`, `find`, `render` and `render-audio` take `--no-ai` for runs of the original with it (`trace`, `find`, `render` and `render-audio` take `--cfg FILE` to start from a `dr.cfg`, as `reference-run.sh --cfg` does); the watch log also holds the rocket flames' picture (`fp` in the trace); `--drive PATH:FROM:TO` (which watches too) drives the player's car along PATH, a file of `x y` lines, from the race's frame FROM to its point TO, holding the arrows as a player would (spec M5); the watch log's `.menus` file has the menus' count of waits, copper row and pulse as they move, and `trace --menus` gives ours (spec M7) |
| `scripts/package.sh linux VERSION dist` and `scripts/smoke-test.sh PROGRAM` | a player's package of the release build (also `windows`, and `macos` from both Mac targets), and its program run on the test scene for ten seconds; `.github/workflows/release.yml` does both on each system (spec M7) |
| `scripts/spike-check.sh screens target/release/deadrally captures/x 10` | screenshots and stats without a monitor (Xvfb; sound to a file) |
| `scripts/fullscreen-check.sh target/release/deadrally captures/fs` | four fullscreen toggles on the real GPU without a monitor (headless Weston) |

## Commits and pull requests

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`; subject at most 50 characters, imperative mood.
- A body only when several things changed, as a `- ` list.
- One topic per pull request; CI must be green.
