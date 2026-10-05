# M2b — Configure, controls and `dr.cfg`: design

- **Date:** 2026-10-05
- **Status:** written without the owner, who asked for M2 to go on without questions (2026-10-04) and for the usage limit to be spared (2026-10-05). Section 2 lists every decision taken on their behalf; [docs/verification/m2b.md](../../verification/m2b.md) records the checks against the original.
- **Scope:** the Configure branch of the main menu and the original's configuration file. Hall of Fame moves to M2c.
- **Builds on:** [M2a](2026-10-05-m2a-main-menu-design.md) (the menu scene, texts from `dr.exe`, `keys`).

## 1. Goal

- **Configure works as in the original:** the music and effects volume popups with their slider, Define Keyboard, Define Gamepad and the gamepad switch, each screen matching the original pixel for pixel.
- **Settings last:** volumes, keys, gamepad mapping and the gamepad switch are kept in a `dr.cfg` with the original's layout, read at start and written when the original writes it.

## 2. Decisions taken on the owner's behalf

1. **M2 is split once more:** M2b is Configure and `dr.cfg`; **M2c** is the Hall of Fame, which needs the records and the original's default records. *Cost if wrong:* one more merge.
2. **DeadRally keeps its own `dr.cfg`** next to its `config.toml`, never in the game folder (nothing is written there). When it has none yet, it reads the game folder's `dr.cfg` once if there is one, else starts from the original's defaults. The file is byte-compatible with the original's. *Cost if wrong:* settings changed in the original after the first start are not picked up.
3. **The gamepad follows `dr.cfg`:** off until the player switches it on in Configure, as in the original; M2a's decision 7 ends. *Cost if wrong:* a gamepad needs switching on once.
4. **The frontend says whether a gamepad is connected** (a new input event); switching the gamepad on without one shows the original's "not detected" popup. *Cost if wrong:* none.
5. **The file's one random byte** (written by `rand()` and never read) is kept as read, 0 in a new file. *Cost if wrong:* none; nothing reads it.

## 3. Facts about the original

From the Windows `dr.exe` (DreeRally `config.c`, `ui/menu.c`; read-only) and the reference run `menu-configure`.

### 3.1 `dr.cfg`

- 8 bytes of header (three bytes, a 32-bit value, one random byte), then 2934 bytes: music and effects volume (0..=0x10000), a mode, the difficulty, "use joystick", other settings, the circuit records (2592 bytes) and the Hall of Fame (200 bytes), the eight keyboard keys, the seven gamepad inputs and the times played, at the offsets of `loadConfig` (0x426xxx; DreeRally `config.c`).
- A file of 7 bytes or less is replaced by the defaults (`defaultConfig`, 0x426700): music 0x8000, effects 0xC000, difficulty 1, joystick off.
- `mainMenu` reads it, counts the start (`timesPlayed`) and writes it back before the intro; Configure writes it when the player leaves it; the game writes it after the end screen.

### 3.2 Configure (menu 3)

- 6 rows at (95, 146), 485 × 192, all active, over the dimmed main menu: music volume, effect volume, define keyboard, define gamepad, the gamepad switch, previous menu.
- **Volume popups:** the menu dims; popup (214, 218, 330 × 70); the caption in small A; `SLIDMUS2` at (314, 250) and `VOLCUR2` at (329 + v, 250) for a level v of 0..=128 (the volume / 512); the level as a percentage (`v · 0.78125`, rounded down) in big A, right-aligned before x = 309. Left/Right (and the stick) move v by 2, applied to the music or effects at once; Enter keeps it and plays effect 22. One tick per key read.
- **Define Keyboard (menu 6):** 9 rows at (50, 93), 532 × 278: each of the eight controls' name and its key's name, then previous menu. Enter on a control: popup (295, 121 + 28r, 323 × 48) with the prompt in small A; the next key (any but 0xAA) becomes the control's key.
- **Define Gamepad (menu 8):** 8 rows at (50, 113), 532 × 250; the same popup; after 15 polls the stick direction or button 1–4 held, or Enter or Escape for "none". While it waits, key reads leave the gamepad alone (0x456B00), so a button is an input, not Enter or Escape.
- **The gamepad switch:** on → off; off → on if a gamepad is connected, else effect 29 and the popup (28, 198, 595 × 86) "not detected", until a key.
- **Leaving:** Escape returns to the main menu with the Configure row kept; "previous menu" also sets the selection back to its first row. Both write `dr.cfg`.
- The names of keys, controls and gamepad inputs, the prompts and the popups' texts are strings of `dr.exe` at fixed addresses (key names 16 bytes apart from 0x442A70; "unavailable" for keys without a name).

## 4. Architecture

- **`deadrally-gamedata`:** `dr_cfg`: parse and write the file (header kept, payload fields by offset, unknown bytes kept); defaults. `text` gains the Configure texts and key names.
- **`deadrally-core`:** `Game::new` takes the configuration; the menu scene gains the Configure states; `Game::take_config()` hands the bytes to write to the frontend whenever the original writes `dr.cfg`. `InputEvent::PadConnected`.
- **Frontend:** finds DeadRally's `dr.cfg` (or imports the game's), passes it in, writes what the game hands out; reports gamepad connections.
- **Headless:** runs start from the defaults and never write.

## 5. Verification

| Check | How | Passes when |
|---|---|---|
| Configure screens | `menu-configure` scenario: both popups, Define Keyboard with a key changed, Define Gamepad, the switch's popup | every shot equals one of our frames (`find`) |
| `dr.cfg` | the file the original writes after that scenario | ours, after the same keys, is byte-equal but for the random byte |

## 6. Tests

Without data: the file's round trip, defaults, short files; the volume steps and limits; key capture (0xAA ignored); gamepad capture and "none"; the switch with and without a gamepad; leaving by Escape and by "previous menu"; when the configuration is handed out. With data: the texts and key names load; the run's frames in a manifest.
