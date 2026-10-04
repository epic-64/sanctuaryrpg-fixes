# sanctuary-pad

SDL proxy DLL that adds controller and keyboard menu navigation to SanctuaryRPG.
See `README.md` (Development) for the source layout.

## Getting a change into the game

`cargo test` only runs the parsing and navigation tests; it does not touch the DLL the
game loads. After any code change, before telling the user to try it in the game:

```powershell
cargo test
.\install.ps1 -Build     # release build, refresh dist/sanctuary_pad.dll, install into the game folder
```

- The game must be closed, otherwise `SDL.dll` is locked and the copy fails. It only
  picks up the new DLL on its next start.
- `dist/sanctuary_pad.dll` is checked in and is what the install scripts use, so commit
  it together with the code change.

## Debugging a screen

Set `dump_screen = true` in the game folder's `sanctuary-pad.ini`, reproduce, and read
`sanctuary-pad-screen.txt` there: it records the screen text, the detected choices and
every key sent. `probe.ps1` runs the game with scripted button presses (see README).
