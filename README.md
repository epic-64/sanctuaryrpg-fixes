# sanctuary-pad

Controller support for **SanctuaryRPG: Black Edition**, which has none of its own.

It is a replacement `SDL.dll` that passes everything through to the game's real SDL and
turns XInput controller input into the key presses the game expects. It also reads the
text on screen, so menus can be navigated with a highlight instead of memorising keys.

## Requirements

- Windows, with the Steam version of SanctuaryRPG: Black Edition
- An XInput controller (Xbox-style, or anything that emulates one)
- [Rust](https://rustup.rs) with the MSVC toolchain and the 32-bit target (the game is a
  32-bit executable):

  ```powershell
  rustup target add i686-pc-windows-msvc
  ```

## Install

Close the game, then from this folder:

```powershell
.\install.ps1
```

If the game is not in the default location, pass its folder:

```powershell
.\install.ps1 -GameDir "D:\Steam\steamapps\common\SanctuaryRPG - Black Edition"
```

If PowerShell refuses to run the script, use:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The script builds the DLL and makes three changes in the game folder:

| File                | What happens                                            |
| ------------------- | ------------------------------------------------------- |
| `SDL_orig.dll`      | The game's original `SDL.dll`, renamed (first install only) |
| `SDL.dll`           | Replaced with the proxy built from this project         |
| `sanctuary-pad.ini` | Default bindings, copied only if the file is not there yet |

Start the game as usual, through Steam or directly.

### Manual install

1. `cargo build --release`
2. In the game folder, rename `SDL.dll` to `SDL_orig.dll`.
3. Copy `target\i686-pc-windows-msvc\release\sanctuary_pad.dll` into the game folder as `SDL.dll`.
4. Copy `sanctuary-pad.ini` into the game folder.

## Linux / Steam Deck (Proton)

This setup has not been tested on a Deck. Nothing in the proxy is known to conflict with
Proton, and the steps below are what it needs.

The DLL cannot be built on Linux, so the repository carries a prebuilt copy in
`dist/sanctuary_pad.dll`. On Windows, `install.ps1` refreshes it on every build; commit
it whenever the code changes.

1. Clone the repository.
2. Close the game.
3. In a terminal (Desktop Mode → Konsole on the Deck), from the cloned folder:

   ```bash
   bash install.sh
   ```

   The script finds the game in your Steam libraries, including ones on an SD card,
   keeps the game's `SDL.dll` as `SDL_orig.dll`, installs the proxy as `SDL.dll` and
   copies `sanctuary-pad.ini` if the game folder has none yet. If it cannot find the
   game, pass the folder:

   ```bash
   bash install.sh --game-dir "/path/to/steamapps/common/SanctuaryRPG - Black Edition"
   ```

4. Set the controller layout to a gamepad. Steam tends to give games without controller
   support a keyboard-and-mouse layout, and with that the proxy never sees a controller.
   Open the game's controller settings and choose the **Gamepad** template.

5. Start the game.

To update, `git pull` and run `bash install.sh` again. To remove the proxy, run
`bash install.sh --uninstall`.

Notes:

- Typing text, such as a character name, needs the on-screen keyboard (Steam + X).
- A game update or verifying the game files restores the original `SDL.dll`. Run
  `bash install.sh` again afterwards.
- If the controller does nothing, look at `sanctuary-pad.log` in the game folder:
  - No log file: the proxy is not being loaded. Set the game's launch options to
    `WINEDLLOVERRIDES="SDL=n,b" %command%`.
  - A log file without `controller 0 connected`: the layout is not a gamepad one (step 4).

## Update

After changing the code, close the game and run `.\install.ps1` again. Your
`sanctuary-pad.ini` in the game folder is kept.

## Uninstall

```powershell
.\install.ps1 -Uninstall
```

This restores the original `SDL.dll`. You can delete `sanctuary-pad.ini` and
`sanctuary-pad.log` from the game folder afterwards.

Verifying the game files in Steam also removes the proxy, because Steam replaces
`SDL.dll` with the original. Run `.\install.ps1` again afterwards.

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
