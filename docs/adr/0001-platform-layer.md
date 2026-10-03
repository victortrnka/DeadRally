# ADR 0001: Platform layer

- **Date:** 2026-10-03
- **Status:** accepted
- **Spec:** `docs/superpowers/specs/2026-10-03-m0-foundations-design.md`, section 7

## Context

DeadRally needs a window, fullscreen, scaling, keyboard, gamepad and audio on Windows, macOS and Linux. The owner preferred a pure-Rust stack; SDL3 was the safer bet. Both candidates were built against the same core (`deadrally_core::host` paces ticks and gates audio identically) and judged by the same checklist.

## Candidates

- **front-sdl:** `sdl3` 0.20 (SDL 3.4.16 built from source, linked statically); SDL renderer, streaming texture.
- **front-rust:** `winit` 0.30, `pixels` 0.17 (wgpu 29) with an own letterboxing presenter, `cpal` 0.18, `gilrs` 0.11.

## Must-pass checklist

Automated runs used `scripts/spike-check.sh` (Xvfb, PipeWire null sink) on the owner's Linux Mint 22.3 machine. The owner's hands-on runs used the same machine's monitor (X11, Radeon HD 7970): short runs, no gamepad, no listening.

| # | Item | front-sdl | front-rust | Evidence and who checked |
|---|---|---|---|---|
| 1 | Aspect-correct scaling with bars; F12 bilinear | pass | pass | Claude: Xvfb screenshots, bar geometry 4/4 each, F12 shot soft; owner: SDL at the monitor |
| 2 | Fullscreen at desktop resolution; `-window`; Alt+Enter | pass | **fail** | Owner: SDL fine; front-rust froze on Alt+Enter from fullscreen (its log stops without the final line) |
| 3 | Keyboard by physical key; gamepad stick, 4 buttons, hot-plug | keyboard pass; gamepad not verified | keyboard pass; gamepad not verified | Owner (SDL keys); Claude (Tab and F12 via xdotool, both); no gamepad available |
| 4 | 5 min audio, no underruns after 1 s, bounded queue | open: 1 underrun at t≈223 s; queue 3–50 ms | 0 underruns after the screenshot phase; queue 68–132 ms | Claude: 300 s soak on a virtual sink; real hardware (27 s runs) showed underruns only at start and at a window-mode stall; nobody listened |
| 5 | 70 ticks/s ± 0.1 % over 60 s; capped catch-up | pass (70.007/s) | pass (70.007/s) | Claude: soak logs, window t≈60→120 s |
| 6 | `cargo build --release` on Linux, macOS, Windows CI | Linux pass; CI not run | Linux pass; CI not run | No GitHub access on the build machine yet |

## Measurements

| Measure | front-sdl | front-rust |
|---|---|---|
| Lines of code (no blanks or comments) | 339 | 608 (including the shader) |
| Crates in `cargo tree` (deduplicated) | 6 | 135 |
| Clean release build, this machine (Xeon E5-1680 v2) | 104 s | 83 s |
| Release binary size: Linux / macOS / Windows | 4.8 MB / not measured / not measured | 13.1 MB / not measured / not measured |
| System packages needed to build on Linux | cmake, X11/Wayland/audio headers (`scripts/install-linux-deps.sh`) | `pkg-config`, `libasound2-dev`, `libudev-dev` |
| Present avg / p99 at 3840x2160, bilinear, Xvfb software rendering | 11.8 / 15.7 ms | 21.2 / 27.1 ms |
| Steady audio queue (latency before the device buffer) | 3–50 ms | 68–132 ms |

## Decision

**front-sdl wins.** front-rust failed must-item 2: it froze on Alt+Enter from fullscreen. The likely cause is in `pixels`: `Pixels::render_with` retries `get_current_texture` in a loop, reconfiguring the surface to its stored (still fullscreen) size after each `Outdated`, so the window's `Resized` event is never processed. Fixing that means patching or replacing `pixels`, which the spec counts as a failure. Beyond that, SDL leads on dependencies, binary size, code size, audio latency and rendering speed; only the clean build is faster for front-rust. The owner's preference for pure Rust would have broken a tie; there was none.

## Consequences

- front-rust was deleted in the commit `refactor: remove the losing frontend`; its code stays in history. front-sdl became `crates/deadrally` (package and binary `deadrally`).
- SDL3 is compiled from source: building needs cmake everywhere and the X11/Wayland/audio headers on Linux. The binary links only the C runtime; SDL loads display and audio libraries at run time.
- Not manually verified: audio by ear (any platform), gamepads, macOS, Windows, Wayland, and the CI builds. They are verified once CI runs and the owner can listen on the Mac.
- Open risk: SDL's steady audio queue sits at 3–50 ms, and one underrun happened in a 5-minute soak on a virtual sink. Revisit the audio gate's margin (prime to the full target, or a smaller SDL device buffer via `SDL_AUDIO_DEVICE_SAMPLE_FRAMES`) when real music arrives, or earlier if the owner hears glitches.
