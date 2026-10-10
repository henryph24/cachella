#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    if let Some(code) = cachella_lib::pdf::entry() {
        std::process::exit(code);
    }
    cachella_lib::run();
}
