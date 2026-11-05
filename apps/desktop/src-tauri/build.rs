fn main() {
    // Without these, replacing an icon file does not rebuild the executable, and the taskbar and
    // tray keep showing the old picture.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
