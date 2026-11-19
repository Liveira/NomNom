// Release builds are GUI-only: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod app;
mod backup;
mod browser;
mod commands;
mod events;
mod filetools;
mod focus;
mod graphics;
mod i18n;
mod integrations;
mod memory;
mod modules;
mod monitor;
mod notifier;
mod organizer;
mod pdftools;
mod pipe_server;
mod quick;
mod shortcuts;
mod state;
mod storage;
mod storage_cleanup;
mod storage_history;
mod storage_ops;
mod storage_tools;
mod toast;
mod tray;
mod updater;
mod windows;

fn main() {
    app::run();
}
