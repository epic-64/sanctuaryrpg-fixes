# Contributing

## Building

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

## Probing the game

`probe.ps1` builds and installs the current code, runs the game with scripted button
presses (no controller needed) and prints what happened. It needs `dump_screen = true` in
the game folder's ini, and it kills any running copy of the game first.

```powershell
.\probe.ps1 -Buttons "start,start,start,start,-,down,a" -Seconds 20 -Screenshot "$env:TEMP\menu.png"
```

One button is pressed every 1.5 seconds, starting 6 seconds after launch; `-` is a pause.

## Debugging a screen

Set `dump_screen = true` in the game folder's `sanctuary-pad.ini`, reproduce, and read
`sanctuary-pad-screen.txt` there: it records the screen text, the detected choices and
every key sent.

## Source layout

| File              | Purpose                                                           |
| ----------------- | ----------------------------------------------------------------- |
| `build.rs`        | Generates the pass-through stubs for SDL's exports from `sdl_exports.txt` |
| `src/lib.rs`      | `SDL_PollEvent` / `SDL_WaitEvent` hooks and button handling       |
| `src/pad.rs`      | XInput polling                                                    |
| `src/menu.rs`     | Finding on-screen choices, selection and highlight                |
| `src/screen.rs`   | Reading and colouring the game's console through libtcod          |
| `src/terminal.rs` | Terminal mode: draws the console with ratatui (Windows console, or stdout under Wine) and reads its keys |
| `src/hook.rs`     | Import-table patch used to draw the highlight before each frame   |
| `src/config.rs`   | `sanctuary-pad.ini` parsing                                       |
