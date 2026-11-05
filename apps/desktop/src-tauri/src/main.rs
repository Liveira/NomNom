// Release builds are GUI-only: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod browser;
mod commands;
mod events;
mod modules;
mod notifier;
mod pipe_server;
mod quick;
mod shortcuts;
mod state;
mod tray;
mod windows;

fn main() {
    app::run();
}
