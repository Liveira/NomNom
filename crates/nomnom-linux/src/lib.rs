//! Per-user Linux desktop integration: XDG autostart (`~/.config/autostart`), Firefox/LibreWolf
//! Native Messaging host manifests (`~/.mozilla/native-messaging-hosts`), and file manager
//! right-click actions for KDE Dolphin (`kio/servicemenus`), GNOME Files/Nautilus, and Nemo.

pub mod autostart;
pub mod native_host;
pub mod shell_menu;
pub mod startup;
