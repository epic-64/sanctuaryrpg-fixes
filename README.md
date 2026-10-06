# sanctuary-fixes

Native controller support for **SanctuaryRPG: Black Edition**.

It is a replacement `SDL.dll` that passes everything through to the game's real SDL and
turns XInput controller input into the key presses the game expects. It also reads the
text on screen, so menus can be navigated with a highlight instead of memorising keys,
on the controller or with the arrow keys.

And it can run the game inside a terminal
instead of its own window, see [Terminal mode](#terminal-mode).

## Install

Clone this repository and close the game. For controller support you need an XInput
controller (Xbox-style, or anything that emulates one).

**Windows**:

```powershell
.\install.ps1
```

**Linux / Steam Deck**:

```bash
bash install.sh
```

Then set the game's controller layout in Steam to the standard **Gamepad** template.

To remove it again, run `.\install.ps1 -Uninstall` or `bash install.sh --uninstall`.

If something does not work, see [Troubleshooting](#troubleshooting) and
[Advanced installation](#advanced-installation).

## Controls

| Controller         | Keyboard      | Action                               |
| ------------------ | ------------- | ------------------------------------ |
| D-pad / left stick | Arrow keys    | Move the highlight between choices   |
| A                  | Space / Enter | Pick the highlighted choice, continue |
| B                  | Escape        | Go back, or erase a character        |

That is all the game needs. Typing text, such as a character name, needs the keyboard.
For the other buttons and the fine print, see [Advanced controls](#advanced-controls).

## Terminal mode

The game can also be played inside a terminal, such as Windows Terminal, GNOME Terminal
or Konsole. Run the launcher from the game folder in the console you want it in.

**Windows**:

```powershell
& "C:\Program Files (x86)\Steam\steamapps\common\SanctuaryRPG - Black Edition\sanctuary-terminal.cmd"
```

**Linux / Steam Deck** (start the game once from Steam first, so that its Proton prefix
exists):

```bash
~/.local/share/Steam/steamapps/common/"SanctuaryRPG - Black Edition"/sanctuary-terminal.sh
```

The game then opens no window of its own. Its screen is drawn into the console with
[ratatui](https://ratatui.rs), in your terminal's font and with the same colours, and
the console gets its prompt back when the game exits. The keyboard and the controller
work as usual. Started any other way (Steam, a shortcut, the `.exe` itself) the game
uses its normal window.

The console has to be at least as large as the game's screen; if it is not, a message
says how large it has to be. Where the game's background is black, the terminal's own
background shows instead.

On Linux the launcher runs the game with the Proton that Steam uses for it, outside of
Steam, and the game draws straight into the terminal it was started from. Wine's messages
go to `sanctuary-terminal.log` in the game folder. If the wrong Proton is picked, pass the
right one with `--proton "/path/to/Proton - Experimental"`.

## Configuration

Edit `sanctuary-pad.ini` in the game folder and restart the game.

```ini
x = 1                    # <button> = <key>
ls = none                # unbind a button
menu_navigation = true   # set to false for plain button-to-key bindings only (and no keyboard navigation)
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

## Advanced controls

### Menu navigation

The highlight appears on screens that list choices such as `[1] Play`, `[22] Load Holon`,
`<S> Save Game` or `>2< EX Whirlwind`.

- Up/down moves between rows, left/right within a row; both wrap around.
- B / Escape picks Back / Return / Cancel / Leave / Close / No, if the screen offers one.
  Otherwise it sends Backspace.
- On screens without choices the arrow keys are passed to the game unchanged, and
  A / Space sends Enter.
- Space types a space when text has already been typed at the `>` prompt.
- Enter confirms a choice typed at the prompt by hand instead of the highlighted one.
- The keyboard works the same way in the game's window and in
  [terminal mode](#terminal-mode).
- The controller is ignored while the game window is not focused.

### Other buttons

On screens without choices, and for all other buttons, the bindings from
`sanctuary-pad.ini` apply (see [Configuration](#configuration)). The defaults are:

| Button     | Key        |
| ---------- | ---------- |
| D-pad / left stick | Arrow keys |
| A, Start   | Enter      |
| B, Back    | Backspace  |
| X, Y       | 1, 2       |
| LB, RB     | 3, 4       |
| LT, RT     | 5, 6       |

> [!WARNING]
> **Stray input at the `>` prompt**
>
> X, Y, the bumpers and the triggers type `1` to `6`, and those characters can end up in
> the `>` input row at the bottom of the screen by accident.
>
> - **B erases** the last character in the row. This is the safe way to clean it up.
> - **START clears the whole row** by sending Enter: the game rejects the stray text and
>   empties the row. If the text happens to be a valid choice (for example a lone `1`),
>   Enter picks that choice instead, so check what the row says first.
> - Picking a highlighted choice with A erases the row first, then types the choice.

## Advanced installation

### What the install scripts do

Both scripts make these changes in the game folder:

| File                | What happens                                                |
| ------------------- | ----------------------------------------------------------- |
| `SDL_orig.dll`      | The game's original `SDL.dll`, renamed (first install only) |
| `SDL.dll`           | Replaced with the proxy from this project                   |
| `sanctuary-pad.ini` | Default bindings, copied only if the file is not there yet  |
| `sanctuary-terminal.cmd` / `.sh` | Launcher for [terminal mode](#terminal-mode) (Windows / Linux) |

Neither script builds anything: they install the prebuilt `dist/sanctuary_pad.dll` that
is checked into the repository. To build it yourself, see [Development](#development).

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

1. In the game folder, rename `SDL.dll` to `SDL_orig.dll`.
2. Copy `dist/sanctuary_pad.dll` into the game folder as `SDL.dll`.
3. Copy `sanctuary-pad.ini` into the game folder.
4. For [terminal mode](#terminal-mode), copy `sanctuary-terminal.cmd` (Windows) or
   `sanctuary-terminal.sh` (Linux) into the game folder.

### Linux / Steam Deck details

- On the Deck, run the script in Desktop Mode (Konsole).
- Steam tends to give games without controller support a keyboard-and-mouse layout. With
  that the proxy never sees a controller, which is why the layout has to be changed to
  the Gamepad template.
- Typing text, such as a character name, needs the on-screen keyboard (Steam + X).
- If `sanctuary-pad.log` does not appear in the game folder after starting the game, the
  proxy is not being loaded. Set the game's launch options to
  `WINEDLLOVERRIDES="SDL=n,b" %command%`.
- To update, `git pull` and run `bash install.sh` again.
- [Terminal mode](#terminal-mode) needs a terminal, so it is for Desktop Mode on the
  Deck. The game has to have been started from Steam once before.

### Uninstalling

The uninstall option restores the original `SDL.dll`. You can delete `sanctuary-pad.ini`
and `sanctuary-pad.log` from the game folder afterwards.

## Development

Building needs [Rust](https://rustup.rs) and a 32-bit Windows target (the game is a 32-bit
executable). On Windows, with the MSVC toolchain:

```powershell
rustup target add i686-pc-windows-msvc
cargo test               # menu parsing, navigation and terminal key tests
.\install.ps1 -Build     # build, refresh dist/sanctuary_pad.dll and install
```

On Linux, cross-compiled with mingw-w64 (`apt install gcc-mingw-w64-i686` on Debian and
Ubuntu). The tests are Windows binaries too, so they run through Wine; `wine` has to be
on the `PATH`, for example Proton's `files/bin`:

```bash
rustup target add i686-pc-windows-gnu
cargo test --target i686-pc-windows-gnu
bash install.sh --build  # build, refresh dist/sanctuary_pad.dll and install
```

Commit `dist/sanctuary_pad.dll` together with the code change, so that installs without
Rust get the new version. Either toolchain's DLL works in the game.

`probe.ps1` builds and installs the current code, runs the game with scripted button presses (no
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
| `src/terminal.rs` | Terminal mode: draws the console with ratatui (Windows console, or stdout under Wine) and reads its keys |
| `src/hook.rs`   | Import-table patch used to draw the highlight before each frame   |
| `src/config.rs` | `sanctuary-pad.ini` parsing                                       |

## License

[MIT](LICENSE). This is an unofficial mod, not affiliated with the makers of SanctuaryRPG.
