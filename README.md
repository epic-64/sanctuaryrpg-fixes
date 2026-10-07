# sanctuary-fixes

Controller support and keyboard menu navigation for **SanctuaryRPG: Black Edition**,
as a drop-in `SDL.dll`. Optionally runs the game inside your terminal.

## Install

Download the latest `sanctuary-fixes-<version>.zip` from
[Releases](https://github.com/epic-64/sanctuaryrpg-fixes/releases), extract it, close
the game and run from the extracted folder:

```powershell
.\install.ps1            # Windows
```

```bash
bash install.sh          # Linux / Steam Deck
```

In Steam, set the game's controller layout to the **Gamepad** template. Done.

## Controls

| Controller         | Keyboard      | Action                     |
| ------------------ | ------------- | -------------------------- |
| D-pad / left stick | Arrow keys    | Move the highlight         |
| A                  | Space / Enter | Pick the highlighted choice |
| B                  | Escape        | Go back                    |

## Terminal mode

Run the launcher that the install put into the game folder, from the terminal you want
the game in:

```powershell
& "C:\Program Files (x86)\Steam\steamapps\common\SanctuaryRPG - Black Edition\sanctuary-terminal.cmd"
```

```bash
~/.local/share/Steam/steamapps/common/"SanctuaryRPG - Black Edition"/sanctuary-terminal.sh
```

On Linux, start the game once from Steam first. Details in [docs/terminal-mode.md](docs/terminal-mode.md).

## More

- [Installation details, uninstalling and troubleshooting](docs/install.md)
- [Controls and configuration](docs/controls.md)
- [Terminal mode](docs/terminal-mode.md)
- [Contributing](docs/contributing.md)

## License

[MIT](LICENSE). This is an unofficial mod, not affiliated with the makers of SanctuaryRPG.
