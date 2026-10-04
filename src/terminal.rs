//! Terminal mode: mirrors libtcod's root console into the console the game was started
//! from with ratatui, and turns the keys typed there into the keys the game expects.

use crate::config::Key;
use crate::screen;
use crate::win::AttachConsole;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::{cursor, execute, terminal};
use ratatui::style::{Color, Style};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter};
use std::sync::OnceLock;
use std::time::Duration;

/// Code page 437, which is how the game's font sheet (`Terminal/*/terminal.png`) is laid out.
const CP437: &str = concat!(
    " ☺☻♥♦♣♠•◘○◙♂♀♪♫☼",
    "►◄↕‼¶§▬↨↑↓→←∟↔▲▼",
    " !\"#$%&'()*+,-./",
    "0123456789:;<=>?",
    "@ABCDEFGHIJKLMNO",
    "PQRSTUVWXYZ[\\]^_",
    "`abcdefghijklmno",
    "pqrstuvwxyz{|}~⌂",
    "ÇüéâäàåçêëèïîìÄÅ",
    "ÉæÆôöòûùÿÖÜ¢£¥₧ƒ",
    "áíóúñÑªº¿⌐¬½¼¡«»",
    "░▒▓│┤╡╢╖╕╣║╗╝╜╛┐",
    "└┴┬├─┼╞╟╚╔╩╦╠═╬╧",
    "╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀",
    "αßΓπΣσµτΦΘΩδ∞φε∩",
    "≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ",
);

fn glyph(code: i32) -> char {
    static GLYPHS: OnceLock<Vec<char>> = OnceLock::new();
    let glyphs = GLYPHS.get_or_init(|| CP437.chars().collect());
    usize::try_from(code).ok().and_then(|code| glyphs.get(code).copied()).unwrap_or('?')
}

/// Converts libtcod's 0xBBGGRR into a terminal colour.
fn color(bgr: u32) -> Color {
    Color::Rgb(bgr as u8, (bgr >> 8) as u8, (bgr >> 16) as u8)
}

const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

pub struct Terminal {
    tui: ratatui::Terminal<CrosstermBackend<BufWriter<File>>>,
    focused: bool,
}

impl Terminal {
    pub fn open() -> io::Result<Self> {
        // The game is a GUI program, so it does not get its parent's console by itself.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
            return Err(io::Error::other("the game was not started from a console"));
        }
        let mut out = BufWriter::new(OpenOptions::new().read(true).write(true).open("CONOUT$")?);
        terminal::enable_raw_mode()?;
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        let tui = ratatui::Terminal::new(CrosstermBackend::new(out))?;
        crate::log("terminal opened");
        Ok(Terminal { tui, focused: true })
    }

    /// Gives the console back to the shell.
    pub fn close(mut self) {
        let _ = execute!(self.tui.backend_mut(), terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
        crate::log("terminal closed");
    }

    /// A real controller is ignored while the console is in the background.
    pub fn focused(&self) -> bool {
        self.focused
    }

    /// Copies the game's console to the terminal.
    pub fn draw(&mut self) {
        let (width, height) = screen::size();
        let (Ok(width), Ok(height)) = (u16::try_from(width), u16::try_from(height)) else { return };
        if width == 0 || height == 0 {
            return;
        }
        let _ = self.tui.draw(|frame| {
            let area = frame.area();
            let buffer = frame.buffer_mut();
            if area.width < width || area.height < height {
                let message = format!(
                    "The game needs {width}x{height} cells, this window has {}x{}. Enlarge it.",
                    area.width, area.height
                );
                buffer.set_stringn(area.x, area.y, message, area.width as usize, Style::default());
                return;
            }
            let (left, top) = (area.x + (area.width - width) / 2, area.y + (area.height - height) / 2);
            for y in 0..height {
                for x in 0..width {
                    let (code, foreground, background) = screen::cell(x as i32, y as i32);
                    buffer[(left + x, top + y)]
                        .set_char(glyph(code))
                        .set_fg(color(foreground))
                        // Black is the game's empty background: let the terminal's own show.
                        .set_bg(if background == 0 { Color::Reset } else { color(background) });
                }
            }
        });
    }

    /// Returns the keys typed since the last call.
    pub fn keys(&mut self) -> Vec<Key> {
        let mut keys = Vec::new();
        while event::poll(Duration::ZERO).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => keys.extend(translate(key)),
                Ok(Event::FocusGained) => self.focused = true,
                Ok(Event::FocusLost) => self.focused = false,
                // The game only redraws when something changes, which may be a while.
                Ok(Event::Resize(..)) => self.draw(),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        keys
    }
}

/// Maps a terminal key to SDL 1.2's `SDLKey` numbering.
fn translate(key: KeyEvent) -> Option<Key> {
    let named = |sym: i32, unicode: u16| Some(Key { sym, unicode, shift: false });
    match key.code {
        KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => Some(Key {
            sym: c.to_ascii_lowercase() as i32,
            unicode: c as u16,
            shift: c.is_ascii_uppercase() || key.modifiers.contains(KeyModifiers::SHIFT),
        }),
        KeyCode::Backspace => named(8, 8),
        KeyCode::Tab => named(9, 9),
        KeyCode::Enter => named(13, 13),
        KeyCode::Esc => named(27, 27),
        KeyCode::Delete => named(127, 127),
        KeyCode::Up => named(273, 0),
        KeyCode::Down => named(274, 0),
        KeyCode::Right => named(275, 0),
        KeyCode::Left => named(276, 0),
        KeyCode::Insert => named(277, 0),
        KeyCode::Home => named(278, 0),
        KeyCode::End => named(279, 0),
        KeyCode::PageUp => named(280, 0),
        KeyCode::PageDown => named(281, 0),
        KeyCode::F(n @ 1..=12) => named(281 + n as i32, 0),
        _ => None,
    }
}
