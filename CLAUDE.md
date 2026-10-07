# sanctuary-pad

SDL proxy DLL that adds controller and keyboard menu navigation to SanctuaryRPG.
See `docs/contributing.md` for the source layout.

## Getting a change into the game

`cargo test` only runs the parsing and navigation tests; it does not touch the DLL the
game loads. After any code change, before telling the user to try it in the game:

```powershell
cargo test
.\install.ps1 -Build     # release build, refresh prebuilt/sanctuary_pad.dll, install into the game folder
```

On Linux (mingw-w64 cross build; the tests run through Wine, so Proton's `files/bin` has
to be on the `PATH` for them):

```bash
cargo test --target i686-pc-windows-gnu
bash install.sh --build  # same as install.ps1 -Build
```

- The game must be closed, otherwise `SDL.dll` is locked and the copy fails. It only
  picks up the new DLL on its next start.
- Terminal mode on Linux (`sanctuary-terminal.sh`) can be tested without a controller or a
  real terminal: fork it in a pseudo-terminal from Python, send keys on a timer and read
  `sanctuary-pad-screen.txt`. The game accepts its first Enter about 12 seconds after launch.
- `prebuilt/sanctuary_pad.dll` is checked in and is what the install scripts use, so commit
  it together with the code change.

## Debugging a screen

Set `dump_screen = true` in the game folder's `sanctuary-pad.ini`, reproduce, and read
`sanctuary-pad-screen.txt` there: it records the screen text, the detected choices and
every key sent. `probe.ps1` runs the game with scripted button presses (see `docs/contributing.md`).

## Releasing

`scripts/release.sh` tags `v<VERSION>` and pushes it; GitHub Actions then runs
`scripts/distribute.sh` and uploads the zip as a release. `VERSION` must match the
version in `Cargo.toml`. For a local zip only, run `scripts/distribute.sh` (output in the
gitignored `dist/`).
