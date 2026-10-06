//! Drop-in replacement for the game's `SDL.dll`.
//!
//! Every SDL export jumps straight into the original library (renamed `SDL_orig.dll`,
//! see `build.rs`) except the two event functions below, which additionally turn XInput
//! controller input into SDL keyboard events. The game only reads the keyboard through
//! libtcod, which in turn only calls `SDL_PollEvent` / `SDL_WaitEvent`.
//!
//! On top of plain button-to-key bindings, `menu` reads the on-screen text so that the
//! d-pad and A can pick the game's `[key] label` choices directly.

#![allow(non_snake_case)]

mod config;
mod hook;
mod menu;
mod pad;
mod screen;
mod terminal;

use config::{Config, Key};
use menu::{Direction, Menu, MenuOption};
use pad::Pad;
use terminal::Terminal;
use std::collections::VecDeque;
use std::ffi::{c_char, c_int, c_void};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub mod win {
    use std::ffi::{c_char, c_void};

    #[link(name = "kernel32")]
    extern "system" {
        pub fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        pub fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        pub fn GetModuleHandleA(name: *const c_char) -> *mut c_void;
        pub fn AttachConsole(process: u32) -> i32;
        pub fn VirtualProtect(address: *mut c_void, size: usize, protection: u32, previous: *mut u32) -> i32;
    }
}

const SDL_KEYDOWN: u8 = 2;
const SDL_APPINPUTFOCUS: u8 = 0x02;
const KMOD_LSHIFT: i32 = 0x0001;
const SDLK_RETURN: i32 = 13;
const SDLK_ESCAPE: i32 = 27;
const SDLK_SPACE: i32 = 32;
const SDLK_UP: i32 = 273;
const SDLK_DOWN: i32 = 274;
const SDLK_RIGHT: i32 = 275;
const SDLK_LEFT: i32 = 276;

const BACKSPACE: Key = Key { sym: 8, unicode: 8, shift: false };
const ENTER: Key = Key { sym: 13, unicode: 13, shift: false };

const PAD_POLL_INTERVAL: Duration = Duration::from_millis(4);
const WAIT_SLEEP: Duration = Duration::from_millis(5);
const WAIT_MODE_WINDOW: Duration = Duration::from_secs(1);
const EVENT_LIFETIME: Duration = Duration::from_secs(2);
/// The game may sit in a pause (animated text, a sound) before it reads a typed choice,
/// so the echo can take a while. This only caps a screen that never echoes at all.
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(15);
const MENU_SYNC_INTERVAL: Duration = Duration::from_millis(50);
const REPEAT_DELAY: Duration = Duration::from_millis(400);
const REPEAT_INTERVAL: Duration = Duration::from_millis(90);
/// Only the directions auto-repeat while held.
const REPEATING: u32 = pad::UP | pad::DOWN | pad::LEFT | pad::RIGHT;

/// `SDL_KeyboardEvent` from SDL 1.2 (20 bytes; `SDL_Event` is a union at least this big).
#[repr(C)]
#[derive(Clone, Copy)]
struct KeyEvent {
    ty: u8,
    which: u8,
    state: u8,
    keysym: Keysym,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Keysym {
    scancode: u8,
    sym: i32,
    modifiers: i32,
    unicode: u16,
}

impl KeyEvent {
    /// A key press. Releases are never sent: the game only acts on presses, and a
    /// release that arrives late could be taken for input by the following screen.
    fn new(key: Key) -> Self {
        KeyEvent {
            ty: SDL_KEYDOWN,
            which: 0,
            state: 1,
            keysym: Keysym {
                scancode: 0,
                sym: key.sym,
                modifiers: if key.shift { KMOD_LSHIFT } else { 0 },
                unicode: key.unicode,
            },
        }
    }
}

/// See [`State::choose`].
struct Confirm {
    /// What the prompt shows once the game has taken the typed choice.
    expected: String,
    /// The option that was picked. Once it is gone from the screen, the key acted by
    /// itself and no Enter must follow it.
    chosen: MenuOption,
    deadline: Instant,
}

mod forwards {
    include!(concat!(env!("OUT_DIR"), "/forwards.rs"));
}

const DLL_PROCESS_DETACH: u32 = 0;
const DLL_PROCESS_ATTACH: u32 = 1;

/// Points every forwarding stub at the real SDL before anyone can call one.
#[no_mangle]
pub unsafe extern "system" fn DllMain(_module: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let lib = win::LoadLibraryA(c"SDL_orig.dll".as_ptr());
        if lib.is_null() {
            return 0;
        }
        let table = &raw mut forwards::FORWARD_TABLE;
        for (i, name) in forwards::FORWARDED.iter().enumerate() {
            (*table)[i] = win::GetProcAddress(lib, name.as_ptr()) as usize;
        }
        if terminal_requested() {
            // The game is shown in a console instead, so SDL must not open a window. This
            // has to be in SDL's environment before the game initialises it.
            let putenv = win::GetProcAddress(lib, c"SDL_putenv".as_ptr());
            if !putenv.is_null() {
                let putenv: unsafe extern "C" fn(*const c_char) -> c_int = std::mem::transmute(putenv);
                putenv(c"SDL_VIDEODRIVER=dummy".as_ptr());
            }
        }
    } else if reason == DLL_PROCESS_DETACH {
        // The console outlives the game, so it has to be handed back the way it was.
        if let Ok(mut guard) = STATE.try_lock() {
            if let Some(terminal) = guard.as_mut().and_then(|state| state.terminal.take()) {
                terminal.close();
            }
        }
    }
    1
}

struct Original {
    poll_event: unsafe extern "C" fn(*mut c_void) -> c_int,
    get_app_state: unsafe extern "C" fn() -> u8,
}

fn original() -> &'static Original {
    static ORIGINAL: OnceLock<Original> = OnceLock::new();
    ORIGINAL.get_or_init(|| unsafe {
        let lib = win::LoadLibraryA(c"SDL_orig.dll".as_ptr());
        assert!(!lib.is_null(), "SDL_orig.dll is missing");
        Original {
            poll_event: std::mem::transmute(win::GetProcAddress(lib, c"SDL_PollEvent".as_ptr())),
            get_app_state: std::mem::transmute(win::GetProcAddress(
                lib,
                c"SDL_GetAppState".as_ptr(),
            )),
        }
    })
}

/// `sanctuary-terminal.cmd` / `.sh` set this to have the game drawn in the console it was
/// started from, instead of in a window. See `Terminal::open` for the value.
fn terminal_requested() -> bool {
    std::env::var_os("SANCTUARY_PAD_TERMINAL").is_some()
}

struct State {
    config: Config,
    /// The console that shows the game in terminal mode.
    terminal: Option<Terminal>,
    pad: Pad,
    menu: Menu,
    held: u32,
    /// A typed menu choice that still has to be confirmed with Enter.
    confirm: Option<Confirm>,
    /// When the held direction should next repeat.
    next_repeat: Option<Instant>,
    last_poll: Instant,
    last_sync: Instant,
    /// Synthesized key events waiting to be handed to the game, with their creation time.
    queue: VecDeque<(Instant, KeyEvent)>,
    /// The last time the game sat in `SDL_WaitEvent`.
    last_wait: Option<Instant>,
    dumper: screen::Dumper,
    dumped_options: Vec<MenuOption>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// The game's `TCODConsole::flush`, once [`flush_hook`] has taken its place.
static GAME_FLUSH: AtomicUsize = AtomicUsize::new(0);

/// Runs in place of the game's `TCODConsole::flush` so that the menu highlight is in
/// the console before every frame is drawn, instead of flickering in afterwards.
unsafe extern "C" fn flush_hook() {
    if let Ok(mut guard) = STATE.lock() {
        if let Some(state) = guard.as_mut() {
            if state.config.menu_navigation {
                state.menu.sync();
            }
            if let Some(terminal) = &mut state.terminal {
                terminal.draw();
            }
        }
    }
    let flush: unsafe extern "C" fn() = std::mem::transmute(GAME_FLUSH.load(Ordering::Relaxed));
    flush();
}

impl State {
    fn new() -> Self {
        let config = Config::load(&game_dir().join("sanctuary-pad.ini"));
        log(&format!("loaded, {} buttons bound", config.bindings.len()));
        if config.menu_navigation || terminal_requested() {
            let patched = unsafe {
                hook::patch_import("libtcod-mingw.dll", c"_ZN11TCODConsole5flushEv", flush_hook as *const () as usize)
            };
            match patched {
                Some(previous) => GAME_FLUSH.store(previous, Ordering::Relaxed),
                None => log("could not hook TCODConsole::flush; the menu highlight may flicker"),
            }
        }
        let terminal = std::env::var("SANCTUARY_PAD_TERMINAL").ok().and_then(|how| {
            Terminal::open(&how).map_err(|error| log(&format!("could not open the terminal: {error}"))).ok()
        });
        let now = Instant::now();
        State {
            config,
            terminal,
            pad: Pad::new(),
            menu: Menu::default(),
            held: 0,
            confirm: None,
            next_repeat: None,
            last_poll: now,
            last_sync: now,
            queue: VecDeque::new(),
            last_wait: None,
            dumper: screen::Dumper::new(),
            dumped_options: Vec::new(),
        }
    }

    /// Samples the controller and queues key events for whatever changed.
    fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_poll) < PAD_POLL_INTERVAL {
            return;
        }
        self.last_poll = now;
        if self.config.dump_screen {
            self.dumper.tick(now);
        }

        if let Some(terminal) = &mut self.terminal {
            for key in terminal.keys() {
                if !self.key(key) {
                    self.queue.push_back((now, KeyEvent::new(key)));
                }
            }
        }

        // XInput reports input even when the window is in the background.
        let focused = match &self.terminal {
            Some(terminal) => terminal.focused(),
            None => unsafe { (original().get_app_state)() & SDL_APPINPUTFOCUS != 0 },
        };
        let buttons = self.pad.buttons(focused);
        let mut fired = buttons & !self.held;
        self.held = buttons;

        if fired & REPEATING != 0 {
            self.next_repeat = Some(now + REPEAT_DELAY);
        } else if buttons & REPEATING == 0 {
            self.next_repeat = None;
        } else if self.next_repeat.is_some_and(|t| now >= t) {
            self.next_repeat = Some(now + REPEAT_INTERVAL);
            fired |= buttons & REPEATING;
        }

        let mut redraw = self.confirm_choice(now);
        let sync_due = fired != 0 || now.duration_since(self.last_sync) >= MENU_SYNC_INTERVAL;
        if self.config.menu_navigation && sync_due {
            self.last_sync = now;
            redraw |= self.menu.sync();
            self.dump_options();
        }
        for &(name, button) in pad::BUTTON_NAMES {
            if fired & button != 0 {
                redraw |= self.press(name, button);
            }
        }
        if redraw {
            self.redraw();
        }
    }

    fn redraw(&mut self) {
        screen::flush();
        if let Some(terminal) = &mut self.terminal {
            terminal.draw();
        }
    }

    /// Lets the keyboard drive the game the way the controller does: the arrow keys move
    /// the menu highlight, Space and Enter are the A button and Escape the B button.
    /// Returns true if the key was used up and must not reach the game.
    fn key(&mut self, key: Key) -> bool {
        let (name, button) = match key.sym {
            SDLK_UP => ("up", pad::UP),
            SDLK_DOWN => ("down", pad::DOWN),
            SDLK_RIGHT => ("right", pad::RIGHT),
            SDLK_LEFT => ("left", pad::LEFT),
            SDLK_SPACE | SDLK_RETURN => ("a", pad::A),
            SDLK_ESCAPE => ("b", pad::B),
            _ => return false,
        };
        if !self.config.menu_navigation {
            return false;
        }
        let mut redraw = self.menu.sync();
        // Without choices on screen the arrow keys keep their own meaning. So does Space
        // in the middle of typed text; otherwise it is A, which advances cutscenes.
        // Escape has no meaning of its own: the game only takes it for a stray glyph.
        let typing = self.menu.prompt().is_some_and(|typed| !typed.is_empty());
        let used = if key.sym == SDLK_RETURN {
            // Enter is still what finishes a choice typed by hand.
            self.menu.is_active() && !typing
        } else {
            self.menu.is_active() || button == pad::B || (button == pad::A && !typing)
        };
        if used {
            redraw |= self.press(name, button);
        }
        if redraw {
            self.redraw();
        }
        used
    }

    /// Handles a button press (or auto-repeat). Returns true if the screen needs a redraw.
    fn press(&mut self, name: &str, button: u32) -> bool {
        if self.menu.is_active() {
            let direction = match button {
                pad::UP => Some(Direction::Up),
                pad::DOWN => Some(Direction::Down),
                pad::LEFT => Some(Direction::Left),
                pad::RIGHT => Some(Direction::Right),
                _ => None,
            };
            if let Some(direction) = direction {
                return self.menu.navigate(direction);
            }
            let choice = match button {
                pad::A => self.menu.selected(),
                pad::B => self.menu.back(),
                _ => None,
            };
            if let Some(option) = choice.cloned() {
                self.debug(&format!("[pad] {name}: menu choice {}", key_names(&option.keys)));
                self.choose(option);
                return false;
            }
        }
        if let Some(&key) = self.config.bindings.get(&button) {
            self.debug(&format!("[pad] {name}: key {}", key.sym));
            self.queue.push_back((Instant::now(), KeyEvent::new(key)));
        }
        false
    }

    /// Enters a menu choice the way the current screen expects it.
    fn choose(&mut self, option: MenuOption) {
        let now = Instant::now();
        let keys = &option.keys;
        let mut tap = |key: Key| self.queue.push_back((now, KeyEvent::new(key)));
        // Screens with a prompt usually read a whole line: clear anything already
        // typed, then type the choice.
        if let Some(typed) = self.menu.prompt() {
            for _ in typed.chars() {
                tap(BACKSPACE);
            }
        }
        keys.iter().for_each(|&key| tap(key));
        // Some screens act on the key itself, and an Enter sent blindly would then land
        // on whatever screen comes next. So it is only sent once the choice shows up at
        // the prompt, which means the game is waiting for the line to be finished.
        // The screen may still be in the middle of being drawn (event screens even change
        // the wording of their choices while they appear), so only the chosen option is
        // watched, not the whole menu.
        self.confirm = self.menu.prompt().is_some().then(|| Confirm {
            expected: option.keys.iter().map(|key| key.unicode as u8 as char).collect(),
            chosen: option.clone(),
            deadline: now + CONFIRM_TIMEOUT,
        });
    }

    /// Sends the Enter for a typed choice once the game has echoed it.
    /// Returns true if the screen needs a redraw.
    fn confirm_choice(&mut self, now: Instant) -> bool {
        let Some(confirm) = &self.confirm else { return false };
        let redraw = self.menu.sync();
        let expected = confirm.expected.to_ascii_lowercase();
        let typed = self.menu.prompt().map(|typed| typed.to_ascii_lowercase());
        if typed.as_deref() == Some(expected.as_str()) {
            self.debug("[pad] choice echoed, sending enter");
            self.queue.push_back((now, KeyEvent::new(ENTER)));
            self.confirm = None;
        } else if now > confirm.deadline
            || !self.menu.options().contains(&confirm.chosen)
            // The prompt is gone, or shows something other than the choice on its way in.
            || !typed.is_some_and(|typed| expected.starts_with(&typed))
        {
            self.debug("[pad] choice not echoed, giving up on the enter");
            self.confirm = None;
        }
        redraw
    }

    fn debug(&self, message: &str) {
        if self.config.dump_screen {
            append("sanctuary-pad-screen.txt", &format!("{message}\n"));
        }
    }

    fn dump_options(&mut self) {
        if self.config.dump_screen && self.menu.options() != self.dumped_options {
            self.dumped_options = self.menu.options().to_vec();
            let list: Vec<String> = self
                .dumped_options
                .iter()
                .map(|o| format!("{}@{},{}+{} {:?}", key_names(&o.keys), o.x, o.y, o.len, o.label))
                .collect();
            self.debug(&format!("[menu] {}", list.join(" | ")));
        }
    }
}

fn key_names(keys: &[Key]) -> String {
    keys.iter().map(|key| key.sym.to_string()).collect::<Vec<_>>().join("+")
}

/// Runs the controller logic and hands back one synthesized key event, if any.
fn next_pad_event(waiting: bool) -> Option<KeyEvent> {
    let mut guard = STATE.lock().ok()?;
    let state = guard.get_or_insert_with(State::new);
    state.tick();

    // libtcod drains SDL_PollEvent to discard stale input before it blocks in
    // SDL_WaitEvent. While the game reads input that way, only a wait may receive our
    // events, or all but the first key of a sequence would be thrown away.
    let now = Instant::now();
    if waiting {
        state.last_wait = Some(now);
    } else if state.last_wait.is_some_and(|t| now.duration_since(t) < WAIT_MODE_WINDOW) {
        return None;
    }
    // Presses made while the game was busy should not fire long afterwards.
    while state.queue.front().is_some_and(|(created, _)| now.duration_since(*created) > EVENT_LIFETIME) {
        state.queue.pop_front();
    }
    state.queue.pop_front().map(|(_, event)| event)
}

/// Offers a key press from the game's own window to the menu navigation.
/// Returns true if the event was used up.
unsafe fn keyboard_event(event: *mut c_void) -> bool {
    let event = event.cast::<KeyEvent>().read_unaligned();
    if event.ty != SDL_KEYDOWN {
        return false;
    }
    let Ok(mut guard) = STATE.lock() else { return false };
    let key = Key { sym: event.keysym.sym, unicode: event.keysym.unicode, shift: false };
    guard.get_or_insert_with(State::new).key(key)
}

unsafe fn poll_event(event: *mut c_void, waiting: bool) -> c_int {
    // A null event only asks whether something is pending; leave that to SDL.
    if event.is_null() {
        return (original().poll_event)(event);
    }
    while (original().poll_event)(event) != 0 {
        if !keyboard_event(event) {
            return 1;
        }
    }
    match next_pad_event(waiting) {
        Some(key_event) => {
            event.cast::<KeyEvent>().write_unaligned(key_event);
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_PollEvent(event: *mut c_void) -> c_int {
    poll_event(event, false)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WaitEvent(event: *mut c_void) -> c_int {
    // SDL 1.2 implements this as a poll-and-sleep loop too, so nothing is lost by doing
    // it here with the controller mixed in.
    let mut scratch = [0u8; 64];
    let target = if event.is_null() { scratch.as_mut_ptr().cast() } else { event };
    loop {
        if poll_event(target, true) != 0 {
            return 1;
        }
        std::thread::sleep(WAIT_SLEEP);
    }
}

fn game_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_default()
}

pub fn log(message: &str) {
    static FILE: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    let file = FILE.get_or_init(|| {
        std::fs::File::create(game_dir().join("sanctuary-pad.log")).ok().map(Mutex::new)
    });
    if let Some(Ok(mut file)) = file.as_ref().map(|f| f.lock()) {
        let _ = writeln!(file, "{message}");
    }
}

/// Appends to a debug file in the game directory.
pub fn append(name: &str, text: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(game_dir().join(name)) {
        let _ = file.write_all(text.as_bytes());
    }
}
