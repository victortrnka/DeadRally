# ADR 0001: Platform layer

- **Date:** 2026-10-04
- **Status:** accepted by the owner on 2026-10-04, with must-items 3 (gamepad) and 6 (CI builds) still to be verified
- **Spec:** `docs/superpowers/specs/2026-10-03-m0-foundations-design.md`, section 7

## Context

DeadRally needs a window, fullscreen, scaling, keyboard, gamepad and audio on Windows, macOS and Linux. The owner preferred a pure-Rust stack; SDL3 was the safer bet. Both candidates were built against the same core (`deadrally_core::host` paces ticks and gates audio identically) and judged by the same checklist.

## Candidates

- **front-sdl:** `sdl3` 0.20 (SDL 3.4.16 built from source, linked statically); SDL renderer, streaming texture.
- **front-rust:** `winit` 0.30, `wgpu` 29 with its own surface handling and letterboxing presenter, `cpal` 0.18, `gilrs` 0.11. The first version used `pixels` 0.17 for the surface (see History).

## How it was checked

All on the owner's Linux Mint 22.3 machine (Xeon E5-1680 v2, Radeon HD 7970, RADV).

- **Screenshots and soak:** `scripts/spike-check.sh` (Xvfb, PipeWire null sink): bar geometry, F12, a 300 s soak and present times at 3840x2160.
- **Fullscreen on the real GPU:** `scripts/fullscreen-check.sh` (headless Weston with Xwayland/DRI3, four fullscreen toggles through the window manager).
- **Hands-on:** the owner at the machine's monitor, short runs, no gamepad, no listening.

## Must-pass checklist (final round)

| # | Item | front-sdl | front-rust | Evidence |
|---|---|---|---|---|
| 1 | Aspect-correct scaling with bars; F12 bilinear | pass | pass | Xvfb screenshots, bar geometry 4/4 each; owner at the monitor (SDL) |
| 2 | Fullscreen at desktop resolution; `-window`; Alt+Enter | pass | pass (after the fix below) | `fullscreen-check.sh` 4/4 each on RADV; owner at the monitor (SDL) |
| 3 | Keyboard by physical key; gamepad stick, 4 buttons, hot-plug | keyboard pass; **gamepad not verified** | keyboard pass; **gamepad not verified** | owner (SDL keys), xdotool (both); no gamepad available |
| 4 | 5 min audio, no underruns after 1 s, bounded queue | pass | pass | 300 s soak, after the drift fix: 0 underruns after the start, flat queue (25–47 ms vs 49–103 ms); nobody has listened yet |
| 5 | 70 ticks/s ± 0.1 % over 60 s; capped catch-up | pass (70.010/s) | pass (70.006/s) | soak window t≈60→120 s |
| 6 | `cargo build --release` on Linux, macOS, Windows CI | Linux pass; **CI not run** | Linux pass; **CI not run** | the build machine has no GitHub access yet |

## Measurements (final round)

| Measure | front-sdl | front-rust |
|---|---|---|
| Lines of code (no blanks or comments) | **339** | 739 (including the shader) |
| Crates in `cargo tree` (deduplicated) | **6** | 131 |
| Clean release build on this machine | 106 s | **84 s** |
| Release binary size on Linux (macOS and Windows not measured) | **4.8 MB** | 13.0 MB |
| Audio queue, mean (latency before the device buffer) | **36 ms** | 74 ms |
| Present avg / p99 at 3840x2160, bilinear, Xvfb software rendering | **12.2 / 16.2 ms** | 26.1 / 35.6 ms |
| System packages needed to build on Linux | cmake, X11/Wayland/audio headers | `pkg-config`, `libasound2-dev`, `libudev-dev` |

## Decision

**front-sdl.** In the final round both candidates pass every must-item that could be verified, so by the spec's rule the measurements decide. SDL leads on five of six: dependencies, binary size, code size, audio latency and rendering speed. Only the clean build is faster for front-rust. This is not a tie, so the owner's preference for pure Rust, the agreed tie-breaker, does not apply. The owner accepted the outcome on 2026-10-04 and noted that a full-Rust stack is still attractive.

## History

1. **First round (2026-10-03).** front-rust froze on Alt+Enter at the owner's monitor, and SDL was declared the winner. The decision was made before the owner had answered, and it claimed without proof that the freeze could only be fixed in a dependency. The owner asked for a rematch.
2. **The freeze, root cause.** It was reproduced on the real GPU with `scripts/fullscreen-check.sh`; Xvfb's software driver never shows it. A gdb stack showed the main thread busy inside `pixels::Pixels::render_with` → `reconfigure_surface`. After the window manager resizes the window, RADV answers "out of date". `pixels` reconfigures the surface to its stored, now stale, size and retries in a loop, so the resize event that would update the size is never handled. Fix: front-rust owns its wgpu surface, reconfigures to the window's current size and skips the frame.
3. **Audio drift (found in review).** Both soaks showed the queue drifting by about 55–62 ppm, the sound card's clock against the system clock. The shared audio gate now holds the averaged queue at its settled length by repeating or dropping one frame per tick. It is tested by simulating a card that is 100 ppm fast or slow.

## Consequences

- front-rust was removed from the tree in the commit `refactor: remove the losing frontend` after this decision. Its fixed version stays in history (`git show 76fe556:crates/front-rust/src/present.rs`), so a full-Rust frontend can be revived later. The core has no platform dependencies, which keeps that cheap.
- SDL3 is compiled from source: building needs cmake everywhere and the X11/Wayland/audio headers on Linux. The binary links only the C runtime; SDL loads display and audio libraries at run time.
- Still to verify: gamepads, listening on real speakers, the CI builds, and macOS, Windows and Wayland by hand.
