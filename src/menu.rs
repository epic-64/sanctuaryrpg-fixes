//! Screen-aware menu navigation.
//!
//! The game presents choices as `[1] Play`, `[22] Load Holon`, `<S> Save Game`, ... and
//! expects the text between the brackets to be typed. This finds those options in the on-screen
//! text, keeps one of them selected and highlighted, and lets the d-pad move the selection.

use crate::config::{parse_key, Key};
use crate::screen;

/// Background colour of the selected option, as 0xBBGGRR.
const HIGHLIGHT: u32 = 0x82_46_28;

/// Labels that the B button should pick, in order of preference.
const BACK_LABELS: &[&str] = &["back", "return", "cancel", "leave", "close", "no"];

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MenuOption {
    /// The keys that pick this option, in typing order.
    pub keys: Vec<Key>,
    pub x: i32,
    pub y: i32,
    /// Width of the `[k] label` text in cells.
    pub len: i32,
    pub label: String,
}

#[derive(Default)]
pub struct Menu {
    options: Vec<MenuOption>,
    selected: usize,
    /// The option the cursor was last on. It outlives screens without options, so the
    /// cursor returns to the same place when a menu is redrawn after a message.
    remembered: Option<MenuOption>,
    /// The column the cursor tries to stay in while moving up and down.
    column: i32,
    /// What has been typed at the game's `>` prompt, if the screen has one.
    prompt: Option<String>,
    /// Highlighted cells with the background colour they had before.
    painted: Vec<(i32, i32, u32)>,
}

impl Menu {
    pub fn is_active(&self) -> bool {
        !self.options.is_empty()
    }

    pub fn options(&self) -> &[MenuOption] {
        &self.options
    }

    pub fn selected_keys(&self) -> Option<Vec<Key>> {
        self.options.get(self.selected).map(|option| option.keys.clone())
    }

    /// Text already typed at the `>` prompt. `None` means the screen has no prompt and
    /// reacts to single key presses instead of a typed line.
    pub fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }

    /// The keys of an option that backs out of the current screen, if there is one.
    pub fn back_keys(&self) -> Option<Vec<Key>> {
        BACK_LABELS.iter().find_map(|wanted| {
            self.options
                .iter()
                .find(|option| option.label.to_ascii_lowercase().starts_with(wanted))
                .map(|option| option.keys.clone())
        })
    }

    /// Re-reads the screen and makes sure the selection is highlighted.
    /// Returns true if the console was modified and needs to be redrawn.
    pub fn sync(&mut self) -> bool {
        self.update(&screen::read());
        self.paint()
    }

    fn update(&mut self, rows: &[String]) {
        let options = parse(rows);
        self.prompt = prompt_of(rows);
        if options != self.options {
            // Keep the cursor where it was if the same choice is still on offer: either
            // within a menu that changed, or in one that comes back unchanged.
            let current = self.options.get(self.selected);
            let same_place = |option: &&MenuOption, previous: &MenuOption| {
                (&option.keys, option.y) == (&previous.keys, previous.y)
            };
            self.selected = options
                .iter()
                .position(|option| current.is_some_and(|previous| same_place(&option, previous)))
                .or_else(|| options.iter().position(|option| Some(option) == self.remembered.as_ref()))
                .unwrap_or(0);
            self.options = options;
            self.column = self.options.get(self.selected).map_or(0, |option| option.x);
            self.remember();
        }
    }

    /// Moves the selection. Returns true if the console needs to be redrawn.
    ///
    /// Up and down step through the rows that have options, left and right through the
    /// options of the current row. That way every option can be reached no matter how
    /// the screen is laid out.
    pub fn navigate(&mut self, direction: Direction) -> bool {
        let Some(current) = self.options.get(self.selected) else { return false };
        let (x, y) = (current.x, current.y);
        let target = match direction {
            Direction::Up | Direction::Down => {
                let rows = self.options.iter().map(|option| option.y);
                // The nearest row in that direction, wrapping around at the ends.
                let row = match direction {
                    Direction::Down => rows.clone().filter(|&row| row > y).min().or(rows.min()),
                    _ => rows.clone().filter(|&row| row < y).max().or(rows.max()),
                };
                let column = self.column;
                self.options
                    .iter()
                    .enumerate()
                    .filter(|(_, option)| Some(option.y) == row)
                    .min_by_key(|(_, option)| (option.x - column).abs())
            }
            Direction::Left | Direction::Right => {
                let in_row = self.options.iter().enumerate().filter(|(_, option)| option.y == y);
                // The next option on this row, wrapping around at the ends.
                let next = match direction {
                    Direction::Right => in_row
                        .clone()
                        .filter(|(_, option)| option.x > x)
                        .min_by_key(|(_, option)| option.x)
                        .or(in_row.min_by_key(|(_, option)| option.x)),
                    _ => in_row
                        .clone()
                        .filter(|(_, option)| option.x < x)
                        .max_by_key(|(_, option)| option.x)
                        .or(in_row.max_by_key(|(_, option)| option.x)),
                };
                self.column = next.map_or(self.column, |(_, option)| option.x);
                next
            }
        };
        self.selected = target.map_or(self.selected, |(index, _)| index);
        self.remember();
        self.paint()
    }

    fn remember(&mut self) {
        if let Some(option) = self.options.get(self.selected) {
            self.remembered = Some(option.clone());
        }
    }

    fn paint(&mut self) -> bool {
        let wanted: Vec<(i32, i32)> = self
            .options
            .get(self.selected)
            .map(|option| (option.x..option.x + option.len).map(|x| (x, option.y)).collect())
            .unwrap_or_default();

        // The game repaints the console whenever it likes, so check the actual cells.
        let up_to_date = wanted.len() == self.painted.len()
            && wanted.iter().zip(&self.painted).all(|(&(x, y), &(px, py, _))| {
                (x, y) == (px, py) && screen::background(x, y) == HIGHLIGHT
            });
        if up_to_date {
            return false;
        }

        let mut changed = false;
        for (x, y, original) in self.painted.drain(..) {
            if screen::background(x, y) == HIGHLIGHT {
                screen::set_background(x, y, original);
                changed = true;
            }
        }
        for (x, y) in wanted {
            self.painted.push((x, y, screen::background(x, y)));
            screen::set_background(x, y, HIGHLIGHT);
            changed = true;
        }
        changed
    }
}

/// What has been typed at the input prompt: the bottom row of the screen, if it is a
/// `>` followed by at most one word. Message rows such as `> Arrat has HIT you` are not
/// prompts.
fn prompt_of(rows: &[String]) -> Option<String> {
    let bottom = rows.iter().rev().find(|row| !row.trim().is_empty())?;
    let typed = bottom.strip_prefix('>')?;
    (!typed.contains(' ')).then(|| typed.to_string())
}

/// Text of a label starting at `rest`: it runs until a wide gap or the next option.
fn label_of(rest: &str) -> &str {
    let end = [rest.find("  "), rest.find(['[', '<'])].into_iter().flatten().min();
    rest[..end.unwrap_or(rest.len())].trim_end()
}

/// Finds every choice on screen, in reading order. Choices are written as `[key] label`,
/// `<key> label` or `>key< label`. The game also marks list entries in other ways, such
/// as ` 1  -Reposition-` or `]3[ Final Blow`; those are recognised by their position in
/// a list.
fn parse(rows: &[String]) -> Vec<MenuOption> {
    let mut options = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        if is_stat_row(row) {
            continue;
        }
        let mut start = 0;
        while let Some(open) = row[start..].find(['[', '<', '>']).map(|i| i + start) {
            let (closer, square) = match row.as_bytes()[open] {
                b'[' => (']', true),
                b'<' => ('>', false),
                _ => ('<', false),
            };
            start = open + 1;
            let Some(close) = row[start..].find(closer).map(|i| i + start) else { continue };
            let token = &row[start..close];
            let label = label_of(&row[close + 1..]);
            // A bracket glued to the previous one, as in `[ON][40]`, is a read-out.
            let standalone = open == 0 || row.as_bytes()[open - 1] != b']';
            // So is one glued to the next: `[O][::::    ]` is an enemy's distance and
            // health bar.
            if row.as_bytes().get(close + 1) == Some(&b'[') {
                continue;
            }
            let Some(keys) = token_keys(token, label.trim_start(), square, standalone) else {
                continue;
            };
            options.push(MenuOption {
                keys,
                x: open as i32,
                y: y as i32,
                len: (close + 1 - open + label.len()) as i32,
                label: label.trim_start().to_string(),
            });
            start = close + 1;
        }
    }

    // Keys in other decorations only count directly above or below a choice whose key
    // is in the same column, which may itself be one found on an earlier pass.
    loop {
        let found: Vec<MenuOption> = options
            .iter()
            .filter(|option| option.keys.len() == 1)
            .flat_map(|option| [(option.x + 1, option.y - 1), (option.x + 1, option.y + 1)])
            .filter(|&(column, y)| !options.iter().any(|option| option.y == y && option.x + 1 == column))
            .filter_map(|(column, y)| bare_option(rows.get(usize::try_from(y).ok()?)?, column as usize, y))
            .collect();
        if found.is_empty() {
            break;
        }
        for option in found {
            if !options.contains(&option) {
                options.push(option);
            }
        }
    }
    options.sort_by_key(|option| (option.y, option.x));
    options
}

/// Whether the row is a read-out such as `NAME: Bob >> Lvl [9] Vassian Barbarian` or
/// `HP: 1751 / 1751`. Those start with a capitalised field name and a colon, and the
/// brackets in them hold values, not choices.
fn is_stat_row(row: &str) -> bool {
    let first_word = row.trim_start().split(' ').next().unwrap_or("");
    first_word
        .strip_suffix(':')
        .is_some_and(|name| !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase()))
}

/// Reads a list entry with a single key character in the given column, whatever is
/// drawn around it, if that is what the row holds.
fn bare_option(row: &str, column: usize, y: i32) -> Option<MenuOption> {
    let bytes = row.as_bytes();
    let key = *bytes.get(column)? as char;
    let separate = |index: usize| bytes.get(index).is_some_and(|c| !c.is_ascii_alphanumeric());
    let spaced = column > 0 && separate(column - 1) && separate(column + 1);
    let label = label_of(row.get(column + 2..)?.trim_start());
    if !spaced || !key.is_ascii_alphanumeric() || label.is_empty() || is_stat_row(row) {
        return None;
    }
    let label_end = row.find(label)? + label.len();
    Some(MenuOption {
        keys: vec![parse_key(&key.to_ascii_lowercase().to_string())?],
        x: column as i32 - 1,
        y,
        len: (label_end + 1 - column) as i32,
        label: label.to_string(),
    })
}

/// Interprets the text between brackets: a letter or digit, a number, or a named key.
fn token_keys(token: &str, label: &str, square: bool, standalone: bool) -> Option<Vec<Key>> {
    let token = token.to_ascii_lowercase();
    let described = label.starts_with(|c: char| c.is_ascii_alphabetic());
    // Angle brackets also wrap plain values such as `<8>`, so those always need a
    // description to count as a choice.
    if !square && !described {
        return None;
    }
    let all = |test: fn(&char) -> bool| token.chars().all(|c| test(&c));
    if token.len() == 1 && all(char::is_ascii_alphanumeric) {
        Some(vec![parse_key(&token)?])
    } else if token == "?" {
        // `[?] Help` is a choice, but combat draws a bare `[?]` next to unknown enemies.
        described.then(|| parse_key(&token)).flatten().map(|key| vec![key])
    } else if (2..=3).contains(&token.len()) && all(char::is_ascii_digit) {
        // Bracketed numbers are also used for read-outs such as a volume level, so only
        // accept the ones that are followed by a description, or that repeat one digit
        // the way the game numbers its secondary choices (`[77]` drawn inside a picture).
        let repeated = standalone && token.bytes().all(|b| b == token.as_bytes()[0]);
        let digits = token.chars().map(|c| parse_key(c.encode_utf8(&mut [0; 4])));
        (described || repeated).then(|| digits.collect()).flatten()
    } else if all(char::is_ascii_alphabetic) {
        // Named keys such as [ESC] or [ENTER]; anything else is not a key.
        Some(vec![parse_key(&token)?])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(rows: &[&str]) -> Menu {
        let rows: Vec<String> = rows.iter().map(|row| row.to_string()).collect();
        Menu { options: parse(&rows), ..Menu::default() }
    }

    fn typed(keys: &[Key]) -> String {
        keys.iter().map(|key| key.sym as u8 as char).collect()
    }

    /// Each option as (typed text, column, label).
    fn summary(rows: &[&str]) -> Vec<(String, i32, String)> {
        menu(rows).options.into_iter().map(|o| (typed(&o.keys), o.x, o.label)).collect()
    }

    fn option(keys: &str, x: i32, label: &str) -> (String, i32, String) {
        (keys.to_string(), x, label.to_string())
    }

    #[test]
    fn parses_main_menu() {
        let parsed = menu(&["", " [1] Play", " [X] Exit", ">"]).options;
        assert_eq!(parsed.len(), 2);
        assert_eq!((parsed[0].x, parsed[0].y, parsed[0].len), (1, 1, 8));
        assert_eq!((typed(&parsed[1].keys), parsed[1].label.as_str()), ("x".to_string(), "Exit"));
    }

    #[test]
    fn parses_several_options_per_row() {
        assert_eq!(
            summary(&[" [1] Attack   [2] Defend [ESC] Menu"]),
            vec![option("1", 1, "Attack"), option("2", 14, "Defend"), option("\x1b", 25, "Menu")]
        );
    }

    #[test]
    fn parses_character_select() {
        assert_eq!(
            summary(&[
                "  [Page ONE]                                                   [Tutorial Off]",
                " [2] New Paladin    -or- [22] Load Holon [<44>] <Classic> <6h 30m>",
            ]),
            vec![option("2", 1, "New Paladin"), option("22", 25, "Load Holon")]
        );
    }

    #[test]
    fn ignores_brackets_that_are_not_choices() {
        assert_eq!(
            summary(&[
                "HP [#####     ] 50/100",
                " ]_[ A%#V%A",
                " [5] Music                [ON][MIN] <Z][X>",
                " [6] Sound Effects        [ON][40] <C][V>",
            ]),
            vec![option("5", 1, "Music"), option("6", 1, "Sound Effects")]
        );
    }

    #[test]
    fn parses_angle_bracket_choices() {
        assert_eq!(
            summary(&[
                "! [C] Character   ! <S> Save Game",
                "! MP: <69/69>     ! <0> Main Menu",
                " [11] Load Bob [<8>] <Classic> <0h 17m>",
            ]),
            vec![
                option("c", 2, "Character"),
                option("s", 20, "Save Game"),
                option("0", 20, "Main Menu"),
                option("11", 1, "Load Bob"),
            ]
        );
    }

    #[test]
    fn parses_combat_skills() {
        assert_eq!(
            summary(&[
                "> CRITICAL! You have HIT Arrat for 316 damage.",
                " [@][:::::      ] [?] <CRIPPLING>",
                "  HP: 1496 / 1634   ATK: 97 - 162   MP: (69/69)>>>>>>>>>>",
                "    [:::::::::::::::::  ][<1>     ]",
                "+----------------------------------------------------------+",
                " 1  -Reposition-     -Restore MP-     (+40 MP)",
                ">2< EX Whirlwind     (--Linker--)     (+10 MP, 250% ATK, -74 HP)",
                "[3] Frenzy           (Starter---)     (-10 MP,  20% ATK)",
                "[4] Heal             (---Heal---)     (-20 MP, Restore 896 HP)",
                "",
                "[Q] Riposte          (----------)     (-30 MP, Deals 182 Damage)",
                "[ ] (Locked - 10)",
                "[R] -Not Ready-      (----------)                 [5] Run (46%)",
                ">",
            ]),
            vec![
                option("1", 0, "-Reposition-"),
                option("2", 0, "EX Whirlwind"),
                option("3", 0, "Frenzy"),
                option("4", 0, "Heal"),
                option("q", 0, "Riposte"),
                option("r", 0, "-Not Ready-"),
                option("5", 50, "Run (46%)"),
            ]
        );
    }

    #[test]
    fn parses_finisher_in_skill_list() {
        assert_eq!(
            summary(&[
                "+----------------------------------------------------------+",
                " 1  -Reposition-     -Restore MP-     (+40 MP)",
                " 2  -Reposition-     -Restore MP-     (+40 MP)",
                "]3[ Final Blow       (--Finisher)     (-20 MP, 300% ATK, 100% Crit)",
                "[4] Heal             (---Heal---)     (-20 MP, Restore 903 HP)",
                "",
                "[Q] Riposte          (----------)     (-30 MP, No Damage)",
                "[ ] (Locked - 10)",
            ]),
            vec![
                option("1", 0, "-Reposition-"),
                option("2", 0, "-Reposition-"),
                option("3", 0, "Final Blow"),
                option("4", 0, "Heal"),
                option("q", 0, "Riposte"),
            ]
        );
    }

    #[test]
    fn parses_numbers_inside_pictures() {
        assert_eq!(
            summary(&[
                "  |    |  |  [77]  |  |    |    |    |  |  [88]  |  |    |",
                " [6] Sound Effects        [ON][44] <C][V>",
            ]),
            vec![option("77", 13, ""), option("88", 43, ""), option("6", 1, "Sound Effects")]
        );
    }

    #[test]
    fn ignores_character_stats() {
        assert_eq!(
            summary(&[
                "   NAME: Bob >> Lvl [9] Vassian Barbarian <Classic> [RES]",
                "   WEAP: Legendary [Dreamhack] [44] (Bow) [+66 DEX, +5 VIT] [NEU]",
                "   ARMR: Greater Plate Mail [4] (Heavy) [+21 WIS, +25 DEX]",
                "   CHRM: Meditative Vortewreck Token [1] (Shiny) [+12 STR, +10 VIT]",
                "    VIT: [065]",
                "  HP: 1751 / 1751   ATK: 123 - 205   MP: <37/63>  GOLD: 7792",
                "  You have VANQUISHED the Blote!     [ + 26 EXP ] [ + 60 GOLD ]",
                " Press any key to continue...",
                " [1] Continue",
            ]),
            vec![option("1", 1, "Continue")]
        );
    }

    #[test]
    fn cursor_returns_after_a_message_screen() {
        let rows = |rows: &[&str]| rows.iter().map(|row| row.to_string()).collect::<Vec<_>>();
        let masteries = rows(&[" [1] Blunt Mastery", " [2] Heavy Mastery", " [3] Magic Mastery"]);
        let mut menu = Menu::default();
        menu.update(&masteries);
        menu.navigate(Direction::Down);
        menu.navigate(Direction::Down);
        assert_eq!(typed(&menu.selected_keys().unwrap()), "3");

        menu.update(&rows(&[" Press any key to continue..."]));
        assert!(!menu.is_active());
        menu.update(&masteries);
        assert_eq!(typed(&menu.selected_keys().unwrap()), "3");

        // A different menu starts at the top.
        menu.update(&rows(&[" Press any key to continue..."]));
        menu.update(&rows(&[" [1] Play", " [2] Settings", " [3] Trials"]));
        assert_eq!(typed(&menu.selected_keys().unwrap()), "1");
    }

    #[test]
    fn ignores_distance_indicator() {
        let menu = menu(&[
            "> You have HIT Murv for 375 damage.",
            "  HP: 0 / 1394   ATK: 19 - 28  [SHOCK]",
            " [O][                    ] [?] <VAMPIRIC>",
            "> Murv has been OBLITERATED!",
        ]);
        assert!(!menu.is_active());
    }

    #[test]
    fn parses_help_option() {
        let rows = [
            "                 [E] Equip    [S] Salvage",
            "                                                 [?] Help",
            ">",
        ];
        assert_eq!(
            summary(&rows),
            vec![option("e", 17, "Equip"), option("s", 30, "Salvage"), option("?", 49, "Help")]
        );
        assert!(reachable(&rows).contains("?"));
    }

    #[test]
    fn finds_input_prompt() {
        let prompt = |rows: &[&str]| prompt_of(&rows.iter().map(|r| r.to_string()).collect::<Vec<_>>());
        assert_eq!(prompt(&[" [1] Play", ">", ""]), Some(String::new()));
        assert_eq!(prompt(&[" [1] Play", ">12"]), Some("12".to_string()));
        // Escape shows up in the row as a glyph, which has to be erased like any character.
        assert_eq!(prompt(&[" [1] Play", ">\x7f\x7f1"]).map(|typed| typed.len()), Some(3));
        assert_eq!(prompt(&["> Arrat has HIT you for 110 damage.", " [Y] Yes   [N] No"]), None);
        assert_eq!(prompt(&[" [1] Play", "> Arrat has HIT you for 110 damage."]), None);
    }

    #[test]
    fn navigates_columns() {
        let mut menu = menu(&[" [1] One      [3] Three", " [2] Two      [4] Four"]);
        let mut go = |direction| {
            menu.navigate(direction);
            typed(&menu.selected_keys().unwrap())
        };
        assert_eq!(go(Direction::Down), "2");
        assert_eq!(go(Direction::Right), "4");
        assert_eq!(go(Direction::Up), "3");
        assert_eq!(go(Direction::Left), "1");
        assert_eq!(go(Direction::Up), "2"); // wraps around, staying in the column
        assert_eq!(go(Direction::Left), "4"); // wraps around within the row
    }

    /// Lets the cursor wander and returns every option it landed on.
    fn reachable(rows: &[&str]) -> std::collections::BTreeSet<String> {
        let mut menu = menu(rows);
        let mut seen = std::collections::BTreeSet::new();
        // Sweep every row left to right, going down through all rows several times.
        for _ in 0..rows.len() * 2 {
            for _ in 0..8 {
                seen.insert(typed(&menu.selected_keys().unwrap()));
                menu.navigate(Direction::Right);
            }
            menu.navigate(Direction::Down);
        }
        seen
    }

    #[test]
    fn reaches_options_outside_the_columns() {
        let rows = [
            " [1] Strike   [2] Slash    [3] Bash",
            " [4] Parry    [5] Dodge             [Q] Link One  [W] Link Two",
            "                                    [E] Link Three",
            "",
            "                      [S] Save",
            "                                                  [X] Exit",
        ];
        let all: std::collections::BTreeSet<String> =
            menu(&rows).options.iter().map(|o| typed(&o.keys)).collect();
        assert_eq!(all.len(), 10);
        assert_eq!(reachable(&rows), all);
    }

    #[test]
    fn keeps_column_across_sparse_rows() {
        let mut menu = menu(&[
            " [1] One      [2] Two",
            "              [3] Three",
            " [4] Four     [5] Five",
        ]);
        menu.navigate(Direction::Down);
        menu.navigate(Direction::Down);
        assert_eq!(typed(&menu.selected_keys().unwrap()), "4");
    }

    #[test]
    fn finds_back_option() {
        let menu = menu(&[" [Q] Swap                                    [A] Back"]);
        assert_eq!(typed(&menu.back_keys().unwrap()), "a");
    }
}
