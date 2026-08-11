#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("overmax {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if let Err(err) = overmax_app::ui::native_app::run_native_app() {
        eprintln!("overmax-rs failed: {err}");
        std::process::exit(1);
    }
}
