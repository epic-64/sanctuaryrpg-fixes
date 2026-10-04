//! XInput polling. The DLL is loaded dynamically so the game still starts if XInput is missing.

use crate::win::{GetProcAddress, LoadLibraryA};
use std::time::{Duration, Instant};

pub const UP: u32 = 0x0001;
pub const DOWN: u32 = 0x0002;
pub const LEFT: u32 = 0x0004;
pub const RIGHT: u32 = 0x0008;
pub const A: u32 = 0x1000;
pub const B: u32 = 0x2000;
pub const LT: u32 = 0x1_0000;
pub const RT: u32 = 0x2_0000;

/// Config names for every button bit, including the virtual trigger bits.
pub const BUTTON_NAMES: &[(&str, u32)] = &[
    ("up", UP),
    ("down", DOWN),
    ("left", LEFT),
    ("right", RIGHT),
    ("start", 0x0010),
    ("back", 0x0020),
    ("ls", 0x0040),
    ("rs", 0x0080),
    ("lb", 0x0100),
    ("rb", 0x0200),
    ("a", A),
    ("b", B),
    ("x", 0x4000),
    ("y", 0x8000),
    ("lt", LT),
    ("rt", RT),
];

const STICK_THRESHOLD: i16 = 16000;
const TRIGGER_THRESHOLD: u8 = 60;
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

#[repr(C)]
#[derive(Default)]
struct XInputState {
    packet: u32,
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    lx: i16,
    ly: i16,
    rx: i16,
    ry: i16,
}

type XInputGetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;

/// Debug aid: `SANCTUARY_PAD_SIMULATE=a,down,a` presses those buttons in turn, without
/// needing a controller.
struct Script {
    steps: Vec<u32>,
    started: Instant,
}

impl Script {
    const START_DELAY_MS: u128 = 6000;
    const STEP_MS: u128 = 1500;
    const HOLD_MS: u128 = 150;

    fn from_env() -> Option<Self> {
        let spec = std::env::var("SANCTUARY_PAD_SIMULATE").ok()?;
        let steps = spec
            .split(',')
            .map(|name| BUTTON_NAMES.iter().find(|(n, _)| *n == name.trim()).map_or(0, |b| b.1))
            .collect();
        Some(Script { steps, started: Instant::now() })
    }

    fn buttons(&self) -> u32 {
        let Some(t) = self.started.elapsed().as_millis().checked_sub(Self::START_DELAY_MS) else {
            return 0;
        };
        match self.steps.get((t / Self::STEP_MS) as usize) {
            Some(&button) if t % Self::STEP_MS < Self::HOLD_MS => button,
            _ => 0,
        }
    }
}

pub struct Pad {
    script: Option<Script>,
    get_state: Option<XInputGetState>,
    index: Option<u32>,
    last_scan: Option<Instant>,
}

impl Pad {
    pub fn new() -> Self {
        let get_state = [c"xinput1_4.dll", c"xinput1_3.dll", c"xinput9_1_0.dll"]
            .iter()
            .find_map(|name| unsafe {
                let lib = LoadLibraryA(name.as_ptr());
                if lib.is_null() {
                    return None;
                }
                let f = GetProcAddress(lib, c"XInputGetState".as_ptr());
                (!f.is_null()).then(|| std::mem::transmute::<_, XInputGetState>(f))
            });
        if get_state.is_none() {
            crate::log("XInput not found; controller support disabled");
        }
        Pad { script: Script::from_env(), get_state, index: None, last_scan: None }
    }

    /// Returns the currently held buttons as a bitmask (the left stick counts as the d-pad).
    /// A real controller is ignored while the game window is not `focused`.
    pub fn buttons(&mut self, focused: bool) -> u32 {
        if let Some(script) = &self.script {
            return script.buttons();
        }
        if !focused {
            return 0;
        }
        let Some(get_state) = self.get_state else { return 0 };
        let mut state = XInputState::default();

        if let Some(index) = self.index {
            if unsafe { get_state(index, &mut state) } != 0 {
                crate::log(&format!("controller {index} disconnected"));
                self.index = None;
                self.last_scan = Some(Instant::now());
                return 0;
            }
        } else {
            // Querying empty slots is slow, so only rescan occasionally.
            if self.last_scan.is_some_and(|t| t.elapsed() < RECONNECT_INTERVAL) {
                return 0;
            }
            self.last_scan = Some(Instant::now());
            self.index = (0..4).find(|&i| unsafe { get_state(i, &mut state) } == 0);
            match self.index {
                Some(index) => crate::log(&format!("controller {index} connected")),
                None => return 0,
            }
        }

        let mut buttons = state.buttons as u32;
        if state.left_trigger > TRIGGER_THRESHOLD {
            buttons |= LT;
        }
        if state.right_trigger > TRIGGER_THRESHOLD {
            buttons |= RT;
        }
        if state.ly > STICK_THRESHOLD {
            buttons |= UP;
        }
        if state.ly < -STICK_THRESHOLD {
            buttons |= DOWN;
        }
        if state.lx < -STICK_THRESHOLD {
            buttons |= LEFT;
        }
        if state.lx > STICK_THRESHOLD {
            buttons |= RIGHT;
        }
        buttons
    }
}
