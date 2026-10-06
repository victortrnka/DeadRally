# DeadRally: instructions for AI agents

DeadRally is a clean, native reimplementation of *Death Rally* (Remedy, 2009) in Rust. Read `docs/PROJECT_BRIEF.md` for the goal and `docs/superpowers/specs/` for the current design. `CONTRIBUTING.md` has the setup.

## Ground rules (brief §2)

1. **Faithfulness first.** Anything that changes how the game plays (timings, physics, prices, AI) must match the original first. Improvements come later, as options that default to the original behaviour.
2. **Never commit game data:** BPA, HAF, the original exe or DLLs, saves, sound, music, or screenshots that are mostly original art. `.gitignore` and `scripts/check-no-game-data.sh` (run in CI) enforce this. Never `git add -f` such files.
3. **Provenance.** New code is ours (GPL-3.0-or-later). Facts, file formats and constants from DreeRally or dRally are fine: describe them in your own words and credit them. Code copied from dRally (MIT) keeps its notice. Do not paste decompiled DreeRally code; re-implement from understanding. When unsure, ask the owner.
4. **Evidence for every gameplay claim:** a parity log, a side-by-side screenshot, or a reference to the original's code (a DreeRally function with its original address).

## Determinism (`crates/core`)

- No clocks, threads, environment reads, `HashMap`/`HashSet` or libm transcendental functions: `crates/core/clippy.toml` bans them. Frontends pace ticks with `deadrally_core::host::Pacer`.
- Overflow checks are on in every profile. Write intentional wrap-around as `wrapping_*`.
- `unsafe` is forbidden in the whole workspace.
- `deadrally-core` must not depend on platform crates; CI's `core-purity` job checks it.
- Everything a frontend shares (pacing, audio gate, letterbox, stats) belongs in `deadrally_core::host`, not in a frontend.

## Commands

| Command | What it does |
|---|---|
| `cargo fmt --all` | format |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint; CI denies warnings |
| `cargo test --workspace` | tests that need no game data |
| `DEADRALLY_DATA=~/games/DeathRally cargo test-data` | tests that need the original data; they fail when it is unset |
| `cargo run -p deadrally-headless -- check-data` | where the data was found and whether it is a known release |
| `cargo run --release -p deadrally-headless -- run --ticks 7000` | determinism hashes; CI compares them across OSes |
| `cargo run --release -p deadrally -- -window` | the game: the original's startup sequence with its sound, then the main menu (`-testscene`: the M0 test scene) |
| `cargo run --release -p deadrally-headless -- dump-assets` | every catalogued image as PNG under `dumps/` (ignored) |
| `scripts/reference-run.sh scripts/reference/startup.scenario captures/startup` | screenshots of the original under Wine on a virtual display |
| `target/release/deadrally-headless find captures/startup/*.png` | the ticks of our startup sequence that match each screenshot exactly; `--key-at TICK:KEY` presses keys, `--ticks N` runs on into the menus |
| `target/release/deadrally-headless render-audio --startup --seconds 100 --out captures/startup.wav` | the startup's sound as the game plays it, the intro and then the menu music; also `--music NAME`, `--effect BANK --number K` |
| `scripts/reference-run.sh --sound scripts/reference/startup-sound.scenario captures/startup-sound` | the original's sound, recorded from a null sink (nothing reaches the speakers); `--cfg FILE` starts it with another `dr.cfg` |
| `target/release/deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115` | does our render sound like the recording; PASS or FAIL against spec M1b §5 |
| `DEADRALLY_BLESS=1 cargo test-data` | rewrite the manifests `crates/gamedata/tests/decoded-images.sha256`, `crates/headless/tests/rendered-audio.sha256`, `menu-run.sha256`, `configure-run.sha256`, `hall-of-fame-run.sha256`, `new-game-run.sha256`, `saved-games-run.sha256`, `shop-purchases-run.sha256`, `market-run.sha256`, `sabotage-run.sha256`, `offer-run.sha256`, `quick-save-run.sha256`, `preview-run.sha256`, `race-start-run.sha256`, `pause-run.sha256`, `abort-run.sha256`, `reversed-run.sha256`, `drive-run.sha256`, `collide-run.sha256`, `pickup-run.sha256`, `pedestrian-run.sha256`, `guns-run.sha256`, `mines-run.sha256`, `rocket-run.sha256`, `wreck-run.sha256` and `spikes-run.sha256`, only after checking the pictures and the sound against the original again |
| `scripts/reference-run.sh --seed 1 scripts/reference/new-game.scenario captures/new-game` | the original with `rand()` seeded (its drivers and races repeat); pass the same `--seed` to `find`, `render` and `render-audio`; `--save SLOT:FILE` gives either a saved game (`find`, `render` and `render-audio --startup`); `--sabotage-clock MS` fixes the clock the sabotage reads in both; `--key-at T:KEY+N` holds a key N ticks (scenarios: `keydown`, `keyup`); `--no-ai` keeps the opponents still in a race (spec M4) |
| `scripts/reference-run.sh --no-ai --watch SCENARIO captures/x` then `target/release/deadrally-headless trace --tick T --key-at ... > ours.txt` and `scripts/compare-watch.py captures/x/watch.log ours.txt` | the race's state in the original's memory each frame against ours (spec M4c); `compare-watch.py WATCH --keys` gives the player's keys the original sampled |
| `scripts/spike-check.sh screens target/release/deadrally captures/x 10` | screenshots and stats without a monitor (Xvfb; sound to a file) |
| `scripts/fullscreen-check.sh target/release/deadrally captures/fs` | four fullscreen toggles on the real GPU without a monitor (headless Weston) |

## Tests

- Tests encode **why**: the name or a comment says what goes wrong for a player if the behaviour changes.
- `#[ignore]` is only for tests that need game data: `#[ignore = "needs game data (DEADRALLY_DATA)"]`. They read the data through `DEADRALLY_DATA` and fail when it is unset.
- Fixtures are generated by the tests in temporary directories. Never commit files derived from game data; hashes of decoded data are facts and may be committed.
- "Done" means verified. Say which checks ran, and say so when one could not run (CI never runs `cargo test-data`).

## Commits

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`.
- Subject at most 50 characters, imperative, English. A body only when several things changed, as a `- ` list.
- No Co-Authored-By or any other attribution.

## Working as an agent (brief §11)

- Work from a written task: goal, files, evidence required.
- One git worktree per task (under `.worktrees/`, which is ignored). Merge only after an independent review.
- Re-run the key checks yourself before reporting success. Checks that silently did not run are the most common false "done".
