//! Access to libtcod's root console, i.e. the text currently on screen.

use crate::win::{GetProcAddress, LoadLibraryA};
use std::ffi::{c_int, c_void, CStr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// libtcod's `TCOD_BKGND_SET`.
const BKGND_SET: c_int = 1;

struct Tcod {
    get_width: unsafe extern "C" fn(*mut c_void) -> c_int,
    get_height: unsafe extern "C" fn(*mut c_void) -> c_int,
    get_char: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int,
    // The `_wrapper` variants take colours as 0xBBGGRR integers instead of structs.
    get_background: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> u32,
    set_background: unsafe extern "C" fn(*mut c_void, c_int, c_int, u32, c_int),
    flush: unsafe extern "C" fn(),
}

fn tcod() -> Option<&'static Tcod> {
    static TCOD: OnceLock<Option<Tcod>> = OnceLock::new();
    TCOD.get_or_init(|| unsafe {
        let lib = LoadLibraryA(c"libtcod-mingw.dll".as_ptr());
        if lib.is_null() {
            return None;
        }
        let find = |name: &CStr| {
            let f = GetProcAddress(lib, name.as_ptr());
            (!f.is_null()).then_some(f)
        };
        Some(Tcod {
            get_width: std::mem::transmute(find(c"TCOD_console_get_width")?),
            get_height: std::mem::transmute(find(c"TCOD_console_get_height")?),
            get_char: std::mem::transmute(find(c"TCOD_console_get_char")?),
            get_background: std::mem::transmute(find(c"TCOD_console_get_char_background_wrapper")?),
            set_background: std::mem::transmute(find(c"TCOD_console_set_char_background_wrapper")?),
            flush: std::mem::transmute(find(c"TCOD_console_flush")?),
        })
    })
    .as_ref()
}

// A null console pointer means the root console throughout libtcod's C API.
const ROOT: *mut c_void = std::ptr::null_mut();

/// Returns the root console as one string per row, with non-ASCII glyphs as spaces.
/// Every cell is one byte, so byte offsets are column numbers.
pub fn read() -> Vec<String> {
    let Some(tcod) = tcod() else { return Vec::new() };
    unsafe {
        let (width, height) = ((tcod.get_width)(ROOT), (tcod.get_height)(ROOT));
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| match (tcod.get_char)(ROOT, x, y) {
                        c @ 32..=126 => c as u8 as char,
                        _ => ' ',
                    })
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }
}

pub fn background(x: i32, y: i32) -> u32 {
    tcod().map_or(0, |tcod| unsafe { (tcod.get_background)(ROOT, x, y) } & 0xff_ffff)
}

pub fn set_background(x: i32, y: i32, color: u32) {
    if let Some(tcod) = tcod() {
        unsafe { (tcod.set_background)(ROOT, x, y, color, BKGND_SET) }
    }
}

/// Redraws the window from the root console.
pub fn flush() {
    if let Some(tcod) = tcod() {
        unsafe { (tcod.flush)() }
    }
}

const DUMP_INTERVAL: Duration = Duration::from_millis(500);

/// Debug aid: writes the screen text to `sanctuary-pad-screen.txt` whenever it changes.
pub struct Dumper {
    last_dump: Instant,
    last_rows: Vec<String>,
}

impl Dumper {
    pub fn new() -> Self {
        Dumper { last_dump: Instant::now(), last_rows: Vec::new() }
    }

    pub fn tick(&mut self, now: Instant) {
        if now.duration_since(self.last_dump) < DUMP_INTERVAL {
            return;
        }
        self.last_dump = now;
        let rows = read();
        if rows != self.last_rows {
            let mut text = String::from("=====\n");
            for row in &rows {
                text.push_str(row);
                text.push('\n');
            }
            crate::append("sanctuary-pad-screen.txt", &text);
            self.last_rows = rows;
        }
    }
}
