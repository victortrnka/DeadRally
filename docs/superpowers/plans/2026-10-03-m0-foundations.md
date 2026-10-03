# M0 Repository Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build DeadRally's M0 foundation: a Rust workspace with a deterministic, platform-free core running a test scene; a validator for the original game data; a headless runner; CI on three OSes; and an evidence-based choice between an SDL3 and a pure-Rust frontend.

**Architecture:** `deadrally-core` holds all game state and never reads a clock; frontends pace it at 70 ticks/s, present its 8-bit indexed frames and play its samples. `deadrally-gamedata` finds and hashes the player's data. `deadrally-headless` runs the core without a window for determinism hashes and data checks. Two frontends (`front-sdl`, `front-rust`) are built against the same core and checklist; the winner becomes `deadrally`.

**Tech Stack:** Rust 1.99.0 (edition 2024), Cargo workspace; sdl3 0.20 (SDL 3.4.16, static); winit 0.30, pixels 0.17 (wgpu 29), cpal 0.18, gilrs 0.11; sha2 0.11, toml 1.1, directories 6; GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-03-m0-foundations-design.md` (read it first; this plan argues from it).

## Global Constraints

- Toolchain pinned in `rust-toolchain.toml`: `channel = "1.99.0"`, components `rustfmt`, `clippy`. Edition 2024, resolver 3.
- Dependency versions (verified): `sdl3 = { version = "0.20.0", features = ["build-from-source-static"] }`, `winit = "0.30.13"`, `pixels = "0.17.2"`, `cpal = "0.18.2"`, `gilrs = "0.11.2"`, `sha2 = "0.11.0"`, `toml = "1.1.6"`, `directories = "6.0.0"`, dev `tempfile = "3.27.0"`. Newer semver-compatible patch releases resolved by Cargo are fine.
- Workspace licence field `GPL-3.0-only` (the owner may switch it to `GPL-3.0-or-later`; change only the root `Cargo.toml`).
- `[workspace.lints.rust] unsafe_code = "forbid"`; every crate has `[lints] workspace = true`.
- `overflow-checks = true` in `dev`, `test` and `release`; dependencies at `opt-level = 2` in `dev`.
- `deadrally-core`: no platform crates, `#![forbid(unsafe_code)]`, determinism bans in `crates/core/clippy.toml`, integer arithmetic only in the test scene.
- Core constants: `TICKS_PER_SECOND = 70`, `AUDIO_SAMPLE_RATE = 44_100`, `AUDIO_CHANNELS = 2`, `AUDIO_FRAMES_PER_TICK = 630`; audio is interleaved stereo `i16`.
- Data directory sources, in order: `--data`, `DEADRALLY_DATA` (empty = unset), `data_path` in `config.toml` under `directories::ProjectDirs::from("", "", "DeadRally")`. No fall-through. Data files are opened read-only.
- Never commit game data or anything derived from it (BPA, HAF, the exe or DLLs, saves, audio, screenshots of original art). Screenshots and logs go under `captures/` (ignored).
- Window title `"DR"` (as the original).
- Commits: prefix `feat:`/`fix:`/`docs:`/`test:`/`refactor:`/`ci:`/`build:`; subject at most 50 characters, imperative, English; a body only when several things changed, as a `- ` list; no Co-Authored-By or any attribution.
- Talk to the owner in Czech; code, comments, docs and commits are in English.

## Review Focus

Inputs and conditions the spec implies but does not spell out, most likely to bite a player first. Each has a test in the task that owns the code.

1. **`~` in `data_path`** (people write `data_path = "~/games/DeathRally"`; nothing expands it in a file) → the home directory is used. Task 6, `a_leading_tilde_means_the_home_directory`.
2. **`DEADRALLY_DATA` set but empty** (`DEADRALLY_DATA= deadrally` to clear it for one run) → treated as unset, the config file is used. Task 6, `an_empty_environment_variable_counts_as_unset`.
3. **Read-only data** (Steam and package managers install it so) → validates normally. Task 5, `read_only_data_validates`.
4. **A long stall** (laptop sleep, debugger, a clock jump) → no overflow panic, at most 5 catch-up ticks. Task 4, `pacer_survives_absurd_elapsed_times` and `pacer_caps_catch_up_after_a_stall_and_counts_the_rest`.
5. **A minimised window** (some platforms report a 0x0 drawable) → no panic, nothing drawn. Task 4, `letterbox_of_a_minimised_window_is_empty`; both frontends skip drawing an empty viewport.

## Before You Start

- **Provenance:** every Rust file, script and workflow below was compiled, linted and tested on 2026-10-03 on the owner's Linux machine with Rust 1.99.0. Expected outputs (test counts, hashes, sizes) are real. If yours differ, stop and find out why before going on.
- **Where:** `/home/trashcan/DeadRally`, branch `m0-foundations` (it already holds the spec commit). Game data: `~/games/DeathRally` (Steam layout; the data is in its `Death Rally` subfolder).
- **Code blocks:** a block preceded by `<!-- write: PATH -->` is the complete content of `PATH`; `<!-- prepend: PATH -->` goes above the content already in `PATH`.
- **[OWNER] steps** need the owner (credentials, sitting at a monitor, a Mac). Ask in Czech, explain exactly what to do, and wait.
- **Progress:** create one todo per task and tick it off after a clean review (owner's rule).
- **Fail loud:** if a check cannot run, say so in the task report; never report it as passed.

## File Map

| Path | Responsibility | Task |
|---|---|---|
| `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml` | workspace, lints, profiles, toolchain, `test-data` alias | 1 |
| `.gitignore`, `scripts/check-no-game-data.sh` | keep game data and local output out of Git | 1 |
| `scripts/install-linux-deps.sh` | system packages (CI and local) | 1 |
| `crates/core/src/lib.rs` | constants, module wiring | 1–4 |
| `crates/core/src/frame.rs` | `Frame`, palette-to-RGBA conversion | 2 |
| `crates/core/src/input.rs` | `InputEvent`, `Key`, `PadButton`, `PadAxis` | 2 |
| `crates/core/clippy.toml` | determinism bans | 2 |
| `crates/core/src/game.rs`, `crates/core/src/test_scene.rs` | `Game`; the throwaway M0 scene | 3 |
| `crates/core/src/host.rs` | pacer, audio gate, letterbox, stats line | 4 |
| `crates/gamedata/src/known_versions.rs`, `validate.rs` | required files, known release hashes, validation | 5 |
| `crates/gamedata/src/config.rs`, `locate.rs` | config file, source precedence, Steam layout | 6 |
| `crates/headless/src/main.rs` | `run --ticks N`, `check-data` | 7 |
| `.github/workflows/ci.yml` | lint, test matrix, macOS Intel, core purity, determinism | 8 |
| `crates/front-sdl/` | SDL3 frontend candidate | 9 |
| `scripts/spike-check.sh` | Xvfb and null-sink runs: screenshots, soak, present times | 9 |
| `crates/front-rust/` | winit + pixels + cpal + gilrs frontend candidate | 10 |
| `docs/adr/0001-platform-layer.md`, `crates/deadrally/` | spike decision; the winning frontend | 11 |
| `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, `docs/PROJECT_BRIEF.md` | rules and setup for agents and humans | 12 |

---

### Task 1: Workspace, toolchain and ignore rules

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `.gitignore`, `scripts/check-no-game-data.sh`, `scripts/install-linux-deps.sh`, `crates/core/Cargo.toml`, `crates/core/src/lib.rs`

**Interfaces:**
- Produces: the workspace every later task adds a crate to (`members = ["crates/*"]`); the `deadrally-core` crate with `TICKS_PER_SECOND`, `AUDIO_SAMPLE_RATE`, `AUDIO_CHANNELS`, `AUDIO_FRAMES_PER_TICK`; the `cargo test-data` alias; `scripts/install-linux-deps.sh [--local]`; `scripts/check-no-game-data.sh`.

- [ ] **Step 1: Install system packages** (needs `sudo`; on this machine it works without a password. If it asks for one, stop and ask the owner to run the script.)

<!-- write: scripts/install-linux-deps.sh -->
```bash
#!/usr/bin/env bash
# Installs the system packages DeadRally needs on Debian, Ubuntu or Mint. CI and developers use
# this same list. SDL3 is compiled from source (cmake plus the X11/Wayland/audio headers); cpal
# links ALSA and gilrs links udev.
#
#   scripts/install-linux-deps.sh           build dependencies
#   scripts/install-linux-deps.sh --local   also the tools for checking frontends without a
#                                           monitor (Xvfb, screenshots, software Vulkan)
set -euo pipefail

packages=(
    build-essential cmake pkg-config
    libasound2-dev libpulse-dev libpipewire-0.3-dev libudev-dev libdbus-1-dev libibus-1.0-dev
    libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxfixes-dev libxi-dev libxss-dev
    libxtst-dev libxkbcommon-dev libwayland-dev libdecor-0-dev
    libegl-dev libgl-dev libgles-dev libdrm-dev libgbm-dev
)
case "${1:-}" in
    "") ;;
    --local) packages+=(xvfb imagemagick xdotool x11-apps mesa-vulkan-drivers) ;;
    *) echo "usage: $0 [--local]" >&2; exit 1 ;;
esac

sudo apt-get update
sudo apt-get install -y --no-install-recommends "${packages[@]}"
```

Then:

```bash
chmod +x scripts/install-linux-deps.sh
scripts/install-linux-deps.sh --local
```

Expected: apt finishes without errors; `cmake --version` prints 3.28 or newer; `command -v Xvfb import xdotool` prints three paths.

- [ ] **Step 2: Install rustup** (user-level; skip if `rustup --version` already works)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
. "$HOME/.cargo/env"
rustup --version
```

Expected: `rustup 1.2x.x`. The installer adds `~/.cargo/bin` to `~/.profile` and `~/.bashrc`.

- [ ] **Step 3: Write the workspace files**

<!-- write: Cargo.toml -->
```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
version = "0.0.0"
edition = "2024"
rust-version = "1.99"
license = "GPL-3.0-only"
repository = "https://github.com/victortrnka/DeadRally"
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"

[profile.dev]
overflow-checks = true

# Optimise dependencies even in debug builds: unoptimised SHA-256 makes data validation take
# seconds instead of a fraction of one, and the platform crates are slow too.
[profile.dev.package."*"]
opt-level = 2

[profile.test]
overflow-checks = true

[profile.release]
overflow-checks = true
```

<!-- write: rust-toolchain.toml -->
```toml
[toolchain]
channel = "1.99.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

<!-- write: .cargo/config.toml -->
```toml
[alias]
# Runs the tests that need the original game data (DEADRALLY_DATA). See CONTRIBUTING.md.
test-data = "test --workspace -- --ignored"
```

<!-- write: crates/core/Cargo.toml -->
```toml
[package]
name = "deadrally-core"
description = "Deterministic game core of DeadRally: no clocks, no threads, no platform code."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[lints]
workspace = true
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 1/70 s of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

/// Simulation ticks per second. The original runs its logic at 70 Hz and stores lap times in
/// 1/70 s units.
pub const TICKS_PER_SECOND: u32 = 70;

/// Output sample rate in Hz.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 44 100 / 70, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 630;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK * TICKS_PER_SECOND as usize == AUDIO_SAMPLE_RATE as usize);
```

- [ ] **Step 4: Build**

```bash
rustup toolchain install
cargo build --workspace
rustc --version
```

Expected: `rustc 1.99.0 (...)`; the build finishes. The compile-time `assert!` in `lib.rs` would fail the build if `AUDIO_FRAMES_PER_TICK * TICKS_PER_SECOND != AUDIO_SAMPLE_RATE`.

- [ ] **Step 5: Write the ignore rules and the tracked-data check**

<!-- write: .gitignore -->
```gitignore
# Build output
/target/

# Original game data and anything derived from it: never commit (brief §2).
# Patterns are case-insensitive by hand because Git on Linux is case-sensitive.
*.[Bb][Pp][Aa]
*.[Hh][Aa][Ff]
[Ee][Nn][Dd].[Bb][Mm][Pp]
[Rr][Mm][Dd].[Bb][Mm][Pp]
[Dd][Rr].[Ss][Gg][0-9]
[Dd][Rr].[Cc][Ff][Gg]
*.[Ee][Xx][Ee]
*.[Dd][Ll][Ll]

# Generated locally: asset dumps (M1), screenshots, audio captures, parity logs
/dumps/
/captures/
*.wav
*.mp3
*.flac
*.xm
*.s3m
*.log
*.raw

# Local environment
.env
.env.*
.worktrees/
.claude/settings.local.json

# Editors and operating systems
.DS_Store
Thumbs.db
desktop.ini
.idea/
.vscode/
*.swp
*.swo
*~
```

<!-- write: scripts/check-no-game-data.sh -->
```bash
#!/usr/bin/env bash
# Fails if any tracked file matches .gitignore: original game data (brief §2) or local output.
# A second line of defence, because `git add -f` bypasses .gitignore.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
offenders=$(git ls-files --cached --ignored --exclude-standard)
if [ -n "$offenders" ]; then
    echo "error: these tracked files match .gitignore and must not be committed:" >&2
    echo "$offenders" >&2
    exit 1
fi
echo "ok: no tracked file matches .gitignore"
```

```bash
chmod +x scripts/check-no-game-data.sh
```

- [ ] **Step 6: Verify the ignore rules catch data in any letter case, and nothing else**

```bash
mkdir -p probe/x dumps captures
touch ENGINE.BPA probe/x/tr0.bpa SANIM.haf ENDANI0.HAF end.bmp RMD.BMP DR.SG0 dr.cfg dr.exe fmod.DLL \
      dumps/a.png captures/s.png out.wav sdlaudio.raw x.log .env .DS_Store probe/x/foo.bmp
git check-ignore ENGINE.BPA probe/x/tr0.bpa SANIM.haf ENDANI0.HAF end.bmp RMD.BMP DR.SG0 dr.cfg dr.exe \
      fmod.DLL dumps/a.png captures/s.png out.wav sdlaudio.raw x.log .env .DS_Store | wc -l
git check-ignore Cargo.toml probe/x/foo.bmp || echo "not ignored: good"
rm -rf probe dumps captures ENGINE.BPA SANIM.haf ENDANI0.HAF end.bmp RMD.BMP DR.SG0 dr.cfg dr.exe fmod.DLL \
       out.wav sdlaudio.raw x.log .env .DS_Store
```

Expected: `17`, then `not ignored: good` (only `end.bmp`/`rmd.bmp` are data; other BMPs are not).

- [ ] **Step 7: Verify the tracked-data check fails on a force-added data file**

```bash
touch Engine.Bpa && git add -f Engine.Bpa
scripts/check-no-game-data.sh; echo "exit=$?"
git rm -q --cached Engine.Bpa && rm Engine.Bpa
scripts/check-no-game-data.sh; echo "exit=$?"
```

Expected: an error naming `Engine.Bpa` and `exit=1`; then `ok: no tracked file matches .gitignore` and `exit=0`.

- [ ] **Step 8: Commit**

```bash
cargo fmt --all --check
git add Cargo.toml Cargo.lock rust-toolchain.toml .cargo/config.toml .gitignore scripts crates/core
git commit -m "build: set up the Cargo workspace" -m "- pinned toolchain, workspace lints and profiles
- core crate with the timing and audio constants
- .gitignore for game data and local output
- scripts for system packages and tracked data"
```

---

### Task 2: Core frame and input types

**Files:**
- Create: `crates/core/src/frame.rs`, `crates/core/src/input.rs`, `crates/core/clippy.toml`, `crates/core/tests/frame_input.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: Task 1's `deadrally-core` crate.
- Produces: `pub struct Frame<'a> { width: u32, height: u32, pixels: &'a [u8], palette: &'a [[u8; 3]; 256], aspect: (u32, u32) }` with `fn write_rgba(&self, out: &mut [u8])`; `pub fn expand_6bit(component: u8) -> u8`; `pub enum InputEvent { Key { key: Key, pressed: bool }, PadButton { button: PadButton, pressed: bool }, PadAxis { axis: PadAxis, value: i16 } }`; `pub enum PadButton { A, B, X, Y }` with `ALL: [PadButton; 4]`; `pub enum PadAxis { StickX, StickY }`; `pub enum Key` (79 variants) with `ALL: &'static [Key]`, where `Key::ALL[k as usize] == k`.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/core/tests/frame_input.rs -->
```rust
//! The frame and input types frontends build on (spec section 5).

use deadrally_core::{Frame, Key, PadButton, expand_6bit};

#[test]
fn six_bit_palette_extremes_map_to_full_black_and_white() {
    // Fades to black and flashes to white must reach the real extremes on every frontend.
    assert_eq!(expand_6bit(0), 0);
    assert_eq!(expand_6bit(63), 255);
    assert_eq!(expand_6bit(32), 130);
    // The VGA DAC ignores the top two bits.
    assert_eq!(expand_6bit(0x40 | 63), 255);
}

#[test]
fn write_rgba_uses_the_palette_for_every_pixel() {
    let mut palette = [[0u8; 3]; 256];
    palette[1] = [63, 0, 0];
    palette[2] = [0, 63, 32];
    let pixels = [1, 2, 0];
    let frame = Frame {
        width: 3,
        height: 1,
        pixels: &pixels,
        palette: &palette,
        aspect: (4, 3),
    };
    let mut rgba = [0u8; 12];
    frame.write_rgba(&mut rgba);
    assert_eq!(rgba, [255, 0, 0, 255, 0, 255, 130, 255, 0, 0, 0, 255]);
}

#[test]
fn key_all_lists_every_key_in_declaration_order() {
    // The test scene indexes its key grid with `key as usize`.
    assert_eq!(Key::ALL.len(), 79);
    for (index, &key) in Key::ALL.iter().enumerate() {
        assert_eq!(key as usize, index);
    }
    for (index, &button) in PadButton::ALL.iter().enumerate() {
        assert_eq!(button as usize, index);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-core --test frame_input`
Expected: compile error, `unresolved imports deadrally_core::Frame, deadrally_core::Key, ...`.

- [ ] **Step 3: Implement**

<!-- write: crates/core/src/frame.rs -->
```rust
/// The picture the core hands to a frontend: 8-bit indexed pixels, the 6-bit VGA palette they
/// index into, and the aspect ratio the picture must be shown at.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    /// Indexed pixels, row-major and tightly packed: `pixels.len() == width * height`.
    pub pixels: &'a [u8],
    /// 256 RGB entries with 6-bit components (0..=63), as on VGA hardware.
    pub palette: &'a [[u8; 3]; 256],
    /// Display aspect ratio as (width, height). 320x200 is shown at 4:3, so its pixels are not
    /// square; frontends must scale to this ratio, not to the pixel dimensions.
    pub aspect: (u32, u32),
}

impl Frame<'_> {
    /// Converts the frame to RGBA8 (4 bytes per pixel, alpha 255). Every frontend uses this one
    /// conversion so colours are identical everywhere.
    ///
    /// # Panics
    ///
    /// If `out.len() != width * height * 4`.
    pub fn write_rgba(&self, out: &mut [u8]) {
        assert_eq!(
            out.len(),
            self.pixels.len() * 4,
            "RGBA buffer has the wrong size"
        );
        let lut: [[u8; 4]; 256] = std::array::from_fn(|i| {
            let [r, g, b] = self.palette[i];
            [expand_6bit(r), expand_6bit(g), expand_6bit(b), 255]
        });
        for (rgba, &index) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.pixels) {
            *rgba = lut[usize::from(index)];
        }
    }
}

/// Expands a 6-bit VGA colour component to 8 bits, so 0 maps to 0 and 63 to 255. The top two
/// bits are ignored, as the VGA DAC ignores them. M2 pins the original's exact formula against
/// the oracle.
#[must_use]
pub fn expand_6bit(component: u8) -> u8 {
    let v = component & 0x3F;
    (v << 2) | (v >> 4)
}
```

<!-- write: crates/core/src/input.rs -->
```rust
/// Something the player did. Frontends forward state changes only (no OS key repeat) and keep
/// presentation keys (Alt+Enter, F12) to themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Key {
        key: Key,
        pressed: bool,
    },
    PadButton {
        button: PadButton,
        pressed: bool,
    },
    /// Stick position from -32768 (left or up) to 32767 (right or down).
    PadAxis {
        axis: PadAxis,
        value: i16,
    },
}

/// The four face buttons of "one stick, four buttons" (brief §1), named by position with Xbox
/// labels: A bottom, B right, X left, Y top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PadButton {
    A,
    B,
    X,
    Y,
}

impl PadButton {
    /// Every button; `PadButton::ALL[b as usize] == b`.
    pub const ALL: [PadButton; 4] = [PadButton::A, PadButton::B, PadButton::X, PadButton::Y];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PadAxis {
    StickX,
    StickY,
}

macro_rules! keys {
    ($($key:ident),+ $(,)?) => {
        /// A physical key position named after the US layout, independent of the active
        /// keyboard layout. M2 adds the mapping to the original's scancodes.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Key {
            $($key),+
        }

        impl Key {
            /// Every key in declaration order; `Key::ALL[k as usize] == k`.
            pub const ALL: &'static [Key] = &[$(Key::$key),+];
        }
    };
}

keys![
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Digit0, Digit1,
    Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, F1, F2, F3, F4, F5, F6, F7, F8,
    F9, F10, F11, F12, Up, Down, Left, Right, Enter, Escape, Space, Backspace, Tab, LeftShift,
    RightShift, LeftCtrl, RightCtrl, LeftAlt, RightAlt, Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7,
    Kp8, Kp9, KpPlus, KpMinus, KpMultiply, KpDivide, KpEnter, KpPeriod,
];
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 1/70 s of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod frame;
mod input;

pub use frame::{Frame, expand_6bit};
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Simulation ticks per second. The original runs its logic at 70 Hz and stores lap times in
/// 1/70 s units.
pub const TICKS_PER_SECOND: u32 = 70;

/// Output sample rate in Hz.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 44 100 / 70, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 630;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK * TICKS_PER_SECOND as usize == AUDIO_SAMPLE_RATE as usize);
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p deadrally-core --test frame_input`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Add the determinism bans**

<!-- write: crates/core/clippy.toml -->
```toml
# The core must be deterministic: same inputs, same outputs, on every OS (spec section 5).
disallowed-methods = [
    { path = "std::time::Instant::now", reason = "the core never reads a clock; the frontend paces ticks" },
    { path = "std::time::SystemTime::now", reason = "the core never reads a clock; the frontend paces ticks" },
    { path = "std::thread::spawn", reason = "the core is single-threaded so its results are reproducible" },
    { path = "std::env::var", reason = "game behaviour must not depend on the environment" },
    { path = "std::env::var_os", reason = "game behaviour must not depend on the environment" },
    { path = "f32::sin", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::cos", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::tan", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::sin_cos", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::asin", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::acos", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::atan", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::atan2", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::sinh", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::cosh", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::tanh", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::powf", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::powi", reason = "not correctly rounded; results may differ between platforms" },
    { path = "f32::exp", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::exp2", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::exp_m1", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::ln", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::ln_1p", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::log", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::log2", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::log10", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::cbrt", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f32::hypot", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::sin", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::cos", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::tan", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::sin_cos", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::asin", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::acos", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::atan", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::atan2", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::sinh", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::cosh", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::tanh", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::powf", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::powi", reason = "not correctly rounded; results may differ between platforms" },
    { path = "f64::exp", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::exp2", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::exp_m1", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::ln", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::ln_1p", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::log", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::log2", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::log10", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::cbrt", reason = "libm differs between platforms; use our own or table-based versions" },
    { path = "f64::hypot", reason = "libm differs between platforms; use our own or table-based versions" },
]
disallowed-types = [
    { path = "std::collections::HashMap", reason = "iteration order is random; use BTreeMap or an array" },
    { path = "std::collections::HashSet", reason = "iteration order is random; use BTreeSet or an array" },
    { path = "std::time::Instant", reason = "the core never reads a clock; the frontend paces ticks" },
    { path = "std::time::SystemTime", reason = "the core never reads a clock; the frontend paces ticks" },
]
```

- [ ] **Step 6: Prove the bans fire, then remove the probe**

```bash
cp crates/core/src/lib.rs target/lib.rs.bak
cat >> crates/core/src/lib.rs <<'EOF'
pub fn probe(x: f32) -> f32 { let _t = std::time::Instant::now(); let _m: std::collections::HashMap<u8, u8> = Default::default(); x.sin() + (x as f64).powf(2.0) as f32 }
EOF
cargo clippy -p deadrally-core -- -D warnings 2>&1 | grep -E "^error: use of a disallowed" | sort
cp target/lib.rs.bak crates/core/src/lib.rs
cargo clippy -p deadrally-core --all-targets -- -D warnings
```

Expected: five lines, `use of a disallowed method f32::sin`, `... f64::powf`, `... std::time::Instant::now`, `use of a disallowed type std::collections::HashMap`, `... std::time::Instant`; then a clean clippy run.

- [ ] **Step 7: Commit**

```bash
cargo fmt --all
git add crates/core
git commit -m "feat: add core frame and input types" -m "- Frame with the shared palette-to-RGBA conversion
- InputEvent, Key, PadButton, PadAxis
- clippy bans on clocks, threads, hash maps and libm"
```

---

### Task 3: Game and the test scene

**Files:**
- Create: `crates/core/src/game.rs`, `crates/core/src/test_scene.rs`, `crates/core/tests/game.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: Task 2's types and the Task 1 constants.
- Produces: `pub struct Game` with `fn new() -> Game` (and `Default`), `fn input(&mut self, event: InputEvent)`, `fn tick(&mut self)`, `fn frame(&self) -> Frame<'_>`, `fn take_audio(&mut self, out: &mut Vec<i16>)` (appends 1260 values per tick). The scene: Tab cycles 640x480 (4:3) → 320x200 (4:3) → 640x360 (16:9); T toggles the 440 Hz tone (on at start); every press clicks; palette entries 0..=3 are pinned.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/core/tests/game.rs -->
```rust
//! The core's contract with frontends and the parity harness (spec section 5).

use deadrally_core::{
    AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, AUDIO_SAMPLE_RATE, Game, InputEvent, Key, PadAxis,
    PadButton, TICKS_PER_SECOND,
};

fn press(game: &mut Game, key: Key) {
    game.input(InputEvent::Key { key, pressed: true });
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

/// Every frame's pixels and palette, plus all audio.
type Recording = (Vec<(Vec<u8>, Vec<[u8; 3]>)>, Vec<i16>);

/// Runs a scripted session from a fresh game.
fn record(script: &[(u32, InputEvent)], ticks: u32) -> Recording {
    let mut game = Game::new();
    let mut frames = Vec::new();
    let mut audio = Vec::new();
    for tick in 0..ticks {
        for &(at, event) in script {
            if at == tick {
                game.input(event);
            }
        }
        game.tick();
        let frame = game.frame();
        frames.push((frame.pixels.to_vec(), frame.palette.to_vec()));
        game.take_audio(&mut audio);
    }
    (frames, audio)
}

#[test]
fn same_inputs_give_identical_frames_and_audio() {
    // Parity testing compares runs tick by tick; any hidden state (clock, hash order,
    // uninitialised data) would make two identical runs differ.
    let script = [
        (
            3,
            InputEvent::Key {
                key: Key::A,
                pressed: true,
            },
        ),
        (
            5,
            InputEvent::PadAxis {
                axis: PadAxis::StickX,
                value: -20_000,
            },
        ),
        (
            9,
            InputEvent::Key {
                key: Key::Tab,
                pressed: true,
            },
        ),
        (
            10,
            InputEvent::Key {
                key: Key::Tab,
                pressed: false,
            },
        ),
        (
            12,
            InputEvent::PadButton {
                button: PadButton::Y,
                pressed: true,
            },
        ),
        (
            20,
            InputEvent::Key {
                key: Key::A,
                pressed: false,
            },
        ),
    ];
    assert_eq!(record(&script, 300), record(&script, 300));
}

#[test]
fn each_tick_produces_exactly_one_tick_of_stereo_audio() {
    // Frontends and the WAV capture assume 630 frames per tick; 70 ticks must be exactly one
    // second at 44.1 kHz or music drifts against the picture.
    let mut game = Game::new();
    let mut audio = Vec::new();
    game.tick();
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
    for _ in 1..TICKS_PER_SECOND {
        game.tick();
    }
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), AUDIO_SAMPLE_RATE as usize * AUDIO_CHANNELS);
}

#[test]
fn take_audio_returns_each_sample_only_once() {
    // A frontend that queued the same samples twice would play an echo.
    let mut game = Game::new();
    game.tick();
    let mut first = Vec::new();
    game.take_audio(&mut first);
    let mut second = Vec::new();
    game.take_audio(&mut second);
    assert!(second.is_empty());
}

#[test]
fn tab_cycles_the_documented_frame_modes() {
    // The spike checks scaling for each of these sizes and aspects.
    let mut game = Game::new();
    let mut seen = Vec::new();
    for _ in 0..4 {
        let frame = game.frame();
        // Read right after the switch, before any tick: size and pixels must agree.
        assert_eq!(frame.pixels.len(), (frame.width * frame.height) as usize);
        seen.push((frame.width, frame.height, frame.aspect));
        press(&mut game, Key::Tab);
    }
    assert_eq!(
        seen,
        [
            (640, 480, (4, 3)),
            (320, 200, (4, 3)),
            (640, 360, (16, 9)),
            (640, 480, (4, 3))
        ]
    );
}

#[test]
fn palette_changes_every_tick() {
    // Palette effects (fades, flashes) are per tick in the original; frontends must re-upload.
    let mut game = Game::new();
    game.tick();
    let before = *game.frame().palette;
    game.tick();
    assert_ne!(before, *game.frame().palette);
}

#[test]
fn a_held_key_is_visible_and_releasing_it_restores_the_picture() {
    // The manual input check relies on this feedback.
    let mut idle = Game::new();
    let mut pressed = Game::new();
    pressed.input(InputEvent::Key {
        key: Key::Space,
        pressed: true,
    });
    idle.tick();
    pressed.tick();
    assert_ne!(idle.frame().pixels, pressed.frame().pixels);

    pressed.input(InputEvent::Key {
        key: Key::Space,
        pressed: false,
    });
    idle.tick();
    pressed.tick();
    assert_eq!(idle.frame().pixels, pressed.frame().pixels);
}

#[test]
fn a_press_clicks_and_t_silences_the_tone() {
    // The latency check listens for the click; T lets the tester hear the click alone.
    let mut game = Game::new();
    press(&mut game, Key::T);
    let mut audio = Vec::new();
    game.tick();
    game.take_audio(&mut audio);
    assert!(audio.iter().any(|&s| s != 0), "pressing T clicks");

    audio.clear();
    game.tick();
    game.take_audio(&mut audio);
    assert!(
        audio.iter().all(|&s| s == 0),
        "tone off and click over: silence"
    );

    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    audio.clear();
    game.tick();
    game.take_audio(&mut audio);
    assert!(audio.iter().any(|&s| s != 0), "a pad button clicks too");
}

#[test]
fn a_repeated_press_without_release_does_not_click_again() {
    // Frontends should not forward OS key repeat; if one does, it must not machine-gun clicks.
    let mut game = Game::new();
    press(&mut game, Key::T);
    game.tick();
    game.tick();
    game.input(InputEvent::Key {
        key: Key::A,
        pressed: true,
    });
    game.tick();
    game.input(InputEvent::Key {
        key: Key::A,
        pressed: true,
    });
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    game.tick();
    audio.clear();
    game.take_audio(&mut audio);
    assert!(audio.iter().all(|&s| s == 0));
}

#[test]
fn stick_extremes_stay_on_screen_in_every_mode() {
    // The dot is drawn with unsigned coordinates; full deflection must not underflow.
    let mut game = Game::new();
    for _ in 0..3 {
        for (x, y) in [
            (i16::MIN, i16::MIN),
            (i16::MAX, i16::MAX),
            (i16::MIN, i16::MAX),
        ] {
            game.input(InputEvent::PadAxis {
                axis: PadAxis::StickX,
                value: x,
            });
            game.input(InputEvent::PadAxis {
                axis: PadAxis::StickY,
                value: y,
            });
            game.tick();
        }
        press(&mut game, Key::Tab);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-core --test game`
Expected: compile error, `unresolved import deadrally_core::Game`.

- [ ] **Step 3: Implement the scene and the game**

<!-- write: crates/core/src/test_scene.rs -->
```rust
//! Throwaway test scene for M0. It exercises the frame, palette, input and audio contract so
//! the platform spike has something to show and play. Removed when M2 brings the real menus.

use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent, Key, PadButton};

// Palette entries 0..=3 are pinned so the bar, the grid and the stick dot do not cycle with
// the ramp.
const WHITE: u8 = 1;
const GREY: u8 = 2;
const RED: u8 = 3;
const PINNED: [[u8; 3]; 4] = [[0, 0, 0], [63, 63, 63], [16, 16, 16], [63, 0, 0]];

const BAR_WIDTH: u32 = 4;
const GRID_COLUMNS: u32 = 16;

/// 440 Hz in 32-bit phase units per sample: 440 * 2^32 / 44 100, rounded.
const TONE_STEP: u32 = 42_852_281;
const TONE_AMPLITUDE: i32 = 2_048;
/// The click is 5 ms (221 frames at 44.1 kHz) of square wave at about 1 kHz.
const CLICK_FRAMES: u32 = 221;
const CLICK_HALF_PERIOD: u32 = 22;
const CLICK_AMPLITUDE: i32 = 8_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Vga640x480,
    Vga320x200,
    Wide640x360,
}

impl Mode {
    fn size(self) -> (u32, u32) {
        match self {
            Mode::Vga640x480 => (640, 480),
            Mode::Vga320x200 => (320, 200),
            Mode::Wide640x360 => (640, 360),
        }
    }

    fn aspect(self) -> (u32, u32) {
        match self {
            Mode::Vga640x480 | Mode::Vga320x200 => (4, 3),
            Mode::Wide640x360 => (16, 9),
        }
    }

    fn next(self) -> Mode {
        match self {
            Mode::Vga640x480 => Mode::Vga320x200,
            Mode::Vga320x200 => Mode::Wide640x360,
            Mode::Wide640x360 => Mode::Vga640x480,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TestScene {
    tick: u64,
    mode: Mode,
    palette: [[u8; 3]; 256],
    pixels: Vec<u8>,
    held_keys: Vec<bool>,
    held_buttons: [bool; 4],
    stick: [i16; 2],
    tone_on: bool,
    tone_phase: u32,
    click_frames_left: u32,
    audio: Vec<i16>,
}

impl TestScene {
    pub(crate) fn new() -> TestScene {
        let mut scene = TestScene {
            tick: 0,
            mode: Mode::Vga640x480,
            palette: [[0; 3]; 256],
            pixels: Vec::new(),
            held_keys: vec![false; Key::ALL.len()],
            held_buttons: [false; 4],
            stick: [0; 2],
            tone_on: true,
            tone_phase: 0,
            click_frames_left: 0,
            audio: Vec::new(),
        };
        scene.set_mode(Mode::Vga640x480);
        scene.update_palette();
        scene
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed } => {
                let was_held = std::mem::replace(&mut self.held_keys[key as usize], pressed);
                if pressed && !was_held {
                    self.click_frames_left = CLICK_FRAMES;
                    match key {
                        Key::Tab => self.set_mode(self.mode.next()),
                        Key::T => self.tone_on = !self.tone_on,
                        _ => {}
                    }
                }
            }
            InputEvent::PadButton { button, pressed } => {
                let was_held = std::mem::replace(&mut self.held_buttons[button as usize], pressed);
                if pressed && !was_held {
                    self.click_frames_left = CLICK_FRAMES;
                }
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[axis as usize] = value;
            }
        }
    }

    pub(crate) fn tick(&mut self) {
        self.tick += 1;
        self.update_palette();
        self.render();
        self.mix_audio();
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        let (width, height) = self.mode.size();
        Frame {
            width,
            height,
            pixels: &self.pixels,
            palette: &self.palette,
            aspect: self.mode.aspect(),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// Switches mode and redraws at once, so `frame()` never pairs the new size with old
    /// pixels.
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        let (width, height) = mode.size();
        self.pixels = vec![0; (width * height) as usize];
        self.render();
    }

    /// Rotates the ramp by one entry per tick, then re-pins the fixed colours.
    fn update_palette(&mut self) {
        let shift = (self.tick % 256) as usize;
        for (index, entry) in self.palette.iter_mut().enumerate() {
            *entry = ramp_color(((index + shift) % 256) as u8);
        }
        self.palette[..PINNED.len()].copy_from_slice(&PINNED);
    }

    fn render(&mut self) {
        let (width, height) = self.mode.size();
        for (y, row) in self.pixels.chunks_exact_mut(width as usize).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = ((x + y) & 0xFF) as u8;
            }
        }

        let bar_x = (self.tick % u64::from(width)) as u32;
        self.fill_rect(bar_x, 0, BAR_WIDTH, height, WHITE);

        let cell_width = width / 20;
        let cell_height = height / 16;
        let key_count = Key::ALL.len();
        for cell in 0..key_count + PadButton::ALL.len() {
            let held = if cell < key_count {
                self.held_keys[cell]
            } else {
                self.held_buttons[cell - key_count]
            };
            let column = cell as u32 % GRID_COLUMNS;
            let row = cell as u32 / GRID_COLUMNS;
            self.fill_rect(
                cell_width * (2 + column),
                cell_height * (2 + row),
                cell_width - 1,
                cell_height - 1,
                if held { WHITE } else { GREY },
            );
        }

        let radius = i32::try_from(height / 8).expect("frame height fits i32");
        let dot = (cell_height / 2).max(2);
        let centre_x = i32::try_from(width / 2).expect("frame width fits i32");
        let centre_y = i32::try_from(height * 3 / 4).expect("frame height fits i32");
        let offset_x = i32::from(self.stick[0]) * radius / 32_768;
        let offset_y = i32::from(self.stick[1]) * radius / 32_768;
        let half_dot = i32::try_from(dot / 2).expect("dot size fits i32");
        let dot_x = u32::try_from(centre_x + offset_x - half_dot).expect("dot stays on screen");
        let dot_y = u32::try_from(centre_y + offset_y - half_dot).expect("dot stays on screen");
        self.fill_rect(dot_x, dot_y, dot, dot, RED);
    }

    /// Fills a rectangle, clipped to the frame.
    fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: u8) {
        let (frame_width, frame_height) = self.mode.size();
        if x >= frame_width || y >= frame_height {
            return;
        }
        let x_end = (x + width).min(frame_width);
        let y_end = (y + height).min(frame_height);
        for row in y..y_end {
            let start = (row * frame_width + x) as usize;
            let end = (row * frame_width + x_end) as usize;
            self.pixels[start..end].fill(color);
        }
    }

    fn mix_audio(&mut self) {
        for _ in 0..AUDIO_FRAMES_PER_TICK {
            let mut sample = 0;
            if self.tone_on {
                sample += triangle(self.tone_phase) * TONE_AMPLITUDE / 32_768;
            }
            self.tone_phase = self.tone_phase.wrapping_add(TONE_STEP);
            if self.click_frames_left > 0 {
                let elapsed = CLICK_FRAMES - self.click_frames_left;
                sample += if (elapsed / CLICK_HALF_PERIOD).is_multiple_of(2) {
                    CLICK_AMPLITUDE
                } else {
                    -CLICK_AMPLITUDE
                };
                self.click_frames_left -= 1;
            }
            let sample = sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            self.audio.extend_from_slice(&[sample, sample]);
        }
    }
}

/// A full-scale triangle wave (-32768..=32767) from a 32-bit phase.
fn triangle(phase: u32) -> i32 {
    let p = (phase >> 16) as i32;
    if p < 32_768 {
        p * 2 - 32_768
    } else {
        (65_535 - p) * 2 - 32_767
    }
}

fn ramp_color(index: u8) -> [u8; 3] {
    [index / 4, index % 64, 63 - index / 4]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_spans_full_scale_without_overflow() {
        // The mixer multiplies this by the amplitude in i32; out-of-range values would clip.
        assert_eq!(triangle(0), -32_768);
        assert_eq!(triangle(0x7FFF_0000), 32_766);
        assert_eq!(triangle(0x8000_0000), 32_767);
        assert_eq!(triangle(u32::MAX), -32_767);
    }

    #[test]
    fn ramp_colours_stay_within_six_bits() {
        // VGA palettes are 6-bit; a value above 63 would be silently masked by the DAC.
        for index in 0..=255u8 {
            assert!(ramp_color(index).iter().all(|&c| c <= 63), "index {index}");
        }
    }
}
```

<!-- write: crates/core/src/game.rs -->
```rust
use crate::test_scene::TestScene;
use crate::{Frame, InputEvent};

/// The whole game state. In M0 it runs the throwaway test scene.
#[derive(Debug)]
pub struct Game {
    scene: TestScene,
}

impl Game {
    #[must_use]
    pub fn new() -> Game {
        Game {
            scene: TestScene::new(),
        }
    }

    pub fn input(&mut self, event: InputEvent) {
        self.scene.input(event);
    }

    /// Advances the simulation by exactly 1/70 s.
    pub fn tick(&mut self) {
        self.scene.tick();
    }

    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        self.scene.frame()
    }

    /// Appends the interleaved stereo samples produced since the last call
    /// (`AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS` per tick).
    pub fn take_audio(&mut self, out: &mut Vec<i16>) {
        self.scene.take_audio(out);
    }
}

impl Default for Game {
    fn default() -> Game {
        Game::new()
    }
}
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 1/70 s of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod frame;
mod game;
mod input;
mod test_scene;

pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Simulation ticks per second. The original runs its logic at 70 Hz and stores lap times in
/// 1/70 s units.
pub const TICKS_PER_SECOND: u32 = 70;

/// Output sample rate in Hz.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 44 100 / 70, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 630;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK * TICKS_PER_SECOND as usize == AUDIO_SAMPLE_RATE as usize);
```

- [ ] **Step 4: Run all core tests**

Run: `cargo test -p deadrally-core`
Expected: unit tests `2 passed` (the scene's own), `frame_input` `3 passed`, `game` `9 passed`.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p deadrally-core --all-targets -- -D warnings
git add crates/core
git commit -m "feat: add the core test scene"
```

---

### Task 4: Shared frontend helpers

**Files:**
- Create: `crates/core/src/host.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: the Task 1 constants.
- Produces (module `deadrally_core::host`): `MAX_CATCH_UP_TICKS = 5`, `AUDIO_TARGET_QUEUE_TICKS = 3`, `AUDIO_MAX_QUEUE_TICKS = 8`; `Pacer::new()`, `Pacer::advance(&mut self, elapsed_nanos: u64) -> u32`, `Pacer::dropped_ticks(&self) -> u64`; `enum AudioDecision { Queue { silence_frames: usize }, Drop }`; `AudioGate::new()`, `AudioGate::decide(&mut self, queued_frames: usize) -> AudioDecision`, `underruns()`, `discarded_ticks()`, `report(&self, queued_frames: usize) -> AudioReport`; `struct Viewport { x, y, width, height: u32 }`; `fn letterbox(output_width: u32, output_height: u32, aspect: (u32, u32)) -> Viewport`; `struct AudioReport { underruns: u64, discarded_ticks: u64, queued_frames: usize }`; `RunStats::new()`, `add_ticks(u32)`, `add_present(micros: u32)`, `line(&self, elapsed_nanos: u64, dropped_ticks: u64, audio: AudioReport) -> String`.

- [ ] **Step 1: Write the failing tests and wire the module**

<!-- write: crates/core/src/host.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_60HZ_NANOS: u64 = 16_666_667;

    #[test]
    fn pacer_runs_exactly_70_ticks_per_second_at_60_hz() {
        // Drift here would desynchronise lap times and music over a long race.
        let mut pacer = Pacer::new();
        let ticks: u32 = (0..36_000).map(|_| pacer.advance(FRAME_60HZ_NANOS)).sum();
        // 36 000 frames of 16 666 667 ns are 600.000012 s: exactly 42 000 ticks.
        assert_eq!(ticks, 42_000);
        assert_eq!(pacer.dropped_ticks(), 0);
    }

    #[test]
    fn pacer_caps_catch_up_after_a_stall_and_counts_the_rest() {
        // After a one-second stall the game must not fast-forward 70 ticks in one frame.
        let mut pacer = Pacer::new();
        assert_eq!(pacer.advance(NANOS_PER_SECOND), MAX_CATCH_UP_TICKS);
        assert_eq!(pacer.dropped_ticks(), 70 - u64::from(MAX_CATCH_UP_TICKS));
    }

    #[test]
    fn pacer_survives_absurd_elapsed_times() {
        // A clock jump (suspend, debugger) must not panic on overflow.
        let mut pacer = Pacer::new();
        assert_eq!(pacer.advance(u64::MAX), MAX_CATCH_UP_TICKS);
        assert_eq!(pacer.advance(0), 0);
    }

    #[test]
    fn audio_gate_primes_an_empty_queue_with_silence() {
        // Queueing one tick at a time into an empty device starves it on the first jitter;
        // starting two ticks ahead keeps the queue near the ~43 ms target.
        let mut gate = AudioGate::new();
        let prime = AudioDecision::Queue {
            silence_frames: 2 * AUDIO_FRAMES_PER_TICK,
        };
        assert_eq!(gate.decide(0), prime);
        assert_eq!(gate.underruns(), 0, "starting is not an underrun");
        assert_eq!(
            gate.decide(AUDIO_FRAMES_PER_TICK),
            AudioDecision::Queue { silence_frames: 0 }
        );
        assert_eq!(gate.decide(0), prime, "after running dry, prime again");
        assert_eq!(gate.underruns(), 1);
    }

    #[test]
    fn audio_gate_drains_to_target_then_queues_again() {
        // Latency must come back down to ~43 ms, not hover just under the maximum.
        let mut gate = AudioGate::new();
        let tick = AUDIO_FRAMES_PER_TICK;
        let queue = AudioDecision::Queue { silence_frames: 0 };
        assert_eq!(gate.decide(AUDIO_MAX_QUEUE_TICKS * tick), queue);
        assert_eq!(
            gate.decide((AUDIO_MAX_QUEUE_TICKS + 1) * tick),
            AudioDecision::Drop
        );
        assert_eq!(gate.decide(5 * tick), AudioDecision::Drop);
        assert_eq!(
            gate.decide((AUDIO_TARGET_QUEUE_TICKS + 1) * tick),
            AudioDecision::Drop
        );
        assert_eq!(gate.decide(AUDIO_TARGET_QUEUE_TICKS * tick), queue);
        assert_eq!(gate.discarded_ticks(), 3);
        assert_eq!(gate.report(7).queued_frames, 7);
    }

    #[test]
    fn letterbox_adds_side_bars_for_4_3_on_16_9() {
        assert_eq!(
            letterbox(1920, 1080, (4, 3)),
            Viewport {
                x: 240,
                y: 0,
                width: 1440,
                height: 1080
            }
        );
    }

    #[test]
    fn letterbox_adds_top_and_bottom_bars_for_16_9_on_4_3() {
        assert_eq!(
            letterbox(1024, 768, (16, 9)),
            Viewport {
                x: 0,
                y: 96,
                width: 1024,
                height: 576
            }
        );
    }

    #[test]
    fn letterbox_of_a_minimised_window_is_empty() {
        // Some platforms report a 0x0 drawable while minimised; that must not panic.
        assert_eq!(
            letterbox(0, 0, (4, 3)),
            Viewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0
            }
        );
    }

    #[test]
    fn stats_line_reports_rate_and_percentiles() {
        let mut stats = RunStats::new();
        stats.add_ticks(70);
        for micros in 1..=100 {
            stats.add_present(micros);
        }
        let audio = AudioReport {
            underruns: 1,
            discarded_ticks: 2,
            queued_frames: 1890,
        };
        assert_eq!(
            stats.line(NANOS_PER_SECOND, 3, audio),
            "t=1.000s ticks=70 rate=70.00/s frames=100 dropped_ticks=3 underruns=1 discarded_ticks=2 queue_ms=42 present_avg_us=50 present_p99_us=99"
        );
    }

    #[test]
    fn stats_line_before_anything_happened_is_all_zero() {
        let line = RunStats::new().line(0, 0, AudioReport::default());
        assert_eq!(
            line,
            "t=0.000s ticks=0 rate=0.00/s frames=0 dropped_ticks=0 underruns=0 discarded_ticks=0 queue_ms=0 present_avg_us=0 present_p99_us=0"
        );
    }
}
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 1/70 s of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod frame;
mod game;
pub mod host;
mod input;
mod test_scene;

pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Simulation ticks per second. The original runs its logic at 70 Hz and stores lap times in
/// 1/70 s units.
pub const TICKS_PER_SECOND: u32 = 70;

/// Output sample rate in Hz.
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 44 100 / 70, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 630;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK * TICKS_PER_SECOND as usize == AUDIO_SAMPLE_RATE as usize);
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-core --lib host`
Expected: compile errors, `cannot find type Pacer`, `cannot find function letterbox`, and so on.

- [ ] **Step 3: Implement above the test module**

<!-- prepend: crates/core/src/host.rs -->
```rust
//! Pure helpers that every frontend shares, so all frontends pace ticks, bound audio latency,
//! letterbox and report in exactly the same way. Nothing here reads a clock: the frontend
//! measures time and passes it in.

use crate::{AUDIO_FRAMES_PER_TICK, AUDIO_SAMPLE_RATE, TICKS_PER_SECOND};

/// The most ticks a frontend runs before presenting a frame. After a stall (a dragged window,
/// a breakpoint, a laptop waking up) the game skips ahead instead of fast-forwarding.
pub const MAX_CATCH_UP_TICKS: u32 = 5;

/// Audio latency the frontend aims for, in ticks (about 43 ms).
pub const AUDIO_TARGET_QUEUE_TICKS: usize = 3;

/// Queued audio above this many ticks is dropped until the queue is back at the target.
pub const AUDIO_MAX_QUEUE_TICKS: usize = 8;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Turns elapsed wall-clock time into a number of ticks to run.
#[derive(Debug, Default)]
pub struct Pacer {
    /// Time not yet turned into ticks, in units of 1/(70 * 10^9) s, so one tick is exactly
    /// 10^9 units and no rounding error accumulates.
    backlog: u64,
    dropped_ticks: u64,
}

impl Pacer {
    #[must_use]
    pub fn new() -> Pacer {
        Pacer::default()
    }

    /// Adds `elapsed_nanos` of wall-clock time and returns how many ticks to run now, at most
    /// [`MAX_CATCH_UP_TICKS`]. Ticks beyond that are dropped and counted.
    pub fn advance(&mut self, elapsed_nanos: u64) -> u32 {
        self.backlog = self
            .backlog
            .saturating_add(elapsed_nanos.saturating_mul(u64::from(TICKS_PER_SECOND)));
        let due = self.backlog / NANOS_PER_SECOND;
        self.backlog %= NANOS_PER_SECOND;
        let max = u64::from(MAX_CATCH_UP_TICKS);
        if due > max {
            self.dropped_ticks += due - max;
            MAX_CATCH_UP_TICKS
        } else {
            u32::try_from(due).expect("due <= MAX_CATCH_UP_TICKS")
        }
    }

    #[must_use]
    pub fn dropped_ticks(&self) -> u64 {
        self.dropped_ticks
    }
}

/// What to do with one tick's samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDecision {
    /// Queue `silence_frames` stereo frames of silence, then the tick's samples.
    Queue { silence_frames: usize },
    /// Drop the tick's samples: the device queue is too long.
    Drop,
}

/// Keeps the audio device queue near [`AUDIO_TARGET_QUEUE_TICKS`], so latency is low but
/// stable and identical in every frontend.
///
/// - An empty queue (at start, or after the device ran dry) is first padded with silence up to
///   one tick below the target, so ordinary jitter does not starve the device. Running dry
///   after the start counts as an underrun.
/// - A queue longer than [`AUDIO_MAX_QUEUE_TICKS`] (the wall clock runs faster than the sound
///   card) drops incoming audio until it has drained back to the target, so latency cannot
///   drift.
#[derive(Debug, Default)]
pub struct AudioGate {
    started: bool,
    draining: bool,
    underruns: u64,
    discarded_ticks: u64,
}

impl AudioGate {
    #[must_use]
    pub fn new() -> AudioGate {
        AudioGate::default()
    }

    /// `queued_frames` is the number of stereo frames still waiting to be played. Call once per
    /// tick, just before queueing that tick's samples.
    pub fn decide(&mut self, queued_frames: usize) -> AudioDecision {
        let queued_ticks = queued_frames / AUDIO_FRAMES_PER_TICK;
        if queued_ticks > AUDIO_MAX_QUEUE_TICKS {
            self.draining = true;
        } else if queued_ticks <= AUDIO_TARGET_QUEUE_TICKS {
            self.draining = false;
        }
        if self.draining {
            self.discarded_ticks += 1;
            return AudioDecision::Drop;
        }
        if queued_frames > 0 {
            return AudioDecision::Queue { silence_frames: 0 };
        }
        if self.started {
            self.underruns += 1;
        }
        self.started = true;
        AudioDecision::Queue {
            silence_frames: (AUDIO_TARGET_QUEUE_TICKS - 1) * AUDIO_FRAMES_PER_TICK,
        }
    }

    /// Times the queue ran dry after audio had started.
    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.underruns
    }

    #[must_use]
    pub fn discarded_ticks(&self) -> u64 {
        self.discarded_ticks
    }

    /// The counters for a stats line.
    #[must_use]
    pub fn report(&self, queued_frames: usize) -> AudioReport {
        AudioReport {
            underruns: self.underruns,
            discarded_ticks: self.discarded_ticks,
            queued_frames,
        }
    }
}

/// Where to draw the frame inside the output, in output pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The largest rectangle with display aspect `aspect` that fits an output of
/// `output_width` x `output_height`, centred; the rest becomes black bars. A zero-sized output
/// (a minimised window) gives a zero-sized viewport.
#[must_use]
pub fn letterbox(output_width: u32, output_height: u32, aspect: (u32, u32)) -> Viewport {
    let (aspect_width, aspect_height) = (u64::from(aspect.0), u64::from(aspect.1));
    let (out_w, out_h) = (u64::from(output_width), u64::from(output_height));
    let (width, height) = if aspect_width == 0 || aspect_height == 0 {
        (out_w, out_h)
    } else if out_w * aspect_height >= out_h * aspect_width {
        (out_h * aspect_width / aspect_height, out_h)
    } else {
        (out_w, out_w * aspect_height / aspect_width)
    };
    let width = u32::try_from(width).expect("width <= output width");
    let height = u32::try_from(height).expect("height <= output height");
    Viewport {
        x: (output_width - width) / 2,
        y: (output_height - height) / 2,
        width,
        height,
    }
}

/// Audio counters a frontend reports.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioReport {
    pub underruns: u64,
    pub discarded_ticks: u64,
    pub queued_frames: usize,
}

/// Counters a frontend prints once per second and once at exit.
#[derive(Debug, Default)]
pub struct RunStats {
    ticks: u64,
    present_micros: Vec<u32>,
}

impl RunStats {
    #[must_use]
    pub fn new() -> RunStats {
        RunStats::default()
    }

    pub fn add_ticks(&mut self, ticks: u32) {
        self.ticks += u64::from(ticks);
    }

    /// Records how long one present took (upload, draw and present), in microseconds.
    pub fn add_present(&mut self, micros: u32) {
        self.present_micros.push(micros);
    }

    /// One cumulative log line, for example
    /// `t=60.000s ticks=4200 rate=70.00/s frames=3600 dropped_ticks=0 underruns=0 discarded_ticks=0 queue_ms=42 present_avg_us=850 present_p99_us=1200`.
    #[must_use]
    pub fn line(&self, elapsed_nanos: u64, dropped_ticks: u64, audio: AudioReport) -> String {
        let millis = elapsed_nanos / 1_000_000;
        let rate_centi = if elapsed_nanos == 0 {
            0
        } else {
            u128::from(self.ticks) * 100 * u128::from(NANOS_PER_SECOND) / u128::from(elapsed_nanos)
        };
        let queue_ms = audio.queued_frames as u64 * 1000 / u64::from(AUDIO_SAMPLE_RATE);
        format!(
            "t={}.{:03}s ticks={} rate={}.{:02}/s frames={} dropped_ticks={} underruns={} discarded_ticks={} queue_ms={} present_avg_us={} present_p99_us={}",
            millis / 1000,
            millis % 1000,
            self.ticks,
            rate_centi / 100,
            rate_centi % 100,
            self.present_micros.len(),
            dropped_ticks,
            audio.underruns,
            audio.discarded_ticks,
            queue_ms,
            self.present_average(),
            self.present_percentile(99),
        )
    }

    fn present_average(&self) -> u64 {
        let count = self.present_micros.len() as u64;
        if count == 0 {
            return 0;
        }
        self.present_micros
            .iter()
            .map(|&m| u64::from(m))
            .sum::<u64>()
            / count
    }

    /// Nearest-rank percentile of the recorded present times; 0 when nothing was presented.
    fn present_percentile(&self, percent: usize) -> u32 {
        if self.present_micros.is_empty() {
            return 0;
        }
        let mut sorted = self.present_micros.clone();
        sorted.sort_unstable();
        let rank = (sorted.len() * percent).div_ceil(100).max(1);
        sorted[rank - 1]
    }
}
```

- [ ] **Step 4: Run all core tests**

Run: `cargo test -p deadrally-core`
Expected: unit tests `12 passed` (10 host, 2 scene), `frame_input` `3 passed`, `game` `9 passed`.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p deadrally-core --all-targets -- -D warnings
git add crates/core
git commit -m "feat: add shared frontend pacing helpers" -m "- tick pacer with a catch-up cap
- audio gate: silence priming, underruns, drain
- letterbox viewport and the stats line"
```

---

### Task 5: Validate the original game data

**Files:**
- Create: `crates/gamedata/Cargo.toml`, `crates/gamedata/src/lib.rs`, `crates/gamedata/src/known_versions.rs`, `crates/gamedata/src/validate.rs`, `crates/gamedata/tests/common/mod.rs`, `crates/gamedata/tests/validate.rs`

**Interfaces:**
- Produces: `REQUIRED_FILES: [&str; 19]` (canonical upper case); `KnownFile { name, size, sha256 }`, `KnownVersion { name, files }`, `KNOWN_VERSIONS` (one entry: `"Steam: Death Rally (Classic), appid 358270"`); `fn validate(dir: &Path) -> Result<Validation, ValidationError>`; `Validation { dir, files: Vec<FileReport>, outcome: Outcome }`; `FileReport { name, path, size, sha256 }`; `Outcome::Known { version }` / `Outcome::Unknown { closest, differing }`; `ValidationError::DirUnreadable { dir, source }` / `ValidationError::Unusable { dir, missing, ambiguous, unreadable }` with `found_no_required_file(&self) -> bool` and a `Display` that names files and the directory.

- [ ] **Step 1: Write the crate shell and the failing tests**

<!-- write: crates/gamedata/Cargo.toml -->
```toml
[package]
name = "deadrally-gamedata"
description = "Finds and validates the player's copy of the original Death Rally data."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[dependencies]
sha2 = "0.11.0"

[dev-dependencies]
tempfile = "3.27.0"

[lints]
workspace = true
```

<!-- write: crates/gamedata/src/lib.rs -->
```rust
//! Finds and validates the player's copy of the original game data (spec section 6).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.
```

<!-- write: crates/gamedata/tests/common/mod.rs -->
```rust
//! Fixtures: fake installs in temporary directories. Real game data is never committed.

use std::fs;
use std::path::Path;

use deadrally_gamedata::REQUIRED_FILES;

/// Writes every required file into `dir` with placeholder contents, so the set is complete
/// but matches no known release.
pub fn fake_install(dir: &Path) {
    fake_install_named(dir, |name| name.to_owned());
}

/// Like [`fake_install`], with each canonical name mapped through `rename` first.
pub fn fake_install_named(dir: &Path, rename: impl Fn(&str) -> String) {
    fs::create_dir_all(dir).unwrap();
    for name in REQUIRED_FILES {
        fs::write(dir.join(rename(name)), format!("placeholder for {name}")).unwrap();
    }
}
```

<!-- write: crates/gamedata/tests/validate.rs -->
```rust
//! Validation of a data directory (spec section 6).

mod common;

use std::fs;

use common::{fake_install, fake_install_named};
use deadrally_gamedata::{Outcome, REQUIRED_FILES, ValidationError, validate};
use tempfile::tempdir;

#[test]
fn a_complete_but_unrecognised_install_is_usable_with_every_file_named() {
    // Modded or unknown releases may still run; the player must learn which files differ.
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    let validation = validate(dir.path()).unwrap();
    assert_eq!(validation.files.len(), REQUIRED_FILES.len());
    match validation.outcome {
        Outcome::Unknown { closest, differing } => {
            assert_eq!(closest, "Steam: Death Rally (Classic), appid 358270");
            assert_eq!(differing, REQUIRED_FILES.to_vec());
        }
        Outcome::Known { .. } => panic!("placeholder files cannot match a real release"),
    }
}

#[test]
fn a_missing_track_is_an_error_that_names_it_and_the_directory() {
    // Without TR5.BPA the game would crash mid-campaign on track 5; fail at start instead.
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    fs::remove_file(dir.path().join("TR5.BPA")).unwrap();
    let error = validate(dir.path()).unwrap_err();
    match &error {
        ValidationError::Unusable { missing, .. } => assert_eq!(missing, &["TR5.BPA"]),
        ValidationError::DirUnreadable { .. } => panic!("the directory is readable"),
    }
    let message = error.to_string();
    assert!(message.contains("TR5.BPA"), "{message}");
    assert!(
        message.contains(&dir.path().display().to_string()),
        "{message}"
    );
}

#[test]
fn names_match_regardless_of_case() {
    // The Steam copy mixes cases (SANIM.haf, end.bmp); other copies may be all lower case.
    let dir = tempdir().unwrap();
    fake_install_named(dir.path(), str::to_ascii_lowercase);
    let validation = validate(dir.path()).unwrap();
    assert!(validation.files[0].path.ends_with("engine.bpa"));
}

#[test]
fn two_names_differing_only_in_case_are_refused_on_case_sensitive_file_systems() {
    // Picking one at random could load the wrong data; on case-insensitive systems (Windows,
    // macOS default) the second write replaces the first, so there is only one file.
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    fs::write(dir.path().join("sanim.haf"), "a second copy").unwrap();
    let entries = fs::read_dir(dir.path()).unwrap().count();
    let case_sensitive = entries == REQUIRED_FILES.len() + 1;
    match validate(dir.path()) {
        Err(ValidationError::Unusable {
            ambiguous, missing, ..
        }) if case_sensitive => {
            assert!(missing.is_empty());
            assert_eq!(
                ambiguous,
                vec![(
                    "SANIM.HAF",
                    vec!["SANIM.HAF".to_owned(), "sanim.haf".to_owned()]
                )]
            );
        }
        Ok(_) if !case_sensitive => {}
        other => panic!("case_sensitive={case_sensitive}, got {other:?}"),
    }
}

#[test]
fn a_directory_in_place_of_a_file_is_reported_as_unreadable() {
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    fs::remove_file(dir.path().join("MENU.BPA")).unwrap();
    fs::create_dir(dir.path().join("MENU.BPA")).unwrap();
    match validate(dir.path()) {
        Err(ValidationError::Unusable {
            unreadable,
            missing,
            ..
        }) => {
            assert!(missing.is_empty());
            assert_eq!(unreadable.len(), 1);
            assert_eq!(unreadable[0].0, "MENU.BPA");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn read_only_data_validates() {
    // Steam and package managers install data read-only; validation must never need write
    // access (and must never write).
    let dir = tempdir().unwrap();
    fake_install(dir.path());
    for name in REQUIRED_FILES {
        let path = dir.path().join(name);
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
    }
    assert!(validate(dir.path()).is_ok());
}

#[test]
fn a_missing_directory_is_an_error_that_names_it() {
    let dir = tempdir().unwrap();
    let absent = dir.path().join("no-such-dir");
    let error = validate(&absent).unwrap_err();
    assert!(matches!(error, ValidationError::DirUnreadable { .. }));
    assert!(error.to_string().contains("no-such-dir"), "{error}");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-gamedata --test validate`
Expected: compile error, `unresolved imports deadrally_gamedata::Outcome, ...`.

- [ ] **Step 3: Implement**

The hashes below were computed from the Steam download on 2026-10-03 and cross-checked against the files (`sha256sum`). Hashes are facts about the data, not the data.

<!-- write: crates/gamedata/src/known_versions.rs -->
```rust
/// The data files the game needs (brief §9), in canonical upper case. Names on disk match
/// case-insensitively.
pub const REQUIRED_FILES: [&str; 19] = [
    "ENGINE.BPA",
    "IBFILES.BPA",
    "MENU.BPA",
    "MUSICS.BPA",
    "TR0.BPA",
    "TR1.BPA",
    "TR2.BPA",
    "TR3.BPA",
    "TR4.BPA",
    "TR5.BPA",
    "TR6.BPA",
    "TR7.BPA",
    "TR8.BPA",
    "TR9.BPA",
    "SANIM.HAF",
    "ENDANI.HAF",
    "ENDANI0.HAF",
    "END.BMP",
    "RMD.BMP",
];

/// One file of a known release. Hashes are facts about the data, not the data itself, so they
/// may be committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownFile {
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

/// A release of the game whose data we have verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownVersion {
    pub name: &'static str,
    pub files: &'static [KnownFile],
}

/// Every release we recognise. Remedy's 2009 freeware download is added once someone hashes
/// a copy; until then it validates as an unknown version.
pub const KNOWN_VERSIONS: &[KnownVersion] = &[STEAM_358270];

/// Downloaded with steamcmd on 2026-10-03 (Windows depot).
const STEAM_358270: KnownVersion = KnownVersion {
    name: "Steam: Death Rally (Classic), appid 358270",
    files: &[
        KnownFile {
            name: "ENGINE.BPA",
            size: 387_193,
            sha256: "8ff2aff5b4c5d10a1ada9c7529b95bded949d4914f917e6dc6e2fe5448cf14c2",
        },
        KnownFile {
            name: "IBFILES.BPA",
            size: 85_989,
            sha256: "93f08b317df159aeb229d6ba86bb1c6f40ba9d28fdc10dd81e67639348dc4c74",
        },
        KnownFile {
            name: "MENU.BPA",
            size: 3_168_933,
            sha256: "b0993e984066a7e5e7edcfca69476624872a7f322648b0b4961f3c05339f15b1",
        },
        KnownFile {
            name: "MUSICS.BPA",
            size: 5_726_559,
            sha256: "9f1e094f2a8763683027614fd97cb8afce5836c158f0d1b25be2842bf334754b",
        },
        KnownFile {
            name: "TR0.BPA",
            size: 337_303,
            sha256: "368fa56187b8e1ca480e7ec1a6f5aad655d259e8b4bc398bc08131eaed4dd45f",
        },
        KnownFile {
            name: "TR1.BPA",
            size: 321_394,
            sha256: "efa9cd8bec6c240675ef6bfe333638442436c4229cbb9c74772257b8683d4305",
        },
        KnownFile {
            name: "TR2.BPA",
            size: 294_365,
            sha256: "7f242b788f920e4b00f254f30e49bc98536d7cc58513056b456236a0f4821130",
        },
        KnownFile {
            name: "TR3.BPA",
            size: 345_851,
            sha256: "d1d7b5db7d6b3529d7c953557c3a76e7570283f9833a306447d9417a95db7b9f",
        },
        KnownFile {
            name: "TR4.BPA",
            size: 260_466,
            sha256: "238be60f379ea282647ed33157a087b519854cdb821d192878f78f9ef9b0af0d",
        },
        KnownFile {
            name: "TR5.BPA",
            size: 431_992,
            sha256: "ab0767d3acf7c1f45bdfb3124a4206cfe3cadb650602a399d33a05b521e5a9dd",
        },
        KnownFile {
            name: "TR6.BPA",
            size: 359_368,
            sha256: "757587f57fe6711b6353a054e654dedcc5b2553d82476c310912e3ac6339e6c2",
        },
        KnownFile {
            name: "TR7.BPA",
            size: 417_431,
            sha256: "c11dc15b3e8f5e2708d0c6e0cca73ac72ce9d8e88a533e11a3a9c006608805c2",
        },
        KnownFile {
            name: "TR8.BPA",
            size: 253_202,
            sha256: "c7bfb5142cbc7c95dd42b9c40b298d2e3b3af9bb9d445b6c441369e6dfa24036",
        },
        KnownFile {
            name: "TR9.BPA",
            size: 427_555,
            sha256: "3c6336f43d7188fc1f1d4d53307ceea43f64342f9262f9a298e28834b5e36912",
        },
        KnownFile {
            name: "SANIM.HAF",
            size: 21_516_352,
            sha256: "4fbb589fe50f8aa7e47ae41245c64578a9112ea843be2089178a08333087c993",
        },
        KnownFile {
            name: "ENDANI.HAF",
            size: 3_796_331,
            sha256: "13eef7520ae0a5180484a32a4a31efcbb7438f050c7e96f158cc59fb67e2526f",
        },
        KnownFile {
            name: "ENDANI0.HAF",
            size: 7_193_340,
            sha256: "cd20359a766a6ee64137869f65caa91ab6b166dc314ef1ef0561943f7b54a362",
        },
        KnownFile {
            name: "END.BMP",
            size: 308_148,
            sha256: "8723a70d39ca89a092765ce3813b7f34bb6168bdc96e7d635013bcd60bb6d190",
        },
        KnownFile {
            name: "RMD.BMP",
            size: 308_278,
            sha256: "af3cdbcecfeb40aaf9952b19149fadbe40cc46641db90cdd30d12d88598dd972",
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_version_lists_exactly_the_required_files() {
        // A version that forgot a file could never validate as known; one with an extra file
        // would demand data the game does not use.
        for version in KNOWN_VERSIONS {
            let mut names: Vec<_> = version.files.iter().map(|f| f.name).collect();
            names.sort_unstable();
            let mut required = REQUIRED_FILES.to_vec();
            required.sort_unstable();
            assert_eq!(names, required, "{}", version.name);
        }
    }

    #[test]
    fn known_hashes_are_lowercase_sha256_hex() {
        // Validation compares hex strings exactly; an uppercase or truncated entry never matches.
        for file in KNOWN_VERSIONS.iter().flat_map(|v| v.files) {
            assert_eq!(file.sha256.len(), 64, "{}", file.name);
            assert!(
                file.sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "{}",
                file.name
            );
        }
    }
}
```

<!-- write: crates/gamedata/src/validate.rs -->
```rust
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::known_versions::{KNOWN_VERSIONS, KnownVersion, REQUIRED_FILES};

/// One required file as found on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileReport {
    /// Canonical upper-case name from [`REQUIRED_FILES`].
    pub name: &'static str,
    /// The file as named on disk.
    pub path: PathBuf,
    pub size: u64,
    /// Lower-case hex.
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every file matches this known release.
    Known { version: &'static str },
    /// All files are present, but some match no known release. Usable, with a loud warning:
    /// the game may behave differently.
    Unknown {
        closest: &'static str,
        differing: Vec<&'static str>,
    },
}

/// A directory that holds every required file.
#[derive(Debug)]
pub struct Validation {
    pub dir: PathBuf,
    /// In [`REQUIRED_FILES`] order.
    pub files: Vec<FileReport>,
    pub outcome: Outcome,
}

#[derive(Debug)]
pub enum ValidationError {
    /// The directory itself cannot be listed (missing, not a directory, no permission).
    DirUnreadable { dir: PathBuf, source: io::Error },
    /// The game cannot run from this directory.
    Unusable {
        dir: PathBuf,
        missing: Vec<&'static str>,
        /// Several entries match one required name, differing only in case (possible on
        /// case-sensitive file systems). We refuse to guess which one is the data.
        ambiguous: Vec<(&'static str, Vec<String>)>,
        unreadable: Vec<(&'static str, io::Error)>,
    },
}

impl ValidationError {
    /// True when the directory holds none of the required files: the caller may then try the
    /// nested Steam layout.
    #[must_use]
    pub fn found_no_required_file(&self) -> bool {
        matches!(self, ValidationError::Unusable { missing, .. } if missing.len() == REQUIRED_FILES.len())
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::DirUnreadable { dir, source } => {
                write!(
                    f,
                    "cannot read the game data directory {}: {source}",
                    dir.display()
                )
            }
            ValidationError::Unusable {
                dir,
                missing,
                ambiguous,
                unreadable,
            } => {
                write!(f, "the game data in {} is unusable", dir.display())?;
                if !missing.is_empty() {
                    write!(f, "\n  missing: {}", missing.join(", "))?;
                }
                for (name, candidates) in ambiguous {
                    write!(
                        f,
                        "\n  ambiguous: {name} matches {} (names differ only in case)",
                        candidates.join(", ")
                    )?;
                }
                for (name, error) in unreadable {
                    write!(f, "\n  unreadable: {name}: {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValidationError::DirUnreadable { source, .. } => Some(source),
            ValidationError::Unusable { .. } => None,
        }
    }
}

/// Checks that `dir` holds every required file and identifies the release by SHA-256.
/// Files are opened read-only; nothing is written.
///
/// # Errors
///
/// [`ValidationError`] when the directory cannot be listed or a file is missing, ambiguous or
/// unreadable.
pub fn validate(dir: &Path) -> Result<Validation, ValidationError> {
    let unreadable_dir = |source| ValidationError::DirUnreadable {
        dir: dir.to_path_buf(),
        source,
    };
    let mut by_upper_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(dir).map_err(unreadable_dir)? {
        let entry = entry.map_err(unreadable_dir)?;
        // A name that is not UTF-8 cannot be one of the data files.
        if let Some(name) = entry.file_name().to_str() {
            by_upper_name
                .entry(name.to_ascii_uppercase())
                .or_default()
                .push(name.to_owned());
        }
    }

    let mut files = Vec::new();
    let mut missing = Vec::new();
    let mut ambiguous = Vec::new();
    let mut unreadable = Vec::new();
    for name in REQUIRED_FILES {
        match by_upper_name.get(name).map(Vec::as_slice) {
            None | Some([]) => missing.push(name),
            Some([on_disk]) => {
                let path = dir.join(on_disk);
                match hash_file(&path) {
                    Ok((size, sha256)) => files.push(FileReport {
                        name,
                        path,
                        size,
                        sha256,
                    }),
                    Err(error) => unreadable.push((name, error)),
                }
            }
            Some(several) => {
                let mut candidates = several.to_vec();
                candidates.sort();
                ambiguous.push((name, candidates));
            }
        }
    }

    if !(missing.is_empty() && ambiguous.is_empty() && unreadable.is_empty()) {
        return Err(ValidationError::Unusable {
            dir: dir.to_path_buf(),
            missing,
            ambiguous,
            unreadable,
        });
    }
    let outcome = classify(&files, KNOWN_VERSIONS);
    Ok(Validation {
        dir: dir.to_path_buf(),
        files,
        outcome,
    })
}

/// Matches the files against known releases; reports the closest one when none matches fully.
fn classify(files: &[FileReport], versions: &[KnownVersion]) -> Outcome {
    let mut closest: Option<(&KnownVersion, Vec<&'static str>)> = None;
    for version in versions {
        let differing: Vec<&'static str> = files
            .iter()
            .filter(|file| {
                !version.files.iter().any(|known| {
                    known.name == file.name
                        && known.size == file.size
                        && known.sha256 == file.sha256
                })
            })
            .map(|file| file.name)
            .collect();
        if differing.is_empty() {
            return Outcome::Known {
                version: version.name,
            };
        }
        if closest
            .as_ref()
            .is_none_or(|(_, best)| differing.len() < best.len())
        {
            closest = Some((version, differing));
        }
    }
    let (version, differing) = closest.expect("KNOWN_VERSIONS is never empty");
    Outcome::Unknown {
        closest: version.name,
        differing,
    }
}

fn hash_file(path: &Path) -> io::Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut size = 0;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    let hex = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((size, hex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KnownFile;

    const TWO_VERSIONS: &[KnownVersion] = &[
        KnownVersion {
            name: "first",
            files: &[
                KnownFile {
                    name: "TR0.BPA",
                    size: 1,
                    sha256: "aa",
                },
                KnownFile {
                    name: "TR1.BPA",
                    size: 1,
                    sha256: "bb",
                },
            ],
        },
        KnownVersion {
            name: "second",
            files: &[
                KnownFile {
                    name: "TR0.BPA",
                    size: 1,
                    sha256: "aa",
                },
                KnownFile {
                    name: "TR1.BPA",
                    size: 1,
                    sha256: "cc",
                },
            ],
        },
    ];

    fn report(name: &'static str, sha256: &str) -> FileReport {
        FileReport {
            name,
            path: PathBuf::from(name),
            size: 1,
            sha256: sha256.to_owned(),
        }
    }

    #[test]
    fn a_full_match_is_reported_as_that_version() {
        let files = [report("TR0.BPA", "aa"), report("TR1.BPA", "cc")];
        assert_eq!(
            classify(&files, TWO_VERSIONS),
            Outcome::Known { version: "second" }
        );
    }

    #[test]
    fn a_partial_match_names_the_closest_version_and_the_odd_files() {
        // The warning must point at the files that differ, so a player can re-download them.
        let files = [report("TR0.BPA", "aa"), report("TR1.BPA", "zz")];
        assert_eq!(
            classify(&files, TWO_VERSIONS),
            Outcome::Unknown {
                closest: "first",
                differing: vec!["TR1.BPA"]
            }
        );
    }

    #[test]
    fn a_size_mismatch_alone_is_not_a_match() {
        let mut file = report("TR0.BPA", "aa");
        file.size = 2;
        let files = [file, report("TR1.BPA", "bb")];
        assert!(matches!(
            classify(&files, TWO_VERSIONS),
            Outcome::Unknown { .. }
        ));
    }
}
```

<!-- write: crates/gamedata/src/lib.rs -->
```rust
//! Finds and validates the player's copy of the original game data (spec section 6).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

mod known_versions;
mod validate;

pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p deadrally-gamedata`
Expected: unit tests `5 passed`, `validate` `7 passed`.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p deadrally-gamedata --all-targets -- -D warnings
git add Cargo.lock crates/gamedata
git commit -m "feat: validate the original game data"
```

---

### Task 6: Locate the data and read the config

**Files:**
- Create: `crates/gamedata/src/config.rs`, `crates/gamedata/src/locate.rs`, `crates/gamedata/tests/config.rs`, `crates/gamedata/tests/locate.rs`, `crates/gamedata/tests/real_data.rs`
- Modify: `crates/gamedata/Cargo.toml`, `crates/gamedata/src/lib.rs`

**Interfaces:**
- Consumes: Task 5's `validate`, `ValidationError::found_no_required_file`.
- Produces: `DATA_ENV_VAR = "DEADRALLY_DATA"`, `STEAM_SUBDIR = "Death Rally"`; `fn locate(cli: Option<&Path>, env: Option<&OsStr>, config_path: Option<&Path>) -> Result<Located, LocateError>`; `Located { source: DataSource, validation: Validation, config_warnings: Vec<String> }`; `DataSource::{CommandLine, Environment, ConfigFile(PathBuf)}` with `Display` `command line (--data)` / `environment (DEADRALLY_DATA)` / `config file (<path>)`; `LocateError::{NotSpecified { config_path }, Config(ConfigError), Invalid { source, error }}`; `fn config_path() -> Option<PathBuf>`; `fn load_config(path: &Path) -> Result<Option<Config>, ConfigError>`; `Config { data_path: Option<PathBuf>, warnings: Vec<String> }`; `ConfigError::{Read, Parse, WrongType}`.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/gamedata/tests/config.rs -->
```rust
//! Reading DeadRally's config file (spec section 6).

use std::fs;

use deadrally_gamedata::{ConfigError, config_path, load_config};
use tempfile::tempdir;

#[test]
fn a_missing_config_file_is_not_an_error() {
    // Most players never write one.
    let dir = tempdir().unwrap();
    assert_eq!(load_config(&dir.path().join("config.toml")).unwrap(), None);
}

#[test]
fn malformed_toml_reports_where() {
    // Users edit this file by hand in M0; the error must point at the broken line.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "# comment\ndata_path = \"unterminated\n").unwrap();
    let error = load_config(&path).unwrap_err();
    assert!(matches!(error, ConfigError::Parse { .. }));
    assert!(error.to_string().contains("line 2"), "{error}");
}

#[test]
fn data_path_must_be_a_string() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "data_path = 42\n").unwrap();
    assert!(matches!(
        load_config(&path),
        Err(ConfigError::WrongType {
            key: "data_path",
            ..
        })
    ));
}

#[test]
fn unknown_keys_are_warnings_not_errors() {
    // Later versions add keys; an older binary must still start with a newer config.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "data_path = \"/games/dr\"\nwidescreen = true\n").unwrap();
    let config = load_config(&path).unwrap().unwrap();
    assert_eq!(
        config.data_path.as_deref(),
        Some(std::path::Path::new("/games/dr"))
    );
    assert_eq!(config.warnings.len(), 1);
    assert!(config.warnings[0].contains("widescreen"));
}

#[test]
fn a_leading_tilde_means_the_home_directory() {
    // Nothing expands `~` inside a file, but it is what people write.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "data_path = \"~/games/DeathRally\"\n").unwrap();
    let config = load_config(&path).unwrap().unwrap();
    let home = directories::BaseDirs::new()
        .unwrap()
        .home_dir()
        .to_path_buf();
    assert_eq!(config.data_path, Some(home.join("games/DeathRally")));
}

#[test]
fn the_config_file_lives_in_a_deadrally_directory() {
    let path = config_path().expect("CI machines and developers have a home directory");
    assert_eq!(path.file_name().unwrap(), "config.toml");
    let parent = path.parent().unwrap().to_string_lossy().to_lowercase();
    assert!(parent.contains("deadrally"), "{parent}");
}
```

<!-- write: crates/gamedata/tests/locate.rs -->
```rust
//! Choosing the data directory: precedence, no fall-through, the Steam layout (spec section 6).

mod common;

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use common::fake_install;
use deadrally_gamedata::{DataSource, LocateError, STEAM_SUBDIR, ValidationError, locate};
use tempfile::tempdir;

fn write_config(path: &Path, data_dir: &Path) {
    let value = data_dir.display().to_string().replace('\\', "\\\\");
    fs::write(path, format!("data_path = \"{value}\"\n")).unwrap();
}

#[test]
fn the_command_line_wins_over_environment_and_config() {
    let root = tempdir().unwrap();
    let (cli, env, from_config) = (
        root.path().join("cli"),
        root.path().join("env"),
        root.path().join("cfg"),
    );
    for dir in [&cli, &env, &from_config] {
        fake_install(dir);
    }
    let config = root.path().join("config.toml");
    write_config(&config, &from_config);

    let located = locate(Some(&cli), Some(env.as_os_str()), Some(&config)).unwrap();
    assert_eq!(located.source, DataSource::CommandLine);
    assert_eq!(located.validation.dir, cli);
}

#[test]
fn the_environment_wins_over_config() {
    let root = tempdir().unwrap();
    let (env, from_config) = (root.path().join("env"), root.path().join("cfg"));
    fake_install(&env);
    fake_install(&from_config);
    let config = root.path().join("config.toml");
    write_config(&config, &from_config);

    let located = locate(None, Some(env.as_os_str()), Some(&config)).unwrap();
    assert_eq!(located.source, DataSource::Environment);
    assert_eq!(located.validation.dir, env);
}

#[test]
fn the_config_file_is_used_when_nothing_else_is_given() {
    let root = tempdir().unwrap();
    let data = root.path().join("data");
    fake_install(&data);
    let config = root.path().join("config.toml");
    write_config(&config, &data);

    let located = locate(None, None, Some(&config)).unwrap();
    assert_eq!(located.source, DataSource::ConfigFile(config));
    assert_eq!(located.validation.dir, data);
}

#[test]
fn an_empty_environment_variable_counts_as_unset() {
    // `DEADRALLY_DATA= deadrally` is a common way to clear a variable for one run.
    let root = tempdir().unwrap();
    let data = root.path().join("data");
    fake_install(&data);
    let config = root.path().join("config.toml");
    write_config(&config, &data);

    let located = locate(None, Some(OsStr::new("")), Some(&config)).unwrap();
    assert!(matches!(located.source, DataSource::ConfigFile(_)));
}

#[test]
fn a_wrong_command_line_path_is_an_error_even_if_the_environment_is_valid() {
    // Silently using a different copy than the one asked for hides mistakes.
    let root = tempdir().unwrap();
    let env = root.path().join("env");
    fake_install(&env);
    let wrong = root.path().join("empty");
    fs::create_dir(&wrong).unwrap();

    match locate(Some(&wrong), Some(env.as_os_str()), None) {
        Err(LocateError::Invalid {
            source: DataSource::CommandLine,
            error,
        }) => {
            assert!(error.found_no_required_file());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_steam_folder_one_level_up_is_accepted() {
    // Players will point at steamapps/common/Death Rally, which holds the data one level down.
    let root = tempdir().unwrap();
    let nested = root.path().join(STEAM_SUBDIR);
    fake_install(&nested);

    let located = locate(Some(root.path()), None, None).unwrap();
    assert_eq!(located.validation.dir, nested);
}

#[test]
fn a_partial_install_is_not_rescued_by_the_steam_subfolder() {
    // If the given directory holds some data files, it is the data directory and its missing
    // files are the error to report.
    let root = tempdir().unwrap();
    fake_install(root.path());
    fs::remove_file(root.path().join("TR0.BPA")).unwrap();
    fake_install(&root.path().join(STEAM_SUBDIR));

    match locate(Some(root.path()), None, None) {
        Err(LocateError::Invalid {
            error: ValidationError::Unusable { missing, .. },
            ..
        }) => {
            assert_eq!(missing, ["TR0.BPA"]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn nothing_specified_names_all_three_sources_and_the_config_path() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    let error = locate(None, None, Some(&config)).unwrap_err();
    let message = error.to_string();
    for needle in [
        "--data",
        "DEADRALLY_DATA",
        "data_path",
        &config.display().to_string(),
    ] {
        assert!(message.contains(needle), "missing {needle:?} in {message}");
    }
}

#[test]
fn a_broken_config_is_an_error_not_a_silent_fallback() {
    let root = tempdir().unwrap();
    let config = root.path().join("config.toml");
    fs::write(&config, "data_path = \n").unwrap();
    assert!(matches!(
        locate(None, None, Some(&config)),
        Err(LocateError::Config(_))
    ));
}

#[test]
fn config_warnings_are_passed_on() {
    let root = tempdir().unwrap();
    let data = root.path().join("data");
    fake_install(&data);
    let config = root.path().join("config.toml");
    let value = data.display().to_string().replace('\\', "\\\\");
    fs::write(
        &config,
        format!("data_path = \"{value}\"\nfullscreen = true\n"),
    )
    .unwrap();

    let located = locate(None, None, Some(&config)).unwrap();
    assert_eq!(located.config_warnings.len(), 1);
    assert!(located.config_warnings[0].contains("fullscreen"));
}
```

<!-- write: crates/gamedata/tests/real_data.rs -->
```rust
//! Tests against the developer's real game data. Run with `cargo test-data`; they read the
//! directory from DEADRALLY_DATA and fail (never pass silently) when it is unset.

use std::path::PathBuf;

use deadrally_gamedata::{DATA_ENV_VAR, Outcome, locate};

fn data_dir() -> PathBuf {
    match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_developers_install_is_a_known_release() {
    // Parity work assumes the data we compare against is exactly a release we know.
    let located = locate(Some(&data_dir()), None, None).unwrap_or_else(|error| panic!("{error}"));
    match located.validation.outcome {
        Outcome::Known { version } => println!("recognised: {version}"),
        Outcome::Unknown { closest, differing } => {
            panic!("unknown release (closest: {closest}); differing: {differing:?}")
        }
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-gamedata --test config --test locate`
Expected: compile errors, `unresolved imports deadrally_gamedata::ConfigError, ...`.

- [ ] **Step 3: Implement**

<!-- write: crates/gamedata/Cargo.toml -->
```toml
[package]
name = "deadrally-gamedata"
description = "Finds and validates the player's copy of the original Death Rally data."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[dependencies]
directories = "6.0.0"
sha2 = "0.11.0"
toml = "1.1.6"

[dev-dependencies]
tempfile = "3.27.0"

[lints]
workspace = true
```

<!-- write: crates/gamedata/src/config.rs -->
```rust
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

/// DeadRally's own settings. M0 knows one key; DeadRally only reads the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// `data_path`: the directory with the original game data.
    pub data_path: Option<PathBuf>,
    /// Human-readable notes about keys that were ignored.
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    /// Not valid TOML; `message` includes the line and column.
    Parse {
        path: PathBuf,
        message: String,
    },
    WrongType {
        path: PathBuf,
        key: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "cannot read config file {}: {source}", path.display())
            }
            ConfigError::Parse { path, message } => {
                write!(
                    f,
                    "config file {} is not valid TOML: {message}",
                    path.display()
                )
            }
            ConfigError::WrongType {
                path,
                key,
                expected,
            } => {
                write!(
                    f,
                    "config file {}: `{key}` must be a {expected}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { .. } | ConfigError::WrongType { .. } => None,
        }
    }
}

/// Where this platform keeps DeadRally's `config.toml`, or `None` when it has no config
/// directory (no home directory, for example).
#[must_use]
pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "DeadRally").map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Reads the config file. A file that does not exist is not an error: `Ok(None)`.
///
/// # Errors
///
/// [`ConfigError`] when the file exists but cannot be read, is not TOML, or `data_path` is not
/// a string.
pub fn load_config(path: &Path) -> Result<Option<Config>, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let table: toml::Table = text
        .parse()
        .map_err(|error: toml::de::Error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    let mut config = Config::default();
    for (key, value) in &table {
        match key.as_str() {
            "data_path" => match value.as_str() {
                Some(data_path) => config.data_path = Some(expand_home(data_path)),
                None => {
                    return Err(ConfigError::WrongType {
                        path: path.to_path_buf(),
                        key: "data_path",
                        expected: "string",
                    });
                }
            },
            unknown => config.warnings.push(format!(
                "unknown key `{unknown}` in {} (ignored)",
                path.display()
            )),
        }
    }
    Ok(Some(config))
}

/// Expands a leading `~`. The shell does this for `--data` and `DEADRALLY_DATA`, but nothing
/// does it inside a file, and `~` is the natural thing to write there.
fn expand_home(value: &str) -> PathBuf {
    let rest = if value == "~" {
        Some("")
    } else {
        value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
    };
    match (rest, BaseDirs::new()) {
        (Some(rest), Some(base)) => base.home_dir().join(rest),
        _ => PathBuf::from(value),
    }
}
```

<!-- write: crates/gamedata/src/locate.rs -->
```rust
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::config::{Config, ConfigError, load_config};
use crate::validate::{Validation, ValidationError, validate};

/// Environment variable naming the game data directory.
pub const DATA_ENV_VAR: &str = "DEADRALLY_DATA";

/// Steam installs the data one level down: `steamapps/common/Death Rally/Death Rally/`.
pub const STEAM_SUBDIR: &str = "Death Rally";

/// Where the data directory came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataSource {
    CommandLine,
    Environment,
    ConfigFile(PathBuf),
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataSource::CommandLine => write!(f, "command line (--data)"),
            DataSource::Environment => write!(f, "environment ({DATA_ENV_VAR})"),
            DataSource::ConfigFile(path) => write!(f, "config file ({})", path.display()),
        }
    }
}

#[derive(Debug)]
pub struct Located {
    pub source: DataSource,
    pub validation: Validation,
    /// Ignored config keys, worth showing to the user.
    pub config_warnings: Vec<String>,
}

#[derive(Debug)]
pub enum LocateError {
    /// No source names a directory. `config_path` is where the config file would be.
    NotSpecified {
        config_path: Option<PathBuf>,
    },
    Config(ConfigError),
    /// The chosen source names a directory that does not hold usable data. Lower-precedence
    /// sources are not tried: a wrong explicit path must not be silently replaced.
    Invalid {
        source: DataSource,
        error: ValidationError,
    },
}

impl fmt::Display for LocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocateError::NotSpecified { config_path } => {
                write!(
                    f,
                    "no game data directory given: pass --data <dir>, set {DATA_ENV_VAR}, or set data_path in "
                )?;
                match config_path {
                    Some(path) => write!(f, "{}", path.display()),
                    None => write!(f, "the config file (this system has no config directory)"),
                }
            }
            LocateError::Config(error) => write!(f, "{error}"),
            LocateError::Invalid { source, error } => {
                write!(f, "game data from the {source}: {error}")
            }
        }
    }
}

impl std::error::Error for LocateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LocateError::NotSpecified { .. } => None,
            LocateError::Config(error) => Some(error),
            LocateError::Invalid { error, .. } => Some(error),
        }
    }
}

/// Finds and validates the game data. The first source that is *specified* wins, in this
/// order: `cli` (`--data`), `env` (the value of `DEADRALLY_DATA`; empty counts as unset), then
/// `data_path` in the config file at `config_path`. The config file is read only when the
/// first two are absent.
///
/// # Errors
///
/// [`LocateError`] when no source is specified, the config file is broken, or the chosen
/// directory does not hold usable data.
pub fn locate(
    cli: Option<&Path>,
    env: Option<&OsStr>,
    config_path: Option<&Path>,
) -> Result<Located, LocateError> {
    let (source, dir, config_warnings) = if let Some(dir) = cli {
        (DataSource::CommandLine, dir.to_path_buf(), Vec::new())
    } else if let Some(dir) = env.filter(|value| !value.is_empty()) {
        (DataSource::Environment, PathBuf::from(dir), Vec::new())
    } else {
        let not_specified = || LocateError::NotSpecified {
            config_path: config_path.map(Path::to_path_buf),
        };
        let path = config_path.ok_or_else(not_specified)?;
        match load_config(path).map_err(LocateError::Config)? {
            Some(Config {
                data_path: Some(dir),
                warnings,
            }) => (DataSource::ConfigFile(path.to_path_buf()), dir, warnings),
            Some(Config {
                data_path: None, ..
            })
            | None => return Err(not_specified()),
        }
    };
    let validation = validate_with_steam_fallback(&dir).map_err(|error| LocateError::Invalid {
        source: source.clone(),
        error,
    })?;
    Ok(Located {
        source,
        validation,
        config_warnings,
    })
}

/// Validates `dir`; if it holds none of the files but has a `Death Rally` subdirectory, that
/// subdirectory is validated instead.
fn validate_with_steam_fallback(dir: &Path) -> Result<Validation, ValidationError> {
    match validate(dir) {
        Err(error) if error.found_no_required_file() && dir.join(STEAM_SUBDIR).is_dir() => {
            validate(&dir.join(STEAM_SUBDIR))
        }
        result => result,
    }
}
```

<!-- write: crates/gamedata/src/lib.rs -->
```rust
//! Finds and validates the player's copy of the original game data (spec section 6).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

mod config;
mod known_versions;
mod locate;
mod validate;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
```

- [ ] **Step 4: Run the tests without and with the real data**

```bash
cargo test -p deadrally-gamedata
cargo test -p deadrally-gamedata --test real_data -- --ignored; echo "exit=$?"
DEADRALLY_DATA=~/games/DeathRally cargo test -p deadrally-gamedata --test real_data -- --ignored --nocapture
```

Expected:
- First command: unit `5 passed`, `config` `6 passed`, `locate` `10 passed`, `real_data` `0 passed; ... 1 ignored`, `validate` `7 passed`.
- Second: the test panics with `DEADRALLY_DATA is not set: point it at your Death Rally data ...` and `exit=101`. This proves the data test cannot pass silently.
- Third: `recognised: Steam: Death Rally (Classic), appid 358270` and `1 passed`, in well under a second (dependencies are optimised in `dev`). Pointing at `~/games/DeathRally` also proves the Steam-subfolder fallback on real data.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p deadrally-gamedata --all-targets -- -D warnings
git add Cargo.lock crates/gamedata
git commit -m "feat: locate game data and read the config" -m "- precedence --data, DEADRALLY_DATA, config file
- no fall-through from a wrong explicit path
- Steam subfolder, ~ expansion, empty env as unset
- real-data test behind cargo test-data"
```

---

### Task 7: Headless runner

**Files:**
- Create: `crates/headless/Cargo.toml`, `crates/headless/src/main.rs`, `crates/headless/tests/cli.rs`

**Interfaces:**
- Consumes: `deadrally_core::Game`, `deadrally_gamedata::{locate, config_path, DATA_ENV_VAR, Outcome, REQUIRED_FILES}`.
- Produces: binary `deadrally-headless` with `run --ticks N` → `ticks=N frames_sha256=<64 hex> audio_sha256=<64 hex>`, and `check-data [--data PATH]` → exit 0 known version, 2 unknown version, 1 unusable or error. CI (Task 8) calls `target/release/deadrally-headless run --ticks 7000`.

- [ ] **Step 1: Write the failing tests against an empty binary**

<!-- write: crates/headless/Cargo.toml -->
```toml
[package]
name = "deadrally-headless"
description = "Runs the DeadRally core without a window: determinism hashes and game data checks."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[dependencies]
deadrally-core = { path = "../core" }
deadrally-gamedata = { path = "../gamedata" }
sha2 = "0.11.0"

[dev-dependencies]
tempfile = "3.27.0"

[lints]
workspace = true
```

<!-- write: crates/headless/src/main.rs -->
```rust
fn main() {}
```

<!-- write: crates/headless/tests/cli.rs -->
```rust
//! The headless binary as CI and developers use it (spec section 8).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::{TempDir, tempdir};

const REQUIRED_FILES: [&str; 19] = deadrally_gamedata::REQUIRED_FILES;

/// The binary with an empty, private config directory and no DEADRALLY_DATA, so the
/// developer's own settings cannot leak into a test.
fn headless(home: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_deadrally-headless"));
    command
        .env_remove("DEADRALLY_DATA")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"));
    command
}

fn fake_install(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    for name in REQUIRED_FILES {
        fs::write(dir.join(name), "placeholder").unwrap();
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn run(home: &TempDir, args: &[&str]) -> Output {
    headless(home).args(args).output().unwrap()
}

#[test]
fn run_prints_the_same_hashes_every_time() {
    // CI compares this line across three operating systems; it must be stable within one.
    let home = tempdir().unwrap();
    let first = run(&home, &["run", "--ticks", "140"]);
    let second = run(&home, &["run", "--ticks", "140"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);

    let line = text(&first.stdout);
    let fields: Vec<&str> = line.trim().split(' ').collect();
    assert_eq!(fields[0], "ticks=140");
    for (field, prefix) in fields[1..].iter().zip(["frames_sha256=", "audio_sha256="]) {
        let hash = field
            .strip_prefix(prefix)
            .unwrap_or_else(|| panic!("{line}"));
        assert_eq!(hash.len(), 64, "{line}");
    }
}

#[test]
fn run_hashes_depend_on_the_number_of_ticks() {
    // A hash that ignored the frames would make the determinism job pass vacuously.
    let home = tempdir().unwrap();
    let short = text(&run(&home, &["run", "--ticks", "1"]).stdout);
    let long = text(&run(&home, &["run", "--ticks", "2"]).stdout);
    let hashes = |line: &str| line.split_once(' ').unwrap().1.to_owned();
    assert_ne!(hashes(&short), hashes(&long));
}

#[test]
fn bad_arguments_print_usage_and_fail() {
    let home = tempdir().unwrap();
    for args in [&[][..], &["run", "--ticks", "many"], &["fly"]] {
        let output = run(&home, args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(text(&output.stderr).contains("usage:"), "{args:?}");
    }
}

#[test]
fn check_data_reports_an_unknown_version_with_exit_status_2() {
    // Usable but unrecognised data must be distinguishable from both success and failure.
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let output = run(&home, &["check-data", "--data", data.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("source: command line (--data)"), "{stdout}");
    assert!(stdout.contains("UNKNOWN VERSION"), "{stdout}");
    assert!(text(&output.stderr).contains("warning: unknown version"));
}

#[test]
fn check_data_names_a_missing_file_and_fails() {
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    fs::remove_file(data.join("SANIM.HAF")).unwrap();
    let output = run(&home, &["check-data", "--data", data.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("SANIM.HAF"));
}

#[test]
fn check_data_uses_the_environment_when_no_option_is_given() {
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let output = headless(&home)
        .arg("check-data")
        .env("DEADRALLY_DATA", &data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stdout).contains("source: environment (DEADRALLY_DATA)"));
}

#[test]
fn check_data_does_not_fall_back_from_a_wrong_option_to_the_environment() {
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let wrong = home.path().join("wrong");
    fs::create_dir(&wrong).unwrap();
    let output = headless(&home)
        .args(["check-data", "--data", wrong.to_str().unwrap()])
        .env("DEADRALLY_DATA", &data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("command line"));
}

#[test]
fn check_data_without_any_source_explains_all_three() {
    let home = tempdir().unwrap();
    let output = run(&home, &["check-data"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    for needle in ["--data", "DEADRALLY_DATA", "data_path"] {
        assert!(stderr.contains(needle), "{stderr}");
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn check_data_recognises_the_developers_install() {
    let data = std::env::var_os("DEADRALLY_DATA")
        .filter(|value| !value.is_empty())
        .expect(
            "DEADRALLY_DATA is not set: point it at your Death Rally data to run `cargo test-data`",
        );
    let home = tempdir().unwrap();
    let output = headless(&home)
        .arg("check-data")
        .env("DEADRALLY_DATA", data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("outcome: known version"));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-headless --test cli`
Expected: `8 failed` (the empty binary prints nothing and exits 0).

- [ ] **Step 3: Implement**

<!-- write: crates/headless/src/main.rs -->
```rust
//! Runs the core without a window (spec section 8).
//!
//! `run --ticks N` hashes every frame and every audio sample, so two runs, or the same run on
//! two operating systems, can be compared with one line. `check-data` reports where the game
//! data was found and whether it is a known release.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadrally_core::Game;
use deadrally_gamedata::{DATA_ENV_VAR, Outcome, config_path, locate};
use sha2::{Digest, Sha256};

const USAGE: &str = "usage:
  deadrally-headless run --ticks N
  deadrally-headless check-data [--data PATH]";

/// Exit status of `check-data` when the data is usable but not a known release.
const EXIT_UNKNOWN_VERSION: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run { ticks: u64 },
    CheckData { data: Option<PathBuf> },
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse(&args) {
        Ok(Command::Run { ticks }) => {
            println!("{}", run(ticks));
            ExitCode::SUCCESS
        }
        Ok(Command::CheckData { data }) => check_data(data.as_deref()),
        Err(message) => {
            eprintln!("error: {message}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn parse(args: &[OsString]) -> Result<Command, String> {
    let mut args = args.iter();
    let command = args.next().ok_or("missing command")?;
    let mut ticks = None;
    let mut data = None;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.to_str() {
            Some("--ticks") if command == "run" => {
                let value = value("--ticks")?;
                let parsed = value.to_str().and_then(|text| text.parse::<u64>().ok());
                ticks = Some(parsed.ok_or(format!(
                    "--ticks: not a number: {}",
                    value.to_string_lossy()
                ))?);
            }
            Some("--data") if command == "check-data" => {
                data = Some(PathBuf::from(value("--data")?))
            }
            _ => return Err(format!("unexpected argument: {}", arg.to_string_lossy())),
        }
    }
    match command.to_str() {
        Some("run") => Ok(Command::Run {
            ticks: ticks.ok_or("run needs --ticks N")?,
        }),
        Some("check-data") => Ok(Command::CheckData { data }),
        _ => Err(format!("unknown command: {}", command.to_string_lossy())),
    }
}

/// Runs `ticks` ticks without input and hashes, per tick in order: width, height and both
/// aspect terms as little-endian u32, the 768 palette bytes, the pixels; and every audio sample
/// as little-endian i16.
fn run(ticks: u64) -> String {
    let mut game = Game::new();
    let mut frames = Sha256::new();
    let mut audio = Sha256::new();
    let mut samples = Vec::new();
    let mut sample_bytes = Vec::new();
    for _ in 0..ticks {
        game.tick();
        let frame = game.frame();
        for value in [frame.width, frame.height, frame.aspect.0, frame.aspect.1] {
            frames.update(value.to_le_bytes());
        }
        frames.update(frame.palette.as_flattened());
        frames.update(frame.pixels);

        samples.clear();
        game.take_audio(&mut samples);
        sample_bytes.clear();
        sample_bytes.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
        audio.update(&sample_bytes);
    }
    format!(
        "ticks={ticks} frames_sha256={} audio_sha256={}",
        hex(&frames.finalize()),
        hex(&audio.finalize())
    )
}

fn check_data(cli: Option<&Path>) -> ExitCode {
    let config = config_path();
    match &config {
        Some(path) => println!("config file: {}", path.display()),
        None => println!("config file: unavailable (this system has no config directory)"),
    }
    let env = std::env::var_os(DATA_ENV_VAR);
    let located = match locate(cli, env.as_deref(), config.as_deref()) {
        Ok(located) => located,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    println!("source: {}", located.source);
    println!("directory: {}", located.validation.dir.display());
    for file in &located.validation.files {
        println!("  {:<12} {:>9}  {}", file.name, file.size, file.sha256);
    }
    match &located.validation.outcome {
        Outcome::Known { version } => {
            println!("outcome: known version: {version}");
            ExitCode::SUCCESS
        }
        Outcome::Unknown { closest, differing } => {
            println!("outcome: UNKNOWN VERSION (closest: {closest})");
            eprintln!(
                "warning: unknown version, the game may behave differently; files that differ from {closest}: {}",
                differing.join(", ")
            );
            ExitCode::from(EXIT_UNKNOWN_VERSION)
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_both_commands() {
        assert_eq!(
            parse(&args(&["run", "--ticks", "7000"])),
            Ok(Command::Run { ticks: 7000 })
        );
        assert_eq!(
            parse(&args(&["check-data"])),
            Ok(Command::CheckData { data: None })
        );
        assert_eq!(
            parse(&args(&["check-data", "--data", "/x"])),
            Ok(Command::CheckData {
                data: Some(PathBuf::from("/x"))
            })
        );
    }

    #[test]
    fn rejects_options_of_the_other_command() {
        // A misplaced option must not be silently ignored.
        assert!(parse(&args(&["run", "--ticks", "1", "--data", "/x"])).is_err());
        assert!(parse(&args(&["check-data", "--ticks", "1"])).is_err());
    }

    #[test]
    fn rejects_bad_tick_counts() {
        for bad in [
            &["run"][..],
            &["run", "--ticks"],
            &["run", "--ticks", "-1"],
            &["run", "--ticks", "ten"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }
}
```

- [ ] **Step 4: Run the tests and check the hashes**

```bash
cargo test -p deadrally-headless
DEADRALLY_DATA=~/games/DeathRally cargo test -p deadrally-headless -- --ignored
cargo run -q --release -p deadrally-headless -- run --ticks 70
```

Expected:
- unit `3 passed`; `cli` `8 passed; ... 1 ignored`; with data, `1 passed`.
- The last command prints exactly `ticks=70 frames_sha256=931fe80287e412b4bcb77fc5d92e0c917a2caeed71aa7cd8fed3a3030447ac94 audio_sha256=d61f7b0fdbaf45e8a95a2ac50bf1b91605700f822553ca7c3f11863ea3d4189c`. A different hash means the core or the hashing differs from the verified code: find the difference before going on.
- `cargo run -q --release -p deadrally-headless -- run --ticks 7000` prints `ticks=7000 frames_sha256=65f65191c6e0f2a11c33950c1adc726cc43340dc8b9df8e7d65b79c7596c890d audio_sha256=06614a9611139d83cb1240594448ab6c67987f23f61f3228b9dd4fd0fc5d49ae`. It takes about 35 s on this machine (no SHA extensions in the CPU); optional here, CI runs it.

- [ ] **Step 5: Check the data report by hand**

```bash
cargo run -q -p deadrally-headless -- check-data --data ~/games/DeathRally; echo "exit=$?"
```

Expected: `source: command line (--data)`, `directory: /home/trashcan/games/DeathRally/Death Rally`, 19 file lines, `outcome: known version: Steam: Death Rally (Classic), appid 358270`, `exit=0`.

- [ ] **Step 6: Lint and commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add Cargo.lock crates/headless
git commit -m "feat: add the headless runner"
```

---

### Task 8: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `scripts/check-no-game-data.sh`, `scripts/install-linux-deps.sh`, `deadrally-headless run --ticks 7000`.
- Produces: jobs `lint`, `test` (ubuntu, macOS ARM, Windows; uploads binaries `target/release/deadrally-*` and the determinism hash), `macos-intel`, `core-purity`, `determinism`. Later tasks need no workflow changes: the artifact glob picks up new binaries.

- [ ] **Step 1: Write the workflow**

<!-- write: .github/workflows/ci.yml -->
```yaml
name: CI

on:
  push:
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - name: No game data or local output is tracked
        run: scripts/check-no-game-data.sh
      - run: scripts/install-linux-deps.sh
      - run: rustup toolchain install
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --locked -- -D warnings

  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - if: runner.os == 'Linux'
        run: scripts/install-linux-deps.sh
      - run: rustup toolchain install
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --locked
      - run: cargo build --workspace --release --locked
      - name: Determinism hash
        shell: bash
        run: target/release/deadrally-headless run --ticks 7000 | tee "determinism-${{ runner.os }}.txt"
      - uses: actions/upload-artifact@v7
        with:
          name: determinism-${{ runner.os }}
          path: determinism-${{ runner.os }}.txt
      - uses: actions/upload-artifact@v7
        with:
          name: deadrally-${{ runner.os }}-${{ runner.arch }}
          if-no-files-found: error
          path: |
            target/release/deadrally-*
            !target/release/*.d
            !target/release/*.pdb

  macos-intel:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v7
      - run: rustup toolchain install
      - run: rustup target add x86_64-apple-darwin
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --workspace --release --locked --target x86_64-apple-darwin
      - uses: actions/upload-artifact@v7
        with:
          name: deadrally-macOS-X64
          if-no-files-found: error
          path: |
            target/x86_64-apple-darwin/release/deadrally-*
            !target/x86_64-apple-darwin/release/*.d

  core-purity:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - run: rustup toolchain install
      - name: deadrally-core depends on no platform crate
        run: |
          tree=$(cargo tree --locked -p deadrally-core -e normal --prefix none)
          echo "$tree"
          if echo "$tree" | grep -Eiq '^(sdl|winit|wgpu|pixels|cpal|gilrs)'; then
            echo "error: deadrally-core depends on a platform crate" >&2
            exit 1
          fi

  determinism:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v8
        with:
          pattern: determinism-*
          merge-multiple: true
      - name: Identical hashes on Linux, macOS and Windows
        run: |
          cat determinism-*.txt
          test "$(ls determinism-*.txt | wc -l)" -eq 3
          test "$(sort -u determinism-*.txt | wc -l)" -eq 1
```

- [ ] **Step 2: Lint the workflow**

```bash
mkdir -p target/tools
curl -sL https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz | tar xz -C target/tools actionlint
target/tools/actionlint .github/workflows/ci.yml && echo ok
```

Expected: `ok`.

- [ ] **Step 3: Run each job's commands locally** (everything except the macOS and Windows parts)

```bash
scripts/check-no-game-data.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
tree=$(cargo tree --locked -p deadrally-core -e normal --prefix none); echo "$tree"
echo "$tree" | grep -Eiq '^(sdl|winit|wgpu|pixels|cpal|gilrs)' && echo IMPURE || echo PURE
```

Expected: all green; the tree is the single line `deadrally-core v0.0.0 (...)`; `PURE`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add the CI workflow"
```

- [ ] **Step 5: [OWNER] First CI run (optional now, required in Task 13)**

This machine has no GitHub credentials. Ask the owner (in Czech) whether they want to set up access now: an SSH key (`ssh-keygen -t ed25519 -C "victor.trnka@gmail.com"`, then add `~/.ssh/id_ed25519.pub` at github.com → Settings → SSH keys, and `git remote set-url origin git@github.com:victortrnka/DeadRally.git`) or a token. With access and their consent: `git push -u origin m0-foundations` and watch the run at github.com/victortrnka/DeadRally/actions. Expected: all five jobs green, `determinism` showing three identical lines. If they prefer to wait, record "CI not yet run" and continue; Task 13 needs it.

---

### Task 9: SDL3 frontend candidate

**Files:**
- Create: `crates/front-sdl/Cargo.toml`, `crates/front-sdl/src/keymap.rs`, `crates/front-sdl/src/main.rs`, `scripts/spike-check.sh`

**Interfaces:**
- Consumes: `deadrally_core::{Game, InputEvent, PadAxis, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE}`, `deadrally_core::host::{AudioDecision, AudioGate, Pacer, RunStats, letterbox}`.
- Produces: binary `deadrally-sdl [-window] [-novsync]`; one stats line per second on stdout (format from `RunStats::line`), plus `final ...` at exit; `scripts/spike-check.sh screens|perf <binary> <out-dir> [soak-seconds]`, used again in Tasks 10 and 11.

- [ ] **Step 1: Write the failing key-map test**

<!-- write: crates/front-sdl/Cargo.toml -->
```toml
[package]
name = "deadrally-front-sdl"
description = "Platform spike candidate: DeadRally frontend on SDL3."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[[bin]]
name = "deadrally-sdl"
path = "src/main.rs"

[dependencies]
deadrally-core = { path = "../core" }
# SDL3 is compiled from source and linked statically: one binary, no SDL3.dll to ship.
sdl3 = { version = "0.20.0", features = ["build-from-source-static"] }

[lints]
workspace = true
```

<!-- write: crates/front-sdl/src/main.rs -->
```rust
mod keymap;

fn main() {}
```

<!-- write: crates/front-sdl/src/keymap.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_key_is_reachable() {
        // A key the frontend cannot produce could never be bound in the controls menu.
        let mut reached: Vec<Key> = (0..512)
            .filter_map(Scancode::from_i32)
            .filter_map(key)
            .collect();
        reached.sort();
        reached.dedup();
        assert_eq!(reached, Key::ALL);
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p deadrally-front-sdl`
Expected: the first build compiles SDL3 from source (several minutes), then compile errors: `cannot find function key`, `use of undeclared type Scancode`.

- [ ] **Step 3: Implement the key map above the test**

<!-- prepend: crates/front-sdl/src/keymap.rs -->
```rust
use deadrally_core::{Key, PadButton};
use sdl3::gamepad::Button;
use sdl3::keyboard::Scancode;

/// Maps an SDL scancode (a physical key position) to the core's key, if the core knows it.
pub fn key(scancode: Scancode) -> Option<Key> {
    Some(match scancode {
        Scancode::A => Key::A,
        Scancode::B => Key::B,
        Scancode::C => Key::C,
        Scancode::D => Key::D,
        Scancode::E => Key::E,
        Scancode::F => Key::F,
        Scancode::G => Key::G,
        Scancode::H => Key::H,
        Scancode::I => Key::I,
        Scancode::J => Key::J,
        Scancode::K => Key::K,
        Scancode::L => Key::L,
        Scancode::M => Key::M,
        Scancode::N => Key::N,
        Scancode::O => Key::O,
        Scancode::P => Key::P,
        Scancode::Q => Key::Q,
        Scancode::R => Key::R,
        Scancode::S => Key::S,
        Scancode::T => Key::T,
        Scancode::U => Key::U,
        Scancode::V => Key::V,
        Scancode::W => Key::W,
        Scancode::X => Key::X,
        Scancode::Y => Key::Y,
        Scancode::Z => Key::Z,
        Scancode::_0 => Key::Digit0,
        Scancode::_1 => Key::Digit1,
        Scancode::_2 => Key::Digit2,
        Scancode::_3 => Key::Digit3,
        Scancode::_4 => Key::Digit4,
        Scancode::_5 => Key::Digit5,
        Scancode::_6 => Key::Digit6,
        Scancode::_7 => Key::Digit7,
        Scancode::_8 => Key::Digit8,
        Scancode::_9 => Key::Digit9,
        Scancode::F1 => Key::F1,
        Scancode::F2 => Key::F2,
        Scancode::F3 => Key::F3,
        Scancode::F4 => Key::F4,
        Scancode::F5 => Key::F5,
        Scancode::F6 => Key::F6,
        Scancode::F7 => Key::F7,
        Scancode::F8 => Key::F8,
        Scancode::F9 => Key::F9,
        Scancode::F10 => Key::F10,
        Scancode::F11 => Key::F11,
        Scancode::F12 => Key::F12,
        Scancode::Up => Key::Up,
        Scancode::Down => Key::Down,
        Scancode::Left => Key::Left,
        Scancode::Right => Key::Right,
        Scancode::Return => Key::Enter,
        Scancode::Escape => Key::Escape,
        Scancode::Space => Key::Space,
        Scancode::Backspace => Key::Backspace,
        Scancode::Tab => Key::Tab,
        Scancode::LShift => Key::LeftShift,
        Scancode::RShift => Key::RightShift,
        Scancode::LCtrl => Key::LeftCtrl,
        Scancode::RCtrl => Key::RightCtrl,
        Scancode::LAlt => Key::LeftAlt,
        Scancode::RAlt => Key::RightAlt,
        Scancode::Kp0 => Key::Kp0,
        Scancode::Kp1 => Key::Kp1,
        Scancode::Kp2 => Key::Kp2,
        Scancode::Kp3 => Key::Kp3,
        Scancode::Kp4 => Key::Kp4,
        Scancode::Kp5 => Key::Kp5,
        Scancode::Kp6 => Key::Kp6,
        Scancode::Kp7 => Key::Kp7,
        Scancode::Kp8 => Key::Kp8,
        Scancode::Kp9 => Key::Kp9,
        Scancode::KpPlus => Key::KpPlus,
        Scancode::KpMinus => Key::KpMinus,
        Scancode::KpMultiply => Key::KpMultiply,
        Scancode::KpDivide => Key::KpDivide,
        Scancode::KpEnter => Key::KpEnter,
        Scancode::KpPeriod => Key::KpPeriod,
        _ => return None,
    })
}

/// Maps a gamepad face button, by position, to the core's button.
pub fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::South => PadButton::A,
        Button::East => PadButton::B,
        Button::West => PadButton::X,
        Button::North => PadButton::Y,
        _ => return None,
    })
}
```

Run: `cargo test -p deadrally-front-sdl`
Expected: `1 passed` (with dead-code warnings until `main.rs` uses the map).

- [ ] **Step 4: Implement the frontend**

<!-- write: crates/front-sdl/src/main.rs -->
```rust
//! Platform spike candidate: the DeadRally test scene on SDL3 (spec section 7).
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost. Alt+Enter toggles
//! fullscreen, F12 toggles bilinear smoothing, closing the window quits. One stats line per
//! second goes to stdout.

mod keymap;

use std::error::Error;
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioDecision, AudioGate, Pacer, RunStats, letterbox};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, Game, InputEvent, PadAxis};
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FRect, ScaleMode};
use sdl3::video::FullscreenType;

const BYTES_PER_SAMPLE: usize = 2;

struct Options {
    windowed: bool,
    vsync: bool,
}

fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        vsync: true,
    };
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-window" => options.windowed = true,
            "-novsync" => options.vsync = false,
            other => return Err(format!("unknown option {other}; known: -window, -novsync")),
        }
    }
    Ok(options)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options()?;
    sdl3::hint::set("SDL_RENDER_VSYNC", if options.vsync { "1" } else { "0" });

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let gamepads = sdl.gamepad()?;
    let audio = sdl.audio()?;

    let mut window = video.window("DR", 640, 480);
    window.resizable();
    if !options.windowed {
        window.fullscreen();
    }
    let mut canvas = window.build()?.into_canvas();
    let texture_creator = canvas.texture_creator();

    let spec = AudioSpec {
        freq: Some(i32::try_from(AUDIO_SAMPLE_RATE)?),
        channels: Some(i32::try_from(AUDIO_CHANNELS)?),
        format: Some(AudioFormat::s16_sys()),
    };
    let stream = audio
        .open_playback_device(&spec)?
        .open_device_stream(Some(&spec))?;
    stream.resume()?;

    let mut game = Game::new();
    let mut texture_size = (0, 0);
    let mut texture = None;
    let mut rgba = Vec::new();
    let mut samples = Vec::new();
    let mut smooth = false;
    let mut open_pads: Vec<Gamepad> = Vec::new();

    let mut pacer = Pacer::new();
    let mut gate = AudioGate::new();
    let mut stats = RunStats::new();
    let start = Instant::now();
    let mut last = start;
    let mut last_report = start;

    let mut events = sdl.event_pump()?;
    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    scancode: Some(Scancode::Return),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) => {
                    let window = canvas.window_mut();
                    let fullscreen = window.fullscreen_state() != FullscreenType::Off;
                    window.set_fullscreen(!fullscreen)?;
                }
                Event::KeyDown {
                    scancode: Some(Scancode::F12),
                    repeat: false,
                    ..
                } => smooth = !smooth,
                Event::KeyUp {
                    scancode: Some(Scancode::F12),
                    ..
                } => {}
                Event::KeyDown {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => {
                    if let Some(key) = keymap::key(scancode) {
                        game.input(InputEvent::Key { key, pressed: true });
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = keymap::key(scancode) {
                        game.input(InputEvent::Key {
                            key,
                            pressed: false,
                        });
                    }
                }
                Event::GamepadAdded { which, .. } => match gamepads.open(which) {
                    Ok(pad) => open_pads.push(pad),
                    Err(error) => eprintln!("cannot open gamepad: {error}"),
                },
                Event::GamepadRemoved { which, .. } => {
                    open_pads.retain(|pad| pad.id().ok() != Some(which))
                }
                Event::GamepadButtonDown { button, .. } => {
                    if let Some(button) = keymap::pad_button(button) {
                        game.input(InputEvent::PadButton {
                            button,
                            pressed: true,
                        });
                    }
                }
                Event::GamepadButtonUp { button, .. } => {
                    if let Some(button) = keymap::pad_button(button) {
                        game.input(InputEvent::PadButton {
                            button,
                            pressed: false,
                        });
                    }
                }
                Event::GamepadAxisMotion {
                    axis: Axis::LeftX,
                    value,
                    ..
                } => {
                    game.input(InputEvent::PadAxis {
                        axis: PadAxis::StickX,
                        value,
                    });
                }
                Event::GamepadAxisMotion {
                    axis: Axis::LeftY,
                    value,
                    ..
                } => {
                    game.input(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value,
                    });
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let ticks = pacer.advance(nanos(now - last));
        last = now;
        for _ in 0..ticks {
            game.tick();
            samples.clear();
            game.take_audio(&mut samples);
            let queued_frames =
                usize::try_from(stream.queued_bytes()?)? / (AUDIO_CHANNELS * BYTES_PER_SAMPLE);
            if let AudioDecision::Queue { silence_frames } = gate.decide(queued_frames) {
                if silence_frames > 0 {
                    stream.put_data_i16(&vec![0; silence_frames * AUDIO_CHANNELS])?;
                }
                stream.put_data_i16(&samples)?;
            }
        }
        stats.add_ticks(ticks);

        let present_start = Instant::now();
        let frame = game.frame();
        if texture.is_none() || texture_size != (frame.width, frame.height) {
            texture = Some(texture_creator.create_texture_streaming(
                PixelFormat::RGBA32,
                frame.width,
                frame.height,
            )?);
            texture_size = (frame.width, frame.height);
            rgba.resize(frame.pixels.len() * 4, 0);
        }
        let texture = texture.as_mut().expect("created above");
        frame.write_rgba(&mut rgba);
        texture.update(None, &rgba, frame.width as usize * 4)?;
        texture.set_scale_mode(if smooth {
            ScaleMode::Linear
        } else {
            ScaleMode::Nearest
        });

        let (output_width, output_height) = canvas.output_size()?;
        let viewport = letterbox(output_width, output_height, frame.aspect);
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        if viewport.width > 0 && viewport.height > 0 {
            let target = FRect::new(
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
            );
            canvas.copy(texture, None, Some(target))?;
        }
        canvas.present();
        stats.add_present(u32::try_from(present_start.elapsed().as_micros()).unwrap_or(u32::MAX));

        if now - last_report >= Duration::from_secs(1) {
            last_report = now;
            let queued_frames =
                usize::try_from(stream.queued_bytes()?)? / (AUDIO_CHANNELS * BYTES_PER_SAMPLE);
            let audio = gate.report(queued_frames);
            println!(
                "{}",
                stats.line(nanos(now - start), pacer.dropped_ticks(), audio)
            );
        }
    }

    let audio = gate.report(0);
    println!(
        "final {}",
        stats.line(nanos(start.elapsed()), pacer.dropped_ticks(), audio)
    );
    Ok(())
}
```

```bash
cargo clippy -p deadrally-front-sdl --all-targets -- -D warnings
cargo build --release -p deadrally-front-sdl
ls -l target/release/deadrally-sdl && ldd target/release/deadrally-sdl
```

Expected: clean clippy; a binary of about 4.3 MB linking only libc, libm, libgcc_s (SDL3 is static and loads X11, Wayland and audio libraries at run time).

- [ ] **Step 5: Smoke test without a display or speakers**

```bash
mkdir -p captures
SDL_VIDEO_DRIVER=offscreen SDL_AUDIO_DRIVER=disk SDL_DISK_AUDIO_FILE=captures/sdlaudio.raw \
  timeout -s KILL 6 target/release/deadrally-sdl -window -novsync > captures/sdl-smoke.log 2>&1; cat captures/sdl-smoke.log
```

Expected: about five lines like `t=4.002s ticks=280 rate=69.95/s frames=7861 dropped_ticks=0 underruns=1 discarded_ticks=0 queue_ms=40 present_avg_us=492 present_p99_us=1171`: rate 69.9–70.1, `dropped_ticks=0`, `queue_ms` between 25 and 60, underruns 0–2 (the disk driver is not a real device; real numbers come from Task 11). With vsync on, the offscreen driver never returns from present; that is why this smoke test uses `-novsync`.

- [ ] **Step 6: Write the spike helper and run it once on this frontend**

<!-- write: scripts/spike-check.sh -->
```bash
#!/usr/bin/env bash
# Platform spike helper (spec section 7). Runs a frontend in a virtual X server with a virtual
# sound card, so the automated checks need neither a monitor nor speakers.
#
#   scripts/spike-check.sh screens <binary> <out-dir> [soak-seconds]
#       Windowed at 1280x720 and 1024x768: a screenshot of every test-scene mode and of F12
#       smoothing, then a soak run (default 300 s). stats.log gets one cumulative line per second.
#   scripts/spike-check.sh perf <binary> <out-dir>
#       Window resized to 3840x2160, bilinear on, vsync off, 30 s. The last stats line holds the
#       present times.
#
# Needs `scripts/install-linux-deps.sh --local` and a running PipeWire or PulseAudio server.
# Fullscreen and Alt+Enter need a window manager, so they are checked by hand, not here.
set -euo pipefail

usage="usage: $0 screens|perf <binary> <out-dir> [soak-seconds]"
mode=${1:?$usage}
binary=$(realpath "${2:?$usage}")
out=${3:?$usage}
soak=${4:-300}
mkdir -p "$out"

export DISPLAY=:99
Xvfb "$DISPLAY" -screen 0 3840x2160x24 -nolisten tcp &
xvfb_pid=$!
sink=$(pactl load-module module-null-sink sink_name=deadrally_null)
alsa_conf=$(mktemp)
printf '%s\n' '</usr/share/alsa/alsa.conf>' \
    'pcm.!default { type pulse device deadrally_null }' \
    'ctl.!default { type pulse }' > "$alsa_conf"
app_pid=
cleanup() {
    if [ -n "$app_pid" ]; then kill "$app_pid" 2>/dev/null || true; fi
    kill "$xvfb_pid" 2>/dev/null || true
    pactl unload-module "$sink" || true
    rm -f "$alsa_conf"
}
trap cleanup EXIT
sleep 2

launch() {
    # SDL plays through PulseAudio (PULSE_SINK); cpal plays through ALSA (ALSA_CONFIG_PATH).
    SDL_AUDIO_DRIVER=pulseaudio PULSE_SINK=deadrally_null ALSA_CONFIG_PATH="$alsa_conf" \
        "$binary" -window "$@" > "$out/stats.log" 2> "$out/stderr.log" &
    app_pid=$!
    window=$(timeout 20 xdotool search --sync --name '^DR$' | head -n 1)
    xdotool windowfocus --sync "$window"
}
resize() { xdotool windowsize --sync "$window" "$1" "$2"; sleep 1; }
shot() { import -window "$window" "$out/$1.png"; }
press() { xdotool key "$@"; sleep 1; }

case "$mode" in
    screens)
        launch
        resize 1280 720
        shot 1280x720-640x480-nearest
        press Tab
        shot 1280x720-320x200-nearest
        press Tab
        shot 1280x720-640x360-nearest
        resize 1024 768
        shot 1024x768-640x360-nearest
        press Tab F12
        shot 1024x768-640x480-bilinear
        press F12
        sleep "$soak"
        ;;
    perf)
        launch -novsync
        resize 3840 2160
        press F12
        sleep 30
        ;;
    *)
        echo "$usage" >&2
        exit 1
        ;;
esac

if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "error: the frontend exited early; see $out/stderr.log" >&2
    exit 1
fi
tail -n 1 "$out/stats.log"
```

```bash
chmod +x scripts/spike-check.sh
scripts/spike-check.sh screens target/release/deadrally-sdl captures/spike-sdl-smoke 10
ls captures/spike-sdl-smoke
```

Expected: the last stats line, then five PNGs and `stats.log`, `stderr.log`. Open the PNGs (Read tool) and check them by eye: 1280x720 with 4:3 modes shows black bars left and right; 640x360 fills 1280x720; at 1024x768 the 16:9 mode has bars top and bottom; the bilinear shot looks soft, the nearest ones sharp. This script is new tooling: if a step fails (window not found, focus, `import`), fix the script, keep the fix, and note it in the commit body.

- [ ] **Step 7: Commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add Cargo.lock crates/front-sdl scripts/spike-check.sh
git commit -m "feat: add the SDL3 frontend candidate" -m "- SDL3 built from source, linked statically
- letterboxed scaling, F12 smoothing, Alt+Enter
- keyboard, gamepad and queued audio via the gate
- scripts/spike-check.sh for Xvfb runs"
```

---

### Task 10: Pure-Rust frontend candidate

**Files:**
- Create: `crates/front-rust/Cargo.toml`, `crates/front-rust/src/keymap.rs`, `crates/front-rust/src/audio.rs`, `crates/front-rust/src/present.rs`, `crates/front-rust/src/present.wgsl`, `crates/front-rust/src/main.rs`

**Interfaces:**
- Consumes: as Task 9.
- Produces: binary `deadrally-rust [-window] [-novsync]` with the same stats lines and keys as `deadrally-sdl`.

- [ ] **Step 1: Write the failing key-map test**

<!-- write: crates/front-rust/Cargo.toml -->
```toml
[package]
name = "deadrally-front-rust"
description = "Platform spike candidate: DeadRally frontend on winit, wgpu (via pixels), cpal and gilrs."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[[bin]]
name = "deadrally-rust"
path = "src/main.rs"

[dependencies]
cpal = "0.18.2"
deadrally-core = { path = "../core" }
gilrs = "0.11.2"
pixels = "0.17.2"
winit = "0.30.13"

[lints]
workspace = true
```

<!-- write: crates/front-rust/src/main.rs -->
```rust
mod keymap;

fn main() {}
```

<!-- write: crates/front-rust/src/keymap.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_key_is_reachable_exactly_once() {
        // A key the frontend cannot produce could never be bound in the controls menu.
        let mut keys: Vec<Key> = KEYS.iter().map(|&(_, key)| key).collect();
        keys.sort();
        assert_eq!(keys, Key::ALL);
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p deadrally-front-rust`
Expected: compile errors, `cannot find value KEYS`, `use of undeclared type Key`.

- [ ] **Step 3: Implement the key map above the test**

<!-- prepend: crates/front-rust/src/keymap.rs -->
```rust
use deadrally_core::{Key, PadButton};
use gilrs::Button;
use winit::keyboard::KeyCode;

/// Physical winit key codes and the core keys they stand for.
const KEYS: &[(KeyCode, Key)] = &[
    (KeyCode::KeyA, Key::A),
    (KeyCode::KeyB, Key::B),
    (KeyCode::KeyC, Key::C),
    (KeyCode::KeyD, Key::D),
    (KeyCode::KeyE, Key::E),
    (KeyCode::KeyF, Key::F),
    (KeyCode::KeyG, Key::G),
    (KeyCode::KeyH, Key::H),
    (KeyCode::KeyI, Key::I),
    (KeyCode::KeyJ, Key::J),
    (KeyCode::KeyK, Key::K),
    (KeyCode::KeyL, Key::L),
    (KeyCode::KeyM, Key::M),
    (KeyCode::KeyN, Key::N),
    (KeyCode::KeyO, Key::O),
    (KeyCode::KeyP, Key::P),
    (KeyCode::KeyQ, Key::Q),
    (KeyCode::KeyR, Key::R),
    (KeyCode::KeyS, Key::S),
    (KeyCode::KeyT, Key::T),
    (KeyCode::KeyU, Key::U),
    (KeyCode::KeyV, Key::V),
    (KeyCode::KeyW, Key::W),
    (KeyCode::KeyX, Key::X),
    (KeyCode::KeyY, Key::Y),
    (KeyCode::KeyZ, Key::Z),
    (KeyCode::Digit0, Key::Digit0),
    (KeyCode::Digit1, Key::Digit1),
    (KeyCode::Digit2, Key::Digit2),
    (KeyCode::Digit3, Key::Digit3),
    (KeyCode::Digit4, Key::Digit4),
    (KeyCode::Digit5, Key::Digit5),
    (KeyCode::Digit6, Key::Digit6),
    (KeyCode::Digit7, Key::Digit7),
    (KeyCode::Digit8, Key::Digit8),
    (KeyCode::Digit9, Key::Digit9),
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::ArrowLeft, Key::Left),
    (KeyCode::ArrowRight, Key::Right),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Space, Key::Space),
    (KeyCode::Backspace, Key::Backspace),
    (KeyCode::Tab, Key::Tab),
    (KeyCode::ShiftLeft, Key::LeftShift),
    (KeyCode::ShiftRight, Key::RightShift),
    (KeyCode::ControlLeft, Key::LeftCtrl),
    (KeyCode::ControlRight, Key::RightCtrl),
    (KeyCode::AltLeft, Key::LeftAlt),
    (KeyCode::AltRight, Key::RightAlt),
    (KeyCode::Numpad0, Key::Kp0),
    (KeyCode::Numpad1, Key::Kp1),
    (KeyCode::Numpad2, Key::Kp2),
    (KeyCode::Numpad3, Key::Kp3),
    (KeyCode::Numpad4, Key::Kp4),
    (KeyCode::Numpad5, Key::Kp5),
    (KeyCode::Numpad6, Key::Kp6),
    (KeyCode::Numpad7, Key::Kp7),
    (KeyCode::Numpad8, Key::Kp8),
    (KeyCode::Numpad9, Key::Kp9),
    (KeyCode::NumpadAdd, Key::KpPlus),
    (KeyCode::NumpadSubtract, Key::KpMinus),
    (KeyCode::NumpadMultiply, Key::KpMultiply),
    (KeyCode::NumpadDivide, Key::KpDivide),
    (KeyCode::NumpadEnter, Key::KpEnter),
    (KeyCode::NumpadDecimal, Key::KpPeriod),
];

/// Maps a physical winit key to the core's key, if the core knows it.
pub fn key(code: KeyCode) -> Option<Key> {
    KEYS.iter()
        .find(|(candidate, _)| *candidate == code)
        .map(|&(_, key)| key)
}

/// Maps a gamepad face button, by position, to the core's button.
pub fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::South => PadButton::A,
        Button::East => PadButton::B,
        Button::West => PadButton::X,
        Button::North => PadButton::Y,
        _ => return None,
    })
}
```

Run: `cargo test -p deadrally-front-rust`
Expected: `1 passed`.

- [ ] **Step 4: Implement audio, presentation and the frontend**

`pixels`' own scaler only scales by whole multiples and assumes square pixels, so it cannot show 320x200 at 4:3 or switch filters. `Presenter` draws the `pixels` texture into the letterbox viewport with its own nearest or linear sampler.

<!-- write: crates/front-rust/src/audio.rs -->
```rust
use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

/// Interleaved stereo samples waiting to be played; the audio thread pops from the front.
pub type SampleQueue = Arc<Mutex<VecDeque<i16>>>;

/// Opens the default output device at 44.1 kHz stereo and starts playing from `queue`.
/// Missing samples are played as silence. cpal does not resample, so a device that cannot run
/// at 44.1 kHz is an error.
pub fn start(queue: SampleQueue) -> Result<cpal::Stream, Box<dyn Error>> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no audio output device")?;
    let channels = u16::try_from(AUDIO_CHANNELS)?;
    let format = device
        .supported_output_configs()?
        .filter(|config| {
            config.channels() == channels
                && config.min_sample_rate() <= AUDIO_SAMPLE_RATE
                && config.max_sample_rate() >= AUDIO_SAMPLE_RATE
        })
        .map(|config| config.sample_format())
        .min_by_key(|format| match format {
            SampleFormat::I16 => 0,
            SampleFormat::F32 => 1,
            _ => 2,
        })
        .ok_or("the audio device cannot play 44.1 kHz stereo")?;
    let config = StreamConfig {
        channels,
        sample_rate: AUDIO_SAMPLE_RATE,
        buffer_size: BufferSize::Default,
    };
    let stream = match format {
        SampleFormat::I16 => build::<i16>(&device, config, queue)?,
        SampleFormat::F32 => build::<f32>(&device, config, queue)?,
        other => return Err(format!("unsupported sample format {other}").into()),
    };
    stream.play()?;
    Ok(stream)
}

fn build<T: SizedSample + FromSample<i16>>(
    device: &cpal::Device,
    config: StreamConfig,
    queue: SampleQueue,
) -> Result<cpal::Stream, cpal::Error> {
    device.build_output_stream(
        config,
        move |out: &mut [T], _| {
            let mut queue = queue.lock().unwrap_or_else(PoisonError::into_inner);
            for slot in out.iter_mut() {
                *slot = T::from_sample(queue.pop_front().unwrap_or(0));
            }
        },
        |error| eprintln!("audio error: {error}"),
        None,
    )
}
```

<!-- write: crates/front-rust/src/present.wgsl -->
```wgsl
// Draws the frame texture over the whole viewport; the viewport is the letterbox rectangle.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// One triangle that covers the viewport; uv runs 0..1 across it.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    var out: VertexOutput;
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@group(0) @binding(0) var frame_texture: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(frame_texture, frame_sampler, in.uv);
}
```

<!-- write: crates/front-rust/src/present.rs -->
```rust
use deadrally_core::host::Viewport;
use pixels::{Pixels, wgpu};

/// Draws the pixels texture into the letterbox viewport with a nearest or linear sampler.
/// The `pixels` crate's own scaler only scales by whole multiples and assumes square pixels,
/// which cannot show 320x200 at 4:3.
pub struct Presenter {
    pipeline: wgpu::RenderPipeline,
    nearest: wgpu::BindGroup,
    linear: wgpu::BindGroup,
}

impl Presenter {
    /// Builds against the current pixels texture; build a new one after `resize_buffer`, which
    /// replaces the texture.
    pub fn new(pixels: &Pixels<'_>) -> Presenter {
        let device = pixels.device();
        let module = device.create_shader_module(wgpu::include_wgsl!("present.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("deadrally_present_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let view = pixels
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = |filter: wgpu::FilterMode| {
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("deadrally_present_sampler"),
                mag_filter: filter,
                min_filter: filter,
                ..wgpu::SamplerDescriptor::default()
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("deadrally_present_bind_group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        };
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("deadrally_present_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("deadrally_present_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: pixels.render_texture_format(),
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Presenter {
            pipeline,
            nearest: bind_group(wgpu::FilterMode::Nearest),
            linear: bind_group(wgpu::FilterMode::Linear),
        }
    }

    /// Clears `target` to black and draws the frame into `viewport`.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: Viewport,
        smooth: bool,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("deadrally_present_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if viewport.width == 0 || viewport.height == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, if smooth { &self.linear } else { &self.nearest }, &[]);
        pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );
        pass.draw(0..3, 0..1);
    }
}
```

<!-- write: crates/front-rust/src/main.rs -->
```rust
//! Platform spike candidate: the DeadRally test scene on winit, wgpu (via pixels), cpal and
//! gilrs (spec section 7).
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost. Alt+Enter toggles
//! fullscreen, F12 toggles bilinear smoothing, closing the window quits. One stats line per
//! second goes to stdout.

mod audio;
mod keymap;
mod present;

use std::error::Error;
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioDecision, AudioGate, Pacer, RunStats, letterbox};
use deadrally_core::{AUDIO_CHANNELS, Game, InputEvent, PadAxis};
use gilrs::{Axis, EventType, Gilrs};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::audio::SampleQueue;
use crate::present::Presenter;

struct Options {
    windowed: bool,
    vsync: bool,
}

fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        vsync: true,
    };
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-window" => options.windowed = true,
            "-novsync" => options.vsync = false,
            other => return Err(format!("unknown option {other}; known: -window, -novsync")),
        }
    }
    Ok(options)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

/// Converts a gilrs stick value (-1.0..=1.0, up is positive) to the core's range (up is
/// negative).
fn stick(value: f32, invert: bool) -> i16 {
    let value = if invert { -value } else { value };
    (value.clamp(-1.0, 1.0) * 32_767.0) as i16
}

/// Window, GPU surface and presenter, created once the event loop is running.
struct Display {
    window: Arc<Window>,
    pixels: Pixels<'static>,
    presenter: Presenter,
    buffer_size: (u32, u32),
}

struct App {
    options: Options,
    display: Option<Display>,
    error: Option<Box<dyn Error>>,
    game: Game,
    gilrs: Option<Gilrs>,
    queue: SampleQueue,
    _stream: Option<cpal::Stream>,
    samples: Vec<i16>,
    smooth: bool,
    alt_held: bool,
    pacer: Pacer,
    gate: AudioGate,
    stats: RunStats,
    start: Instant,
    last: Instant,
    last_report: Instant,
}

impl App {
    fn create_display(&self, event_loop: &ActiveEventLoop) -> Result<Display, Box<dyn Error>> {
        let mut attributes = Window::default_attributes()
            .with_title("DR")
            .with_inner_size(LogicalSize::new(640, 480));
        if !self.options.windowed {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = Arc::new(event_loop.create_window(attributes)?);
        let size = window.inner_size();
        let frame = self.game.frame();
        let surface =
            SurfaceTexture::new(size.width.max(1), size.height.max(1), Arc::clone(&window));
        let pixels = PixelsBuilder::new(frame.width, frame.height, surface)
            .enable_vsync(self.options.vsync)
            .build()?;
        let presenter = Presenter::new(&pixels);
        Ok(Display {
            window,
            pixels,
            presenter,
            buffer_size: (frame.width, frame.height),
        })
    }

    fn poll_gamepads(&mut self) {
        let Some(gilrs) = &mut self.gilrs else { return };
        while let Some(event) = gilrs.next_event() {
            let input = match event.event {
                EventType::ButtonPressed(button, _) => {
                    keymap::pad_button(button).map(|button| InputEvent::PadButton {
                        button,
                        pressed: true,
                    })
                }
                EventType::ButtonReleased(button, _) => {
                    keymap::pad_button(button).map(|button| InputEvent::PadButton {
                        button,
                        pressed: false,
                    })
                }
                EventType::AxisChanged(Axis::LeftStickX, value, _) => Some(InputEvent::PadAxis {
                    axis: PadAxis::StickX,
                    value: stick(value, false),
                }),
                EventType::AxisChanged(Axis::LeftStickY, value, _) => Some(InputEvent::PadAxis {
                    axis: PadAxis::StickY,
                    value: stick(value, true),
                }),
                _ => None,
            };
            if let Some(input) = input {
                self.game.input(input);
            }
        }
    }

    fn advance(&mut self) {
        let now = Instant::now();
        let ticks = self.pacer.advance(nanos(now - self.last));
        self.last = now;
        for _ in 0..ticks {
            self.game.tick();
            self.samples.clear();
            self.game.take_audio(&mut self.samples);
            let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            if let AudioDecision::Queue { silence_frames } =
                self.gate.decide(queue.len() / AUDIO_CHANNELS)
            {
                queue.extend(std::iter::repeat_n(0, silence_frames * AUDIO_CHANNELS));
                queue.extend(&self.samples);
            }
        }
        self.stats.add_ticks(ticks);

        if now - self.last_report >= Duration::from_secs(1) {
            self.last_report = now;
            println!("{}", self.stats_line(now));
        }
    }

    fn stats_line(&self, now: Instant) -> String {
        let queued_frames = self
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
            / AUDIO_CHANNELS;
        self.stats.line(
            nanos(now - self.start),
            self.pacer.dropped_ticks(),
            self.gate.report(queued_frames),
        )
    }

    fn present(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(display) = &mut self.display else {
            return Ok(());
        };
        let present_start = Instant::now();
        let frame = self.game.frame();
        if display.buffer_size != (frame.width, frame.height) {
            display.pixels.resize_buffer(frame.width, frame.height)?;
            display.presenter = Presenter::new(&display.pixels);
            display.buffer_size = (frame.width, frame.height);
        }
        frame.write_rgba(display.pixels.frame_mut());
        let size = display.window.inner_size();
        let viewport = letterbox(size.width, size.height, frame.aspect);
        let (presenter, smooth) = (&display.presenter, self.smooth);
        display.pixels.render_with(|encoder, target, _| {
            presenter.render(encoder, target, viewport, smooth);
            Ok(())
        })?;
        self.stats
            .add_present(u32::try_from(present_start.elapsed().as_micros()).unwrap_or(u32::MAX));
        Ok(())
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Box<dyn Error>) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.display.is_none() {
            match self.create_display(event_loop) {
                Ok(display) => self.display = Some(display),
                Err(error) => self.fail(event_loop, error),
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(display) = &mut self.display
                    && size.width > 0
                    && size.height > 0
                    && let Err(error) = display.pixels.resize_surface(size.width, size.height)
                {
                    self.fail(event_loop, error.into());
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.alt_held = modifiers.state().alt_key(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                match code {
                    KeyCode::Enter if pressed && self.alt_held => {
                        if !repeat && let Some(display) = &self.display {
                            let fullscreen = display.window.fullscreen().is_some();
                            display.window.set_fullscreen(
                                (!fullscreen).then_some(Fullscreen::Borderless(None)),
                            );
                        }
                    }
                    KeyCode::F12 => {
                        if pressed && !repeat {
                            self.smooth = !self.smooth;
                        }
                    }
                    _ if repeat => {}
                    _ => {
                        if let Some(key) = keymap::key(code) {
                            self.game.input(InputEvent::Key { key, pressed });
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.present() {
                    self.fail(event_loop, error);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.poll_gamepads();
        self.advance();
        if let Some(display) = &self.display {
            display.window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options()?;
    let queue = SampleQueue::default();
    let stream = audio::start(Arc::clone(&queue))
        .inspect_err(|error| eprintln!("no audio: {error}"))
        .ok();
    let gilrs = Gilrs::new()
        .inspect_err(|error| eprintln!("no gamepads: {error}"))
        .ok();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let now = Instant::now();
    let mut app = App {
        options,
        display: None,
        error: None,
        game: Game::new(),
        gilrs,
        queue,
        _stream: stream,
        samples: Vec::new(),
        smooth: false,
        alt_held: false,
        pacer: Pacer::new(),
        gate: AudioGate::new(),
        stats: RunStats::new(),
        start: now,
        last: now,
        last_report: now,
    };
    event_loop.run_app(&mut app)?;
    println!("final {}", app.stats_line(Instant::now()));
    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
```

```bash
cargo clippy -p deadrally-front-rust --all-targets -- -D warnings
cargo build --release -p deadrally-front-rust
ls -l target/release/deadrally-rust && ldd target/release/deadrally-rust
```

Expected: clean clippy; a binary of about 13 MB that also links `libudev.so.1` and `libasound.so.2`.

- [ ] **Step 5: Smoke test under Xvfb**

```bash
scripts/spike-check.sh screens target/release/deadrally-rust captures/spike-rust-smoke 10
```

Expected: as in Task 9 Step 6, a final stats line with `rate=` about 70 and five screenshots with the same bar geometry. If `stderr.log` says wgpu found no adapter, check `vulkaninfo --summary` under `DISPLAY=:99`. `mesa-vulkan-drivers` (from `--local`) provides a software Vulkan device.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add Cargo.lock crates/front-rust
git commit -m "feat: add the pure-Rust frontend candidate" -m "- winit window and input, gilrs gamepads
- pixels surface with a letterboxing wgpu presenter
- cpal output fed through the shared audio gate"
```

---

### Task 11: Evaluate the spike and decide

**Files:**
- Create: `docs/adr/0001-platform-layer.md`
- Delete: the losing `crates/front-*`
- Move: the winning `crates/front-*` to `crates/deadrally/`
- Modify (only if `front-rust` wins): `scripts/install-linux-deps.sh`

**Interfaces:**
- Produces: package and binary `deadrally` (Tasks 12 and 13 refer to `cargo run -p deadrally`).

- [ ] **Step 1: Automated checks for both frontends** (about 12 minutes)

```bash
for f in sdl rust; do
  scripts/spike-check.sh screens target/release/deadrally-$f captures/spike-$f 300
  scripts/spike-check.sh perf target/release/deadrally-$f captures/perf-$f
done
```

Then verify the bar geometry from the screenshots instead of by eye:

```bash
python3 - <<'EOF'
from PIL import Image
def black(img, box):
    return all(p[:3] == (0, 0, 0) for p in img.crop(box).getdata())
checks = {
    "1280x720-640x480-nearest": [((0, 0, 160, 720), True), ((1120, 0, 1280, 720), True), ((160, 0, 1120, 720), False)],
    "1280x720-320x200-nearest": [((0, 0, 160, 720), True), ((1120, 0, 1280, 720), True), ((160, 0, 1120, 720), False)],
    "1280x720-640x360-nearest": [((0, 0, 1280, 720), False)],
    "1024x768-640x360-nearest": [((0, 0, 1024, 96), True), ((0, 672, 1024, 768), True), ((0, 96, 1024, 672), False)],
}
for f in ("sdl", "rust"):
    for name, boxes in checks.items():
        img = Image.open(f"captures/spike-{f}/{name}.png").convert("RGB")
        ok = all(black(img, box) == expected for box, expected in boxes)
        print(f, name, img.size, "ok" if ok else "WRONG")
EOF
```

Expected: eight `ok` lines with sizes `(1280, 720)` or `(1024, 768)`. From each `captures/spike-*/stats.log` take the line at `t=60` and the last line: `rate` must be 69.93–70.07 (±0.1 %). `underruns` must not grow after `t=1`, and `queue_ms` must stay below about 120 (bounded) for the whole soak. From `captures/perf-*/stats.log` take `present_avg_us` and `present_p99_us` of the last line. Under Xvfb this is software rendering: compare the two frontends, not absolute numbers.

- [ ] **Step 2: Static measurements**

```bash
for f in sdl rust; do
  echo "$f loc: $(cat crates/front-$f/src/* | grep -cvE '^\s*(//.*)?$')"
  echo "$f crates: $(cargo tree --locked -p deadrally-front-$f -e normal --prefix none | sed 's/ (\*)$//' | sort -u | wc -l)"
done
cargo clean --release && /usr/bin/time -f "sdl build: %e s" cargo build -q --release --locked -p deadrally-front-sdl
cargo clean --release && /usr/bin/time -f "rust build: %e s" cargo build -q --release --locked -p deadrally-front-rust
cargo build -q --release --locked --workspace
stat -c "%n %s bytes" target/release/deadrally-sdl target/release/deadrally-rust
```

Reference values from the plan's verification run: lines of code 339 (SDL) and 608 (Rust, including the shader); crates 6 (SDL) and 135 (Rust); binary sizes 4.3 MB and 13.1 MB. Record what you measure, not these. If CI has run, take the macOS and Windows binary sizes from the artifacts too.

- [ ] **Step 3: [OWNER] Hands-on check at this machine's monitor**

Ask the owner (in Czech) to sit at the Linux machine's own screen, open a terminal on its desktop, and for each binary run `cd ~/DeadRally && target/release/deadrally-sdl` and then `target/release/deadrally-rust` (fullscreen by default; quit with Alt+F4). Give them this checklist to answer per frontend:

1. Starts fullscreen at the desktop resolution; 640x480 mode has black bars left and right, and the grid cells look square.
2. Tab cycles three modes: 320x200 fills the same 4:3 area; 640x360 is 16:9 with bars top and bottom (or none on a 16:9 screen).
3. F12 switches sharp and smooth; Alt+Enter goes to a window and back to fullscreen.
4. Keys light their grid cells. With a gamepad: the stick moves the red dot, the four face buttons light the last four cells, unplugging and replugging works.
5. Listen for five minutes: a steady quiet tone, no crackles or gaps; each key press clicks without noticeable delay; T silences the tone.

Write down their answers per item and frontend.

- [ ] **Step 4: [OWNER] The same check on the Mac** (needs CI artifacts; skip and record "not manually verified on macOS" if CI has not run)

Artifacts `deadrally-macOS-ARM64` (and `deadrally-macOS-X64` for Intel Macs) from the latest CI run. On the Mac: unzip, `xattr -d com.apple.quarantine deadrally-sdl deadrally-rust`, `chmod +x`, run each, same checklist, quit with Cmd+Q. Windows only if the owner has a Windows machine (artifact `deadrally-Windows-X64`); otherwise record "not manually verified on Windows". Wayland is recorded as "not manually verified" unless the owner tests a Wayland session.

- [ ] **Step 5: Decide by the spec's rule** (spec section 7)

A frontend that fails any must-item loses. If both pass, the measurements decide; a tie goes to `front-rust`. If neither passes, stop and discuss with the owner. Tell the owner the outcome and the reasons in Czech before writing the ADR.

- [ ] **Step 6: Write the ADR** (fill every cell with what you measured or observed; write "not run" where a check could not run)

<!-- write: docs/adr/0001-platform-layer.md -->
```markdown
# ADR 0001: Platform layer

- **Date:** YYYY-MM-DD
- **Status:** accepted
- **Spec:** `docs/superpowers/specs/2026-10-03-m0-foundations-design.md`, section 7

## Context

DeadRally needs a window, fullscreen, scaling, keyboard, gamepad and audio on Windows, macOS and Linux. The owner preferred a pure-Rust stack; SDL3 was the safer bet. Both candidates were built against the same core (`deadrally_core::host` paces ticks and gates audio identically) and judged by the same checklist.

## Candidates

- **front-sdl:** `sdl3` 0.20 (SDL 3.4.16 built from source, linked statically); SDL renderer, streaming texture.
- **front-rust:** `winit` 0.30, `pixels` 0.17 (wgpu 29) with an own letterboxing presenter, `cpal` 0.18, `gilrs` 0.11.

## Must-pass checklist

| # | Item | front-sdl | front-rust | Evidence and who checked |
|---|---|---|---|---|
| 1 | Aspect-correct scaling with bars; F12 bilinear | | | |
| 2 | Fullscreen at desktop resolution; `-window`; Alt+Enter | | | |
| 3 | Keyboard by physical key; gamepad stick, 4 buttons, hot-plug | | | |
| 4 | 5 min audio, no underruns after 1 s, bounded queue | | | |
| 5 | 70 ticks/s ± 0.1 % over 60 s; capped catch-up | | | |
| 6 | `cargo build --release` on Linux, macOS, Windows CI | | | |

## Measurements

| Measure | front-sdl | front-rust |
|---|---|---|
| Lines of code (no blanks or comments) | | |
| Crates in `cargo tree` (deduplicated) | | |
| Clean release build, this machine | | |
| Release binary size: Linux / macOS / Windows | | |
| System packages needed to build on Linux | cmake, X11/Wayland/audio headers (`scripts/install-linux-deps.sh`) | `pkg-config`, `libasound2-dev`, `libudev-dev` |
| Present avg / p99 at 3840x2160, bilinear, Xvfb software rendering | | |

## Decision

**front-… wins**, because …

## Consequences

- The losing frontend was deleted in commit `…`; its code stays in history.
- Not manually verified: … (for example Windows, Wayland).
- …
```

```bash
git add docs/adr/0001-platform-layer.md
git commit -m "docs: record the platform layer decision"
```

- [ ] **Step 7: Delete the loser** (`LOSER` is `sdl` or `rust`)

```bash
git rm -rq crates/front-LOSER
cargo build --workspace
git add Cargo.lock
```

If **front-rust won**, SDL's build dependencies are no longer needed. Replace the package list in `scripts/install-linux-deps.sh` with:

```bash
packages=(build-essential pkg-config libasound2-dev libudev-dev)
```

and the comment above it with `# Installs the system packages DeadRally needs on Debian, Ubuntu or Mint. CI and developers use this same list. cpal links ALSA and gilrs links udev.` (keep the `--local` tools). Then:

```bash
git add scripts/install-linux-deps.sh   # only if changed
git commit -m "refactor: remove the losing frontend"
```

- [ ] **Step 8: Make the winner the game** (`WINNER` is `sdl` or `rust`)

```bash
git mv crates/front-WINNER crates/deadrally
```

In `crates/deadrally/Cargo.toml` set `name = "deadrally"`, `description = "DeadRally: the game's window, input and sound around deadrally-core."`, and in `[[bin]]` set `name = "deadrally"`. In `crates/deadrally/src/main.rs` replace the first doc line `//! Platform spike candidate: the DeadRally test scene on …` with `//! DeadRally: the game's frontend (see docs/adr/0001-platform-layer.md). In M0 it runs the test scene on …`, keeping the rest of that sentence. Then:

```bash
cargo build --release --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/spike-check.sh screens target/release/deadrally captures/final 5
git add -A crates Cargo.lock
git commit -m "refactor: make the winning frontend the game"
```

Expected: everything green; `target/release/deadrally` exists.

---

### Task 12: Contributor docs and brief update

**Files:**
- Create: `CLAUDE.md`, `CONTRIBUTING.md`
- Modify: `README.md`, `docs/PROJECT_BRIEF.md`

- [ ] **Step 1: Write `CLAUDE.md`**

<!-- write: CLAUDE.md -->
```markdown
# DeadRally: instructions for AI agents

DeadRally is a clean, native reimplementation of *Death Rally* (Remedy, 2009) in Rust. Read `docs/PROJECT_BRIEF.md` for the goal and `docs/superpowers/specs/` for the current design. `CONTRIBUTING.md` has the setup.

## Ground rules (brief §2)

1. **Faithfulness first.** Anything that changes how the game plays (timings, physics, prices, AI) must match the original first. Improvements come later, as options that default to the original behaviour.
2. **Never commit game data:** BPA, HAF, the original exe or DLLs, saves, sound, music, or screenshots that are mostly original art. `.gitignore` and `scripts/check-no-game-data.sh` (run in CI) enforce this. Never `git add -f` such files.
3. **Provenance.** New code is ours (GPL-3.0). Facts, file formats and constants from DreeRally or dRally are fine: describe them in your own words and credit them. Code copied from dRally (MIT) keeps its notice. Do not paste decompiled DreeRally code; re-implement from understanding. When unsure, ask the owner.
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
| `cargo run --release -p deadrally -- -window` | the game (M0: the test scene) |
| `scripts/spike-check.sh screens target/release/deadrally captures/x 10` | screenshots and stats without a monitor (Xvfb, null sink) |

## Tests

- Tests encode **why**: the name or a comment says what goes wrong for a player if the behaviour changes.
- `#[ignore]` is only for tests that need game data: `#[ignore = "needs game data (DEADRALLY_DATA)"]`. They read the data through `DEADRALLY_DATA` and fail when it is unset.
- Fixtures are generated by the tests in temporary directories. Never commit files derived from game data.
- "Done" means verified. Say which checks ran, and say so when one could not run (CI never runs `cargo test-data`).

## Commits

- Prefixes `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `ci:`, `build:`.
- Subject at most 50 characters, imperative, English. A body only when several things changed, as a `- ` list.
- No Co-Authored-By or any other attribution.

## Working as an agent (brief §11)

- Work from a written task: goal, files, evidence required.
- One git worktree per task (under `.worktrees/`, which is ignored). Merge only after an independent review.
- Re-run the key checks yourself before reporting success. Checks that silently did not run are the most common false "done".
```

- [ ] **Step 2: Write `CONTRIBUTING.md`** (use the setup lines for the frontend that won; the other variant is shown after the block)

<!-- write: CONTRIBUTING.md -->
````markdown
# Contributing to DeadRally

Thank you for helping. Read [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) first; this file covers the practical side.

## Rules

1. **Faithfulness first.** Gameplay must match the original before anything is improved; improvements are options that default to the original behaviour.
2. **No game data in the repository**, ever: no BPA or HAF files, no executables or DLLs, no saves, sound, music, or screenshots of original art. CI rejects tracked files that match `.gitignore`.
3. **Know where code comes from.** Our code is GPL-3.0. Facts and formats from [DreeRally](https://github.com/victortrnka/DreeRally/tree/0.4.x) and [dRally](https://github.com/urxp/dRally) are welcome with credit; copied dRally code keeps its MIT notice; do not paste decompiled DreeRally code.
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
````

If **front-rust** won, replace the macOS and Windows lines with:

```markdown
- **macOS:** Xcode command line tools (`xcode-select --install`).
- **Windows:** Visual Studio Build Tools with the "Desktop development with C++" workload.
```

- [ ] **Step 3: Replace `README.md`**

<!-- write: README.md -->
````markdown
# DeadRally

Original Death Rally reincarnation for modern systems: a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) for Windows, macOS and Linux, written in Rust.

**Status:** M0, repository foundations. The game shows a test scene; nothing is playable yet. [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) covers the goal, the approach and the roadmap.

The repository contains no game data. You need your own copy of the game: Death Rally (Classic) on Steam (free) or Remedy's 2009 freeware release.

## Quick start

```
scripts/install-linux-deps.sh               # Linux; see CONTRIBUTING.md for macOS and Windows
export DEADRALLY_DATA=~/games/DeathRally    # your copy of the game
cargo run -p deadrally-headless -- check-data
cargo run --release -p deadrally -- -window
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the full setup, the data configuration and the rules.

Licence: GPL-3.0, see [LICENSE](LICENSE).
````

- [ ] **Step 4: Update the brief** (`docs/PROJECT_BRIEF.md`; each change replaces one exact passage)

**4a.** Section 5, first paragraph. Replace:

```text
Language and libraries are open decisions (section 11). The recommendation is **C++20, SDL3 and CMake**: SDL covers window, input, gamepad and audio on all three platforms, and C++ allows clean types while staying close to the reference C code.
```

with:

```text
**Decided in M0:** Rust (edition 2024) in a Cargo workspace; the platform layer was chosen by a spike recorded in `docs/adr/0001-platform-layer.md`. The modules below map onto crates: `deadrally-core` holds the deterministic game (the gfx, ui, game, race and audio logic) and has no platform dependencies; `deadrally-gamedata` reads the original data (assets); `deadrally` is the frontend (platform); `deadrally-headless` and future tools cover the rest (tools).
```

**4b.** Section 7, the M0 row. Replace:

```text
CMake builds on Win/macOS/Linux in CI; the asset path is configurable; code style is agreed; the CLAUDE.md/CONTRIBUTING are written; the game data is detected and validated.
```

with:

```text
The Cargo workspace builds and tests on Win/macOS/Linux in CI; the core has no platform dependencies and is deterministic across OSes; the asset path is configurable; code style is agreed (rustfmt, clippy); the CLAUDE.md/CONTRIBUTING are written; the game data is detected and validated; the platform layer is chosen (ADR 0001).
```

**4c.** Section 8, last bullet. Replace:

```text
Run memory checks (ASan/UBSan) from day one.
```

with:

```text
In Rust, safe code rules out this class of bug: keep `unsafe` forbidden and overflow checks on.
```

**4d.** Section 10, step 4. Replace:

```text
Set up the DeadRally skeleton for M0: CMake, SDL, CI matrix (windows-latest, macos-latest, ubuntu-latest), formatting/lint, ASan/UBSan in debug, an asset-path setting, and a `CLAUDE.md` / `CONTRIBUTING.md` with the rules from section 2.
```

with:

```text
Set up the DeadRally skeleton for M0: a Cargo workspace, the platform spike (ADR 0001), CI matrix (windows-latest, macos-latest, ubuntu-latest), rustfmt and clippy, overflow checks, an asset-path setting, and a `CLAUDE.md` / `CONTRIBUTING.md` with the rules from section 2. Done; see `docs/superpowers/specs/2026-10-03-m0-foundations-design.md`.
```

**4e.** Section 12, the table. Change the header row to `| Decision | Recommendation or decision |`, replace the `Language`, `Platform layer` and `Renderer` rows, and add an `Order of work` row after `Reuse policy`:

```text
| Language | **Decided: Rust** (2026-10-03; C++20 and C11 were the alternatives) |
| Platform layer | **Decided by spike:** see `docs/adr/0001-platform-layer.md` |
| Renderer | indexed framebuffers, converted to RGBA by the core and uploaded as a texture by the frontend |
| Order of work | **Decided:** M0, then sub-project B (DreeRally build and Wine runs of the oracle on Linux), then M1 |
```

Verify nothing of the old stack is left:

```bash
grep -nE "C\+\+20|CMake|ASan|UBSan" docs/PROJECT_BRIEF.md
```

Expected: only the Language row's "C++20 and C11 were the alternatives".

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md CONTRIBUTING.md README.md docs/PROJECT_BRIEF.md
git commit -m "docs: add contributor docs and update the brief" -m "- CLAUDE.md for agents, CONTRIBUTING.md for people
- README with status and quick start
- brief: Rust workspace, ADR 0001, no ASan/CMake"
```

---

### Task 13: Final verification and merge

**Files:** none (verification only; a config file in the owner's home with consent)

- [ ] **Step 1: Run everything locally**

```bash
scripts/check-no-game-data.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
DEADRALLY_DATA=~/games/DeathRally cargo test-data
cargo run -q --release -p deadrally-headless -- run --ticks 70
```

Expected: all green; the hash line from Task 7 Step 4 unchanged.

- [ ] **Step 2: Check the data report through all three sources** (spec section 16, item 3)

```bash
cargo run -q -p deadrally-headless -- check-data --data ~/games/DeathRally; echo "exit=$?"
DEADRALLY_DATA=~/games/DeathRally cargo run -q -p deadrally-headless -- check-data; echo "exit=$?"
```

Expected: `source: command line (--data)` then `source: environment (DEADRALLY_DATA)`, both `known version`, `exit=0`.

For the config file, ask the owner (in Czech) whether DeadRally may create `~/.config/deadrally/config.toml` with `data_path = "~/games/DeathRally"`. If yes:

```bash
mkdir -p ~/.config/deadrally
printf 'data_path = "~/games/DeathRally"\n' > ~/.config/deadrally/config.toml
env -u DEADRALLY_DATA cargo run -q -p deadrally-headless -- check-data; echo "exit=$?"
```

Expected: `config file: /home/trashcan/.config/deadrally/config.toml`, `source: config file (...)`, `known version`, `exit=0`. If the owner declines, record "config source checked by tests only".

- [ ] **Step 3: Independent review**

Use superpowers:requesting-code-review on the whole branch (`git diff master...m0-foundations`) against the spec. Fix what it finds (one commit per fix, `fix:` or `refactor:`), re-running Step 1.

- [ ] **Step 4: [OWNER] Push and CI**

With the owner's consent (and GitHub access from Task 8 Step 5): `git push -u origin m0-foundations`. Expected on GitHub Actions: `lint`, `test` (3 OSes), `macos-intel`, `core-purity`, `determinism` all green, and the determinism job prints three identical lines. If a job fails, fix it on the branch and push again.

- [ ] **Step 5: [OWNER] Merge**

Ask the owner how to merge (in Czech): a pull request on GitHub, or locally with `git switch master && git merge --no-ff m0-foundations && git push`. After the merge, confirm the CI run on `master` is green (spec section 16, item 1). Report to the owner: what is done and verified, what was not manually verified (from the ADR), and the next step (sub-project B, the oracle on Linux).
