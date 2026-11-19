// Release builds are GUI-only: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod app;
mod backup;
mod browser;
mod clipboard;
mod commands;
mod events;
mod filetools;
mod focus;
mod goals;
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
mod rename;
mod shortcuts;
mod state;
mod storage;
mod storage_cleanup;
mod storage_history;
mod storage_ops;
mod storage_tools;
mod subscriptions;
mod toast;
mod translate;
mod tray;
mod updater;
mod windows;

fn main() {
    app::run();
}
