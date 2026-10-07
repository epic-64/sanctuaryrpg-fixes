# Controls and configuration

## Basic controls

| Controller         | Keyboard      | Action                                |
| ------------------ | ------------- | ------------------------------------- |
| D-pad / left stick | Arrow keys    | Move the highlight between choices    |
| A                  | Space / Enter | Pick the highlighted choice, continue |
| B                  | Escape        | Go back, or erase a character         |

Typing text, such as a character name, needs the keyboard.

## Menu navigation

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
  [terminal mode](terminal-mode.md).
- The controller is ignored while the game window is not focused.

## Other buttons

On screens without choices, and for all other buttons, the bindings from
`sanctuary-pad.ini` apply (see [Configuration](#configuration)). The defaults are:

| Button             | Key        |
| ------------------ | ---------- |
| D-pad / left stick | Arrow keys |
| A, Start           | Enter      |
| B, Back            | Backspace  |
| X, Y               | 1, 2       |
| LB, RB             | 3, 4       |
| LT, RT             | 5, 6       |

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

## Configuration

Edit `sanctuary-pad.ini` in the game folder and restart the game.

```ini
x = 1                    # <button> = <key>
ls = none                # unbind a button
menu_navigation = true   # set to false for plain button-to-key bindings only (and no keyboard navigation)
dump_screen = false      # debug logging, see Troubleshooting in install.md
```

- Buttons: `up` `down` `left` `right` `a` `b` `x` `y` `lb` `rb` `lt` `rt` `start` `back` `ls` `rs`
- Keys: any single character, or `enter` `escape` `space` `tab` `backspace` `up` `down` `left` `right` `none`
