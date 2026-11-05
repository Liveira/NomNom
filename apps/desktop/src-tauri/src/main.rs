// Release builds are GUI-only: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod events;
mod modules;
mod notifier;
mod state;
mod tray;
mod windows;

fn main() {
    app::run();
}
