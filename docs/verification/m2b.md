# M2b: verification against the original

The checks of the M2b spec (section 5), run against the original `dr.exe` under Wine as in [M2a](m2a.md). Screenshots and the original's `dr.cfg` stay under `captures/` and are never committed: the file holds the original's default names.

## Results

| Check (spec section 5) | Scenario | Result |
|---|---|---|
| Configure screens | `menu-configure` | **Pass.** All 17 shots equal one of our frames: Configure over the main menu, both volume popups before and after keys, Define Keyboard before and after Q became accelerate's key, its prompt, Define Gamepad, the "not detected" popup, Escape back to the main menu and the Configure row kept. |
| `dr.cfg` | `menu-configure` | **Pass.** The file the original wrote at the end equals ours after the same keys in all 2942 bytes but the 8th, the random byte nothing reads. |
| Defaults | the same file | **Pass.** `defaultConfig` run over `dr.exe` gives the original's fresh file: with the run's three changes (music 45 %, effects 76 %, Q for accelerate) and one start counted, every byte but the random one is equal. |

## What the original does that the spec did not say

- **Define Gamepad hides the gamepad from key reads while it waits** (0x456B00, set by 0x42CBF0): otherwise button 1 would read as Enter and button 2 as Escape, both meaning "none".
- **Only the knob's box and the percentage reach the screen** in a volume popup's turn; the slider is redrawn into the buffer only (`showAdjustOptions`; the shots agree).

## The game itself

The game binary, on its own Xvfb display with SDL's disk audio driver and its own configuration directory (`XDG_CONFIG_HOME`), went through Configure (music volume down two steps), back to the main menu and out through the exit question; it wrote `dr.cfg` there with the music at 0x7800 and one start counted, and exited by itself. No stream reached the sound server.

An earlier attempt gave a relative `XDG_CONFIG_HOME`, which the directories library ignores: that run wrote `~/.config/deadrally/dr.cfg` on the build machine. The file was new and was removed at once; the script now passes an absolute path.

## The manifest

`crates/headless/tests/configure-run.sha256` holds our frames at the 17 shots' ticks, the run's sound and the last `dr.cfg` written, whose hash equals the original's file with its random byte set to 0. `menu-run.sha256` gained the last `dr.cfg` of the M2a run.

## Runs

```
$ python3 /tmp/key-ticks.py target/release/deadrally-headless captures/menu-configure 9200
13.994 ms per tick; 17 of 17 shots match
$ deadrally-headless find --ticks 9200 $(cat captures/menu-configure/keys.args) captures/menu-configure/*.png
idle.png: ticks 6383-6384
configure.png: ticks 6527-6528
music.png: ticks 6576-6634
music-left.png: ticks 6678-6743
after-music.png: ticks 6776-6777
effects.png: ticks 6856-6920
effects-right.png: ticks 6926-6993
keyboard.png: ticks 7098-7099
press-key.png: ticks 7136-7207
after-q.png: ticks 7240-7241
after-keyboard.png: ticks 7312-7313
gamepad.png: ticks 7418-7419
after-gamepad.png: ticks 7490-7491
not-detected.png: ticks 7564-7635
after-not-detected.png: ticks 7670-7671
escape.png: ticks 7740-7741
previous.png: ticks 7812-7813
```
