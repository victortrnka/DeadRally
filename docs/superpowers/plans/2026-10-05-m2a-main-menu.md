# M2a Main Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After the title, DeadRally shows the original's main menu: the fade from the title, the animated palette and cursor, the bottom message panel, the highlight moved by keyboard and gamepad, the start submenu, the credits and the exit question with the end screen; text drawn glyph for glyph from the player's own `dr.exe`; every screen and sound measured to match the original under Wine.

**Architecture:** `deadrally-gamedata` gains a PE reader (`exe`) and the strings and font metrics at fixed addresses of `dr.exe` (`text`), and the menu's pictures, palettes, effects and texts in `Assets`. `deadrally-core` gains an indexed 640x480 `canvas`, bitmap `font`s, the original's one-key input (`keys`: set-1 scancodes, SDL 1.2's repeat, the joystick path of `eventDetected`) and the `menu` scene, a state machine whose states are the waits of the original's straight-line code; the startup hands over to it, and `Game::quit_requested` ends the frontend. `deadrally-headless` presses named keys, so `find` and `render-audio` follow a scenario through the menus.

**Tech Stack:** Rust 1.99.0 (edition 2024); no new crates. For reference runs only: Wine 9 with a 32-bit prefix, Xvfb, xdotool, ImageMagick, `pactl` and `parec`.

**Spec:** `docs/superpowers/specs/2026-10-05-m2a-main-menu-design.md` (written without the owner, who asked for M2 to go on without questions; Task 7 brings in what the reference runs measured). Read it first; this plan argues from it.

## Global Constraints

- Toolchain pinned in `rust-toolchain.toml` (`1.99.0`), edition 2024; `unsafe_code = "forbid"`; overflow checks in every profile. No new dependencies, no new lint exceptions (no `#[allow]`).
- The core's determinism bans stay (`crates/core/clippy.toml`: no clocks, threads, environment reads, `HashMap`, libm transcendental functions). The menu palette's ramps use plain `f32`/`f64` arithmetic step by step, as the original's x87 code does at 53-bit precision.
- The original's strings and font metrics are read from the player's `dr.exe` at fixed addresses of the known release (spec 3.1); DeadRally ships no game text. Never quote the game's text in code, tests or docs: test fixtures use made-up bytes.
- Menu screens are 640x480 indexed pictures shown at 4:3; a tick is 14 ms with 672 audio frames at 48 kHz.
- Fixed values (spec 3.2–3.5): title to black in 26 steps of 4 % from 100 %; the menu fades in from 0 to 98 % in steps of 2 over 50 ticks and stays at 98 %; entries 16–31 pulse 100 → 49 → 100 % in steps of 3 (34 calls); the background copper row steps on calls where `calls % 70 == 1`; a menu pass is two ticks; the cursor has 50 frames; effects 25 (move), 22 (back), 28 (choose) on channel 1 at volume 0xC000 and pitch 0x28000; the end screen holds 560 ticks and its fade-out lowers the volume mask `(65500 − 2620·step) >> 8`.
- **Nothing reaches the speakers.** The original runs only through `scripts/reference-run.sh`, whose `--sound` mode plays into a null sink and stops if `pactl` finds the game's stream anywhere else; Task 6 also runs it under a watcher that stops Wine if any stream reaches a real output. The frontend is never started with sound except through the scripts' disk driver.
- Game data is opened read-only. Never commit game data, screenshots or recordings: `captures/` and `dumps/` are ignored by Git. SHA-256 hashes of frames and sound may be committed.
- DreeRally and dRally are read-only documentation: never build, run or change them. The original `dr.exe` runs only under Wine on its own Xvfb display, never on the owner's display `:0`, never from the Steam install directly.
- Commits: prefixes `feat:`/`fix:`/`docs:`/`test:`/`refactor:`/`ci:`/`build:`; subject at most 50 characters, imperative, English; a body only when several things changed, as a `- ` list; no Co-Authored-By or any attribution.
- Talk to the owner in Czech; code, comments, docs and commits are in English.

## Review Focus

Inputs and conditions the spec implies but does not spell out, most likely to bite first. Each has a test in the task that owns the code.

1. **A `dr.exe` that is not the known release** (a string missing, unterminated, or holding bytes the original's never do) → the data fails to load with the address named, never garbage on screen. Task 3, `a_file_that_is_not_the_known_release_is_refused_with_the_address` and `a_byte_the_original_never_uses_is_refused_with_its_address`; Task 2, `addresses_outside_the_file_and_unterminated_strings_are_errors`.
2. **An `END.BMP` of another size** (from an unknown release) → a load error naming the file, not a panic when the player chooses to exit. Task 4, `an_end_screen_of_another_size_is_an_error_not_a_crash_later`.
3. **Escape pressed again on the exit row** → nothing happens; a player cannot leave the game by pressing Escape twice. Task 5, `escape_jumps_to_exit_once`.
4. **A key pressed while a screen fades in** → read before the screen's first wait, as the original does, never lost and never held for a later screen. Task 5, `a_key_during_a_credits_fade_in_moves_on_as_soon_as_it_is_done`.
5. **A gamepad held through a fade or pushed fresh** → the original's hold-off and repeat, not a runaway highlight. Task 5, `a_fresh_push_of_the_stick_holds_its_repeat_off_for_700_ms` and `the_gamepad_moves_and_chooses_like_the_keys`.

## Before You Start

- **Provenance:** every Rust file, script and scenario below was compiled, linted, tested and run on 2026-10-05 on the owner's Linux machine (Rust 1.99.0; Wine 9.0; the Steam data). The expected outputs (test counts, hashes, measured numbers) are real. If yours differ, stop and find out why before going on.
- **Where:** `/home/trashcan/DeadRally`, branch `m2a-menu` (it holds the spec and this plan). Game data: `export DEADRALLY_DATA=~/games/DeathRally` (Steam layout; DeadRally finds the `Death Rally` subfolder by itself).
- **Code blocks:** a block preceded by `<!-- write: PATH -->` is the complete content of `PATH`; `<!-- prepend: PATH -->` goes above the content already in `PATH`. Copy blocks exactly; apply them with the script in "Applying code blocks".
- **No [OWNER] steps:** the owner asked to be left alone until the work is done (spec status line).
- **Progress:** create one todo per task and tick it off when its checks pass (owner's rule).
- **Fail loud:** if a check cannot run, say so in the task report; never report it as passed.

## Applying code blocks

Save this as `/tmp/apply-plan.py` once. `python3 /tmp/apply-plan.py PLAN TASK STEP REPO` applies every block of one step and prints what it wrote:

```python
"""Applies the write/prepend blocks of one step of an implementation plan to a repository."""
import pathlib
import re
import sys

plan, task, step, repo = sys.argv[1], sys.argv[2], sys.argv[3], pathlib.Path(sys.argv[4])
text = pathlib.Path(plan).read_text()
tasks = re.split(r"^### Task (\d+):", text, flags=re.M)
body = dict(zip(tasks[1::2], tasks[2::2]))[task]
steps = re.split(r"^- \[ \] \*\*Step (\d+):", body, flags=re.M)
body = dict(zip(steps[1::2], steps[2::2]))[step]
blocks = re.findall(r"<!-- (write|prepend): (\S+) -->\n(`{3,})\w*\n(.*?)\n\3\n", body, flags=re.S)
if not blocks:
    sys.exit(f"task {task} step {step} has no code blocks")
for mode, path, _fence, content in blocks:
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    if mode == "write":
        target.write_text(content + "\n")
    else:
        target.write_text(content + "\n\n" + target.read_text())
    print(mode, path)
```

For example: `python3 /tmp/apply-plan.py docs/superpowers/plans/2026-10-05-m2a-main-menu.md 2 1 .`

## File Map

| Path | Responsibility | Task |
|---|---|---|
| `crates/core/src/audio/music.rs` | a note of an empty sample slot stops the channel; the song's master volume can change | 1, 5 |
| `crates/gamedata/src/exe.rs` | PE sections; bytes and NUL-terminated strings at virtual addresses | 2 |
| `crates/gamedata/src/text.rs` | the menu texts, the panel's lines, the exit question and the font metrics of `dr.exe` | 3 |
| `crates/gamedata/src/assets.rs`, `known_versions.rs`, `lib.rs` | `DR.EXE` required; the menu's pictures, palettes, effects and texts in `Assets` | 2, 3, 4 |
| `crates/gamedata/tests/catalog_data.rs`, `crates/headless/tests/cli.rs` | the menu assets against the real data; twenty required files | 4 |
| `crates/core/tests/common/mod.rs` | synthetic menu assets, one colour per picture | 4 |
| `crates/core/src/canvas.rs`, `font.rs` | the original's drawing buffer and bitmap fonts | 5 |
| `crates/core/src/keys.rs` | one remembered key, SDL 1.2's repeat, the joystick path | 5 |
| `crates/core/src/menu/palette.rs`, `menu/draw.rs`, `menu/mod.rs` | the menu's palette, its drawing, the scene | 5 |
| `crates/core/src/audio/mod.rs`, `effects.rs` | volumes and the mask, the menu's effects bank and triggers | 5 |
| `crates/core/src/startup.rs`, `game.rs`, `lib.rs`, `crates/deadrally/src/main.rs` | the hand-over to the menu; quitting | 5 |
| `crates/core/tests/menu.rs`, `startup.rs` | the menu and the hand-over on synthetic assets | 4, 5 |
| `crates/headless/tests/rendered-audio.sha256` | the startup's sound, changed at the intro's end | 5 |
| `crates/headless/src/main.rs` | `--key-at TICK:KEY` | 6 |
| `scripts/reference/menu-*.scenario` | screenshots and recordings of the original's menus | 6 |
| `crates/headless/tests/common/mod.rs`, `menu_run.rs`, `menu-run.sha256`, `rendered_audio.rs` | the manifest of a run through the menus | 6 |
| `docs/verification/m2a.md`, the spec, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md` | what the runs showed; status; commands | 7 |

---

### Task 1: Silence notes of empty sample slots

Found by the menu's recording (Task 6): FMOD plays a note whose sample slot is empty as silence, so the note sounding on that channel stops. M1b's player ignored such a note, so the channel's previous looping note sounded on at the new note's volume. The menu music's order 47 starts such notes on two channels: a stray tone of 0.8 s, in M1b's recording of the startup too (its largest loudness difference, 2.55 dB).

**Files:**
- Modify: `crates/core/src/audio/music.rs`

**Interfaces:** none new; `Music` behaves differently for one kind of cell.

- [ ] **Step 1: Write the failing test**

The whole file with the test `a_note_of_an_empty_sample_slot_silences_the_channel` added; the fix comes in Step 3.

<!-- write: crates/core/src/audio/music.rs -->
```rust
//! The music player: Scream Tracker 3 modules with the 13 commands the game's music uses
//! (spec M1b §3.2, §4.2). Timing is counted in output samples, so the tempo never drifts.

use std::sync::Arc;

use deadrally_gamedata::s3m::{self, Cell, Module, NO_NOTE, NOTE_CUT, ORDER_SKIP};

use super::mixer::{Loop, Voice};
use super::tables::{S3M_PERIODS, vibrato};
use crate::AUDIO_SAMPLE_RATE;

/// Scream Tracker's clock: frequency = 14317056 / period.
const CLOCK: u32 = 14_317_056;
/// FMOD mixes at this rate and counts a tick as a whole number of its samples:
/// `44100 * 5 / (2 * tempo)`, rounded down. At tempo 141 that is 781 samples, not 781.9, so
/// the original plays such music 0.1 % fast; at tempo 125 it is exactly 882 (20 ms).
const FMOD_RATE: u32 = 44_100;
/// Period limits of Scream Tracker 3.
const MIN_PERIOD: i32 = 64;
const MAX_PERIOD: i32 = 32_767;

#[derive(Clone, Debug)]
struct SampleData {
    data: Arc<[i16]>,
    looping: Loop,
    c2spd: u32,
    volume: i32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    enabled: bool,
    /// 0..=255.
    pan: i64,
    voice: Option<Voice>,
    /// Current sample (1-based instrument number), 0 = none yet.
    instrument: u8,
    period: i32,
    target_period: i32,
    /// 0..=64.
    volume: i32,
    /// Vibrato offset of this tick, in period units.
    period_delta: i32,
    command: u8,
    info: u8,
    // Effect memories.
    volume_slide: u8,
    porta: u8,
    tone_porta: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_position: u8,
    offset: u8,
    retrigger: u8,
    retrigger_count: u8,
    /// A row's note held back by SDx until this tick.
    delayed: Option<(u8, Cell)>,
}

#[derive(Debug)]
pub(crate) struct Music {
    orders: Vec<u8>,
    patterns: Vec<s3m::Pattern>,
    samples: Vec<Option<SampleData>>,
    global_volume: i32,
    channels: Vec<Channel>,
    fading: Vec<Voice>,
    speed: u8,
    tempo: u8,
    order: usize,
    row: usize,
    tick: u8,
    /// Output frames left in the current tick, and the carried fraction of a frame.
    frames_left: u32,
    remainder: u32,
    /// Where the next row comes from after a B or C command.
    jump: Option<(usize, usize)>,
    /// Gain applied to every channel, in 16.16 (the original's master volume).
    gain: i64,
}

impl Music {
    /// A player at order `first_order` (counted as the game counts them, markers included),
    /// row 0 and tick 0. `gain` scales the whole module ([`UNITY`] = as loud as the module
    /// asks). As FMOD does (measured), the module's own master volume scales it by
    /// `master / 64` (the game's music has 48, 2.5 dB below full), and a stereo module plays
    /// at twice a mono module's level.
    pub(crate) fn new(module: &Module, gain: i64, first_order: usize) -> Music {
        let stereo = if module.stereo { 2 } else { 1 };
        let gain = gain * i64::from(module.master_volume) * stereo / 64;
        let samples = module
            .samples
            .iter()
            .map(|sample| {
                (!sample.data.is_empty()).then(|| SampleData {
                    data: Arc::from(sample.data.as_slice()),
                    looping: sample
                        .looped
                        .map_or(Loop::None, |(start, end)| Loop::Forward { start, end }),
                    c2spd: sample.c2spd,
                    volume: i32::from(sample.volume),
                })
            })
            .collect();
        let channels = module
            .channels
            .iter()
            .map(|channel| Channel {
                enabled: channel.enabled,
                // FMOD 3 puts a stereo module's channels fully on their side (measured on the
                // menu music); mono modules play in the centre.
                pan: match (module.stereo, channel.pan < 8) {
                    (false, _) => 128,
                    (true, true) => 0,
                    (true, false) => 255,
                },
                ..Channel::default()
            })
            .collect();
        let mut music = Music {
            orders: module.orders.clone(),
            patterns: module.patterns.clone(),
            samples,
            global_volume: i32::from(module.global_volume),
            channels,
            fading: Vec::new(),
            speed: module.initial_speed.max(1),
            tempo: module.initial_tempo.max(32),
            order: first_order,
            row: 0,
            tick: 0,
            frames_left: 0,
            remainder: 0,
            jump: None,
            gain,
        };
        music.skip_marker_orders();
        music
    }

    /// Fades every channel out, as `FMUSIC_StopSong`.
    pub(crate) fn stop(&mut self) {
        for channel in &mut self.channels {
            if let Some(mut voice) = channel.voice.take() {
                voice.release();
                self.fading.push(voice);
            }
        }
        self.orders.clear();
    }

    /// Fades every channel out and hands the fading voices over, for music being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop();
        self.fading
    }

    /// Adds the music to `out` (interleaved stereo), advancing ticks exactly on time.
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        let mut done = 0;
        let frames = out.len() / 2;
        while done < frames {
            if self.frames_left == 0 {
                if !self.orders.is_empty() {
                    self.process_tick();
                }
                self.start_tick_timer();
            }
            let count = (frames - done).min(self.frames_left as usize);
            let part = &mut out[2 * done..2 * (done + count)];
            for channel in &mut self.channels {
                if let Some(voice) = &mut channel.voice {
                    voice.mix_into(part);
                    if voice.finished() {
                        channel.voice = None;
                    }
                }
            }
            for voice in &mut self.fading {
                voice.mix_into(part);
            }
            self.fading.retain(|voice| !voice.finished());
            done += count;
            self.frames_left -= u32::try_from(count).expect("at most a tick");
        }
    }

    /// The next tick's length: FMOD's whole number of 44.1 kHz samples, converted to our rate
    /// exactly by carrying the remainder.
    fn start_tick_timer(&mut self) {
        let fmod_samples = FMOD_RATE * 5 / (2 * u32::from(self.tempo));
        let scaled = fmod_samples * AUDIO_SAMPLE_RATE + self.remainder;
        self.frames_left = scaled / FMOD_RATE;
        self.remainder = scaled % FMOD_RATE;
    }

    /// Moves past 254 and 255, which FMOD drops from the order list.
    fn skip_marker_orders(&mut self) {
        while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
            self.order += 1;
        }
        if self.order >= self.orders.len() {
            // The song loops from its start, as FMOD's looping music does.
            self.order = 0;
            while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
                self.order += 1;
            }
        }
    }

    fn process_tick(&mut self) {
        if self.tick == 0 {
            self.process_row();
        } else {
            for index in 0..self.channels.len() {
                self.channel_tick(index);
            }
        }
        for index in 0..self.channels.len() {
            self.update_voice(index);
        }
        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.next_row();
        }
    }

    fn next_row(&mut self) {
        if let Some((order, row)) = self.jump.take() {
            self.order = order;
            self.row = row;
            self.skip_marker_orders();
            return;
        }
        self.row += 1;
        if self.row >= s3m::ROWS {
            self.row = 0;
            self.order += 1;
            self.skip_marker_orders();
        }
    }

    fn process_row(&mut self) {
        let Some(&pattern) = self.orders.get(self.order) else {
            return;
        };
        let cells = self.patterns[usize::from(pattern)].rows[self.row];
        for (index, cell) in cells.iter().enumerate() {
            if !self.channels[index].enabled {
                continue;
            }
            let channel = &mut self.channels[index];
            channel.command = cell.command;
            channel.info = cell.info;
            channel.period_delta = 0;
            if cell.command == command('S') && cell.info >> 4 == 0xD && cell.info & 0xF > 0 {
                channel.delayed = Some((cell.info & 0xF, *cell));
                continue;
            }
            channel.delayed = None;
            self.start_cell(index, cell);
            self.row_effect(index, cell);
        }
    }

    /// The note, instrument and volume of a cell.
    fn start_cell(&mut self, index: usize, cell: &Cell) {
        let tone_porta = cell.command == command('G');
        if cell.instrument != 0 {
            let channel = &mut self.channels[index];
            channel.instrument = cell.instrument;
            if let Some(Some(sample)) = self.samples.get(usize::from(cell.instrument) - 1) {
                channel.volume = sample.volume;
            }
        }
        if cell.note == NOTE_CUT {
            self.cut(index);
        } else if cell.note != NO_NOTE {
            let instrument = self.channels[index].instrument;
            if let Some(Some(sample)) = usize::from(instrument)
                .checked_sub(1)
                .and_then(|i| self.samples.get(i))
            {
                let period = note_period(cell.note, sample.c2spd);
                let channel = &mut self.channels[index];
                if tone_porta && channel.voice.is_some() {
                    channel.target_period = period;
                } else {
                    let offset = if cell.command == command('O') {
                        if cell.info != 0 {
                            channel.offset = cell.info;
                        }
                        u32::from(channel.offset) * 256
                    } else {
                        0
                    };
                    let voice = Voice::new(Arc::clone(&sample.data), sample.looping, offset);
                    if let Some(mut old) = channel.voice.replace(voice) {
                        old.release();
                        self.fading.push(old);
                    }
                    let channel = &mut self.channels[index];
                    channel.period = period;
                    channel.target_period = period;
                    channel.vibrato_position = 0;
                    channel.retrigger_count = 0;
                }
            }
        }
        if let Some(volume) = cell.volume {
            self.channels[index].volume = i32::from(volume);
        }
    }

    fn cut(&mut self, index: usize) {
        if let Some(mut voice) = self.channels[index].voice.take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    /// Commands that act on the row's first tick.
    fn row_effect(&mut self, index: usize, cell: &Cell) {
        let info = cell.info;
        let channel = &mut self.channels[index];
        match letter(cell.command) {
            'A' if info > 0 => self.speed = info,
            'T' if info >= 0x20 => self.tempo = info,
            'B' => {
                let row = self.jump.map_or(0, |(_, row)| row);
                self.jump = Some((usize::from(info), row));
            }
            'C' => {
                let row = usize::from((info >> 4) * 10 + (info & 0xF)).min(s3m::ROWS - 1);
                let order = self.jump.map_or(self.order + 1, |(order, _)| order);
                self.jump = Some((order, row));
            }
            'D' | 'K' => {
                if info != 0 {
                    channel.volume_slide = info;
                }
                let slide = channel.volume_slide;
                // Fine slides (DxF up, DFy down) act once, on this tick.
                if slide & 0x0F == 0x0F && slide >> 4 != 0 {
                    channel.volume = (channel.volume + i32::from(slide >> 4)).min(64);
                } else if slide >> 4 == 0x0F && slide & 0x0F != 0 {
                    channel.volume = (channel.volume - i32::from(slide & 0x0F)).max(0);
                }
            }
            'E' | 'F' => {
                if info != 0 {
                    channel.porta = info;
                }
                let porta = channel.porta;
                let sign = if letter(cell.command) == 'E' { 1 } else { -1 };
                // EFx fine (x * 4), EEx extra fine (x), on this tick only.
                match porta >> 4 {
                    0xF => channel.period += sign * 4 * i32::from(porta & 0xF),
                    0xE => channel.period += sign * i32::from(porta & 0xF),
                    _ => {}
                }
                channel.period = channel.period.clamp(MIN_PERIOD, MAX_PERIOD);
            }
            'G' => {
                if info != 0 {
                    channel.tone_porta = info;
                }
            }
            'H' => {
                if info >> 4 != 0 {
                    channel.vibrato_speed = info >> 4;
                }
                if info & 0xF != 0 {
                    channel.vibrato_depth = info & 0xF;
                }
            }
            'Q' if info != 0 => channel.retrigger = info,
            _ => {}
        }
    }

    /// Commands that act on every tick but the first.
    fn channel_tick(&mut self, index: usize) {
        if !self.channels[index].enabled {
            return;
        }
        if let Some((at, cell)) = self.channels[index].delayed {
            if self.tick == at {
                self.channels[index].delayed = None;
                self.start_cell(index, &cell);
            }
            return;
        }
        let channel = &mut self.channels[index];
        channel.period_delta = 0;
        match letter(channel.command) {
            'D' => volume_slide(channel),
            'K' => {
                volume_slide(channel);
                vibrato_tick(channel);
            }
            'E' | 'F' => {
                let porta = channel.porta;
                if porta >> 4 < 0xE {
                    let sign = if letter(channel.command) == 'E' {
                        1
                    } else {
                        -1
                    };
                    channel.period = (channel.period + sign * 4 * i32::from(porta))
                        .clamp(MIN_PERIOD, MAX_PERIOD);
                }
            }
            'G' => {
                let speed = 4 * i32::from(channel.tone_porta);
                if channel.period < channel.target_period {
                    channel.period = (channel.period + speed).min(channel.target_period);
                } else {
                    channel.period = (channel.period - speed).max(channel.target_period);
                }
            }
            'H' => vibrato_tick(channel),
            // Without an interval nothing repeats, and nothing is counted.
            'Q' if channel.retrigger & 0xF != 0 => {
                let interval = channel.retrigger & 0xF;
                channel.retrigger_count += 1;
                if channel.retrigger_count >= interval {
                    channel.retrigger_count = 0;
                    channel.volume = retrigger_volume(channel.volume, channel.retrigger >> 4);
                    let sample = usize::from(channel.instrument)
                        .checked_sub(1)
                        .and_then(|i| self.samples.get(i))
                        .and_then(Option::as_ref);
                    if let Some(sample) = sample {
                        let voice = Voice::new(Arc::clone(&sample.data), sample.looping, 0);
                        if let Some(mut old) = channel.voice.replace(voice) {
                            old.release();
                            self.fading.push(old);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pushes the channel's pitch and volume into its voice.
    fn update_voice(&mut self, index: usize) {
        let global = i64::from(self.global_volume);
        let gain = self.gain;
        let channel = &mut self.channels[index];
        let Some(voice) = &mut channel.voice else {
            return;
        };
        let period = (channel.period + channel.period_delta).clamp(MIN_PERIOD, MAX_PERIOD);
        voice.set_frequency(CLOCK / u32::try_from(period).expect("positive"));
        let volume = i64::from(channel.volume) * global * gain / (64 * 64);
        let left = volume * (255 - channel.pan) / 255;
        let right = volume * channel.pan / 255;
        voice.set_volume(left, right);
    }

    #[cfg(test)]
    fn position(&self) -> (usize, usize, u8) {
        (self.order, self.row, self.tick)
    }
}

fn command(letter: char) -> u8 {
    letter as u8 - b'@'
}

fn letter(command: u8) -> char {
    if (1..=26).contains(&command) {
        char::from(b'@' + command)
    } else {
        ' '
    }
}

/// Scream Tracker's period of a note byte (octave in the high nibble) for a sample's C2SPD.
/// The octave shift comes last, so high notes keep their precision.
fn note_period(note: u8, c2spd: u32) -> i32 {
    let (octave, semitone) = (u32::from(note >> 4), usize::from(note & 0xF).min(11));
    let period =
        (8363 * 16 * u64::from(S3M_PERIODS[semitone]) / u64::from(c2spd.max(1))) >> octave.min(9);
    i32::try_from(period)
        .unwrap_or(MAX_PERIOD)
        .clamp(MIN_PERIOD, MAX_PERIOD)
}

fn volume_slide(channel: &mut Channel) {
    let slide = channel.volume_slide;
    let (up, down) = (slide >> 4, slide & 0x0F);
    if down == 0 && up != 0 {
        channel.volume = (channel.volume + i32::from(up)).min(64);
    } else if up == 0 && down != 0 {
        channel.volume = (channel.volume - i32::from(down)).max(0);
    }
}

fn vibrato_tick(channel: &mut Channel) {
    let wave = vibrato(channel.vibrato_position);
    channel.period_delta = (wave * i32::from(channel.vibrato_depth)) >> 5;
    channel.vibrato_position = (channel.vibrato_position + channel.vibrato_speed) & 63;
}

/// Qxy's volume change `x` applied to a volume 0..=64.
fn retrigger_volume(volume: i32, change: u8) -> i32 {
    let changed = match change {
        1..=5 => volume - (1 << (change - 1)),
        6 => volume * 2 / 3,
        7 => volume / 2,
        9..=13 => volume + (1 << (change - 9)),
        14 => volume * 3 / 2,
        15 => volume * 2,
        _ => volume,
    };
    changed.clamp(0, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{Channel as S3mChannel, Pattern, Sample};

    use crate::audio::mixer::UNITY;

    const C4: u8 = 0x40;

    fn module(rows: &[(usize, usize, Cell)], speed: u8, tempo: u8) -> Module {
        let mut pattern = Pattern {
            rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
        };
        for &(row, channel, cell) in rows {
            pattern.rows[row][channel] = cell;
        }
        let mut channels = [S3mChannel::default(); s3m::CHANNELS];
        channels[0] = S3mChannel {
            enabled: true,
            pan: 3,
        };
        channels[1] = S3mChannel {
            enabled: true,
            pan: 12,
        };
        Module {
            title: "Test".into(),
            orders: vec![0, 1],
            initial_speed: speed,
            initial_tempo: tempo,
            global_volume: 64,
            master_volume: 48,
            stereo: false,
            channels,
            samples: vec![Sample {
                name: "Tone".into(),
                c2spd: 8363,
                volume: 32,
                looped: Some((0, 100)),
                data: vec![10_000; 100],
            }],
            patterns: vec![pattern.clone(), pattern],
        }
    }

    fn cell(note: u8, instrument: u8, volume: Option<u8>, command: char, info: u8) -> Cell {
        Cell {
            note,
            instrument,
            volume,
            command: if command == ' ' {
                0
            } else {
                super::command(command)
            },
            info,
        }
    }

    fn play(music: &mut Music, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        music.mix_into(&mut out);
        out
    }

    #[test]
    fn a_tick_lasts_fmods_whole_number_of_samples() {
        // At tempo 125 a tick is 882 samples at 44.1 kHz, exactly 20 ms: 960 of ours.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 960 * 6);
        assert_eq!(music.position(), (0, 1, 0));
        // At tempo 141 FMOD counts 781 samples (not 781.9) per tick, which is why the menu
        // music runs 0.1 % fast in the original: 147 ticks are exactly 124 960 of our samples.
        let mut faster = Music::new(&module(&[], 1, 141), UNITY, 0);
        play(&mut faster, 124_960);
        assert_eq!(
            faster.position(),
            (0, 19, 0),
            "147 rows: both patterns, then the song loops to row 19 of its start"
        );
    }

    #[test]
    fn middle_c_plays_at_the_samples_c2spd() {
        assert_eq!(note_period(C4, 8363), 1712);
        assert_eq!(CLOCK / 1712, 8362);
        assert_eq!(
            note_period(0x50, 8363),
            856,
            "an octave up halves the period"
        );
        assert_eq!(
            note_period(C4, 16_726),
            856,
            "a doubled C2SPD sounds an octave up"
        );
    }

    #[test]
    fn notes_play_with_the_samples_volume_and_the_volume_column_overrides_it() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(C4, 1, Some(64), ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        let out = play(&mut music, 960);
        // Mono: centred, 32 / 64 of full volume on each side, and the module's master volume
        // 48 / 64 on top.
        let expected = 10_000 * 32 / 64 * 127 / 255 * 48 / 64;
        assert!(
            (out[2 * 900] - expected).abs() <= 2,
            "{} vs {expected}",
            out[2 * 900]
        );
        let louder = play(&mut music, 960);
        assert!(
            (louder[2 * 900] - 2 * expected).abs() <= 4,
            "{}",
            louder[2 * 900]
        );
    }

    #[test]
    fn speed_tempo_jump_and_break_commands_steer_the_song() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(NO_NOTE, 0, None, 'A', 2)),
                    (0, 1, cell(NO_NOTE, 0, None, 'C', 0x10)),
                ],
                6,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 2);
        assert_eq!(
            music.position(),
            (1, 10, 0),
            "speed 2, then a break to row 10 of the next order"
        );
        let mut jumping = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'B', 0))], 1, 125),
            UNITY,
            0,
        );
        play(&mut jumping, 960);
        assert_eq!(jumping.position(), (0, 0, 0), "B00 loops the first order");
        // As in minifmod, a new tempo already sets the length of the tick that sets it.
        let mut tempo = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'T', 250))], 1, 125),
            UNITY,
            0,
        );
        play(&mut tempo, 480 * 3);
        assert_eq!(
            tempo.position(),
            (0, 3, 0),
            "tempo 250: ticks of 480 samples"
        );
    }

    #[test]
    fn stereo_modules_play_each_channel_fully_on_its_side_and_twice_as_loud() {
        // FMOD 3 pans a stereo module's channels hard left or right, and plays them at twice a
        // mono module's level: only that matches the stereo image and the loudness of the
        // original's menu music, recorded at two music volumes (docs/verification/m1b.md).
        let mut stereo = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        stereo.stereo = true;
        let mut music = Music::new(&stereo, UNITY, 0);
        let out = play(&mut music, 960);
        let full = 10_000 * 48 / 64 * 2;
        assert!((out[2 * 900] - full).abs() <= 4, "left {}", out[2 * 900]);
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn a_song_can_start_at_a_later_order() {
        // The game starts the menu music at order 45 (musicSetOrder), not at its beginning.
        let mut later = module(&[], 1, 125);
        later.orders = vec![0, s3m::ORDER_END, 1];
        let music = Music::new(&later, UNITY, 2);
        assert_eq!(music.position(), (2, 0, 0));
        let at_marker = Music::new(&later, UNITY, 1);
        assert_eq!(at_marker.position(), (2, 0, 0), "a marker is skipped");
    }

    #[test]
    fn playback_runs_on_through_section_markers_as_fmod_does() {
        // FMOD drops 254 and 255 from the order list (the game numbers orders for
        // FMUSIC_SetOrder without them), so a pattern before a 255 is followed by the next
        // section, not by the song's start.
        let mut base = module(&[], 1, 125);
        base.orders = vec![0, s3m::ORDER_END, s3m::ORDER_SKIP, 1];
        let mut music = Music::new(&base, UNITY, 0);
        play(&mut music, 960 * s3m::ROWS);
        assert_eq!(music.position(), (3, 0, 0));
    }

    /// A module whose one sample rises steadily (0, 8, 16, ...), so the output shows how far
    /// into the sample a voice is.
    fn rising(rows: &[(usize, usize, Cell)], speed: u8) -> Module {
        let mut rising = module(rows, speed, 125);
        rising.samples[0].data = (0..4000).map(|i| i16::try_from(i * 8).unwrap()).collect();
        rising.samples[0].looped = None;
        rising
    }

    #[test]
    fn the_sample_offset_starts_a_note_further_into_its_sample() {
        // O (545 times in TR5) starts notes part way into their sample; ignoring it would play
        // the sample's beginning instead.
        let mut plain = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 6),
            UNITY,
            0,
        );
        let mut offset = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), 'O', 8))], 6),
            UNITY,
            0,
        );
        let (plain, offset) = (play(&mut plain, 300), play(&mut offset, 300));
        // 8 * 256 = 2048 samples in: about 2100 instead of about 50 at frame 299.
        assert!(plain[2 * 299] > 0);
        assert!(
            offset[2 * 299] > 20 * plain[2 * 299],
            "{} vs {}",
            offset[2 * 299],
            plain[2 * 299]
        );
    }

    #[test]
    fn vibrato_with_volume_slide_does_both() {
        // K (277 times in TR1) keeps an earlier H's vibrato going while it slides the volume;
        // doing only one of the two freezes the note or its level.
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, Some(10), 'H', 0x48)),
                    (1, 0, cell(NO_NOTE, 0, None, 'K', 0x20)),
                ],
                4,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        let position = music.channels[0].vibrato_position;
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        assert_eq!(
            music.channels[0].vibrato_position,
            position + 3 * 4,
            "the vibrato goes on at speed 4"
        );
    }

    #[test]
    fn portamento_up_lowers_the_period_and_its_fine_form_acts_once() {
        // F (252 times in TR5) slides notes up; the wrong direction, or FFx on every tick,
        // would detune whole phrases.
        let mut up = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut up, 960 * 3);
        assert_eq!(up.channels[0].period, 1712 - 2 * 8, "2 ticks of 4 * 2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 0xF3))], 3, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 3);
        assert_eq!(
            fine.channels[0].period,
            1712 - 3 * 4,
            "FF3: once, on the first tick"
        );
    }

    #[test]
    fn retrigger_restarts_the_note_at_its_interval_and_changes_its_volume() {
        // Q (90 times in TR9) drums a note several times per row; without the restarts, or the
        // volume change, a roll becomes one long note.
        let mut music = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(20), 'Q', 0xA3))], 7),
            UNITY,
            0,
        );
        let out = play(&mut music, 960 * 7);
        // Restarted at tick 3: 200 frames later a voice plays again, near the sample's start.
        let (restarted, before) = (out[2 * (3 * 960 + 200)], out[2 * (3 * 960 - 1)]);
        assert!(
            restarted > 0 && restarted < before / 4,
            "{restarted} vs {before}"
        );
        assert_eq!(
            music.channels[0].volume, 24,
            "+2 at each restart, on ticks 3 and 6"
        );
    }

    #[test]
    fn a_retrigger_without_an_interval_never_overflows() {
        // Q00 before any Q with an interval repeats nothing; counting its ticks anyway overflowed
        // after 255 of them and crashed the game in the middle of the music.
        let mut long = module(
            &[
                (0, 0, cell(C4, 1, Some(64), 'Q', 0)),
                (1, 0, cell(NO_NOTE, 0, None, 'Q', 0)),
            ],
            200,
            125,
        );
        long.orders = vec![0];
        let mut music = Music::new(&long, UNITY, 0);
        play(&mut music, 960 * 400);
        assert_eq!(music.position(), (0, 2, 0));
    }

    #[test]
    fn a_song_of_markers_only_stays_silent() {
        // An order list without a pattern must not hang the player looking for one.
        let mut base = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        base.orders = vec![s3m::ORDER_SKIP, s3m::ORDER_END];
        let mut music = Music::new(&base, UNITY, 0);
        assert!(play(&mut music, 960 * 4).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn volume_slides_act_after_the_first_tick_and_fine_ones_on_it() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x20))], 4, 125),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x3F))], 4, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 4);
        assert_eq!(fine.channels[0].volume, 13, "one fine step of +3");
    }

    #[test]
    fn portamentos_move_the_period() {
        let mut down = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'E', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut down, 960 * 3);
        assert_eq!(down.channels[0].period, 1712 + 2 * 8);
        let mut towards = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(0x50, 1, None, 'G', 0xFF)),
                ],
                3,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut towards, 960 * 6);
        assert_eq!(
            towards.channels[0].period, 856,
            "the tone portamento stops at its target"
        );
    }

    #[test]
    fn vibrato_wobbles_around_the_note() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'H', 0x48))], 6, 125),
            UNITY,
            0,
        );
        let mut deltas = Vec::new();
        for _ in 0..6 {
            play(&mut music, 960);
            deltas.push(music.channels[0].period_delta);
        }
        assert_eq!(deltas[0], 0, "no vibrato on the first tick");
        assert!(
            deltas[1..].iter().all(|&delta| delta >= 0) && deltas[5] > 0,
            "{deltas:?}"
        );
        assert_eq!(
            music.channels[0].period, 1712,
            "the note itself does not move"
        );
    }

    #[test]
    fn a_note_of_an_empty_sample_slot_silences_the_channel() {
        // The menu music's order 47 starts with notes of a sample slot its author emptied;
        // FMOD plays them as silence. Ignoring them left the channel's looping note playing,
        // brought back up by their volume: a stray tone in the menu.
        let mut emptied = module(
            &[
                (0, 0, cell(C4, 1, Some(64), ' ', 0)),
                (1, 0, cell(C4, 2, Some(32), ' ', 0)),
            ],
            1,
            125,
        );
        emptied.samples.push(Sample::default());
        let mut music = Music::new(&emptied, UNITY, 0);
        play(&mut music, 960);
        let out = play(&mut music, 960);
        assert!(out[2 * 900..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn note_delay_and_cut_and_retrigger() {
        let mut delayed = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'S', 0xD2))], 4, 125),
            UNITY,
            0,
        );
        play(&mut delayed, 960 * 2);
        assert!(delayed.channels[0].voice.is_none(), "not before tick 2");
        play(&mut delayed, 960);
        assert!(delayed.channels[0].voice.is_some());
        let mut cut = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(NOTE_CUT, 0, None, ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut cut, 960 * 2);
        assert!(cut.channels[0].voice.is_none());
        assert_eq!(retrigger_volume(40, 0xF), 64);
        assert_eq!(retrigger_volume(40, 0x7), 20);
        assert_eq!(retrigger_volume(40, 0x3), 36);
    }

    #[test]
    fn the_same_module_always_renders_the_same_samples() {
        let rows = [
            (0, 0, cell(C4, 1, Some(40), 'H', 0x46)),
            (8, 1, cell(0x45, 1, None, 'Q', 0x93)),
        ];
        let render = || {
            let mut music = Music::new(&module(&rows, 3, 131), UNITY, 0);
            play(&mut music, 48_000)
        };
        assert_eq!(render(), render());
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p deadrally-core --lib audio::music::tests::a_note_of_an_empty_sample_slot_silences_the_channel`
Expected: FAIL (`assertion failed: out[2 * 900..].iter().all(|&sample| sample == 0)`).

- [ ] **Step 3: Implement**

<!-- write: crates/core/src/audio/music.rs -->
```rust
//! The music player: Scream Tracker 3 modules with the 13 commands the game's music uses
//! (spec M1b §3.2, §4.2). Timing is counted in output samples, so the tempo never drifts.

use std::sync::Arc;

use deadrally_gamedata::s3m::{self, Cell, Module, NO_NOTE, NOTE_CUT, ORDER_SKIP};

use super::mixer::{Loop, Voice};
use super::tables::{S3M_PERIODS, vibrato};
use crate::AUDIO_SAMPLE_RATE;

/// Scream Tracker's clock: frequency = 14317056 / period.
const CLOCK: u32 = 14_317_056;
/// FMOD mixes at this rate and counts a tick as a whole number of its samples:
/// `44100 * 5 / (2 * tempo)`, rounded down. At tempo 141 that is 781 samples, not 781.9, so
/// the original plays such music 0.1 % fast; at tempo 125 it is exactly 882 (20 ms).
const FMOD_RATE: u32 = 44_100;
/// Period limits of Scream Tracker 3.
const MIN_PERIOD: i32 = 64;
const MAX_PERIOD: i32 = 32_767;

#[derive(Clone, Debug)]
struct SampleData {
    data: Arc<[i16]>,
    looping: Loop,
    c2spd: u32,
    volume: i32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    enabled: bool,
    /// 0..=255.
    pan: i64,
    voice: Option<Voice>,
    /// Current sample (1-based instrument number), 0 = none yet.
    instrument: u8,
    period: i32,
    target_period: i32,
    /// 0..=64.
    volume: i32,
    /// Vibrato offset of this tick, in period units.
    period_delta: i32,
    command: u8,
    info: u8,
    // Effect memories.
    volume_slide: u8,
    porta: u8,
    tone_porta: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_position: u8,
    offset: u8,
    retrigger: u8,
    retrigger_count: u8,
    /// A row's note held back by SDx until this tick.
    delayed: Option<(u8, Cell)>,
}

#[derive(Debug)]
pub(crate) struct Music {
    orders: Vec<u8>,
    patterns: Vec<s3m::Pattern>,
    samples: Vec<Option<SampleData>>,
    global_volume: i32,
    channels: Vec<Channel>,
    fading: Vec<Voice>,
    speed: u8,
    tempo: u8,
    order: usize,
    row: usize,
    tick: u8,
    /// Output frames left in the current tick, and the carried fraction of a frame.
    frames_left: u32,
    remainder: u32,
    /// Where the next row comes from after a B or C command.
    jump: Option<(usize, usize)>,
    /// Gain applied to every channel, in 16.16 (the original's master volume).
    gain: i64,
}

impl Music {
    /// A player at order `first_order` (counted as the game counts them, markers included),
    /// row 0 and tick 0. `gain` scales the whole module ([`UNITY`] = as loud as the module
    /// asks). As FMOD does (measured), the module's own master volume scales it by
    /// `master / 64` (the game's music has 48, 2.5 dB below full), and a stereo module plays
    /// at twice a mono module's level.
    pub(crate) fn new(module: &Module, gain: i64, first_order: usize) -> Music {
        let stereo = if module.stereo { 2 } else { 1 };
        let gain = gain * i64::from(module.master_volume) * stereo / 64;
        let samples = module
            .samples
            .iter()
            .map(|sample| {
                (!sample.data.is_empty()).then(|| SampleData {
                    data: Arc::from(sample.data.as_slice()),
                    looping: sample
                        .looped
                        .map_or(Loop::None, |(start, end)| Loop::Forward { start, end }),
                    c2spd: sample.c2spd,
                    volume: i32::from(sample.volume),
                })
            })
            .collect();
        let channels = module
            .channels
            .iter()
            .map(|channel| Channel {
                enabled: channel.enabled,
                // FMOD 3 puts a stereo module's channels fully on their side (measured on the
                // menu music); mono modules play in the centre.
                pan: match (module.stereo, channel.pan < 8) {
                    (false, _) => 128,
                    (true, true) => 0,
                    (true, false) => 255,
                },
                ..Channel::default()
            })
            .collect();
        let mut music = Music {
            orders: module.orders.clone(),
            patterns: module.patterns.clone(),
            samples,
            global_volume: i32::from(module.global_volume),
            channels,
            fading: Vec::new(),
            speed: module.initial_speed.max(1),
            tempo: module.initial_tempo.max(32),
            order: first_order,
            row: 0,
            tick: 0,
            frames_left: 0,
            remainder: 0,
            jump: None,
            gain,
        };
        music.skip_marker_orders();
        music
    }

    /// Fades every channel out, as `FMUSIC_StopSong`.
    pub(crate) fn stop(&mut self) {
        for channel in &mut self.channels {
            if let Some(mut voice) = channel.voice.take() {
                voice.release();
                self.fading.push(voice);
            }
        }
        self.orders.clear();
    }

    /// Fades every channel out and hands the fading voices over, for music being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop();
        self.fading
    }

    /// Adds the music to `out` (interleaved stereo), advancing ticks exactly on time.
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        let mut done = 0;
        let frames = out.len() / 2;
        while done < frames {
            if self.frames_left == 0 {
                if !self.orders.is_empty() {
                    self.process_tick();
                }
                self.start_tick_timer();
            }
            let count = (frames - done).min(self.frames_left as usize);
            let part = &mut out[2 * done..2 * (done + count)];
            for channel in &mut self.channels {
                if let Some(voice) = &mut channel.voice {
                    voice.mix_into(part);
                    if voice.finished() {
                        channel.voice = None;
                    }
                }
            }
            for voice in &mut self.fading {
                voice.mix_into(part);
            }
            self.fading.retain(|voice| !voice.finished());
            done += count;
            self.frames_left -= u32::try_from(count).expect("at most a tick");
        }
    }

    /// The next tick's length: FMOD's whole number of 44.1 kHz samples, converted to our rate
    /// exactly by carrying the remainder.
    fn start_tick_timer(&mut self) {
        let fmod_samples = FMOD_RATE * 5 / (2 * u32::from(self.tempo));
        let scaled = fmod_samples * AUDIO_SAMPLE_RATE + self.remainder;
        self.frames_left = scaled / FMOD_RATE;
        self.remainder = scaled % FMOD_RATE;
    }

    /// Moves past 254 and 255, which FMOD drops from the order list.
    fn skip_marker_orders(&mut self) {
        while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
            self.order += 1;
        }
        if self.order >= self.orders.len() {
            // The song loops from its start, as FMOD's looping music does.
            self.order = 0;
            while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
                self.order += 1;
            }
        }
    }

    fn process_tick(&mut self) {
        if self.tick == 0 {
            self.process_row();
        } else {
            for index in 0..self.channels.len() {
                self.channel_tick(index);
            }
        }
        for index in 0..self.channels.len() {
            self.update_voice(index);
        }
        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.next_row();
        }
    }

    fn next_row(&mut self) {
        if let Some((order, row)) = self.jump.take() {
            self.order = order;
            self.row = row;
            self.skip_marker_orders();
            return;
        }
        self.row += 1;
        if self.row >= s3m::ROWS {
            self.row = 0;
            self.order += 1;
            self.skip_marker_orders();
        }
    }

    fn process_row(&mut self) {
        let Some(&pattern) = self.orders.get(self.order) else {
            return;
        };
        let cells = self.patterns[usize::from(pattern)].rows[self.row];
        for (index, cell) in cells.iter().enumerate() {
            if !self.channels[index].enabled {
                continue;
            }
            let channel = &mut self.channels[index];
            channel.command = cell.command;
            channel.info = cell.info;
            channel.period_delta = 0;
            if cell.command == command('S') && cell.info >> 4 == 0xD && cell.info & 0xF > 0 {
                channel.delayed = Some((cell.info & 0xF, *cell));
                continue;
            }
            channel.delayed = None;
            self.start_cell(index, cell);
            self.row_effect(index, cell);
        }
    }

    /// The note, instrument and volume of a cell.
    fn start_cell(&mut self, index: usize, cell: &Cell) {
        let tone_porta = cell.command == command('G');
        if cell.instrument != 0 {
            let channel = &mut self.channels[index];
            channel.instrument = cell.instrument;
            if let Some(Some(sample)) = self.samples.get(usize::from(cell.instrument) - 1) {
                channel.volume = sample.volume;
            }
        }
        if cell.note == NOTE_CUT {
            self.cut(index);
        } else if cell.note != NO_NOTE {
            let instrument = self.channels[index].instrument;
            if let Some(Some(sample)) = usize::from(instrument)
                .checked_sub(1)
                .and_then(|i| self.samples.get(i))
            {
                let period = note_period(cell.note, sample.c2spd);
                let channel = &mut self.channels[index];
                if tone_porta && channel.voice.is_some() {
                    channel.target_period = period;
                } else {
                    let offset = if cell.command == command('O') {
                        if cell.info != 0 {
                            channel.offset = cell.info;
                        }
                        u32::from(channel.offset) * 256
                    } else {
                        0
                    };
                    let voice = Voice::new(Arc::clone(&sample.data), sample.looping, offset);
                    if let Some(mut old) = channel.voice.replace(voice) {
                        old.release();
                        self.fading.push(old);
                    }
                    let channel = &mut self.channels[index];
                    channel.period = period;
                    channel.target_period = period;
                    channel.vibrato_position = 0;
                    channel.retrigger_count = 0;
                }
            } else if instrument != 0 && !(tone_porta && self.channels[index].voice.is_some()) {
                // FMOD plays a note of an empty sample slot as silence: the note sounding stops.
                self.cut(index);
            }
        }
        if let Some(volume) = cell.volume {
            self.channels[index].volume = i32::from(volume);
        }
    }

    fn cut(&mut self, index: usize) {
        if let Some(mut voice) = self.channels[index].voice.take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    /// Commands that act on the row's first tick.
    fn row_effect(&mut self, index: usize, cell: &Cell) {
        let info = cell.info;
        let channel = &mut self.channels[index];
        match letter(cell.command) {
            'A' if info > 0 => self.speed = info,
            'T' if info >= 0x20 => self.tempo = info,
            'B' => {
                let row = self.jump.map_or(0, |(_, row)| row);
                self.jump = Some((usize::from(info), row));
            }
            'C' => {
                let row = usize::from((info >> 4) * 10 + (info & 0xF)).min(s3m::ROWS - 1);
                let order = self.jump.map_or(self.order + 1, |(order, _)| order);
                self.jump = Some((order, row));
            }
            'D' | 'K' => {
                if info != 0 {
                    channel.volume_slide = info;
                }
                let slide = channel.volume_slide;
                // Fine slides (DxF up, DFy down) act once, on this tick.
                if slide & 0x0F == 0x0F && slide >> 4 != 0 {
                    channel.volume = (channel.volume + i32::from(slide >> 4)).min(64);
                } else if slide >> 4 == 0x0F && slide & 0x0F != 0 {
                    channel.volume = (channel.volume - i32::from(slide & 0x0F)).max(0);
                }
            }
            'E' | 'F' => {
                if info != 0 {
                    channel.porta = info;
                }
                let porta = channel.porta;
                let sign = if letter(cell.command) == 'E' { 1 } else { -1 };
                // EFx fine (x * 4), EEx extra fine (x), on this tick only.
                match porta >> 4 {
                    0xF => channel.period += sign * 4 * i32::from(porta & 0xF),
                    0xE => channel.period += sign * i32::from(porta & 0xF),
                    _ => {}
                }
                channel.period = channel.period.clamp(MIN_PERIOD, MAX_PERIOD);
            }
            'G' => {
                if info != 0 {
                    channel.tone_porta = info;
                }
            }
            'H' => {
                if info >> 4 != 0 {
                    channel.vibrato_speed = info >> 4;
                }
                if info & 0xF != 0 {
                    channel.vibrato_depth = info & 0xF;
                }
            }
            'Q' if info != 0 => channel.retrigger = info,
            _ => {}
        }
    }

    /// Commands that act on every tick but the first.
    fn channel_tick(&mut self, index: usize) {
        if !self.channels[index].enabled {
            return;
        }
        if let Some((at, cell)) = self.channels[index].delayed {
            if self.tick == at {
                self.channels[index].delayed = None;
                self.start_cell(index, &cell);
            }
            return;
        }
        let channel = &mut self.channels[index];
        channel.period_delta = 0;
        match letter(channel.command) {
            'D' => volume_slide(channel),
            'K' => {
                volume_slide(channel);
                vibrato_tick(channel);
            }
            'E' | 'F' => {
                let porta = channel.porta;
                if porta >> 4 < 0xE {
                    let sign = if letter(channel.command) == 'E' {
                        1
                    } else {
                        -1
                    };
                    channel.period = (channel.period + sign * 4 * i32::from(porta))
                        .clamp(MIN_PERIOD, MAX_PERIOD);
                }
            }
            'G' => {
                let speed = 4 * i32::from(channel.tone_porta);
                if channel.period < channel.target_period {
                    channel.period = (channel.period + speed).min(channel.target_period);
                } else {
                    channel.period = (channel.period - speed).max(channel.target_period);
                }
            }
            'H' => vibrato_tick(channel),
            // Without an interval nothing repeats, and nothing is counted.
            'Q' if channel.retrigger & 0xF != 0 => {
                let interval = channel.retrigger & 0xF;
                channel.retrigger_count += 1;
                if channel.retrigger_count >= interval {
                    channel.retrigger_count = 0;
                    channel.volume = retrigger_volume(channel.volume, channel.retrigger >> 4);
                    let sample = usize::from(channel.instrument)
                        .checked_sub(1)
                        .and_then(|i| self.samples.get(i))
                        .and_then(Option::as_ref);
                    if let Some(sample) = sample {
                        let voice = Voice::new(Arc::clone(&sample.data), sample.looping, 0);
                        if let Some(mut old) = channel.voice.replace(voice) {
                            old.release();
                            self.fading.push(old);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pushes the channel's pitch and volume into its voice.
    fn update_voice(&mut self, index: usize) {
        let global = i64::from(self.global_volume);
        let gain = self.gain;
        let channel = &mut self.channels[index];
        let Some(voice) = &mut channel.voice else {
            return;
        };
        let period = (channel.period + channel.period_delta).clamp(MIN_PERIOD, MAX_PERIOD);
        voice.set_frequency(CLOCK / u32::try_from(period).expect("positive"));
        let volume = i64::from(channel.volume) * global * gain / (64 * 64);
        let left = volume * (255 - channel.pan) / 255;
        let right = volume * channel.pan / 255;
        voice.set_volume(left, right);
    }

    #[cfg(test)]
    fn position(&self) -> (usize, usize, u8) {
        (self.order, self.row, self.tick)
    }
}

fn command(letter: char) -> u8 {
    letter as u8 - b'@'
}

fn letter(command: u8) -> char {
    if (1..=26).contains(&command) {
        char::from(b'@' + command)
    } else {
        ' '
    }
}

/// Scream Tracker's period of a note byte (octave in the high nibble) for a sample's C2SPD.
/// The octave shift comes last, so high notes keep their precision.
fn note_period(note: u8, c2spd: u32) -> i32 {
    let (octave, semitone) = (u32::from(note >> 4), usize::from(note & 0xF).min(11));
    let period =
        (8363 * 16 * u64::from(S3M_PERIODS[semitone]) / u64::from(c2spd.max(1))) >> octave.min(9);
    i32::try_from(period)
        .unwrap_or(MAX_PERIOD)
        .clamp(MIN_PERIOD, MAX_PERIOD)
}

fn volume_slide(channel: &mut Channel) {
    let slide = channel.volume_slide;
    let (up, down) = (slide >> 4, slide & 0x0F);
    if down == 0 && up != 0 {
        channel.volume = (channel.volume + i32::from(up)).min(64);
    } else if up == 0 && down != 0 {
        channel.volume = (channel.volume - i32::from(down)).max(0);
    }
}

fn vibrato_tick(channel: &mut Channel) {
    let wave = vibrato(channel.vibrato_position);
    channel.period_delta = (wave * i32::from(channel.vibrato_depth)) >> 5;
    channel.vibrato_position = (channel.vibrato_position + channel.vibrato_speed) & 63;
}

/// Qxy's volume change `x` applied to a volume 0..=64.
fn retrigger_volume(volume: i32, change: u8) -> i32 {
    let changed = match change {
        1..=5 => volume - (1 << (change - 1)),
        6 => volume * 2 / 3,
        7 => volume / 2,
        9..=13 => volume + (1 << (change - 9)),
        14 => volume * 3 / 2,
        15 => volume * 2,
        _ => volume,
    };
    changed.clamp(0, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{Channel as S3mChannel, Pattern, Sample};

    use crate::audio::mixer::UNITY;

    const C4: u8 = 0x40;

    fn module(rows: &[(usize, usize, Cell)], speed: u8, tempo: u8) -> Module {
        let mut pattern = Pattern {
            rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
        };
        for &(row, channel, cell) in rows {
            pattern.rows[row][channel] = cell;
        }
        let mut channels = [S3mChannel::default(); s3m::CHANNELS];
        channels[0] = S3mChannel {
            enabled: true,
            pan: 3,
        };
        channels[1] = S3mChannel {
            enabled: true,
            pan: 12,
        };
        Module {
            title: "Test".into(),
            orders: vec![0, 1],
            initial_speed: speed,
            initial_tempo: tempo,
            global_volume: 64,
            master_volume: 48,
            stereo: false,
            channels,
            samples: vec![Sample {
                name: "Tone".into(),
                c2spd: 8363,
                volume: 32,
                looped: Some((0, 100)),
                data: vec![10_000; 100],
            }],
            patterns: vec![pattern.clone(), pattern],
        }
    }

    fn cell(note: u8, instrument: u8, volume: Option<u8>, command: char, info: u8) -> Cell {
        Cell {
            note,
            instrument,
            volume,
            command: if command == ' ' {
                0
            } else {
                super::command(command)
            },
            info,
        }
    }

    fn play(music: &mut Music, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        music.mix_into(&mut out);
        out
    }

    #[test]
    fn a_tick_lasts_fmods_whole_number_of_samples() {
        // At tempo 125 a tick is 882 samples at 44.1 kHz, exactly 20 ms: 960 of ours.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 960 * 6);
        assert_eq!(music.position(), (0, 1, 0));
        // At tempo 141 FMOD counts 781 samples (not 781.9) per tick, which is why the menu
        // music runs 0.1 % fast in the original: 147 ticks are exactly 124 960 of our samples.
        let mut faster = Music::new(&module(&[], 1, 141), UNITY, 0);
        play(&mut faster, 124_960);
        assert_eq!(
            faster.position(),
            (0, 19, 0),
            "147 rows: both patterns, then the song loops to row 19 of its start"
        );
    }

    #[test]
    fn middle_c_plays_at_the_samples_c2spd() {
        assert_eq!(note_period(C4, 8363), 1712);
        assert_eq!(CLOCK / 1712, 8362);
        assert_eq!(
            note_period(0x50, 8363),
            856,
            "an octave up halves the period"
        );
        assert_eq!(
            note_period(C4, 16_726),
            856,
            "a doubled C2SPD sounds an octave up"
        );
    }

    #[test]
    fn notes_play_with_the_samples_volume_and_the_volume_column_overrides_it() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(C4, 1, Some(64), ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        let out = play(&mut music, 960);
        // Mono: centred, 32 / 64 of full volume on each side, and the module's master volume
        // 48 / 64 on top.
        let expected = 10_000 * 32 / 64 * 127 / 255 * 48 / 64;
        assert!(
            (out[2 * 900] - expected).abs() <= 2,
            "{} vs {expected}",
            out[2 * 900]
        );
        let louder = play(&mut music, 960);
        assert!(
            (louder[2 * 900] - 2 * expected).abs() <= 4,
            "{}",
            louder[2 * 900]
        );
    }

    #[test]
    fn speed_tempo_jump_and_break_commands_steer_the_song() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(NO_NOTE, 0, None, 'A', 2)),
                    (0, 1, cell(NO_NOTE, 0, None, 'C', 0x10)),
                ],
                6,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 2);
        assert_eq!(
            music.position(),
            (1, 10, 0),
            "speed 2, then a break to row 10 of the next order"
        );
        let mut jumping = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'B', 0))], 1, 125),
            UNITY,
            0,
        );
        play(&mut jumping, 960);
        assert_eq!(jumping.position(), (0, 0, 0), "B00 loops the first order");
        // As in minifmod, a new tempo already sets the length of the tick that sets it.
        let mut tempo = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'T', 250))], 1, 125),
            UNITY,
            0,
        );
        play(&mut tempo, 480 * 3);
        assert_eq!(
            tempo.position(),
            (0, 3, 0),
            "tempo 250: ticks of 480 samples"
        );
    }

    #[test]
    fn stereo_modules_play_each_channel_fully_on_its_side_and_twice_as_loud() {
        // FMOD 3 pans a stereo module's channels hard left or right, and plays them at twice a
        // mono module's level: only that matches the stereo image and the loudness of the
        // original's menu music, recorded at two music volumes (docs/verification/m1b.md).
        let mut stereo = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        stereo.stereo = true;
        let mut music = Music::new(&stereo, UNITY, 0);
        let out = play(&mut music, 960);
        let full = 10_000 * 48 / 64 * 2;
        assert!((out[2 * 900] - full).abs() <= 4, "left {}", out[2 * 900]);
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn a_song_can_start_at_a_later_order() {
        // The game starts the menu music at order 45 (musicSetOrder), not at its beginning.
        let mut later = module(&[], 1, 125);
        later.orders = vec![0, s3m::ORDER_END, 1];
        let music = Music::new(&later, UNITY, 2);
        assert_eq!(music.position(), (2, 0, 0));
        let at_marker = Music::new(&later, UNITY, 1);
        assert_eq!(at_marker.position(), (2, 0, 0), "a marker is skipped");
    }

    #[test]
    fn playback_runs_on_through_section_markers_as_fmod_does() {
        // FMOD drops 254 and 255 from the order list (the game numbers orders for
        // FMUSIC_SetOrder without them), so a pattern before a 255 is followed by the next
        // section, not by the song's start.
        let mut base = module(&[], 1, 125);
        base.orders = vec![0, s3m::ORDER_END, s3m::ORDER_SKIP, 1];
        let mut music = Music::new(&base, UNITY, 0);
        play(&mut music, 960 * s3m::ROWS);
        assert_eq!(music.position(), (3, 0, 0));
    }

    /// A module whose one sample rises steadily (0, 8, 16, ...), so the output shows how far
    /// into the sample a voice is.
    fn rising(rows: &[(usize, usize, Cell)], speed: u8) -> Module {
        let mut rising = module(rows, speed, 125);
        rising.samples[0].data = (0..4000).map(|i| i16::try_from(i * 8).unwrap()).collect();
        rising.samples[0].looped = None;
        rising
    }

    #[test]
    fn the_sample_offset_starts_a_note_further_into_its_sample() {
        // O (545 times in TR5) starts notes part way into their sample; ignoring it would play
        // the sample's beginning instead.
        let mut plain = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 6),
            UNITY,
            0,
        );
        let mut offset = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), 'O', 8))], 6),
            UNITY,
            0,
        );
        let (plain, offset) = (play(&mut plain, 300), play(&mut offset, 300));
        // 8 * 256 = 2048 samples in: about 2100 instead of about 50 at frame 299.
        assert!(plain[2 * 299] > 0);
        assert!(
            offset[2 * 299] > 20 * plain[2 * 299],
            "{} vs {}",
            offset[2 * 299],
            plain[2 * 299]
        );
    }

    #[test]
    fn vibrato_with_volume_slide_does_both() {
        // K (277 times in TR1) keeps an earlier H's vibrato going while it slides the volume;
        // doing only one of the two freezes the note or its level.
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, Some(10), 'H', 0x48)),
                    (1, 0, cell(NO_NOTE, 0, None, 'K', 0x20)),
                ],
                4,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        let position = music.channels[0].vibrato_position;
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        assert_eq!(
            music.channels[0].vibrato_position,
            position + 3 * 4,
            "the vibrato goes on at speed 4"
        );
    }

    #[test]
    fn portamento_up_lowers_the_period_and_its_fine_form_acts_once() {
        // F (252 times in TR5) slides notes up; the wrong direction, or FFx on every tick,
        // would detune whole phrases.
        let mut up = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut up, 960 * 3);
        assert_eq!(up.channels[0].period, 1712 - 2 * 8, "2 ticks of 4 * 2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 0xF3))], 3, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 3);
        assert_eq!(
            fine.channels[0].period,
            1712 - 3 * 4,
            "FF3: once, on the first tick"
        );
    }

    #[test]
    fn retrigger_restarts_the_note_at_its_interval_and_changes_its_volume() {
        // Q (90 times in TR9) drums a note several times per row; without the restarts, or the
        // volume change, a roll becomes one long note.
        let mut music = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(20), 'Q', 0xA3))], 7),
            UNITY,
            0,
        );
        let out = play(&mut music, 960 * 7);
        // Restarted at tick 3: 200 frames later a voice plays again, near the sample's start.
        let (restarted, before) = (out[2 * (3 * 960 + 200)], out[2 * (3 * 960 - 1)]);
        assert!(
            restarted > 0 && restarted < before / 4,
            "{restarted} vs {before}"
        );
        assert_eq!(
            music.channels[0].volume, 24,
            "+2 at each restart, on ticks 3 and 6"
        );
    }

    #[test]
    fn a_retrigger_without_an_interval_never_overflows() {
        // Q00 before any Q with an interval repeats nothing; counting its ticks anyway overflowed
        // after 255 of them and crashed the game in the middle of the music.
        let mut long = module(
            &[
                (0, 0, cell(C4, 1, Some(64), 'Q', 0)),
                (1, 0, cell(NO_NOTE, 0, None, 'Q', 0)),
            ],
            200,
            125,
        );
        long.orders = vec![0];
        let mut music = Music::new(&long, UNITY, 0);
        play(&mut music, 960 * 400);
        assert_eq!(music.position(), (0, 2, 0));
    }

    #[test]
    fn a_song_of_markers_only_stays_silent() {
        // An order list without a pattern must not hang the player looking for one.
        let mut base = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        base.orders = vec![s3m::ORDER_SKIP, s3m::ORDER_END];
        let mut music = Music::new(&base, UNITY, 0);
        assert!(play(&mut music, 960 * 4).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn volume_slides_act_after_the_first_tick_and_fine_ones_on_it() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x20))], 4, 125),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x3F))], 4, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 4);
        assert_eq!(fine.channels[0].volume, 13, "one fine step of +3");
    }

    #[test]
    fn portamentos_move_the_period() {
        let mut down = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'E', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut down, 960 * 3);
        assert_eq!(down.channels[0].period, 1712 + 2 * 8);
        let mut towards = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(0x50, 1, None, 'G', 0xFF)),
                ],
                3,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut towards, 960 * 6);
        assert_eq!(
            towards.channels[0].period, 856,
            "the tone portamento stops at its target"
        );
    }

    #[test]
    fn vibrato_wobbles_around_the_note() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'H', 0x48))], 6, 125),
            UNITY,
            0,
        );
        let mut deltas = Vec::new();
        for _ in 0..6 {
            play(&mut music, 960);
            deltas.push(music.channels[0].period_delta);
        }
        assert_eq!(deltas[0], 0, "no vibrato on the first tick");
        assert!(
            deltas[1..].iter().all(|&delta| delta >= 0) && deltas[5] > 0,
            "{deltas:?}"
        );
        assert_eq!(
            music.channels[0].period, 1712,
            "the note itself does not move"
        );
    }

    #[test]
    fn a_note_of_an_empty_sample_slot_silences_the_channel() {
        // The menu music's order 47 starts with notes of a sample slot its author emptied;
        // FMOD plays them as silence. Ignoring them left the channel's looping note playing,
        // brought back up by their volume: a stray tone in the menu.
        let mut emptied = module(
            &[
                (0, 0, cell(C4, 1, Some(64), ' ', 0)),
                (1, 0, cell(C4, 2, Some(32), ' ', 0)),
            ],
            1,
            125,
        );
        emptied.samples.push(Sample::default());
        let mut music = Music::new(&emptied, UNITY, 0);
        play(&mut music, 960);
        let out = play(&mut music, 960);
        assert!(out[2 * 900..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn note_delay_and_cut_and_retrigger() {
        let mut delayed = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'S', 0xD2))], 4, 125),
            UNITY,
            0,
        );
        play(&mut delayed, 960 * 2);
        assert!(delayed.channels[0].voice.is_none(), "not before tick 2");
        play(&mut delayed, 960);
        assert!(delayed.channels[0].voice.is_some());
        let mut cut = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(NOTE_CUT, 0, None, ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut cut, 960 * 2);
        assert!(cut.channels[0].voice.is_none());
        assert_eq!(retrigger_volume(40, 0xF), 64);
        assert_eq!(retrigger_volume(40, 0x7), 20);
        assert_eq!(retrigger_volume(40, 0x3), 36);
    }

    #[test]
    fn the_same_module_always_renders_the_same_samples() {
        let rows = [
            (0, 0, cell(C4, 1, Some(40), 'H', 0x46)),
            (8, 1, cell(0x45, 1, None, 'Q', 0x93)),
        ];
        let render = || {
            let mut music = Music::new(&module(&rows, 3, 131), UNITY, 0);
            play(&mut music, 48_000)
        };
        assert_eq!(render(), render());
    }
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (214 passed, 16 ignored).

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test --release -p deadrally-headless --test rendered_audio -- --ignored`
Expected: pass. The manifest renders the first 30 s of each module from order 0 and 10 s of the menu music after the intro; none of them reaches a note of an empty slot.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/audio/music.rs
git commit -m "fix: silence notes of empty sample slots"
```

---

### Task 2: Reading `dr.exe`

Spec 3.1 and 4.1. The original's strings and font metrics exist only in its executable. `exe` reads a PE file's section table and gives the bytes and NUL-terminated strings at virtual addresses; it never runs anything.

**Files:**
- Create: `crates/gamedata/src/exe.rs`
- Modify: `crates/gamedata/src/lib.rs`

**Interfaces:**
- Produces: `exe::Exe::parse(Vec<u8>) -> Result<Exe, ExeError>`; `Exe::bytes_at(&self, va: u32, len: usize) -> Result<&[u8], ExeError>`; `Exe::string_at(&self, va: u32, max: usize) -> Result<&[u8], ExeError>` (without the NUL); `ExeError::{NotPe(String), Address(u32), Unterminated(u32)}` with `Display`.

- [ ] **Step 1: Write the failing tests**

The test module comes first; Step 3 puts the code above it.

<!-- write: crates/gamedata/src/exe.rs -->
```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal 32-bit PE: image base 0x400000 and one section at 0x401000 (virtual size
    /// 0x200) whose file bytes, at offset 0x200, are `data`.
    pub(crate) fn build(data: &[u8]) -> Vec<u8> {
        build_at(0x1000, 0x200, data)
    }

    /// Like [`build`], with the section at relative address `rva` and `virtual_size` long.
    pub(crate) fn build_at(rva: u32, virtual_size: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xE0u16.to_le_bytes());
        let optional = 0x98;
        bytes[optional..optional + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        bytes[optional + 28..optional + 32].copy_from_slice(&0x40_0000u32.to_le_bytes());
        let section = optional + 0xE0;
        bytes[section..section + 5].copy_from_slice(b".data");
        bytes[section + 8..section + 12].copy_from_slice(&virtual_size.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&rva.to_le_bytes());
        bytes[section + 16..section + 20]
            .copy_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        bytes.extend_from_slice(data);
        bytes
    }

    #[test]
    fn strings_are_read_at_their_virtual_address() {
        // The game's texts are found by the addresses the original uses; reading the file
        // offset instead would land 0x400000 bytes off.
        let exe = Exe::parse(build(b"xyHello\0World\0")).unwrap();
        assert_eq!(exe.string_at(0x40_1002, 50).unwrap(), b"Hello");
        assert_eq!(exe.string_at(0x40_1008, 50).unwrap(), b"World");
        assert_eq!(exe.bytes_at(0x40_1000, 2).unwrap(), b"xy");
    }

    #[test]
    fn addresses_outside_the_file_and_unterminated_strings_are_errors() {
        // A different release keeps its strings elsewhere; reading garbage would show it.
        let exe = Exe::parse(build(b"abc\0defg")).unwrap();
        assert_eq!(
            exe.string_at(0x40_0000, 10),
            Err(ExeError::Address(0x40_0000))
        );
        assert_eq!(
            exe.string_at(0x40_1004, 10),
            Err(ExeError::Unterminated(0x40_1004))
        );
        assert_eq!(
            exe.string_at(0x40_1000, 2),
            Err(ExeError::Unterminated(0x40_1000))
        );
        assert_eq!(
            exe.bytes_at(0x40_1006, 4),
            Err(ExeError::Address(0x40_1006))
        );
    }

    #[test]
    fn other_files_are_not_executables() {
        assert!(matches!(
            Exe::parse(b"BM".to_vec()),
            Err(ExeError::NotPe(_))
        ));
        let mut no_pe = build(b"");
        no_pe[0x80] = b'X';
        assert!(matches!(Exe::parse(no_pe), Err(ExeError::NotPe(_))));
    }
}
```

<!-- write: crates/gamedata/src/lib.rs -->
```rust
//! Finds, validates and decodes the player's copy of the original game data (M0 spec section 6,
//! M1a spec section 4, M1b spec section 4.1, M2a spec section 4.1).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

pub mod assets;
pub mod bmp;
pub mod bpa;
pub mod bpk;
pub mod catalog;
pub mod cmf;
mod config;
pub mod exe;
pub mod haf;
pub mod image;
mod known_versions;
mod locate;
mod lzw;
pub mod s3m;
pub mod sound;
pub mod track;
mod validate;
pub mod xm;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use lzw::LzwError;
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-gamedata --lib exe`
Expected: FAIL to compile (`Exe`, `ExeError` do not exist yet).

- [ ] **Step 3: Implement**

<!-- prepend: crates/gamedata/src/exe.rs -->
```rust
//! The Windows `dr.exe` as a data file (spec M2a §3.1, §4.1): its strings and tables are read
//! at the virtual addresses the original uses. It is only read, never run.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExeError {
    /// Not a 32-bit PE file, or its headers do not hold together.
    NotPe(String),
    /// Nothing of the file lies at this address.
    Address(u32),
    /// The string at this address does not end within the bytes allowed for it.
    Unterminated(u32),
}

impl fmt::Display for ExeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExeError::NotPe(problem) => write!(f, "not a Windows executable: {problem}"),
            ExeError::Address(address) => {
                write!(f, "the executable has no data at address {address:#x}")
            }
            ExeError::Unterminated(address) => {
                write!(f, "the string at address {address:#x} does not end")
            }
        }
    }
}

impl std::error::Error for ExeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Section {
    /// Virtual address (image base included) and size in memory.
    address: u32,
    virtual_size: u32,
    /// Where its bytes are in the file, and how many the file holds.
    file_offset: usize,
    file_size: usize,
}

/// A PE file's bytes and its sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exe {
    bytes: Vec<u8>,
    sections: Vec<Section>,
}

impl Exe {
    /// # Errors
    ///
    /// [`ExeError::NotPe`] when the bytes are not a PE file with 32-bit optional headers.
    pub fn parse(bytes: Vec<u8>) -> Result<Exe, ExeError> {
        let bad = |problem: &str| ExeError::NotPe(problem.to_owned());
        let u16_at = |offset: usize| {
            bytes
                .get(offset..offset + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or_else(|| bad("the headers run past the end"))
        };
        let u32_at = |offset: usize| {
            bytes
                .get(offset..offset + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| bad("the headers run past the end"))
        };
        if !bytes.starts_with(b"MZ") {
            return Err(bad("no MZ header"));
        }
        let pe = u32_at(0x3C)? as usize;
        if bytes.get(pe..pe + 4) != Some(b"PE\0\0".as_slice()) {
            return Err(bad("no PE signature"));
        }
        let section_count = usize::from(u16_at(pe + 6)?);
        let optional_size = usize::from(u16_at(pe + 20)?);
        let optional = pe + 24;
        if u16_at(optional)? != 0x10B {
            return Err(bad("not a 32-bit executable"));
        }
        let image_base = u32_at(optional + 28)?;
        let table = optional + optional_size;
        let sections = (0..section_count)
            .map(|index| {
                let at = table + 40 * index;
                Ok(Section {
                    address: image_base
                        .checked_add(u32_at(at + 12)?)
                        .ok_or_else(|| bad("a section lies past 4 GB"))?,
                    virtual_size: u32_at(at + 8)?,
                    file_offset: u32_at(at + 20)? as usize,
                    file_size: u32_at(at + 16)? as usize,
                })
            })
            .collect::<Result<Vec<_>, ExeError>>()?;
        Ok(Exe { bytes, sections })
    }

    /// The file's bytes from virtual address `address` to the end of its section.
    fn rest_of_section(&self, address: u32) -> Option<&[u8]> {
        self.sections.iter().find_map(|section| {
            let offset = address.checked_sub(section.address)? as usize;
            let end = section.file_size.min(section.virtual_size as usize);
            (offset < end)
                .then(|| {
                    self.bytes
                        .get(section.file_offset + offset..section.file_offset + end)
                })
                .flatten()
        })
    }

    /// The `length` bytes at virtual address `address`, which must lie in one section's bytes
    /// in the file.
    ///
    /// # Errors
    ///
    /// [`ExeError::Address`] when they do not.
    pub fn bytes_at(&self, address: u32, length: usize) -> Result<&[u8], ExeError> {
        self.rest_of_section(address)
            .and_then(|rest| rest.get(..length))
            .ok_or(ExeError::Address(address))
    }

    /// The NUL-terminated string at `address`, without its NUL, at most `max` bytes long.
    ///
    /// # Errors
    ///
    /// [`ExeError::Address`] when nothing lies there, [`ExeError::Unterminated`] when no NUL
    /// follows within `max` bytes.
    pub fn string_at(&self, address: u32, max: usize) -> Result<&[u8], ExeError> {
        let rest = self
            .rest_of_section(address)
            .ok_or(ExeError::Address(address))?;
        let window = &rest[..rest.len().min(max.saturating_add(1))];
        let end = window
            .iter()
            .position(|&b| b == 0)
            .ok_or(ExeError::Unterminated(address))?;
        Ok(&window[..end])
    }
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (217 passed, 16 ignored).

- [ ] **Step 5: Commit**

```bash
git add crates/gamedata
git commit -m "feat: read sections and strings of dr.exe"
```

---

### Task 3: The menu texts

Spec 3.1. The menu text table (9 menus of 9 rows, 50 bytes each, at 0x446368), the four start-up lines of the bottom panel, the exit question, "yes" and "no", and the three font metric tables, each checked to be terminated and printable (0xFA, the 1-pixel gap, allowed).

**Files:**
- Create: `crates/gamedata/src/text.rs`
- Modify: `crates/gamedata/src/lib.rs`

**Interfaces:**
- Consumes: `exe::Exe` (Task 2).
- Produces: `text::Texts { menus: Vec<Vec<Vec<u8>>>, panel: Vec<Vec<u8>>, exit_question: Vec<u8>, yes: Vec<u8>, no: Vec<u8>, big: Metrics, small: Metrics, medium: Metrics }` with `Texts::read(&Exe) -> Result<Texts, TextError>`; `text::Metrics { width: u8, height: u8, advances: Vec<u8> }` (advance of character *c* at `advances[c − 32]`); `text::GAP = 0xFA`; `TextError::{Exe(ExeError), Unprintable { address: u32 }}`.

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/gamedata/src/text.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::exe::tests::{build, build_at};

    /// A section from 0x443000 to 0x448000 with every string and table where the known release
    /// keeps it: menu `m` row `r` reads "m.r", the other strings their address's last digit.
    fn known_layout() -> Vec<u8> {
        let mut data = vec![0u8; 0x5000];
        let mut put = |address: u32, bytes: &[u8]| {
            let at = (address - 0x44_3000) as usize;
            data[at..at + bytes.len()].copy_from_slice(bytes);
        };
        for menu in 0..MENUS as u32 {
            for row in 0..MENU_ROWS as u32 {
                let address = MENU_TABLE + MENU_ROW_BYTES * (9 * menu + row);
                put(address, format!("{menu}.{row}").as_bytes());
            }
        }
        for (index, &address) in PANEL_LINES.iter().enumerate() {
            put(address, format!("line {index}").as_bytes());
        }
        put(EXIT_QUESTION, b"Quit?");
        put(YES, b"Y");
        put(NO, b"N");
        put(BIG_METRICS, &[32, 32, 20, 9]);
        put(SMALL_METRICS, &[16, 16, 10, 5]);
        put(MEDIUM_METRICS, &[9, 12, 9, 9]);
        build_at(0x4_3000, 0x5000, &data)
    }

    #[test]
    fn menu_rows_are_fifty_bytes_apart_and_menus_nine_rows() {
        // A wrong stride shows another menu's text, or half of two rows.
        let texts = Texts::read(&Exe::parse(known_layout()).unwrap()).unwrap();
        assert_eq!(texts.menus[0][0], b"0.0");
        assert_eq!(texts.menus[3][4], b"3.4");
        assert_eq!(texts.menus[8][8], b"8.8");
        assert_eq!(texts.panel[3], b"line 3");
        assert_eq!(
            (texts.exit_question.as_slice(), texts.yes.as_slice()),
            (b"Quit?".as_slice(), b"Y".as_slice())
        );
        assert_eq!(
            (texts.big.width, texts.big.height, &texts.big.advances[..2]),
            (32, 32, [20, 9].as_slice())
        );
        assert_eq!(texts.small.advances.len(), 96);
        assert_eq!(
            texts.medium.advances.len(),
            62,
            "the medium font has 62 glyphs"
        );
    }

    #[test]
    fn a_byte_the_original_never_uses_is_refused_with_its_address() {
        let mut bytes = known_layout();
        let at = bytes.len() - 0x5000 + (YES - 0x44_3000) as usize;
        bytes[at] = 0x01;
        assert_eq!(
            Texts::read(&Exe::parse(bytes).unwrap()),
            Err(TextError::Unprintable { address: YES })
        );
    }

    #[test]
    fn a_file_that_is_not_the_known_release_is_refused_with_the_address() {
        // Another executable keeps different bytes at these addresses; drawing them would fill
        // the menus with garbage instead of saying what is wrong.
        let exe = Exe::parse(build(&[0x7F; 0x200])).unwrap();
        let error = Texts::read(&exe).unwrap_err();
        assert!(
            matches!(error, TextError::Exe(ExeError::Address(_))),
            "{error}"
        );
    }
}
```

<!-- write: crates/gamedata/src/lib.rs -->
```rust
//! Finds, validates and decodes the player's copy of the original game data (M0 spec section 6,
//! M1a spec section 4, M1b spec section 4.1, M2a spec section 4.1).
//!
//! Data files are only ever opened for reading; nothing is written into the data directory.

pub mod assets;
pub mod bmp;
pub mod bpa;
pub mod bpk;
pub mod catalog;
pub mod cmf;
mod config;
pub mod exe;
pub mod haf;
pub mod image;
mod known_versions;
mod locate;
mod lzw;
pub mod s3m;
pub mod sound;
pub mod text;
pub mod track;
mod validate;
pub mod xm;

pub use config::{Config, ConfigError, config_path, load_config};
pub use known_versions::{KNOWN_VERSIONS, KnownFile, KnownVersion, REQUIRED_FILES};
pub use locate::{DATA_ENV_VAR, DataSource, LocateError, Located, STEAM_SUBDIR, locate};
pub use lzw::LzwError;
pub use validate::{FileReport, Outcome, Validation, ValidationError, validate};
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-gamedata --lib text`
Expected: FAIL to compile (`Texts`, `TextError` do not exist yet).

- [ ] **Step 3: Implement**

<!-- prepend: crates/gamedata/src/text.rs -->
```rust
//! The original's menu strings and font metrics, read from the player's `dr.exe` at the known
//! release's addresses (spec M2a §3.1). DeadRally ships none of the game's text.

use std::fmt;

use crate::exe::{Exe, ExeError};

/// Menus in the text table, and rows per menu.
pub const MENUS: usize = 9;
pub const MENU_ROWS: usize = 9;
/// The byte that draws nothing and moves the pen one pixel.
pub const GAP: u8 = 0xFA;

/// The menu text table: 50 bytes per row (`dr.exe` 0x446368).
const MENU_TABLE: u32 = 0x44_6368;
const MENU_ROW_BYTES: u32 = 50;
/// The bottom panel's start-up lines, in the order `mainMenu` (0x43A020) adds them; an empty
/// line comes between the third and the fourth.
const PANEL_LINES: [u32; 4] = [0x44_4370, 0x44_433C, 0x44_4300, 0x44_42C0];
const EXIT_QUESTION: u32 = 0x44_42B0;
const YES: u32 = 0x44_3CDC;
const NO: u32 = 0x44_3CD8;
/// Font descriptors: width, height, then one advance per glyph from character 32.
const BIG_METRICS: u32 = 0x44_5848;
const SMALL_METRICS: u32 = 0x44_58B0;
const MEDIUM_METRICS: u32 = 0x44_5928;
/// The longest string read anywhere but the menu table.
const MAX_LINE: usize = 150;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    Exe(ExeError),
    /// A byte that is neither printable ASCII nor the gap.
    Unprintable {
        address: u32,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextError::Exe(error) => write!(f, "dr.exe: {error}"),
            TextError::Unprintable { address } => write!(
                f,
                "dr.exe: the text at address {address:#x} is not the original's; is this the known release?"
            ),
        }
    }
}

impl std::error::Error for TextError {}

impl From<ExeError> for TextError {
    fn from(error: ExeError) -> TextError {
        TextError::Exe(error)
    }
}

/// A font's cell size and the pen advance of each glyph, character 32 first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub width: u8,
    pub height: u8,
    pub advances: Vec<u8>,
}

/// Everything M2a reads from `dr.exe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texts {
    /// `menus[m][r]`: row `r` of menu `m`, empty where the menu has no such row.
    pub menus: Vec<Vec<Vec<u8>>>,
    /// The bottom panel's four start-up lines.
    pub panel: Vec<Vec<u8>>,
    pub exit_question: Vec<u8>,
    pub yes: Vec<u8>,
    pub no: Vec<u8>,
    pub big: Metrics,
    pub small: Metrics,
    pub medium: Metrics,
}

impl Texts {
    /// # Errors
    ///
    /// [`TextError`] when a string is missing, does not end, or holds a byte the original's
    /// strings never do: the executable is not the known release.
    pub fn read(exe: &Exe) -> Result<Texts, TextError> {
        let text = |address: u32, max: usize| -> Result<Vec<u8>, TextError> {
            let bytes = exe.string_at(address, max)?;
            if bytes.iter().all(|&b| (32..127).contains(&b) || b == GAP) {
                Ok(bytes.to_vec())
            } else {
                Err(TextError::Unprintable { address })
            }
        };
        let menus = (0..MENUS as u32)
            .map(|menu| {
                (0..MENU_ROWS as u32)
                    .map(|row| {
                        let address = MENU_TABLE + MENU_ROW_BYTES * (MENU_ROWS as u32 * menu + row);
                        text(address, MENU_ROW_BYTES as usize - 1)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metrics = |address: u32, glyphs: usize| -> Result<Metrics, TextError> {
            let bytes = exe.bytes_at(address, 2 + glyphs)?;
            Ok(Metrics {
                width: bytes[0],
                height: bytes[1],
                advances: bytes[2..].to_vec(),
            })
        };
        Ok(Texts {
            menus,
            panel: PANEL_LINES
                .iter()
                .map(|&address| text(address, MAX_LINE))
                .collect::<Result<Vec<_>, _>>()?,
            exit_question: text(EXIT_QUESTION, MAX_LINE)?,
            yes: text(YES, MAX_LINE)?,
            no: text(NO, MAX_LINE)?,
            big: metrics(BIG_METRICS, 96)?,
            small: metrics(SMALL_METRICS, 96)?,
            medium: metrics(MEDIUM_METRICS, 62)?,
        })
    }
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (220 passed, 16 ignored).

- [ ] **Step 5: Commit**

```bash
git add crates/gamedata
git commit -m "feat: read the menu texts from dr.exe"
```

---

### Task 4: The main menu's data

Spec 2 (decision 2) and 4.1. `DR.EXE` joins the required files and the known release; `Assets` gains `MenuAssets`. `END.BMP` must be 640x480, the menu's whole screen; the catalogue fixes every other picture's size. The core's startup tests build `Assets` by hand, so they gain the menu's assets from a shared synthetic fixture whose pictures each have a colour of their own.

**Files:**
- Create: `crates/core/tests/common/mod.rs`
- Modify: `crates/gamedata/src/known_versions.rs`, `crates/gamedata/src/assets.rs`, `crates/gamedata/tests/catalog_data.rs`, `crates/headless/tests/cli.rs`, `crates/core/tests/startup.rs`

**Interfaces:**
- Consumes: `Texts::read` (Task 3), `bmp::decode`, `sound::load_effects`, `catalog` (M1a, M1b).
- Produces:
  - `REQUIRED_FILES: [&str; 20]`, `"DR.EXE"` last; the known release's `DR.EXE` (365 952 bytes, SHA-256 `54fe789f…be58c`).
  - `assets::MenuAssets { background: Image, panel_line: Image, corners_focused: Vec<Image>, corners_unfocused: Vec<Image>, cursor: Vec<Image>, big_a, big_b, big_d, small_a, small_b, small_c: Vec<Image>, palette: Palette, copper: Palette, background_copper: Vec<[u8; 3]>, credits: Vec<Picture>, end: Picture, effects: Bank, texts: Texts }`; `Assets { menu: MenuAssets, .. }`; `AssetError::{Exe(ExeError), Text(TextError), Size { name, expected, actual }}`.
  - Test fixture `common::menu_assets() -> MenuAssets` and its colours `BACKGROUND`, `BIG_A`, `BIG_B`, `BIG_D`, `SMALL`, `CURSOR`, `CREDITS`, `END` and sounds `BACK_SOUND`, `MOVE_SOUND`, `CHOOSE_SOUND` (left only, right only, both sides).

- [ ] **Step 1: Write the failing tests**

<!-- write: crates/gamedata/tests/catalog_data.rs -->
```rust
//! The decoders and the catalogue against the developer's real game data. Run with
//! `cargo test-data`; the tests read DEADRALLY_DATA and fail (never pass silently) when it is
//! unset.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::catalog::{self, Layout};
use deadrally_gamedata::haf::{Animation, FRAME_HEIGHT, FRAME_WIDTH};
use deadrally_gamedata::track::TrackInfo;
use deadrally_gamedata::{DATA_ENV_VAR, bmp, locate};
use sha2::{Digest, Sha256};

const ARCHIVES: [&str; 13] = [
    "ENGINE.BPA",
    "IBFILES.BPA",
    "MENU.BPA",
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
];

fn data_dir() -> PathBuf {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    locate(Some(&dir), None, None)
        .unwrap_or_else(|error| panic!("{error}"))
        .validation
        .dir
}

fn archive(name: &str) -> Archive {
    Archive::open(&data_dir().join(name)).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_archive_has_its_documented_entries() {
    // Opening checks that the sizes add up to the file; the counts catch a misread directory.
    let counts = [
        ("ENGINE.BPA", 39),
        ("IBFILES.BPA", 17),
        ("MENU.BPA", 167),
        ("MUSICS.BPA", 16),
    ];
    for (name, count) in counts {
        assert_eq!(archive(name).names().count(), count, "{name}");
    }
    assert_eq!(archive("TR0.BPA").names().count(), 12);
    for track in 1..10 {
        assert_eq!(
            archive(&format!("TR{track}.BPA")).names().count(),
            13,
            "TR{track}"
        );
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_animations_have_their_documented_frames_and_length() {
    // The intro's length in ticks is what its music (M1b) is cut to.
    for (name, frames, ticks) in [
        ("SANIM.haf", 1626, 5732),
        ("ENDANI.haf", 368, 1620),
        ("ENDANI0.HAF", 383, 1915),
    ] {
        let animation = Animation::open(&data_dir().join(name)).unwrap();
        assert_eq!(animation.len(), frames, "{name}");
        let total: u32 = animation.delays.iter().map(|&delay| u32::from(delay)).sum();
        assert_eq!(total, ticks, "{name}");
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_bpk_entry_is_catalogued_or_explained() {
    // An uncatalogued image could never be drawn or dumped; the gap would surface only when a
    // later milestone needs it.
    let mut missing = Vec::new();
    for name in ARCHIVES {
        for entry in archive(name).names() {
            let explained = catalog::NOT_IMAGES
                .iter()
                .any(|not| not.archive == name && not.name == entry);
            if entry.ends_with(".BPK") && catalog::find(name, entry).is_none() && !explained {
                missing.push(format!("{name}/{entry}"));
            }
        }
    }
    assert!(missing.is_empty(), "uncatalogued: {missing:?}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn every_catalogued_image_decodes_to_its_shape() {
    // A wrong width with the right total size is caught by the dump and the reference
    // screenshots; a wrong total size is caught here.
    let mut failures = Vec::new();
    for name in ARCHIVES {
        let archive = archive(name);
        for entry in catalog::IMAGES.iter().filter(|entry| entry.archive == name) {
            let bytes = archive
                .read(entry.name)
                .unwrap_or_else(|error| panic!("{error}"));
            match entry.decode(bytes) {
                Ok(frames) if frames.len() == entry.frames as usize => {}
                Ok(frames) => failures.push(format!("{}: {} frames", entry.name, frames.len())),
                Err(error) => failures.push(format!("{}: {error}", entry.name)),
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn track_images_match_their_info_files() {
    // The race (M4) takes the track size from INF.BIN; the images must agree with it.
    for track in 0..10 {
        let archive = archive(&format!("TR{track}.BPA"));
        let info = TrackInfo::parse(archive.read(&format!("TR{track}-INF.BIN")).unwrap()).unwrap();
        for part in ["IMA", "MAS", "VAI", "LR1"] {
            let entry =
                catalog::find(&format!("TR{track}.BPA"), &format!("TR{track}-{part}.BPK")).unwrap();
            assert_eq!(entry.layout, Layout::Rix3Track);
            let divisor = if matches!(part, "VAI" | "LR1") { 4 } else { 1 };
            assert_eq!(
                (entry.width, entry.height),
                (info.width / divisor, info.height / divisor),
                "TR{track}-{part}"
            );
        }
    }
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_startup_assets_load_with_their_documented_shapes() {
    // The game refuses to start when these fail, so a wrong shape here is a broken start.
    let dir = data_dir();
    let validation = deadrally_gamedata::validate(&dir).unwrap();
    let assets = deadrally_gamedata::assets::Assets::load(&validation)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(assets.intro.len(), 1626);
    let size =
        |picture: &deadrally_gamedata::assets::Picture| (picture.image.width, picture.image.height);
    assert_eq!(size(&assets.letterbox), (320, 200));
    assert_eq!(size(&assets.apogee), (640, 480));
    assert_eq!(size(&assets.remedy), (640, 480));
    assert_eq!(size(&assets.title), (640, 480));
    assert_eq!(assets.intro_music.orders.len(), 42);
    assert_eq!(assets.intro_effects.instruments.len(), 40);
    assert_eq!(assets.menu_music.orders.len(), 94);
    let menu = &assets.menu;
    assert_eq!((menu.background.width, menu.background.height), (640, 480));
    assert_eq!((menu.panel_line.width, menu.panel_line.height), (640, 10));
    assert_eq!(
        (menu.corners_focused.len(), menu.corners_unfocused.len()),
        (4, 4)
    );
    assert_eq!(menu.cursor.len(), 50);
    for font in [
        &menu.big_a,
        &menu.big_b,
        &menu.big_d,
        &menu.small_a,
        &menu.small_b,
        &menu.small_c,
    ] {
        assert_eq!(font.len(), 96);
    }
    assert_eq!(menu.background_copper.len(), 512);
    assert_eq!(menu.credits.len(), 2);
    assert_eq!(size(&menu.end), (640, 480));
    assert_eq!(menu.effects.instruments.len(), 31);
    // Row counts of the main menu and the start submenu, from dr.exe.
    let rows = |m: usize| {
        menu.texts.menus[m]
            .iter()
            .filter(|row| !row.is_empty())
            .count()
    };
    assert_eq!((rows(0), rows(1)), (6, 6));
    assert_eq!(menu.texts.panel.len(), 4);
    assert_eq!((menu.texts.big.width, menu.texts.small.height), (32, 16));
}

/// One line per decoded picture: the SHA-256 of its frames' pixels (palettes first where the
/// file stores them), its name and its shape as width x height x frames. The shape is part of
/// the picture: the same bytes at another width draw a skewed sprite.
fn manifest(dir: &Path) -> String {
    let mut lines = String::new();
    let mut line = |name: &str, shape: (u32, u32, usize), hasher: Sha256| {
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let (width, height, frames) = shape;
        writeln!(lines, "{hash}  {name} {width}x{height}x{frames}").unwrap();
    };
    for name in ARCHIVES {
        let archive = archive(name);
        for entry in catalog::IMAGES.iter().filter(|entry| entry.archive == name) {
            let bytes = archive.read(entry.name).unwrap();
            let mut hasher = Sha256::new();
            if let Some(palette) = entry.embedded_palette(bytes).unwrap() {
                hasher.update(palette.0.as_flattened());
            }
            let frames = entry.decode(bytes).unwrap();
            for frame in &frames {
                hasher.update(&frame.pixels);
            }
            let shape = (frames[0].width, frames[0].height, frames.len());
            line(&format!("{name}/{}", entry.name), shape, hasher);
        }
    }
    for name in ["SANIM.haf", "ENDANI.haf", "ENDANI0.HAF"] {
        let animation = Animation::open(&dir.join(name)).unwrap();
        let mut hasher = Sha256::new();
        for index in 0..animation.len() {
            let frame = animation
                .frame(index)
                .unwrap_or_else(|error| panic!("{error}"));
            hasher.update(frame.palette.0.as_flattened());
            hasher.update(&frame.pixels);
        }
        line(name, (FRAME_WIDTH, FRAME_HEIGHT, animation.len()), hasher);
    }
    for name in ["rmd.bmp", "end.bmp"] {
        let (image, palette) = bmp::decode(&std::fs::read(dir.join(name)).unwrap()).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(palette.0.as_flattened());
        hasher.update(&image.pixels);
        line(name, (image.width, image.height, 1), hasher);
    }
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn decoded_pictures_match_the_committed_manifest() {
    // The manifest was written after the pictures were checked by eye and against the original
    // (docs/verification/m1a.md); a decoder change must not alter any of them unnoticed.
    let actual = manifest(&data_dir());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/decoded-images.sha256");
    if std::env::var_os("DEADRALLY_BLESS").is_some() {
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    let only_in = |these: &str, those: &str| -> Vec<String> {
        these
            .lines()
            .filter(|line| !those.lines().any(|other| other == *line))
            .map(str::to_owned)
            .collect()
    };
    let (new, gone) = (only_in(&actual, &expected), only_in(&expected, &actual));
    assert!(
        new.is_empty() && gone.is_empty(),
        "decoded pictures differ from {}:\ndecoded now:\n{}\nin the manifest:\n{}\nIf the \
         change is intended, check the pictures again and rewrite the manifest with \
         DEADRALLY_BLESS=1 cargo test-data",
        path.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
```

<!-- write: crates/headless/tests/cli.rs -->
```rust
//! The headless binary as CI and developers use it (spec section 8).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::{TempDir, tempdir};

const REQUIRED_FILES: [&str; 20] = deadrally_gamedata::REQUIRED_FILES;

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

fn write_png(path: &Path, width: u32, height: u32, rgb: &[u8]) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgb).unwrap();
    writer.finish().unwrap();
}

#[test]
fn compare_succeeds_only_for_identical_pictures() {
    // The verification scripts rely on the exit status; "close enough" must fail.
    let home = tempdir().unwrap();
    let path = |name: &str| home.path().join(name);
    write_png(&path("a.png"), 2, 1, &[0, 0, 0, 10, 10, 10]);
    write_png(&path("b.png"), 2, 1, &[0, 0, 0, 10, 11, 10]);
    write_png(&path("c.png"), 1, 2, &[0, 0, 0, 10, 10, 10]);
    let compare = |a: &str, b: &str| {
        run(
            &home,
            &[
                "compare",
                path(a).to_str().unwrap(),
                path(b).to_str().unwrap(),
            ],
        )
    };

    let same = compare("a.png", "a.png");
    assert_eq!(same.status.code(), Some(0));
    assert!(text(&same.stdout).contains("0 pixels differ"));

    let close = compare("a.png", "b.png");
    assert_eq!(close.status.code(), Some(1));
    assert!(
        text(&close.stdout).contains("1 pixels differ, largest channel difference 1"),
        "{}",
        text(&close.stdout)
    );

    let other_size = compare("a.png", "c.png");
    assert_eq!(other_size.status.code(), Some(1));
    assert!(text(&other_size.stdout).contains("sizes differ"));
}

/// A 48 kHz WAV of a 440 Hz tone whose loudness changes every 200 ms without repeating, so
/// `compare-audio` can line two of them up in one place only.
fn write_tone(path: &Path, seconds: u32, gain: f64, channels: u16) {
    const RATE: u32 = 48_000;
    let mut data = Vec::new();
    for i in 0..seconds * RATE {
        let t = f64::from(i) / f64::from(RATE);
        let segment = (t * 5.0) as u64;
        let mixed = segment
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407)
            >> 33;
        let level = 0.1 + 0.8 * (mixed % 1000) as f64 / 1000.0;
        let value =
            (gain * level * (2.0 * std::f64::consts::PI * 440.0 * t).sin() * 32767.0) as i16;
        for _ in 0..channels {
            data.extend_from_slice(&value.to_le_bytes());
        }
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    let block = 2 * u32::from(channels);
    for field in [
        16,
        1 | u32::from(channels) << 16,
        RATE,
        RATE * block,
        block | 16 << 16,
    ] {
        bytes.extend_from_slice(&field.to_le_bytes());
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data);
    fs::write(path, bytes).unwrap();
}

#[test]
fn compare_audio_passes_only_within_the_tolerances() {
    // The sound checks rely on the exit status: a quieter render, too short an overlap or a
    // measure that could not be taken must fail, not pass with a remark.
    let home = tempdir().unwrap();
    let path = |name: &str| home.path().join(name);
    write_tone(&path("original.wav"), 40, 1.0, 2);
    write_tone(&path("quieter.wav"), 40, 0.5, 2);
    write_tone(&path("brief.wav"), 15, 1.0, 2);
    write_tone(&path("mono.wav"), 40, 1.0, 1);
    let compare = |ours: &str, options: &[&str]| {
        let (original, ours) = (path("original.wav"), path(ours));
        let mut args = vec![
            "compare-audio",
            original.to_str().unwrap(),
            ours.to_str().unwrap(),
        ];
        args.extend_from_slice(options);
        run(&home, &args)
    };

    let same = compare("original.wav", &[]);
    assert_eq!(same.status.code(), Some(0), "{}", text(&same.stdout));
    assert!(text(&same.stdout).contains("result: PASS"));

    let quieter = compare("quieter.wav", &[]);
    assert_eq!(quieter.status.code(), Some(1), "{}", text(&quieter.stdout));
    assert!(text(&quieter.stdout).contains("median loudness difference above"));

    let short = compare("original.wav", &["--min-overlap", "50"]);
    assert_eq!(short.status.code(), Some(1), "{}", text(&short.stdout));
    assert!(text(&short.stdout).contains("overlap below 50 s"));

    // 15 s hold one 10 s piece: too few to measure a tempo on.
    let brief = compare("brief.wav", &["--min-overlap", "10"]);
    assert_eq!(brief.status.code(), Some(1), "{}", text(&brief.stdout));
    assert!(text(&brief.stdout).contains("tempo measured on fewer than 3 pieces"));

    // A mono file has no stereo image to compare.
    let mono = compare("mono.wav", &[]);
    assert_eq!(mono.status.code(), Some(1), "{}", text(&mono.stdout));
    assert!(text(&mono.stdout).contains("stereo balance not measured"));
}

#[test]
fn find_rejects_a_screenshot_that_is_not_window_sized() {
    // A shot of the whole virtual screen or with window decorations can never match; say so
    // instead of searching for minutes.
    let home = tempdir().unwrap();
    let shot = home.path().join("shot.png");
    write_png(&shot, 2, 1, &[0; 6]);
    let output = run(&home, &["find", shot.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text(&output.stderr).contains("the original's window is 640x480"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn dump_assets_refuses_to_write_into_the_game_data() {
    // The install must stay exactly as the player's copy is; dumps go elsewhere.
    let home = tempdir().unwrap();
    let data = home.path().join("data");
    fake_install(&data);
    let out = data.join("dumps");
    let output = run(
        &home,
        &[
            "dump-assets",
            "--data",
            data.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("inside the game data directory"));
    assert!(!out.exists());
}

#[test]
fn dump_assets_refuses_the_steam_folder_above_the_data() {
    // With Steam's layout the data sits one level down; the folder the player named is still
    // the game's install and must not fill up with dumps.
    let home = tempdir().unwrap();
    let named = home.path().join("Death Rally");
    fake_install(&named.join("Death Rally"));
    let out = named.join("dumps");
    let output = run(
        &home,
        &[
            "dump-assets",
            "--data",
            named.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    assert!(
        text(&output.stderr).contains("game's install"),
        "{}",
        text(&output.stderr)
    );
    assert!(!out.exists());
}

fn data_env() -> std::ffi::OsString {
    std::env::var_os("DEADRALLY_DATA")
        .filter(|value| !value.is_empty())
        .expect(
            "DEADRALLY_DATA is not set: point it at your Death Rally data to run `cargo test-data`",
        )
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn dump_assets_writes_one_png_per_catalogued_image() {
    let home = tempdir().unwrap();
    let out = home.path().join("dumps");
    let output = headless(&home)
        .args(["dump-assets", "--out", out.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let count = deadrally_gamedata::catalog::IMAGES.len();
    assert!(text(&output.stdout).contains(&format!("wrote {count} images")));
    let written = walk_pngs(&out);
    assert_eq!(written, count);
    assert!(out.join("MENU/APOGEE.png").is_file());
}

fn walk_pngs(dir: &Path) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| {
            if path.is_dir() {
                walk_pngs(&path)
            } else {
                usize::from(path.extension().is_some_and(|ext| ext == "png"))
            }
        })
        .sum()
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn find_locates_a_rendered_frame_in_the_startup_sequence() {
    // find is how screenshots of the original are matched; it must at least find our own
    // frames, at the tick they were rendered.
    let home = tempdir().unwrap();
    let shot = home.path().join("tick-1000.png");
    let render = headless(&home)
        .args(["render", "--tick", "1000", "--out", shot.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(render.status.code(), Some(0), "{}", text(&render.stderr));
    let found = headless(&home)
        .args(["find", "--ticks", "1100", shot.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(found.status.code(), Some(0), "{}", text(&found.stdout));
    let line = text(&found.stdout);
    let ticks = line.split("ticks ").nth(1).unwrap().trim();
    let (first, last) = ticks.split_once('-').unwrap_or((ticks, ticks));
    let (first, last): (u64, u64) = (first.parse().unwrap(), last.parse().unwrap());
    assert!((first..=last).contains(&1000), "{line}");
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn render_audio_writes_the_whole_intro_and_the_music_after_it_as_a_48_khz_wav() {
    // compare-audio and the owner's listening use this file: another rate would shift every
    // pitch, and a short file would hide how the intro ends and the menu music starts.
    let home = tempdir().unwrap();
    let out = home.path().join("startup.wav");
    let output = headless(&home)
        .args(["render-audio", "--startup", "--out", out.to_str().unwrap()])
        .env("DEADRALLY_DATA", data_env())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let bytes = fs::read(&out).unwrap();
    let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2, "channels");
    assert_eq!(field(24), 48_000, "rate");
    // The intro's 5732 ticks and 2 s (143 ticks) after it, 672 frames of 4 bytes each.
    assert_eq!(field(40), (5732 + 143) * 672 * 4, "data bytes");
}
```

<!-- write: crates/core/tests/common/mod.rs -->
```rust
//! Synthetic menu assets: every picture is one colour of its own, so a test can tell from a
//! pixel which picture, font or cursor frame the menu drew there. Real game data is never
//! committed.

use deadrally_gamedata::assets::{MenuAssets, Picture};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::text::{Metrics, Texts};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// `MENUBG5`, white in `MENU.PAL`.
pub const BACKGROUND: u8 = 1;
/// The big fonts: selected row (A), active row (B), dim row (D).
pub const BIG_A: u8 = 50;
pub const BIG_B: u8 = 51;
pub const BIG_D: u8 = 52;
/// The small fonts A, B and C.
pub const SMALL: [u8; 3] = [60, 61, 62];
/// Cursor frame k is colour `CURSOR + k`.
pub const CURSOR: u8 = 100;
/// The two credits screens and the end screen, each full red, green or blue in its palette.
pub const CREDITS: [u8; 2] = [200, 201];
pub const END: u8 = 202;
/// The menu's effects (`MEN-SAM`): the back sound plays on the left only, the move sound on
/// the right only, the choose sound on both sides.
pub const BACK_SOUND: usize = 22;
pub const MOVE_SOUND: usize = 25;
pub const CHOOSE_SOUND: usize = 28;

fn solid(width: u32, height: u32, colour: u8) -> Image {
    Image::new(width, height, vec![colour; (width * height) as usize])
}

fn glyphs(size: u32, colour: u8) -> Vec<Image> {
    (0..96).map(|_| solid(size, size, colour)).collect()
}

fn full_screen(colour: u8, rgb: [u8; 3]) -> Picture {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(colour)] = rgb;
    Picture {
        image: solid(640, 480, colour),
        palette,
    }
}

/// A short tone, panned (255 left, 0 right, 128 both).
fn sound(panning: u8) -> Instrument {
    Instrument {
        name: "Beep".into(),
        data: vec![2000; 2000],
        looping: Looping::None,
        volume: 64,
        finetune: 0,
        relative_note: 0,
        panning,
        fadeout: 0,
    }
}

fn texts() -> Texts {
    let metrics = |size: u8| Metrics {
        width: size,
        height: size,
        advances: vec![size; 96],
    };
    Texts {
        // Every menu has six one-letter rows.
        menus: vec![
            (0..9)
                .map(|row| if row < 6 { b"M".to_vec() } else { Vec::new() })
                .collect();
            9
        ],
        panel: (0..4).map(|line| vec![b'a' + line]).collect(),
        exit_question: b"?".to_vec(),
        yes: b"Y".to_vec(),
        no: b"N".to_vec(),
        big: metrics(32),
        small: metrics(16),
        medium: metrics(9),
    }
}

pub fn menu_assets() -> MenuAssets {
    let mut palette = Palette::BLACK;
    palette.0[usize::from(BACKGROUND)] = [63, 63, 63];
    // The pulsing entries.
    palette.0[16..32].fill([63, 63, 63]);
    let mut copper = Palette::BLACK;
    copper.0[0] = [63, 0, 32];
    let mut effects = vec![None; CHOOSE_SOUND];
    effects[BACK_SOUND - 1] = Some(sound(255));
    effects[MOVE_SOUND - 1] = Some(sound(0));
    effects[CHOOSE_SOUND - 1] = Some(sound(128));
    MenuAssets {
        background: solid(640, 480, BACKGROUND),
        panel_line: solid(640, 10, 99),
        corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
        corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
        cursor: (0..50).map(|k| solid(20, 20, CURSOR + k)).collect(),
        big_a: glyphs(32, BIG_A),
        big_b: glyphs(32, BIG_B),
        big_d: glyphs(32, BIG_D),
        small_a: glyphs(16, SMALL[0]),
        small_b: glyphs(16, SMALL[1]),
        small_c: glyphs(16, SMALL[2]),
        palette,
        copper,
        background_copper: (0..512).map(|row| [(row % 64) as u8, 0, 0]).collect(),
        credits: vec![
            full_screen(CREDITS[0], [63, 0, 0]),
            full_screen(CREDITS[1], [0, 63, 0]),
        ],
        end: full_screen(END, [0, 0, 63]),
        effects: Bank {
            linear_frequencies: true,
            instruments: effects,
        },
        texts: texts(),
    }
}
```

<!-- write: crates/core/tests/startup.rs -->
```rust
//! The startup sequence's timeline on synthetic assets (spec M1a §5.2). Each test pins down
//! something a player of the original would notice: a logo that holds too long, a fade that
//! ends at the wrong brightness, a key that does not skip.

mod common;

use deadrally_core::{AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, Game, InputEvent, Key, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use std::path::PathBuf;

use deadrally_gamedata::haf::{Animation, FRAME_PIXELS, HafFrame};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// Intro delays: frame 0 at tick 4, frame 1 at tick 6, the last frame at tick 9.
const DELAYS: [u8; 3] = [4, 2, 3];
const INTRO_END: u32 = 9;
/// Pixel values that tell the pictures apart; each picture's palette makes its own colour
/// full red, green or blue.
const APOGEE: u8 = 1;
const REMEDY: u8 = 2;
const TITLE: u8 = 3;
/// A logo takes 25 fade-in ticks, 180 hold ticks and 26 fade-out ticks.
const FADE_IN: u32 = 25;
const HOLD: u32 = 180;
const LOGO: u32 = FADE_IN + HOLD + 26;

fn palette(entries: &[(usize, [u8; 3])]) -> Palette {
    let mut palette = Palette::BLACK;
    for &(index, rgb) in entries {
        palette.0[index] = rgb;
    }
    palette
}

fn picture(pixel: u8, rgb: [u8; 3]) -> Picture {
    Picture {
        image: Image::new(4, 3, vec![pixel; 12]),
        palette: palette(&[(usize::from(pixel), rgb)]),
    }
}

/// Frame `k` is all pixel `16 + k`, coloured grey level `10 + k`.
fn intro_frame(k: u8) -> HafFrame {
    HafFrame {
        // Entries below 16 belong to the letterbox; the intro must not take them from frames.
        palette: palette(&[(0, [63, 63, 63]), (usize::from(16 + k), [10 + k; 3])]),
        pixels: vec![16 + k; FRAME_PIXELS],
    }
}

fn assets() -> Assets {
    let mut letterbox = vec![0u8; 320 * 200];
    letterbox[..320].fill(5);
    Assets {
        intro: Animation::from_frames(DELAYS.to_vec(), (0..3).map(intro_frame).collect()),
        letterbox: Picture {
            image: Image::new(320, 200, letterbox),
            // Entries from 16 on must stay black until the first frame sets them.
            palette: palette(&[(5, [20, 30, 40]), (16, [63, 63, 63])]),
        },
        apogee: picture(APOGEE, [63, 0, 0]),
        remedy: picture(REMEDY, [0, 63, 0]),
        title: picture(TITLE, [0, 0, 63]),
        intro_music: music(false),
        intro_effects: effects(),
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

/// A module that is silent, or plays one endless tone from its first row.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let mut pattern = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    pattern.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone { vec![0] } else { Vec::new() },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![pattern],
    }
}

/// Music that is silent from its first order and plays a tone from order 45, where the original
/// starts the menu music.
fn menu_music() -> Module {
    let mut module = music(true);
    module.patterns.push(s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    });
    module.orders = [vec![1; 45], vec![0]].concat();
    module
}

/// Effect 1 is an endless constant tone.
fn effects() -> Bank {
    Bank {
        linear_frequencies: true,
        instruments: vec![Some(Instrument {
            name: "Hum".into(),
            data: vec![1000; 1000],
            looping: Looping::Forward {
                start: 0,
                length: 1000,
            },
            volume: 64,
            finetune: 0,
            relative_note: 0,
            panning: 128,
            fadeout: 0,
        })],
    }
}

/// The last left sample of each of the next `ticks` ticks.
fn loudness(game: &mut Game, ticks: u32) -> Vec<i16> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let mut audio = Vec::new();
            game.take_audio(&mut audio);
            audio[audio.len() - 2]
        })
        .collect()
}

fn press(game: &mut Game) {
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: true,
    });
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: false,
    });
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// Which picture is on screen and how bright its colour is (0..=63).
fn shown(game: &Game) -> (u8, u8) {
    let frame = game.frame();
    let pixel = frame.pixels[0];
    let brightness = frame.palette[usize::from(pixel)].into_iter().max().unwrap();
    (pixel, brightness)
}

/// The intro row shown at row 40 (the first animation row) and its colour.
fn intro_row(game: &Game) -> (u8, [u8; 3]) {
    let frame = game.frame();
    assert_eq!((frame.width, frame.height), (320, 200));
    let pixel = frame.pixels[40 * 320];
    (pixel, frame.palette[usize::from(pixel)])
}

#[test]
fn the_intro_starts_with_the_letterbox_and_black_animation_colours() {
    let game = Game::new(assets());
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (320, 200, (4, 3))
    );
    assert_eq!(frame.pixels[0], 5);
    assert_eq!(frame.palette[5], [20, 30, 40]);
    assert_eq!(frame.palette[16], [0, 0, 0]);
}

#[test]
fn each_intro_frame_appears_its_delay_after_the_previous_one() {
    // The intro is cut to its music (M1b); a frame early or late drifts out of sync.
    let mut game = Game::new(assets());
    run(&mut game, 3);
    assert_eq!(intro_row(&game), (0, [0, 0, 0]), "nothing before tick 4");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (16, [10, 10, 10]), "frame 0 at tick 4");
    assert_eq!(
        game.frame().palette[0],
        [0, 0, 0],
        "entries below 16 stay the letterbox's"
    );
    run(&mut game, 1);
    assert_eq!(intro_row(&game).0, 16, "frame 0 still at tick 5");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (17, [11, 11, 11]), "frame 1 at tick 6");
}

#[test]
fn the_last_intro_frame_is_never_shown() {
    // openAnimation blacks the palette as soon as the last frame is drawn, before it is shown.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END - 1);
    assert_eq!(intro_row(&game).0, 17);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn a_key_ends_the_intro_when_the_next_frame_is_due() {
    // The original checks for a key once per frame, so the intro runs on until the next frame
    // would have been shown.
    let mut game = Game::new(assets());
    run(&mut game, 1);
    press(&mut game);
    run(&mut game, 2);
    assert_eq!(intro_row(&game).0, 0, "still waiting for frame 0");
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0), "frame 0 is skipped too");
}

#[test]
fn a_corrupt_intro_frame_ends_the_intro_instead_of_crashing() {
    // Only data of an unknown version can hold one, and the player was warned at start-up;
    // the game should still reach its menus. The broken frame comes first: the last frame is
    // never decoded, so it could not show the problem.
    let mut record = vec![0u8; 768];
    record.extend([8, 2, 0xFF, 0xFF, 0, 0x3B]);
    let mut haf = vec![2, 0, 0, 0, 1, 1];
    for _ in 0..2 {
        haf.extend(u16::try_from(record.len()).unwrap().to_le_bytes());
        haf.extend(&record);
    }
    let mut broken = assets();
    broken.intro = Animation::from_bytes(PathBuf::from("BROKEN.HAF"), haf).unwrap();
    assert!(broken.intro.frame(0).is_err());
    let mut game = Game::new(broken);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn an_empty_intro_goes_straight_to_the_logos() {
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    let mut game = Game::new(empty);
    assert_eq!(shown(&game), (APOGEE, 0));
    run(&mut game, FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
}

#[test]
fn an_empty_intro_still_starts_the_menu_music() {
    // The original starts the menu music after `checkAndOpenAnimation`, whether or not that
    // played anything.
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    empty.menu_music = menu_music();
    let mut game = Game::new(empty);
    assert!(loudness(&mut game, 3).iter().all(|&level| level > 0));
}

#[test]
fn a_pad_button_skips_like_a_key() {
    let mut game = Game::new(assets());
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    assert_eq!(shown(&game).0, APOGEE);
}

/// Red brightness of the Apogee logo for each tick after the intro.
fn apogee_brightness(game: &mut Game, ticks: u32) -> Vec<u8> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let (pixel, brightness) = shown(game);
            assert_eq!(pixel, APOGEE);
            brightness
        })
        .collect()
}

#[test]
fn a_logo_fades_in_holds_and_fades_out_like_the_original() {
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END);
    let brightness = apogee_brightness(&mut game, LOGO - 1);
    // The fade-in climbs one 4 % step per tick from black and stops at 96 %: 63 shows as 60.
    assert_eq!(&brightness[..4], [0, 3, 5, 8]);
    assert_eq!(brightness[24], 60);
    // The hold lasts 180 ticks when nobody presses a key.
    assert!(brightness[25..205].iter().all(|&level| level == 60));
    // The fade-out starts at 100 %, a visible flash from 60 to 63, and steps down to black.
    assert_eq!(&brightness[205..208], [63, 60, 58]);
    assert_eq!(brightness[229], 3);
    // Its last, black step already has Remedy drawn under it: the same black screen.
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 0));
}

#[test]
fn a_key_during_the_fade_in_ends_the_hold_after_one_tick() {
    // The original remembers the press until the hold asks, so impatient players see the logo
    // at full fade for a single tick.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + 5);
    press(&mut game);
    let brightness = apogee_brightness(&mut game, FADE_IN - 5 + 1);
    assert_eq!(brightness.last(), Some(&60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63), "the fade-out starts");
}

#[test]
fn a_key_during_a_fade_out_ends_the_next_logos_hold_after_one_tick() {
    // The remembered press survives the change of screen, as in the original: a player who
    // presses while Apogee fades out sees the Remedy logo for a single hold tick.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + FADE_IN + HOLD + 10);
    assert_eq!(shown(&game).0, APOGEE, "Apogee is fading out");
    press(&mut game);
    run(&mut game, LOGO - FADE_IN - HOLD - 10 + FADE_IN);
    assert_eq!(shown(&game), (REMEDY, 60), "Remedy's fade-in is done");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 63), "the fade-out starts");
}

#[test]
fn a_key_with_the_last_intro_frame_carries_into_the_apogee_hold() {
    // openAnimation stops after its last frame without checking for a key, so the press waits
    // for the Apogee hold.
    let mut game = Game::new(assets());
    run(&mut game, 7);
    press(&mut game);
    run(&mut game, INTRO_END - 7 + FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn a_key_during_the_hold_starts_the_fade_out_on_the_next_tick() {
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + FADE_IN + 50);
    press(&mut game);
    game.tick();
    assert_eq!(
        shown(&game),
        (APOGEE, 60),
        "the hold tick that reads the key"
    );
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn the_title_fades_in_after_both_logos_and_stays_at_92_percent() {
    // Measured on the original: it sets the last fade step and then loads the main menu
    // without showing another frame, so the title never gets brighter than 92 % (63 as 58).
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + 2 * LOGO);
    assert_eq!(shown(&game), (TITLE, 0));
    run(&mut game, FADE_IN - 1);
    assert_eq!(shown(&game), (TITLE, 58));
    // M2's main menu continues from here; until then nothing else happens, keys included.
    press(&mut game);
    run(&mut game, 1_000);
    assert_eq!(shown(&game), (TITLE, 58));
}

#[test]
fn the_audio_stream_stays_full_when_nothing_plays() {
    // The frontend paces itself on the audio queue; missing samples would stall or drift it.
    let mut game = Game::new(assets());
    run(&mut game, 30);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), 30 * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
    assert!(
        audio.iter().all(|&sample| sample == 0),
        "no music, no effects in these assets"
    );
}

#[test]
fn the_intro_plays_its_music_and_stops_it_when_it_ends() {
    // The original stops the song as the intro ends; music running on would play over the
    // logos.
    let mut with_music = assets();
    with_music.intro_music = music(true);
    let mut game = Game::new(with_music);
    let during = loudness(&mut game, INTRO_END - 1);
    assert!(during.iter().all(|&level| level > 0), "{during:?}");
    let after = loudness(&mut game, 5);
    assert_eq!(&after[1..], [0, 0, 0, 0], "{after:?}");
}

#[test]
fn the_menu_music_starts_when_the_intro_ends_and_plays_through_the_logos() {
    // `mainMenu` starts the menu music from order 45 right after the intro, before the logos;
    // they and the title are not silent.
    let mut with_menu = assets();
    with_menu.menu_music = menu_music();
    let mut game = Game::new(with_menu);
    let intro = loudness(&mut game, INTRO_END - 1);
    assert!(intro.iter().all(|&level| level == 0), "{intro:?}");
    let after = loudness(&mut game, LOGO + 10);
    assert!(after[1..].iter().all(|&level| level > 0), "{after:?}");
    assert_eq!(
        shown(&game).0,
        REMEDY,
        "the music goes on through the logos"
    );
}

#[test]
fn the_menu_music_plays_at_the_default_configurations_half_volume() {
    // The intro always plays at full volume: the original applies dr.cfg's volumes only when it
    // starts the menu music, and a fresh dr.cfg has the music at 50 % (FMOD master volume 63
    // against the intro's 127).
    let mut same = assets();
    let mut tone = music(true);
    tone.orders = vec![0; 46];
    same.intro_music = tone.clone();
    same.menu_music = tone;
    let mut game = Game::new(same);
    let intro = i32::from(loudness(&mut game, INTRO_END - 1)[2]);
    let menu = i32::from(loudness(&mut game, 5)[4]);
    assert!(intro > 0, "{intro}");
    assert!(
        (menu * 127 - intro * 63).abs() <= 127 * 2,
        "{menu} vs {intro}"
    );
}

#[test]
fn a_key_that_ends_the_intro_stops_its_sound() {
    // As in the original, the intro's music stops, and frames that were still due never start
    // their effects (the menu music, silent in these assets, takes over).
    let mut with_music = assets();
    with_music.intro_music = music(true);
    with_music.intro.effects = vec![1, 1, 1];
    let mut game = Game::new(with_music);
    run(&mut game, 1);
    press(&mut game);
    loudness(&mut game, 3);
    let after = loudness(&mut game, 10);
    assert!(after[1..].iter().all(|&level| level == 0), "{after:?}");
}

#[test]
fn a_frames_effect_sounds_when_the_frame_is_shown() {
    // Effects mark moments of the intro's picture; one early or late is out of sync with it.
    let mut timed = assets();
    timed.intro.effects = vec![0, 1, 0];
    let mut game = Game::new(timed);
    let levels = loudness(&mut game, 6);
    assert_eq!(&levels[..5], [0, 0, 0, 0, 0], "frame 1 appears at tick 6");
    assert!(levels[5] > 0);
}

#[test]
fn the_intros_effects_take_channels_one_to_six_in_turn() {
    // The seventh effect reuses channel 1 and cuts the first one off, so at most six effects
    // sound together, as in the original.
    let mut busy = assets();
    busy.intro = Animation::from_frames(vec![1; 10], (0..10).map(|k| intro_frame(k % 3)).collect());
    busy.intro.effects = vec![1; 10];
    let mut game = Game::new(busy);
    let levels = loudness(&mut game, 9);
    let one = i32::from(levels[0]);
    assert!(one > 0);
    for (voices, &level) in levels.iter().enumerate().take(6) {
        let expected = one * (voices as i32 + 1);
        assert!(
            (i32::from(level) - expected).abs() <= 6,
            "{voices}: {levels:?}"
        );
    }
    assert!(
        (i32::from(levels[7]) - 6 * one).abs() <= 6,
        "still six voices: {levels:?}"
    );
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --workspace --no-run`
Expected: FAIL to compile (`MenuAssets` does not exist; `REQUIRED_FILES` has 19 entries).

- [ ] **Step 3: Implement**

<!-- write: crates/gamedata/src/known_versions.rs -->
```rust
/// The data files the game needs (brief §9), in canonical upper case. Names on disk match
/// case-insensitively. `DR.EXE` holds the original's texts (spec M2a §3.1); it is read, never
/// run.
pub const REQUIRED_FILES: [&str; 20] = [
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
    "DR.EXE",
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
            name: "DR.EXE",
            size: 365_952,
            sha256: "54fe789faca583d67b8e73e7c58908f3f1468c5c8f75942239a60483ae9be58c",
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

<!-- write: crates/gamedata/src/assets.rs -->
```rust
//! The decoded data the startup sequence and the main menu need (spec M1a §4.2, M1b §4.1,
//! M2a §4.1).

use std::fmt;
use std::path::PathBuf;

use crate::bmp::{self, BmpError};
use crate::bpa::{Archive, BpaError};
use crate::catalog::{self, CatalogError};
use crate::exe::{Exe, ExeError};
use crate::haf::{Animation, HafError};
use crate::image::{Image, Palette, PaletteError};
use crate::s3m::Module;
use crate::sound::{self, SoundError};
use crate::text::{TextError, Texts};
use crate::validate::Validation;
use crate::xm::Bank;

/// An image with the palette it is shown with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub image: Image,
    pub palette: Palette,
}

/// Everything the startup sequence shows.
#[derive(Debug)]
pub struct Assets {
    /// `SANIM.haf`.
    pub intro: Animation,
    /// `FRAMES.BPK`: the intro's 320x200 letterbox; the game uses palette entries 0..=15.
    pub letterbox: Picture,
    /// `APOGEE.BPK` with `APOGEE.PAL`.
    pub apogee: Picture,
    /// `rmd.bmp`.
    pub remedy: Picture,
    /// `STARTSCR.BPK` with `STARTSCR.PAL`.
    pub title: Picture,
    /// `TR0-MUS.CMF`: the music under the intro.
    pub intro_music: Module,
    /// `SANIM-E.CMF`: the intro's effects, numbered as in `SANIM.haf`'s table.
    pub intro_effects: Bank,
    /// `MEN-MUS.CMF`: the music that starts when the intro ends and goes on into the menus.
    pub menu_music: Module,
    pub menu: MenuAssets,
}

/// What the main menu draws and plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAssets {
    /// `MENUBG5.BPK`, 640x480.
    pub background: Image,
    /// `CHATLIN1.BPK`, 640x10: the bottom panel's frame lines.
    pub panel_line: Image,
    /// `CORN3A.BPK` and `CORN3B.BPK`: popup corners (top left, top right, bottom left, bottom
    /// right) of a focused and of an unfocused popup.
    pub corners_focused: Vec<Image>,
    pub corners_unfocused: Vec<Image>,
    /// `CURSOR.BPK`: 50 frames, 20x20.
    pub cursor: Vec<Image>,
    /// `F-BIG3A`, `-B`, `-D` (32x32) and `F-SMA3A`, `-B`, `-C` (16x16): 96 glyphs each.
    pub big_a: Vec<Image>,
    pub big_b: Vec<Image>,
    pub big_d: Vec<Image>,
    pub small_a: Vec<Image>,
    pub small_b: Vec<Image>,
    pub small_c: Vec<Image>,
    /// `MENU.PAL`.
    pub palette: Palette,
    /// `COPPER.PAL`: one colour per player colour, whose ramps the menu palette gets.
    pub copper: Palette,
    /// `BGCOP.PAL`: 512 colours for the background copper rows.
    pub background_copper: Vec<[u8; 3]>,
    /// `CREDIT1.BPK` and `CREDIT2.BPK` with their palettes.
    pub credits: Vec<Picture>,
    /// `end.bmp`: the screen the game ends on.
    pub end: Picture,
    /// `MEN-SAM.CMF`: the menus' effects.
    pub effects: Bank,
    /// The strings and font metrics in `dr.exe`.
    pub texts: Texts,
}

#[derive(Debug)]
pub enum AssetError {
    Archive(BpaError),
    Image {
        name: &'static str,
        error: CatalogError,
    },
    Palette {
        name: &'static str,
        error: PaletteError,
    },
    Bmp {
        path: PathBuf,
        error: BmpError,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Animation(HafError),
    Sound(SoundError),
    Exe(ExeError),
    Text(TextError),
    Size {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Archive(error) => write!(f, "{error}"),
            AssetError::Image { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Palette { name, error } => write!(f, "MENU.BPA/{name}: {error}"),
            AssetError::Bmp { path, error } => write!(f, "{}: {error}", path.display()),
            AssetError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            AssetError::Animation(error) => write!(f, "{error}"),
            AssetError::Sound(error) => write!(f, "{error}"),
            AssetError::Exe(error) => write!(f, "dr.exe: {error}"),
            AssetError::Text(error) => write!(f, "{error}"),
            AssetError::Size {
                name,
                expected,
                actual,
            } => write!(f, "MENU.BPA/{name}: {actual} bytes, not {expected}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<BpaError> for AssetError {
    fn from(error: BpaError) -> AssetError {
        AssetError::Archive(error)
    }
}

impl Assets {
    /// Loads the startup sequence's data from a validated data directory.
    ///
    /// # Errors
    ///
    /// [`AssetError`] naming the file and entry that failed.
    pub fn load(validation: &Validation) -> Result<Assets, AssetError> {
        let path = |name: &str| -> PathBuf {
            validation
                .files
                .iter()
                .find(|file| file.name == name)
                .map(|file| file.path.clone())
                .unwrap_or_else(|| panic!("{name} is a required file, so validation found it"))
        };
        let menu = Archive::open(&path("MENU.BPA"))?;
        let musics = Archive::open(&path(sound::ARCHIVE))?;
        let remedy_path = path("RMD.BMP");
        let remedy_bytes = std::fs::read(&remedy_path).map_err(|source| AssetError::Read {
            path: remedy_path.clone(),
            source,
        })?;
        let (image, palette) = bmp::decode(&remedy_bytes).map_err(|error| AssetError::Bmp {
            path: remedy_path,
            error,
        })?;
        Ok(Assets {
            intro: Animation::open(&path("SANIM.HAF")).map_err(AssetError::Animation)?,
            letterbox: letterbox(&menu)?,
            apogee: picture(&menu, "APOGEE.BPK", "APOGEE.PAL")?,
            remedy: Picture { image, palette },
            title: picture(&menu, "STARTSCR.BPK", "STARTSCR.PAL")?,
            intro_music: sound::load_music(&musics, "TR0-MUS.CMF").map_err(AssetError::Sound)?,
            intro_effects: sound::load_effects(&musics, "SANIM-E.CMF")
                .map_err(AssetError::Sound)?,
            menu_music: sound::load_music(&musics, "MEN-MUS.CMF").map_err(AssetError::Sound)?,
            menu: menu_assets(&menu, &musics, &path("END.BMP"), &path("DR.EXE"))?,
        })
    }
}

/// All frames of a catalogued `MENU.BPA` image.
fn frames(menu: &Archive, name: &'static str) -> Result<Vec<Image>, AssetError> {
    let entry = catalog::find("MENU.BPA", name).expect("menu images are catalogued");
    entry
        .decode(menu.read(name)?)
        .map_err(|error| AssetError::Image { name, error })
}

fn palette(menu: &Archive, name: &'static str) -> Result<Palette, AssetError> {
    Palette::from_bytes(menu.read(name)?).map_err(|error| AssetError::Palette { name, error })
}

fn bmp_picture(path: &std::path::Path) -> Result<Picture, AssetError> {
    let bytes = std::fs::read(path).map_err(|source| AssetError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let (image, palette) = bmp::decode(&bytes).map_err(|error| AssetError::Bmp {
        path: path.to_path_buf(),
        error,
    })?;
    Ok(Picture { image, palette })
}

/// A picture the menu copies over its whole screen, which must be 640x480.
fn full_screen(picture: Picture, path: &std::path::Path) -> Result<Picture, AssetError> {
    let (width, height) = (picture.image.width, picture.image.height);
    if (width, height) == (640, 480) {
        Ok(picture)
    } else {
        Err(AssetError::Bmp {
            path: path.to_path_buf(),
            error: BmpError::Unsupported(format!("{width}x{height}, not 640x480")),
        })
    }
}

fn menu_assets(
    menu: &Archive,
    musics: &Archive,
    end: &std::path::Path,
    exe: &std::path::Path,
) -> Result<MenuAssets, AssetError> {
    const BGCOP: &str = "BGCOP.PAL";
    let background_copper = menu.read(BGCOP)?;
    // 512 colours of 6-bit components.
    if background_copper.len() != 3 * 512 {
        return Err(AssetError::Size {
            name: BGCOP,
            expected: 3 * 512,
            actual: background_copper.len(),
        });
    }
    if let Some((index, &value)) = background_copper.iter().enumerate().find(|(_, c)| **c > 63) {
        return Err(AssetError::Palette {
            name: BGCOP,
            error: PaletteError::NotSixBit { index, value },
        });
    }
    let exe_bytes = std::fs::read(exe).map_err(|source| AssetError::Read {
        path: exe.to_path_buf(),
        source,
    })?;
    let exe = Exe::parse(exe_bytes).map_err(AssetError::Exe)?;
    Ok(MenuAssets {
        background: frames(menu, "MENUBG5.BPK")?.remove(0),
        panel_line: frames(menu, "CHATLIN1.BPK")?.remove(0),
        corners_focused: frames(menu, "CORN3A.BPK")?,
        corners_unfocused: frames(menu, "CORN3B.BPK")?,
        cursor: frames(menu, "CURSOR.BPK")?,
        big_a: frames(menu, "F-BIG3A.BPK")?,
        big_b: frames(menu, "F-BIG3B.BPK")?,
        big_d: frames(menu, "F-BIG3D.BPK")?,
        small_a: frames(menu, "F-SMA3A.BPK")?,
        small_b: frames(menu, "F-SMA3B.BPK")?,
        small_c: frames(menu, "F-SMA3C.BPK")?,
        palette: palette(menu, "MENU.PAL")?,
        copper: palette(menu, "COPPER.PAL")?,
        background_copper: background_copper.as_chunks::<3>().0.to_vec(),
        credits: vec![
            picture(menu, "CREDIT1.BPK", "CREDIT1.PAL")?,
            picture(menu, "CREDIT2.BPK", "CREDIT2.PAL")?,
        ],
        end: full_screen(bmp_picture(end)?, end)?,
        effects: sound::load_effects(musics, "MEN-SAM.CMF").map_err(AssetError::Sound)?,
        texts: Texts::read(&exe).map_err(AssetError::Text)?,
    })
}

fn picture(
    menu: &Archive,
    image: &'static str,
    palette: &'static str,
) -> Result<Picture, AssetError> {
    let entry = catalog::find("MENU.BPA", image).expect("these images are catalogued");
    let mut frames = entry
        .decode(menu.read(image)?)
        .map_err(|error| AssetError::Image { name: image, error })?;
    let palette =
        Palette::from_bytes(menu.read(palette)?).map_err(|error| AssetError::Palette {
            name: palette,
            error,
        })?;
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

fn letterbox(menu: &Archive) -> Result<Picture, AssetError> {
    const NAME: &str = "FRAMES.BPK";
    let entry = catalog::find("MENU.BPA", NAME).expect("FRAMES.BPK is catalogued");
    let bytes = menu.read(NAME)?;
    let image_error = |error| AssetError::Image { name: NAME, error };
    let mut frames = entry.decode(bytes).map_err(image_error)?;
    let palette = entry
        .embedded_palette(bytes)
        .map_err(image_error)?
        .expect("FRAMES.BPK embeds its palette");
    Ok(Picture {
        image: frames.remove(0),
        palette,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_end_screen_of_another_size_is_an_error_not_a_crash_later() {
        // The menu copies END.BMP over its whole 640x480 screen; a BMP of another size (from
        // an unknown release) is reported when the data loads, naming the file.
        let path = std::path::Path::new("END.BMP");
        let screen = |width: u32, height: u32| Picture {
            image: Image::new(width, height, vec![0; (width * height) as usize]),
            palette: Palette::BLACK,
        };
        assert!(full_screen(screen(640, 480), path).is_ok());
        let error = full_screen(screen(320, 200), path).unwrap_err();
        assert_eq!(
            error.to_string(),
            "END.BMP: unsupported BMP: 320x200, not 640x480"
        );
    }
}
```

- [ ] **Step 4: Run the checks, with the data**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (221 passed, 16 ignored), among them `an_end_screen_of_another_size_is_an_error_not_a_crash_later`.

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test -p deadrally-gamedata --test catalog_data -- --ignored && DEADRALLY_DATA=~/games/DeathRally cargo test --release -p deadrally-headless --test cli -- --ignored`
Expected: all pass (11 passed), among them `the_startup_assets_load_with_their_documented_shapes` (the menu's shapes, the rows of both menus from `dr.exe`) and `check_data_recognises_the_developers_install` (`DR.EXE` is the known release's).

- [ ] **Step 5: Commit**

```bash
git add crates/gamedata crates/headless/tests/cli.rs crates/core/tests
git commit -m "feat: load the main menu's data"
```

---

### Task 5: The main menu

Spec 3.2–3.5 and 4.2. The building blocks first, each with its own tests: the drawing buffer, the fonts, the input, the menu's palette and drawing, the sound's new controls; then the scene that uses them, the hand-over from the startup and quitting. The blocks are used only by the scene, so until Step 23 the build warns about unused code; the checks that deny warnings run at the end of the task.

**Files:**
- Create: `crates/core/src/canvas.rs`, `crates/core/src/font.rs`, `crates/core/src/keys.rs`, `crates/core/src/menu/palette.rs`, `crates/core/src/menu/draw.rs`, `crates/core/src/menu/mod.rs`, `crates/core/tests/menu.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/src/audio/mod.rs`, `crates/core/src/audio/effects.rs`, `crates/core/src/audio/music.rs`, `crates/core/src/startup.rs`, `crates/core/src/game.rs`, `crates/deadrally/src/main.rs`, `crates/core/tests/startup.rs`, `crates/headless/tests/rendered-audio.sha256`

**Interfaces:**
- Consumes: `assets::{Assets, MenuAssets, Picture}`, `text::{Texts, Metrics, GAP}` (Tasks 3, 4); `fade::{fade, fade_component}` (M1a); `Sound`, `Music`, `Effects` (M1b; Task 1).
- Produces (all `pub(crate)` unless stated):
  - `canvas::{WIDTH = 640, HEIGHT = 480, at(x, y) -> usize, Canvas}`; `Canvas::{default, pixels, copy_all, copy_rows, restore, copy_from, fill, draw(&Image, offset, transparent)}`; offsets are linear, `y * 640 + x`, and drawing stops at the buffer's end.
  - `font::Font::new(Vec<Image>, &Metrics)`, `Font::draw(&mut Canvas, &[u8], offset) -> usize` (the pen's end).
  - `keys::{ESCAPE, ENTER, SPACE, UP, DOWN, LEFT, RIGHT, Y, N, PAD_LEFT, PAD_RIGHT, PAD_UP, PAD_DOWN}`, `keys::scancode(Key) -> u8`, `keys::Keys` (`Default`) with `event(InputEvent)`, `tick()` (one poll, SDL's repeat) and `take() -> u8` (`eventDetected`).
  - `menu::palette::MenuPalette::{new(&Palette, [u8; 3], &[[u8; 3]]), compose, shown, fade(percent), show(&Palette, percent), after_wait}`.
  - `menu::draw::{POPUP_FILL = 0xC4, CURSOR_FRAMES = 50, MenuTable, MAIN_MENU, START_MENU, Focus, Graphics, Panel}`; `Graphics::{new(&MenuAssets), cursor, popup, menu, update_cursor, move_highlight, panel_frame, panel_text}`; `Panel::startup(&Texts)`.
  - `menu::Menu::{new(Assets, Sound, Keys, Vec<i16>, &Palette), input, tick, frame, take_audio, quit_requested}`.
  - `audio::DEFAULT_EFFECTS_VOLUME = 0xC000`; `Sound::{set_effects_volume(u32), set_mask(u32), load_effects(&Bank), trigger_at(channel, effect, volume, pitch)}`; `Effects::into_fading(self) -> Vec<Voice>`; `Music::set_gain(i64)`.
  - `startup::Startup::{finished() -> bool, into_menu(self) -> Menu}`.
  - Public: `Game::quit_requested(&self) -> bool`.

- [ ] **Step 1: Write the failing tests of the drawing buffer**

<!-- write: crates/core/src/canvas.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_drawing_keeps_what_colour_0_covers() {
        // Glyphs, corners and the cursor are drawn this way; an opaque 0 would cut boxes
        // into the background around every letter.
        let mut canvas = Canvas::default();
        canvas.fill(at(0, 0), 4, 1, 9);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), true);
        assert_eq!(&canvas.pixels()[..4], [1, 9, 2, 9]);
        canvas.draw(&Image::new(3, 1, vec![1, 0, 2]), at(0, 0), false);
        assert_eq!(&canvas.pixels()[..4], [1, 0, 2, 9]);
    }

    #[test]
    fn drawing_past_the_right_edge_goes_on_in_the_next_row() {
        // The original works on linear offsets; text that runs past column 639 shows at the
        // left of the next row, and a faithful copy must too.
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(3, 1, vec![5, 6, 7]), at(638, 0), false);
        assert_eq!(&canvas.pixels()[at(638, 0)..at(1, 1)], [5, 6, 7]);
    }

    #[test]
    fn nothing_is_drawn_past_the_end() {
        let mut canvas = Canvas::default();
        canvas.draw(&Image::new(2, 2, vec![1; 4]), at(639, 479), false);
        canvas.fill(at(639, 479), 5, 5, 3);
        assert_eq!(canvas.pixels()[at(639, 479)], 3);
        assert_eq!(canvas.pixels().len(), WIDTH * HEIGHT);
    }

    #[test]
    fn restoring_copies_a_region_back_from_a_picture() {
        let background = Image::new(
            640,
            480,
            (0..WIDTH * HEIGHT).map(|i| (i % 251) as u8).collect(),
        );
        let mut canvas = Canvas::default();
        canvas.restore(&background, at(10, 20), 3, 2);
        assert_eq!(canvas.pixels()[at(10, 20)], background.pixels[at(10, 20)]);
        assert_eq!(canvas.pixels()[at(12, 21)], background.pixels[at(12, 21)]);
        assert_eq!(canvas.pixels()[at(13, 21)], 0, "only the region");
        canvas.copy_rows(&background, 100, 2);
        assert_eq!(
            canvas.pixels()[at(639, 101)],
            background.pixels[at(639, 101)]
        );
        assert_eq!(canvas.pixels()[at(0, 102)], 0);
    }
}
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 14 ms of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod audio;
mod canvas;
mod fade;
mod frame;
mod game;
pub mod host;
mod input;
mod startup;
mod test_scene;

pub use audio::{render_effect, render_music};
pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Length of one simulation tick: 14 ms, as in the Windows version, which counts time as
/// `SDL_GetTicks() / 14` (about 71.43 ticks per second; DOS ran at 70 Hz).
pub const TICK_NANOS: u64 = 14_000_000;

/// Output sample rate in Hz. At 48 kHz a 14 ms tick is a whole number of frames.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 48 000 × 0.014, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 672;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK as u64 * 1_000_000_000 == AUDIO_SAMPLE_RATE as u64 * TICK_NANOS);
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-core --lib canvas`
Expected: FAIL to compile (`Canvas`, `at` do not exist yet).

- [ ] **Step 3: Implement the drawing buffer**

<!-- prepend: crates/core/src/canvas.rs -->
```rust
//! The original's 640x480 drawing buffer and its operations (spec M2a §4.2). Positions are
//! linear offsets, `y * 640 + x`, as the original passes them: a picture drawn past the right
//! edge goes on at the start of the next row, as it does there. Nothing is drawn past the end.

use deadrally_gamedata::image::Image;

/// Menu screens are 640x480.
pub(crate) const WIDTH: usize = 640;
pub(crate) const HEIGHT: usize = 480;

/// The offset of column `x`, row `y`.
pub(crate) const fn at(x: usize, y: usize) -> usize {
    y * WIDTH + x
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Canvas {
    pixels: Vec<u8>,
}

impl Default for Canvas {
    fn default() -> Canvas {
        Canvas {
            pixels: vec![0; WIDTH * HEIGHT],
        }
    }
}

impl Canvas {
    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Copies the whole of `picture`, which must be 640x480.
    pub(crate) fn copy_all(&mut self, picture: &Image) {
        self.pixels.copy_from_slice(&picture.pixels);
    }

    /// Copies rows `first..first + rows` of `picture` (640 wide) into the same rows.
    pub(crate) fn copy_rows(&mut self, picture: &Image, first: usize, rows: usize) {
        let range = at(0, first)..at(0, first + rows).min(self.pixels.len());
        self.pixels[range.clone()].copy_from_slice(&picture.pixels[range]);
    }

    /// Copies a `width` x `height` region of `picture` (640 wide) at `offset` into the same place.
    pub(crate) fn restore(&mut self, picture: &Image, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&picture.pixels[start..end]);
        }
    }

    /// Copies a `width` x `height` region at `offset` from `other` into the same place
    /// (`copyRectToVram`, 0x41AA40, and `refreshAllScreen`, 0x41A210, for the whole screen).
    pub(crate) fn copy_from(&mut self, other: &Canvas, offset: usize, width: usize, height: usize) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].copy_from_slice(&other.pixels[start..end]);
        }
    }

    /// Fills `width` x `height` pixels from `offset` with `colour`.
    pub(crate) fn fill(&mut self, offset: usize, width: usize, height: usize, colour: u8) {
        for row in 0..height {
            let start = offset + row * WIDTH;
            let end = (start + width).min(self.pixels.len());
            if start >= end {
                break;
            }
            self.pixels[start..end].fill(colour);
        }
    }

    /// Draws `image` at `offset`; with `transparent`, colour 0 leaves the canvas as it is.
    pub(crate) fn draw(&mut self, image: &Image, offset: usize, transparent: bool) {
        let width = image.width as usize;
        for (row, source) in image.pixels.chunks_exact(width.max(1)).enumerate() {
            let start = offset + row * WIDTH;
            if start >= self.pixels.len() {
                break;
            }
            let end = (start + width).min(self.pixels.len());
            let target = &mut self.pixels[start..end];
            for (pixel, &colour) in target.iter_mut().zip(source) {
                if !transparent || colour != 0 {
                    *pixel = colour;
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run them to see them pass**

Run: `cargo test -p deadrally-core --lib canvas`
Expected: 4 passed; warnings that `Canvas` and its methods are never used (until Step 23).

- [ ] **Step 5: Write the failing tests of the fonts**

<!-- write: crates/core/src/font.rs -->
```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::at;

    /// A 2x1 font: glyph `k` is filled with colour `k + 1`, its left pixel transparent for
    /// odd `k`; every advance is 3.
    pub(crate) fn font() -> Font {
        let glyphs = (0..96u8)
            .map(|k| {
                let left = if k % 2 == 1 { 0 } else { k + 1 };
                Image::new(2, 1, vec![left, k + 1])
            })
            .collect();
        Font::new(
            glyphs,
            &Metrics {
                width: 2,
                height: 1,
                advances: vec![3; 96],
            },
        )
    }

    #[test]
    fn each_byte_draws_its_glyph_and_moves_the_pen_by_its_advance() {
        // A wrong glyph offset prints the neighbouring letter; a wrong advance spaces the text.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, b" !\"", at(0, 0));
        assert_eq!(end, 9);
        assert_eq!(&canvas.pixels()[..8], [1, 1, 0, 0, 2, 0, 3, 3]);
    }

    #[test]
    fn the_gap_byte_moves_the_pen_one_pixel_and_draws_nothing() {
        // The menu table pads rows with 0xFA to place text a pixel at a time.
        let mut canvas = Canvas::default();
        let end = font().draw(&mut canvas, &[GAP, b' '], at(0, 0));
        assert_eq!(end, 4);
        assert_eq!(&canvas.pixels()[..3], [0, 1, 1]);
    }

    #[test]
    fn bytes_without_a_glyph_draw_nothing_and_do_not_move_the_pen() {
        let mut canvas = Canvas::default();
        assert_eq!(font().draw(&mut canvas, &[7, 200], at(5, 1)), at(5, 1));
        assert!(canvas.pixels().iter().all(|&p| p == 0));
    }
}
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 14 ms of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod audio;
mod canvas;
mod fade;
mod font;
mod frame;
mod game;
pub mod host;
mod input;
mod startup;
mod test_scene;

pub use audio::{render_effect, render_music};
pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Length of one simulation tick: 14 ms, as in the Windows version, which counts time as
/// `SDL_GetTicks() / 14` (about 71.43 ticks per second; DOS ran at 70 Hz).
pub const TICK_NANOS: u64 = 14_000_000;

/// Output sample rate in Hz. At 48 kHz a 14 ms tick is a whole number of frames.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 48 000 × 0.014, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 672;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK as u64 * 1_000_000_000 == AUDIO_SAMPLE_RATE as u64 * TICK_NANOS);
```

- [ ] **Step 6: Run them to see them fail**

Run: `cargo test -p deadrally-core --lib font`
Expected: FAIL to compile (`Font` does not exist yet).

- [ ] **Step 7: Implement the fonts**

<!-- prepend: crates/core/src/font.rs -->
```rust
//! The original's bitmap fonts (`drawTextWithFont`, 0x41A2D0; spec M2a §3.1): glyph `c - 32` of
//! a sheet of equal cells, drawn with colour 0 transparent, the pen moving by the glyph's
//! advance from `dr.exe`'s metrics.

use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::{GAP, Metrics};

use crate::canvas::Canvas;

#[derive(Clone, Debug)]
pub(crate) struct Font {
    glyphs: Vec<Image>,
    advances: Vec<u8>,
}

impl Font {
    pub(crate) fn new(glyphs: Vec<Image>, metrics: &Metrics) -> Font {
        Font {
            glyphs,
            advances: metrics.advances.clone(),
        }
    }

    /// The glyph and advance of byte `c`, if the font has one (characters 32 onwards).
    fn glyph(&self, c: u8) -> Option<(&Image, usize)> {
        let index = usize::from(c.checked_sub(32)?);
        Some((
            self.glyphs.get(index)?,
            usize::from(*self.advances.get(index)?),
        ))
    }

    /// Draws `text` with the pen starting at `offset`; returns where the pen ends.
    pub(crate) fn draw(&self, canvas: &mut Canvas, text: &[u8], offset: usize) -> usize {
        let mut pen = offset;
        for &c in text {
            if c == GAP {
                pen += 1;
            } else if let Some((glyph, advance)) = self.glyph(c) {
                canvas.draw(glyph, pen, true);
                pen += advance;
            }
        }
        pen
    }
}
```

- [ ] **Step 8: Run them to see them pass**

Run: `cargo test -p deadrally-core --lib font`
Expected: 3 passed.

- [ ] **Step 9: Write the failing tests of the input**

<!-- write: crates/core/src/keys.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool) -> InputEvent {
        InputEvent::Key { key, pressed }
    }

    #[test]
    fn the_last_key_down_is_remembered_until_it_is_read() {
        // One byte in the original: a second press replaces the first, and reading clears it.
        let mut keys = Keys::default();
        keys.event(key(Key::Up, true));
        keys.event(key(Key::Up, false));
        keys.event(key(Key::Escape, true));
        keys.event(key(Key::Escape, false));
        assert_eq!(keys.take(), ESCAPE);
        assert_eq!(keys.take(), 0);
    }

    #[test]
    fn arrows_report_the_keypads_codes() {
        // windib drops the 0xE0 prefix, so the menus see 0x48 for both Up and keypad 8.
        assert_eq!(scancode(Key::Up), scancode(Key::Kp8));
        assert_eq!(scancode(Key::KpEnter), ENTER);
        assert_eq!(scancode(Key::Y), Y);
        assert_eq!(scancode(Key::N), N);
    }

    #[test]
    fn a_held_key_repeats_after_half_a_second_then_every_third_tick() {
        // SDL 1.2's repeat, polled once a tick: the delay passes at the 36th poll (504 ms),
        // the first repeat comes 3 polls later (42 ms > 30 ms), then every 3 polls.
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        assert_eq!(keys.take(), DOWN);
        let polls: Vec<u32> = (1..=48)
            .filter(|_| {
                keys.tick();
                keys.take() == DOWN
            })
            .collect();
        assert_eq!(polls, [39, 42, 45, 48]);
    }

    #[test]
    fn releasing_the_key_or_holding_a_modifier_does_not_repeat() {
        let mut keys = Keys::default();
        keys.event(key(Key::Down, true));
        keys.event(key(Key::Down, false));
        keys.take();
        keys.event(key(Key::LeftShift, true));
        assert_eq!(keys.take(), 0x2A, "a modifier is still a key press");
        for _ in 0..100 {
            keys.tick();
            assert_eq!(keys.take(), 0);
        }
    }

    #[test]
    fn a_fresh_push_of_the_stick_holds_its_repeat_off_for_700_ms() {
        // The menus read every 2 ticks: a held push moves once, waits 700 ms, then moves on
        // every read.
        let mut keys = Keys::default();
        let reads: Vec<u8> = (0..60)
            .map(|read| {
                if read == 1 {
                    keys.event(InputEvent::PadAxis {
                        axis: PadAxis::StickY,
                        value: 20_000,
                    });
                }
                keys.tick();
                keys.tick();
                keys.take()
            })
            .collect();
        assert_eq!(reads[1], PAD_DOWN);
        // 700 ms is 25 reads of 28 ms; the hold-off ends at read 1 + 25.
        assert!(reads[2..26].iter().all(|&code| code == 0), "{reads:?}");
        assert!(
            reads[26..].iter().all(|&code| code == PAD_DOWN),
            "{reads:?}"
        );
    }

    #[test]
    fn pad_buttons_confirm_and_go_back() {
        // Buttons 0 and 2 answer like Enter, 1 and 3 like Escape.
        let mut keys = Keys::default();
        let push = |keys: &mut Keys, button, pressed| {
            keys.event(InputEvent::PadButton { button, pressed });
        };
        push(&mut keys, PadButton::A, true);
        assert_eq!(keys.take(), ENTER);
        push(&mut keys, PadButton::A, false);
        keys.tick();
        assert_eq!(keys.take(), 0);
        push(&mut keys, PadButton::B, true);
        keys.tick();
        assert_eq!(keys.take(), ESCAPE);
    }
}
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 14 ms of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod audio;
mod canvas;
mod fade;
mod font;
mod frame;
mod game;
pub mod host;
mod input;
mod keys;
mod startup;
mod test_scene;

pub use audio::{render_effect, render_music};
pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Length of one simulation tick: 14 ms, as in the Windows version, which counts time as
/// `SDL_GetTicks() / 14` (about 71.43 ticks per second; DOS ran at 70 Hz).
pub const TICK_NANOS: u64 = 14_000_000;

/// Output sample rate in Hz. At 48 kHz a 14 ms tick is a whole number of frames.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 48 000 × 0.014, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 672;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK as u64 * 1_000_000_000 == AUDIO_SAMPLE_RATE as u64 * TICK_NANOS);
```

- [ ] **Step 10: Run them to see them fail**

Run: `cargo test -p deadrally-core --lib keys`
Expected: FAIL to compile (`Keys`, `scancode` and the codes do not exist yet).

- [ ] **Step 11: Implement the input**

<!-- prepend: crates/core/src/keys.rs -->
```rust
//! The player's input as the original reads it (spec M2a §3.3): one remembered key, its PC
//! set-1 scancode as SDL 1.2's `windib` driver reports it, kept until `eventDetected`
//! (0x417EB0) reads and clears it; SDL 1.2's key repeat (`SDL_EnableKeyRepeat(500, 30)`); and
//! the joystick, polled by `eventDetected` itself.

use crate::input::{InputEvent, Key, PadAxis, PadButton};

/// Scancodes the menus act on.
pub(crate) const ESCAPE: u8 = 0x01;
pub(crate) const ENTER: u8 = 0x1C;
pub(crate) const SPACE: u8 = 0x39;
pub(crate) const UP: u8 = 0x48;
pub(crate) const DOWN: u8 = 0x50;
pub(crate) const LEFT: u8 = 0x4B;
pub(crate) const RIGHT: u8 = 0x4D;
pub(crate) const Y: u8 = 0x15;
pub(crate) const N: u8 = 0x31;
/// The joystick's codes are DirectInput's: set-1 with the extended bit.
pub(crate) const PAD_LEFT: u8 = 0xCB;
pub(crate) const PAD_RIGHT: u8 = 0xCD;
pub(crate) const PAD_UP: u8 = 0xC8;
pub(crate) const PAD_DOWN: u8 = 0xD0;

/// One tick, in SDL milliseconds.
const TICK_MS: i64 = 14;
/// SDL 1.2 key repeat: the delay before the first repeat and the interval after it.
const REPEAT_DELAY_MS: i64 = 500;
const REPEAT_INTERVAL_MS: i64 = 30;
/// `eventDetected`'s joystick: stick positions past ±50 (raw / 256) count; a first push holds
/// the repeat off for 700 ms; 400 ms tell a fresh push from a held one.
const STICK_THRESHOLD: i32 = 50;
const HOLD_OFF_MS: i64 = 700;
const FRESH_MS: i64 = 400;
const UNSET_MS: i64 = 250;

/// The PC set-1 scancode SDL 1.2's `windib` driver reports for a key (no 0xE0 prefix, so the
/// arrows share the keypad's codes).
pub(crate) fn scancode(key: Key) -> u8 {
    match key {
        Key::Escape => 0x01,
        Key::Digit1 => 0x02,
        Key::Digit2 => 0x03,
        Key::Digit3 => 0x04,
        Key::Digit4 => 0x05,
        Key::Digit5 => 0x06,
        Key::Digit6 => 0x07,
        Key::Digit7 => 0x08,
        Key::Digit8 => 0x09,
        Key::Digit9 => 0x0A,
        Key::Digit0 => 0x0B,
        Key::Backspace => 0x0E,
        Key::Tab => 0x0F,
        Key::Q => 0x10,
        Key::W => 0x11,
        Key::E => 0x12,
        Key::R => 0x13,
        Key::T => 0x14,
        Key::Y => 0x15,
        Key::U => 0x16,
        Key::I => 0x17,
        Key::O => 0x18,
        Key::P => 0x19,
        Key::Enter | Key::KpEnter => 0x1C,
        Key::LeftCtrl | Key::RightCtrl => 0x1D,
        Key::A => 0x1E,
        Key::S => 0x1F,
        Key::D => 0x20,
        Key::F => 0x21,
        Key::G => 0x22,
        Key::H => 0x23,
        Key::J => 0x24,
        Key::K => 0x25,
        Key::L => 0x26,
        Key::LeftShift => 0x2A,
        Key::Z => 0x2C,
        Key::X => 0x2D,
        Key::C => 0x2E,
        Key::V => 0x2F,
        Key::B => 0x30,
        Key::N => 0x31,
        Key::M => 0x32,
        Key::KpDivide => 0x35,
        Key::RightShift => 0x36,
        Key::KpMultiply => 0x37,
        Key::LeftAlt | Key::RightAlt => 0x38,
        Key::Space => 0x39,
        Key::F1 => 0x3B,
        Key::F2 => 0x3C,
        Key::F3 => 0x3D,
        Key::F4 => 0x3E,
        Key::F5 => 0x3F,
        Key::F6 => 0x40,
        Key::F7 => 0x41,
        Key::F8 => 0x42,
        Key::F9 => 0x43,
        Key::F10 => 0x44,
        Key::Kp7 => 0x47,
        Key::Up | Key::Kp8 => 0x48,
        Key::Kp9 => 0x49,
        Key::KpMinus => 0x4A,
        Key::Left | Key::Kp4 => 0x4B,
        Key::Kp5 => 0x4C,
        Key::Right | Key::Kp6 => 0x4D,
        Key::KpPlus => 0x4E,
        Key::Kp1 => 0x4F,
        Key::Down | Key::Kp2 => 0x50,
        Key::Kp3 => 0x51,
        Key::Kp0 => 0x52,
        Key::KpPeriod => 0x53,
        Key::F11 => 0x57,
        Key::F12 => 0x58,
    }
}

/// SDL 1.2 repeats every key but the modifiers.
fn repeats(key: Key) -> bool {
    !matches!(
        key,
        Key::LeftShift
            | Key::RightShift
            | Key::LeftCtrl
            | Key::RightCtrl
            | Key::LeftAlt
            | Key::RightAlt
    )
}

/// The key SDL repeats while it is held.
#[derive(Clone, Copy, Debug)]
struct Repeat {
    key: Key,
    /// Still waiting for the first delay.
    first: bool,
    /// Milliseconds since SDL's repeat timestamp.
    elapsed: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Keys {
    /// `[0x456BF8]`: the last key-down's scancode, 0 when read.
    remembered: u8,
    repeat: Option<Repeat>,
    /// Ticks polled so far: SDL's clock is `14 * ticks` milliseconds.
    ticks: i64,
    stick: [i32; 2],
    buttons: [bool; 4],
    /// `eventDetected`'s joystick timestamps: 0x456B28, 0x456B2C and the hold-off 0x456B1C.
    pad_pushed_ms: Option<i64>,
    pad_called_ms: Option<i64>,
    hold_off_until_ms: i64,
}

impl Keys {
    /// An input event from the frontend. Key-downs become the remembered key at once, as the
    /// next poll of the original's event loop would make them.
    pub(crate) fn event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed: true } => {
                self.remembered = scancode(key);
                if repeats(key) {
                    self.repeat = Some(Repeat {
                        key,
                        first: true,
                        elapsed: 0,
                    });
                }
            }
            InputEvent::Key {
                key,
                pressed: false,
            } => {
                if self.repeat.is_some_and(|repeat| repeat.key == key) {
                    self.repeat = None;
                }
            }
            InputEvent::PadButton { button, pressed } => {
                self.buttons[PadButton::ALL
                    .iter()
                    .position(|&b| b == button)
                    .expect("every button is listed")] = pressed;
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[usize::from(axis == PadAxis::StickY)] = i32::from(value) / 256;
            }
        }
    }

    /// One poll of the event loop, once per tick: SDL 1.2 repeats the held key.
    pub(crate) fn tick(&mut self) {
        self.ticks += 1;
        if let Some(repeat) = &mut self.repeat {
            repeat.elapsed += TICK_MS;
            if repeat.first {
                if repeat.elapsed > REPEAT_DELAY_MS {
                    repeat.first = false;
                    repeat.elapsed = 0;
                }
            } else if repeat.elapsed > REPEAT_INTERVAL_MS {
                repeat.elapsed = 0;
                self.remembered = scancode(repeat.key);
            }
        }
    }

    /// `eventDetected`: the remembered key, cleared, unless the joystick says something.
    pub(crate) fn take(&mut self) -> u8 {
        let key = std::mem::take(&mut self.remembered);
        let now = TICK_MS * self.ticks;
        let pushed = self.pad_pushed_ms.unwrap_or(now - UNSET_MS);
        let called = *self.pad_called_ms.get_or_insert(now - UNSET_MS);
        let pad = self.pad_code();
        if now >= self.hold_off_until_ms {
            if pad != 0 {
                if now - pushed >= FRESH_MS && now - called < FRESH_MS {
                    self.hold_off_until_ms = now + HOLD_OFF_MS;
                }
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
                pad
            } else {
                (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
                key
            }
        } else if pad != 0 {
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now), Some(now));
            0
        } else {
            self.hold_off_until_ms = now;
            (self.pad_pushed_ms, self.pad_called_ms) = (Some(now - FRESH_MS), Some(now));
            key
        }
    }

    /// The joystick's code, later checks winning as in the original: buttons over the stick,
    /// the vertical axis over the horizontal one.
    fn pad_code(&self) -> u8 {
        let [x, y] = self.stick;
        let mut code = 0;
        if x < -STICK_THRESHOLD {
            code = PAD_LEFT;
        }
        if x > STICK_THRESHOLD {
            code = PAD_RIGHT;
        }
        if y < -STICK_THRESHOLD {
            code = PAD_UP;
        }
        if y > STICK_THRESHOLD {
            code = PAD_DOWN;
        }
        for (pressed, button_code) in self.buttons.iter().zip([ENTER, ESCAPE, ENTER, ESCAPE]) {
            if *pressed {
                code = button_code;
            }
        }
        code
    }
}
```

- [ ] **Step 12: Run them to see them pass**

Run: `cargo test -p deadrally-core --lib keys`
Expected: 6 passed.

- [ ] **Step 13: Write the failing tests of the menu's palette and drawing**

`menu/mod.rs` declares only the two modules for now; Step 23 writes the scene into it.

<!-- write: crates/core/src/menu/palette.rs -->
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Palette {
        Palette(std::array::from_fn(|i| {
            [(i % 64) as u8, 63, (63 - i % 64) as u8]
        }))
    }

    fn background() -> Vec<[u8; 3]> {
        (0..BACKGROUND_ROWS)
            .map(|row| [(row % 64) as u8, 1, 32])
            .collect()
    }

    #[test]
    fn the_player_ramp_follows_the_originals_float_arithmetic() {
        // Values from an emulation of 0x418B00's f32/f64 steps; plain integer maths is off by
        // one in places, which shows as a different shade of the player's colour.
        let ramp = player_ramp([63, 0, 32]);
        assert_eq!(ramp[0], [6, 0, 3]);
        assert_eq!(ramp[1], [9, 0, 4]);
        assert_eq!(ramp[15], [59, 0, 30]);
        assert_eq!(ramp[16], [63, 0, 32]);
        assert_eq!(ramp[17], [63, 3, 33]);
        assert_eq!(ramp[31], [63, 59, 61]);
        let other = player_ramp([40, 21, 7]);
        assert_eq!(other[4], [13, 6, 2]);
        assert_eq!(other[25], [52, 44, 38]);
    }

    #[test]
    fn the_copper_ramp_has_seven_entries_capped_at_63() {
        // DreeRally's loop writes one entry; the original writes 176..=182.
        assert_eq!(
            copper_ramp([63, 0, 32]),
            [
                [10, 0, 5],
                [19, 0, 9],
                [28, 0, 14],
                [37, 0, 19],
                [46, 0, 23],
                [55, 0, 28],
                [63, 0, 32]
            ]
        );
        assert_eq!(copper_ramp([40, 21, 7])[3], [23, 12, 4]);
    }

    #[test]
    fn the_fade_in_scales_the_composed_palette_and_stops_at_98_percent() {
        // The original never raises the menu to 100 %: a component of 63 shows as 62.
        let mut palette = MenuPalette::new(&menu(), [63, 0, 32], &background());
        assert_eq!(palette.shown(), &Palette::BLACK);
        palette.fade(98);
        assert_eq!(palette.shown().0[1], [1, 62, 61]);
        assert_eq!(palette.shown().0[64], [6, 0, 3], "the player ramp, at 98 %");
        assert_eq!(
            palette.shown().0[192 + 31],
            [
                fade_component(30, 98 << 16),
                0,
                fade_component(15, 98 << 16)
            ],
            "background row 511 (511 % 64 = 63): 63 * 31 / 64 = 30, 32 * 31 / 64 = 15"
        );
    }

    #[test]
    fn the_pulse_runs_from_100_down_to_49_and_back_in_34_calls() {
        // The pulsing entries animate the menu's highlights; another period or phase would
        // show other shades in the screenshots.
        let mut palette = MenuPalette::new(&menu(), [0, 0, 0], &background());
        let levels: Vec<[u8; 3]> = (0..35)
            .map(|_| {
                palette.after_wait();
                palette.shown().0[16 + 15]
            })
            .collect();
        let component = |level: i64| fade_component(31, level << 16);
        assert_eq!(levels[0][0], component(100));
        assert_eq!(levels[1][0], component(97));
        assert_eq!(levels[17][0], component(49));
        assert_eq!(levels[18][0], component(52));
        assert_eq!(levels[34][0], component(100), "a period of 34 calls");
    }

    #[test]
    fn the_background_steps_a_row_on_the_first_call_and_every_70th_after() {
        let mut palette = MenuPalette::new(&menu(), [0, 0, 0], &background());
        // Row r of this background is (r % 64, 1, 32); entry 192 + 31 shows row * 31 / 64.
        palette.after_wait();
        assert_eq!(
            palette.shown().0[192 + 31],
            [30, 0, 15],
            "row 510, full brightness"
        );
        for _ in 1..70 {
            palette.after_wait();
        }
        assert_eq!(palette.shown().0[192 + 31][0], 30, "still row 510");
        palette.after_wait();
        assert_eq!(palette.shown().0[192 + 31][0], 29, "call 71: row 509");
    }
}
```

<!-- write: crates/core/src/menu/draw.rs -->
```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::canvas::{HEIGHT, WIDTH};

    /// Graphics where every picture has its own colour: corners 11..=14 (focused) and 21..=24,
    /// cursor frame k colour 100 + k with a transparent top-left pixel, glyphs as in
    /// `font::tests::font` but 32x32 (big) or 16x16 (small) and colour 50 + font.
    pub(crate) fn graphics() -> Graphics {
        let solid = |w: u32, h: u32, colour: u8| Image::new(w, h, vec![colour; (w * h) as usize]);
        let glyphs = |size: u32, colour: u8| {
            (0..96)
                .map(|_| solid(size, size, colour))
                .collect::<Vec<_>>()
        };
        let metrics = |size: u8| deadrally_gamedata::text::Metrics {
            width: size,
            height: size,
            advances: vec![size; 96],
        };
        Graphics {
            background: Image::new(
                640,
                480,
                (0..WIDTH * HEIGHT).map(|i| (i % 7) as u8 + 1).collect(),
            ),
            panel_line: solid(640, 10, 99),
            corners_focused: (11..15).map(|c| solid(32, 20, c)).collect(),
            corners_unfocused: (21..25).map(|c| solid(32, 20, c)).collect(),
            cursor: (0..50)
                .map(|k| {
                    let mut frame = solid(20, 20, 100 + k);
                    frame.pixels[0] = 0;
                    frame
                })
                .collect(),
            big_a: Font::new(glyphs(32, 50), &metrics(32)),
            big_b: Font::new(glyphs(32, 51), &metrics(32)),
            big_d: Font::new(glyphs(32, 52), &metrics(32)),
            small: [
                Font::new(glyphs(16, 60), &metrics(16)),
                Font::new(glyphs(16, 61), &metrics(16)),
                Font::new(glyphs(16, 62), &metrics(16)),
            ],
            menus: texts().menus,
        }
    }

    pub(crate) fn texts() -> Texts {
        let metrics = deadrally_gamedata::text::Metrics {
            width: 32,
            height: 32,
            advances: vec![32; 96],
        };
        Texts {
            menus: (0..9)
                .map(|m| {
                    (0..9)
                        .map(|r| {
                            if r < 6 {
                                vec![b'A' + m as u8]
                            } else {
                                Vec::new()
                            }
                        })
                        .collect()
                })
                .collect(),
            panel: (0..4).map(|i| vec![b'a' + i]).collect(),
            exit_question: b"?".to_vec(),
            yes: b"Y".to_vec(),
            no: b"N".to_vec(),
            big: metrics.clone(),
            small: metrics.clone(),
            medium: metrics,
        }
    }

    #[test]
    fn a_popup_fills_exactly_w_minus_6_columns() {
        // DreeRally fills two columns short on the main menu; the original fills x+2..=x+w-5.
        let mut screen = Canvas::default();
        graphics().popup(&mut screen, 145, 124, 349, 192, Focus::Focused);
        let p = screen.pixels();
        assert_eq!(p[at(147, 200)], POPUP_FILL);
        assert_eq!(
            p[at(489 - 1, 200)],
            POPUP_FILL,
            "x + w - 5 is the right line"
        );
        assert_eq!(p[at(489, 200)], LINE_FOCUSED);
        assert_eq!(p[at(146, 200)], LINE_FOCUSED, "left line at x + 1");
        assert_eq!(p[at(490, 200)], 0, "the shadow columns are left alone");
        assert_eq!(p[at(177, 125)], LINE_FOCUSED, "top line from x + 32");
        assert_eq!(
            p[at(461, 309)],
            LINE_FOCUSED,
            "bottom line at y + h - 7 to x + w - 33"
        );
        assert_eq!(p[at(145, 124)], 11, "top-left corner");
        assert_eq!(p[at(462, 124)], 12, "top-right corner");
        assert_eq!(p[at(145, 296)], 13, "bottom-left corner");
        assert_eq!(p[at(462 + 31, 296 + 19)], 14, "bottom-right corner");
    }

    #[test]
    fn a_focused_menu_shows_the_cursor_and_its_rows_in_three_fonts() {
        let mut screen = Canvas::default();
        let mut menu = MAIN_MENU;
        menu.selected = 2;
        graphics().menu(&mut screen, &menu, Focus::Focused, 7);
        let p = screen.pixels();
        let text_row = |row: usize| p[at(177, 129 + 28 * row)];
        assert_eq!(text_row(2), 50, "selected: big A");
        assert_eq!(text_row(0), 51, "active: big B");
        assert_eq!(text_row(1), 52, "inactive: big D");
        assert_eq!(
            p[at(155, 135 + 56)],
            107,
            "cursor frame 7 at (x + 9, y + 11 + 28 * 2)"
        );
        let mut dim = Canvas::default();
        graphics().menu(&mut dim, &menu, Focus::Unfocused, 7);
        assert_eq!(
            dim.pixels()[at(177, 129 + 56)],
            52,
            "unfocused: everything big D"
        );
        assert_eq!(dim.pixels()[at(155, 191)], POPUP_FILL, "no cursor");
        assert_eq!(dim.pixels()[at(145, 124)], 21, "unfocused corners");
    }

    #[test]
    fn the_cursor_update_copies_its_box_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        graphics().update_cursor(&mut screen, &mut shown, &MAIN_MENU, 3);
        assert_eq!(
            shown.pixels()[at(154, 135)],
            POPUP_FILL,
            "transparent pixel over the fill"
        );
        assert_eq!(shown.pixels()[at(155, 135)], 103);
        assert_eq!(shown.pixels()[at(174, 135)], 0, "only the 20x20 box");
    }

    #[test]
    fn moving_the_highlight_redraws_both_rows_and_copies_both_to_the_shown_buffer() {
        let (mut screen, mut shown) = (Canvas::default(), Canvas::default());
        let mut menu = MAIN_MENU;
        graphics().move_highlight(&mut screen, &mut shown, &mut menu, 2, 5, 9);
        assert_eq!(menu.selected, 2);
        let s = shown.pixels();
        assert_eq!(s[at(177, 129)], 51, "the old row in big B");
        assert_eq!(s[at(177, 129 + 56)], 50, "the new row in big A");
        assert_eq!(s[at(155, 135 + 56)], 109, "cursor frame 9");
        assert_eq!(s[at(152, 129 + 28)], 0, "row 1 is not copied");
    }

    #[test]
    fn the_panel_shows_its_last_six_lines_with_the_startup_text_in_small_b() {
        let panel = Panel::startup(&texts());
        let mut screen = Canvas::default();
        graphics().panel_text(&mut screen, &panel);
        let p = screen.pixels();
        assert_eq!(
            p[at(12, 380)],
            graphics().background.pixels[at(12, 380)],
            "line 16 is empty; the restore starts at row 380"
        );
        for k in [1, 2, 3, 5] {
            assert_eq!(p[at(12, 378 + 15 * k)], 61, "line {}: small B", 16 + k);
        }
        assert_eq!(
            p[at(12, 445)],
            graphics().background.pixels[at(12, 445)],
            "line 20 is empty (the glyphs above reach row 438)"
        );
    }
}
```

<!-- write: crates/core/src/menu/mod.rs -->
```rust
//! The main menu (spec M2a §3.2–§3.5), from the title's fade to black to the end screen.
//!
//! The original runs this as straight code with waits in it (`waitWithRefresh`, 0x43D870); the
//! screen shown during a tick is what the shown buffer and the palette hold when that tick's
//! wait starts. Here [`State`] names the wait the menu stands at, and [`Menu::tick`] runs the
//! code from it to the next one.

pub(crate) mod draw;
pub(crate) mod palette;
```

<!-- write: crates/core/src/lib.rs -->
```rust
//! Deterministic game core of DeadRally.
//!
//! The core never reads a clock, spawns a thread or touches the platform. A frontend feeds it
//! [`InputEvent`]s, calls [`Game::tick`] once for every 14 ms of wall-clock time, presents
//! [`Game::frame`] and plays the samples from [`Game::take_audio`]. Same inputs, same outputs,
//! on every OS: parity with the original is impossible otherwise.
#![forbid(unsafe_code)]

mod audio;
mod canvas;
mod fade;
mod font;
mod frame;
mod game;
pub mod host;
mod input;
mod keys;
mod menu;
mod startup;
mod test_scene;

pub use audio::{render_effect, render_music};
pub use frame::{Frame, expand_6bit};
pub use game::Game;
pub use input::{InputEvent, Key, PadAxis, PadButton};

/// Length of one simulation tick: 14 ms, as in the Windows version, which counts time as
/// `SDL_GetTicks() / 14` (about 71.43 ticks per second; DOS ran at 70 Hz).
pub const TICK_NANOS: u64 = 14_000_000;

/// Output sample rate in Hz. At 48 kHz a 14 ms tick is a whole number of frames.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// Audio channels; samples are interleaved left, right.
pub const AUDIO_CHANNELS: usize = 2;

/// Stereo frames produced per tick: 48 000 × 0.014, exactly.
pub const AUDIO_FRAMES_PER_TICK: usize = 672;

const _: () =
    assert!(AUDIO_FRAMES_PER_TICK as u64 * 1_000_000_000 == AUDIO_SAMPLE_RATE as u64 * TICK_NANOS);
```

- [ ] **Step 14: Run them to see them fail**

Run: `cargo test -p deadrally-core --lib menu`
Expected: FAIL to compile (`MenuPalette`, `Graphics` and the rest do not exist yet).

- [ ] **Step 15: Implement the menu's palette and drawing**

<!-- prepend: crates/core/src/menu/palette.rs -->
```rust
//! The main menu's palette (spec M2a §3.2, §3.3), as `dr.exe` computes it: `MENU.PAL` with the
//! player colour's ramps (`sub_418B00` 0x418B00, `sub_41ED20` 0x41ED20), the background copper
//! rows (`sub_4224E0` 0x4224E0, 0x42A570) and the pulse of entries 16–31 (`sub_4220D0`
//! 0x4220D0). The ramps use the original's f32 and f64 arithmetic step by step: the x87 runs at
//! 53-bit precision there, so IEEE f64 with the same f32 roundings gives its exact results.

use deadrally_gamedata::image::Palette;

use crate::fade::{fade, fade_component};

/// `BGCOP.PAL`'s rows, three components each.
pub(crate) const BACKGROUND_ROWS: usize = 512;
/// The copper rows step on every 70th call of 0x42A570.
const COPPER_PERIOD: u32 = 70;
/// The pulse runs between these levels, 3 % a tick.
const PULSE_TOP: i64 = 100;
const PULSE_BOTTOM: i64 = 49;
const PULSE_STEP: i64 = 3;

/// The player colour's 32-entry ramp at 64..=95 (`sub_418B00`): from a tenth of the colour up to
/// the colour, then on towards white.
pub(crate) fn player_ramp(colour: [u8; 3]) -> [[u8; 3]; 32] {
    let mut ramp = [[0; 3]; 32];
    for c in 0..3 {
        let a = f32::from(colour[c]);
        let tenth = 0.1 * f64::from(a);
        let rise = ((f64::from(a) - tenth) * 0.0625) as f32;
        let to_white = ((63.0 - f64::from(a)) * 0.0625) as f32;
        for i in 0..16u8 {
            let step = f64::from(f32::from(i));
            ramp[usize::from(i)][c] = (step * f64::from(rise) + tenth) as u8;
            ramp[16 + usize::from(i)][c] = (step * f64::from(to_white) + f64::from(a)) as u8;
        }
    }
    ramp
}

/// The copper ramp at 176..=182 (`sub_41ED20`): from a sixth of the colour up to it.
pub(crate) fn copper_ramp(colour: [u8; 3]) -> [[u8; 3]; 7] {
    // The f32 constant at 0x443198, 1/7 rounded.
    let seventh = f64::from(f32::from_bits(0x3E12_4925));
    let mut ramp = [[0; 3]; 7];
    for c in 0..3 {
        let a = f64::from(colour[c]);
        let step = (a * seventh) as f32;
        let sixth = a * (1.0 / 6.0);
        for (i, entry) in ramp.iter_mut().enumerate() {
            entry[c] = ((f64::from(step) * i as f64 + sixth) as u8).min(63);
        }
    }
    ramp
}

/// Background copper row `row` spread over entries 192..=223: `row * i / 64`.
fn background_ramp(row: [u8; 3]) -> [[u8; 3]; 32] {
    std::array::from_fn(|i| row.map(|component| (usize::from(component) * i / 64) as u8))
}

/// The palette the menu composes once and fades in, and the entries that keep moving after.
#[derive(Clone, Debug)]
pub(crate) struct MenuPalette {
    /// `MENU.PAL` (`palette2`): the pulse scales it.
    menu: Palette,
    /// The composed palette (`palette1`) the fades scale.
    composed: Palette,
    /// The player colour's entry of `COPPER.PAL`.
    colour: [u8; 3],
    background: Vec<[u8; 3]>,
    /// What the hardware shows.
    shown: Palette,
    pulse: i64,
    rising: bool,
    /// Calls of 0x42A570 so far (0x456BA0), and the copper row (0x456754).
    calls: u32,
    row: usize,
}

impl MenuPalette {
    /// The palette as `sub_4224E0` composes it at the first start: background row 511, the
    /// pulse at 100 %. `colour` is the player colour's entry of `COPPER.PAL`; `background` holds
    /// `BGCOP.PAL`'s [`BACKGROUND_ROWS`] rows. Nothing is shown yet: the title has faded to
    /// black.
    pub(crate) fn new(menu: &Palette, colour: [u8; 3], background: &[[u8; 3]]) -> MenuPalette {
        assert_eq!(background.len(), BACKGROUND_ROWS, "BGCOP.PAL has 512 rows");
        let mut palette = MenuPalette {
            menu: menu.clone(),
            composed: menu.clone(),
            colour,
            background: background.to_vec(),
            shown: Palette::BLACK,
            pulse: PULSE_TOP,
            rising: false,
            calls: 0,
            row: BACKGROUND_ROWS - 1,
        };
        palette.compose();
        palette
    }

    /// `sub_4224E0`: the composed palette from `MENU.PAL`, the ramps, the current background
    /// row and entries 16–31 at the pulse's current level.
    pub(crate) fn compose(&mut self) {
        let mut composed = self.menu.clone();
        composed.0[64..96].copy_from_slice(&player_ramp(self.colour));
        composed.0[176..183].copy_from_slice(&copper_ramp(self.colour));
        composed.0[192..224].copy_from_slice(&background_ramp(self.background[self.row]));
        let level = self.pulse << 16;
        for entry in 16..32 {
            composed.0[entry] =
                self.menu.0[entry].map(|component| fade_component(component, level));
        }
        self.composed = composed;
    }

    pub(crate) fn shown(&self) -> &Palette {
        &self.shown
    }

    /// The whole composed palette at `percent` %, as the menu's fades show it.
    pub(crate) fn fade(&mut self, percent: i64) {
        self.shown = fade(&self.composed, percent << 16);
    }

    /// Another picture's palette at `percent` %, for the screens the menu leads to.
    pub(crate) fn show(&mut self, palette: &Palette, percent: i64) {
        self.shown = fade(palette, percent << 16);
    }

    /// What follows each wait of 0x42A570 (and 0x42A480) in the menu: the pulse writes entries
    /// 16–31 at its level and moves on; every 70th call the background steps a row, its 32
    /// entries shown at full brightness.
    pub(crate) fn after_wait(&mut self) {
        self.calls += 1;
        let level = self.pulse << 16;
        for entry in 16..32 {
            self.shown.0[entry] =
                self.menu.0[entry].map(|component| fade_component(component, level));
        }
        if self.pulse == PULSE_BOTTOM {
            self.rising = true;
            self.pulse += PULSE_STEP;
        } else if self.pulse == PULSE_TOP {
            self.rising = false;
            self.pulse -= PULSE_STEP;
        } else if self.rising {
            self.pulse += PULSE_STEP;
        } else {
            self.pulse -= PULSE_STEP;
        }
        if self.calls % COPPER_PERIOD == 1 {
            self.row = self.row.checked_sub(1).unwrap_or(BACKGROUND_ROWS - 1);
            self.shown.0[192..224].copy_from_slice(&background_ramp(self.background[self.row]));
        }
    }
}
```

<!-- prepend: crates/core/src/menu/draw.rs -->
```rust
//! The menu screen's drawing, as `dr.exe` does it into its screen buffer and copies to the
//! shown buffer (spec M2a §3.3, §3.4): popups (`createPopup` 0x41A530), menus (`drawMenu`
//! 0x41A880), the cursor (`updateCursor` 0x41AB50), the highlight's moves (`refreshMenuUp`
//! 0x41AF40, `refreshMenuDown` 0x41B1A0, 0x41ACF0) and the bottom panel (0x41A7A0, 0x41E810).

use deadrally_gamedata::assets::MenuAssets;
use deadrally_gamedata::image::Image;
use deadrally_gamedata::text::Texts;

use crate::canvas::{Canvas, at};
use crate::font::Font;

/// The fill colour of popups and of the cursor's box.
pub(crate) const POPUP_FILL: u8 = 0xC4;
/// Popup lines: a focused popup's, an unfocused one's.
const LINE_FOCUSED: u8 = 7;
const LINE_UNFOCUSED: u8 = 4;
/// Corner pictures are 32x20, the cursor 20x20, a big glyph 32 high.
const CORNER_WIDTH: usize = 32;
const CORNER_HEIGHT: usize = 20;
const CURSOR_SIZE: usize = 20;
pub(crate) const CURSOR_FRAMES: usize = 50;

/// One menu of the table at 0x4456F0 and its active rows (0x4457F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuTable {
    /// Which menu of `dr.exe`'s text table its rows are.
    pub(crate) text: usize,
    pub(crate) rows: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) row_height: usize,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) selected: usize,
    pub(crate) active: [bool; 9],
}

/// The main menu: start, multiplayer (inactive), configure, hall of fame, credits, exit.
pub(crate) const MAIN_MENU: MenuTable = MenuTable {
    text: 0,
    rows: 6,
    x: 145,
    y: 124,
    row_height: 28,
    width: 349,
    height: 192,
    selected: 0,
    active: [true, false, true, true, true, true, false, false, false],
};

/// The start submenu at the first start: rows 0, 3 and 5 active.
pub(crate) const START_MENU: MenuTable = MenuTable {
    text: 1,
    rows: 6,
    x: 109,
    y: 171,
    row_height: 28,
    width: 421,
    height: 192,
    selected: 0,
    active: [true, false, false, true, false, true, false, false, false],
};

/// How a menu is drawn: unfocused (mode 0) or focused (mode 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Unfocused,
    Focused,
}

/// The menu's pictures, fonts and rows.
#[derive(Clone, Debug)]
pub(crate) struct Graphics {
    pub(crate) background: Image,
    panel_line: Image,
    corners_focused: Vec<Image>,
    corners_unfocused: Vec<Image>,
    cursor: Vec<Image>,
    pub(crate) big_a: Font,
    pub(crate) big_b: Font,
    pub(crate) big_d: Font,
    pub(crate) small: [Font; 3],
    /// `dr.exe`'s menu text table: `menus[m][r]` is row `r` of menu `m`.
    menus: Vec<Vec<Vec<u8>>>,
}

impl Graphics {
    pub(crate) fn new(assets: &MenuAssets) -> Graphics {
        let texts = &assets.texts;
        Graphics {
            background: assets.background.clone(),
            panel_line: assets.panel_line.clone(),
            corners_focused: assets.corners_focused.clone(),
            corners_unfocused: assets.corners_unfocused.clone(),
            cursor: assets.cursor.clone(),
            big_a: Font::new(assets.big_a.clone(), &texts.big),
            big_b: Font::new(assets.big_b.clone(), &texts.big),
            big_d: Font::new(assets.big_d.clone(), &texts.big),
            small: [
                Font::new(assets.small_a.clone(), &texts.small),
                Font::new(assets.small_b.clone(), &texts.small),
                Font::new(assets.small_c.clone(), &texts.small),
            ],
            menus: texts.menus.clone(),
        }
    }

    pub(crate) fn cursor(&self, frame: usize) -> &Image {
        &self.cursor[frame % self.cursor.len()]
    }

    /// `createPopup(x, y, w, h, focus)`: fill, corners, then lines; nothing outside is
    /// cleared, so a popup drawn over another blends their corners.
    pub(crate) fn popup(
        &self,
        screen: &mut Canvas,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        focus: Focus,
    ) {
        if h > 8 {
            screen.fill(at(x + 2, y + 2), w - 6, h - 8, POPUP_FILL);
        }
        let (corners, line) = match focus {
            Focus::Unfocused => (&self.corners_unfocused, LINE_UNFOCUSED),
            Focus::Focused => (&self.corners_focused, LINE_FOCUSED),
        };
        let right = x + w - CORNER_WIDTH;
        let bottom = y + h - CORNER_HEIGHT;
        for (corner, offset) in
            corners
                .iter()
                .zip([at(x, y), at(right, y), at(x, bottom), at(right, bottom)])
        {
            screen.draw(corner, offset, true);
        }
        if w > 64 {
            screen.fill(at(x + 32, y + 1), w - 64, 1, line);
            screen.fill(at(x + 32, y + h - 7), w - 64, 1, line);
        }
        if h > 40 {
            screen.fill(at(x + 1, y + 20), 1, h - 40, line);
            screen.fill(at(x + w - 5, y + 20), 1, h - 40, line);
        }
    }

    /// `drawMenu(menu, focus)`: its popup and rows; the selected row with the cursor when the
    /// menu has focus. Nothing reaches the shown buffer.
    pub(crate) fn menu(&self, screen: &mut Canvas, menu: &MenuTable, focus: Focus, cursor: usize) {
        self.popup(screen, menu.x, menu.y, menu.width, menu.height, focus);
        for row in 0..menu.rows {
            let text = &self.menus[menu.text][row];
            let at_text = at(menu.x + 32, menu.y + 5 + row * menu.row_height);
            let font = if row == menu.selected {
                if focus == Focus::Focused {
                    screen.draw(self.cursor(cursor), self.cursor_at(menu), true);
                    &self.big_a
                } else {
                    &self.big_d
                }
            } else if menu.active[row] && focus == Focus::Focused {
                &self.big_b
            } else {
                &self.big_d
            };
            font.draw(screen, text, at_text);
        }
    }

    /// Where the selected row's cursor goes.
    fn cursor_at(&self, menu: &MenuTable) -> usize {
        at(menu.x + 9, menu.y + 11 + menu.selected * menu.row_height)
    }

    /// `updateCursor`: the cursor's box refilled, frame `frame` drawn, the box copied to the
    /// shown buffer.
    pub(crate) fn update_cursor(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &MenuTable,
        frame: usize,
    ) {
        let offset = self.cursor_at(menu);
        screen.fill(offset, CURSOR_SIZE, CURSOR_SIZE, POPUP_FILL);
        screen.draw(self.cursor(frame), offset, true);
        shown.copy_from(screen, offset, CURSOR_SIZE, CURSOR_SIZE);
    }

    /// Moves the highlight to row `to` as `refreshMenuUp`/`refreshMenuDown` and 0x41ACF0 do:
    /// both rows' areas refilled and redrawn, the cursor drawn with frame `frame`, both
    /// copied to the shown buffer. `base` is 6 for Up and the jump to the last row, 5 for Down.
    pub(crate) fn move_highlight(
        &self,
        screen: &mut Canvas,
        shown: &mut Canvas,
        menu: &mut MenuTable,
        to: usize,
        base: usize,
        frame: usize,
    ) {
        let rows_at = |row: usize| menu.y + base + row * menu.row_height;
        let text_at = |row: usize| at(menu.x + 32, menu.y + 5 + row * menu.row_height);
        let old = menu.selected;
        screen.fill(
            at(menu.x + 9, rows_at(old) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_b
            .draw(screen, &self.menus[menu.text][old], text_at(old));
        menu.selected = to;
        screen.fill(
            at(menu.x + 9, rows_at(to) + 4),
            menu.width - 20,
            22,
            POPUP_FILL,
        );
        self.big_a
            .draw(screen, &self.menus[menu.text][to], text_at(to));
        screen.draw(self.cursor(frame), self.cursor_at(menu), true);
        shown.copy_from(screen, at(menu.x + 7, rows_at(old)), menu.width - 10, 32);
        shown.copy_from(screen, at(menu.x + 7, rows_at(to)), menu.width - 10, 32);
    }

    /// `drawTransparentBlock(x, y, w, h)`: the background restored, then the panel's two
    /// frame lines.
    pub(crate) fn panel_frame(&self, screen: &mut Canvas, x: usize, y: usize, w: usize, h: usize) {
        screen.restore(&self.background, at(x + 2, y - 4), w - 6, h);
        screen.draw(&self.panel_line, at(0, y + 1), true);
        screen.draw(&self.panel_line, at(0, y + h - 9), true);
    }

    /// `drawBottomMenuText`: rows 380..=468 restored, then the panel's last six lines at
    /// (12, 378 + 15k), each in its own small font.
    pub(crate) fn panel_text(&self, screen: &mut Canvas, panel: &Panel) {
        screen.copy_rows(&self.background, 380, 89);
        for (k, line) in panel.lines[16..].iter().enumerate() {
            if let Some(font) = self.small.get(usize::from(line.font)) {
                font.draw(screen, &line.text, at(12, 378 + 15 * k));
            }
        }
    }
}

/// One line of the bottom panel and its font (0, 1, 2: small A, B, C).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: Vec<u8>,
    pub(crate) font: u8,
}

/// The bottom message panel: 22 lines, new ones pushed in at the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Panel {
    lines: Vec<PanelLine>,
}

impl Panel {
    /// The panel as `mainMenu` fills it at start-up: the four start-up lines in small B, an
    /// empty line before the last.
    pub(crate) fn startup(texts: &Texts) -> Panel {
        let mut panel = Panel {
            lines: vec![PanelLine::default(); 22],
        };
        let [first, second, third, last] = [0, 1, 2, 3].map(|i| texts.panel[i].clone());
        for text in [first, second, third, Vec::new(), last] {
            panel.push(text, 1);
        }
        panel
    }

    fn push(&mut self, text: Vec<u8>, font: u8) {
        self.lines.remove(0);
        self.lines.push(PanelLine { text, font });
    }
}
```

- [ ] **Step 16: Run them to see them pass**

Run: `cargo test -p deadrally-core --lib menu`
Expected: 10 passed.

- [ ] **Step 17: Write the failing tests of the sound's volumes**

M1b's `Sound` with its new tests: a new bank lets the old one's effects fade, the effects volume and the mask scale the effects stream, the mask scales the music while it plays, and `music_master` takes the mask.

<!-- write: crates/core/src/audio/mod.rs -->
```rust
//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod music;
pub(crate) mod tables;

use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, Voice, clip};
use self::music::Music;
use crate::AUDIO_CHANNELS;

/// FMOD's master volume for music, 0..=256, at a music volume of the game's configuration
/// (0..=0x10000): `255 * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it with
/// the game's volume mask at 255.
pub(crate) fn music_master(volume: u32) -> i64 {
    (255 * i64::from(volume >> 8)) >> 9
}

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// The effects' share: the volume of the stream minifmod mixes into while the intro plays,
/// 254 of 255 (`255 * 255 >> 8`).
pub(crate) const EFFECTS_GAIN: i64 = UNITY * 254 / 255;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug, Default)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    mix: Vec<i64>,
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        let gain = UNITY * music_master(volume) / 256;
        self.music = Some(Music::new(module, gain, first_order));
    }

    /// Makes `bank` the source of [`Sound::trigger`], silencing the old bank's effects.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based).
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, effects::FULL, effects::FULL);
        }
    }

    /// Stops the music and every effect, with a short fade so nothing clicks.
    pub(crate) fn stop(&mut self) {
        if let Some(music) = &mut self.music {
            music.stop();
        }
        if let Some(effects) = &mut self.effects {
            effects.stop_all();
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(music) = &mut self.music {
            music.mix_into(&mut self.mix);
        }
        for voice in &mut self.fading {
            voice.mix_into(&mut self.mix);
        }
        self.fading.retain(|voice| !voice.finished());
        if let Some(effects) = &mut self.effects {
            let mut part = vec![0; self.mix.len()];
            effects.mix_into(&mut part);
            for (sum, effect) in self.mix.iter_mut().zip(part) {
                *sum += (effect * EFFECTS_GAIN) >> 16;
            }
        }
        out.extend(self.mix.iter().map(|&value| clip(value)));
    }
}

/// Renders `frames` stereo frames of `module` from its first order at the default music
/// volume, mixed as the game plays music in its menus and races.
#[must_use]
pub fn render_music(module: &Module, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.play_music(module, 0, DEFAULT_MUSIC_VOLUME);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

/// Renders effect `effect` (1-based) of `bank` at full volume and normal pitch, as the game
/// triggers it, for `frames` stereo frames.
#[must_use]
pub fn render_effect(bank: &Bank, effect: u8, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.load_effects(bank);
    sound.trigger(1, effect);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{self, Cell, Channel, Pattern, Sample};
    use deadrally_gamedata::xm::{Instrument, Looping};

    fn bank() -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![Some(Instrument {
                name: "Loud".into(),
                data: vec![i16::MAX; 20_000],
                looping: Looping::None,
                volume: 64,
                finetune: 0,
                relative_note: 0,
                panning: 255,
                fadeout: 0,
            })],
        }
    }

    #[test]
    fn nothing_loaded_is_silence_of_the_right_length() {
        // The frontend paces itself on the audio queue; every tick must deliver its samples.
        let mut out = Vec::new();
        Sound::default().render(672, &mut out);
        assert_eq!(out.len(), 672 * AUDIO_CHANNELS);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn overlapping_effects_clip_instead_of_wrapping() {
        // Four full-scale effects add up to twice the 16-bit range; wrapping would turn the
        // overload into noise, clipping only flattens it.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        for channel in 1..=4 {
            sound.trigger(channel, 1);
        }
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 900], i16::MAX, "left, panned hard left");
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn new_music_fades_the_old_out_instead_of_cutting_it() {
        // The intro's music gives way to the menu music; a cut at full level would click.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut silent = loud.clone();
        silent.orders.clear();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        assert!(before > 0);
        sound.play_music(&silent, 0, FULL_VOLUME);
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_configured_music_volume_sets_fmods_master_volume() {
        // musicSetmusicVolume (0x43C280): 255 * (volume >> 8) >> 9.
        assert_eq!(music_master(FULL_VOLUME, FULL_MASK), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME, FULL_MASK), 63);
        assert_eq!(music_master(0x1_0000, FULL_MASK), 127);
        assert_eq!(music_master(0, FULL_MASK), 0);
    }

    #[test]
    fn a_new_bank_lets_the_old_banks_effects_fade_out() {
        // The intro's effects stop as the menu's bank is loaded; cutting them at full level
        // would click.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        sound.stop();
        sound.load_effects(&bank());
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_effects_volume_and_the_mask_scale_the_effects_stream() {
        // The menus play effects at dr.cfg's 75 %: the stream at 255 * 192 >> 8 = 191 of 255
        // instead of the intro's 254. The end screen's mask lowers everything.
        let level = |sound: &mut Sound| {
            sound.load_effects(&bank());
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(1000, &mut out);
            i64::from(out[2 * 999])
        };
        let full = level(&mut Sound::default());
        let mut menu = Sound::default();
        menu.set_effects_volume(DEFAULT_EFFECTS_VOLUME);
        let at_75 = level(&mut menu);
        assert!(
            (at_75 * 254 - full * 191).abs() <= 254 * 2,
            "{at_75} vs {full}"
        );
        let mut quiet = Sound::default();
        quiet.set_mask(0);
        assert_eq!(level(&mut quiet), 0);
    }

    #[test]
    fn the_mask_scales_the_music_while_it_plays() {
        // The end screen fades the music out through the mask, 255 down to 0.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        sound.set_mask(0x80);
        out.clear();
        sound.render(2000, &mut out);
        // 255 * 255 >> 9 = 127 against 128 * 255 >> 9 = 63.
        assert!(
            (i64::from(out[2 * 1999]) * 127 - full * 63).abs() <= 127 * 2,
            "{} vs {full}",
            out[2 * 1999]
        );
    }

    #[test]
    fn stopping_fades_everything_out() {
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        sound.stop();
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 999], 0);
    }
}
```

- [ ] **Step 18: Run them to see them fail**

Run: `cargo test -p deadrally-core --lib audio`
Expected: FAIL to compile (`load_effects`, `set_mask`, `set_effects_volume`, `FULL_MASK` do not exist yet).

- [ ] **Step 19: Implement the sound's volumes**

`Sound` keeps the original's three volume globals (the mask, the music's and the effects' volumes); a new effects bank hands the old one's voices to a fading list, as new music already does.

<!-- write: crates/core/src/audio/mod.rs -->
```rust
//! Sound: the music and effect players and the mixer they share (spec M1b §4.2).

pub(crate) mod effects;
pub(crate) mod mixer;
pub(crate) mod music;
pub(crate) mod tables;

use deadrally_gamedata::s3m::Module;
use deadrally_gamedata::xm::Bank;

use self::effects::Effects;
use self::mixer::{UNITY, Voice, clip};
use self::music::Music;
use crate::AUDIO_CHANNELS;

/// FMOD's master volume for music, 0..=256, at a music volume of the game's configuration
/// (0..=0x10000): `mask * (volume >> 8) >> 9`, as `musicSetmusicVolume` (0x43C280) sets it,
/// with the game's volume mask (0x456A34) at 255 unless the end screen lowers it.
fn music_master(volume: u32, mask: u32) -> i64 {
    (i64::from(mask) * i64::from(volume >> 8)) >> 9
}

/// The volume mask's normal value.
const FULL_MASK: u32 = 255;

/// `dr.cfg`'s default effects volume, 75 % (`defaultConfig`, 0x426700).
pub(crate) const DEFAULT_EFFECTS_VOLUME: u32 = 0xC000;

/// The music volume the intro plays at, whatever `dr.cfg` says: the game's volume globals
/// start at 255 and take `dr.cfg`'s values only when the menu music starts.
pub(crate) const FULL_VOLUME: u32 = 0xFF00;

/// `dr.cfg`'s default music volume, 50 % (`defaultConfig`, 0x426700). The menus play at it
/// until M2's Configure menu can change it.
pub(crate) const DEFAULT_MUSIC_VOLUME: u32 = 0x8000;

/// Everything audible: one piece of music and one bank of effects at a time, as in the
/// original.
#[derive(Debug)]
pub(crate) struct Sound {
    music: Option<Music>,
    /// Voices of music that newer music replaced, fading out.
    fading: Vec<Voice>,
    effects: Option<Effects>,
    /// Voices of a bank that a newer bank replaced, fading out.
    fading_effects: Vec<Voice>,
    mix: Vec<i64>,
    /// The original's volume globals: the mask (0x456A34), the music's (0x456A30) and the
    /// effects' (0x456A2C) volumes as `dr.cfg` gives them; all full until the intro ends.
    mask: u32,
    music_volume: u32,
    effects_volume: u32,
}

impl Default for Sound {
    fn default() -> Sound {
        Sound {
            music: None,
            fading: Vec::new(),
            effects: None,
            fading_effects: Vec::new(),
            mix: Vec::new(),
            mask: FULL_MASK,
            music_volume: FULL_VOLUME,
            effects_volume: FULL_VOLUME,
        }
    }
}

impl Sound {
    /// Starts `module` at order `first_order` (counted as the game counts them) and the
    /// configured music `volume` (0..=0x10000), fading out any music playing.
    pub(crate) fn play_music(&mut self, module: &Module, first_order: usize, volume: u32) {
        if let Some(old) = self.music.take() {
            self.fading.extend(old.into_fading());
        }
        self.music_volume = volume;
        self.music = Some(Music::new(module, self.music_gain(), first_order));
    }

    fn music_gain(&self) -> i64 {
        UNITY * music_master(self.music_volume, self.mask) / 256
    }

    /// The effects stream's share: `mask * (volume >> 8) >> 8` of 255 (`musicSetVolume`,
    /// 0x43C250), 254 of 255 while the intro plays.
    fn effects_gain(&self) -> i64 {
        UNITY * ((i64::from(self.mask) * i64::from(self.effects_volume >> 8)) >> 8) / 255
    }

    /// The configured effects volume (0..=0x10000) for the effects stream.
    pub(crate) fn set_effects_volume(&mut self, volume: u32) {
        self.effects_volume = volume;
    }

    /// The volume mask (`setMusicVolume`, 0x43C2B0): 0..=255 over music and effects alike.
    pub(crate) fn set_mask(&mut self, mask: u32) {
        self.mask = mask;
        let gain = self.music_gain();
        if let Some(music) = &mut self.music {
            music.set_gain(gain);
        }
    }

    /// Makes `bank` the source of [`Sound::trigger`]; the old bank's effects fade out.
    pub(crate) fn load_effects(&mut self, bank: &Bank) {
        if let Some(old) = self.effects.take() {
            self.fading_effects.extend(old.into_fading());
        }
        self.effects = Some(Effects::new(bank));
    }

    /// Plays effect `effect` of the loaded bank on `channel` (both 1-based) at full volume and
    /// normal pitch, as the intro does.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8) {
        self.trigger_at(channel, effect, effects::FULL, effects::FULL);
    }

    /// Plays effect `effect` on `channel` at `volume` and `pitch` (16.16, 0x10000 full and
    /// normal), as `loadMenuSoundEffect` (0x43C380) does.
    pub(crate) fn trigger_at(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        if let Some(effects) = &mut self.effects {
            effects.trigger(channel, effect, volume, pitch);
        }
    }

    /// Stops the music and every effect, with a short fade so nothing clicks.
    pub(crate) fn stop(&mut self) {
        if let Some(music) = &mut self.music {
            music.stop();
        }
        if let Some(effects) = &mut self.effects {
            effects.stop_all();
        }
    }

    /// Appends `frames` interleaved stereo frames to `out`.
    pub(crate) fn render(&mut self, frames: usize, out: &mut Vec<i16>) {
        self.mix.clear();
        self.mix.resize(frames * AUDIO_CHANNELS, 0);
        if let Some(music) = &mut self.music {
            music.mix_into(&mut self.mix);
        }
        for voice in &mut self.fading {
            voice.mix_into(&mut self.mix);
        }
        self.fading.retain(|voice| !voice.finished());
        let mut part = vec![0; self.mix.len()];
        if let Some(effects) = &mut self.effects {
            effects.mix_into(&mut part);
        }
        for voice in &mut self.fading_effects {
            voice.mix_into(&mut part);
        }
        self.fading_effects.retain(|voice| !voice.finished());
        let gain = self.effects_gain();
        for (sum, effect) in self.mix.iter_mut().zip(part) {
            *sum += (effect * gain) >> 16;
        }
        out.extend(self.mix.iter().map(|&value| clip(value)));
    }
}

/// Renders `frames` stereo frames of `module` from its first order at the default music
/// volume, mixed as the game plays music in its menus and races.
#[must_use]
pub fn render_music(module: &Module, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.play_music(module, 0, DEFAULT_MUSIC_VOLUME);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

/// Renders effect `effect` (1-based) of `bank` at full volume and normal pitch, as the game
/// triggers it, for `frames` stereo frames.
#[must_use]
pub fn render_effect(bank: &Bank, effect: u8, frames: usize) -> Vec<i16> {
    let mut sound = Sound::default();
    sound.load_effects(bank);
    sound.trigger(1, effect);
    let mut out = Vec::with_capacity(frames * AUDIO_CHANNELS);
    sound.render(frames, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{self, Cell, Channel, Pattern, Sample};
    use deadrally_gamedata::xm::{Instrument, Looping};

    fn bank() -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![Some(Instrument {
                name: "Loud".into(),
                data: vec![i16::MAX; 20_000],
                looping: Looping::None,
                volume: 64,
                finetune: 0,
                relative_note: 0,
                panning: 255,
                fadeout: 0,
            })],
        }
    }

    #[test]
    fn nothing_loaded_is_silence_of_the_right_length() {
        // The frontend paces itself on the audio queue; every tick must deliver its samples.
        let mut out = Vec::new();
        Sound::default().render(672, &mut out);
        assert_eq!(out.len(), 672 * AUDIO_CHANNELS);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn overlapping_effects_clip_instead_of_wrapping() {
        // Four full-scale effects add up to twice the 16-bit range; wrapping would turn the
        // overload into noise, clipping only flattens it.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        for channel in 1..=4 {
            sound.trigger(channel, 1);
        }
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 900], i16::MAX, "left, panned hard left");
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn new_music_fades_the_old_out_instead_of_cutting_it() {
        // The intro's music gives way to the menu music; a cut at full level would click.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut silent = loud.clone();
        silent.orders.clear();
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        assert!(before > 0);
        sound.play_music(&silent, 0, FULL_VOLUME);
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_configured_music_volume_sets_fmods_master_volume() {
        // musicSetmusicVolume (0x43C280): 255 * (volume >> 8) >> 9.
        assert_eq!(music_master(FULL_VOLUME, FULL_MASK), 127);
        assert_eq!(music_master(DEFAULT_MUSIC_VOLUME, FULL_MASK), 63);
        assert_eq!(music_master(0x1_0000, FULL_MASK), 127);
        assert_eq!(music_master(0, FULL_MASK), 0);
    }

    #[test]
    fn a_new_bank_lets_the_old_banks_effects_fade_out() {
        // The intro's effects stop as the menu's bank is loaded; cutting them at full level
        // would click.
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        let before = out[2 * 999];
        sound.stop();
        sound.load_effects(&bank());
        out.clear();
        sound.render(1000, &mut out);
        assert!(
            (out[0] - before).abs() < before / 10,
            "{} after {before}",
            out[0]
        );
        assert_eq!(out[2 * 999], 0, "faded out");
    }

    #[test]
    fn the_effects_volume_and_the_mask_scale_the_effects_stream() {
        // The menus play effects at dr.cfg's 75 %: the stream at 255 * 192 >> 8 = 191 of 255
        // instead of the intro's 254. The end screen's mask lowers everything.
        let level = |sound: &mut Sound| {
            sound.load_effects(&bank());
            sound.trigger(1, 1);
            let mut out = Vec::new();
            sound.render(1000, &mut out);
            i64::from(out[2 * 999])
        };
        let full = level(&mut Sound::default());
        let mut menu = Sound::default();
        menu.set_effects_volume(DEFAULT_EFFECTS_VOLUME);
        let at_75 = level(&mut menu);
        assert!(
            (at_75 * 254 - full * 191).abs() <= 254 * 2,
            "{at_75} vs {full}"
        );
        let mut quiet = Sound::default();
        quiet.set_mask(0);
        assert_eq!(level(&mut quiet), 0);
    }

    #[test]
    fn the_mask_scales_the_music_while_it_plays() {
        // The end screen fades the music out through the mask, 255 down to 0.
        let mut loud = Module {
            title: String::new(),
            orders: vec![0],
            initial_speed: 6,
            initial_tempo: 125,
            global_volume: 64,
            master_volume: 64,
            stereo: false,
            channels: [Channel::default(); s3m::CHANNELS],
            samples: vec![Sample {
                name: String::new(),
                c2spd: 8363,
                volume: 64,
                looped: Some((0, 100)),
                data: vec![20_000; 100],
            }],
            patterns: vec![Pattern {
                rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
            }],
        };
        loud.channels[0].enabled = true;
        loud.patterns[0].rows[0][0].note = 0x40;
        loud.patterns[0].rows[0][0].instrument = 1;
        let mut sound = Sound::default();
        sound.play_music(&loud, 0, FULL_VOLUME);
        let mut out = Vec::new();
        sound.render(2000, &mut out);
        let full = i64::from(out[2 * 1999]);
        sound.set_mask(0x80);
        out.clear();
        sound.render(2000, &mut out);
        // 255 * 255 >> 9 = 127 against 128 * 255 >> 9 = 63.
        assert!(
            (i64::from(out[2 * 1999]) * 127 - full * 63).abs() <= 127 * 2,
            "{} vs {full}",
            out[2 * 1999]
        );
    }

    #[test]
    fn stopping_fades_everything_out() {
        let mut sound = Sound::default();
        sound.load_effects(&bank());
        sound.trigger(1, 1);
        sound.stop();
        let mut out = Vec::new();
        sound.render(1000, &mut out);
        assert_eq!(out[2 * 999], 0);
    }
}
```

<!-- write: crates/core/src/audio/effects.rs -->
```rust
//! Sound effects: instruments of an XM bank triggered one at a time, as the original's
//! modified minifmod does (`FMUSIC_UpdateXMNote`, 0x43EC40; spec M1b §3.3).

use std::sync::Arc;

use deadrally_gamedata::xm::{Bank, Looping};

use super::mixer::{Loop, UNITY, Voice};
use super::tables::scale_by_exp2;

/// Effect channels: the banks have 8 or 16; the game uses 1-6 in the intro.
pub(crate) const CHANNELS: usize = 16;

/// Full volume and normal pitch in the game's calls (`0x10000`).
pub(crate) const FULL: u32 = 0x1_0000;

/// The original's lowest playback rate.
const MIN_HZ: u32 = 100;

#[derive(Debug)]
struct Sound {
    data: Arc<[i16]>,
    looping: Loop,
    relative_note: i32,
    finetune: i32,
    panning: i64,
}

#[derive(Debug)]
pub(crate) struct Effects {
    sounds: Vec<Option<Sound>>,
    channels: [Option<Voice>; CHANNELS],
    /// Voices cut off by a new effect on their channel, fading out.
    fading: Vec<Voice>,
}

impl Effects {
    pub(crate) fn new(bank: &Bank) -> Effects {
        let sounds = bank
            .instruments
            .iter()
            .map(|instrument| {
                instrument.as_ref().map(|instrument| Sound {
                    data: Arc::from(instrument.data.as_slice()),
                    looping: match instrument.looping {
                        Looping::None => Loop::None,
                        Looping::Forward { start, length } => Loop::Forward {
                            start,
                            end: start + length,
                        },
                        Looping::PingPong { start, length } => Loop::PingPong {
                            start,
                            end: start + length,
                        },
                    },
                    relative_note: i32::from(instrument.relative_note),
                    finetune: i32::from(instrument.finetune),
                    panning: i64::from(instrument.panning),
                })
            })
            .collect();
        Effects {
            sounds,
            channels: std::array::from_fn(|_| None),
            fading: Vec::new(),
        }
    }

    /// Plays effect `effect` (1-based) on `channel` (1-based) at `volume` and `pitch`, where
    /// [`FULL`] is full volume and normal pitch. A sound still playing there fades out.
    pub(crate) fn trigger(&mut self, channel: usize, effect: u8, volume: u32, pitch: u32) {
        let slot = channel - 1;
        if let Some(mut old) = self.channels[slot].take() {
            old.release();
            self.fading.push(old);
        }
        let Some(Some(sound)) = usize::from(effect)
            .checked_sub(1)
            .and_then(|index| self.sounds.get(index))
        else {
            return;
        };
        // The volume byte (volume * 64 >> 16) + 16 sets the channel volume 0..=64; the final
        // volume is 64 * volume * 255 / (64 * 64) / 2 of 255 (fadeout and global volume full).
        let channel_volume = i64::from((volume.min(FULL) * 64) >> 16);
        let final_volume = channel_volume * 255 / 128;
        let (left, right) = pan(final_volume, sound.panning);
        let mut voice = Voice::new(Arc::clone(&sound.data), sound.looping, 0);
        voice.set_frequency(frequency(sound, pitch));
        voice.set_volume(left, right);
        self.channels[slot] = Some(voice);
    }

    /// Silences `channel` (1-based) with a short fade.
    pub(crate) fn stop(&mut self, channel: usize) {
        if let Some(mut voice) = self.channels[channel - 1].take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    pub(crate) fn stop_all(&mut self) {
        for channel in 1..=CHANNELS {
            self.stop(channel);
        }
    }

    /// Fades every effect out and hands the fading voices over, for a bank being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop_all();
        self.fading
    }

    /// Adds every playing effect to `out` (interleaved stereo).
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        for slot in &mut self.channels {
            if let Some(voice) = slot {
                voice.mix_into(out);
                if voice.finished() {
                    *slot = None;
                }
            }
        }
        for voice in &mut self.fading {
            voice.mix_into(out);
        }
        self.fading.retain(|voice| !voice.finished());
    }
}

/// minifmod's linear frequency: period `7680 - (6 * pitch / 65536 + 46 + relative) * 64 -
/// finetune / 2`, rounded down, then `8363 * 2^((4608 - period) / 768)` Hz, rounded down.
fn frequency(sound: &Sound, pitch: u32) -> u32 {
    // In 1/512 period units, so the fractions of the original's double arithmetic are exact.
    let period_512 = 7680 * 512
        - 3 * i64::from(pitch)
        - i64::from(46 + sound.relative_note) * 32768
        - i64::from(sound.finetune) * 256;
    let period = period_512.div_euclid(512);
    let steps = i32::try_from(4608 - period).unwrap_or(i32::MAX);
    scale_by_exp2(8363, steps).max(MIN_HZ)
}

/// minifmod's pan law on a 0..=255 volume: left gets `pan / 255`, right `(255 - pan) / 255`.
/// The gains are scaled so 255 becomes [`UNITY`].
fn pan(volume: i64, panning: i64) -> (i64, i64) {
    let left = volume * panning / 255;
    let right = volume * (255 - panning) / 255;
    (left * UNITY / 255, right * UNITY / 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::xm::Instrument;

    fn bank(relative_note: i8, panning: u8) -> Bank {
        Bank {
            linear_frequencies: true,
            instruments: vec![
                None,
                Some(Instrument {
                    name: "Boom".into(),
                    data: vec![1000; 48_000],
                    looping: Looping::None,
                    volume: 64,
                    finetune: 0,
                    relative_note,
                    panning,
                    fadeout: 0,
                }),
            ],
        }
    }

    #[test]
    fn normal_pitch_plays_note_52_plus_the_relative_note() {
        // Period 7680 - 52 * 64 = 4352 is 256 steps above XM's 8363 Hz: 2^(1/3) higher.
        let effects = Effects::new(&bank(0, 128));
        assert_eq!(frequency(effects.sounds[1].as_ref().unwrap(), FULL), 10_536);
        let lower = Effects::new(&bank(-12, 128));
        assert_eq!(
            frequency(lower.sounds[1].as_ref().unwrap(), FULL),
            5268,
            "an octave down"
        );
    }

    #[test]
    fn full_volume_is_127_of_255_split_by_the_pan_law() {
        // The original halves every effect (the 0.5 in its volume formula); playing them at
        // full scale would drown the music.
        assert_eq!(pan(127, 255), (127 * UNITY / 255, 0));
        assert_eq!(pan(127, 0), (0, 127 * UNITY / 255));
        let (left, right) = pan(127, 128);
        assert_eq!(left, 63 * UNITY / 255);
        assert_eq!(right, 63 * UNITY / 255);
    }

    #[test]
    fn a_triggered_effect_plays_and_an_empty_one_is_silent() {
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(3, 2, FULL, FULL);
        let mut out = vec![0i64; 400];
        effects.mix_into(&mut out);
        assert!(out[398] > 0, "{}", out[398]);
        effects.trigger(3, 1, FULL, FULL);
        let mut later = vec![0i64; 2 * 1000];
        effects.mix_into(&mut later);
        assert_eq!(
            later[1998], 0,
            "the old effect faded out, the empty one is silent"
        );
    }

    #[test]
    fn a_retriggered_channel_hands_its_old_sound_over_instead_of_cutting_it() {
        // minifmod moves the old voice to a spare channel and ramps it out while the new one
        // ramps in; a hard cut would click.
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(1, 2, FULL, FULL);
        let mut before = vec![0i64; 2 * 400];
        effects.mix_into(&mut before);
        let level = before[2 * 399];
        effects.trigger(1, 2, FULL, FULL);
        let mut after = vec![0i64; 2 * 20];
        effects.mix_into(&mut after);
        // Ten frames in, the old voice has lost what the new one has gained.
        assert!(
            (after[2 * 10] - level).abs() < level / 10,
            "{} after {level}",
            after[2 * 10]
        );
    }

    #[test]
    fn effect_numbers_outside_the_bank_are_silent() {
        // The HAF tables and the game's calls name effects by number; one the bank lacks must
        // play nothing rather than crash.
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(1, 0, FULL, FULL);
        effects.trigger(2, 3, FULL, FULL);
        effects.trigger(3, 255, FULL, FULL);
        let mut out = vec![0i64; 2 * 1000];
        effects.mix_into(&mut out);
        assert!(out.iter().all(|&sample| sample == 0));
    }

    #[test]
    fn stopping_all_channels_silences_everything() {
        // The intro's end stops channels 1-6; a sound that kept playing would bleed into the logos.
        let mut effects = Effects::new(&bank(0, 128));
        effects.trigger(1, 2, FULL, FULL);
        effects.trigger(6, 2, FULL, FULL);
        effects.stop_all();
        let mut out = vec![0i64; 2 * 1000];
        effects.mix_into(&mut out);
        assert_eq!(out[1998], 0);
    }
}
```

<!-- write: crates/core/src/audio/music.rs -->
```rust
//! The music player: Scream Tracker 3 modules with the 13 commands the game's music uses
//! (spec M1b §3.2, §4.2). Timing is counted in output samples, so the tempo never drifts.

use std::sync::Arc;

use deadrally_gamedata::s3m::{self, Cell, Module, NO_NOTE, NOTE_CUT, ORDER_SKIP};

use super::mixer::{Loop, Voice};
use super::tables::{S3M_PERIODS, vibrato};
use crate::AUDIO_SAMPLE_RATE;

/// Scream Tracker's clock: frequency = 14317056 / period.
const CLOCK: u32 = 14_317_056;
/// FMOD mixes at this rate and counts a tick as a whole number of its samples:
/// `44100 * 5 / (2 * tempo)`, rounded down. At tempo 141 that is 781 samples, not 781.9, so
/// the original plays such music 0.1 % fast; at tempo 125 it is exactly 882 (20 ms).
const FMOD_RATE: u32 = 44_100;
/// Period limits of Scream Tracker 3.
const MIN_PERIOD: i32 = 64;
const MAX_PERIOD: i32 = 32_767;

#[derive(Clone, Debug)]
struct SampleData {
    data: Arc<[i16]>,
    looping: Loop,
    c2spd: u32,
    volume: i32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    enabled: bool,
    /// 0..=255.
    pan: i64,
    voice: Option<Voice>,
    /// Current sample (1-based instrument number), 0 = none yet.
    instrument: u8,
    period: i32,
    target_period: i32,
    /// 0..=64.
    volume: i32,
    /// Vibrato offset of this tick, in period units.
    period_delta: i32,
    command: u8,
    info: u8,
    // Effect memories.
    volume_slide: u8,
    porta: u8,
    tone_porta: u8,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_position: u8,
    offset: u8,
    retrigger: u8,
    retrigger_count: u8,
    /// A row's note held back by SDx until this tick.
    delayed: Option<(u8, Cell)>,
}

#[derive(Debug)]
pub(crate) struct Music {
    orders: Vec<u8>,
    patterns: Vec<s3m::Pattern>,
    samples: Vec<Option<SampleData>>,
    global_volume: i32,
    channels: Vec<Channel>,
    fading: Vec<Voice>,
    speed: u8,
    tempo: u8,
    order: usize,
    row: usize,
    tick: u8,
    /// Output frames left in the current tick, and the carried fraction of a frame.
    frames_left: u32,
    remainder: u32,
    /// Where the next row comes from after a B or C command.
    jump: Option<(usize, usize)>,
    /// Gain applied to every channel, in 16.16 (the original's master volume).
    gain: i64,
    /// The module's own share of it: its master volume, doubled for a stereo module.
    module_gain: i64,
}

impl Music {
    /// A player at order `first_order` (counted as the game counts them, markers included),
    /// row 0 and tick 0. `gain` scales the whole module ([`UNITY`] = as loud as the module
    /// asks). As FMOD does (measured), the module's own master volume scales it by
    /// `master / 64` (the game's music has 48, 2.5 dB below full), and a stereo module plays
    /// at twice a mono module's level.
    pub(crate) fn new(module: &Module, gain: i64, first_order: usize) -> Music {
        let module_gain = i64::from(module.master_volume) * if module.stereo { 2 } else { 1 };
        let gain = gain * module_gain / 64;
        let samples = module
            .samples
            .iter()
            .map(|sample| {
                (!sample.data.is_empty()).then(|| SampleData {
                    data: Arc::from(sample.data.as_slice()),
                    looping: sample
                        .looped
                        .map_or(Loop::None, |(start, end)| Loop::Forward { start, end }),
                    c2spd: sample.c2spd,
                    volume: i32::from(sample.volume),
                })
            })
            .collect();
        let channels = module
            .channels
            .iter()
            .map(|channel| Channel {
                enabled: channel.enabled,
                // FMOD 3 puts a stereo module's channels fully on their side (measured on the
                // menu music); mono modules play in the centre.
                pan: match (module.stereo, channel.pan < 8) {
                    (false, _) => 128,
                    (true, true) => 0,
                    (true, false) => 255,
                },
                ..Channel::default()
            })
            .collect();
        let mut music = Music {
            orders: module.orders.clone(),
            patterns: module.patterns.clone(),
            samples,
            global_volume: i32::from(module.global_volume),
            channels,
            fading: Vec::new(),
            speed: module.initial_speed.max(1),
            tempo: module.initial_tempo.max(32),
            order: first_order,
            row: 0,
            tick: 0,
            frames_left: 0,
            remainder: 0,
            jump: None,
            gain,
            module_gain,
        };
        music.skip_marker_orders();
        music
    }

    /// Fades every channel out, as `FMUSIC_StopSong`.
    pub(crate) fn stop(&mut self) {
        for channel in &mut self.channels {
            if let Some(mut voice) = channel.voice.take() {
                voice.release();
                self.fading.push(voice);
            }
        }
        self.orders.clear();
    }

    /// A new master volume for the song (`FMUSIC_SetMasterVolume`), as [`Music::new`]'s `gain`.
    pub(crate) fn set_gain(&mut self, gain: i64) {
        self.gain = gain * self.module_gain / 64;
    }

    /// Fades every channel out and hands the fading voices over, for music being replaced.
    pub(crate) fn into_fading(mut self) -> Vec<Voice> {
        self.stop();
        self.fading
    }

    /// Adds the music to `out` (interleaved stereo), advancing ticks exactly on time.
    pub(crate) fn mix_into(&mut self, out: &mut [i64]) {
        let mut done = 0;
        let frames = out.len() / 2;
        while done < frames {
            if self.frames_left == 0 {
                if !self.orders.is_empty() {
                    self.process_tick();
                }
                self.start_tick_timer();
            }
            let count = (frames - done).min(self.frames_left as usize);
            let part = &mut out[2 * done..2 * (done + count)];
            for channel in &mut self.channels {
                if let Some(voice) = &mut channel.voice {
                    voice.mix_into(part);
                    if voice.finished() {
                        channel.voice = None;
                    }
                }
            }
            for voice in &mut self.fading {
                voice.mix_into(part);
            }
            self.fading.retain(|voice| !voice.finished());
            done += count;
            self.frames_left -= u32::try_from(count).expect("at most a tick");
        }
    }

    /// The next tick's length: FMOD's whole number of 44.1 kHz samples, converted to our rate
    /// exactly by carrying the remainder.
    fn start_tick_timer(&mut self) {
        let fmod_samples = FMOD_RATE * 5 / (2 * u32::from(self.tempo));
        let scaled = fmod_samples * AUDIO_SAMPLE_RATE + self.remainder;
        self.frames_left = scaled / FMOD_RATE;
        self.remainder = scaled % FMOD_RATE;
    }

    /// Moves past 254 and 255, which FMOD drops from the order list.
    fn skip_marker_orders(&mut self) {
        while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
            self.order += 1;
        }
        if self.order >= self.orders.len() {
            // The song loops from its start, as FMOD's looping music does.
            self.order = 0;
            while self.order < self.orders.len() && self.orders[self.order] >= ORDER_SKIP {
                self.order += 1;
            }
        }
    }

    fn process_tick(&mut self) {
        if self.tick == 0 {
            self.process_row();
        } else {
            for index in 0..self.channels.len() {
                self.channel_tick(index);
            }
        }
        for index in 0..self.channels.len() {
            self.update_voice(index);
        }
        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.next_row();
        }
    }

    fn next_row(&mut self) {
        if let Some((order, row)) = self.jump.take() {
            self.order = order;
            self.row = row;
            self.skip_marker_orders();
            return;
        }
        self.row += 1;
        if self.row >= s3m::ROWS {
            self.row = 0;
            self.order += 1;
            self.skip_marker_orders();
        }
    }

    fn process_row(&mut self) {
        let Some(&pattern) = self.orders.get(self.order) else {
            return;
        };
        let cells = self.patterns[usize::from(pattern)].rows[self.row];
        for (index, cell) in cells.iter().enumerate() {
            if !self.channels[index].enabled {
                continue;
            }
            let channel = &mut self.channels[index];
            channel.command = cell.command;
            channel.info = cell.info;
            channel.period_delta = 0;
            if cell.command == command('S') && cell.info >> 4 == 0xD && cell.info & 0xF > 0 {
                channel.delayed = Some((cell.info & 0xF, *cell));
                continue;
            }
            channel.delayed = None;
            self.start_cell(index, cell);
            self.row_effect(index, cell);
        }
    }

    /// The note, instrument and volume of a cell.
    fn start_cell(&mut self, index: usize, cell: &Cell) {
        let tone_porta = cell.command == command('G');
        if cell.instrument != 0 {
            let channel = &mut self.channels[index];
            channel.instrument = cell.instrument;
            if let Some(Some(sample)) = self.samples.get(usize::from(cell.instrument) - 1) {
                channel.volume = sample.volume;
            }
        }
        if cell.note == NOTE_CUT {
            self.cut(index);
        } else if cell.note != NO_NOTE {
            let instrument = self.channels[index].instrument;
            if let Some(Some(sample)) = usize::from(instrument)
                .checked_sub(1)
                .and_then(|i| self.samples.get(i))
            {
                let period = note_period(cell.note, sample.c2spd);
                let channel = &mut self.channels[index];
                if tone_porta && channel.voice.is_some() {
                    channel.target_period = period;
                } else {
                    let offset = if cell.command == command('O') {
                        if cell.info != 0 {
                            channel.offset = cell.info;
                        }
                        u32::from(channel.offset) * 256
                    } else {
                        0
                    };
                    let voice = Voice::new(Arc::clone(&sample.data), sample.looping, offset);
                    if let Some(mut old) = channel.voice.replace(voice) {
                        old.release();
                        self.fading.push(old);
                    }
                    let channel = &mut self.channels[index];
                    channel.period = period;
                    channel.target_period = period;
                    channel.vibrato_position = 0;
                    channel.retrigger_count = 0;
                }
            } else if instrument != 0 && !(tone_porta && self.channels[index].voice.is_some()) {
                // FMOD plays a note of an empty sample slot as silence: the note sounding stops.
                self.cut(index);
            }
        }
        if let Some(volume) = cell.volume {
            self.channels[index].volume = i32::from(volume);
        }
    }

    fn cut(&mut self, index: usize) {
        if let Some(mut voice) = self.channels[index].voice.take() {
            voice.release();
            self.fading.push(voice);
        }
    }

    /// Commands that act on the row's first tick.
    fn row_effect(&mut self, index: usize, cell: &Cell) {
        let info = cell.info;
        let channel = &mut self.channels[index];
        match letter(cell.command) {
            'A' if info > 0 => self.speed = info,
            'T' if info >= 0x20 => self.tempo = info,
            'B' => {
                let row = self.jump.map_or(0, |(_, row)| row);
                self.jump = Some((usize::from(info), row));
            }
            'C' => {
                let row = usize::from((info >> 4) * 10 + (info & 0xF)).min(s3m::ROWS - 1);
                let order = self.jump.map_or(self.order + 1, |(order, _)| order);
                self.jump = Some((order, row));
            }
            'D' | 'K' => {
                if info != 0 {
                    channel.volume_slide = info;
                }
                let slide = channel.volume_slide;
                // Fine slides (DxF up, DFy down) act once, on this tick.
                if slide & 0x0F == 0x0F && slide >> 4 != 0 {
                    channel.volume = (channel.volume + i32::from(slide >> 4)).min(64);
                } else if slide >> 4 == 0x0F && slide & 0x0F != 0 {
                    channel.volume = (channel.volume - i32::from(slide & 0x0F)).max(0);
                }
            }
            'E' | 'F' => {
                if info != 0 {
                    channel.porta = info;
                }
                let porta = channel.porta;
                let sign = if letter(cell.command) == 'E' { 1 } else { -1 };
                // EFx fine (x * 4), EEx extra fine (x), on this tick only.
                match porta >> 4 {
                    0xF => channel.period += sign * 4 * i32::from(porta & 0xF),
                    0xE => channel.period += sign * i32::from(porta & 0xF),
                    _ => {}
                }
                channel.period = channel.period.clamp(MIN_PERIOD, MAX_PERIOD);
            }
            'G' => {
                if info != 0 {
                    channel.tone_porta = info;
                }
            }
            'H' => {
                if info >> 4 != 0 {
                    channel.vibrato_speed = info >> 4;
                }
                if info & 0xF != 0 {
                    channel.vibrato_depth = info & 0xF;
                }
            }
            'Q' if info != 0 => channel.retrigger = info,
            _ => {}
        }
    }

    /// Commands that act on every tick but the first.
    fn channel_tick(&mut self, index: usize) {
        if !self.channels[index].enabled {
            return;
        }
        if let Some((at, cell)) = self.channels[index].delayed {
            if self.tick == at {
                self.channels[index].delayed = None;
                self.start_cell(index, &cell);
            }
            return;
        }
        let channel = &mut self.channels[index];
        channel.period_delta = 0;
        match letter(channel.command) {
            'D' => volume_slide(channel),
            'K' => {
                volume_slide(channel);
                vibrato_tick(channel);
            }
            'E' | 'F' => {
                let porta = channel.porta;
                if porta >> 4 < 0xE {
                    let sign = if letter(channel.command) == 'E' {
                        1
                    } else {
                        -1
                    };
                    channel.period = (channel.period + sign * 4 * i32::from(porta))
                        .clamp(MIN_PERIOD, MAX_PERIOD);
                }
            }
            'G' => {
                let speed = 4 * i32::from(channel.tone_porta);
                if channel.period < channel.target_period {
                    channel.period = (channel.period + speed).min(channel.target_period);
                } else {
                    channel.period = (channel.period - speed).max(channel.target_period);
                }
            }
            'H' => vibrato_tick(channel),
            // Without an interval nothing repeats, and nothing is counted.
            'Q' if channel.retrigger & 0xF != 0 => {
                let interval = channel.retrigger & 0xF;
                channel.retrigger_count += 1;
                if channel.retrigger_count >= interval {
                    channel.retrigger_count = 0;
                    channel.volume = retrigger_volume(channel.volume, channel.retrigger >> 4);
                    let sample = usize::from(channel.instrument)
                        .checked_sub(1)
                        .and_then(|i| self.samples.get(i))
                        .and_then(Option::as_ref);
                    if let Some(sample) = sample {
                        let voice = Voice::new(Arc::clone(&sample.data), sample.looping, 0);
                        if let Some(mut old) = channel.voice.replace(voice) {
                            old.release();
                            self.fading.push(old);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Pushes the channel's pitch and volume into its voice.
    fn update_voice(&mut self, index: usize) {
        let global = i64::from(self.global_volume);
        let gain = self.gain;
        let channel = &mut self.channels[index];
        let Some(voice) = &mut channel.voice else {
            return;
        };
        let period = (channel.period + channel.period_delta).clamp(MIN_PERIOD, MAX_PERIOD);
        voice.set_frequency(CLOCK / u32::try_from(period).expect("positive"));
        let volume = i64::from(channel.volume) * global * gain / (64 * 64);
        let left = volume * (255 - channel.pan) / 255;
        let right = volume * channel.pan / 255;
        voice.set_volume(left, right);
    }

    #[cfg(test)]
    fn position(&self) -> (usize, usize, u8) {
        (self.order, self.row, self.tick)
    }
}

fn command(letter: char) -> u8 {
    letter as u8 - b'@'
}

fn letter(command: u8) -> char {
    if (1..=26).contains(&command) {
        char::from(b'@' + command)
    } else {
        ' '
    }
}

/// Scream Tracker's period of a note byte (octave in the high nibble) for a sample's C2SPD.
/// The octave shift comes last, so high notes keep their precision.
fn note_period(note: u8, c2spd: u32) -> i32 {
    let (octave, semitone) = (u32::from(note >> 4), usize::from(note & 0xF).min(11));
    let period =
        (8363 * 16 * u64::from(S3M_PERIODS[semitone]) / u64::from(c2spd.max(1))) >> octave.min(9);
    i32::try_from(period)
        .unwrap_or(MAX_PERIOD)
        .clamp(MIN_PERIOD, MAX_PERIOD)
}

fn volume_slide(channel: &mut Channel) {
    let slide = channel.volume_slide;
    let (up, down) = (slide >> 4, slide & 0x0F);
    if down == 0 && up != 0 {
        channel.volume = (channel.volume + i32::from(up)).min(64);
    } else if up == 0 && down != 0 {
        channel.volume = (channel.volume - i32::from(down)).max(0);
    }
}

fn vibrato_tick(channel: &mut Channel) {
    let wave = vibrato(channel.vibrato_position);
    channel.period_delta = (wave * i32::from(channel.vibrato_depth)) >> 5;
    channel.vibrato_position = (channel.vibrato_position + channel.vibrato_speed) & 63;
}

/// Qxy's volume change `x` applied to a volume 0..=64.
fn retrigger_volume(volume: i32, change: u8) -> i32 {
    let changed = match change {
        1..=5 => volume - (1 << (change - 1)),
        6 => volume * 2 / 3,
        7 => volume / 2,
        9..=13 => volume + (1 << (change - 9)),
        14 => volume * 3 / 2,
        15 => volume * 2,
        _ => volume,
    };
    changed.clamp(0, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use deadrally_gamedata::s3m::{Channel as S3mChannel, Pattern, Sample};

    use crate::audio::mixer::UNITY;

    const C4: u8 = 0x40;

    fn module(rows: &[(usize, usize, Cell)], speed: u8, tempo: u8) -> Module {
        let mut pattern = Pattern {
            rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
        };
        for &(row, channel, cell) in rows {
            pattern.rows[row][channel] = cell;
        }
        let mut channels = [S3mChannel::default(); s3m::CHANNELS];
        channels[0] = S3mChannel {
            enabled: true,
            pan: 3,
        };
        channels[1] = S3mChannel {
            enabled: true,
            pan: 12,
        };
        Module {
            title: "Test".into(),
            orders: vec![0, 1],
            initial_speed: speed,
            initial_tempo: tempo,
            global_volume: 64,
            master_volume: 48,
            stereo: false,
            channels,
            samples: vec![Sample {
                name: "Tone".into(),
                c2spd: 8363,
                volume: 32,
                looped: Some((0, 100)),
                data: vec![10_000; 100],
            }],
            patterns: vec![pattern.clone(), pattern],
        }
    }

    fn cell(note: u8, instrument: u8, volume: Option<u8>, command: char, info: u8) -> Cell {
        Cell {
            note,
            instrument,
            volume,
            command: if command == ' ' {
                0
            } else {
                super::command(command)
            },
            info,
        }
    }

    fn play(music: &mut Music, frames: usize) -> Vec<i64> {
        let mut out = vec![0; 2 * frames];
        music.mix_into(&mut out);
        out
    }

    #[test]
    fn a_tick_lasts_fmods_whole_number_of_samples() {
        // At tempo 125 a tick is 882 samples at 44.1 kHz, exactly 20 ms: 960 of ours.
        let mut music = Music::new(&module(&[], 6, 125), UNITY, 0);
        play(&mut music, 960 * 6);
        assert_eq!(music.position(), (0, 1, 0));
        // At tempo 141 FMOD counts 781 samples (not 781.9) per tick, which is why the menu
        // music runs 0.1 % fast in the original: 147 ticks are exactly 124 960 of our samples.
        let mut faster = Music::new(&module(&[], 1, 141), UNITY, 0);
        play(&mut faster, 124_960);
        assert_eq!(
            faster.position(),
            (0, 19, 0),
            "147 rows: both patterns, then the song loops to row 19 of its start"
        );
    }

    #[test]
    fn middle_c_plays_at_the_samples_c2spd() {
        assert_eq!(note_period(C4, 8363), 1712);
        assert_eq!(CLOCK / 1712, 8362);
        assert_eq!(
            note_period(0x50, 8363),
            856,
            "an octave up halves the period"
        );
        assert_eq!(
            note_period(C4, 16_726),
            856,
            "a doubled C2SPD sounds an octave up"
        );
    }

    #[test]
    fn notes_play_with_the_samples_volume_and_the_volume_column_overrides_it() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(C4, 1, Some(64), ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        let out = play(&mut music, 960);
        // Mono: centred, 32 / 64 of full volume on each side, and the module's master volume
        // 48 / 64 on top.
        let expected = 10_000 * 32 / 64 * 127 / 255 * 48 / 64;
        assert!(
            (out[2 * 900] - expected).abs() <= 2,
            "{} vs {expected}",
            out[2 * 900]
        );
        let louder = play(&mut music, 960);
        assert!(
            (louder[2 * 900] - 2 * expected).abs() <= 4,
            "{}",
            louder[2 * 900]
        );
    }

    #[test]
    fn speed_tempo_jump_and_break_commands_steer_the_song() {
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(NO_NOTE, 0, None, 'A', 2)),
                    (0, 1, cell(NO_NOTE, 0, None, 'C', 0x10)),
                ],
                6,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 2);
        assert_eq!(
            music.position(),
            (1, 10, 0),
            "speed 2, then a break to row 10 of the next order"
        );
        let mut jumping = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'B', 0))], 1, 125),
            UNITY,
            0,
        );
        play(&mut jumping, 960);
        assert_eq!(jumping.position(), (0, 0, 0), "B00 loops the first order");
        // As in minifmod, a new tempo already sets the length of the tick that sets it.
        let mut tempo = Music::new(
            &module(&[(0, 0, cell(NO_NOTE, 0, None, 'T', 250))], 1, 125),
            UNITY,
            0,
        );
        play(&mut tempo, 480 * 3);
        assert_eq!(
            tempo.position(),
            (0, 3, 0),
            "tempo 250: ticks of 480 samples"
        );
    }

    #[test]
    fn stereo_modules_play_each_channel_fully_on_its_side_and_twice_as_loud() {
        // FMOD 3 pans a stereo module's channels hard left or right, and plays them at twice a
        // mono module's level: only that matches the stereo image and the loudness of the
        // original's menu music, recorded at two music volumes (docs/verification/m1b.md).
        let mut stereo = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        stereo.stereo = true;
        let mut music = Music::new(&stereo, UNITY, 0);
        let out = play(&mut music, 960);
        let full = 10_000 * 48 / 64 * 2;
        assert!((out[2 * 900] - full).abs() <= 4, "left {}", out[2 * 900]);
        assert_eq!(out[2 * 900 + 1], 0, "right");
    }

    #[test]
    fn a_song_can_start_at_a_later_order() {
        // The game starts the menu music at order 45 (musicSetOrder), not at its beginning.
        let mut later = module(&[], 1, 125);
        later.orders = vec![0, s3m::ORDER_END, 1];
        let music = Music::new(&later, UNITY, 2);
        assert_eq!(music.position(), (2, 0, 0));
        let at_marker = Music::new(&later, UNITY, 1);
        assert_eq!(at_marker.position(), (2, 0, 0), "a marker is skipped");
    }

    #[test]
    fn playback_runs_on_through_section_markers_as_fmod_does() {
        // FMOD drops 254 and 255 from the order list (the game numbers orders for
        // FMUSIC_SetOrder without them), so a pattern before a 255 is followed by the next
        // section, not by the song's start.
        let mut base = module(&[], 1, 125);
        base.orders = vec![0, s3m::ORDER_END, s3m::ORDER_SKIP, 1];
        let mut music = Music::new(&base, UNITY, 0);
        play(&mut music, 960 * s3m::ROWS);
        assert_eq!(music.position(), (3, 0, 0));
    }

    /// A module whose one sample rises steadily (0, 8, 16, ...), so the output shows how far
    /// into the sample a voice is.
    fn rising(rows: &[(usize, usize, Cell)], speed: u8) -> Module {
        let mut rising = module(rows, speed, 125);
        rising.samples[0].data = (0..4000).map(|i| i16::try_from(i * 8).unwrap()).collect();
        rising.samples[0].looped = None;
        rising
    }

    #[test]
    fn the_sample_offset_starts_a_note_further_into_its_sample() {
        // O (545 times in TR5) starts notes part way into their sample; ignoring it would play
        // the sample's beginning instead.
        let mut plain = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 6),
            UNITY,
            0,
        );
        let mut offset = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(64), 'O', 8))], 6),
            UNITY,
            0,
        );
        let (plain, offset) = (play(&mut plain, 300), play(&mut offset, 300));
        // 8 * 256 = 2048 samples in: about 2100 instead of about 50 at frame 299.
        assert!(plain[2 * 299] > 0);
        assert!(
            offset[2 * 299] > 20 * plain[2 * 299],
            "{} vs {}",
            offset[2 * 299],
            plain[2 * 299]
        );
    }

    #[test]
    fn vibrato_with_volume_slide_does_both() {
        // K (277 times in TR1) keeps an earlier H's vibrato going while it slides the volume;
        // doing only one of the two freezes the note or its level.
        let mut music = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, Some(10), 'H', 0x48)),
                    (1, 0, cell(NO_NOTE, 0, None, 'K', 0x20)),
                ],
                4,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        let position = music.channels[0].vibrato_position;
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        assert_eq!(
            music.channels[0].vibrato_position,
            position + 3 * 4,
            "the vibrato goes on at speed 4"
        );
    }

    #[test]
    fn portamento_up_lowers_the_period_and_its_fine_form_acts_once() {
        // F (252 times in TR5) slides notes up; the wrong direction, or FFx on every tick,
        // would detune whole phrases.
        let mut up = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut up, 960 * 3);
        assert_eq!(up.channels[0].period, 1712 - 2 * 8, "2 ticks of 4 * 2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'F', 0xF3))], 3, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 3);
        assert_eq!(
            fine.channels[0].period,
            1712 - 3 * 4,
            "FF3: once, on the first tick"
        );
    }

    #[test]
    fn retrigger_restarts_the_note_at_its_interval_and_changes_its_volume() {
        // Q (90 times in TR9) drums a note several times per row; without the restarts, or the
        // volume change, a roll becomes one long note.
        let mut music = Music::new(
            &rising(&[(0, 0, cell(C4, 1, Some(20), 'Q', 0xA3))], 7),
            UNITY,
            0,
        );
        let out = play(&mut music, 960 * 7);
        // Restarted at tick 3: 200 frames later a voice plays again, near the sample's start.
        let (restarted, before) = (out[2 * (3 * 960 + 200)], out[2 * (3 * 960 - 1)]);
        assert!(
            restarted > 0 && restarted < before / 4,
            "{restarted} vs {before}"
        );
        assert_eq!(
            music.channels[0].volume, 24,
            "+2 at each restart, on ticks 3 and 6"
        );
    }

    #[test]
    fn a_retrigger_without_an_interval_never_overflows() {
        // Q00 before any Q with an interval repeats nothing; counting its ticks anyway overflowed
        // after 255 of them and crashed the game in the middle of the music.
        let mut long = module(
            &[
                (0, 0, cell(C4, 1, Some(64), 'Q', 0)),
                (1, 0, cell(NO_NOTE, 0, None, 'Q', 0)),
            ],
            200,
            125,
        );
        long.orders = vec![0];
        let mut music = Music::new(&long, UNITY, 0);
        play(&mut music, 960 * 400);
        assert_eq!(music.position(), (0, 2, 0));
    }

    #[test]
    fn a_song_of_markers_only_stays_silent() {
        // An order list without a pattern must not hang the player looking for one.
        let mut base = module(&[(0, 0, cell(C4, 1, Some(64), ' ', 0))], 1, 125);
        base.orders = vec![s3m::ORDER_SKIP, s3m::ORDER_END];
        let mut music = Music::new(&base, UNITY, 0);
        assert!(play(&mut music, 960 * 4).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn volume_slides_act_after_the_first_tick_and_fine_ones_on_it() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x20))], 4, 125),
            UNITY,
            0,
        );
        play(&mut music, 960 * 4);
        assert_eq!(music.channels[0].volume, 16, "3 ticks of +2");
        let mut fine = Music::new(
            &module(&[(0, 0, cell(C4, 1, Some(10), 'D', 0x3F))], 4, 125),
            UNITY,
            0,
        );
        play(&mut fine, 960 * 4);
        assert_eq!(fine.channels[0].volume, 13, "one fine step of +3");
    }

    #[test]
    fn portamentos_move_the_period() {
        let mut down = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'E', 2))], 3, 125),
            UNITY,
            0,
        );
        play(&mut down, 960 * 3);
        assert_eq!(down.channels[0].period, 1712 + 2 * 8);
        let mut towards = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(0x50, 1, None, 'G', 0xFF)),
                ],
                3,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut towards, 960 * 6);
        assert_eq!(
            towards.channels[0].period, 856,
            "the tone portamento stops at its target"
        );
    }

    #[test]
    fn vibrato_wobbles_around_the_note() {
        let mut music = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'H', 0x48))], 6, 125),
            UNITY,
            0,
        );
        let mut deltas = Vec::new();
        for _ in 0..6 {
            play(&mut music, 960);
            deltas.push(music.channels[0].period_delta);
        }
        assert_eq!(deltas[0], 0, "no vibrato on the first tick");
        assert!(
            deltas[1..].iter().all(|&delta| delta >= 0) && deltas[5] > 0,
            "{deltas:?}"
        );
        assert_eq!(
            music.channels[0].period, 1712,
            "the note itself does not move"
        );
    }

    #[test]
    fn a_note_of_an_empty_sample_slot_silences_the_channel() {
        // The menu music's order 47 starts with notes of a sample slot its author emptied;
        // FMOD plays them as silence. Ignoring them left the channel's looping note playing,
        // brought back up by their volume: a stray tone in the menu.
        let mut emptied = module(
            &[
                (0, 0, cell(C4, 1, Some(64), ' ', 0)),
                (1, 0, cell(C4, 2, Some(32), ' ', 0)),
            ],
            1,
            125,
        );
        emptied.samples.push(Sample::default());
        let mut music = Music::new(&emptied, UNITY, 0);
        play(&mut music, 960);
        let out = play(&mut music, 960);
        assert!(out[2 * 900..].iter().all(|&sample| sample == 0));
    }

    #[test]
    fn note_delay_and_cut_and_retrigger() {
        let mut delayed = Music::new(
            &module(&[(0, 0, cell(C4, 1, None, 'S', 0xD2))], 4, 125),
            UNITY,
            0,
        );
        play(&mut delayed, 960 * 2);
        assert!(delayed.channels[0].voice.is_none(), "not before tick 2");
        play(&mut delayed, 960);
        assert!(delayed.channels[0].voice.is_some());
        let mut cut = Music::new(
            &module(
                &[
                    (0, 0, cell(C4, 1, None, ' ', 0)),
                    (1, 0, cell(NOTE_CUT, 0, None, ' ', 0)),
                ],
                1,
                125,
            ),
            UNITY,
            0,
        );
        play(&mut cut, 960 * 2);
        assert!(cut.channels[0].voice.is_none());
        assert_eq!(retrigger_volume(40, 0xF), 64);
        assert_eq!(retrigger_volume(40, 0x7), 20);
        assert_eq!(retrigger_volume(40, 0x3), 36);
    }

    #[test]
    fn the_same_module_always_renders_the_same_samples() {
        let rows = [
            (0, 0, cell(C4, 1, Some(40), 'H', 0x46)),
            (8, 1, cell(0x45, 1, None, 'Q', 0x93)),
        ];
        let render = || {
            let mut music = Music::new(&module(&rows, 3, 131), UNITY, 0);
            play(&mut music, 48_000)
        };
        assert_eq!(render(), render());
    }
}
```

- [ ] **Step 20: Run them to see them pass**

Run: `cargo test -p deadrally-core --lib audio`
Expected: all pass.

- [ ] **Step 21: Write the failing tests of the scene**

`menu.rs` drives the whole game on synthetic assets: from the title into the menu, the highlight, the sounds (by side: the fixture's back sound is left only, the move sound right only, the choose sound both), the gamepad, the submenu, the exit question, the end screen, quitting and the credits. The title is now 640x480 in `startup.rs`'s assets, as the menu takes it over as its screen, and the title test follows it into the fade to black.

<!-- write: crates/core/tests/menu.rs -->
```rust
//! The main menu on synthetic assets (spec M2a §3.2–§3.5): what a player of the original sees
//! and hears while moving through it. Every picture of the fixture has a colour of its own
//! (`common`), so a pixel tells which font, cursor frame or screen is drawn there.

mod common;

use common::{BACKGROUND, BIG_A, BIG_B, BIG_D, CREDITS, CURSOR, END, SMALL};
use deadrally_core::{Game, InputEvent, Key, PadAxis, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::haf::Animation;
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::Bank;

/// Without an intro: two logos of 25 + 180 + 26 ticks, the title's 25-tick fade-in, its
/// 26-tick fade to black and the menu's 50-tick fade-in.
const MENU_SHOWN: u32 = 2 * (25 + 180 + 26) + 25 + 26 + 50;
/// Where the main menu and the start submenu stand.
const MAIN: (usize, usize) = (145, 124);
const START: (usize, usize) = (109, 171);
/// Inside the exit question's "yes" and "no".
const YES: (usize, usize) = (212, 241);
const NO: (usize, usize) = (382, 241);

/// Which sides an effect sounded on: the fixture's back sound is left only, the move sound
/// right only, the choose sound both.
const SILENT: (bool, bool) = (false, false);
const BACK: (bool, bool) = (true, false);
const MOVE: (bool, bool) = (false, true);
const CHOOSE: (bool, bool) = (true, true);

fn picture(pixel: u8, width: u32, height: u32) -> Picture {
    Picture {
        image: Image::new(width, height, vec![pixel; (width * height) as usize]),
        palette: Palette::BLACK,
    }
}

/// A module that is silent, or plays one endless tone from order 45 on, where the original
/// starts the menu music.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let silent = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    let mut playing = silent.clone();
    playing.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone {
            [vec![1; 45], vec![0]].concat()
        } else {
            Vec::new()
        },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![playing, silent],
    }
}

fn assets() -> Assets {
    Assets {
        intro: Animation::from_frames(Vec::new(), Vec::new()),
        letterbox: picture(0, 320, 200),
        apogee: picture(1, 4, 3),
        remedy: picture(2, 4, 3),
        title: picture(3, 640, 480),
        intro_music: music(false),
        intro_effects: Bank {
            linear_frequencies: true,
            instruments: Vec::new(),
        },
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// A game in the main menu, its fade-in over.
fn in_menu(assets: Assets) -> Game {
    let mut game = Game::new(assets);
    run(&mut game, MENU_SHOWN);
    game
}

fn press(game: &mut Game, key: Key) {
    game.input(InputEvent::Key { key, pressed: true });
    game.input(InputEvent::Key {
        key,
        pressed: false,
    });
}

/// Presses `key` and runs two menu passes: the first reads it, the second shows the result
/// everywhere (the exit question copies its words to the screen a pass later).
fn step(game: &mut Game, key: Key) {
    press(game, key);
    run(game, 4);
}

/// Waits for earlier effects to end, presses `key`, and tells which sides sounded.
fn sound_after(game: &mut Game, key: Key) -> (bool, bool) {
    run(game, 100);
    game.take_audio(&mut Vec::new());
    step(game, key);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    let side = |side: usize| audio.iter().skip(side).step_by(2).any(|&s| s != 0);
    (side(0), side(1))
}

fn pixel(game: &Game, (x, y): (usize, usize)) -> u8 {
    game.frame().pixels[y * 640 + x]
}

/// The font each of a menu's six rows is drawn in, read inside the row's glyph.
fn row_fonts(game: &Game, (x, y): (usize, usize)) -> Vec<u8> {
    (0..6)
        .map(|row| pixel(game, (x + 33, y + 15 + 28 * row)))
        .collect()
}

/// The main menu's highlighted row.
fn selected(game: &Game) -> usize {
    let fonts = row_fonts(game, MAIN);
    fonts
        .iter()
        .position(|&font| font == BIG_A)
        .unwrap_or_else(|| panic!("no row highlighted: {fonts:?}"))
}

/// How the shown palette makes `colour`.
fn colour(game: &Game, colour: u8) -> [u8; 3] {
    game.frame().palette[usize::from(colour)]
}

#[test]
fn the_menu_fades_in_after_the_title_and_stays_just_below_full_brightness() {
    // The original's fade-in stops at 98 %: white shows as 62, never 63.
    let mut game = Game::new(assets());
    run(&mut game, MENU_SHOWN - 1);
    let before = colour(&game, BACKGROUND);
    game.tick();
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (640, 480, (4, 3))
    );
    assert_eq!(pixel(&game, (5, 5)), BACKGROUND);
    assert!(before[0] < 62, "{before:?}");
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    run(&mut game, 300);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
}

#[test]
fn the_main_menu_highlights_start_and_dims_multiplayer() {
    let game = in_menu(assets());
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
    let cursor = pixel(&game, (MAIN.0 + 15, MAIN.1 + 16));
    assert!((CURSOR..CURSOR + 50).contains(&cursor), "{cursor}");
}

#[test]
fn the_bottom_panel_shows_the_start_up_lines_with_a_gap_before_the_last() {
    // `mainMenu` pushes three lines, an empty one and the last into the 22-line panel, which
    // shows its last six at (12, 378 + 15k) in small B.
    let game = in_menu(assets());
    let line = |k: usize| pixel(&game, (13, 378 + 15 * k + 8));
    assert_eq!(
        (0..6).map(line).collect::<Vec<_>>(),
        [
            BACKGROUND, SMALL[1], SMALL[1], SMALL[1], BACKGROUND, SMALL[1]
        ]
    );
}

#[test]
fn up_and_down_move_the_highlight_past_the_inactive_row_and_around() {
    let mut game = in_menu(assets());
    let rows: Vec<usize> = [Key::Down, Key::Down, Key::Up, Key::Up, Key::Up, Key::Down]
        .into_iter()
        .map(|key| {
            step(&mut game, key);
            selected(&game)
        })
        .collect();
    assert_eq!(rows, [2, 3, 2, 0, 5, 0]);
    let fonts = row_fonts(&game, MAIN);
    assert_eq!(fonts, [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]);
}

#[test]
fn each_move_sounds_and_other_keys_are_ignored() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(sound_after(&mut game, Key::Up), MOVE);
    assert_eq!(sound_after(&mut game, Key::A), SILENT);
    assert_eq!(selected(&game), 0);
}

#[test]
fn escape_jumps_to_exit_once() {
    // Escape in the main menu only moves the highlight to the last row; there it does
    // nothing, so a player cannot leave the game by pressing it twice.
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Escape), MOVE);
    assert_eq!(selected(&game), 5);
    assert_eq!(sound_after(&mut game, Key::Escape), SILENT);
    assert_eq!(selected(&game), 5);
    assert!(!game.quit_requested());
}

#[test]
fn the_cursor_turns_one_frame_every_menu_pass() {
    // A pass waits two ticks; the cursor runs through its 50 frames in 100 ticks.
    let mut game = in_menu(assets());
    let at = (MAIN.0 + 15, MAIN.1 + 16);
    let frames: Vec<u8> = (0..6)
        .map(|_| {
            game.tick();
            pixel(&game, at)
        })
        .collect();
    let k = frames[0];
    assert_eq!(frames, [k, k + 1, k + 1, k + 2, k + 2, k + 3]);
    run(&mut game, 100);
    assert_eq!(pixel(&game, at), k + 3);
}

#[test]
fn the_highlights_pulse_while_the_menu_waits() {
    // Entries 16–31 go down from 100 % to 49 % and back up in 34 ticks.
    let mut game = in_menu(assets());
    let levels: Vec<u8> = (0..35)
        .map(|_| {
            game.tick();
            colour(&game, 16)[0]
        })
        .collect();
    assert_eq!(levels[34], levels[0]);
    let lowest = *levels.iter().min().unwrap();
    assert!((30..=31).contains(&lowest), "{levels:?}");
    assert_eq!(*levels.iter().max().unwrap(), 63, "{levels:?}");
}

#[test]
fn a_held_key_moves_the_highlight_again_after_half_a_second() {
    let mut game = in_menu(assets());
    game.input(InputEvent::Key {
        key: Key::Down,
        pressed: true,
    });
    run(&mut game, 30);
    assert_eq!(selected(&game), 2, "one move in the first 420 ms");
    run(&mut game, 14);
    assert_ne!(selected(&game), 2, "SDL's repeat has started");
}

#[test]
fn the_gamepad_moves_and_chooses_like_the_keys() {
    // `eventDetected` treats a push as fresh when it was last called within 400 ms without
    // the stick: a pass of the menu first.
    let mut game = in_menu(assets());
    run(&mut game, 2);
    game.input(InputEvent::PadAxis {
        axis: PadAxis::StickY,
        value: 20_000,
    });
    run(&mut game, 20);
    assert_eq!(
        selected(&game),
        2,
        "a push moves once, then holds off for 700 ms"
    );
    let stick = |game: &mut Game, value: i16| {
        game.input(InputEvent::PadAxis {
            axis: PadAxis::StickY,
            value,
        });
        run(game, 4);
    };
    stick(&mut game, 0);
    stick(&mut game, -20_000);
    stick(&mut game, 0);
    assert_eq!(selected(&game), 0);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: false,
    });
    run(&mut game, 4);
    assert_eq!(
        row_fonts(&game, MAIN)[0],
        BIG_D,
        "the main menu has lost focus"
    );
}

#[test]
fn start_opens_its_submenu_and_escape_closes_it() {
    let mut game = in_menu(assets());
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(
        row_fonts(&game, START),
        [BIG_A, BIG_D, BIG_D, BIG_B, BIG_D, BIG_B],
        "only new game, load game and back are active at the first start"
    );
    assert_eq!(sound_after(&mut game, Key::Down), MOVE);
    assert_eq!(row_fonts(&game, START)[3], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Escape), BACK);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_A, BIG_D, BIG_B, BIG_B, BIG_B, BIG_B]
    );
}

#[test]
fn the_submenus_last_row_returns_and_starts_it_over_at_the_top() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Enter);
    step(&mut game, Key::Down);
    step(&mut game, Key::Down);
    assert_eq!(row_fonts(&game, START)[5], BIG_A);
    assert_eq!(sound_after(&mut game, Key::Space), CHOOSE);
    assert_eq!(selected(&game), 0, "back in the main menu");
    step(&mut game, Key::Enter);
    assert_eq!(row_fonts(&game, START)[0], BIG_A);
}

#[test]
fn rows_waiting_for_later_milestones_do_nothing_when_chosen() {
    // Configure and the Hall of Fame come with M2b.
    let mut game = in_menu(assets());
    step(&mut game, Key::Down);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_A, BIG_B, BIG_B, BIG_B]
    );
}

#[test]
fn the_exit_question_starts_on_no_and_escape_answers_it() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Escape);
    assert_eq!(sound_after(&mut game, Key::Enter), CHOOSE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(row_fonts(&game, MAIN)[0], BIG_D, "the main menu dims");
    assert_eq!(sound_after(&mut game, Key::Left), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_A, BIG_B));
    assert_eq!(sound_after(&mut game, Key::Y), SILENT, "already on yes");
    assert_eq!(sound_after(&mut game, Key::N), MOVE);
    assert_eq!((pixel(&game, YES), pixel(&game, NO)), (BIG_B, BIG_A));
    assert_eq!(sound_after(&mut game, Key::Escape), CHOOSE);
    assert_eq!(
        row_fonts(&game, MAIN),
        [BIG_B, BIG_D, BIG_B, BIG_B, BIG_B, BIG_A]
    );
    run(&mut game, 1_000);
    assert!(!game.quit_requested());
}

/// Answers the exit question with yes; returns with the key not yet read.
fn answer_yes(game: &mut Game) {
    step(game, Key::Escape);
    step(game, Key::Enter);
    step(game, Key::Left);
    press(game, Key::Enter);
}

#[test]
fn yes_shows_the_end_screen_until_a_key_and_then_the_game_quits() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    // A pass reads the key; the menu takes 26 ticks to black, the end screen 25 to 96 %.
    run(&mut game, 2 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), END);
    assert_eq!(colour(&game, END), [0, 0, 60]);
    run(&mut game, 300);
    assert!(!game.quit_requested());
    press(&mut game, Key::Space);
    // The hold reads the key at once; the fade-out takes 26 ticks.
    run(&mut game, 26);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
    assert_eq!(colour(&game, END), [0, 0, 0]);
}

#[test]
fn the_end_screen_stops_waiting_after_560_ticks() {
    let mut game = in_menu(assets());
    answer_yes(&mut game);
    run(&mut game, 2 + 26 + 25 + 560 + 25);
    assert!(!game.quit_requested());
    game.tick();
    assert!(game.quit_requested());
}

#[test]
fn the_music_fades_out_with_the_end_screen() {
    let mut with_music = assets();
    with_music.menu_music = music(true);
    let mut game = in_menu(with_music);
    answer_yes(&mut game);
    run(&mut game, 100);
    let loudest = |game: &mut Game| {
        let mut audio = Vec::new();
        game.take_audio(&mut audio);
        audio.iter().map(|&s| s.unsigned_abs()).max().unwrap()
    };
    let held = loudest(&mut game);
    assert!(held > 0);
    press(&mut game, Key::Space);
    run(&mut game, 14);
    let halfway = loudest(&mut game);
    assert!(halfway < held, "{halfway} vs {held}");
    run(&mut game, 13);
    assert!(game.quit_requested());
    // The music's next tick takes the last step to silence.
    run(&mut game, 2);
    loudest(&mut game);
    run(&mut game, 1);
    assert_eq!(loudest(&mut game), 0, "silent once the game has ended");
}

#[test]
fn the_credits_show_two_screens_each_until_a_key_and_return_to_the_menu() {
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    assert_eq!(selected(&game), 4);
    press(&mut game, Key::Enter);
    // A pass reads the key; the menu fades out in 51 ticks, the first screen in in 25.
    run(&mut game, 2 + 51 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0]);
    assert_eq!(colour(&game, CREDITS[0]), [60, 0, 0]);
    run(&mut game, 500);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "it waits for a key");
    press(&mut game, Key::Space);
    run(&mut game, 1 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
    press(&mut game, Key::Space);
    // Back to the menu as it was, fading in over 50 ticks.
    run(&mut game, 1 + 26 + 50);
    assert_eq!(selected(&game), 4);
    assert_eq!(colour(&game, BACKGROUND), [62, 62, 62]);
    step(&mut game, Key::Down);
    assert_eq!(selected(&game), 5, "the menu works again");
}

#[test]
fn a_key_during_a_credits_fade_in_moves_on_as_soon_as_it_is_done() {
    // The original reads the key before the screen's first wait, so an impatient player does
    // not see it held at all.
    let mut game = in_menu(assets());
    step(&mut game, Key::Up);
    step(&mut game, Key::Up);
    press(&mut game, Key::Enter);
    run(&mut game, 2 + 51 + 10);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[0], "fading in");
    press(&mut game, Key::Space);
    // The rest of the fade-in, the fade to black and the second screen's fade-in.
    run(&mut game, 15 + 26 + 25);
    assert_eq!(pixel(&game, (5, 5)), CREDITS[1]);
    assert_eq!(colour(&game, CREDITS[1]), [0, 60, 0]);
}
```

<!-- write: crates/core/tests/startup.rs -->
```rust
//! The startup sequence's timeline on synthetic assets (spec M1a §5.2). Each test pins down
//! something a player of the original would notice: a logo that holds too long, a fade that
//! ends at the wrong brightness, a key that does not skip.

mod common;

use deadrally_core::{AUDIO_CHANNELS, AUDIO_FRAMES_PER_TICK, Game, InputEvent, Key, PadButton};
use deadrally_gamedata::assets::{Assets, Picture};
use std::path::PathBuf;

use deadrally_gamedata::haf::{Animation, FRAME_PIXELS, HafFrame};
use deadrally_gamedata::image::{Image, Palette};
use deadrally_gamedata::s3m::{self, Cell, Module, Sample};
use deadrally_gamedata::xm::{Bank, Instrument, Looping};

/// Intro delays: frame 0 at tick 4, frame 1 at tick 6, the last frame at tick 9.
const DELAYS: [u8; 3] = [4, 2, 3];
const INTRO_END: u32 = 9;
/// Pixel values that tell the pictures apart; each picture's palette makes its own colour
/// full red, green or blue.
const APOGEE: u8 = 1;
const REMEDY: u8 = 2;
const TITLE: u8 = 3;
/// A logo takes 25 fade-in ticks, 180 hold ticks and 26 fade-out ticks.
const FADE_IN: u32 = 25;
const HOLD: u32 = 180;
const LOGO: u32 = FADE_IN + HOLD + 26;

fn palette(entries: &[(usize, [u8; 3])]) -> Palette {
    let mut palette = Palette::BLACK;
    for &(index, rgb) in entries {
        palette.0[index] = rgb;
    }
    palette
}

fn picture(pixel: u8, rgb: [u8; 3]) -> Picture {
    Picture {
        image: Image::new(4, 3, vec![pixel; 12]),
        palette: palette(&[(usize::from(pixel), rgb)]),
    }
}

/// Frame `k` is all pixel `16 + k`, coloured grey level `10 + k`.
fn intro_frame(k: u8) -> HafFrame {
    HafFrame {
        // Entries below 16 belong to the letterbox; the intro must not take them from frames.
        palette: palette(&[(0, [63, 63, 63]), (usize::from(16 + k), [10 + k; 3])]),
        pixels: vec![16 + k; FRAME_PIXELS],
    }
}

fn assets() -> Assets {
    let mut letterbox = vec![0u8; 320 * 200];
    letterbox[..320].fill(5);
    Assets {
        intro: Animation::from_frames(DELAYS.to_vec(), (0..3).map(intro_frame).collect()),
        letterbox: Picture {
            image: Image::new(320, 200, letterbox),
            // Entries from 16 on must stay black until the first frame sets them.
            palette: palette(&[(5, [20, 30, 40]), (16, [63, 63, 63])]),
        },
        apogee: picture(APOGEE, [63, 0, 0]),
        remedy: picture(REMEDY, [0, 63, 0]),
        // The main menu takes the title over as its 640x480 screen.
        title: Picture {
            image: Image::new(640, 480, vec![TITLE; 640 * 480]),
            ..picture(TITLE, [0, 0, 63])
        },
        intro_music: music(false),
        intro_effects: effects(),
        menu_music: music(false),
        menu: common::menu_assets(),
    }
}

/// A module that is silent, or plays one endless tone from its first row.
fn music(tone: bool) -> Module {
    let mut channels = [s3m::Channel::default(); s3m::CHANNELS];
    channels[0] = s3m::Channel {
        enabled: true,
        pan: 3,
    };
    let mut pattern = s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    };
    pattern.rows[0][0] = Cell {
        note: 0x40,
        instrument: 1,
        volume: Some(64),
        command: 0,
        info: 0,
    };
    Module {
        title: "Tone".into(),
        orders: if tone { vec![0] } else { Vec::new() },
        initial_speed: 6,
        initial_tempo: 125,
        global_volume: 64,
        master_volume: 48,
        stereo: false,
        channels,
        samples: vec![Sample {
            name: "Tone".into(),
            c2spd: 8363,
            volume: 64,
            looped: Some((0, 1000)),
            data: vec![4000; 1000],
        }],
        patterns: vec![pattern],
    }
}

/// Music that is silent from its first order and plays a tone from order 45, where the original
/// starts the menu music.
fn menu_music() -> Module {
    let mut module = music(true);
    module.patterns.push(s3m::Pattern {
        rows: vec![[Cell::default(); s3m::CHANNELS]; s3m::ROWS],
    });
    module.orders = [vec![1; 45], vec![0]].concat();
    module
}

/// Effect 1 is an endless constant tone.
fn effects() -> Bank {
    Bank {
        linear_frequencies: true,
        instruments: vec![Some(Instrument {
            name: "Hum".into(),
            data: vec![1000; 1000],
            looping: Looping::Forward {
                start: 0,
                length: 1000,
            },
            volume: 64,
            finetune: 0,
            relative_note: 0,
            panning: 128,
            fadeout: 0,
        })],
    }
}

/// The last left sample of each of the next `ticks` ticks.
fn loudness(game: &mut Game, ticks: u32) -> Vec<i16> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let mut audio = Vec::new();
            game.take_audio(&mut audio);
            audio[audio.len() - 2]
        })
        .collect()
}

fn press(game: &mut Game) {
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: true,
    });
    game.input(InputEvent::Key {
        key: Key::Space,
        pressed: false,
    });
}

fn run(game: &mut Game, ticks: u32) {
    for _ in 0..ticks {
        game.tick();
    }
}

/// Which picture is on screen and how bright its colour is (0..=63).
fn shown(game: &Game) -> (u8, u8) {
    let frame = game.frame();
    let pixel = frame.pixels[0];
    let brightness = frame.palette[usize::from(pixel)].into_iter().max().unwrap();
    (pixel, brightness)
}

/// The intro row shown at row 40 (the first animation row) and its colour.
fn intro_row(game: &Game) -> (u8, [u8; 3]) {
    let frame = game.frame();
    assert_eq!((frame.width, frame.height), (320, 200));
    let pixel = frame.pixels[40 * 320];
    (pixel, frame.palette[usize::from(pixel)])
}

#[test]
fn the_intro_starts_with_the_letterbox_and_black_animation_colours() {
    let game = Game::new(assets());
    let frame = game.frame();
    assert_eq!(
        (frame.width, frame.height, frame.aspect),
        (320, 200, (4, 3))
    );
    assert_eq!(frame.pixels[0], 5);
    assert_eq!(frame.palette[5], [20, 30, 40]);
    assert_eq!(frame.palette[16], [0, 0, 0]);
}

#[test]
fn each_intro_frame_appears_its_delay_after_the_previous_one() {
    // The intro is cut to its music (M1b); a frame early or late drifts out of sync.
    let mut game = Game::new(assets());
    run(&mut game, 3);
    assert_eq!(intro_row(&game), (0, [0, 0, 0]), "nothing before tick 4");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (16, [10, 10, 10]), "frame 0 at tick 4");
    assert_eq!(
        game.frame().palette[0],
        [0, 0, 0],
        "entries below 16 stay the letterbox's"
    );
    run(&mut game, 1);
    assert_eq!(intro_row(&game).0, 16, "frame 0 still at tick 5");
    run(&mut game, 1);
    assert_eq!(intro_row(&game), (17, [11, 11, 11]), "frame 1 at tick 6");
}

#[test]
fn the_last_intro_frame_is_never_shown() {
    // openAnimation blacks the palette as soon as the last frame is drawn, before it is shown.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END - 1);
    assert_eq!(intro_row(&game).0, 17);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn a_key_ends_the_intro_when_the_next_frame_is_due() {
    // The original checks for a key once per frame, so the intro runs on until the next frame
    // would have been shown.
    let mut game = Game::new(assets());
    run(&mut game, 1);
    press(&mut game);
    run(&mut game, 2);
    assert_eq!(intro_row(&game).0, 0, "still waiting for frame 0");
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0), "frame 0 is skipped too");
}

#[test]
fn a_corrupt_intro_frame_ends_the_intro_instead_of_crashing() {
    // Only data of an unknown version can hold one, and the player was warned at start-up;
    // the game should still reach its menus. The broken frame comes first: the last frame is
    // never decoded, so it could not show the problem.
    let mut record = vec![0u8; 768];
    record.extend([8, 2, 0xFF, 0xFF, 0, 0x3B]);
    let mut haf = vec![2, 0, 0, 0, 1, 1];
    for _ in 0..2 {
        haf.extend(u16::try_from(record.len()).unwrap().to_le_bytes());
        haf.extend(&record);
    }
    let mut broken = assets();
    broken.intro = Animation::from_bytes(PathBuf::from("BROKEN.HAF"), haf).unwrap();
    assert!(broken.intro.frame(0).is_err());
    let mut game = Game::new(broken);
    run(&mut game, 1);
    assert_eq!(shown(&game), (APOGEE, 0));
}

#[test]
fn an_empty_intro_goes_straight_to_the_logos() {
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    let mut game = Game::new(empty);
    assert_eq!(shown(&game), (APOGEE, 0));
    run(&mut game, FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
}

#[test]
fn an_empty_intro_still_starts_the_menu_music() {
    // The original starts the menu music after `checkAndOpenAnimation`, whether or not that
    // played anything.
    let mut empty = assets();
    empty.intro = Animation::from_frames(Vec::new(), Vec::new());
    empty.menu_music = menu_music();
    let mut game = Game::new(empty);
    assert!(loudness(&mut game, 3).iter().all(|&level| level > 0));
}

#[test]
fn a_pad_button_skips_like_a_key() {
    let mut game = Game::new(assets());
    game.input(InputEvent::PadButton {
        button: PadButton::A,
        pressed: true,
    });
    run(&mut game, 4);
    assert_eq!(shown(&game).0, APOGEE);
}

/// Red brightness of the Apogee logo for each tick after the intro.
fn apogee_brightness(game: &mut Game, ticks: u32) -> Vec<u8> {
    (0..ticks)
        .map(|_| {
            game.tick();
            let (pixel, brightness) = shown(game);
            assert_eq!(pixel, APOGEE);
            brightness
        })
        .collect()
}

#[test]
fn a_logo_fades_in_holds_and_fades_out_like_the_original() {
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END);
    let brightness = apogee_brightness(&mut game, LOGO - 1);
    // The fade-in climbs one 4 % step per tick from black and stops at 96 %: 63 shows as 60.
    assert_eq!(&brightness[..4], [0, 3, 5, 8]);
    assert_eq!(brightness[24], 60);
    // The hold lasts 180 ticks when nobody presses a key.
    assert!(brightness[25..205].iter().all(|&level| level == 60));
    // The fade-out starts at 100 %, a visible flash from 60 to 63, and steps down to black.
    assert_eq!(&brightness[205..208], [63, 60, 58]);
    assert_eq!(brightness[229], 3);
    // Its last, black step already has Remedy drawn under it: the same black screen.
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 0));
}

#[test]
fn a_key_during_the_fade_in_ends_the_hold_after_one_tick() {
    // The original remembers the press until the hold asks, so impatient players see the logo
    // at full fade for a single tick.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + 5);
    press(&mut game);
    let brightness = apogee_brightness(&mut game, FADE_IN - 5 + 1);
    assert_eq!(brightness.last(), Some(&60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63), "the fade-out starts");
}

#[test]
fn a_key_during_a_fade_out_ends_the_next_logos_hold_after_one_tick() {
    // The remembered press survives the change of screen, as in the original: a player who
    // presses while Apogee fades out sees the Remedy logo for a single hold tick.
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + FADE_IN + HOLD + 10);
    assert_eq!(shown(&game).0, APOGEE, "Apogee is fading out");
    press(&mut game);
    run(&mut game, LOGO - FADE_IN - HOLD - 10 + FADE_IN);
    assert_eq!(shown(&game), (REMEDY, 60), "Remedy's fade-in is done");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (REMEDY, 63), "the fade-out starts");
}

#[test]
fn a_key_with_the_last_intro_frame_carries_into_the_apogee_hold() {
    // openAnimation stops after its last frame without checking for a key, so the press waits
    // for the Apogee hold.
    let mut game = Game::new(assets());
    run(&mut game, 7);
    press(&mut game);
    run(&mut game, INTRO_END - 7 + FADE_IN);
    assert_eq!(shown(&game), (APOGEE, 60));
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 60), "one hold tick");
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn a_key_during_the_hold_starts_the_fade_out_on_the_next_tick() {
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + FADE_IN + 50);
    press(&mut game);
    game.tick();
    assert_eq!(
        shown(&game),
        (APOGEE, 60),
        "the hold tick that reads the key"
    );
    game.tick();
    assert_eq!(shown(&game), (APOGEE, 63));
}

#[test]
fn the_title_fades_in_after_both_logos_and_then_to_black_for_the_menu() {
    // Measured on the original: the title fades in to 92 % (63 as 58); loading the main menu
    // shows the pending 96 % step, and `transitionToBlack` then takes the title from 100 % to
    // black in 26 ticks. The original's load takes a moment, so it may skip a step or two of
    // the fade to black; here loading takes no time (spec M2a decision 5).
    let mut game = Game::new(assets());
    run(&mut game, INTRO_END + 2 * LOGO);
    assert_eq!(shown(&game), (TITLE, 0));
    run(&mut game, FADE_IN - 1);
    assert_eq!(shown(&game), (TITLE, 58));
    let brightness: Vec<u8> = (0..27)
        .map(|_| {
            game.tick();
            shown(&game).1
        })
        .collect();
    assert_eq!(&brightness[..4], [60, 63, 60, 58], "{brightness:?}");
    assert_eq!(brightness[25], 3, "{brightness:?}");
    assert_eq!(brightness[26], 0, "{brightness:?}");
}

#[test]
fn the_audio_stream_stays_full_when_nothing_plays() {
    // The frontend paces itself on the audio queue; missing samples would stall or drift it.
    let mut game = Game::new(assets());
    run(&mut game, 30);
    let mut audio = Vec::new();
    game.take_audio(&mut audio);
    assert_eq!(audio.len(), 30 * AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS);
    assert!(
        audio.iter().all(|&sample| sample == 0),
        "no music, no effects in these assets"
    );
}

#[test]
fn the_intro_plays_its_music_and_stops_it_when_it_ends() {
    // The original stops the song as the intro ends; music running on would play over the
    // logos.
    let mut with_music = assets();
    with_music.intro_music = music(true);
    let mut game = Game::new(with_music);
    let during = loudness(&mut game, INTRO_END - 1);
    assert!(during.iter().all(|&level| level > 0), "{during:?}");
    let after = loudness(&mut game, 5);
    assert_eq!(&after[1..], [0, 0, 0, 0], "{after:?}");
}

#[test]
fn the_menu_music_starts_when_the_intro_ends_and_plays_through_the_logos() {
    // `mainMenu` starts the menu music from order 45 right after the intro, before the logos;
    // they and the title are not silent.
    let mut with_menu = assets();
    with_menu.menu_music = menu_music();
    let mut game = Game::new(with_menu);
    let intro = loudness(&mut game, INTRO_END - 1);
    assert!(intro.iter().all(|&level| level == 0), "{intro:?}");
    let after = loudness(&mut game, LOGO + 10);
    assert!(after[1..].iter().all(|&level| level > 0), "{after:?}");
    assert_eq!(
        shown(&game).0,
        REMEDY,
        "the music goes on through the logos"
    );
}

#[test]
fn the_menu_music_plays_at_the_default_configurations_half_volume() {
    // The intro always plays at full volume: the original applies dr.cfg's volumes only when it
    // starts the menu music, and a fresh dr.cfg has the music at 50 % (FMOD master volume 63
    // against the intro's 127).
    let mut same = assets();
    let mut tone = music(true);
    tone.orders = vec![0; 46];
    same.intro_music = tone.clone();
    same.menu_music = tone;
    let mut game = Game::new(same);
    let intro = i32::from(loudness(&mut game, INTRO_END - 1)[2]);
    let menu = i32::from(loudness(&mut game, 5)[4]);
    assert!(intro > 0, "{intro}");
    assert!(
        (menu * 127 - intro * 63).abs() <= 127 * 2,
        "{menu} vs {intro}"
    );
}

#[test]
fn a_key_that_ends_the_intro_stops_its_sound() {
    // As in the original, the intro's music stops, and frames that were still due never start
    // their effects (the menu music, silent in these assets, takes over).
    let mut with_music = assets();
    with_music.intro_music = music(true);
    with_music.intro.effects = vec![1, 1, 1];
    let mut game = Game::new(with_music);
    run(&mut game, 1);
    press(&mut game);
    loudness(&mut game, 3);
    let after = loudness(&mut game, 10);
    assert!(after[1..].iter().all(|&level| level == 0), "{after:?}");
}

#[test]
fn a_frames_effect_sounds_when_the_frame_is_shown() {
    // Effects mark moments of the intro's picture; one early or late is out of sync with it.
    let mut timed = assets();
    timed.intro.effects = vec![0, 1, 0];
    let mut game = Game::new(timed);
    let levels = loudness(&mut game, 6);
    assert_eq!(&levels[..5], [0, 0, 0, 0, 0], "frame 1 appears at tick 6");
    assert!(levels[5] > 0);
}

#[test]
fn the_intros_effects_take_channels_one_to_six_in_turn() {
    // The seventh effect reuses channel 1 and cuts the first one off, so at most six effects
    // sound together, as in the original.
    let mut busy = assets();
    busy.intro = Animation::from_frames(vec![1; 10], (0..10).map(|k| intro_frame(k % 3)).collect());
    busy.intro.effects = vec![1; 10];
    let mut game = Game::new(busy);
    let levels = loudness(&mut game, 9);
    let one = i32::from(levels[0]);
    assert!(one > 0);
    for (voices, &level) in levels.iter().enumerate().take(6) {
        let expected = one * (voices as i32 + 1);
        assert!(
            (i32::from(level) - expected).abs() <= 6,
            "{voices}: {levels:?}"
        );
    }
    assert!(
        (i32::from(levels[7]) - 6 * one).abs() <= 6,
        "still six voices: {levels:?}"
    );
}
```

- [ ] **Step 22: Run them to see them fail**

Run: `cargo test -p deadrally-core --test menu --test startup`
Expected: FAIL to compile (`Game` has no `quit_requested`).

- [ ] **Step 23: Implement the scene**

`Menu` is a state machine whose states are the waits of the original's straight-line code: `tick` runs the code from one wait to the next, and the frame is the shown buffer and the palette at the wait. The startup ends when the title's last fade step is set (decision 5: loading takes no time) and hands its data, sound and remembered key to the menu; it now loads `MEN-SAM` and the default effects volume as the intro ends. The frontend leaves its loop when the game asks.

<!-- write: crates/core/src/menu/mod.rs -->
```rust
//! The main menu (spec M2a §3.2–§3.5), from the title's fade to black to the end screen.
//!
//! The original runs this as straight code with waits in it (`waitWithRefresh`, 0x43D870); the
//! screen shown during a tick is what the shown buffer and the palette hold when that tick's
//! wait starts. Here [`State`] names the wait the menu stands at, and [`Menu::tick`] runs the
//! code from it to the next one.

pub(crate) mod draw;
pub(crate) mod palette;

use deadrally_gamedata::assets::Assets;

use self::draw::{
    CURSOR_FRAMES, Focus, Graphics, MAIN_MENU, MenuTable, POPUP_FILL, Panel, START_MENU,
};
use self::palette::MenuPalette;
use crate::audio::{DEFAULT_EFFECTS_VOLUME, Sound};
use crate::canvas::{Canvas, HEIGHT, WIDTH, at};
use crate::keys::{self, Keys};
use crate::{AUDIO_FRAMES_PER_TICK, Frame};

/// The menus' sounds (`loadMenuSoundEffect`, 0x43C380): channel 1, at the configured effects
/// volume and pitch 0x28000.
const SOUND_CHANNEL: usize = 1;
const SOUND_PITCH: u32 = 0x2_8000;
const MOVE_SOUND: u8 = 25;
const BACK_SOUND: u8 = 22;
const CHOOSE_SOUND: u8 = 28;

/// The player's colour at the first start: driver 19's, 0 until a game sets it.
const PLAYER_COLOUR: usize = 0;
/// The main menu's rows: 0 start, 4 credits, 5 exit.
const START_ROW: usize = 0;
const CREDITS_ROW: usize = 4;
const EXIT_ROW: usize = 5;
/// The start submenu's last row returns to the main menu.
const START_MENU_BACK: usize = 5;
/// The exit question's popup and its yes/no at (x, y) = (180, 238).
const YES_NO_X: usize = 180;
const YES_NO_Y: usize = 238;
/// The end screen shows for at most 560 ticks; its fade-out lowers the music from 65500 in
/// steps of 2620.
const END_HOLD_TICKS: u32 = 560;
const END_VOLUME: u32 = 65_500;
const END_VOLUME_STEP: u32 = 2620;

/// Fades: 4 % a tick (`fadeIn` 0x427280 25 steps to 96 %, `transitionToBlack` 0x427300 26
/// steps from 100 % to 0), the menu's own fades 2 % a tick over 50 steps.
const FADE_IN_STEPS: u32 = 25;
const FADE_OUT_STEPS: u32 = 26;
const MENU_FADE_STEPS: u32 = 50;

/// The wait the menu stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// `transitionToBlack` on the title: wait `step` of 26.
    TitleToBlack {
        step: u32,
    },
    /// The menu's fade-in, wait `step` of 50.
    FadeIn {
        step: u32,
    },
    /// `readEventInMenu`: the first or second wait of a pass, in the main menu or the start
    /// submenu.
    Main {
        second: bool,
    },
    Start {
        second: bool,
    },
    /// `drawYesNoMenu` for the exit question; `yes` is the side selected.
    Exit {
        second: bool,
        yes: bool,
    },
    /// `showEndScreen`: the menu to black, `END.BMP` in, held, out with the music.
    EndToBlack {
        step: u32,
    },
    EndIn {
        step: u32,
    },
    EndHold {
        ticks: u32,
    },
    EndOut {
        step: u32,
    },
    /// The game has ended.
    Ended,
    /// `showCredits`: the menu out (50 down to 0), each credits screen in, held, out, the
    /// menu back in.
    CreditsOut {
        step: u32,
    },
    CreditsIn {
        screen: usize,
        step: u32,
    },
    CreditsHold {
        screen: usize,
    },
    CreditsToBlack {
        screen: usize,
        step: u32,
    },
    CreditsBack {
        step: u32,
    },
}

#[derive(Debug)]
pub(crate) struct Menu {
    assets: Assets,
    graphics: Graphics,
    /// The original's screen buffer, the shown buffer, and the credits' copy of the screen.
    screen: Canvas,
    shown: Canvas,
    saved: Canvas,
    palette: MenuPalette,
    keys: Keys,
    sound: Sound,
    audio: Vec<i16>,
    main: MenuTable,
    start: MenuTable,
    panel: Panel,
    /// The cursor's frame (0x45FBF8).
    cursor: usize,
    state: State,
}

impl Menu {
    /// Takes over from the startup when the title has faded in: the title is shown at its
    /// last fade step, `title_shown`, and the menu stands at `transitionToBlack`'s first wait.
    pub(crate) fn new(
        assets: Assets,
        sound: Sound,
        keys: Keys,
        audio: Vec<i16>,
        title_shown: &deadrally_gamedata::image::Palette,
    ) -> Menu {
        let menu_assets = &assets.menu;
        let colour = menu_assets.copper.0[PLAYER_COLOUR];
        let mut palette =
            MenuPalette::new(&menu_assets.palette, colour, &menu_assets.background_copper);
        palette.show(title_shown, 100);
        let graphics = Graphics::new(menu_assets);
        let panel = Panel::startup(&menu_assets.texts);
        let mut shown = Canvas::default();
        shown.copy_all(&assets.title.image);
        Menu {
            graphics,
            screen: Canvas::default(),
            shown,
            saved: Canvas::default(),
            palette,
            keys,
            sound,
            audio,
            main: MAIN_MENU,
            start: START_MENU,
            panel,
            cursor: 0,
            state: State::TitleToBlack { step: 0 },
            assets,
        }
    }

    pub(crate) fn input(&mut self, event: crate::InputEvent) {
        self.keys.event(event);
    }

    /// The player chose to exit and the end screen is over.
    pub(crate) fn quit_requested(&self) -> bool {
        self.state == State::Ended
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.state = self.run();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            pixels: self.shown.pixels(),
            palette: &self.palette.shown().0,
            aspect: (4, 3),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// The code from the wait at `self.state` to the next wait.
    fn run(&mut self) -> State {
        match self.state {
            State::TitleToBlack { step } => {
                let title = self.assets.title.palette.clone();
                self.palette.show(&title, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::TitleToBlack { step: step + 1 };
                }
                self.set_up();
                State::FadeIn { step: 0 }
            }
            State::FadeIn { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    return State::FadeIn { step: step + 1 };
                }
                self.shown = self.screen.clone();
                State::Main { second: false }
            }
            State::Main { second: false } => {
                self.palette.after_wait();
                State::Main { second: true }
            }
            State::Main { second: true } => {
                self.palette.after_wait();
                self.update_cursor_main();
                self.main_key()
            }
            State::Start { second: false } => {
                self.palette.after_wait();
                State::Start { second: true }
            }
            State::Start { second: true } => {
                self.palette.after_wait();
                self.graphics.update_cursor(
                    &mut self.screen,
                    &mut self.shown,
                    &self.start,
                    self.cursor,
                );
                self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
                self.start_key()
            }
            State::Exit { second: false, yes } => {
                self.palette.after_wait();
                State::Exit { second: true, yes }
            }
            State::Exit { second: true, yes } => {
                self.palette.after_wait();
                self.exit_key(yes)
            }
            State::EndToBlack { step } => {
                self.palette.fade(100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::EndToBlack { step: step + 1 };
                }
                self.screen.copy_all(&self.assets.menu.end.image);
                self.shown = self.screen.clone();
                State::EndIn { step: 0 }
            }
            State::EndIn { step } => {
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    State::EndIn { step: step + 1 }
                } else {
                    State::EndHold { ticks: 0 }
                }
            }
            State::EndHold { ticks } => {
                // `do { wait; i++ } while (!eventDetected() && i < 560)`.
                if self.keys.take() != 0 || ticks + 1 >= END_HOLD_TICKS {
                    State::EndOut { step: 0 }
                } else {
                    State::EndHold { ticks: ticks + 1 }
                }
            }
            State::EndOut { step } => {
                self.sound
                    .set_mask((END_VOLUME - END_VOLUME_STEP * step) >> 8);
                let end = self.assets.menu.end.palette.clone();
                self.palette.show(&end, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    State::EndOut { step: step + 1 }
                } else {
                    State::Ended
                }
            }
            State::Ended => State::Ended,
            State::CreditsOut { step } => {
                // `for (e = 50; e >= 0; e--)`: here `step` counts e down.
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step > 0 {
                    return State::CreditsOut { step: step - 1 };
                }
                self.show_credits(0);
                State::CreditsIn { screen: 0, step: 0 }
            }
            State::CreditsIn { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 4 * i64::from(step));
                if step + 1 < FADE_IN_STEPS {
                    return State::CreditsIn {
                        screen,
                        step: step + 1,
                    };
                }
                // A key is checked before the first wait: one pressed during the fade-in moves
                // on at once.
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsHold { screen } => {
                if self.keys.take() != 0 {
                    State::CreditsToBlack { screen, step: 0 }
                } else {
                    State::CreditsHold { screen }
                }
            }
            State::CreditsToBlack { screen, step } => {
                let palette = self.assets.menu.credits[screen].palette.clone();
                self.palette.show(&palette, 100 - 4 * i64::from(step));
                if step + 1 < FADE_OUT_STEPS {
                    return State::CreditsToBlack {
                        screen,
                        step: step + 1,
                    };
                }
                if screen == 0 {
                    self.show_credits(1);
                    return State::CreditsIn { screen: 1, step: 0 };
                }
                self.palette.compose();
                self.screen = self.saved.clone();
                self.shown = self.screen.clone();
                State::CreditsBack { step: 0 }
            }
            State::CreditsBack { step } => {
                if step % 2 == 1 {
                    self.update_cursor_main();
                }
                self.palette.fade(2 * i64::from(step));
                if step + 1 < MENU_FADE_STEPS {
                    State::CreditsBack { step: step + 1 }
                } else {
                    self.main_pass()
                }
            }
        }
    }

    /// `mainMenu` after the title: the background, the bottom panel, the main menu, the
    /// palette composed; the menu's fade-in follows.
    fn set_up(&mut self) {
        self.screen.copy_all(&self.graphics.background);
        self.graphics
            .panel_frame(&mut self.screen, 0, 371, 639, 109);
        self.graphics.panel_text(&mut self.screen, &self.panel);
        self.draw_main();
        self.shown = self.screen.clone();
        self.palette.compose();
    }

    /// The top of `mainMenu`'s loop: rows 84..=366 restored, the main menu drawn with focus.
    fn draw_main(&mut self) {
        self.screen.copy_rows(&self.graphics.background, 84, 283);
        self.main.active[1] = false;
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Focused, self.cursor);
    }

    /// A pass of `mainMenu`'s loop without the fade: drawn, shown, waiting for a key.
    fn main_pass(&mut self) -> State {
        self.draw_main();
        self.shown = self.screen.clone();
        State::Main { second: false }
    }

    fn update_cursor_main(&mut self) {
        self.graphics
            .update_cursor(&mut self.screen, &mut self.shown, &self.main, self.cursor);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
    }

    fn sound(&mut self, effect: u8) {
        self.sound
            .trigger_at(SOUND_CHANNEL, effect, DEFAULT_EFFECTS_VOLUME, SOUND_PITCH);
    }

    /// Moves the highlight of `menu` (the main menu when `main`): `Some(row)`, or Up/Down.
    fn move_highlight(&mut self, main: bool, key: u8) {
        let menu = if main {
            &mut self.main
        } else {
            &mut self.start
        };
        let (to, base) = match key {
            keys::UP | keys::PAD_UP => {
                let mut row = menu.selected;
                loop {
                    row = if row == 0 { menu.rows - 1 } else { row - 1 };
                    if menu.active[row] {
                        break (row, 6);
                    }
                }
            }
            keys::DOWN | keys::PAD_DOWN => {
                let mut row = menu.selected;
                loop {
                    row = if row + 1 >= menu.rows { 0 } else { row + 1 };
                    if menu.active[row] {
                        break (row, 5);
                    }
                }
            }
            _ => (menu.rows - 1, 6),
        };
        self.graphics.move_highlight(
            &mut self.screen,
            &mut self.shown,
            menu,
            to,
            base,
            self.cursor,
        );
    }

    /// The key read at the end of a main menu pass.
    fn main_key(&mut self) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                if self.main.selected != self.main.rows - 1 {
                    self.move_highlight(true, keys::ESCAPE);
                    self.sound(MOVE_SOUND);
                }
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                return self.choose(self.main.selected);
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(true, key);
                self.sound(MOVE_SOUND);
            }
            _ => {}
        }
        State::Main { second: false }
    }

    /// What a main menu row does.
    fn choose(&mut self, row: usize) -> State {
        match row {
            START_ROW => self.start_pass(),
            CREDITS_ROW => {
                self.saved = self.screen.clone();
                self.palette.compose();
                State::CreditsOut {
                    step: MENU_FADE_STEPS,
                }
            }
            EXIT_ROW => self.ask_exit(),
            // Configure and the Hall of Fame come with M2b.
            _ => self.main_pass(),
        }
    }

    /// A pass of `startRacingMenu`'s loop: the main menu dimmed, the submenu with focus.
    fn start_pass(&mut self) -> State {
        self.screen.copy_rows(&self.graphics.background, 92, 275);
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics
            .menu(&mut self.screen, &self.start, Focus::Focused, self.cursor);
        self.shown = self.screen.clone();
        State::Start { second: false }
    }

    fn start_key(&mut self) -> State {
        match self.keys.take() {
            keys::ESCAPE => {
                self.sound(BACK_SOUND);
                self.main_pass()
            }
            keys::ENTER | keys::SPACE | 0x9C => {
                self.sound(CHOOSE_SOUND);
                if self.start.selected == START_MENU_BACK {
                    self.start.selected = 0;
                    return self.main_pass();
                }
                // New game and loading wait for M3.
                self.start_pass()
            }
            key @ (keys::UP | keys::PAD_UP | keys::DOWN | keys::PAD_DOWN) => {
                self.move_highlight(false, key);
                self.sound(MOVE_SOUND);
                State::Start { second: false }
            }
            _ => State::Start { second: false },
        }
    }

    /// `mainMenu`'s exit question: the main menu dimmed over itself, the question's popup,
    /// "no" selected.
    fn ask_exit(&mut self) -> State {
        self.graphics
            .menu(&mut self.screen, &self.main, Focus::Unfocused, self.cursor);
        self.graphics
            .popup(&mut self.screen, 170, 200, 300, 80, Focus::Focused);
        let question = &self.assets.menu.texts.exit_question;
        self.graphics.small[0].draw(&mut self.screen, question, at(253, 208));
        self.draw_yes_no(false);
        self.shown = self.screen.clone();
        State::Exit {
            second: false,
            yes: false,
        }
    }

    /// The two answers, the selected one in big A.
    fn draw_yes_no(&mut self, yes: bool) {
        let texts = &self.assets.menu.texts;
        let (yes_font, no_font) = if yes {
            (&self.graphics.big_a, &self.graphics.big_b)
        } else {
            (&self.graphics.big_b, &self.graphics.big_a)
        };
        yes_font.draw(
            &mut self.screen,
            &texts.yes,
            at(YES_NO_X + 30, YES_NO_Y - 7),
        );
        no_font.draw(
            &mut self.screen,
            &texts.no,
            at(YES_NO_X + 200, YES_NO_Y - 7),
        );
    }

    fn exit_key(&mut self, yes: bool) -> State {
        let cursor_x = if yes { YES_NO_X + 7 } else { YES_NO_X + 177 };
        let cursor_at = at(cursor_x, YES_NO_Y);
        self.screen.fill(cursor_at, 20, 20, POPUP_FILL);
        let cursor = self.graphics.cursor(self.cursor).clone();
        self.screen.draw(&cursor, cursor_at, true);
        self.shown
            .copy_from(&self.screen, at(YES_NO_X + 2, YES_NO_Y), 240, 28);
        self.cursor = (self.cursor + 1) % CURSOR_FRAMES;
        let key = match self.keys.take() {
            keys::Y => keys::PAD_LEFT,
            keys::N => keys::PAD_RIGHT,
            key => key,
        };
        let answer = match key {
            keys::LEFT | keys::PAD_LEFT | keys::RIGHT | keys::PAD_RIGHT => {
                let left = matches!(key, keys::LEFT | keys::PAD_LEFT);
                if left != yes {
                    self.sound(MOVE_SOUND);
                }
                self.screen
                    .fill(at(YES_NO_X + 2, YES_NO_Y), 240, 25, POPUP_FILL);
                self.draw_yes_no(left);
                return State::Exit {
                    second: false,
                    yes: left,
                };
            }
            keys::ESCAPE => false,
            keys::ENTER | 0x9C => yes,
            _ => {
                return State::Exit { second: false, yes };
            }
        };
        self.sound(CHOOSE_SOUND);
        if answer {
            self.palette.compose();
            State::EndToBlack { step: 0 }
        } else {
            self.main_pass()
        }
    }

    /// Credits screen `screen` drawn and shown, its palette black.
    fn show_credits(&mut self, screen: usize) {
        self.screen
            .copy_all(&self.assets.menu.credits[screen].image);
        self.shown = self.screen.clone();
    }
}
```

<!-- write: crates/core/src/startup.rs -->
```rust
//! The original's startup sequence: the intro, the Apogee and Remedy logos and the title screen
//! (spec M1a §3.6 and §5.2), with the intro's music and effects and then the menu music
//! (spec M1b §4.3).
//!
//! Each step follows the Windows version's loops tick for tick, so a screenshot of the original
//! can be found in our timeline: `openAnimation` (0x4185B0), `apogeeScreen` (0x427380) and
//! `showStartScreen` (0x427880).

use deadrally_gamedata::assets::{Assets, Picture};
use deadrally_gamedata::haf::FRAME_PIXELS;
use deadrally_gamedata::image::Palette;

use crate::audio::{DEFAULT_EFFECTS_VOLUME, DEFAULT_MUSIC_VOLUME, FULL_VOLUME, Sound};
use crate::fade::{FADE_FULL, FADE_STEP, fade};
use crate::keys::Keys;
use crate::menu::Menu;
use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent};

/// The intro's screen: 320x200, with the animation's 320x120 frames from row 40.
const INTRO_WIDTH: u32 = 320;
const INTRO_HEIGHT: u32 = 200;
const INTRO_FIRST_ROW: usize = 40;
/// The letterbox owns palette entries 0..=15, the animation frames the rest.
const LETTERBOX_COLOURS: usize = 16;
/// The intro's effects take channels 1..=6 in turn.
const INTRO_EFFECT_CHANNELS: usize = 6;
/// The menu music starts at this order (`musicSetOrder(0x2D00)` in `mainMenu`, 0x43A0C5).
const MENU_MUSIC_ORDER: usize = 45;

/// Fade-in ticks: brightness 0, 4, ..., 96 %. The original's loop stops before 100 %.
const FADE_IN_TICKS: u32 = 25;
/// Fade-out ticks: brightness 100, 96, ..., 0 %.
const FADE_OUT_TICKS: u32 = 26;
/// Longest hold of a logo, in ticks.
const HOLD_TICKS: u32 = 180;

/// A logo or the title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Apogee,
    Remedy,
    Title,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// `next` is the frame being waited for; `waited` counts ticks since the previous frame.
    Intro {
        next: usize,
        waited: u32,
    },
    FadeIn {
        screen: Screen,
        ticks: u32,
    },
    Hold {
        screen: Screen,
        ticks: u32,
    },
    FadeOut {
        screen: Screen,
        ticks: u32,
    },
    /// The title has faded in; the main menu takes over (`mainMenu` goes on to load it).
    Done,
}

#[derive(Debug)]
pub(crate) struct Startup {
    assets: Assets,
    stage: Stage,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    palette: Palette,
    /// The original keeps the last key press until something asks for it (`eventDetected`,
    /// 0x417EB0, reads and clears it), so a press during a fade-in ends the following hold
    /// after one tick.
    keys: Keys,
    sound: Sound,
    /// The channel the intro's next effect plays on.
    effect_channel: usize,
    /// Samples rendered since the last `take_audio`.
    audio: Vec<i16>,
}

impl Startup {
    pub(crate) fn new(assets: Assets) -> Startup {
        let mut startup = Startup {
            assets,
            stage: Stage::Intro { next: 0, waited: 0 },
            width: 0,
            height: 0,
            pixels: Vec::new(),
            palette: Palette::BLACK,
            keys: Keys::default(),
            sound: Sound::default(),
            effect_channel: 1,
            audio: Vec::new(),
        };
        if startup.assets.intro.is_empty() {
            // `openAnimation` plays nothing when the file has no frames.
            startup.stage = startup.end_intro();
        } else {
            startup.show_letterbox();
            // `openAnimation` loads the music and the effects and starts the music just before
            // the first frame, at full volume: `dr.cfg`'s volumes apply only after the intro.
            startup
                .sound
                .play_music(&startup.assets.intro_music, 0, FULL_VOLUME);
            startup.sound.load_effects(&startup.assets.intro_effects);
        }
        startup
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        self.keys.event(event);
    }

    /// The title has faded in and the main menu should take over.
    pub(crate) fn finished(&self) -> bool {
        self.stage == Stage::Done
    }

    /// The main menu, taking over the data, the sound and the remembered key.
    pub(crate) fn into_menu(self) -> Menu {
        Menu::new(
            self.assets,
            self.sound,
            self.keys,
            self.audio,
            &self.palette,
        )
    }

    pub(crate) fn tick(&mut self) {
        self.keys.tick();
        self.stage = self.next_stage();
        self.sound.render(AUDIO_FRAMES_PER_TICK, &mut self.audio);
    }

    fn next_stage(&mut self) -> Stage {
        match self.stage {
            Stage::Intro { next, waited } => self.tick_intro(next, waited + 1),
            // After the title's last step the original loads the main menu without presenting
            // a frame; the menu's first wait shows this step (spec M2a decision 5: loading takes
            // no time here).
            Stage::FadeIn {
                screen: Screen::Title,
                ticks,
            } if ticks + 1 == FADE_IN_TICKS => {
                self.palette = fade(&self.assets.title.palette, i64::from(ticks) * FADE_STEP);
                Stage::Done
            }
            Stage::FadeIn { screen, ticks } => {
                let level = i64::from(ticks) * FADE_STEP;
                self.palette = fade(&picture(&self.assets, screen).palette, level);
                if ticks + 1 < FADE_IN_TICKS {
                    Stage::FadeIn {
                        screen,
                        ticks: ticks + 1,
                    }
                } else {
                    Stage::Hold { screen, ticks: 0 }
                }
            }
            Stage::Hold { screen, ticks } => {
                // `do { wait } while (!eventDetected() && ticks < 180)`: the key is read first,
                // so even the last hold tick consumes a pending press.
                if self.keys.take() != 0 || ticks + 1 >= HOLD_TICKS {
                    Stage::FadeOut { screen, ticks: 0 }
                } else {
                    Stage::Hold {
                        screen,
                        ticks: ticks + 1,
                    }
                }
            }
            Stage::FadeOut { screen, ticks } => {
                let level = FADE_FULL - i64::from(ticks) * FADE_STEP;
                self.palette = fade(&picture(&self.assets, screen).palette, level);
                if ticks + 1 < FADE_OUT_TICKS {
                    Stage::FadeOut {
                        screen,
                        ticks: ticks + 1,
                    }
                } else {
                    self.show(match screen {
                        Screen::Apogee => Screen::Remedy,
                        Screen::Remedy | Screen::Title => Screen::Title,
                    })
                }
            }
            Stage::Done => Stage::Done,
        }
    }

    /// One tick of `openAnimation`: when frame `next` is due it replaces the previous one, then
    /// the original checks for a key before showing it. So a key press ends the intro at the
    /// next frame, which is never shown, and the last frame is never shown either.
    ///
    /// The original also checks once before frame 0. A press made while the game loads is read
    /// only when a frame is next shown (`refreshScreen`, 0x43B580), that is during frame 0's
    /// wait, so it ends the intro when frame 0 is due, as here.
    fn tick_intro(&mut self, mut next: usize, mut waited: u32) -> Stage {
        let intro = &self.assets.intro;
        let mut due = None;
        while waited >= u32::from(intro.delays[next]) {
            due = Some(next);
            next += 1;
            waited = 0;
            if next == intro.len() || self.keys.take() != 0 {
                // The frame ending the intro is never shown, and its effect, which the
                // original starts and cuts at once, never sounds.
                return self.end_intro();
            }
            // The original triggers a frame's effect right after drawing it.
            let effect = intro.effects[next - 1];
            if effect != 0 {
                self.sound.trigger(self.effect_channel, effect);
                self.effect_channel = self.effect_channel % INTRO_EFFECT_CHANNELS + 1;
            }
        }
        if let Some(index) = due {
            match intro.frame(index) {
                Ok(frame) => {
                    self.palette.0[LETTERBOX_COLOURS..]
                        .copy_from_slice(&frame.palette.0[LETTERBOX_COLOURS..]);
                    let start = INTRO_FIRST_ROW * INTRO_WIDTH as usize;
                    self.pixels[start..start + FRAME_PIXELS].copy_from_slice(&frame.pixels);
                }
                // Only data of an unknown version can get here (the known version's frames are
                // all tested), and the player was warned about it at start-up.
                Err(_) => return self.end_intro(),
            }
        }
        Stage::Intro { next, waited }
    }

    /// The intro's sound stops (`openAnimation`, `checkAndOpenAnimation`); `mainMenu` then
    /// starts the menu music at the configured volume and shows the logos.
    fn end_intro(&mut self) -> Stage {
        self.sound.stop();
        self.sound.play_music(
            &self.assets.menu_music,
            MENU_MUSIC_ORDER,
            DEFAULT_MUSIC_VOLUME,
        );
        self.sound.load_effects(&self.assets.menu.effects);
        self.sound.set_effects_volume(DEFAULT_EFFECTS_VOLUME);
        self.show(Screen::Apogee)
    }

    /// Black screen with the letterbox's colours set, as `openAnimation` starts.
    fn show_letterbox(&mut self) {
        let letterbox = &self.assets.letterbox;
        assert_eq!(
            (letterbox.image.width, letterbox.image.height),
            (INTRO_WIDTH, INTRO_HEIGHT),
            "the intro letterbox is 320x200"
        );
        self.pixels.clear();
        self.pixels.extend_from_slice(&letterbox.image.pixels);
        (self.width, self.height) = (INTRO_WIDTH, INTRO_HEIGHT);
        self.palette = Palette::BLACK;
        self.palette.0[..LETTERBOX_COLOURS]
            .copy_from_slice(&letterbox.palette.0[..LETTERBOX_COLOURS]);
    }

    /// The picture drawn under a black palette, ready to fade in.
    #[must_use]
    fn show(&mut self, screen: Screen) -> Stage {
        let image = &picture(&self.assets, screen).image;
        self.pixels.clear();
        self.pixels.extend_from_slice(&image.pixels);
        (self.width, self.height) = (image.width, image.height);
        self.palette = Palette::BLACK;
        Stage::FadeIn { screen, ticks: 0 }
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: self.width,
            height: self.height,
            pixels: &self.pixels,
            palette: &self.palette.0,
            aspect: (4, 3),
        }
    }

    /// The startup's sound: the intro's music and effects, then the menu music.
    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }
}

fn picture(assets: &Assets, screen: Screen) -> &Picture {
    match screen {
        Screen::Apogee => &assets.apogee,
        Screen::Remedy => &assets.remedy,
        Screen::Title => &assets.title,
    }
}
```

<!-- write: crates/core/src/game.rs -->
```rust
use deadrally_gamedata::assets::Assets;

use crate::menu::Menu;
use crate::startup::Startup;
use crate::test_scene::TestScene;
use crate::{Frame, InputEvent};

/// The whole game state: the original's startup sequence and then its main menu, or the M0
/// test scene.
#[derive(Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Debug)]
enum Scene {
    Test(Box<TestScene>),
    Startup(Box<Startup>),
    Menu(Box<Menu>),
    /// Only while one scene hands over to the next.
    Handover,
}

impl Game {
    /// Starts the original's startup sequence (intro, Apogee, Remedy, title), then the main
    /// menu.
    ///
    /// # Panics
    ///
    /// If the intro letterbox is not 320x200 ([`Assets::load`] guarantees it is).
    #[must_use]
    pub fn new(assets: Assets) -> Game {
        Game {
            scene: Scene::Startup(Box::new(Startup::new(assets))),
        }
    }

    /// The M0 test scene, which needs no game data: `-testscene`, headless runs and CI.
    #[must_use]
    pub fn test_scene() -> Game {
        Game {
            scene: Scene::Test(Box::new(TestScene::new())),
        }
    }

    pub fn input(&mut self, event: InputEvent) {
        match &mut self.scene {
            Scene::Test(scene) => scene.input(event),
            Scene::Startup(scene) => scene.input(event),
            Scene::Menu(scene) => scene.input(event),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    /// Advances the simulation by exactly one 14 ms tick.
    pub fn tick(&mut self) {
        match &mut self.scene {
            Scene::Test(scene) => scene.tick(),
            Scene::Startup(scene) => scene.tick(),
            Scene::Menu(scene) => scene.tick(),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
        if matches!(&self.scene, Scene::Startup(startup) if startup.finished())
            && let Scene::Startup(startup) = std::mem::replace(&mut self.scene, Scene::Handover)
        {
            self.scene = Scene::Menu(Box::new(startup.into_menu()));
        }
    }

    /// The player chose to exit the game and its end screen is over: the frontend should close.
    #[must_use]
    pub fn quit_requested(&self) -> bool {
        matches!(&self.scene, Scene::Menu(menu) if menu.quit_requested())
    }

    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        match &self.scene {
            Scene::Test(scene) => scene.frame(),
            Scene::Startup(scene) => scene.frame(),
            Scene::Menu(scene) => scene.frame(),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    /// Appends the interleaved stereo samples produced since the last call
    /// (`AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS` per tick).
    pub fn take_audio(&mut self, out: &mut Vec<i16>) {
        match &mut self.scene {
            Scene::Test(scene) => scene.take_audio(out),
            Scene::Startup(scene) => scene.take_audio(out),
            Scene::Menu(scene) => scene.take_audio(out),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }
}
```

<!-- write: crates/deadrally/src/main.rs -->
```rust
//! DeadRally: the game's frontend (see docs/adr/0001-platform-layer.md). It plays the
//! original's startup sequence on SDL3, silently until M1b.
//!
//! Options: `-window` starts windowed (default: borderless fullscreen at the desktop
//! resolution); `-novsync` turns vsync off, for measuring present cost; `-testscene` runs the
//! M0 test scene, which needs no game data; `--data <dir>` names the game data directory (else
//! `DEADRALLY_DATA`, else `data_path` in the config file). Alt+Enter toggles fullscreen, F12
//! toggles bilinear smoothing, closing the window quits. One stats line per second goes to
//! stdout.

mod keymap;

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use deadrally_core::host::{AudioGate, Pacer, RunStats, letterbox};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE, Game, InputEvent, PadAxis};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::{DATA_ENV_VAR, LocateError, Outcome, config_path, locate};
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FRect, ScaleMode};
use sdl3::video::FullscreenType;

const BYTES_PER_SAMPLE: usize = 2;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    windowed: bool,
    vsync: bool,
    test_scene: bool,
    data: Option<PathBuf>,
}

fn parse_options(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options {
        windowed: false,
        vsync: true,
        test_scene: false,
        data: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-window") => options.windowed = true,
            Some("-novsync") => options.vsync = false,
            Some("-testscene") => options.test_scene = true,
            Some("--data") => {
                options.data = Some(PathBuf::from(
                    args.next().ok_or("--data needs a directory")?,
                ));
            }
            _ => {
                return Err(format!(
                    "unknown option {}; known: -window, -novsync, -testscene, --data <dir>",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(options)
}

/// The startup sequence on the player's data, plus a warning to show when the data is not a
/// known release.
fn load_game(data: Option<&Path>) -> Result<(Game, Option<String>), String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let hint = || {
        let file = config.as_deref().map_or_else(
            || "the config file".to_owned(),
            |path| path.display().to_string(),
        );
        format!(
            "Point DeadRally at your copy of Death Rally (the folder that holds MENU.BPA) with \
             --data <dir>, the {DATA_ENV_VAR} environment variable, or data_path in {file}."
        )
    };
    let located = locate(data, env.as_deref(), config.as_deref()).map_err(|error| match error {
        // This one already names all three ways.
        LocateError::NotSpecified { .. } => error.to_string(),
        _ => format!("{error}\n\n{}", hint()),
    })?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    let dir = &located.validation.dir;
    let warning = match &located.validation.outcome {
        Outcome::Known { .. } => None,
        Outcome::Unknown { closest, differing } => Some(format!(
            "The game data in {} is not a release DeadRally knows (closest: {closest}; \
             different: {}). The game starts anyway, but it may not match the original.",
            dir.display(),
            differing.join(", ")
        )),
    };
    let assets = Assets::load(&located.validation).map_err(|error| {
        format!(
            "cannot read the game data in {}: {error}\n\n{}",
            dir.display(),
            hint()
        )
    })?;
    Ok((Game::new(assets), warning))
}

/// Shows `message` in a dialog as well as on stderr; the dialog is best effort (there may be
/// no display at all).
fn tell(flag: MessageBoxFlag, title: &str, message: &str) {
    eprintln!("{}: {message}", title.to_lowercase());
    let _ = show_simple_message_box(flag, &format!("DeadRally: {title}"), message, None);
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options(std::env::args_os().skip(1))?;
    let mut game = if options.test_scene {
        Game::test_scene()
    } else {
        match load_game(options.data.as_deref()) {
            Ok((game, warning)) => {
                if let Some(warning) = warning {
                    tell(MessageBoxFlag::WARNING, "Warning", &warning);
                }
                game
            }
            Err(message) => {
                tell(MessageBoxFlag::ERROR, "Error", &message);
                std::process::exit(1);
            }
        }
    };
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

    let mut texture_size = (0, 0);
    let mut texture = None;
    let mut rgba = Vec::new();
    let mut samples = Vec::new();
    let mut outgoing = Vec::new();
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
                    // Alt's own press still reaches the game before this, so Alt+Enter skips
                    // the intro or a logo. The original does the same: refreshScreen (0x43B580)
                    // remembers every key press except F12 and Enter with Alt.
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
            outgoing.clear();
            gate.feed(queued_frames, &samples, &mut outgoing);
            if !outgoing.is_empty() {
                stream.put_data_i16(&outgoing)?;
            }
        }
        stats.add_ticks(ticks);
        if game.quit_requested() {
            break 'running;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(list: &[&str]) -> Result<Options, String> {
        parse_options(list.iter().map(OsString::from))
    }

    #[test]
    fn options_select_the_scene_and_the_data() {
        let options = parse(&["-window", "-testscene", "--data", "/games/dr"]).unwrap();
        assert!(options.windowed && options.test_scene && options.vsync);
        assert_eq!(options.data, Some(PathBuf::from("/games/dr")));
        assert_eq!(parse(&[]).unwrap().data, None);
    }

    #[test]
    fn unusable_data_says_how_to_point_at_other_data() {
        // A player whose copy is incomplete or damaged must learn how to choose another one.
        let empty = tempfile::tempdir().unwrap();
        let message = load_game(Some(empty.path())).expect_err("an empty folder is no game data");
        for needle in ["--data", "DEADRALLY_DATA", "data_path"] {
            assert!(message.contains(needle), "{needle} missing in: {message}");
        }
    }

    #[test]
    fn unknown_or_incomplete_options_are_errors() {
        // A typo such as -testcsene must not silently start the real game instead.
        assert!(parse(&["-testcsene"]).unwrap_err().contains("-testscene"));
        assert!(parse(&["--data"]).is_err());
    }
}
```

- [ ] **Step 24: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (266 passed, 16 ignored), no warnings.

Check that the menu tests can fail, one mutation at a time, each undone after (`git checkout crates/core/src/menu`):
- `END_HOLD_TICKS` 560 → 500: `the_end_screen_stops_waiting_after_560_ticks` fails.
- In `main_key`, `if self.main.selected != self.main.rows - 1` → `if true`: `escape_jumps_to_exit_once` fails.
- In `CreditsIn`, drop the key check after the last step (go to `CreditsHold` always): `a_key_during_a_credits_fade_in_moves_on_as_soon_as_it_is_done` fails.
- In `Panel::startup`, drop the empty line: `the_bottom_panel_shows_the_start_up_lines_with_a_gap_before_the_last` fails.

- [ ] **Step 25: Run the data tests**

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: FAIL in `rendered_sound_matches_the_committed_manifest` only: the startup line `startup 6446 ticks` is now `ad0ce99384b328909cbbab9052e86d4677ccaf8963d08f8e2933886663ba2bac`. The rest pass.

- [ ] **Step 26: Rewrite the manifest's startup line**

The startup's sound changed in 276 samples at the intro's last tick (3 ms at 80.234 s): the intro's effects still fading as it ends now fade at the menu's effects volume, which the original sets there (spec 3.5). To confirm, render the startup with this tree and with `master` (`git stash` is not needed: build `master` in a second worktree) and compare sample by sample; the first difference is at sample 7702464 (tick 5731), the last 276 samples later. Then:

<!-- write: crates/headless/tests/rendered-audio.sha256 -->
```text
ad0ce99384b328909cbbab9052e86d4677ccaf8963d08f8e2933886663ba2bac  startup 6446 ticks
04be087277f0c8f8a76899abb164b30b71341c5238ad40aee594151d648ba8fa  MUSICS.BPA/MEN-MUS.CMF first 30 s
399ebd9f5d8f0b8c4a8914297fa88e459cc3ebcadccfcbf5346f300c4c5ef7a8  MUSICS.BPA/TR0-MUS.CMF first 30 s
78a2791785dc8413c7b5fe932881e799dd12fdbbf5cdbc26c848b11c8ff453c0  MUSICS.BPA/TR1-MUS.CMF first 30 s
3453e9e904f6a899c8f1c2a7d860a8a5ea394b4f0d57f16a450e0e206f6d440e  MUSICS.BPA/TR2-MUS.CMF first 30 s
b98fcbfe66faf4b381a3d4167b612af821762504a8e89349cc2c08a8eda0962d  MUSICS.BPA/TR3-MUS.CMF first 30 s
1e845e839dacb800c52240b29284e9ab5fe739c55eff8caeb5925035e611a2cd  MUSICS.BPA/TR4-MUS.CMF first 30 s
2b858b5e88e245bb31be7feb885f23ee8b62383c783d3404715795f87ae35313  MUSICS.BPA/TR5-MUS.CMF first 30 s
b98310016f3de3888b73758b89b91fbf9e3efa8a0617d66f55765115bd094e41  MUSICS.BPA/TR6-MUS.CMF first 30 s
df59238aa423f24847b0a97b14bf3de336f4d879277b7156fa7d2339857cf3b5  MUSICS.BPA/TR7-MUS.CMF first 30 s
67cadb8d31dc0e465a3f3523062a3fc5c7c3632251e9ba7d23a4822e0ddb83f8  MUSICS.BPA/TR8-MUS.CMF first 30 s
07e2d11a647504083664c71b222b9bfb8728db92e0c0a5c9a85b43404ac11682  MUSICS.BPA/TR9-MUS.CMF first 30 s
e500fd900c35dac0cbce0413bc5ba4b8320bfea545fb6f7410eb5534a64f4996  MUSICS.BPA/SANIM-E.CMF every effect, 2 s each
9b9a73e43f1e56eff9f60f90d3ef4b21bae6999406d266fcecdbb5761d939a2d  MUSICS.BPA/ENDANI-E.CMF every effect, 2 s each
10799cfdd2a5d8761b17bbf66104fda5e2f5e3effcea99607225aaffee686659  MUSICS.BPA/ENDANI0E.CMF every effect, 2 s each
8107b9a0e7c380335725c4bfe36f9b3295b9dc6e62ce9fe9a4ae62b5d1dc167c  MUSICS.BPA/GEN-EFE.CMF every effect, 2 s each
604cf8f8380785e8140ef3981dbc219cf58b0f8ab2d15435bd35754827cdf8aa  MUSICS.BPA/MEN-SAM.CMF every effect, 2 s each
```

Run: `DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (16 passed).

- [ ] **Step 27: Run the game to its end**

The real frontend on its own Xvfb display, its sound written to a file; keys skip the startup, answer the exit question with yes and end the end screen. Save this as `/tmp/frontend-quit.sh` (`chmod +x`), and `/tmp/watch-silent.sh` from M1b's plan (Task 1, Step 1) if it is not there:

```bash
#!/usr/bin/env bash
# End-to-end: the real frontend on its own Xvfb, sound to a file; keys skip the startup, answer
# the exit question with yes and end the end screen; the game must exit by itself.
set -uo pipefail
out=$1; mkdir -p "$out"
fd=$(mktemp)
Xvfb -displayfd 3 -screen 0 1280x1024x24 -nolisten tcp 3>"$fd" 2>/dev/null &
xvfb=$!
for _ in $(seq 100); do [ -s "$fd" ] && break; sleep 0.05; done
export DISPLAY=":$(head -n1 "$fd")"
alsa=$(mktemp); printf '%s\n' 'pcm.!default { type null }' > "$alsa"
SDL_AUDIO_DRIVER=disk SDL_AUDIO_DISK_OUTPUT_FILE="$out/audio.raw" PULSE_SERVER=unix:/nonexistent \
  PIPEWIRE_REMOTE=/nonexistent ALSA_CONFIG_PATH="$alsa" DEADRALLY_DATA=~/games/DeathRally \
  target/release/deadrally -window > "$out/stats.log" 2> "$out/stderr.log" &
app=$!
timeout 20 xdotool search --sync --name '^DR$' >/dev/null; sleep 1
win=$(xdotool search --name '^DR$' | tail -n1); xdotool windowfocus --sync "$win"
key() { xdotool key --window "$win" "$1"; sleep "$2"; }
key space 1; key space 1; key space 1                # intro, both logos
sleep 3                                              # title, fade to black, menu fade-in
import -window "$win" "$out/menu.png"
key Escape 0.5; key Return 0.5; key Left 0.5
import -window "$win" "$out/question.png"
key Return 2.5
import -window "$win" "$out/end.png"
key space 0.2
for i in $(seq 50); do kill -0 $app 2>/dev/null || break; sleep 0.1; done
if kill -0 $app 2>/dev/null; then echo "FAIL: still running"; kill $app; status=1; else wait $app; echo "exited by itself, status $?"; status=0; fi
kill $xvfb; rm -f "$fd" "$alsa"
exit $status
```

Run: `cargo build --release -p deadrally && /tmp/watch-silent.sh /tmp/frontend-quit.sh captures/frontend-quit`
Expected: `exited by itself, status 0` and `watch: command exit 0, leaks 0`; `captures/frontend-quit/menu.png` shows the main menu, `question.png` the exit question with "yes" selected, `end.png` the end screen.

- [ ] **Step 28: Commit**

```bash
git add crates/core crates/deadrally/src/main.rs crates/headless/tests/rendered-audio.sha256
git commit -m "feat: add the main menu" -m "- the drawing buffer, fonts and the original's one-key input
- the menu's palette, popups, cursor and bottom panel
- the scene: the fade from the title, the highlight, the start submenu, the exit question, the end screen and the credits
- volumes, the volume mask and the menu's effects in Sound
- the frontend quits when the game ends"
```

---

### Task 6: Checking the menus against the original

Spec 4.3 and 5. `--key-at` names its key, so `find` and `render-audio --startup` follow a scenario through the menus. Three scenarios of the original's menus; their screenshots must equal our frames, the recording must sound like our render; then a manifest pins the frames and the sound of that run.

**Files:**
- Create: `scripts/reference/menu-transition.scenario`, `scripts/reference/menu-explore.scenario`, `scripts/reference/menu-keys.scenario`, `crates/headless/tests/common/mod.rs`, `crates/headless/tests/menu_run.rs`, `crates/headless/tests/menu-run.sha256`
- Modify: `crates/headless/src/main.rs`, `crates/headless/tests/rendered_audio.rs`

**Interfaces:**
- Consumes: `Game::{new, input, tick, frame, take_audio, quit_requested}` (Task 5).
- Produces: `--key-at T[:KEY]` with `KEY` one of space (the default), enter, escape, up, down, left, right, y, n; an unknown name is an error that lists the known ones. Test helpers `common::{located, hash, hex, check_manifest}` for the data tests.

- [ ] **Step 1: Write the failing tests**

The headless binary's tests, among them `a_key_name_the_scenarios_do_not_use_is_an_error_that_lists_the_known_ones`, over M1b's code:

<!-- write: crates/headless/src/main.rs -->
```rust
//! Runs the core without a window (spec section 8, spec M1a section 7).
//!
//! `run --ticks N` hashes every frame and every audio sample, so two runs, or the same run on
//! two operating systems, can be compared with one line. `check-data` reports where the game
//! data was found and whether it is a known release. `dump-assets` writes every catalogued
//! image as a PNG. `render`, `compare` and `find` check the startup sequence against
//! screenshots of the original (scripts/reference-run.sh). `render-audio` writes what the game
//! plays as a WAV, and `compare-audio` checks it against a recording of the original (spec M1b
//! sections 4.4 and 5).

mod audio_compare;
mod dump;
mod rgb;
mod wav;
mod window;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadrally_core::{
    AUDIO_SAMPLE_RATE, Game, InputEvent, Key, TICK_NANOS, render_effect, render_music,
};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound;
use deadrally_gamedata::{DATA_ENV_VAR, Located, Outcome, config_path, locate};
use sha2::{Digest, Sha256};

use crate::rgb::{Difference, Rgb};
use crate::wav::Wav;

const USAGE: &str = "usage:
  deadrally-headless run --ticks N
  deadrally-headless check-data [--data PATH]
  deadrally-headless dump-assets [--data PATH] [--out DIR]
  deadrally-headless render [--data PATH] --tick T [--key-at T]... --out FILE.png
  deadrally-headless compare A.png B.png
  deadrally-headless find [--data PATH] [--key-at T]... [--ticks N] SHOT.png...
  deadrally-headless render-audio [--data PATH] --startup [--key-at T]... [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --music NAME [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --effect BANK --number K --out FILE.wav
  deadrally-headless compare-audio ORIGINAL.wav OURS.wav [--min-overlap S]";

/// Exit status of `check-data` when the data is usable but not a known release.
const EXIT_UNKNOWN_VERSION: u8 = 2;

/// `find` runs this many ticks by default: the whole startup sequence of the known version
/// (6219 ticks) and then some.
const FIND_TICKS: u64 = 7_000;

/// `render-audio --startup` renders the whole intro and then this many ticks (2 s) by default:
/// the menu music starting.
const STARTUP_AFTER_INTRO_TICKS: u64 = 143;

/// `render-audio --music` renders this many seconds by default.
const MUSIC_SECONDS: u64 = 30;
/// `render-audio --effect` renders at most this many seconds, then trims the silence.
const EFFECT_SECONDS: u64 = 10;

/// `compare-audio` tolerances (spec M1b §5).
const LOUDNESS_MEDIAN_DB: f64 = 1.5;
const LOUDNESS_MAX_DB: f64 = 4.0;
const BAND_DB: f64 = 3.0;
const TEMPO_PERCENT: f64 = 0.15;
const PITCH_CENTS: f64 = 10.0;
const BALANCE_DB: f64 = 1.0;
const MIN_OVERLAP_SECONDS: u64 = 25;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run {
        ticks: u64,
    },
    CheckData {
        data: Option<PathBuf>,
    },
    DumpAssets {
        data: Option<PathBuf>,
        out: PathBuf,
    },
    Render {
        data: Option<PathBuf>,
        tick: u64,
        keys: Vec<u64>,
        out: PathBuf,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
    },
    Find {
        data: Option<PathBuf>,
        keys: Vec<u64>,
        ticks: u64,
        shots: Vec<PathBuf>,
    },
    RenderAudio {
        data: Option<PathBuf>,
        source: AudioSource,
        keys: Vec<u64>,
        seconds: Option<u64>,
        out: PathBuf,
    },
    CompareAudio {
        original: PathBuf,
        ours: PathBuf,
        min_overlap: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum AudioSource {
    Startup,
    Music(String),
    Effect { bank: String, number: u8 },
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("error: {message}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match command {
        Command::Run { ticks } => {
            println!("{}", run(ticks));
            Ok(ExitCode::SUCCESS)
        }
        Command::CheckData { data } => Ok(check_data(data.as_deref())),
        Command::DumpAssets { data, out } => locate_data(data.as_deref()).and_then(|located| {
            let count = dump::dump_assets(&located.validation, &out)?;
            println!("wrote {count} images to {}", out.display());
            Ok(ExitCode::SUCCESS)
        }),
        Command::Render {
            data,
            tick,
            keys,
            out,
        } => render(data.as_deref(), tick, &keys, &out).map(|()| ExitCode::SUCCESS),
        Command::Compare { a, b } => compare(&a, &b),
        Command::Find {
            data,
            keys,
            ticks,
            shots,
        } => find(data.as_deref(), &keys, ticks, &shots),
        Command::RenderAudio {
            data,
            source,
            keys,
            seconds,
            out,
        } => {
            render_audio(data.as_deref(), &source, &keys, seconds, &out).map(|()| ExitCode::SUCCESS)
        }
        Command::CompareAudio {
            original,
            ours,
            min_overlap,
        } => compare_audio(&original, &ours, min_overlap),
    };
    result.unwrap_or_else(|message| {
        eprintln!("error: {message}");
        ExitCode::FAILURE
    })
}

fn parse(args: &[OsString]) -> Result<Command, String> {
    let mut args = args.iter();
    let command = args.next().ok_or("missing command")?.to_string_lossy();
    let command = &*command;
    let options: &[&str] = match command {
        "run" => &["--ticks"],
        "check-data" => &["--data"],
        "dump-assets" => &["--data", "--out"],
        "render" => &["--data", "--tick", "--key-at", "--out"],
        "compare" => &[],
        "find" => &["--data", "--key-at", "--ticks"],
        "render-audio" => &[
            "--data",
            "--music",
            "--effect",
            "--number",
            "--seconds",
            "--key-at",
            "--out",
        ],
        "compare-audio" => &["--min-overlap"],
        _ => return Err(format!("unknown command: {command}")),
    };
    let takes_files = matches!(command, "compare" | "find" | "compare-audio");
    let (mut data, mut out, mut ticks, mut tick) = (None, None, None, None);
    let (mut startup, mut music, mut effect, mut effect_number, mut seconds, mut min_overlap) =
        (false, None, None, None, None, None);
    let mut keys = Vec::new();
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        let name = arg.to_str().unwrap_or_default();
        if command == "render-audio" && name == "--startup" {
            startup = true;
        } else if options.contains(&name) {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            let number = || {
                value
                    .to_str()
                    .and_then(|text| text.parse::<u64>().ok())
                    .ok_or(format!("{name}: not a number: {}", value.to_string_lossy()))
            };
            match name {
                "--data" => data = Some(PathBuf::from(value)),
                "--out" => out = Some(PathBuf::from(value)),
                "--ticks" => ticks = Some(number()?),
                "--tick" => tick = Some(number()?),
                "--key-at" => keys.push(number()?),
                "--music" => music = Some(value.to_string_lossy().into_owned()),
                "--effect" => effect = Some(value.to_string_lossy().into_owned()),
                "--number" => {
                    effect_number = Some(
                        u8::try_from(number()?).map_err(|_| "--number: an effect is 1 to 255")?,
                    );
                }
                "--seconds" => seconds = Some(number()?),
                "--min-overlap" => min_overlap = Some(number()?),
                _ => unreachable!("every option is handled"),
            }
        } else if takes_files && !name.starts_with("--") {
            files.push(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {}", arg.to_string_lossy()));
        }
    }
    match command {
        "run" => Ok(Command::Run {
            ticks: ticks.ok_or("run needs --ticks N")?,
        }),
        "check-data" => Ok(Command::CheckData { data }),
        "dump-assets" => Ok(Command::DumpAssets {
            data,
            out: out.unwrap_or_else(|| PathBuf::from("dumps")),
        }),
        "render" => Ok(Command::Render {
            data,
            tick: tick.ok_or("render needs --tick T")?,
            keys,
            out: out.ok_or("render needs --out FILE.png")?,
        }),
        "compare" => match <[PathBuf; 2]>::try_from(files) {
            Ok([a, b]) => Ok(Command::Compare { a, b }),
            Err(_) => Err("compare needs exactly two PNG files".into()),
        },
        "compare-audio" => match <[PathBuf; 2]>::try_from(files) {
            Ok([original, ours]) => Ok(Command::CompareAudio {
                original,
                ours,
                min_overlap: min_overlap.unwrap_or(MIN_OVERLAP_SECONDS),
            }),
            Err(_) => Err("compare-audio needs exactly two WAV files".into()),
        },
        "render-audio" => {
            let source = match (startup, music, effect, effect_number) {
                (true, None, None, None) => AudioSource::Startup,
                (false, Some(name), None, None) => AudioSource::Music(name),
                (false, None, Some(bank), Some(number)) if number > 0 => {
                    AudioSource::Effect { bank, number }
                }
                _ => {
                    return Err("render-audio needs one of --startup, --music NAME, or --effect BANK --number K".into());
                }
            };
            if source != AudioSource::Startup && !keys.is_empty() {
                return Err("--key-at only applies to --startup".into());
            }
            Ok(Command::RenderAudio {
                data,
                source,
                keys,
                seconds,
                out: out.ok_or("render-audio needs --out FILE.wav")?,
            })
        }
        _ => {
            if files.is_empty() {
                return Err("find needs at least one screenshot".into());
            }
            Ok(Command::Find {
                data,
                keys,
                ticks: ticks.unwrap_or(FIND_TICKS),
                shots: files,
            })
        }
    }
}

/// Runs `ticks` ticks without input and hashes, per tick in order: width, height and both
/// aspect terms as little-endian u32, the 768 palette bytes, the pixels; and every audio sample
/// as little-endian i16.
fn run(ticks: u64) -> String {
    let mut game = Game::test_scene();
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

/// Finds and validates the data like `check-data`, warning instead of failing on an unknown
/// version.
fn locate_data(cli: Option<&Path>) -> Result<Located, String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let located =
        locate(cli, env.as_deref(), config.as_deref()).map_err(|error| error.to_string())?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    if let Outcome::Unknown { closest, .. } = &located.validation.outcome {
        eprintln!(
            "warning: unknown data version (closest: {closest}); it may not match the original"
        );
    }
    Ok(located)
}

/// Runs `ticks` ticks, pressing and releasing a key after each tick count in `keys` (0: before
/// the first tick), and calls `each` with the tick count and the game after every tick.
fn play(game: &mut Game, ticks: u64, keys: &[u64], mut each: impl FnMut(u64, &Game)) {
    for done in 0..ticks {
        if keys.contains(&done) {
            for pressed in [true, false] {
                game.input(InputEvent::Key {
                    key: Key::Space,
                    pressed,
                });
            }
        }
        game.tick();
        each(done + 1, game);
    }
}

/// Writes the frame after `tick` ticks as the original's window would show it.
fn render(data: Option<&Path>, tick: u64, keys: &[u64], out: &Path) -> Result<(), String> {
    let located = locate_data(data)?;
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let mut game = Game::new(assets);
    play(&mut game, tick, keys, |_, _| {});
    window::present(&game.frame())?.write_png(out)
}

/// Exit status 0 only when the pictures are identical.
fn compare(a: &Path, b: &Path) -> Result<ExitCode, String> {
    let (first, second) = (Rgb::read_png(a)?, Rgb::read_png(b)?);
    let Some(difference) = first.difference(&second) else {
        println!(
            "sizes differ: {}x{} and {}x{}",
            first.width, first.height, second.width, second.height
        );
        return Ok(ExitCode::FAILURE);
    };
    println!(
        "{}x{}: {} pixels differ, largest channel difference {}",
        first.width, first.height, difference.pixels, difference.max_channel
    );
    Ok(if difference.pixels == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// For each screenshot of the original, the ticks of our startup sequence that show exactly
/// the same picture. Exit status 0 only when every screenshot has a match.
fn find(
    data: Option<&Path>,
    keys: &[u64],
    ticks: u64,
    shots: &[PathBuf],
) -> Result<ExitCode, String> {
    let pictures = shots
        .iter()
        .map(|path| Rgb::read_png(path))
        .collect::<Result<Vec<_>, _>>()?;
    for (path, picture) in shots.iter().zip(&pictures) {
        if (picture.width, picture.height) != (window::WIDTH, window::HEIGHT) {
            return Err(format!(
                "{}: {}x{}, the original's window is {}x{}",
                path.display(),
                picture.width,
                picture.height,
                window::WIDTH,
                window::HEIGHT
            ));
        }
    }
    let located = locate_data(data)?;
    // First only exact matches, which fail fast on the first differing byte.
    let mut matches = vec![Vec::new(); shots.len()];
    let mut equal = vec![false; shots.len()];
    timeline(&located, keys, ticks, |tick, window, changed| {
        for (index, picture) in pictures.iter().enumerate() {
            if changed {
                equal[index] = window.pixels == picture.pixels;
            }
            if equal[index] {
                matches[index].push(tick);
            }
        }
    })?;
    // Then, for screenshots without a match, the nearest picture, to help find out why.
    let unmatched: Vec<usize> = (0..shots.len())
        .filter(|&index| matches[index].is_empty())
        .collect();
    let mut closest: Vec<Option<(Difference, u64)>> = vec![None; shots.len()];
    if !unmatched.is_empty() {
        timeline(&located, keys, ticks, |tick, window, changed| {
            if !changed {
                return;
            }
            for &index in &unmatched {
                let difference = window.difference(&pictures[index]).expect("window-sized");
                if closest[index].is_none_or(|(best, _)| difference < best) {
                    closest[index] = Some((difference, tick));
                }
            }
        })?;
    }
    for (index, path) in shots.iter().enumerate() {
        match closest[index] {
            None => println!("{}: ticks {}", path.display(), ranges(&matches[index])),
            Some((difference, tick)) => println!(
                "{}: no exact match; closest is tick {tick}: {} pixels differ, largest channel difference {}",
                path.display(),
                difference.pixels,
                difference.max_channel
            ),
        }
    }
    Ok(if unmatched.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Plays the startup sequence for `ticks` ticks and calls `each` with every tick count from 0,
/// the window picture and whether it changed since the previous call. Most ticks repeat the
/// previous picture, so callers can skip work on those.
fn timeline(
    located: &Located,
    keys: &[u64],
    ticks: u64,
    mut each: impl FnMut(u64, &Rgb, bool),
) -> Result<(), String> {
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let mut game = Game::new(assets);
    let mut previous: Option<(Vec<u8>, Vec<[u8; 3]>, Rgb)> = None;
    let mut failure = None;
    let mut visit = |tick: u64, game: &Game| {
        let frame = game.frame();
        let changed = previous
            .as_ref()
            .is_none_or(|(pixels, palette, _)| pixels != frame.pixels || palette != frame.palette);
        if changed {
            match window::present(&frame) {
                Ok(window) => {
                    previous = Some((frame.pixels.to_vec(), frame.palette.to_vec(), window));
                }
                Err(error) => {
                    failure.get_or_insert(format!("tick {tick}: {error}"));
                    return;
                }
            }
        }
        if let Some((_, _, window)) = &previous {
            each(tick, window, changed);
        }
    };
    visit(0, &game);
    play(&mut game, ticks, keys, &mut visit);
    failure.map_or(Ok(()), Err)
}

/// A sound file's entry name from `TR0-MUS` or `tr0-mus.cmf`.
fn sound_entry(name: &str) -> String {
    let upper = name.to_ascii_uppercase();
    if upper.ends_with(".CMF") {
        upper
    } else {
        format!("{upper}.CMF")
    }
}

/// Writes the startup's sound (the intro, then the menu music), a piece of music or one effect
/// as a 48 kHz 16-bit WAV.
fn render_audio(
    data: Option<&Path>,
    source: &AudioSource,
    keys: &[u64],
    seconds: Option<u64>,
    out: &Path,
) -> Result<(), String> {
    let located = locate_data(data)?;
    let frames = |seconds: u64| {
        usize::try_from(seconds * u64::from(AUDIO_SAMPLE_RATE)).unwrap_or(usize::MAX)
    };
    let musics = || {
        let file = located
            .validation
            .files
            .iter()
            .find(|file| file.name == sound::ARCHIVE)
            .ok_or("MUSICS.BPA is not among the validated files")?;
        Archive::open(&file.path).map_err(|error| error.to_string())
    };
    let samples = match source {
        AudioSource::Startup => {
            let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
            let intro_ticks = assets
                .intro
                .delays
                .iter()
                .map(|&delay| u64::from(delay))
                .sum::<u64>();
            let ticks = seconds.map_or(intro_ticks + STARTUP_AFTER_INTRO_TICKS, |seconds| {
                seconds * 1_000_000_000 / TICK_NANOS
            });
            let mut game = Game::new(assets);
            let mut audio = Vec::new();
            for done in 0..ticks {
                if keys.contains(&done) {
                    for pressed in [true, false] {
                        game.input(InputEvent::Key {
                            key: Key::Space,
                            pressed,
                        });
                    }
                }
                game.tick();
                game.take_audio(&mut audio);
            }
            audio
        }
        AudioSource::Music(name) => {
            let module = sound::load_music(&musics()?, &sound_entry(name))
                .map_err(|error| error.to_string())?;
            render_music(&module, frames(seconds.unwrap_or(MUSIC_SECONDS)))
        }
        AudioSource::Effect { bank, number } => {
            let bank = sound::load_effects(&musics()?, &sound_entry(bank))
                .map_err(|error| error.to_string())?;
            let mut samples =
                render_effect(&bank, *number, frames(seconds.unwrap_or(EFFECT_SECONDS)));
            let end = samples
                .iter()
                .rposition(|&sample| sample != 0)
                .map_or(0, |last| (last / 2 + 1) * 2);
            samples.truncate(end);
            samples
        }
    };
    Wav {
        rate: AUDIO_SAMPLE_RATE,
        channels: 2,
        samples,
    }
    .write(out)
}

/// Exit status 0 only when every measure is within the spec's tolerances.
fn compare_audio(original: &Path, ours: &Path, min_overlap: u64) -> Result<ExitCode, String> {
    let report = audio_compare::compare(&Wav::read(original)?, &Wav::read(ours)?);
    println!(
        "lag: {:.0} ms (envelope correlation {:.3})",
        report.lag_ms, report.correlation
    );
    println!("overlap: {:.1} s", report.overlap_seconds);
    println!(
        "loudness per second, ours - original: median {:.2} dB, largest {:.2} dB",
        report.loudness_median_db, report.loudness_max_db
    );
    let bands: Vec<String> = report
        .bands
        .iter()
        .map(|(hz, difference)| format!("{hz:.0} Hz {difference:+.1} dB"))
        .collect();
    println!("octave bands, ours - original: {}", bands.join(", "));
    println!(
        "tempo, ours - original: {:+.3} % (over {} pieces of 10 s)",
        report.tempo_percent, report.tempo_pieces
    );
    println!("pitch, ours - original: {:+.0} cents", report.pitch_cents);
    println!(
        "stereo balance (left - right), ours - original: largest {:+.2} dB (over {} pieces of 10 s)",
        report.balance_db, report.balance_pieces
    );
    let mut failures = Vec::new();
    if report.overlap_seconds < min_overlap as f64 {
        failures.push(format!("overlap below {min_overlap} s"));
    }
    if report.loudness_median_db > LOUDNESS_MEDIAN_DB {
        failures.push(format!(
            "median loudness difference above {LOUDNESS_MEDIAN_DB} dB"
        ));
    }
    if report.loudness_max_db > LOUDNESS_MAX_DB {
        failures.push(format!(
            "largest loudness difference above {LOUDNESS_MAX_DB} dB"
        ));
    }
    for (hz, difference) in &report.bands {
        if difference.abs() > BAND_DB {
            failures.push(format!("{hz:.0} Hz band off by more than {BAND_DB} dB"));
        }
    }
    // A measure that could not be taken fails: passing it would hide the difference it exists
    // to find.
    if report.tempo_pieces < audio_compare::MIN_TEMPO_PIECES {
        failures.push(format!(
            "tempo measured on fewer than {} pieces of 10 s",
            audio_compare::MIN_TEMPO_PIECES
        ));
    } else if report.tempo_percent.abs() > TEMPO_PERCENT {
        failures.push(format!("tempo off by more than {TEMPO_PERCENT} %"));
    }
    if report.pitch_cents.abs() > PITCH_CENTS {
        failures.push(format!("pitch off by more than {PITCH_CENTS} cents"));
    }
    if report.balance_pieces == 0 {
        failures.push("stereo balance not measured (a mono or silent file)".to_owned());
    } else if report.balance_db.abs() > BALANCE_DB {
        failures.push(format!("stereo balance off by more than {BALANCE_DB} dB"));
    }
    if failures.is_empty() {
        println!("result: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("result: FAIL ({})", failures.join("; "));
        Ok(ExitCode::FAILURE)
    }
}

/// `[3, 4, 5, 9]` as `3-5, 9`.
fn ranges(ticks: &[u64]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0;
    for index in 1..=ticks.len() {
        if index == ticks.len() || ticks[index] != ticks[index - 1] + 1 {
            parts.push(if index - 1 == start {
                ticks[start].to_string()
            } else {
                format!("{}-{}", ticks[start], ticks[index - 1])
            });
            start = index;
        }
    }
    parts.join(", ")
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

    #[test]
    fn a_key_name_the_scenarios_do_not_use_is_an_error_that_lists_the_known_ones() {
        // A typo would otherwise press nothing, and a shot taken after it would match the
        // wrong frame.
        let error = parse(&args(&[
            "render", "--tick", "3", "--key-at", "2:enetr", "--out", "a.png",
        ]))
        .unwrap_err();
        assert!(error.contains("unknown key enetr"), "{error}");
        assert!(error.contains("escape"), "{error}");
    }

    #[test]
    fn parses_the_asset_commands() {
        assert_eq!(
            parse(&args(&["dump-assets"])),
            Ok(Command::DumpAssets {
                data: None,
                out: PathBuf::from("dumps")
            })
        );
        assert_eq!(
            parse(&args(&[
                "render", "--tick", "300", "--key-at", "10", "--key-at", "20:Down", "--out",
                "a.png"
            ])),
            Ok(Command::Render {
                data: None,
                tick: 300,
                keys: vec![(10, Key::Space), (20, Key::Down)],
                out: PathBuf::from("a.png")
            })
        );
        assert_eq!(
            parse(&args(&["compare", "a.png", "b.png"])),
            Ok(Command::Compare {
                a: PathBuf::from("a.png"),
                b: PathBuf::from("b.png")
            })
        );
        assert_eq!(
            parse(&args(&["find", "--data", "/x", "a.png", "b.png"])),
            Ok(Command::Find {
                data: Some(PathBuf::from("/x")),
                keys: vec![],
                ticks: FIND_TICKS,
                shots: vec![PathBuf::from("a.png"), PathBuf::from("b.png")]
            })
        );
    }

    #[test]
    fn rejects_incomplete_asset_commands() {
        for bad in [
            &["render", "--out", "a.png"][..],
            &["render", "--tick", "3"],
            &["render", "--tick", "3", "--out", "a.png", "extra.png"],
            &["compare", "a.png"],
            &["compare", "a.png", "b.png", "c.png"],
            &["find"],
            &["find", "--ticks", "many", "a.png"],
            &["dump-assets", "--tick", "3"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn tick_lists_are_printed_as_ranges() {
        assert_eq!(ranges(&[3, 4, 5, 9]), "3-5, 9");
        assert_eq!(ranges(&[7]), "7");
        assert_eq!(ranges(&[1, 3]), "1, 3");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p deadrally-headless --bin deadrally-headless`
Expected: FAIL to compile (`mismatched types`: the tests expect each `--key-at` to give a tick and a key, M1b's code gives a tick).

- [ ] **Step 3: Implement**

<!-- write: crates/headless/src/main.rs -->
```rust
//! Runs the core without a window (spec section 8, spec M1a section 7).
//!
//! `run --ticks N` hashes every frame and every audio sample, so two runs, or the same run on
//! two operating systems, can be compared with one line. `check-data` reports where the game
//! data was found and whether it is a known release. `dump-assets` writes every catalogued
//! image as a PNG. `render`, `compare` and `find` check the startup sequence and the menus
//! against screenshots of the original (scripts/reference-run.sh). `render-audio` writes what
//! the game plays as a WAV, and `compare-audio` checks it against a recording of the original
//! (spec M1b sections 4.4 and 5).

mod audio_compare;
mod dump;
mod rgb;
mod wav;
mod window;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deadrally_core::{
    AUDIO_SAMPLE_RATE, Game, InputEvent, Key, TICK_NANOS, render_effect, render_music,
};
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound;
use deadrally_gamedata::{DATA_ENV_VAR, Located, Outcome, config_path, locate};
use sha2::{Digest, Sha256};

use crate::rgb::{Difference, Rgb};
use crate::wav::Wav;

const USAGE: &str = "usage:
  deadrally-headless run --ticks N
  deadrally-headless check-data [--data PATH]
  deadrally-headless dump-assets [--data PATH] [--out DIR]
  deadrally-headless render [--data PATH] --tick T [--key-at T[:KEY]]... --out FILE.png
  deadrally-headless compare A.png B.png
  deadrally-headless find [--data PATH] [--key-at T[:KEY]]... [--ticks N] SHOT.png...
  deadrally-headless render-audio [--data PATH] --startup [--key-at T[:KEY]]... [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --music NAME [--seconds S] --out FILE.wav
  deadrally-headless render-audio [--data PATH] --effect BANK --number K --out FILE.wav
  deadrally-headless compare-audio ORIGINAL.wav OURS.wav [--min-overlap S]";

/// Exit status of `check-data` when the data is usable but not a known release.
const EXIT_UNKNOWN_VERSION: u8 = 2;

/// `find` runs this many ticks by default: the whole startup sequence of the known version
/// (6219 ticks) and then some.
const FIND_TICKS: u64 = 7_000;

/// `render-audio --startup` renders the whole intro and then this many ticks (2 s) by default:
/// the menu music starting.
const STARTUP_AFTER_INTRO_TICKS: u64 = 143;

/// `render-audio --music` renders this many seconds by default.
const MUSIC_SECONDS: u64 = 30;
/// `render-audio --effect` renders at most this many seconds, then trims the silence.
const EFFECT_SECONDS: u64 = 10;

/// `compare-audio` tolerances (spec M1b §5).
const LOUDNESS_MEDIAN_DB: f64 = 1.5;
const LOUDNESS_MAX_DB: f64 = 4.0;
const BAND_DB: f64 = 3.0;
const TEMPO_PERCENT: f64 = 0.15;
const PITCH_CENTS: f64 = 10.0;
const BALANCE_DB: f64 = 1.0;
const MIN_OVERLAP_SECONDS: u64 = 25;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run {
        ticks: u64,
    },
    CheckData {
        data: Option<PathBuf>,
    },
    DumpAssets {
        data: Option<PathBuf>,
        out: PathBuf,
    },
    Render {
        data: Option<PathBuf>,
        tick: u64,
        keys: Vec<Press>,
        out: PathBuf,
    },
    Compare {
        a: PathBuf,
        b: PathBuf,
    },
    Find {
        data: Option<PathBuf>,
        keys: Vec<Press>,
        ticks: u64,
        shots: Vec<PathBuf>,
    },
    RenderAudio {
        data: Option<PathBuf>,
        source: AudioSource,
        keys: Vec<Press>,
        seconds: Option<u64>,
        out: PathBuf,
    },
    CompareAudio {
        original: PathBuf,
        ours: PathBuf,
        min_overlap: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum AudioSource {
    Startup,
    Music(String),
    Effect { bank: String, number: u8 },
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("error: {message}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let result = match command {
        Command::Run { ticks } => {
            println!("{}", run(ticks));
            Ok(ExitCode::SUCCESS)
        }
        Command::CheckData { data } => Ok(check_data(data.as_deref())),
        Command::DumpAssets { data, out } => locate_data(data.as_deref()).and_then(|located| {
            let count = dump::dump_assets(&located.validation, &out)?;
            println!("wrote {count} images to {}", out.display());
            Ok(ExitCode::SUCCESS)
        }),
        Command::Render {
            data,
            tick,
            keys,
            out,
        } => render(data.as_deref(), tick, &keys, &out).map(|()| ExitCode::SUCCESS),
        Command::Compare { a, b } => compare(&a, &b),
        Command::Find {
            data,
            keys,
            ticks,
            shots,
        } => find(data.as_deref(), &keys, ticks, &shots),
        Command::RenderAudio {
            data,
            source,
            keys,
            seconds,
            out,
        } => {
            render_audio(data.as_deref(), &source, &keys, seconds, &out).map(|()| ExitCode::SUCCESS)
        }
        Command::CompareAudio {
            original,
            ours,
            min_overlap,
        } => compare_audio(&original, &ours, min_overlap),
    };
    result.unwrap_or_else(|message| {
        eprintln!("error: {message}");
        ExitCode::FAILURE
    })
}

fn parse(args: &[OsString]) -> Result<Command, String> {
    let mut args = args.iter();
    let command = args.next().ok_or("missing command")?.to_string_lossy();
    let command = &*command;
    let options: &[&str] = match command {
        "run" => &["--ticks"],
        "check-data" => &["--data"],
        "dump-assets" => &["--data", "--out"],
        "render" => &["--data", "--tick", "--key-at", "--out"],
        "compare" => &[],
        "find" => &["--data", "--key-at", "--ticks"],
        "render-audio" => &[
            "--data",
            "--music",
            "--effect",
            "--number",
            "--seconds",
            "--key-at",
            "--out",
        ],
        "compare-audio" => &["--min-overlap"],
        _ => return Err(format!("unknown command: {command}")),
    };
    let takes_files = matches!(command, "compare" | "find" | "compare-audio");
    let (mut data, mut out, mut ticks, mut tick) = (None, None, None, None);
    let (mut startup, mut music, mut effect, mut effect_number, mut seconds, mut min_overlap) =
        (false, None, None, None, None, None);
    let mut keys = Vec::new();
    let mut files = Vec::new();
    while let Some(arg) = args.next() {
        let name = arg.to_str().unwrap_or_default();
        if command == "render-audio" && name == "--startup" {
            startup = true;
        } else if options.contains(&name) {
            let value = args.next().ok_or(format!("{name} needs a value"))?;
            let number = || {
                value
                    .to_str()
                    .and_then(|text| text.parse::<u64>().ok())
                    .ok_or(format!("{name}: not a number: {}", value.to_string_lossy()))
            };
            match name {
                "--data" => data = Some(PathBuf::from(value)),
                "--out" => out = Some(PathBuf::from(value)),
                "--ticks" => ticks = Some(number()?),
                "--tick" => tick = Some(number()?),
                "--key-at" => keys.push(press(&value.to_string_lossy())?),
                "--music" => music = Some(value.to_string_lossy().into_owned()),
                "--effect" => effect = Some(value.to_string_lossy().into_owned()),
                "--number" => {
                    effect_number = Some(
                        u8::try_from(number()?).map_err(|_| "--number: an effect is 1 to 255")?,
                    );
                }
                "--seconds" => seconds = Some(number()?),
                "--min-overlap" => min_overlap = Some(number()?),
                _ => unreachable!("every option is handled"),
            }
        } else if takes_files && !name.starts_with("--") {
            files.push(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {}", arg.to_string_lossy()));
        }
    }
    match command {
        "run" => Ok(Command::Run {
            ticks: ticks.ok_or("run needs --ticks N")?,
        }),
        "check-data" => Ok(Command::CheckData { data }),
        "dump-assets" => Ok(Command::DumpAssets {
            data,
            out: out.unwrap_or_else(|| PathBuf::from("dumps")),
        }),
        "render" => Ok(Command::Render {
            data,
            tick: tick.ok_or("render needs --tick T")?,
            keys,
            out: out.ok_or("render needs --out FILE.png")?,
        }),
        "compare" => match <[PathBuf; 2]>::try_from(files) {
            Ok([a, b]) => Ok(Command::Compare { a, b }),
            Err(_) => Err("compare needs exactly two PNG files".into()),
        },
        "compare-audio" => match <[PathBuf; 2]>::try_from(files) {
            Ok([original, ours]) => Ok(Command::CompareAudio {
                original,
                ours,
                min_overlap: min_overlap.unwrap_or(MIN_OVERLAP_SECONDS),
            }),
            Err(_) => Err("compare-audio needs exactly two WAV files".into()),
        },
        "render-audio" => {
            let source = match (startup, music, effect, effect_number) {
                (true, None, None, None) => AudioSource::Startup,
                (false, Some(name), None, None) => AudioSource::Music(name),
                (false, None, Some(bank), Some(number)) if number > 0 => {
                    AudioSource::Effect { bank, number }
                }
                _ => {
                    return Err("render-audio needs one of --startup, --music NAME, or --effect BANK --number K".into());
                }
            };
            if source != AudioSource::Startup && !keys.is_empty() {
                return Err("--key-at only applies to --startup".into());
            }
            Ok(Command::RenderAudio {
                data,
                source,
                keys,
                seconds,
                out: out.ok_or("render-audio needs --out FILE.wav")?,
            })
        }
        _ => {
            if files.is_empty() {
                return Err("find needs at least one screenshot".into());
            }
            Ok(Command::Find {
                data,
                keys,
                ticks: ticks.unwrap_or(FIND_TICKS),
                shots: files,
            })
        }
    }
}

/// Runs `ticks` ticks without input and hashes, per tick in order: width, height and both
/// aspect terms as little-endian u32, the 768 palette bytes, the pixels; and every audio sample
/// as little-endian i16.
fn run(ticks: u64) -> String {
    let mut game = Game::test_scene();
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

/// Finds and validates the data like `check-data`, warning instead of failing on an unknown
/// version.
fn locate_data(cli: Option<&Path>) -> Result<Located, String> {
    let config = config_path();
    let env = std::env::var_os(DATA_ENV_VAR);
    let located =
        locate(cli, env.as_deref(), config.as_deref()).map_err(|error| error.to_string())?;
    for warning in &located.config_warnings {
        eprintln!("warning: {warning}");
    }
    if let Outcome::Unknown { closest, .. } = &located.validation.outcome {
        eprintln!(
            "warning: unknown data version (closest: {closest}); it may not match the original"
        );
    }
    Ok(located)
}

/// A key pressed and released after a number of ticks (0: before the first tick).
type Press = (u64, Key);

/// The keys `--key-at T:KEY` can name; `T` alone presses Space.
const KEY_NAMES: [(&str, Key); 9] = [
    ("space", Key::Space),
    ("enter", Key::Enter),
    ("escape", Key::Escape),
    ("up", Key::Up),
    ("down", Key::Down),
    ("left", Key::Left),
    ("right", Key::Right),
    ("y", Key::Y),
    ("n", Key::N),
];

/// Parses `T` or `T:KEY`.
fn press(text: &str) -> Result<Press, String> {
    let (tick, name) = text.split_once(':').unwrap_or((text, "space"));
    let tick = tick
        .parse()
        .map_err(|_| format!("--key-at: not a number: {tick}"))?;
    let key = KEY_NAMES
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|&(_, key)| key)
        .ok_or_else(|| {
            let names: Vec<&str> = KEY_NAMES.iter().map(|(known, _)| *known).collect();
            format!(
                "--key-at: unknown key {name}; known keys: {}",
                names.join(", ")
            )
        })?;
    Ok((tick, key))
}

/// Presses and releases the keys due before tick `done + 1`, in the order given.
fn press_due(game: &mut Game, keys: &[Press], done: u64) {
    for &(_, key) in keys.iter().filter(|(tick, _)| *tick == done) {
        for pressed in [true, false] {
            game.input(InputEvent::Key { key, pressed });
        }
    }
}

/// Runs `ticks` ticks, pressing and releasing the keys in `keys`, and calls `each` with the tick
/// count and the game after every tick.
fn play(game: &mut Game, ticks: u64, keys: &[Press], mut each: impl FnMut(u64, &Game)) {
    for done in 0..ticks {
        press_due(game, keys, done);
        game.tick();
        each(done + 1, game);
    }
}

/// Writes the frame after `tick` ticks as the original's window would show it.
fn render(data: Option<&Path>, tick: u64, keys: &[Press], out: &Path) -> Result<(), String> {
    let located = locate_data(data)?;
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let mut game = Game::new(assets);
    play(&mut game, tick, keys, |_, _| {});
    window::present(&game.frame())?.write_png(out)
}

/// Exit status 0 only when the pictures are identical.
fn compare(a: &Path, b: &Path) -> Result<ExitCode, String> {
    let (first, second) = (Rgb::read_png(a)?, Rgb::read_png(b)?);
    let Some(difference) = first.difference(&second) else {
        println!(
            "sizes differ: {}x{} and {}x{}",
            first.width, first.height, second.width, second.height
        );
        return Ok(ExitCode::FAILURE);
    };
    println!(
        "{}x{}: {} pixels differ, largest channel difference {}",
        first.width, first.height, difference.pixels, difference.max_channel
    );
    Ok(if difference.pixels == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// For each screenshot of the original, the ticks of our startup sequence that show exactly
/// the same picture. Exit status 0 only when every screenshot has a match.
fn find(
    data: Option<&Path>,
    keys: &[Press],
    ticks: u64,
    shots: &[PathBuf],
) -> Result<ExitCode, String> {
    let pictures = shots
        .iter()
        .map(|path| Rgb::read_png(path))
        .collect::<Result<Vec<_>, _>>()?;
    for (path, picture) in shots.iter().zip(&pictures) {
        if (picture.width, picture.height) != (window::WIDTH, window::HEIGHT) {
            return Err(format!(
                "{}: {}x{}, the original's window is {}x{}",
                path.display(),
                picture.width,
                picture.height,
                window::WIDTH,
                window::HEIGHT
            ));
        }
    }
    let located = locate_data(data)?;
    // First only exact matches, which fail fast on the first differing byte.
    let mut matches = vec![Vec::new(); shots.len()];
    let mut equal = vec![false; shots.len()];
    timeline(&located, keys, ticks, |tick, window, changed| {
        for (index, picture) in pictures.iter().enumerate() {
            if changed {
                equal[index] = window.pixels == picture.pixels;
            }
            if equal[index] {
                matches[index].push(tick);
            }
        }
    })?;
    // Then, for screenshots without a match, the nearest picture, to help find out why.
    let unmatched: Vec<usize> = (0..shots.len())
        .filter(|&index| matches[index].is_empty())
        .collect();
    let mut closest: Vec<Option<(Difference, u64)>> = vec![None; shots.len()];
    if !unmatched.is_empty() {
        timeline(&located, keys, ticks, |tick, window, changed| {
            if !changed {
                return;
            }
            for &index in &unmatched {
                let difference = window.difference(&pictures[index]).expect("window-sized");
                if closest[index].is_none_or(|(best, _)| difference < best) {
                    closest[index] = Some((difference, tick));
                }
            }
        })?;
    }
    for (index, path) in shots.iter().enumerate() {
        match closest[index] {
            None => println!("{}: ticks {}", path.display(), ranges(&matches[index])),
            Some((difference, tick)) => println!(
                "{}: no exact match; closest is tick {tick}: {} pixels differ, largest channel difference {}",
                path.display(),
                difference.pixels,
                difference.max_channel
            ),
        }
    }
    Ok(if unmatched.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Plays the startup sequence for `ticks` ticks and calls `each` with every tick count from 0,
/// the window picture and whether it changed since the previous call. Most ticks repeat the
/// previous picture, so callers can skip work on those.
fn timeline(
    located: &Located,
    keys: &[Press],
    ticks: u64,
    mut each: impl FnMut(u64, &Rgb, bool),
) -> Result<(), String> {
    let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
    let mut game = Game::new(assets);
    let mut previous: Option<(Vec<u8>, Vec<[u8; 3]>, Rgb)> = None;
    let mut failure = None;
    let mut visit = |tick: u64, game: &Game| {
        let frame = game.frame();
        let changed = previous
            .as_ref()
            .is_none_or(|(pixels, palette, _)| pixels != frame.pixels || palette != frame.palette);
        if changed {
            match window::present(&frame) {
                Ok(window) => {
                    previous = Some((frame.pixels.to_vec(), frame.palette.to_vec(), window));
                }
                Err(error) => {
                    failure.get_or_insert(format!("tick {tick}: {error}"));
                    return;
                }
            }
        }
        if let Some((_, _, window)) = &previous {
            each(tick, window, changed);
        }
    };
    visit(0, &game);
    play(&mut game, ticks, keys, &mut visit);
    failure.map_or(Ok(()), Err)
}

/// A sound file's entry name from `TR0-MUS` or `tr0-mus.cmf`.
fn sound_entry(name: &str) -> String {
    let upper = name.to_ascii_uppercase();
    if upper.ends_with(".CMF") {
        upper
    } else {
        format!("{upper}.CMF")
    }
}

/// Writes the startup's sound (the intro, then the menu music), a piece of music or one effect
/// as a 48 kHz 16-bit WAV.
fn render_audio(
    data: Option<&Path>,
    source: &AudioSource,
    keys: &[Press],
    seconds: Option<u64>,
    out: &Path,
) -> Result<(), String> {
    let located = locate_data(data)?;
    let frames = |seconds: u64| {
        usize::try_from(seconds * u64::from(AUDIO_SAMPLE_RATE)).unwrap_or(usize::MAX)
    };
    let musics = || {
        let file = located
            .validation
            .files
            .iter()
            .find(|file| file.name == sound::ARCHIVE)
            .ok_or("MUSICS.BPA is not among the validated files")?;
        Archive::open(&file.path).map_err(|error| error.to_string())
    };
    let samples = match source {
        AudioSource::Startup => {
            let assets = Assets::load(&located.validation).map_err(|error| error.to_string())?;
            let intro_ticks = assets
                .intro
                .delays
                .iter()
                .map(|&delay| u64::from(delay))
                .sum::<u64>();
            let ticks = seconds.map_or(intro_ticks + STARTUP_AFTER_INTRO_TICKS, |seconds| {
                seconds * 1_000_000_000 / TICK_NANOS
            });
            let mut game = Game::new(assets);
            let mut audio = Vec::new();
            for done in 0..ticks {
                press_due(&mut game, keys, done);
                game.tick();
                game.take_audio(&mut audio);
            }
            audio
        }
        AudioSource::Music(name) => {
            let module = sound::load_music(&musics()?, &sound_entry(name))
                .map_err(|error| error.to_string())?;
            render_music(&module, frames(seconds.unwrap_or(MUSIC_SECONDS)))
        }
        AudioSource::Effect { bank, number } => {
            let bank = sound::load_effects(&musics()?, &sound_entry(bank))
                .map_err(|error| error.to_string())?;
            let mut samples =
                render_effect(&bank, *number, frames(seconds.unwrap_or(EFFECT_SECONDS)));
            let end = samples
                .iter()
                .rposition(|&sample| sample != 0)
                .map_or(0, |last| (last / 2 + 1) * 2);
            samples.truncate(end);
            samples
        }
    };
    Wav {
        rate: AUDIO_SAMPLE_RATE,
        channels: 2,
        samples,
    }
    .write(out)
}

/// Exit status 0 only when every measure is within the spec's tolerances.
fn compare_audio(original: &Path, ours: &Path, min_overlap: u64) -> Result<ExitCode, String> {
    let report = audio_compare::compare(&Wav::read(original)?, &Wav::read(ours)?);
    println!(
        "lag: {:.0} ms (envelope correlation {:.3})",
        report.lag_ms, report.correlation
    );
    println!("overlap: {:.1} s", report.overlap_seconds);
    println!(
        "loudness per second, ours - original: median {:.2} dB, largest {:.2} dB",
        report.loudness_median_db, report.loudness_max_db
    );
    let bands: Vec<String> = report
        .bands
        .iter()
        .map(|(hz, difference)| format!("{hz:.0} Hz {difference:+.1} dB"))
        .collect();
    println!("octave bands, ours - original: {}", bands.join(", "));
    println!(
        "tempo, ours - original: {:+.3} % (over {} pieces of 10 s)",
        report.tempo_percent, report.tempo_pieces
    );
    println!("pitch, ours - original: {:+.0} cents", report.pitch_cents);
    println!(
        "stereo balance (left - right), ours - original: largest {:+.2} dB (over {} pieces of 10 s)",
        report.balance_db, report.balance_pieces
    );
    let mut failures = Vec::new();
    if report.overlap_seconds < min_overlap as f64 {
        failures.push(format!("overlap below {min_overlap} s"));
    }
    if report.loudness_median_db > LOUDNESS_MEDIAN_DB {
        failures.push(format!(
            "median loudness difference above {LOUDNESS_MEDIAN_DB} dB"
        ));
    }
    if report.loudness_max_db > LOUDNESS_MAX_DB {
        failures.push(format!(
            "largest loudness difference above {LOUDNESS_MAX_DB} dB"
        ));
    }
    for (hz, difference) in &report.bands {
        if difference.abs() > BAND_DB {
            failures.push(format!("{hz:.0} Hz band off by more than {BAND_DB} dB"));
        }
    }
    // A measure that could not be taken fails: passing it would hide the difference it exists
    // to find.
    if report.tempo_pieces < audio_compare::MIN_TEMPO_PIECES {
        failures.push(format!(
            "tempo measured on fewer than {} pieces of 10 s",
            audio_compare::MIN_TEMPO_PIECES
        ));
    } else if report.tempo_percent.abs() > TEMPO_PERCENT {
        failures.push(format!("tempo off by more than {TEMPO_PERCENT} %"));
    }
    if report.pitch_cents.abs() > PITCH_CENTS {
        failures.push(format!("pitch off by more than {PITCH_CENTS} cents"));
    }
    if report.balance_pieces == 0 {
        failures.push("stereo balance not measured (a mono or silent file)".to_owned());
    } else if report.balance_db.abs() > BALANCE_DB {
        failures.push(format!("stereo balance off by more than {BALANCE_DB} dB"));
    }
    if failures.is_empty() {
        println!("result: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("result: FAIL ({})", failures.join("; "));
        Ok(ExitCode::FAILURE)
    }
}

/// `[3, 4, 5, 9]` as `3-5, 9`.
fn ranges(ticks: &[u64]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0;
    for index in 1..=ticks.len() {
        if index == ticks.len() || ticks[index] != ticks[index - 1] + 1 {
            parts.push(if index - 1 == start {
                ticks[start].to_string()
            } else {
                format!("{}-{}", ticks[start], ticks[index - 1])
            });
            start = index;
        }
    }
    parts.join(", ")
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

    #[test]
    fn a_key_name_the_scenarios_do_not_use_is_an_error_that_lists_the_known_ones() {
        // A typo would otherwise press nothing, and a shot taken after it would match the
        // wrong frame.
        let error = parse(&args(&[
            "render", "--tick", "3", "--key-at", "2:enetr", "--out", "a.png",
        ]))
        .unwrap_err();
        assert!(error.contains("unknown key enetr"), "{error}");
        assert!(error.contains("escape"), "{error}");
    }

    #[test]
    fn parses_the_asset_commands() {
        assert_eq!(
            parse(&args(&["dump-assets"])),
            Ok(Command::DumpAssets {
                data: None,
                out: PathBuf::from("dumps")
            })
        );
        assert_eq!(
            parse(&args(&[
                "render", "--tick", "300", "--key-at", "10", "--key-at", "20:Down", "--out",
                "a.png"
            ])),
            Ok(Command::Render {
                data: None,
                tick: 300,
                keys: vec![(10, Key::Space), (20, Key::Down)],
                out: PathBuf::from("a.png")
            })
        );
        assert_eq!(
            parse(&args(&["compare", "a.png", "b.png"])),
            Ok(Command::Compare {
                a: PathBuf::from("a.png"),
                b: PathBuf::from("b.png")
            })
        );
        assert_eq!(
            parse(&args(&["find", "--data", "/x", "a.png", "b.png"])),
            Ok(Command::Find {
                data: Some(PathBuf::from("/x")),
                keys: vec![],
                ticks: FIND_TICKS,
                shots: vec![PathBuf::from("a.png"), PathBuf::from("b.png")]
            })
        );
    }

    #[test]
    fn rejects_incomplete_asset_commands() {
        for bad in [
            &["render", "--out", "a.png"][..],
            &["render", "--tick", "3"],
            &["render", "--tick", "3", "--out", "a.png", "extra.png"],
            &["compare", "a.png"],
            &["compare", "a.png", "b.png", "c.png"],
            &["find"],
            &["find", "--ticks", "many", "a.png"],
            &["dump-assets", "--tick", "3"],
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn tick_lists_are_printed_as_ranges() {
        assert_eq!(ranges(&[3, 4, 5, 9]), "3-5, 9");
        assert_eq!(ranges(&[7]), "7");
        assert_eq!(ranges(&[1, 3]), "1, 3");
    }
}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass (267 passed, 17 ignored).

- [ ] **Step 5: Write the scenarios**

<!-- write: scripts/reference/menu-transition.scenario -->
```text
# The title to the main menu (spec M2a section 5): a shot every 50 ms from the title's fade-in
# through the fade to black and the menu's fade-in. No keys. Compare with
#   deadrally-headless find --ticks 7400 captures/menu-transition/*.png
at 86500 shot t86500
at 86550 shot t86550
at 86600 shot t86600
at 86650 shot t86650
at 86700 shot t86700
at 86750 shot t86750
at 86800 shot t86800
at 86850 shot t86850
at 86900 shot t86900
at 86950 shot t86950
at 87000 shot t87000
at 87050 shot t87050
at 87100 shot t87100
at 87150 shot t87150
at 87200 shot t87200
at 87250 shot t87250
at 87300 shot t87300
at 87350 shot t87350
at 87400 shot t87400
at 87450 shot t87450
at 87500 shot t87500
at 87550 shot t87550
at 87600 shot t87600
at 87650 shot t87650
at 87700 shot t87700
at 87750 shot t87750
at 87800 shot t87800
at 87850 shot t87850
at 87900 shot t87900
at 87950 shot t87950
at 88000 shot t88000
at 88050 shot t88050
at 88100 shot t88100
at 88150 shot t88150
at 88200 shot t88200
at 88250 shot t88250
at 88300 shot t88300
at 88350 shot t88350
at 88400 shot t88400
at 88450 shot t88450
at 88500 shot t88500
```

<!-- write: scripts/reference/menu-explore.scenario -->
```text
# The idle main menu (spec M2a section 5): its pulsing highlights and creeping background rows
# over 13 s. No keys. Compare with
#   deadrally-headless find --ticks 7400 captures/menu-explore/*.png
at 88000 shot t88000
at 88250 shot t88250
at 88500 shot t88500
at 88750 shot t88750
at 89000 shot t89000
at 89250 shot t89250
at 89500 shot t89500
at 89750 shot t89750
at 90000 shot t90000
at 90250 shot t90250
at 90500 shot t90500
at 90750 shot t90750
at 91000 shot t91000
at 91250 shot t91250
at 91500 shot t91500
at 91750 shot t91750
at 92000 shot t92000
at 92250 shot t92250
at 92500 shot t92500
at 92750 shot t92750
at 93000 shot t93000
at 95000 shot t95000
at 95500 shot t95500
at 96000 shot t96000
at 96500 shot t96500
at 97000 shot t97000
at 97500 shot t97500
at 98000 shot t98000
at 98500 shot t98500
at 99000 shot t99000
at 99500 shot t99500
at 100000 shot t100000
at 100500 shot t100500
at 101000 shot t101000
```

<!-- write: scripts/reference/menu-keys.scenario -->
```text
# The main menu with keys (spec M2a section 5): moving the highlight (wrapping, skipping the
# inactive row), Escape, the start submenu, the exit question, the credits and the end screen.
# Run it without and with --sound. Compare with `deadrally-headless find --ticks 9100` and a
# --key-at for every key, at the tick where the original read it; docs/verification/m2a.md
# says how run.log gives those ticks. With --sound, render-audio --startup with the same keys
# and compare-audio --min-overlap 115.
at 89500 shot idle
at 90000 key Down
at 90500 shot after-90000
at 91000 key Down
at 91500 shot after-91000
at 92000 key Up
at 92500 shot after-92000
at 93000 key Up
at 93500 shot after-93000
at 94000 key Down
at 94500 shot after-94000
at 95000 key Escape
at 95500 shot after-95000
at 96000 key Up
at 96500 shot after-96000
at 97000 key Up
at 97500 shot after-97000
at 98000 key Up
at 98500 shot after-98000
at 99000 key Up
at 99500 shot after-99000
at 100000 key Return
at 100500 shot after-100000
at 101000 key Down
at 101500 shot after-101000
at 102000 key Down
at 102500 shot after-102000
at 103000 key Return
at 103500 shot after-103000
at 104000 key Return
at 104500 shot after-104000
at 105000 key Escape
at 105500 shot after-105000
at 106000 key Escape
at 106500 shot after-106000
at 107000 key Return
at 107500 shot after-107000
at 108000 key Left
at 108500 shot after-108000
at 109000 key Right
at 109500 shot after-109000
at 110000 key Escape
at 110500 shot after-110000
at 111000 key Up
at 111500 shot after-111000
at 112000 key Return
at 112200 shot credits-112200
at 112400 shot credits-112400
at 112700 shot credits-112700
at 113000 shot credits-113000
at 113500 shot credits-113500
at 114000 key space
at 114200 shot credits-114200
at 114500 shot credits-114500
at 114800 shot credits-114800
at 115200 shot credits-115200
at 116000 key space
at 116200 shot credits-116200
at 116500 shot credits-116500
at 116900 shot credits-116900
at 117500 shot credits-117500
at 118000 key Down
at 118500 shot after-118000
at 119000 key Return
at 119500 shot after-119000
at 120000 key Left
at 120500 shot after-120000
at 121000 key Return
at 121200 shot end-121200
at 121500 shot end-121500
at 121800 shot end-121800
at 122200 shot end-122200
at 123000 shot end-123000
at 124000 shot end-124000
at 125000 key space
at 125100 shot end-125100
at 125200 shot end-125200
```

- [ ] **Step 6: Screenshots of the original**

Run:
```bash
scripts/reference-run.sh scripts/reference/menu-transition.scenario captures/menu-transition
scripts/reference-run.sh scripts/reference/menu-explore.scenario captures/menu-explore
scripts/reference-run.sh scripts/reference/menu-keys.scenario captures/menu-keys
cargo build --release -p deadrally-headless
target/release/deadrally-headless find --ticks 7400 captures/menu-transition/*.png captures/menu-explore/*.png
```
Expected: `done: 41 shots`, `done: 34 shots`, `done: 47 shots`; `find` prints a `ticks` line for each of the 75 shots and exits 0. The transition's shots fall from tick 5965 to 6320: the title's fade-in to 92 % (up to 6218), about 0.15 s held there while the original loads, the fade to black (to 6245), the menu's fade-in (6246 to 6295).

- [ ] **Step 7: The keys' ticks**

The scenario gives Wine's milliseconds; `find` needs the ticks at which the original read each key. Save this as `/tmp/key-ticks.py`:

```python
#!/usr/bin/env python3
"""Finds the ticks at which the original read a scenario's keys, from a reference run's run.log
and its screenshots: the shot before the first key anchors the run, the shots that match one
tick or one menu pass put it on a line of milliseconds per tick, and a key whose shots still do
not match is moved a tick or two (in a fade the cursor stands on the frame of the pass that
read the key).

usage: key-ticks.py HEADLESS RUN_DIR [TICKS]
Prints the --key-at arguments; the match count goes to stderr."""
import pathlib
import re
import subprocess
import sys

headless, run = sys.argv[1], pathlib.Path(sys.argv[2])
ticks = sys.argv[3] if len(sys.argv) > 3 else "9100"
NAMES = {"Down": "down", "Up": "up", "Return": "enter", "Escape": "escape", "Left": "left",
         "Right": "right", "space": "space", "y": "y", "n": "n"}
shots, keys = {}, []
for line in (run / "run.log").read_text().splitlines():
    m = re.match(r"(\d+) ms: (shot|key) (\S+)", line)
    if m and m.group(2) == "shot":
        shots[m.group(3)] = int(m.group(1))
    elif m:
        keys.append((int(m.group(1)), NAMES[m.group(3)]))


def find(args, names):
    out = subprocess.run([headless, "find", "--ticks", ticks, *args, *(str(run / f"{n}.png") for n in names)],
                         capture_output=True, text=True).stdout
    found = {}
    for line in out.splitlines():
        m = re.match(r".*/([^/]+)\.png: ticks (.*)$", line)
        if m:
            found[m.group(1)] = [tuple(int(x) for x in (part.split("-") * 2)[:2]) for part in m.group(2).split(", ")]
    return found


def arguments(ticks_of_keys):
    return [a for (_, name), tick in zip(keys, ticks_of_keys) for a in ("--key-at", f"{tick}:{name}")]


first_key = min(ms for ms, _ in keys)
anchor = max((name for name, ms in shots.items() if ms < first_key), key=shots.get)
(lo, hi), = find([], [anchor])[anchor]
a, b = (lo + hi) / 2 - shots[anchor] / 14.0, 1 / 14.0
ticks_of_keys = [round(a + b * ms) for ms, _ in keys]
for _ in range(5):
    found = find(arguments(ticks_of_keys), list(shots))
    points = [(shots[n], (r[0][0] + r[0][1]) / 2) for n, r in found.items() if len(r) == 1 and r[0][1] - r[0][0] <= 1]
    mx = sum(x for x, _ in points) / len(points)
    my = sum(y for _, y in points) / len(points)
    b = sum((x - mx) * (y - my) for x, y in points) / sum((x - mx) ** 2 for x, _ in points)
    a = my - b * mx
    new = [round(a + b * ms) for ms, _ in keys]
    if new == ticks_of_keys:
        break
    ticks_of_keys = new
found = find(arguments(ticks_of_keys), list(shots))
matched = sum(1 for n in shots if n in found)
for name in sorted((n for n in shots if n not in found), key=shots.get):
    if name in found:
        continue
    before = max(i for i, (ms, _) in enumerate(keys) if ms < shots[name])
    best = (matched, ticks_of_keys)
    for shift in (1, -1, 2, -2):
        trial = list(ticks_of_keys)
        trial[before] += shift
        count = len(find(arguments(trial), list(shots)))
        if count > best[0]:
            best = (count, trial)
    matched, ticks_of_keys = best
    found = find(arguments(ticks_of_keys), list(shots))
print(f"{1 / b:.3f} ms per tick; {matched} of {len(shots)} shots match", file=sys.stderr)
print(" ".join(arguments(ticks_of_keys)))
```

Run:
```bash
python3 /tmp/key-ticks.py target/release/deadrally-headless captures/menu-keys > captures/menu-keys/keys.args
target/release/deadrally-headless find --ticks 9100 $(cat captures/menu-keys/keys.args) captures/menu-keys/*.png
```
Expected (a minute or two; up to ten when keys must be moved): `47 of 47 shots match` and about 14.0 ms per tick; `find` prints a `ticks` line for every shot and exits 0. The record's run had its keys at the ticks in `crates/headless/tests/menu_run.rs` (`KEYS`); a new run's differ by a few ticks with Wine's timing.

- [ ] **Step 8: The menu's sound**

Save this watcher as `/tmp/guard-real-sink.sh` (`chmod +x`). The runner already refuses to record if the game's stream is not on its null sink; the watcher also stops Wine at once if any stream reaches a real output during the run:

```bash
#!/usr/bin/env bash
# Runs a command (a reference run with --sound) and stops Wine at once if any stream appears on
# a real sink (any sink but a deadrally_ref null sink). Prints LEAK lines; exits with the command's status.
set -uo pipefail
"$@" &
cmd_pid=$!
leaks=0
while kill -0 "$cmd_pid" 2>/dev/null; do
    # A stream on a real sink is a leak; one not yet linked (sink 4294967295) plays nowhere.
    real=$(pactl list short sinks | awk '$2 !~ /^deadrally_ref_/ {print $1}')
    bad=$(pactl list short sink-inputs | awk -v real=" $(echo $real) " 'index(real, " " $2 " ") != 0 {print}')
    if [ -n "$bad" ]; then
        echo "LEAK: stream off the null sink: $bad" >&2; leaks=1
        WINEPREFIX="${XDG_CACHE_HOME:-$HOME/.cache}/deadrally/reference/wineprefix" wineserver -k 2>/dev/null
    fi
    sleep 0.1
done
wait "$cmd_pid"; status=$?
echo "guard: command exit $status, leaks $leaks"
exit "$status"
```

Run:
```bash
/tmp/guard-real-sink.sh scripts/reference-run.sh --sound scripts/reference/menu-keys.scenario captures/menu-keys-sound
python3 /tmp/key-ticks.py target/release/deadrally-headless captures/menu-keys-sound > captures/menu-keys-sound/keys.args
target/release/deadrally-headless render-audio --startup $(cat captures/menu-keys-sound/keys.args) --seconds 128 --out captures/menu-keys-sound/ours.wav
target/release/deadrally-headless compare-audio captures/menu-keys-sound/sound.wav captures/menu-keys-sound/ours.wav --min-overlap 115
```
Expected: `guard: command exit 0, leaks 0`; `47 of 47 shots match`; `result: PASS` with loudness per second within about 0.24 dB (median) and 1.44 dB (largest), balance within 0.1 dB. Without Task 1's fix the largest was 2.08 dB, at 99 s.

- [ ] **Step 9: Pin the run in a manifest**

The frames at the ticks where the record's screenshots matched, and the whole run's sound, with the record's keys. The data tests share their helpers now.

<!-- write: crates/headless/tests/common/mod.rs -->
```rust
//! What the tests against the developer's real game data share: finding the data, hashing,
//! and the committed manifests they check.

use std::path::{Path, PathBuf};

use deadrally_gamedata::{DATA_ENV_VAR, Located, locate};
use sha2::{Digest, Sha256};

/// The data DEADRALLY_DATA points at; fails (never passes silently) when it is unset.
pub fn located() -> Located {
    let dir = match std::env::var_os(DATA_ENV_VAR) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{DATA_ENV_VAR} is not set: point it at your Death Rally data to run `cargo test-data`"
        ),
    };
    locate(Some(&dir), None, None).unwrap_or_else(|error| panic!("{error}"))
}

pub fn hash(samples: &[i16], hasher: &mut Sha256) {
    for sample in samples {
        hasher.update(sample.to_le_bytes());
    }
}

pub fn hex(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Checks `actual` against the committed manifest `name` in tests/, line by line, or rewrites
/// the manifest when DEADRALLY_BLESS is set. `what` names what the manifest pins.
pub fn check_manifest(name: &str, actual: &str, what: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    if std::env::var_os("DEADRALLY_BLESS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    let only_in = |these: &str, those: &str| -> Vec<String> {
        these
            .lines()
            .filter(|line| !those.lines().any(|other| other == *line))
            .map(str::to_owned)
            .collect()
    };
    let (new, gone) = (only_in(actual, &expected), only_in(&expected, actual));
    assert!(
        new.is_empty() && gone.is_empty(),
        "{what} differs from {}:\nnow:\n{}\nin the manifest:\n{}\nIf the change is intended, \
         measure it against the original again and rewrite the manifest with \
         DEADRALLY_BLESS=1 cargo test-data",
        path.display(),
        new.join("\n"),
        gone.join("\n")
    );
}
```

<!-- write: crates/headless/tests/rendered_audio.rs -->
```rust
//! Rendered sound against the developer's real game data. Run with `cargo test-data`; the
//! test reads DEADRALLY_DATA and fails (never passes silently) when it is unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{AUDIO_SAMPLE_RATE, Game, render_effect, render_music};
use deadrally_gamedata::Located;
use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::bpa::Archive;
use deadrally_gamedata::sound::{self, EFFECTS, MUSIC};
use sha2::{Digest, Sha256};

/// Seconds of each module: several patterns, so most of each module's commands play.
const MUSIC_SECONDS: usize = 30;
/// Seconds of each effect: all of most samples, and a few loops of the looped ones.
const EFFECT_SECONDS: usize = 2;
/// Ticks of the startup after the intro: 10 s of the menu music.
const MENU_MUSIC_TICKS: u32 = 714;

/// One line per sound: the startup as the game plays it (the whole intro and the first 10 s of
/// the menu music after it), the start of every module and every effect of every bank.
fn manifest(located: &Located) -> String {
    let mut lines = String::new();
    let assets = Assets::load(&located.validation).unwrap_or_else(|error| panic!("{error}"));
    let ticks = assets
        .intro
        .delays
        .iter()
        .map(|&delay| u32::from(delay))
        .sum::<u32>()
        + MENU_MUSIC_TICKS;
    let mut game = Game::new(assets);
    let mut audio = Vec::new();
    for _ in 0..ticks {
        game.tick();
        game.take_audio(&mut audio);
    }
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  startup {ticks} ticks", hex(hasher)).unwrap();

    let archive = Archive::open(&located.validation.dir.join(sound::ARCHIVE))
        .unwrap_or_else(|error| panic!("{error}"));
    let frames = |seconds: usize| seconds * AUDIO_SAMPLE_RATE as usize;
    for name in MUSIC {
        let module = sound::load_music(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        let mut hasher = Sha256::new();
        hash(&render_music(&module, frames(MUSIC_SECONDS)), &mut hasher);
        writeln!(
            lines,
            "{}  {}/{name} first {MUSIC_SECONDS} s",
            hex(hasher),
            sound::ARCHIVE
        )
        .unwrap();
    }
    for name in EFFECTS {
        let bank = sound::load_effects(&archive, name).unwrap_or_else(|error| panic!("{error}"));
        let mut hasher = Sha256::new();
        for (index, instrument) in bank.instruments.iter().enumerate() {
            if instrument.is_some() {
                let effect = u8::try_from(index + 1).expect("banks have under 256 instruments");
                hash(
                    &render_effect(&bank, effect, frames(EFFECT_SECONDS)),
                    &mut hasher,
                );
            }
        }
        writeln!(
            lines,
            "{}  {}/{name} every effect, {EFFECT_SECONDS} s each",
            hex(hasher),
            sound::ARCHIVE
        )
        .unwrap();
    }
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn rendered_sound_matches_the_committed_manifest() {
    // The manifest was written after the intro and the menu music were measured against the
    // original (docs/verification/m1b.md); a player change must not alter any sound unnoticed.
    check_manifest(
        "rendered-audio.sha256",
        &manifest(&located()),
        "rendered sound",
    );
}
```

<!-- write: crates/headless/tests/menu_run.rs -->
```rust
//! A run through the main menu against the developer's real game data. Run with
//! `cargo test-data`; the test reads DEADRALLY_DATA and fails (never passes silently) when it is
//! unset.

mod common;

use std::fmt::Write as _;

use common::{check_manifest, hash, hex, located};
use deadrally_core::{Game, InputEvent, Key};
use deadrally_gamedata::assets::Assets;
use sha2::{Digest, Sha256};

/// The keys of `scripts/reference/menu-keys.scenario`, at the ticks where the original read
/// them in the run its screenshots come from (docs/verification/m2a.md): the highlight up and
/// down, Escape, the start submenu, the exit question, the credits and the end screen.
const KEYS: [(u64, Key); 30] = [
    (6423, Key::Down),
    (6495, Key::Down),
    (6566, Key::Up),
    (6638, Key::Up),
    (6709, Key::Down),
    (6780, Key::Escape),
    (6852, Key::Up),
    (6923, Key::Up),
    (6995, Key::Up),
    (7066, Key::Up),
    (7137, Key::Enter),
    (7209, Key::Down),
    (7280, Key::Down),
    (7352, Key::Enter),
    (7423, Key::Enter),
    (7495, Key::Escape),
    (7566, Key::Escape),
    (7637, Key::Enter),
    (7709, Key::Left),
    (7780, Key::Right),
    (7852, Key::Escape),
    (7923, Key::Up),
    (7995, Key::Enter),
    (8138, Key::Space),
    (8280, Key::Space),
    (8423, Key::Down),
    (8495, Key::Enter),
    (8566, Key::Left),
    (8638, Key::Enter),
    (8923, Key::Space),
];

/// The frame after each of these ticks equals the scenario's screenshot of that name.
const SHOTS: [(u64, &str); 47] = [
    (6388, "idle"),
    (6459, "after-90000"),
    (6530, "after-91000"),
    (6602, "after-92000"),
    (6673, "after-93000"),
    (6745, "after-94000"),
    (6816, "after-95000"),
    (6888, "after-96000"),
    (6959, "after-97000"),
    (7030, "after-98000"),
    (7102, "after-99000"),
    (7173, "after-100000"),
    (7245, "after-101000"),
    (7316, "after-102000"),
    (7387, "after-103000"),
    (7459, "after-104000"),
    (7530, "after-105000"),
    (7602, "after-106000"),
    (7673, "after-107000"),
    (7745, "after-108000"),
    (7816, "after-109000"),
    (7887, "after-110000"),
    (7959, "after-111000"),
    (8009, "credits-112200"),
    (8023, "credits-112400"),
    (8045, "credits-112700"),
    (8066, "credits-113000"),
    (8101, "credits-113500"),
    (8151, "credits-114200"),
    (8172, "credits-114500"),
    (8194, "credits-114800"),
    (8223, "credits-115200"),
    (8294, "credits-116200"),
    (8315, "credits-116500"),
    (8344, "credits-116900"),
    (8387, "credits-117500"),
    (8458, "after-118000"),
    (8530, "after-119000"),
    (8601, "after-120000"),
    (8651, "end-121200"),
    (8672, "end-121500"),
    (8694, "end-121800"),
    (8723, "end-122200"),
    (8780, "end-123000"),
    (8851, "end-124000"),
    (8929, "end-125100"),
    (8937, "end-125200"),
];

/// The run ends when the game asks to quit, a little after the last screenshot.
const MAX_TICKS: u64 = 9_000;

/// One line per screenshot (the frame's pixels and palette) and one for the whole run's sound.
fn manifest() -> String {
    let assets = Assets::load(&located().validation).unwrap_or_else(|error| panic!("{error}"));
    let mut game = Game::new(assets);
    let mut lines = String::new();
    let mut audio = Vec::new();
    let mut ticks = 0;
    while !game.quit_requested() && ticks < MAX_TICKS {
        for &(_, key) in KEYS.iter().filter(|(at, _)| *at == ticks) {
            for pressed in [true, false] {
                game.input(InputEvent::Key { key, pressed });
            }
        }
        game.tick();
        game.take_audio(&mut audio);
        ticks += 1;
        for &(_, name) in SHOTS.iter().filter(|(at, _)| *at == ticks) {
            let frame = game.frame();
            let mut hasher = Sha256::new();
            hasher.update(frame.pixels);
            hasher.update(frame.palette.as_flattened());
            writeln!(lines, "{}  frame after tick {ticks} ({name})", hex(hasher)).unwrap();
        }
    }
    assert!(game.quit_requested(), "the end screen asks to quit");
    let mut hasher = Sha256::new();
    hash(&audio, &mut hasher);
    writeln!(lines, "{}  sound of {ticks} ticks", hex(hasher)).unwrap();
    lines
}

#[test]
#[ignore = "needs game data (DEADRALLY_DATA)"]
fn the_menu_run_matches_the_committed_manifest() {
    // The manifest was written after every screenshot of the run equalled our frame at its
    // tick and the sound was measured against the recording (docs/verification/m2a.md); a
    // change to the menu must not alter a frame or a sound unnoticed.
    check_manifest("menu-run.sha256", &manifest(), "the menu run");
}
```

<!-- write: crates/headless/tests/menu-run.sha256 -->
```text
4091f209f53873669b575594a55d8d529b10866af3ee09636b78ec5fb5a88ffc  frame after tick 6388 (idle)
ef966cc32439295fcf34bde1850e533f2b2331afcb5b4b66294e826d4f6a7170  frame after tick 6459 (after-90000)
b07819bd8cc55c1455860c54fda520cd63189ed5ea94e25ef3540346fd0c1922  frame after tick 6530 (after-91000)
01854e40d80c07270e3b36b08631e784fba8000ff6c5f391fb88e477c0bcbe67  frame after tick 6602 (after-92000)
fadd7683d2dae99861e80bf9833ff0846a5aa4310ea81d1aa5d620dbf20ed905  frame after tick 6673 (after-93000)
ad9d61b963b4eaf3d80613c0ee31b9110a8d28252b86446a18d697cd8b841bb0  frame after tick 6745 (after-94000)
1acf2ce5e6d30b7a34c913b6b323d424c3009889d91f3cd70038275dd28e3df0  frame after tick 6816 (after-95000)
640e0c16c8320386d735e39a61d8e41312857669532a5b5cbeb87468761f1748  frame after tick 6888 (after-96000)
d0bc27dc83207189d402fc7d5dcbd795d5345f5653a99dd652034571c7a33a2f  frame after tick 6959 (after-97000)
1b773537df59b122953371584cf0656af186c1570054b96e6f219941a8ed737c  frame after tick 7030 (after-98000)
80331e091c0b5e52b3b66090b4ec77185a3764ef6d5c7fd1b270056370d786c2  frame after tick 7102 (after-99000)
6ab169852c9f953a15f6226d3debce997d080b10f822c4c77ac20e8d6ba8fd2a  frame after tick 7173 (after-100000)
e3b9bfb4b1a8ff509e3f0b9259df34f002f942b8a9450978c4c3e0dc8748f543  frame after tick 7245 (after-101000)
7427840326c71b49d052b2529f92aaaa443cc669432ba3823a7e8bcf8195a6ab  frame after tick 7316 (after-102000)
98c8743c58e7a29dfaa3badccab4858367d7c4f91864f4cdc4011e8f9367caa6  frame after tick 7387 (after-103000)
b281e2a4327828e06a87ecf3bd12b5612f485236ea722d3dc3552bd26084e982  frame after tick 7459 (after-104000)
6fe7e6b60bbcb94b3dac25e04781d1d620ffa71f5c3d07069e1a1dcda1d2f5c2  frame after tick 7530 (after-105000)
21e48b7d57ebc8978a701c4b4c1d0ae0273a9f201bf80a1b7889bf02c2efab73  frame after tick 7602 (after-106000)
063c6d3b0d544d0f1a267a871027d22c15f4c9366aed7eafa1458811c8d7bb15  frame after tick 7673 (after-107000)
509018dbbbfbdc8d8edbde8eb9e29dfc525348fd9f1f4875a97592bb61325656  frame after tick 7745 (after-108000)
498021ccc73ab7ce1916176a2b985af5ef44fc13fd33bbc833118df6daa5d9f3  frame after tick 7816 (after-109000)
da71965e7e30f057b691e737f52dc8b256be2850de82cfe4cffcee08b7a79613  frame after tick 7887 (after-110000)
40574faea663331b128c485a76dd5c2dc74dd96d10d8750268209bae1105c27b  frame after tick 7959 (after-111000)
1094740e3da60c169fa7adc5e6627eee0db2ed5d487ab088df55c96fd1cafd7d  frame after tick 8009 (credits-112200)
c9e67afd45e2624dd96a5c3a453c32fceb16dae70edb972e87c2a1d208409957  frame after tick 8023 (credits-112400)
c1b7ac8185dcc0380f523de59e5c1c94dd3b67f342019b0478bc5a7f9b9c2b59  frame after tick 8045 (credits-112700)
50eaf6ac5362b18de4da2b0e4746ed8bfbc067bdc83bbb0c11f38221ff3b5a6a  frame after tick 8066 (credits-113000)
d9f3fbfdc473cc63fb9b00bdc358d6418fd100c31056eb50219d2ab74a5aa125  frame after tick 8101 (credits-113500)
77be5323a9e7279a229bc2d88a080af933c6dc62a4a8388021036c7d9f4c33e8  frame after tick 8151 (credits-114200)
7a845707162c6342184e5ae6a486403355935a68e3be435a41020a0899637a09  frame after tick 8172 (credits-114500)
d41c68a51b922ed2651b140016d0c6f8ceec2d06b602af0c4e7903137101513e  frame after tick 8194 (credits-114800)
d41c68a51b922ed2651b140016d0c6f8ceec2d06b602af0c4e7903137101513e  frame after tick 8223 (credits-115200)
378bcbe5a034c323d022631d5a7a52eb546552b973620ede5b5eba6b65b14fcc  frame after tick 8294 (credits-116200)
16244e24809f55ca1685095f59bd896bec67fdd362a402db9eb234bcb383b803  frame after tick 8315 (credits-116500)
a32a59d003d866c5db1b92e1df2e2285df0438c415715e8a56e111b0e9e04883  frame after tick 8344 (credits-116900)
2c172d19dd1d4d7c11439cffe8c242ef4c9d53b2cf5bb7ef03fcf19cab7d2d5b  frame after tick 8387 (credits-117500)
e92184dbc4786883202185c916cfec3ef6a76b8125fe6e86b591e0047394cb76  frame after tick 8458 (after-118000)
d9be32a0bb2c4c5cb56b83bbc079c729d923760a09cc2ea563f90fffb94e7f7e  frame after tick 8530 (after-119000)
657f69ff1e9000129fef6473e8543f8020596f52347303726a8ef39fcfecc5d1  frame after tick 8601 (after-120000)
0986c35e10dd394700a7db11cbbafba71f50a41219867fa4f74d403334c62fa4  frame after tick 8651 (end-121200)
5057bd4d15b516a7f32cc5e436b46d0348c1f3aa02dd117222c6bc2904ee0fe1  frame after tick 8672 (end-121500)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8694 (end-121800)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8723 (end-122200)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8780 (end-123000)
21c81ecc39788dc1048ecfd30a9ecca9f18c65c77a6428eb6d29e944e35aa8ed  frame after tick 8851 (end-124000)
1e2e2786cf91022acf1426e0a82dd36ea556c2aff152f43c2674427746bb8fd6  frame after tick 8929 (end-125100)
ded37fa17b599c118051f025dab24ce4e618ac7ebd2cebce5b92b66a2e4c8a4e  frame after tick 8937 (end-125200)
58bfe20cc23819042164f1355af7939fec741aa26581c123a212279ed2af9120  sound of 8950 ticks
```

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (17 passed), among them `the_menu_run_matches_the_committed_manifest`. It cannot fail first: it pins what Task 5 made and Steps 6 to 8 checked. To see that it can fail, change `END_HOLD_TICKS` in `crates/core/src/menu/mod.rs` to 500, run `cargo test --release -p deadrally-headless --test menu_run -- --ignored` (FAIL: the last frames and the sound differ), and undo it.

- [ ] **Step 10: Commit**

```bash
git add crates/headless scripts/reference
git commit -m "feat: follow the menus in headless runs" -m "- --key-at names its key
- scenarios of the original's menus
- a manifest of a run through the menus"
```

---

### Task 7: The record and the documentation

**Files:**
- Create: `docs/verification/m2a.md`
- Modify: `docs/superpowers/specs/2026-10-05-m2a-main-menu-design.md`, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`

- [ ] **Step 1: Write the record and the documentation**

The record holds what Task 6's runs showed, from the run of 2026-10-05. If your runs gave other numbers, write yours into its Results and Runs. The spec gains what the runs measured (spec status line).

<!-- write: docs/verification/m2a.md -->
````markdown
# M2a: verification against the original

The checks of the M2a spec (section 5), run against the original `dr.exe` under Wine. Screenshots and recordings stay under `captures/` and are never committed; this file keeps only what they showed.

## Setup

- **Original:** `dr.exe` of the Steam release, SHA-256 `54fe789faca583d67b8e73e7c58908f3f1468c5c8f75942239a60483ae9be58c`, with the known data release `check-data` reports. A fresh copy each run, so a fresh `dr.cfg`: music 50 %, effects 75 %, no joystick.
- **Runner:** `scripts/reference-run.sh` as in [M1a](m1a.md) and [M1b](m1b.md): `dr.exe -window -nogl` on its own Xvfb display; with `--sound` the game plays into a PulseAudio null sink that `parec` records. The sound runs also had a watcher that stopped Wine at once if any stream appeared on a real output; none did.
- **Pictures:** `deadrally-headless find` renders every tick the way the original's `-nogl` window shows it and reports the ticks that equal a screenshot exactly (every pixel, every channel). For scenarios with keys it presses them with `--key-at` at the ticks where the original read them (see [Key ticks](#key-ticks)).
- **Sound:** `deadrally-headless render-audio --startup` with the same keys, against the recording, with `compare-audio` as in M1b.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| Title to menu | `menu-transition` | **Pass.** All 41 shots, 50 ms apart from the title's fade-in through the fade to black and the menu's fade-in, equal one of our frames. |
| Text and popups | `menu-keys`, `menu-explore` | **Pass.** The main menu, the start submenu, the exit question and the bottom panel are pixel-identical; so are 34 shots of the idle menu over 13 s, with its pulsing highlights and creeping background rows. |
| Navigation | `menu-keys` | **Pass.** Every shot after a key equals our frame: Down and Up wrapping and skipping the inactive row, Escape to the last row, the submenu opened and closed, the exit question's two sides and its Escape. |
| Credits, end screen | `menu-keys` | **Pass.** Every shot through both credits screens, their fades and the end screen's fade-in, hold and fade-out equals our frame. |
| Sound | `menu-keys`, recorded | **Pass.** Over the whole run: loudness per second within 0.24 dB (median) and 1.44 dB (largest), bands within 0.9 dB, tempo +0.052 % over 12 pieces, pitch +0 cents, balance +0.10 dB. Over the menu (86 to 122 s), every second is within 0.33 dB; each key's effect peaks within 1.2 dB of the recording's. |

All 47 shots of `menu-keys` match in both runs of it, with and without sound, and every shot's tick lies within about a tick of where its capture time puts it.

## The game itself

`find` and `render-audio` run the same `Game::tick`, `Game::frame` and `Game::take_audio` as the game. The game binary was also run on its own Xvfb display with SDL's disk audio driver, the sound servers and ALSA unreachable, as in M1b: keys skipped the intro and both logos, then Escape, Enter, Left and Enter answered the exit question with yes, and a key ended the end screen. The game exited by itself with status 0. No stream appeared on the sound server, checked twice a second.

## What the original does that the spec did not say

Found by the checks above and now part of the spec (sections 2 and 3) and the code:

- **The original loads the menu for about 0.15 s** with the title at 92 %, then drops the first steps of the fade to black: the shot after the hold already shows 68 %. DeadRally loads nothing there (spec decision 5) and shows the pending 96 % step, a flash to 100 % and every step; those three frames, 42 ms in all, are the only ones of the transition that the original does not show.
- **The exit question's words reach the screen a pass later:** a move between "yes" and "no" redraws them in the screen buffer only, and the next pass copies them with the cursor's box. The shots catch the words a pass behind the cursor.
- **A key during a credits screen's fade-in** is read before the screen's first wait and moves on at once.
- **The end screen's fade-out lowers the music with it,** through the volume mask that `dr.cfg`'s volumes are multiplied by.
- **Effects sound about 150 ms after their key** in the recording, against the tick in DeadRally: the effects stream's latency, seen in M1b too. Every key's effect has the same loudness in both.
- **A note on an empty sample slot stops the channel.** The first recording held a stray tone for 0.8 s at 99 s, in M1b's recording of the startup too (its largest loudness difference, 2.55 dB). The menu music's order 47 starts notes of a sample slot its author emptied; FMOD plays them as silence, and our player had let the channel's previous looping note sound on at their volume. With the fix, M1b's recording compares within 1.65 dB (largest) and 0.10 dB balance, against 2.55 dB and 0.20 dB before.
- **The intro's effects that are still fading when it ends fade at the menu's effects volume.** The rendered-sound manifest's startup line changed for this alone: 276 samples, 3 ms at the intro's last tick.

## Not measured

- **The gamepad.** Wine gives the original no joystick here. DeadRally follows the disassembly of `eventDetected` (spec section 3.3); `crates/core/src/keys.rs` and `crates/core/tests/menu.rs` test it.
- **Key repeat.** The scenarios press and release keys. The repeat follows SDL 1.2.13's algorithm as the original sets it up (spec section 3.3).

## Scenarios

- `scripts/reference/menu-transition.scenario`: no keys; a shot every 50 ms from 86.5 to 88.5 s, the title's fade-in to the menu's.
- `scripts/reference/menu-explore.scenario`: no keys; the idle menu from 88 to 101 s.
- `scripts/reference/menu-keys.scenario`: from 90 s, a key every second or two through the main menu, the submenu, the exit question (answered no), the credits and the exit question again (answered yes), then the end screen and a key that ends it; a shot after every key and through every fade. Run twice: without sound and with `--sound`.

## Key ticks

A scenario's times are Wine's milliseconds, not ticks. `run.log` gives the time of every key and shot; the shots of the idle menu after each key match two ticks each, which puts the run on a line of about 14 ms a tick (13.999 in the first run, 14.004 in the sound run, at most 1.3 ticks off it). The keys go on that line. In a fade the cursor stands still on the frame of the pass that read the key, so the fade shots pin the pass exactly: in the sound run the credits, the second credits screen and the end screen's keys had to move one tick later than the line put them.

## The manifest

`crates/headless/tests/menu-run.sha256` holds the hashes of our frames at the ticks where the first `menu-keys` run's screenshots matched, and of the whole run's sound, with the keys at that run's ticks. `cargo test-data` checks it (`crates/headless/tests/menu_run.rs`).

## Runs

The output this record is based on. Tick numbers move by a few ticks from run to run with Wine's timing.

### menu-transition

```
$ deadrally-headless find --ticks 7400 captures/menu-transition/*.png
t86500.png: ticks 5976, 6182
t86650.png: ticks 5965, 6193
t86700.png: ticks 6197, 6243
t86950.png: ticks 6214, 6226
t87000.png: ticks 6218, 6222
t87150.png: ticks 6218, 6222
t87200.png: ticks 6212, 6228
t87400.png: ticks 6198, 6242
t87450.png: ticks 5732-5733, 5963-5964, 6194-6195, 6245-6246
t87500.png: ticks 6249
t88150.png: ticks 6295
t88200.png: ticks 6299-6300
t88500.png: ticks 6319-6320
```

(13 of the 41 lines.) A fade shows each level twice, going in and coming out, so most shots match two ticks. The title fades in up to tick 6218 (92 %); 87.00 to 87.15 s hold it while the original loads; from 87.20 s the fade to black (to tick 6245) and the menu's fade-in (ticks 6246 to 6295); the menu is steady from 88.20 s.

### menu-explore

All 34 shots match, each at two ticks (a menu pass) or two pairs a pulse period apart, for example:

```
$ deadrally-headless find --ticks 7400 captures/menu-explore/*.png
t88000.png: ticks 6307-6308
t92750.png: ticks 6647-6648, 6747-6748
t101000.png: ticks 7235-7236
```

### menu-keys

```
$ deadrally-headless find --ticks 9100 --key-at 6423:down --key-at 6495:down --key-at 6566:up --key-at 6638:up --key-at 6709:down --key-at 6780:escape --key-at 6852:up --key-at 6923:up --key-at 6995:up --key-at 7066:up --key-at 7137:enter --key-at 7209:down --key-at 7280:down --key-at 7352:enter --key-at 7423:enter --key-at 7495:escape --key-at 7566:escape --key-at 7637:enter --key-at 7709:left --key-at 7780:right --key-at 7852:escape --key-at 7923:up --key-at 7995:enter --key-at 8138:space --key-at 8280:space --key-at 8423:down --key-at 8495:enter --key-at 8566:left --key-at 8638:enter --key-at 8923:space captures/menu-keys/*.png
idle.png: ticks 6387-6388
after-90000.png: ticks 6459-6460
after-91000.png: ticks 6529-6530
after-92000.png: ticks 6601-6602
after-93000.png: ticks 6673-6674
after-94000.png: ticks 6745-6746
after-95000.png: ticks 6815-6816
after-96000.png: ticks 6887-6888
after-97000.png: ticks 6959-6960
after-98000.png: ticks 7029-7030
after-99000.png: ticks 7101-7102
after-100000.png: ticks 7173-7174
after-101000.png: ticks 7245-7246
after-102000.png: ticks 7315-7316
after-103000.png: ticks 7387-7388
after-104000.png: ticks 7459-7460
after-105000.png: ticks 7529-7530
after-106000.png: ticks 7601-7602
after-107000.png: ticks 7673-7674
after-108000.png: ticks 7745-7746
after-109000.png: ticks 7815-7816
after-110000.png: ticks 7887-7888
after-111000.png: ticks 7959-7960
credits-112200.png: ticks 8009
credits-112400.png: ticks 8023
credits-112700.png: ticks 8045
credits-113000.png: ticks 8066, 8148
credits-113500.png: ticks 8073-8139, 8141
credits-114200.png: ticks 8063, 8151
credits-114500.png: ticks 8172, 8301
credits-114800.png: ticks 8190-8281, 8283
credits-115200.png: ticks 8190-8281, 8283
credits-116200.png: ticks 8179, 8294
credits-116500.png: ticks 8315
credits-116900.png: ticks 8344
credits-117500.png: ticks 8387-8388
after-118000.png: ticks 8457-8458
after-119000.png: ticks 8529-8530
after-120000.png: ticks 8601-8602
end-121200.png: ticks 8651
end-121500.png: ticks 8672, 8944
end-121800.png: ticks 8690-8924, 8926
end-122200.png: ticks 8690-8924, 8926
end-123000.png: ticks 8690-8924, 8926
end-124000.png: ticks 8690-8924, 8926
end-125100.png: ticks 8687, 8929
end-125200.png: ticks 8679, 8937
```

### menu-keys with sound

The second run's keys, at its own ticks; its 47 shots all match too.

```
$ deadrally-headless render-audio --startup --key-at 6432:down --key-at 6504:down --key-at 6575:up --key-at 6646:up --key-at 6718:down --key-at 6789:escape --key-at 6861:up --key-at 6932:up --key-at 7004:up --key-at 7075:up --key-at 7147:enter --key-at 7218:down --key-at 7290:down --key-at 7361:enter --key-at 7433:enter --key-at 7504:escape --key-at 7575:escape --key-at 7647:enter --key-at 7718:left --key-at 7790:right --key-at 7861:escape --key-at 7933:up --key-at 8005:enter --key-at 8148:space --key-at 8290:space --key-at 8433:down --key-at 8505:enter --key-at 8576:left --key-at 8648:enter --key-at 8933:space --seconds 128 --out captures/menu-keys-sound/ours.wav
$ deadrally-headless compare-audio captures/menu-keys-sound/sound.wav captures/menu-keys-sound/ours.wav --min-overlap 115
lag: 2020 ms (envelope correlation 0.942)
overlap: 123.5 s
loudness per second, ours - original: median 0.24 dB, largest 1.44 dB
octave bands, ours - original: 63 Hz +0.9 dB, 125 Hz +0.4 dB, 250 Hz +0.1 dB, 500 Hz +0.1 dB, 1000 Hz +0.2 dB, 2000 Hz -0.0 dB, 4000 Hz +0.1 dB, 8000 Hz +0.4 dB
tempo, ours - original: +0.052 % (over 12 pieces of 10 s)
pitch, ours - original: +0 cents
stereo balance (left - right), ours - original: largest +0.10 dB (over 12 pieces of 10 s)
result: PASS
```

Before the fix of empty sample slots, the same comparison gave 2.08 dB (largest) and +0.23 dB balance; the menu's seconds were within 2.09 dB, all but one within 0.6 dB.

M1b's recording of the startup, against the current render:

```
$ deadrally-headless render-audio --startup --seconds 122 --out captures/startup-sound/ours.wav
$ deadrally-headless compare-audio captures/startup-sound/sound.wav captures/startup-sound/ours.wav --min-overlap 115
lag: 2020 ms (envelope correlation 0.937)
overlap: 120.2 s
loudness per second, ours - original: median 0.28 dB, largest 1.65 dB
octave bands, ours - original: 63 Hz +0.7 dB, 125 Hz +0.4 dB, 250 Hz +0.2 dB, 500 Hz +0.2 dB, 1000 Hz +0.2 dB, 2000 Hz +0.1 dB, 4000 Hz +0.2 dB, 8000 Hz +0.5 dB
tempo, ours - original: +0.059 % (over 12 pieces of 10 s)
pitch, ours - original: +0 cents
stereo balance (left - right), ours - original: largest +0.10 dB (over 12 pieces of 10 s)
result: PASS
```
````

<!-- write: docs/superpowers/specs/2026-10-05-m2a-main-menu-design.md -->
```markdown
# M2a — Text and the main menu: design

- **Date:** 2026-10-05
- **Status:** written without the owner, who asked for the work to go on from M1b into M2 without questions (2026-10-04). Section 2 lists every decision taken on their behalf. Section 3 has been checked against the original under Wine; [docs/verification/m2a.md](../../verification/m2a.md) records the runs, and section 3 now holds what they measured.
- **Scope:** the first half of milestone M2 in [PROJECT_BRIEF.md](../../PROJECT_BRIEF.md) §7. M2b (Configure, Define Keyboard, Define Gamepad, Hall of Fame and `dr.cfg`) gets its own spec.
- **Builds on:** [M1a](2026-10-04-m1a-assets-design.md) (the picture catalogue, the startup sequence, `find`), [M1b](2026-10-04-m1b-sound-design.md) (the players, the menu music).

## 1. Goal

When M2a is done:

- **The title screen gives way to the main menu as in the original:** the fade to black, the menu fading in, its animated palette and cursor, the bottom message panel, the menu music going on.
- **The main menu works with the keyboard and a gamepad:** moving the highlight, the start submenu, the exit question and the end screen, the credits, each with the original's sounds.
- **Text renders glyph for glyph:** the original's fonts, metrics and strings, read from the player's own data, never copied into DeadRally.
- **Each screen is shown to match the original pixel for pixel,** and the menu's sounds to sound the same, measured against the original under Wine as in M1.

## 2. Decisions taken on the owner's behalf

Each with what it costs if it is wrong.

1. **M2 is split** like M1: **M2a** is text and the main menu; **M2b** is Configure, Define Keyboard, Define Gamepad, Hall of Fame and `dr.cfg`. Until M2b, those two main-menu items do nothing. *Cost if wrong:* none to the result; a later merge for the rest of M2.
2. **The original's strings and font metrics are read from the player's `dr.exe`,** the only place they exist (there is no text file). `dr.exe` joins the required data files and the known release; it is read, never run. DeadRally ships no game text. *Cost if wrong:* DeadRally needs the Windows version's executable among the data, which the Steam release has; a release without it (the DOS one) cannot show menus until another source is found.
3. **Items that lead to M3 do nothing yet:** the start submenu opens, highlights and closes, but its entries (new game, load and the rest) wait for M3. *Cost if wrong:* none; a player sees a menu whose entries do not respond.
4. **Volumes are the original's defaults** (music 50 %, effects 75 %) until M2b reads and writes `dr.cfg`. *Cost if wrong:* as in M1b.
5. **Loading takes no time:** where the original loads graphics between screens and its clock runs on, DeadRally loads everything at start-up and moves straight on, as M1a did for the logos. *Measured:* the original holds the title at 92 % for about 0.15 s while it loads the menu, then drops the first steps of the fade to black; DeadRally shows the pending 96 % step, the 100 % flash and every step. *Cost if wrong:* three title frames that differ from the original's, 42 ms in all.
6. **The game can end:** the exit item's "yes" shows the end screen and then asks the frontend to quit, as the original exits. *Cost if wrong:* none.
7. **The gamepad works from the start:** the original reads the joystick only when `dr.cfg` enables it, and a fresh `dr.cfg` does not; DeadRally reads it as if enabled until M2b reads `dr.cfg`. *Cost if wrong:* a pad steers the menus where the original's needs configuring first.

## 3. Facts about the original

From the Windows `dr.exe` (addresses below are its functions), read through DreeRally (`ui/menu.c`, `ui/util/popup.c`, `asset/imageUtil.c`, `graphics.c`, `dr.c`; read-only) and checked in the disassembly where DreeRally is known to differ (its `doc/FINDINGS.md`, `doc/KNOWN-ISSUES.md`).

### 3.1 Text

- **Strings live in `dr.exe` only,** NUL-terminated, at fixed addresses of the known release. M2a needs: the menu text table (9 menus × 9 rows × 50 bytes at 0x446368; a row may hold byte 0xFA, drawn as a 1-pixel gap), the four bottom-panel start-up lines, the exit question, and "yes" and "no".
- **Fonts** are `MENU.BPA` images cut into glyph cells, glyph *k* for character `k + 32`, colour 0 transparent; the colours, outlines and shading are in the pictures:

| Font | Cell | Glyphs | Use |
|---|---|---|---|
| `F-BIG3A` | 32 × 32 | 96 | selected item, popup text |
| `F-BIG3B` | 32 × 32 | 96 | active item |
| `F-BIG3D` | 32 × 32 | 96 | dim item, unfocused menu |
| `F-SMA3A`, `-B`, `-C` | 16 × 16 | 96 | captions, panel text (three colours) |
| `F-MED1A` | 9 × 12 | 62 | Hall of Fame (M2b) |

- **Metrics** are byte tables in `dr.exe` (`{width, height, advance[96]}`): big at 0x445848, small at 0x4458B0, medium at 0x445928. The advance of character *c* is `table[c − 30]` (the two-byte header, then glyph `c − 32`).
- **Drawing** (`drawTextWithFont`, 0x41A2D0): each byte draws its glyph transparently at the pen and advances it; 0xFA advances 1 pixel without drawing; no clipping, no colour parameter.

### 3.2 From the title to the main menu (`mainMenu`, 0x43A020)

1. The menu music has been playing since the intro ended (M1b). The title has faded in to 92 % (M1a).
2. The graphics load (instant here, decision 5); `transitionToBlack` (0x427300) fades the title from 100 % to 0 % over 26 ticks; its first frame shows the pending 96 % step (M1a).
3. The menu palette is set up (`loadPaletteMenu` 0x419EA0, `sub_418B00`, `sub_4224E0` 0x4224E0): `MENU.PAL`, the player colour's ramp (entries 64–95), the copper ramp (176–182), the background copper rows (192–223) and the pulsing entries (16–31).
4. `MENUBG5` is drawn whole; the bottom panel is framed (`drawTransparentBlock(0, 371, 639, 109)`: background restored, `CHATLIN1` at rows 372 and 471) and its lines drawn (`drawBottomMenuText`, 0x41E810); the main menu's popup and items are drawn (`drawMenu`, 0x41A880).
5. The palette fades in from 0 % to 98 % in steps of 2 over 50 ticks, the cursor turning every other tick. It stays at 98 %: a component of 63 shows as 62 (measured).

### 3.3 The main menu

- **Layout** (`dr.exe` 0x4456F0, 7 values per menu: count, x, y, row height, width, height, selection): main menu 6 rows at (145, 124), 28 high, 349 × 192; start submenu 6 rows at (109, 171), 421 × 192.
- **Active rows** (0x4457F0): main menu rows 0, 2, 3, 4, 5 (row 1, multiplayer, is always inactive); start submenu rows 0, 3, 5.
- **Popup** (`createPopup`, 0x41A530): fill colour 196 over rows Y+2..Y+H−7 and columns X+2..X+W−5; 32 × 20 corners from `CORN3A` (focused) or `CORN3B`; border lines in colour 7 (focused) or 4.
- **Rows** (`drawMenu`): text at (X+32, Y+5+28i); the selected row in `F-BIG3A` with the 20 × 20 cursor (`CURSOR`, 50 frames) at (X+9, Y+11+28i) when the menu has focus; other active rows in `F-BIG3B` (focused menu) or `F-BIG3D`; inactive rows in `F-BIG3D`.
- **The loop** (`readEventInMenu`, 0x42E0B0): every pass takes two ticks, turns the cursor one frame and reads the one remembered key (M1a's `eventDetected`).
  - Up and Down move the highlight, wrapping and skipping inactive rows (`refreshMenuUp` 0x41AF40, `refreshMenuDown` 0x41B1A0), with effect 25.
  - Enter, Space and keypad Enter choose the row, with effect 28.
  - Escape in the main menu moves the highlight to the last row (exit) with effect 25 if it is not there already; in a submenu it closes it with effect 22.
- **Keys repeat** while held, as SDL 1.2's `SDL_EnableKeyRepeat(500, 30)` set up by the original. SDL checks the delay when the game polls, once a tick: the first repeat comes at the 39th poll (546 ms), then every third (42 ms, since 28 ms is not more than 30).
- **A gamepad** steers the menus through `eventDetected` (0x417EB0), which polls it: the stick as the keypad's arrows (codes 0xC8, 0xD0, 0xCB, 0xCD, past ±50 of 128), buttons 1 and 3 as Enter, 2 and 4 as Escape. A fresh push (the previous call less than 400 ms ago, the stick released before) acts at once and holds the repeat off for 700 ms; after that the code comes back on every call, every pass of a menu. A push the game has not polled for 400 ms or more repeats at once. (From the disassembly; Wine gives the original no gamepad to measure.)
- **Items:** 0 opens the start submenu (`startRacingMenu`, 0x439CD0: the main menu dims, the submenu gets focus); 1 is inactive; 2 and 3 wait for M2b; 4 shows the credits (`showCredits`, 0x4274E0); 5 asks whether to exit.
- **Exit question:** the main menu dims; a popup (170, 200, 300 × 80) with the question in `F-SMA3A` at (253, 208); "yes" and "no" in big letters (`drawYesNoMenu`, 0x42E310), "no" first selected, the cursor beside the selected word. Left/Y and Right/N move, with effect 25 when the side changes; the words reach the screen a pass later, with the cursor's box. Enter answers the selected side, Escape answers "no", both with effect 28.
- **End screen** (`showEndScreen`): the menu fades to black (26 ticks), `END.BMP` fades in (25 ticks to 96 %) and holds for 560 ticks or until a key; its fade-out (26 ticks) lowers the music and effects with it, the volume mask going from 65500 down in steps of 2620 (`>> 8`). Then the game ends.
- **Credits** (`showCredits`): the menu fades out from 100 % (51 ticks), then each of `CREDIT1` and `CREDIT2` fades in (25 ticks), waits for a key and fades out (26 ticks); a key pressed during a fade-in is read before the screen's first wait, so it moves on at once. The menu comes back as it was and fades in over 50 ticks.
- **The palette moves while the menu waits:** entries 16–31 pulse every tick (100 % down to 49 % and back over 34 ticks, `sub_4220D0` 0x4220D0); the background copper rows step every 70 ticks (0x42A570).

### 3.4 The bottom message panel

- 22 rows of up to 150 bytes, each with a font (small A, B or C); new lines push the old ones up (`bottomMenuText` 0x462000, fonts 0x461EC0).
- At start-up it holds four lines from `dr.exe` and one empty line, in small B, in rows 17–21.
- `drawBottomMenuText` restores rows 380–468 from `MENUBG5` and draws rows 16–21 at (12, 378 + 15k). It is redrawn only when a screen is rebuilt.

### 3.5 Sound

- `MEN-SAM` is loaded with `MEN-MUS` when the intro ends; its effects play on channel 1 at the configured effects volume and pitch `0x28000` (`loadMenuSoundEffect`, 0x43C380).
- At the default 75 % the volume reaches the sound twice: as its volume byte (`(volume · 64 >> 16) + 16`) and as the effects stream's volume (`255 · 192 >> 8 = 191` of 255). Measured: every key's effect is within 1.3 dB of the recording. The original's effects sound about 150 ms after the key (its effects stream's latency, M1b §3.4); DeadRally plays them on the tick.
- The intro's effects that are still fading when it ends fade at the menu's effects volume, which the original sets as the intro ends.
- **Found by the menu's recording, in M1b's player:** FMOD plays a note whose sample slot is empty as silence, so the channel's previous note stops. The menu music's order 47 starts such notes on two channels; ignoring them let a looping note come back at the new note's volume, a stray tone of 0.8 s.

## 4. Architecture

### 4.1 `deadrally-gamedata`

- **`exe`:** reads a PE file's sections and gives the bytes and NUL-terminated strings at virtual addresses.
- **`text`:** the strings and font metrics M2a needs, by address, checked to be printable and terminated; an error names the address.
- **Required files:** `DR.EXE` joins `REQUIRED_FILES` and the known release (size and SHA-256 of the Steam executable).
- **`Assets`** gains the menu's pictures (fonts, `CORN3A`/`B`, `CURSOR`, `MENUBG5`, `CHATLIN1`, `CREDIT1`/`2`), palettes (`MENU.PAL`, `COPPER.PAL`, `BGCOP.PAL`, the credits'), `END.BMP`, `MEN-SAM` and the texts. `END.BMP` must be 640 × 480, the menu's whole screen; the catalogue fixes every other picture's size.

### 4.2 `deadrally-core`

- **`canvas`:** an indexed picture with the original's operations: copy a region, blit with colour 0 transparent, fill a rectangle.
- **`font`:** a font's glyphs and advances; draw a byte string. Nothing in M2a measures text; M2b adds that if its screens need it.
- **`menu`:** the main menu scene: the fade from the title, the palette, the popups, the cursor, navigation, the submenu, the exit question, the end screen and the credits; the bottom panel.
- **`keys`:** the one remembered key of the original (scancodes), key repeat, and the gamepad's mapping, shared by the startup and the menus.
- **`Game`:** the startup hands over to the menu scene when the title is done; `Game::quit_requested()` tells the frontend the player chose to exit.
- **Sound:** `Sound` loads `MEN-SAM` when the intro ends and triggers effects at a given volume and pitch; a new bank fades the old one's effects out.

### 4.3 Frontend and headless

- The frontend shows 640 × 480 frames as it shows the startup's, and quits when the game asks.
- `find`, `render` and `render-audio --startup` run on through the menus with `--key-at` keys; `find` gains the keys a scenario presses (arrows, Enter, Escape, Y, N).

## 5. Verification against the original

| Check | How | Passes when |
|---|---|---|
| Title to menu | a scenario with shots through the fade to black, the menu's fade-in and its idle palette | every shot equals one of our frames exactly (`find`) |
| Text and popups | shots of the main menu, the start submenu and the exit question | equal exactly |
| Navigation | keys moving the highlight, wrapping, Escape to the last row | each shot after a key equals one of our frames |
| Credits, end screen | shots through both | equal exactly |
| Sound | a recording of the menu while keys move the highlight, against `render-audio --startup` with the same keys | `compare-audio` as in M1b over the menu part |

Records go to `docs/verification/m2a.md`; shots and recordings stay under `captures/`.

## 6. Tests

- **Without data:** PE sections and strings on a synthetic executable; text rendering (advances, the 0xFA gap, transparency); popups and rows on a synthetic canvas; navigation (wrapping, skipping inactive rows, Escape to the last row); key repeat timing; the fade and palette steps; the exit question; quitting.
- **With data:** `DR.EXE` is the known release; every string M2a reads is printable and terminated; the menu assets load; a manifest (hashes) of our frames at the ticks where the screenshots of a run through the menus matched, and of that run's sound, written after both were checked against the original.

## 7. Done criteria

1. Merged to `master` with CI green on the merge commit.
2. `cargo test` and `cargo test-data` pass.
3. Every check of section 5 passes and is recorded in `docs/verification/m2a.md`.
4. The game shows the main menu after the title and quits from it.

## 8. Risks

| Risk | Mitigation |
|---|---|
| DreeRally's menu code has known errors (popup fill width, palette loops that run once, the key switch) | each is checked in the disassembly; the screenshots decide |
| Palette animation makes screenshots time-dependent | `find` searches every tick, as for the fades in M1a |
| Key timing under Wine moves a scenario's keys by a few ticks | the shots compare pictures, not times; `find` reports where each falls |
| An unknown `dr.exe` has its strings elsewhere | texts are checked when loaded; an error names the address |
```

<!-- write: README.md -->
````markdown
# DeadRally

Original Death Rally reincarnation for modern systems: a clean, native, 64-bit reimplementation of *Death Rally for Windows* (Remedy, 2009) for Windows, macOS and Linux, written in Rust.

**Status:** M2a, the main menu. The game starts like the original: the intro with its music and effects, the Apogee and Remedy logos, the title screen, then the main menu with its credits and its exit, under the menu music. Nothing else is playable yet. [docs/PROJECT_BRIEF.md](docs/PROJECT_BRIEF.md) covers the goal, the approach and the roadmap.

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
````

<!-- write: CONTRIBUTING.md -->
````markdown
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
````

<!-- write: CLAUDE.md -->
```markdown
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
| `DEADRALLY_BLESS=1 cargo test-data` | rewrite the manifests `crates/gamedata/tests/decoded-images.sha256`, `crates/headless/tests/rendered-audio.sha256` and `crates/headless/tests/menu-run.sha256`, only after checking the pictures and the sound against the original again |
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
```

- [ ] **Step 2: Run every check**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && scripts/check-no-game-data.sh && DEADRALLY_DATA=~/games/DeathRally cargo test-data`
Expected: all pass (267 passed, 17 ignored; data 17 passed).

- [ ] **Step 3: Commit**

```bash
git add docs README.md CONTRIBUTING.md CLAUDE.md
git commit -m "docs: record the M2a verification"
```

- [ ] **Step 4: Final review and fixes**

Follow superpowers:executing-plans, Final Review: a fresh reviewer on the whole branch (`git merge-base master HEAD` to `HEAD`) with this plan's Review Focus, then one fix pass for Critical and Important findings, each with a test that failed first.

- [ ] **Step 5: Push and wait for CI**

```bash
git push -u origin m2a-menu
```
Expected: every CI job green on the branch's last commit (there is no `gh` on the build machine; read the run through `https://api.github.com/repos/victortrnka/DeadRally/actions/runs?branch=m2a-menu`).

- [ ] **Step 6: Merge**

The owner asked for the merge once everything is green.

```bash
git checkout master
git pull --ff-only
git merge --no-ff m2a-menu -m "feat: merge M2a main menu"
cargo test --workspace && DEADRALLY_DATA=~/games/DeathRally cargo test-data
git push origin master
```
Expected: tests pass on the merged tree; CI green on the merge commit. Then delete the local branch (`git branch -d m2a-menu`); keep the remote one.
