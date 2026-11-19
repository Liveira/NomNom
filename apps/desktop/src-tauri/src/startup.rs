//! The Startup page: what starts with the computer, which ones can be turned off, and (on Windows)
//! how much each one slowed the last start-ups, from Windows' own record of them.

use nomnom_core::startup::{self as rules, Effect};
use serde::Serialize;
use std::collections::HashMap;

type CmdResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    /// What the page sends back to turn it off or on.
    pub id: String,
    pub name: String,
    pub command: String,
    /// The program's name in lower case, to match against the start-up record.
    pub program: Option<String>,
    pub enabled: bool,
    pub can_change: bool,
    /// "Your Run list", "Startup folder", "Every user" and so on.
    pub from: String,
    pub icon_url: Option<String>,
    pub effect: Option<Effect>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub items: Vec<Item>,
    /// How long the last start-up took, when Windows says.
    pub last_boot_ms: Option<u64>,
    /// Whether per-program slowdowns are known on this system.
    pub has_record: bool,
}

#[cfg(windows)]
fn read_record(channel_query: &str, count: u32) -> String {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("wevtutil")
        .args([
            "qe",
            "Microsoft-Windows-Diagnostics-Performance/Operational",
            &format!("/q:*[System[(EventID={channel_query})]]"),
            &format!("/c:{count}"),
            "/rd:true",
            "/f:xml",
        ])
        .creation_flags(0x0800_0000) // no console window
        .output();
    out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

#[cfg(windows)]
fn items() -> (Vec<Item>, Option<u64>, bool) {
    use nomnom_win::startup::{list, Kind};
    let slow = rules::parse_slowdowns(&read_record("101", 300));
    let effects: HashMap<String, Effect> = rules::effects(&slow);
    let boot = rules::parse_boot_time(&read_record("100", 1));
    let items = list()
        .into_iter()
        .map(|e| {
            let kind = match e.kind {
                Kind::UserRun => "user_run",
                Kind::MachineRun => "machine_run",
                Kind::UserFolder => "user_folder",
                Kind::MachineFolder => "machine_folder",
            };
            let program = rules::program_name(&e.command);
            let exe = rules::program_of(&e.command);
            let icon_url = exe.as_deref().and_then(|p| {
                nomnom_activity::extract_app_meta(Some(p), None)
                    .icon_png
                    .as_deref()
                    .map(nomnom_activity::encode_icon_data_url)
            });
            Item {
                id: format!("{kind}:{}", e.name),
                effect: program.as_ref().and_then(|p| effects.get(p)).cloned(),
                name: e.name,
                command: e.command,
                program,
                enabled: e.enabled,
                can_change: e.kind.can_change(),
                from: match e.kind {
                    Kind::UserRun => "Your start-up list",
                    Kind::MachineRun => "Every user's start-up list",
                    Kind::UserFolder => "Your Startup folder",
                    Kind::MachineFolder => "Every user's Startup folder",
                }
                .into(),
                icon_url,
            }
        })
        .collect();
    (items, boot, !slow.is_empty() || boot.is_some())
}

#[cfg(not(windows))]
fn items() -> (Vec<Item>, Option<u64>, bool) {
    let items = nomnom_linux::startup::list()
        .into_iter()
        .map(|e| Item {
            id: e.file.clone(),
            program: rules::program_name(&e.command),
            name: e.name,
            command: e.command,
            enabled: e.enabled,
            can_change: true,
            from: if e.own {
                "Your start-up list".into()
            } else {
                "Set up by the system".into()
            },
            icon_url: None,
            effect: None,
        })
        .collect();
    (items, None, false)
}

#[tauri::command]
pub async fn su_list() -> CmdResult<Overview> {
    let (items, last_boot_ms, has_record) = tokio::task::spawn_blocking(items)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Overview {
        items,
        last_boot_ms,
        has_record,
    })
}

#[tauri::command]
pub fn su_set(id: String, enabled: bool) -> CmdResult<()> {
    #[cfg(windows)]
    {
        use nomnom_win::startup::{set_enabled, Kind};
        let (kind, name) = id.split_once(':').ok_or("That isn't a start-up entry.")?;
        let kind = match kind {
            "user_run" => Kind::UserRun,
            "machine_run" => Kind::MachineRun,
            "user_folder" => Kind::UserFolder,
            "machine_folder" => Kind::MachineFolder,
            _ => return Err("That isn't a start-up entry.".into()),
        };
        // Only a name that is really in the list can be changed.
        if !nomnom_win::startup::list()
            .iter()
            .any(|e| e.kind == kind && e.name == name)
        {
            return Err("That start-up entry is gone.".into());
        }
        set_enabled(kind, name, enabled)
    }
    #[cfg(not(windows))]
    {
        nomnom_linux::startup::set_enabled(&id, enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_entry_cannot_be_changed() {
        assert!(su_set("garbage".into(), false).is_err());
        assert!(su_set("user_run:NoSuchNomNomEntry".into(), false).is_err());
    }
}
