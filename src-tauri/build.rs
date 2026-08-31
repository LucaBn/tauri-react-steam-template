use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    let out_dir = env::var("OUT_DIR").unwrap();
    let _profile = env::var("PROFILE").unwrap();

    let target_dir = PathBuf::from(out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();

    let dll_name = "steam_api64.dll";
    
    if let Ok(entries) = fs::read_dir(target_dir.join("build")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("steamworks-sys") {
                let src_dll = path.join("out").join(dll_name);
                let dest_dll = target_dir.join(dll_name);
                
                if src_dll.exists() {
                    let _ = fs::copy(src_dll, dest_dll);
                }
            }
        }
    }
}