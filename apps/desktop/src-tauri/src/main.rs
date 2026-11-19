// Release builds are GUI-only: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod activity;
mod app;
mod backup;
mod browser;
mod clipboard;
mod colorpick;
mod commands;
mod events;
mod filetools;
mod focus;
mod goals;
mod graphics;
mod i18n;
mod integrations;
mod launcher;
mod memory;
mod mini;
mod modules;
mod monitor;
mod notes;
mod notifications;
mod notifier;
mod ocr;
mod organizer;
mod pdftools;
mod phone;
mod pipe_server;
mod quick;
mod reminders;
mod rename;
mod settings_io;
mod shortcuts;
mod startup;
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
