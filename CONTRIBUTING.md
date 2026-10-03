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

`<dir>` may be the folder holding `ENGINE.BPA` or Steam's `Death Rally` folder above it. Check your setup:

```
cargo run -p deadrally-headless -- check-data
```

Exit status 0 means a known release, 2 an unknown release (usable, but parity checks may differ), 1 unusable.

## Setting up

- **Rust:** install [rustup](https://rustup.rs). The toolchain version is pinned in `rust-toolchain.toml` and installs itself.
- **Linux (Debian, Ubuntu, Mint):** `scripts/install-linux-deps.sh`; add `--local` for Xvfb and the screenshot tools.
- **macOS:** Xcode command line tools (`xcode-select --install`) and CMake (`brew install cmake`).
- **Windows:** Visual Studio Build Tools with the "Desktop development with C++" workload, and CMake.

## Build, run, test

```
cargo build --workspace
cargo run --release -p deadrally -- -window     # the game; M0 shows a test scene
cargo test --workspace
DEADRALLY_DATA=~/games/DeathRally cargo test-data
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

In the game: `-window` starts windowed, Alt+Enter toggles fullscreen, F12 toggles smoothing.

CI does not run `cargo test-data`, because GitHub has no game data. Run it yourself when you touch data code.

## Builds from CI on macOS

CI builds are not signed, so macOS blocks them until you remove the quarantine flag:

```
xattr -d com.apple.quarantine deadrally
```

## Commits and pull requests

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`; subject at most 50 characters, imperative mood.
- A body only when several things changed, as a `- ` list.
- One topic per pull request; CI must be green.
