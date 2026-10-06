// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Propagate a failed startup into the exit code rather than discarding it.
    if let Err(e) = oxeylyzer_tauri_lib::run() {
        eprintln!("Oxeylyzer failed to start: {e}");
        std::process::exit(1);
    }
}
