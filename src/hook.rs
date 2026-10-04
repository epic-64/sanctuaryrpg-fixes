//! Import-table patching for the game executable.

use crate::win::{GetModuleHandleA, VirtualProtect};
use std::ffi::{c_void, CStr};

const PAGE_READWRITE: u32 = 0x04;

/// Redirects one of the executable's imports to `replacement` and returns the address
/// it pointed to before.
pub unsafe fn patch_import(dll: &str, symbol: &CStr, replacement: usize) -> Option<usize> {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    let read = |address: usize| (address as *const u32).read_unaligned() as usize;

    // PE32: the import directory is the second data directory of the optional header.
    let nt_headers = base + read(base + 0x3c);
    let mut descriptor = base + read(nt_headers + 0x80);
    loop {
        let (lookup_rva, name_rva, thunk_rva) =
            (read(descriptor), read(descriptor + 12), read(descriptor + 16));
        if name_rva == 0 {
            return None;
        }
        descriptor += 20;
        let name = CStr::from_ptr((base + name_rva) as *const _).to_string_lossy();
        if !name.eq_ignore_ascii_case(dll) || lookup_rva == 0 {
            continue;
        }

        for index in 0.. {
            let entry = read(base + lookup_rva + index * 4);
            if entry == 0 {
                break;
            }
            let by_ordinal = entry & 0x8000_0000 != 0;
            // Skip the two-byte hint in front of the name.
            if by_ordinal || CStr::from_ptr((base + entry + 2) as *const _) != symbol {
                continue;
            }
            let slot = (base + thunk_rva + index * 4) as *mut usize;
            let mut protection = 0;
            if VirtualProtect(slot as *mut c_void, 4, PAGE_READWRITE, &mut protection) == 0 {
                return None;
            }
            let previous = slot.read();
            slot.write(replacement);
            VirtualProtect(slot as *mut c_void, 4, protection, &mut protection);
            return Some(previous);
        }
    }
}
