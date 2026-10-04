# sanctuary-pad

Controller support for **SanctuaryRPG: Black Edition**, which has none of its own.

It is a replacement `SDL.dll` that passes everything through to the game's real SDL and
turns XInput controller input into the key presses the game expects. It also reads the
text on screen, so menus can be navigated with a highlight instead of memorising keys.

## Install

Clone this repository and close the game. You need an XInput controller (Xbox-style, or
anything that emulates one).

**Windows** (needs [Rust](https://rustup.rs) and `rustup target add i686-pc-windows-msvc`):

```powershell
.\install.ps1
```

**Linux / Steam Deck** (nothing to build; not yet tested on a Deck):

```bash
bash install.sh
```

Then set the game's controller layout in Steam to the **Gamepad** template.

To remove it again, run `.\install.ps1 -Uninstall` or `bash install.sh --uninstall`.

If something does not work, see [Troubleshooting](#troubleshooting) and
[Advanced installation](#advanced-installation).

## Controls

On screens that list choices such as `[1] Play`, `[22] Load Holon`, `<S> Save Game` or `>2< EX Whirlwind`:

| Input                 | Action                                                         |
| --------------------- | -------------------------------------------------------------- |
| D-pad / left stick    | Up/down moves between rows, left/right within a row (both wrap) |
| A                     | Pick the highlighted choice                                    |
| B                     | Pick Back / Return / Cancel / Leave / Close / No, if offered   |

On every other screen, and for all other buttons, the bindings from
`sanctuary-pad.ini` apply. The defaults are:

| Button     | Key        |
| ---------- | ---------- |
| D-pad / left stick | Arrow keys |
| A, Start   | Enter      |
| B, Back    | Escape     |
| X, Y       | 1, 2       |
| LB, RB     | 3, 4       |
| LT, RT     | 5, 6       |

The controller is ignored while the game window is not focused. Typing text, such as a
character name, still needs the keyboard.

## Configuration

Edit `sanctuary-pad.ini` in the game folder and restart the game.

```ini
x = 1                    # <button> = <key>
ls = none                # unbind a button
menu_navigation = true   # set to false for plain button-to-key bindings only
dump_screen = false      # debug logging, see Troubleshooting
```

- Buttons: `up` `down` `left` `right` `a` `b` `x` `y` `lb` `rb` `lt` `rt` `start` `back` `ls` `rs`
- Keys: any single character, or `enter` `escape` `space` `tab` `backspace` `up` `down` `left` `right` `none`

## Troubleshooting

- **The game does not start.** `SDL_orig.dll` is missing from the game folder. Run
  `.\install.ps1 -Uninstall`, or verify the game files in Steam, then install again.
- **The controller does nothing.** Check `sanctuary-pad.log` in the game folder. It
  should say `controller 0 connected`. If it does not, the pad is not visible to XInput.
  If the log file does not exist at all, the proxy is not installed.
- **Double inputs.** Steam Input is also translating the controller. Disable it for this
  game under Properties → Controller.
- **A screen is not navigable, or the wrong thing is highlighted.** Set
  `dump_screen = true`, reproduce the problem and look at `sanctuary-pad-screen.txt` in
  the game folder. It records the screen text, the choices that were detected and every
  key that was sent.

## Advanced installation

### What the install scripts do

Both scripts make the same three changes in the game folder:

| File                | What happens                                                |
| ------------------- | ----------------------------------------------------------- |
| `SDL_orig.dll`      | The game's original `SDL.dll`, renamed (first install only) |
| `SDL.dll`           | Replaced with the proxy from this project                   |
| `sanctuary-pad.ini` | Default bindings, copied only if the file is not there yet  |

`install.ps1` builds the DLL first and also copies it to `dist/sanctuary_pad.dll`.
`install.sh` does not build; it installs that prebuilt copy from `dist/`.

Verifying the game files in Steam, or a game update, puts the original `SDL.dll` back.
Run the install script again afterwards.

### Game in a different folder

Both scripts search your Steam libraries, including ones on other drives or an SD card.
If the game is not found, or you want a different copy, point them at the folder:

```powershell
.\install.ps1 -GameDir "D:\Steam\steamapps\common\SanctuaryRPG - Black Edition"
```

```bash
bash install.sh --game-dir "/path/to/steamapps/common/SanctuaryRPG - Black Edition"
```

### PowerShell refuses to run the script

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

### Manual install

1. `cargo build --release` on Windows, or take `dist/sanctuary_pad.dll`.
2. In the game folder, rename `SDL.dll` to `SDL_orig.dll`.
3. Copy `sanctuary_pad.dll` into the game folder as `SDL.dll`.
4. Copy `sanctuary-pad.ini` into the game folder.

### Linux / Steam Deck details

- On the Deck, run the script in Desktop Mode (Konsole).
- Steam tends to give games without controller support a keyboard-and-mouse layout. With
  that the proxy never sees a controller, which is why the layout has to be changed to
  the Gamepad template.
- Typing text, such as a character name, needs the on-screen keyboard (Steam + X).
- If `sanctuary-pad.log` does not appear in the game folder after starting the game, the
  proxy is not being loaded. Set the game's launch options to
  `WINEDLLOVERRIDES="SDL=n,b" %command%`.
- The DLL cannot be built on Linux. To get a newer build, run `.\install.ps1` on Windows,
  commit `dist/sanctuary_pad.dll`, then `git pull` and `bash install.sh` on Linux.

### Uninstalling

The uninstall option restores the original `SDL.dll`. You can delete `sanctuary-pad.ini`
and `sanctuary-pad.log` from the game folder afterwards.

## Development

```powershell
cargo test    # menu parsing and navigation tests
```

`probe.ps1` installs the current build, runs the game with scripted button presses (no
controller needed) and prints what happened. It needs `dump_screen = true` in the game
folder's ini, and it kills any running copy of the game first.

```powershell
.\probe.ps1 -Buttons "start,start,start,start,-,down,a" -Seconds 20 -Screenshot "$env:TEMP\menu.png"
```

One button is pressed every 1.5 seconds, starting 6 seconds after launch; `-` is a pause.

Source layout:

| File            | Purpose                                                           |
| --------------- | ----------------------------------------------------------------- |
| `build.rs`      | Generates the pass-through stubs for SDL's exports from `sdl_exports.txt` |
| `src/lib.rs`    | `SDL_PollEvent` / `SDL_WaitEvent` hooks and button handling       |
| `src/pad.rs`    | XInput polling                                                    |
| `src/menu.rs`   | Finding on-screen choices, selection and highlight                |
| `src/screen.rs` | Reading and colouring the game's console through libtcod          |
| `src/hook.rs`   | Import-table patch used to draw the highlight before each frame   |
| `src/config.rs` | `sanctuary-pad.ini` parsing                                       |
