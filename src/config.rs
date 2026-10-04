//! Button-to-key bindings, read from `sanctuary-pad.ini` next to the game executable.

use crate::pad::BUTTON_NAMES;
use std::collections::HashMap;
use std::path::Path;

/// A key as SDL 1.2 describes it: an `SDLKey` plus the character it types.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    pub sym: i32,
    pub unicode: u16,
    pub shift: bool,
}

const DEFAULT_BINDINGS: &[(&str, &str)] = &[
    ("up", "up"),
    ("down", "down"),
    ("left", "left"),
    ("right", "right"),
    ("a", "enter"),
    // The game has no use for Escape (it types a stray glyph), so B erases instead.
    ("b", "backspace"),
    ("x", "1"),
    ("y", "2"),
    ("lb", "3"),
    ("rb", "4"),
    ("lt", "5"),
    ("rt", "6"),
    ("start", "enter"),
    ("back", "backspace"),
];

pub struct Config {
    pub bindings: HashMap<u32, Key>,
    /// Let the d-pad and A/B pick the `[key] label` choices shown on screen.
    pub menu_navigation: bool,
    pub dump_screen: bool,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let mut config = Config { bindings: HashMap::new(), menu_navigation: true, dump_screen: false };
        for (button, key) in DEFAULT_BINDINGS {
            config.set(button, key);
        }
        match std::fs::read_to_string(path) {
            Ok(text) => {
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with(['#', ';', '[']) {
                        continue;
                    }
                    let parsed = line
                        .split_once('=')
                        .is_some_and(|(name, value)| config.set(name.trim(), value.trim()));
                    if !parsed {
                        crate::log(&format!("ignoring config line: {line}"));
                    }
                }
            }
            Err(_) => crate::log("no sanctuary-pad.ini found; using default bindings"),
        }
        config
    }

    fn set(&mut self, name: &str, value: &str) -> bool {
        let name = name.to_ascii_lowercase();
        let enabled = matches!(value, "1" | "true" | "yes" | "on");
        match name.as_str() {
            "menu_navigation" => self.menu_navigation = enabled,
            "dump_screen" => self.dump_screen = enabled,
            _ => return self.bind(&name, value),
        }
        true
    }

    fn bind(&mut self, name: &str, value: &str) -> bool {
        let Some(&(_, button)) = BUTTON_NAMES.iter().find(|(n, _)| *n == name) else {
            return false;
        };
        if value.is_empty() || value.eq_ignore_ascii_case("none") {
            self.bindings.remove(&button);
            return true;
        }
        match parse_key(value) {
            Some(key) => {
                self.bindings.insert(button, key);
                true
            }
            None => false,
        }
    }
}

pub fn parse_key(value: &str) -> Option<Key> {
    let named = |sym: i32, unicode: u16| Some(Key { sym, unicode, shift: false });
    match value.to_ascii_lowercase().as_str() {
        "backspace" => return named(8, 8),
        "tab" => return named(9, 9),
        "enter" | "return" => return named(13, 13),
        "escape" | "esc" => return named(27, 27),
        "space" => return named(32, 32),
        "up" => return named(273, 0),
        "down" => return named(274, 0),
        "right" => return named(275, 0),
        "left" => return named(276, 0),
        _ => {}
    }
    let mut chars = value.chars();
    let (c, None) = (chars.next()?, chars.next()) else { return None };
    if !c.is_ascii_graphic() {
        return None;
    }
    Some(Key {
        sym: c.to_ascii_lowercase() as i32,
        unicode: c as u16,
        shift: c.is_ascii_uppercase(),
    })
}
