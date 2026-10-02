# M0 — Repository foundations: design

- **Date:** 2026-10-03
- **Status:** agreed in brainstorming with the owner; this written spec awaits review.
- **Scope:** milestone M0 of [PROJECT_BRIEF.md](../../PROJECT_BRIEF.md) §7.
- **Supersedes:** the C++20/CMake/ASan parts of the brief (§5, the M0 row in §7, §10 step 4, §12). M0 edits the brief to match (section 13).

## 1. Goal

Lay the foundation that every later milestone builds on:

- a Rust workspace whose deterministic game core has no platform dependencies;
- a platform frontend chosen by evidence from a spike of two candidates;
- a locator and validator for the player's original game data;
- CI on Windows, macOS and Linux;
- the docs and rules that keep the project faithful to the original.

M0 contains **no game logic and no asset decoding**. The core runs a throwaway test scene that exercises the platform contract.

## 2. Owner decisions

| Decision | Choice | Why |
|---|---|---|
| Language | **Rust** (edition 2024) | No implicit integer conversions: the signedness, `sar`/`shr` and int/float traps of brief §8 become explicit in the types. Safe Rust removes the heap overruns that DreeRally hit. Cargo replaces CMake and dependency management on all three OSes. The compiler catches more agent mistakes. The brief's approach is "specify from the oracle, then write fresh", so being far from C matters little. |
| Platform layer | **Spike two frontends, keep the winner** (section 7) | The owner prefers a pure-Rust stack; SDL3 is the safer bet. Evidence decides; a tie goes to pure Rust. |
| Oracle tooling on Linux | **Separate sub-project B**, in the DreeRally repo | DreeRally's tooling is macOS-specific (clang-cl + xwin + CrossOver). Porting it to Linux and Wine is real work in another repo. It is needed before the first parity check in M1, not in M0. |
| Steam library auto-detection | **Moved to M7** | Developers use an environment variable. Players benefit only once installers exist. |

The brief's sub-project order is therefore: **A = M0** (this spec), **B = oracle on Linux** (DreeRally build, Wine runs of the original and of DreeRally under Xvfb, scripted keys, screenshots, orighook), then M1, M2 and so on. Each one gets its own spec, plan and implementation.

## 3. Scope

**In M0**

1. Cargo workspace, pinned toolchain, GPL-3.0 metadata, `.gitignore`.
2. `deadrally-core`: deterministic core skeleton (fixed 1/70 s tick, frame and audio output, input events) running the test scene.
3. `deadrally-gamedata`: locating and validating the game data, plus config file reading.
4. `deadrally-headless`: determinism runner and `check-data` command.
5. Platform spike: `front-sdl` and `front-rust`, ADR 0001, loser deleted, winner renamed `deadrally`.
6. CI: fmt, clippy, tests, release builds as artifacts, core purity check, cross-OS determinism check.
7. Docs: `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, brief update, `docs/adr/`.
8. Local development environment on the owner's Linux machine (Linux Mint 22.3).

**Not in M0:** BPA/BPK/HAF/XM reading (M1), the oracle (sub-project B), saves and their import (M3), the settings UI and writing the config (M7), Steam auto-detection (M7), installers (M7), name and branding (before any public release), milestone issues on GitHub (proposed after M0).

## 4. Workspace layout

```
Cargo.toml              [workspace], resolver 3, [workspace.package], [workspace.lints]
Cargo.lock              committed (the workspace ships binaries)
rust-toolchain.toml     pinned stable (latest stable when M0 starts), components rustfmt + clippy
.cargo/config.toml      alias test-data
.gitignore
crates/
  core/                 deadrally-core      lib (with its own clippy.toml, section 5)
  gamedata/             deadrally-gamedata  lib
  headless/             deadrally-headless  bin
  front-sdl/            deadrally-front-sdl bin "deadrally-sdl"   (spike)
  front-rust/           deadrally-front-rust bin "deadrally-rust" (spike)
scripts/
  install-linux-deps.sh system packages for building the frontends (CI and local)
docs/
  PROJECT_BRIEF.md
  adr/0001-platform-layer.md
  superpowers/specs/…
```

After the spike, the winning frontend moves to `crates/deadrally/`: package `deadrally`, binary `deadrally`.

Dependency rules:

| Crate | May depend on |
|---|---|
| `deadrally-core` | nothing in M0 (it gains `deadrally-gamedata` in M1) |
| `deadrally-gamedata` | small pure-Rust crates: `sha2`, `serde`, `toml`, `directories` |
| `deadrally-headless` | `deadrally-core`, `deadrally-gamedata`, `sha2` |
| frontends | `deadrally-core` and their platform crates |

Frontends do not touch game data in M0, because the test scene needs none.

## 5. Core contract

```rust
pub const TICKS_PER_SECOND: u32 = 70;
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;
pub const AUDIO_FRAMES_PER_TICK: usize = 630; // 44_100 / 70, exact

pub struct Game { /* private */ }

impl Game {
    pub fn new() -> Game;                              // M1 adds game data as a parameter
    pub fn input(&mut self, event: InputEvent);
    pub fn tick(&mut self);                            // advances exactly 1/70 s
    pub fn frame(&self) -> Frame<'_>;
    pub fn take_audio(&mut self, out: &mut Vec<i16>);  // appends the samples produced since the last call
}

pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],              // indexed, tightly packed, len = width * height
    pub palette: &'a [[u8; 3]; 256],   // 6-bit VGA values, 0..=63
    pub aspect: (u32, u32),            // display aspect ratio, e.g. (4, 3) or (16, 9)
}

impl Frame<'_> {
    pub fn write_rgba(&self, out: &mut [u8]);  // the single palette-to-RGBA conversion all frontends use
}

pub enum InputEvent {
    Key { key: Key, pressed: bool },          // physical key position, not a character
    PadButton { button: PadButton, pressed: bool },
    PadAxis { axis: PadAxis, value: i16 },
}
pub enum PadButton { A, B, X, Y }             // "one stick, four buttons" (brief §1)
pub enum PadAxis { StickX, StickY }
```

- **`Key`** is the project's own enum of physical keys: letters, digits, F1–F12, arrows, Enter, Escape, Space, Backspace, Tab, the modifiers and the keypad. Each frontend maps its scancodes to it. M2 adds the mapping to the original's scancodes and text entry.
- **Audio** is interleaved stereo `i16`, so each tick produces 1260 values.
- **The frontend owns wall-clock time.** It calls `tick()` once for every 1/70 s that has elapsed and catches up at most 5 ticks per presented frame; ticks dropped beyond that are counted and logged. Then it presents the latest `frame()`. The core never reads a clock.
- **The frontend handles Alt+Enter and F12 itself** (they are presentation concerns) and does not forward them.
- **Frame sizes and aspect** are whatever the core reports. The frontend scales to the window or desktop, keeps the aspect ratio and adds black bars. This keeps widescreen (M8+) a core-only change.
- **The 6-bit to 8-bit conversion** in M0 is `(v << 2) | (v >> 4)`. M2 pins the exact formula against the oracle.

### Determinism rules (enforced)

- `deadrally-core` declares `#![forbid(unsafe_code)]`. The workspace sets `unsafe_code = "forbid"`. A frontend that truly needs `unsafe` gets a crate-level exception with a comment explaining why.
- `crates/core/clippy.toml` disallows:
  - **methods:** `std::time::Instant::now`, `std::time::SystemTime::now`, `std::thread::spawn`, `std::env::var`, `std::env::var_os`, and the transcendental float methods (`sin`, `cos`, `tan`, `powf`, `exp`, `ln`, `log*`, `atan*`, `sinh`/`cosh`/`tanh`) on `f32` and `f64`. Their results differ between platforms' libm; later milestones use our own or table-based versions.
  - **types:** `std::collections::HashMap`, `std::collections::HashSet`, `std::time::Instant`, `std::time::SystemTime`.
- `overflow-checks = true` in every profile, release included. Intentional wrap-around is written explicitly (`wrapping_*`). This is revisited in M7.
- The test scene uses integer arithmetic only.

### Test scene (throwaway, removed when M2 brings real menus)

Lives in `crates/core/src/test_scene.rs`. It shows:

- **a background ramp** with palette index = `(x + y) & 0xFF`, and the palette rotates by one entry every tick, which exercises per-tick palette changes;
- **a vertical bar** moving one pixel per tick, which shows judder and timing;
- **a grid of cells**, one per `Key` and `PadButton`, lit while held, plus a dot for the stick position;
- **three frame modes**, cycled with Tab: 640×480 (4:3), 320×200 (shown 4:3) and 640×360 (16:9).

Audio: a quiet 440 Hz triangle wave from an integer phase accumulator, toggled with T and on by default, plus a 5 ms click on every key or button press.

## 6. Game data

### Locating

Sources in precedence order:

1. `--data <path>` on the command line
2. the `DEADRALLY_DATA` environment variable
3. `data_path` in the config file

**The first source that is specified is used. Failure does not fall through:** an invalid `--data` is an error even if the environment variable is valid. If none is specified, the error names all three sources and the config file location.

If the chosen directory lacks the files but contains a `Death Rally` subdirectory, that subdirectory is used. Steam nests the data like this: `steamapps/common/Death Rally/Death Rally/`.

### Config file

- **Format:** TOML at `config.toml` in the platform config directory given by `directories::ProjectDirs::from("", "", "DeadRally")`. `check-data` prints the exact path.
- **Contents in M0:** a single key, `data_path`. DeadRally only reads it; the owner writes it by hand.
- **Problems:**
  - A missing config file is not an error.
  - A malformed file is: the error carries the parse location.
  - An unknown key is a warning.

### Validation

**Required files** (brief §9): `ENGINE.BPA`, `IBFILES.BPA`, `MENU.BPA`, `MUSICS.BPA`, `TR0.BPA`…`TR9.BPA`, `SANIM.HAF`, `ENDANI.HAF`, `ENDANI0.HAF`, `END.BMP`, `RMD.BMP`. Names match **case-insensitively** by listing the directory. Originals mix cases (`SANIM.haf`, `end.bmp`) and Linux file systems are case-sensitive. If two entries differ only in case, that is an error. `dr.exe` is not required; only the oracle needs it.

**Outcomes:**

| Outcome | Condition | Behaviour |
|---|---|---|
| Known version | every file present and every SHA-256 matches one known version | OK; report the version name |
| Unknown version | every file present, at least one hash unknown | loud warning naming the differing files ("unknown version, may behave differently"); usable |
| Unusable | a file is missing or unreadable | error naming each missing file and the directory searched |

All files are hashed on every validation, about 45 MB and 0.1–0.3 s. Data files are only ever opened read-only, and nothing is ever written into the data directory. Config, and saves from M3 on, live in DeadRally's own user directories.

**Known versions** live in `crates/gamedata/src/known_versions.rs`. Hashes are not game data and may be committed. M0 ships one version:

**Steam, Death Rally (Classic), appid 358270** (downloaded 2026-10-03 with steamcmd, Windows depot):

| File (as on disk) | Size | SHA-256 |
|---|---:|---|
| `ENGINE.BPA` | 387193 | `8ff2aff5b4c5d10a1ada9c7529b95bded949d4914f917e6dc6e2fe5448cf14c2` |
| `IBFILES.BPA` | 85989 | `93f08b317df159aeb229d6ba86bb1c6f40ba9d28fdc10dd81e67639348dc4c74` |
| `MENU.BPA` | 3168933 | `b0993e984066a7e5e7edcfca69476624872a7f322648b0b4961f3c05339f15b1` |
| `MUSICS.BPA` | 5726559 | `9f1e094f2a8763683027614fd97cb8afce5836c158f0d1b25be2842bf334754b` |
| `TR0.BPA` | 337303 | `368fa56187b8e1ca480e7ec1a6f5aad655d259e8b4bc398bc08131eaed4dd45f` |
| `TR1.BPA` | 321394 | `efa9cd8bec6c240675ef6bfe333638442436c4229cbb9c74772257b8683d4305` |
| `TR2.BPA` | 294365 | `7f242b788f920e4b00f254f30e49bc98536d7cc58513056b456236a0f4821130` |
| `TR3.BPA` | 345851 | `d1d7b5db7d6b3529d7c953557c3a76e7570283f9833a306447d9417a95db7b9f` |
| `TR4.BPA` | 260466 | `238be60f379ea282647ed33157a087b519854cdb821d192878f78f9ef9b0af0d` |
| `TR5.BPA` | 431992 | `ab0767d3acf7c1f45bdfb3124a4206cfe3cadb650602a399d33a05b521e5a9dd` |
| `TR6.BPA` | 359368 | `757587f57fe6711b6353a054e654dedcc5b2553d82476c310912e3ac6339e6c2` |
| `TR7.BPA` | 417431 | `c11dc15b3e8f5e2708d0c6e0cca73ac72ce9d8e88a533e11a3a9c006608805c2` |
| `TR8.BPA` | 253202 | `c7bfb5142cbc7c95dd42b9c40b298d2e3b3af9bb9d445b6c441369e6dfa24036` |
| `TR9.BPA` | 427555 | `3c6336f43d7188fc1f1d4d53307ceea43f64342f9262f9a298e28834b5e36912` |
| `SANIM.haf` | 21516352 | `4fbb589fe50f8aa7e47ae41245c64578a9112ea843be2089178a08333087c993` |
| `ENDANI.haf` | 3796331 | `13eef7520ae0a5180484a32a4a31efcbb7438f050c7e96f158cc59fb67e2526f` |
| `ENDANI0.HAF` | 7193340 | `cd20359a766a6ee64137869f65caa91ab6b166dc314ef1ef0561943f7b54a362` |
| `end.bmp` | 308148 | `8723a70d39ca89a092765ce3813b7f34bb6168bdc96e7d635013bcd60bb6d190` |
| `rmd.bmp` | 308278 | `af3cdbcecfeb40aaf9952b19149fadbe40cc46641db90cdd30d12d88598dd972` |

For reference only, not validated: `dr.exe` is 365952 bytes with SHA-256 `54fe789faca583d67b8e73e7c58908f3f1468c5c8f75942239a60483ae9be58c`. Remedy's 2009 freeware release is added as a second known version once someone hashes a copy. Until then it validates as "unknown version".

## 7. Platform spike

### Candidates

- **`front-sdl`:** the `sdl3` crate with SDL3 built from source and linked statically. The SDL renderer presents the frame as a streaming texture with logical presentation (letterbox) and nearest or linear scaling.
- **`front-rust`:** winit for the window and events, wgpu for GPU presentation (through `pixels` if it is maintained and compatible with current winit, otherwise wgpu directly with one textured quad and a nearest or linear sampler), cpal for audio and gilrs for the gamepad. GPU presentation is chosen so the scaling comparison with SDL is fair.

Both frontends implement the same loop around the core (section 5) and the same audio policy:

- **Pacing:** the wall clock paces the ticks.
- **Audio queue:** the frontend queues each tick's samples to the device, aiming for about 3 ticks (≈43 ms) in the queue. When more than 8 ticks are queued, it discards queued audio down to 3 ticks.
- **Logging:** queue depth, underruns and dropped ticks are logged once per second.
- **Determinism:** the trimming happens only in the frontend, so the core's audio stays deterministic.

### Must-pass checklist (both frontends, all items)

1. **Scaling:** scales to the desktop resolution with the correct aspect for every test-scene mode, with black bars. Nearest by default; F12 toggles bilinear.
2. **Window modes:** starts as borderless fullscreen at the desktop resolution; `-window` starts windowed; Alt+Enter toggles at runtime.
3. **Input:** keyboard press and release by physical key reach the core. Gamepad stick and four buttons reach the core, including hot-plug and unplug.
4. **Audio:** 5 minutes of the test tone with zero underruns after the first second, and a queue depth that stays bounded (no drift).
5. **Timing:**
   - over 60 s the core advances 70 ticks/s ± 0.1 % on average;
   - after a stall (for example, dragging the window), catch-up is capped as specified.
6. **CI:** `cargo build --release` succeeds on ubuntu-latest, macos-latest and windows-latest. The only extra setup allowed is system packages (`scripts/install-linux-deps.sh` via apt); no manual SDK downloads or custom toolchains.

A must-item that can only be met by patching a dependency counts as failed.

### Measured for comparison

- frontend lines of code (excluding blank lines and comments)
- crates in `cargo tree` (deduplicated)
- clean release build time on CI Linux
- release binary size per OS
- required system packages
- average and 99th-percentile present time at 3840×2160 with bilinear on

### Who verifies what

| Item | Verified by |
|---|---|
| windowed rendering | Claude: screenshots under Xvfb |
| timing, underrun and queue logs | Claude |
| CI builds | Claude |
| fullscreen, Alt+Enter, F12, gamepad, listening | owner: this Linux machine at its monitor (X11) and the owner's Mac |
| Windows | owner, if a Windows machine is available; otherwise recorded as **not manually verified** |
| Wayland | recorded as **not manually verified** unless tested |

### Decision rule

- A frontend that fails any must-item loses.
- If both pass, the measurements decide; a tie goes to `front-rust`.
- If neither passes, stop and discuss with the owner.

The result, measurements and verification status go into `docs/adr/0001-platform-layer.md` (context, options, measurements, decision, consequences). The losing crate is deleted in its own commit, so it stays in history. The winner is renamed (section 4).

## 8. Headless runner

```
deadrally-headless run --ticks N
    prints: ticks=N frames_sha256=<hex> audio_sha256=<hex>
deadrally-headless check-data [--data PATH]
    prints: which source was used, the resolved directory, the config file path, the outcome and the per-file details
    exit status: 0 known version, 2 unknown version (usable), 1 unusable or error
```

`frames_sha256` hashes, for every tick in order, the frame's width, height and both aspect terms (each as little-endian `u32`), then the 768 palette bytes, then the pixels. `audio_sha256` hashes every sample as little-endian `i16`. The runner feeds no input in M0; scripted input arrives with the first parity work (M1/M2).

## 9. CI (GitHub Actions, `.github/workflows/ci.yml`)

| Job | Runner(s) | Does |
|---|---|---|
| `lint` | ubuntu-latest | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; the tracked-game-data check (section 12) |
| `test` | ubuntu-latest, macos-latest, windows-latest | `cargo test --workspace --locked`; `cargo build --workspace --release --locked`; `deadrally-headless run --ticks 7000` → upload hash; upload frontend release binaries as artifacts |
| `macos-intel` | macos-latest | `cargo build --workspace --release --locked --target x86_64-apple-darwin`; upload binaries |
| `core-purity` | ubuntu-latest | fails if `cargo tree -p deadrally-core -e normal` mentions `sdl`, `winit`, `wgpu`, `pixels`, `cpal` or `gilrs` |
| `determinism` | ubuntu-latest, after `test` | fails unless the three uploaded hashes are identical |

- **Caching:** `Swatinem/rust-cache`.
- **Toolchain:** installed from `rust-toolchain.toml`.
- **Linux packages:** come from `scripts/install-linux-deps.sh`, the same script used locally.
- **Unsigned macOS artifacts:** Gatekeeper blocks them; CONTRIBUTING documents `xattr -d com.apple.quarantine <binary>`.
- **No self-hosted runner in M0:** on a public repository, a fork's pull request could run code on the owner's machine, which also hosts game servers.

## 10. Testing policy

- **Tests without game data** run everywhere, including CI.
- **Tests that need game data:**
  - They are marked `#[ignore = "needs game data (DEADRALLY_DATA)"]`. `#[ignore]` is reserved for this purpose.
  - They run with `cargo test-data` (alias for `cargo test --workspace -- --ignored`).
  - They read the data directory through one helper. If `DEADRALLY_DATA` is unset, they **fail** with a message saying so; they never pass vacuously.
  - CI does not run them. This is stated in CONTRIBUTING, and Claude runs them locally whenever touching data code.
- **Tests encode why** (owner rule 9). The name or comment states the consequence the test protects against. For example: a missing `TR5.BPA` must fail validation and be named, otherwise the game would crash mid-campaign on track 5.
- **Fixtures** for gamedata tests are generated in temporary directories by the tests themselves (files with the right names and wrong contents, missing files, case variants, case collisions). Fixture files are never committed.
- **Required M0 tests:**
  - **gamedata, without data:** each outcome of section 6, precedence without fall-through, the nested `Death Rally` fallback, case-insensitive matching, case collision, malformed and missing config.
  - **gamedata, with data:** the real install validates as Steam 358270.
  - **core:**
    - two `Game`s run for the same number of ticks produce identical frames and audio;
    - `take_audio` yields exactly 1260 values per tick;
    - `write_rgba` maps 0 to 0 and 63 to 255;
    - frame mode cycling reports the documented sizes and aspects.
  - **headless:** `run` output is stable across two invocations.

## 11. Lints, style, toolchain

- `rustfmt` with the default configuration.
- Clippy at its default lint set, with warnings denied in CI.
- `[workspace.lints]` sets `rust.unsafe_code = "forbid"`. Every crate opts in with `[lints] workspace = true`.
- `rust-toolchain.toml` pins an exact stable version. Toolchain updates are deliberate `build:` commits (section 14).
- Profiles: `overflow-checks = true` for `dev`, `test` and `release`.

## 12. `.gitignore`

The owner asked for a thorough one. It covers:

- **Build output:** `/target/`.
- **Game data and anything derived from it** (brief §2; case-insensitive patterns, because Linux is case-sensitive):
  - `*.[Bb][Pp][Aa]`, `*.[Hh][Aa][Ff]`
  - `[Ee][Nn][Dd].[Bb][Mm][Pp]`, `[Rr][Mm][Dd].[Bb][Mm][Pp]`
  - `[Dd][Rr].[Ss][Gg][0-9]`, `[Dd][Rr].[Cc][Ff][Gg]`
  - `*.[Ee][Xx][Ee]`, `*.[Dd][Ll][Ll]`
- **Generated local output:**
  - `/dumps/` (asset dumps, M1)
  - `/captures/` (screenshots, WAV captures, parity logs)
  - `*.wav`, `*.mp3`, `*.flac`, `*.xm`, `*.s3m`
  - `*.log`
- **Local environment:** `.env`, `.env.*`, `.worktrees/`, `.claude/settings.local.json`.
- **Editors and OS:** `.DS_Store`, `Thumbs.db`, `desktop.ini`, `.idea/`, `.vscode/`, `*.swp`, `*.swo`, `*~`.

`Cargo.lock` is **not** ignored. A CI step (in the `lint` job) fails if `git ls-files` matches any of the game-data patterns, as a second line of defence against `git add -f`.

## 13. Documents

- **`CLAUDE.md`** is for agents. It covers:
  - the ground rules of brief §2: faithfulness first, never commit assets, provenance and licensing, evidence for every gameplay claim;
  - the determinism rules (section 5);
  - the commands: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo test-data`, `deadrally-headless`;
  - the testing policy (section 10);
  - the commit rules (section 14);
  - the brief's agent practices (§11): small briefs, worktrees, "done means verified", re-running key checks.
- **`CONTRIBUTING.md`** is for humans: the same rules, plus how to get the game data, set `DEADRALLY_DATA` or the config, install dependencies, build, run, and the macOS quarantine note.
- **`README.md`** gains a status line, build-and-run instructions and the data setup.
- **`docs/PROJECT_BRIEF.md`** gets these edits:
  - §5 describes the Rust workspace instead of C++/CMake; the directory sketch becomes the crate list with future modules noted.
  - §7 M0 describes the Cargo workspace; "ASan/UBSan" is replaced by "safe Rust + overflow checks".
  - §8's ASan/UBSan advice gets the same replacement.
  - §10 step 4 follows suit.
  - §12 records the decisions taken (language, platform per ADR 0001, sub-project B).
- **`docs/adr/0001-platform-layer.md`** records the spike result.

## 14. Workflow

- **Branch:** M0 work happens on `m0-foundations`. Each plan task is one commit. After an independent review, the branch merges into `master`.
- **Commit subjects:**
  - prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`, matching the existing `docs:` commits;
  - at most 50 characters, imperative mood, English;
  - a body only when several things changed, with `- ` lists;
  - no Co-Authored-By or other attribution.
- **Pushing:** this machine has no GitHub credentials yet. The owner sets them up (an SSH key added to GitHub, or a token) before the first push. Claude pushes only with the owner's consent.

## 15. Local development environment (this machine)

- **Present:** gcc 13, make, pkg-config, python3, wine 9, git.
- **To install:**
  - `rustup` (user-level, from rustup.rs) and the pinned toolchain;
  - `cmake` (builds SDL3 from source);
  - `xvfb` (headless screenshots);
  - the build packages in `scripts/install-linux-deps.sh` (ALSA, udev, X11/Wayland/xkbcommon development headers; the exact list is settled during the spike).
- **Who runs what:** system packages need `sudo`. If it requires a password, the owner runs the script; Claude never handles passwords.
- **Game data:** `~/games/DeathRally/Death Rally` (steamcmd install), used read-only through `DEADRALLY_DATA`.

## 16. Done criteria for M0

1. On the merge commit to `master`, every CI job is green, including `determinism` (identical hashes on three OSes) and `core-purity`.
2. `cargo test` passes locally and in CI. `cargo test-data` passes locally against the real install, which validates as Steam 358270.
3. `deadrally-headless check-data` reports the real install correctly via `--data`, via `DEADRALLY_DATA` and via the config file.
4. Both frontends were evaluated against section 7. ADR 0001 records the measurements, the decision and the per-item verification status, including anything not manually verified. The loser is deleted and the winner renamed `deadrally`.
5. `.gitignore`, `CLAUDE.md`, `CONTRIBUTING.md`, the updated `README.md` and the updated brief are committed.
6. The local environment is set up and documented, and `scripts/install-linux-deps.sh` works on a clean ubuntu-latest runner.

## 17. Risks

| Risk | Mitigation |
|---|---|
| `sdl3` crate bindings are pre-1.0 and their API changes | Pin versions; the frontend is small. |
| `pixels` is unmaintained or lags winit | Fall back to wgpu directly (allowed in section 7). |
| Audio clock drift versus the wall clock | Queue watermarks (section 7); a must-item checks bounded depth over 5 minutes. |
| Fullscreen semantics differ (macOS Spaces, Wayland) | Manual checks; untested platforms are recorded as such in the ADR. |
| Cross-OS float differences surface later | The `determinism` job exists from M0; transcendental functions are banned in core. |
| The owner cannot see this machine's display (remote session) | Xvfb screenshots for Claude; physical checks by the owner per section 7. |
