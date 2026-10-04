# DeadRally

Original Death Rally reincarnation for modern systems: a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) for Windows, macOS and Linux, written in Rust.

**Status:** M1a, game data and pictures. The game starts like the original, without sound: intro, Apogee and Remedy logos, title screen. Nothing is playable yet. [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) covers the goal, the approach and the roadmap.

The repository contains no game data. You need your own copy of the game: Death Rally (Classic) on Steam (free) or Remedy's 2009 freeware release.

## Quick start

```
scripts/install-linux-deps.sh               # Linux; see CONTRIBUTING.md for macOS and Windows
export DEADRALLY_DATA=~/games/DeathRally    # your copy of the game
cargo run -p deadrally-headless -- check-data
cargo run --release -p deadrally -- -window
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the full setup, the data configuration and the rules.

Licence: GPL-3.0-or-later, see [LICENSE](LICENSE).
