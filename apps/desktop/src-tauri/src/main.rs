// Release builds are GUI-only: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod browser;
mod commands;
mod events;
mod filetools;
mod integrations;
mod modules;
mod notifier;
mod organizer;
mod pipe_server;
mod quick;
mod shortcuts;
mod state;
mod storage;
mod storage_history;
mod storage_ops;
mod storage_tools;
mod tray;
mod windows;

fn main() {
    app::run();
}
