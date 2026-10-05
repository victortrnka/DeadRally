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

All of it was repeated with fresh runs while the implementation plan was carried out, the same day: the 75 shots of `menu-transition` and `menu-explore` and the 47 of each `menu-keys` run matched again, and the recording compared within 0.27 dB (median) and 1.44 dB (largest), balance +0.11 dB, tempo +0.056 %.

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
