use std::{env, fs, path::PathBuf};

/// Exports we implement ourselves; everything else jumps straight into SDL_orig.dll.
const HOOKED: &[&str] = &["SDL_PollEvent", "SDL_WaitEvent"];

fn main() {
    println!("cargo:rerun-if-changed=sdl_exports.txt");
    let exports = fs::read_to_string("sdl_exports.txt").expect("sdl_exports.txt");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // link.exe refuses `name = SDL_orig.name` forwarders in this setup, so each forwarded
    // export is a one-instruction stub that jumps through a table filled in at load time.
    let mut def = String::from("EXPORTS\n");
    let mut names = String::new();
    let mut stubs = String::new();
    let mut count = 0;
    for line in exports.lines() {
        let mut fields = line.split_whitespace();
        let (Some(name), Some(ordinal)) = (fields.next(), fields.next()) else { continue };
        def.push_str(&format!("    {name} @{ordinal}\n"));
        if HOOKED.contains(&name) {
            continue;
        }
        names.push_str(&format!("    c\"{name}\",\n"));
        stubs.push_str(&format!(
            "    \".globl _{name}\",\n    \"_{name}:\",\n    \"jmp dword ptr [{{table}} + {}]\",\n",
            count * 4
        ));
        count += 1;
    }

    let forwards = format!(
        "pub const FORWARDED: [&std::ffi::CStr; {count}] = [\n{names}];\n\n\
         #[no_mangle]\n\
         pub static mut FORWARD_TABLE: [usize; {count}] = [0; {count}];\n\n\
         std::arch::global_asm!(\n{stubs}    table = sym FORWARD_TABLE,\n);\n"
    );
    fs::write(out_dir.join("forwards.rs"), forwards).unwrap();

    // This comes after rustc's own /DEF on the linker command line and replaces it.
    let def_path = out_dir.join("sdl.def");
    fs::write(&def_path, def).unwrap();
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def_path.display());
}
