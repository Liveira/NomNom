//! What the System Monitor adds beyond the processor and memory: the graphics card, the battery,
//! and the health of each disk. It only reads. Disk health is also looked at once a day, and NomNom
//! says so when a disk needs attention, so there is time to copy things off it.

use crate::i18n::{lang_of, Lang};
use crate::state::AppState;
use crate::toast::{self, Toast};
use nomnom_core::hardware::{self as rules, DiskHealth, Gpu, Verdict};
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

type CmdResult<T> = Result<T, String>;

#[derive(Debug, Serialize)]
pub struct Battery {
    pub percent: u8,
    pub plugged: bool,
    pub charging: bool,
    pub minutes_left: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct Hardware {
    pub gpus: Vec<Gpu>,
    pub battery: Option<Battery>,
}

#[derive(Debug, Serialize)]
pub struct DiskView {
    #[serde(flatten)]
    pub disk: DiskHealth,
    pub verdict: Verdict,
    pub why: Vec<String>,
}

fn gpus() -> Vec<Gpu> {
    let mut cmd = std::process::Command::new("nvidia-smi");
    cmd.args([
        &format!("--query-gpu={}", rules::NVIDIA_QUERY),
        "--format=csv,noheader,nounits",
    ])
    .stdin(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| rules::parse_nvidia_smi(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

#[cfg(windows)]
fn battery() -> Option<Battery> {
    nomnom_win::power::battery().map(|b| Battery {
        percent: b.percent,
        plugged: b.plugged,
        charging: b.charging,
        minutes_left: b.minutes_left,
    })
}

#[cfg(not(windows))]
fn battery() -> Option<Battery> {
    let dir = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for e in dir.flatten() {
        let p = e.path();
        let kind = std::fs::read_to_string(p.join("type")).unwrap_or_default();
        if kind.trim() != "Battery" {
            continue;
        }
        let percent: u8 = std::fs::read_to_string(p.join("capacity"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let status = std::fs::read_to_string(p.join("status")).unwrap_or_default();
        let status = status.trim();
        return Some(Battery {
            percent: percent.min(100),
            plugged: status != "Discharging",
            charging: status == "Charging",
            minutes_left: None,
        });
    }
    None
}

/// The graphics card and the battery, now.
#[tauri::command]
pub async fn hw_status() -> CmdResult<Hardware> {
    tokio::task::spawn_blocking(|| Hardware {
        gpus: gpus(),
        battery: battery(),
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(windows)]
const DISK_SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$rows = @()
foreach ($d in Get-PhysicalDisk) {
  $r = $d | Get-StorageReliabilityCounter
  $rows += [pscustomobject]@{ name = [string]$d.FriendlyName; media = [string]$d.MediaType; health = [string]$d.HealthStatus; status = [string]$d.OperationalStatus; size = $d.Size; temp = $r.Temperature; wear = $r.Wear; hours = $r.PowerOnHours; readErrors = $r.ReadErrorsTotal; writeErrors = $r.WriteErrorsTotal }
}
if ($rows.Count -eq 0) { '[]' } else { ConvertTo-Json -InputObject @($rows) -Compress }
"#;

#[cfg(windows)]
fn read_disks() -> CmdResult<Vec<DiskHealth>> {
    use std::os::windows::process::CommandExt;
    let powershell = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| r"C:\Windows".into())
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let out = std::process::Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", DISK_SCRIPT])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| format!("Couldn't ask Windows about the disks: {e}"))?;
    Ok(rules::parse_disk_health(&String::from_utf8_lossy(
        &out.stdout,
    )))
}

#[cfg(not(windows))]
fn read_disks() -> CmdResult<Vec<DiskHealth>> {
    Err("Reading disk health needs smartctl and administrator rights, so NomNom does not do it here.".into())
}

fn view(disks: Vec<DiskHealth>) -> Vec<DiskView> {
    disks
        .into_iter()
        .map(|d| {
            let (verdict, why) = rules::verdict(&d);
            DiskView {
                disk: d,
                verdict,
                why,
            }
        })
        .collect()
}

/// How each disk is doing, from the system's own counters.
#[tauri::command]
pub async fn hw_disks() -> CmdResult<Vec<DiskView>> {
    let list = tokio::task::spawn_blocking(read_disks)
        .await
        .map_err(|e| e.to_string())??;
    Ok(view(list))
}

/// The names of disks already reported, so the same one isn't announced every day.
static TOLD: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn alert_text(lang: Lang, name: &str, bad: bool) -> (String, String) {
    match (lang, bad) {
        (Lang::En, true) => (
            "A disk needs attention now".into(),
            format!("{name} looks like it is failing. Copy what matters off it."),
        ),
        (Lang::En, false) => (
            "A disk to keep an eye on".into(),
            format!("{name} shows signs of wear. Make sure you have a backup."),
        ),
        (Lang::Pt, true) => (
            "Um disco precisa de atenção agora".into(),
            format!("{name} parece estar falhando. Copie o que importa para outro lugar."),
        ),
        (Lang::Pt, false) => (
            "Um disco para ficar de olho".into(),
            format!("{name} mostra sinais de desgaste. Confira se você tem um backup."),
        ),
        (Lang::Es, true) => (
            "Un disco necesita atención ahora".into(),
            format!("{name} parece estar fallando. Copia lo importante a otro sitio."),
        ),
        (Lang::Es, false) => (
            "Un disco para vigilar".into(),
            format!(
                "{name} muestra señales de desgaste. Comprueba que tienes una copia de seguridad."
            ),
        ),
    }
}

/// Once a day (and a little after start), when it is switched on.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(90)).await;
        loop {
            let on = app.state::<AppState>().settings.get().monitor.disk_alerts;
            if on {
                if let Ok(Ok(list)) = tokio::task::spawn_blocking(read_disks).await {
                    let lang = lang_of(&app.state::<AppState>().settings.get());
                    for v in view(list) {
                        if v.verdict == Verdict::Good {
                            continue;
                        }
                        let key = format!("{}:{:?}", v.disk.name, v.verdict);
                        let fresh = {
                            let mut told = TOLD.lock().unwrap();
                            if told.contains(&key) {
                                false
                            } else {
                                told.push(key);
                                true
                            }
                        };
                        if fresh {
                            let (title, body) =
                                alert_text(lang, &v.disk.name, v.verdict == Verdict::Bad);
                            toast::show(&app, Toast::new("warning", title, body).opens("monitor"));
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}

/// For the page: switch the daily check on or off is a setting; this only says whether it is on.
#[tauri::command]
pub fn hw_alerts_on(state: State<'_, AppState>) -> bool {
    state.settings.get().monitor.disk_alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failing_disk_gets_the_urgent_words_in_every_language() {
        for lang in [Lang::En, Lang::Pt, Lang::Es] {
            let (title, body) = alert_text(lang, "Disk A", true);
            assert!(!title.is_empty() && body.contains("Disk A"));
            let (t2, _) = alert_text(lang, "Disk A", false);
            assert_ne!(title, t2);
        }
    }

    #[test]
    fn views_carry_their_verdict() {
        let v = view(vec![DiskHealth {
            name: "X".into(),
            health: "Unhealthy".into(),
            ..Default::default()
        }]);
        assert_eq!(v[0].verdict, Verdict::Bad);
        assert!(!v[0].why.is_empty());
    }
}
