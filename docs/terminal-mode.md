# Terminal mode

The game can be played inside a terminal, such as Windows Terminal, GNOME Terminal or
Konsole, instead of its own window. Run the launcher from the game folder in the console
you want it in.

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
