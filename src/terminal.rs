//! Terminal mode: mirrors libtcod's root console into the console the game was started
//! from with ratatui, and turns the keys typed there into the keys the game expects.
//!
//! On Windows the game attaches to its parent's console and crossterm talks to it. Under
//! Proton there is no usable console, but Wine hands the game the Unix terminal as its
//! standard input and output, so there the escape sequences are written straight to
//! stdout and the raw keys are read from stdin (`sanctuary-terminal.sh` puts the terminal
//! into raw mode and keeps its size in a file for us).

use crate::config::Key;
use crate::screen;
use crate::win::AttachConsole;
use ratatui::backend::{Backend, ClearType, CrosstermBackend, WindowSize};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::{cursor, execute, terminal};
use ratatui::layout::{Position, Size};
use ratatui::style::{Color, Style};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Stdout, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

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

/// How often the size file is checked for a resized terminal.
const SIZE_CHECK_INTERVAL: Duration = Duration::from_millis(250);

/// Where the screen is drawn.
enum Output {
    /// A Windows console, through crossterm.
    Console(CrosstermBackend<BufWriter<File>>),
    /// The Unix terminal on stdout, with our own escape sequences.
    Stdio(Ansi),
}

/// Where the keys come from.
enum Input {
    /// crossterm's event queue on the console.
    Console,
    /// Raw bytes read from stdin by a thread; the terminal size lives in a file.
    Stdio { bytes: Receiver<Vec<u8>>, size_file: PathBuf, size_checked: Instant },
}

pub struct Terminal {
    tui: ratatui::Terminal<Output>,
    input: Input,
    focused: bool,
}

impl Terminal {
    /// `how` is the value of `SANCTUARY_PAD_TERMINAL`: `1` for the parent console, or the
    /// file in which the launcher keeps the terminal size (`rows columns`) for stdio.
    pub fn open(how: &str) -> io::Result<Self> {
        if how == "1" {
            Self::open_console()
        } else {
            Self::open_stdio(PathBuf::from(how))
        }
    }

    fn open_console() -> io::Result<Self> {
        // The game is a GUI program, so it does not get its parent's console by itself.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
            return Err(io::Error::other("the game was not started from a console"));
        }
        let mut out = BufWriter::new(OpenOptions::new().read(true).write(true).open("CONOUT$")?);
        terminal::enable_raw_mode()?;
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        let tui = ratatui::Terminal::new(Output::Console(CrosstermBackend::new(out)))?;
        crate::log("terminal opened (console)");
        Ok(Terminal { tui, input: Input::Console, focused: true })
    }

    fn open_stdio(size_file: PathBuf) -> io::Result<Self> {
        let size = read_size(&size_file)?;
        let mut ansi = Ansi { out: BufWriter::new(io::stdout()), size };
        // Alternate screen, hidden cursor, focus reports.
        ansi.out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?1004h\x1b[2J")?;
        ansi.out.flush()?;
        let (sender, bytes) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stdin = io::stdin();
            let mut buffer = [0u8; 256];
            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if sender.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let tui = ratatui::Terminal::new(Output::Stdio(ansi))?;
        crate::log(&format!("terminal opened (stdio, {}x{})", size.width, size.height));
        let input = Input::Stdio { bytes, size_file, size_checked: Instant::now() };
        Ok(Terminal { tui, input, focused: true })
    }

    /// Gives the console back to the shell.
    pub fn close(mut self) {
        match self.tui.backend_mut() {
            Output::Console(backend) => {
                let _ = execute!(backend, terminal::LeaveAlternateScreen, cursor::Show);
                let _ = terminal::disable_raw_mode();
            }
            Output::Stdio(ansi) => {
                let _ = ansi.out.write_all(b"\x1b[0m\x1b[?1004l\x1b[?1049l\x1b[?25h");
                let _ = ansi.out.flush();
            }
        }
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
        match &mut self.input {
            Input::Console => {
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
            }
            Input::Stdio { bytes, size_file, size_checked } => {
                let mut events = Vec::new();
                while let Ok(chunk) = bytes.try_recv() {
                    parse_input(&chunk, &mut events);
                }
                let mut resized = false;
                if size_checked.elapsed() >= SIZE_CHECK_INTERVAL {
                    *size_checked = Instant::now();
                    if let Ok(size) = read_size(size_file) {
                        if let Output::Stdio(ansi) = self.tui.backend_mut() {
                            if ansi.size != size {
                                ansi.size = size;
                                resized = true;
                                crate::log(&format!("terminal resized to {}x{}", size.width, size.height));
                            }
                        }
                    }
                }
                for event in events {
                    match event {
                        TtyEvent::Key(key) => keys.push(key),
                        TtyEvent::FocusGained => self.focused = true,
                        TtyEvent::FocusLost => self.focused = false,
                    }
                }
                if resized {
                    self.draw();
                }
            }
        }
        keys
    }
}

/// Reads `rows columns` (the output of `stty size`) from the launcher's size file.
fn read_size(file: &PathBuf) -> io::Result<Size> {
    let text = std::fs::read_to_string(file)?;
    let mut numbers = text.split_whitespace().map(|n| n.parse::<u16>().ok());
    match (numbers.next().flatten(), numbers.next().flatten()) {
        (Some(height), Some(width)) if width > 0 && height > 0 => Ok(Size { width, height }),
        _ => Err(io::Error::other(format!("no terminal size in {}", file.display()))),
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

#[derive(Debug, PartialEq)]
enum TtyEvent {
    Key(Key),
    FocusGained,
    FocusLost,
}

fn named(sym: i32, unicode: u16) -> TtyEvent {
    TtyEvent::Key(Key { sym, unicode, shift: false })
}

/// Turns the raw bytes a terminal sends into keys: plain characters, control characters,
/// and the `ESC [` / `ESC O` sequences for arrows, editing keys, function keys and focus.
fn parse_input(bytes: &[u8], events: &mut Vec<TtyEvent>) {
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == 0x1b && i + 1 < bytes.len() && matches!(bytes[i + 1], b'[' | b'O') {
            let ss3 = bytes[i + 1] == b'O';
            let mut end = i + 2;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b';') {
                end += 1;
            }
            // A sequence cut off at the end of a read is dropped rather than misread.
            let Some(&final_byte) = bytes.get(end) else { break };
            let number = std::str::from_utf8(&bytes[i + 2..end])
                .ok()
                .and_then(|params| params.split(';').next()?.parse::<u16>().ok());
            events.extend(sequence(final_byte, number, ss3));
            i = end + 1;
            continue;
        }
        events.extend(match byte {
            0x1b => Some(named(27, 27)),
            b'\r' | b'\n' => Some(named(13, 13)),
            0x7f | 0x08 => Some(named(8, 8)),
            b'\t' => Some(named(9, 9)),
            0x20..=0x7e => Some(TtyEvent::Key(Key {
                sym: byte.to_ascii_lowercase() as i32,
                unicode: byte as u16,
                shift: byte.is_ascii_uppercase(),
            })),
            _ => None,
        });
        i += 1;
    }
}

fn sequence(final_byte: u8, number: Option<u16>, ss3: bool) -> Option<TtyEvent> {
    let function = |n: i32| Some(named(281 + n, 0));
    match (final_byte, ss3) {
        (b'A', _) => Some(named(273, 0)),
        (b'B', _) => Some(named(274, 0)),
        (b'C', _) => Some(named(275, 0)),
        (b'D', _) => Some(named(276, 0)),
        (b'H', _) => Some(named(278, 0)),
        (b'F', _) => Some(named(279, 0)),
        (b'P', true) => function(1),
        (b'Q', true) => function(2),
        (b'R', true) => function(3),
        (b'S', true) => function(4),
        (b'I', false) => Some(TtyEvent::FocusGained),
        (b'O', false) => Some(TtyEvent::FocusLost),
        (b'~', false) => match number? {
            1 | 7 => Some(named(278, 0)),
            2 => Some(named(277, 0)),
            3 => Some(named(127, 127)),
            4 | 8 => Some(named(279, 0)),
            5 => Some(named(280, 0)),
            6 => Some(named(281, 0)),
            n @ 11..=15 => function(n as i32 - 10),
            n @ 17..=21 => function(n as i32 - 11),
            n @ 23..=24 => function(n as i32 - 12),
            _ => None,
        },
        _ => None,
    }
}

/// A ratatui backend that writes plain ANSI escape sequences to stdout. The size comes
/// from the launcher, since the terminal cannot be asked from inside Wine.
pub struct Ansi {
    out: BufWriter<Stdout>,
    size: Size,
}

impl Ansi {
    fn write_color(&mut self, color: Color, foreground: bool) -> io::Result<()> {
        match color {
            Color::Rgb(r, g, b) => {
                write!(self.out, "\x1b[{};2;{r};{g};{b}m", if foreground { 38 } else { 48 })
            }
            _ => write!(self.out, "\x1b[{}m", if foreground { 39 } else { 49 }),
        }
    }
}

impl Backend for Ansi {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        let mut fg = Color::Reset;
        let mut bg = Color::Reset;
        let mut last: Option<(u16, u16)> = None;
        self.out.write_all(b"\x1b[0m")?;
        for (x, y, cell) in content {
            if last != Some((x.wrapping_sub(1), y)) {
                write!(self.out, "\x1b[{};{}H", y + 1, x + 1)?;
            }
            last = Some((x, y));
            if cell.fg != fg {
                fg = cell.fg;
                self.write_color(fg, true)?;
            }
            if cell.bg != bg {
                bg = cell.bg;
                self.write_color(bg, false)?;
            }
            self.out.write_all(cell.symbol().as_bytes())?;
        }
        self.out.write_all(b"\x1b[0m")
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.out.write_all(b"\x1b[?25l")
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.out.write_all(b"\x1b[?25h")
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(Position::ORIGIN)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        let Position { x, y } = position.into();
        write!(self.out, "\x1b[{};{}H", y + 1, x + 1)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.out.write_all(b"\x1b[0m\x1b[2J")
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.out.write_all(match clear_type {
            ClearType::All => b"\x1b[2J",
            ClearType::AfterCursor => b"\x1b[J",
            ClearType::BeforeCursor => b"\x1b[1J",
            ClearType::CurrentLine => b"\x1b[2K",
            ClearType::UntilNewLine => b"\x1b[K",
        })
    }

    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize { columns_rows: self.size, pixels: Size::default() })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

impl Backend for Output {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        match self {
            Output::Console(backend) => backend.draw(content),
            Output::Stdio(backend) => backend.draw(content),
        }
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        match self {
            Output::Console(backend) => backend.hide_cursor(),
            Output::Stdio(backend) => backend.hide_cursor(),
        }
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        match self {
            Output::Console(backend) => backend.show_cursor(),
            Output::Stdio(backend) => backend.show_cursor(),
        }
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        match self {
            Output::Console(backend) => backend.get_cursor_position(),
            Output::Stdio(backend) => backend.get_cursor_position(),
        }
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        match self {
            Output::Console(backend) => backend.set_cursor_position(position),
            Output::Stdio(backend) => backend.set_cursor_position(position),
        }
    }

    fn clear(&mut self) -> io::Result<()> {
        match self {
            Output::Console(backend) => backend.clear(),
            Output::Stdio(backend) => backend.clear(),
        }
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        match self {
            Output::Console(backend) => backend.clear_region(clear_type),
            Output::Stdio(backend) => backend.clear_region(clear_type),
        }
    }

    fn size(&self) -> io::Result<Size> {
        match self {
            Output::Console(backend) => backend.size(),
            Output::Stdio(backend) => backend.size(),
        }
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        match self {
            Output::Console(backend) => backend.window_size(),
            Output::Stdio(backend) => backend.window_size(),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Console(backend) => Backend::flush(backend),
            Output::Stdio(backend) => backend.flush(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bytes: &[u8]) -> Vec<(i32, u16, bool)> {
        let mut events = Vec::new();
        parse_input(bytes, &mut events);
        events
            .into_iter()
            .map(|event| match event {
                TtyEvent::Key(key) => (key.sym, key.unicode, key.shift),
                TtyEvent::FocusGained => (-1, 0, false),
                TtyEvent::FocusLost => (-2, 0, false),
            })
            .collect()
    }

    #[test]
    fn characters_and_controls() {
        assert_eq!(keys(b"aZ \r\x7f\t\x1b"), vec![
            (b'a' as i32, b'a' as u16, false),
            (b'z' as i32, b'Z' as u16, true),
            (32, 32, false),
            (13, 13, false),
            (8, 8, false),
            (9, 9, false),
            (27, 27, false),
        ]);
    }

    #[test]
    fn escape_sequences() {
        assert_eq!(keys(b"\x1b[A\x1b[B\x1b[C\x1b[D"), vec![(273, 0, false), (274, 0, false), (275, 0, false), (276, 0, false)]);
        assert_eq!(keys(b"\x1b[1;2A"), vec![(273, 0, false)]);
        assert_eq!(keys(b"\x1b[3~\x1b[5~\x1b[6~\x1b[H\x1bOF"), vec![
            (127, 127, false),
            (280, 0, false),
            (281, 0, false),
            (278, 0, false),
            (279, 0, false),
        ]);
        assert_eq!(keys(b"\x1bOP\x1b[15~\x1b[24~"), vec![(282, 0, false), (286, 0, false), (293, 0, false)]);
        assert_eq!(keys(b"\x1b[I\x1b[O"), vec![(-1, 0, false), (-2, 0, false)]);
    }

    #[test]
    fn escape_before_text_is_a_key() {
        assert_eq!(keys(b"\x1bq"), vec![(27, 27, false), (b'q' as i32, b'q' as u16, false)]);
        // Cut-off sequences are dropped, multibyte characters are ignored.
        assert_eq!(keys(b"\x1b["), vec![]);
        assert_eq!(keys("é1".as_bytes()), vec![(b'1' as i32, b'1' as u16, false)]);
    }
}
