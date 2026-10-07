# Installation details

## What the install scripts do

Both scripts make these changes in the game folder:

| File                | What happens                                                |
| ------------------- | ----------------------------------------------------------- |
| `SDL_orig.dll`      | The game's original `SDL.dll`, renamed (first install only) |
| `SDL.dll`           | Replaced with the proxy from this project                   |
| `sanctuary-pad.ini` | Default bindings, copied only if the file is not there yet  |
| `sanctuary-terminal.cmd` / `.sh` | Launcher for [terminal mode](terminal-mode.md) (Windows / Linux) |

Neither script builds anything: they install the prebuilt `dist/sanctuary_pad.dll` that
is checked into the repository. To build it yourself, see [Contributing](contributing.md).

Verifying the game files in Steam, or a game update, puts the original `SDL.dll` back.
Run the install script again afterwards.

## Game in a different folder

Both scripts search your Steam libraries, including ones on other drives or an SD card.
If the game is not found, or you want a different copy, point them at the folder:

```powershell
.\install.ps1 -GameDir "D:\Steam\steamapps\common\SanctuaryRPG - Black Edition"
```

```bash
bash install.sh --game-dir "/path/to/steamapps/common/SanctuaryRPG - Black Edition"
```

## PowerShell refuses to run the script

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

## Manual install

1. In the game folder, rename `SDL.dll` to `SDL_orig.dll`.
2. Copy `dist/sanctuary_pad.dll` into the game folder as `SDL.dll`.
3. Copy `sanctuary-pad.ini` into the game folder.
4. For [terminal mode](terminal-mode.md), copy `sanctuary-terminal.cmd` (Windows) or
   `sanctuary-terminal.sh` (Linux) into the game folder.

## Linux / Steam Deck details

- On the Deck, run the script in Desktop Mode (Konsole).
- Steam tends to give games without controller support a keyboard-and-mouse layout. With
  that the proxy never sees a controller, which is why the layout has to be changed to
  the Gamepad template.
- Typing text, such as a character name, needs the on-screen keyboard (Steam + X).
- If `sanctuary-pad.log` does not appear in the game folder after starting the game, the
  proxy is not being loaded. Set the game's launch options to
  `WINEDLLOVERRIDES="SDL=n,b" %command%`.
- To update, `git pull` and run `bash install.sh` again.
- [Terminal mode](terminal-mode.md) needs a terminal, so it is for Desktop Mode on the
  Deck. The game has to have been started from Steam once before.

## Uninstalling

```powershell
.\install.ps1 -Uninstall
```

```bash
bash install.sh --uninstall
```

This restores the original `SDL.dll`. You can delete `sanctuary-pad.ini` and
`sanctuary-pad.log` from the game folder afterwards.

## Troubleshooting

- **The game does not start.** `SDL_orig.dll` is missing from the game folder. Run
  `.\install.ps1 -Uninstall`, or verify the game files in Steam, then install again.
- **The controller does nothing.** Check `sanctuary-pad.log` in the game folder. It
  should say `controller 0 connected`. If it does not, the pad is not visible to XInput.
  If the log file does not exist at all, the proxy is not installed.
- **Double inputs.** Steam Input is also translating the controller. Disable it for this
  game under Properties → Controller.
- **A screen is not navigable, or the wrong thing is highlighted.** Set
  `dump_screen = true` in `sanctuary-pad.ini`, reproduce the problem and look at
  `sanctuary-pad-screen.txt` in the game folder. It records the screen text, the choices
  that were detected and every key that was sent.
