//! System monitor: processor, memory, disks, network and the busiest programs, read from the
//! operating system. It only looks; it never stops or changes anything. The numbers are read only
//! while the Monitor tab is open, or every few seconds if the tray tooltip is switched on.

use crate::state::AppState;
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};
use tauri::{AppHandle, Manager, State};

pub struct MonitorState {
    sys: Mutex<System>,
    nets: Mutex<Networks>,
    last_net: Mutex<Option<(Instant, u64, u64)>>,
}

impl Default for MonitorState {
    fn default() -> Self {
        let mut sys = System::new();
        // The first reading of the processor is always zero; this one is thrown away.
        sys.refresh_cpu_usage();
        MonitorState {
            sys: Mutex::new(sys),
            nets: Mutex::new(Networks::new_with_refreshed_list()),
            last_net: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub used: u64,
    pub removable: bool,
}

#[derive(Serialize)]
pub struct ProcInfo {
    pub name: String,
    pub pid: u32,
    pub cpu: f32,
    pub memory: u64,
}

#[derive(Serialize)]
pub struct Snapshot {
    /// Percent of all processors, 0 to 100.
    pub cpu: f32,
    pub cores: Vec<f32>,
    pub cpu_name: String,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disks: Vec<DiskInfo>,
    /// Bytes per second since the last reading.
    pub net_down: u64,
    pub net_up: u64,
    pub uptime_secs: u64,
    /// Highest temperature the system reports, if it reports any.
    pub temp_c: Option<f32>,
    pub by_cpu: Vec<ProcInfo>,
    pub by_memory: Vec<ProcInfo>,
}

fn procs(sys: &System, n: usize, key: impl Fn(&ProcInfo) -> f64) -> Vec<ProcInfo> {
    let mut all: Vec<ProcInfo> = sys
        .processes()
        .values()
        .map(|p| ProcInfo {
            name: p.name().to_string_lossy().into_owned(),
            pid: p.pid().as_u32(),
            cpu: p.cpu_usage(),
            memory: p.memory(),
        })
        .collect();
    all.sort_by(|a, b| key(b).total_cmp(&key(a)));
    all.truncate(n);
    all
}

/// Merges programs that run as several processes (a browser) into one row.
pub fn merge_by_name(list: Vec<ProcInfo>) -> Vec<ProcInfo> {
    let mut out: Vec<ProcInfo> = Vec::new();
    for p in list {
        match out
            .iter_mut()
            .find(|o| o.name.eq_ignore_ascii_case(&p.name))
        {
            Some(o) => {
                o.cpu += p.cpu;
                o.memory += p.memory;
            }
            None => out.push(p),
        }
    }
    out
}

#[tauri::command]
pub async fn mon_snapshot(mon: State<'_, MonitorState>) -> Result<Snapshot, String> {
    // Reading takes a few milliseconds; keep it off the thread that draws the window.
    let (cpu, cores, cpu_name, mem_used, mem_total, swap_used, swap_total, by_cpu, by_memory) = {
        let mut sys = mon.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let cores: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let n = sys.cpus().len().max(1) as f32;
        let all: Vec<ProcInfo> = procs(&sys, usize::MAX, |p| f64::from(p.cpu));
        let merged = merge_by_name(all);
        let mut by_cpu = merged
            .iter()
            .map(|p| ProcInfo {
                name: p.name.clone(),
                pid: p.pid,
                cpu: p.cpu / n,
                memory: p.memory,
            })
            .collect::<Vec<_>>();
        by_cpu.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
        by_cpu.truncate(8);
        let mut by_memory = merged;
        by_memory.sort_by_key(|p| std::cmp::Reverse(p.memory));
        by_memory.truncate(8);
        (
            sys.global_cpu_usage(),
            cores,
            sys.cpus()
                .first()
                .map(|c| c.brand().trim().to_string())
                .unwrap_or_default(),
            sys.used_memory(),
            sys.total_memory(),
            sys.used_swap(),
            sys.total_swap(),
            by_cpu,
            by_memory,
        )
    };

    let disks: Vec<DiskInfo> = Disks::new_with_refreshed_list()
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| DiskInfo {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            total: d.total_space(),
            used: d.total_space().saturating_sub(d.available_space()),
            removable: d.is_removable(),
        })
        .collect();

    let (net_down, net_up) = {
        let mut nets = mon.nets.lock().unwrap();
        nets.refresh(true);
        let (rx, tx) = nets.iter().fold((0u64, 0u64), |(r, t), (_, n)| {
            (r + n.total_received(), t + n.total_transmitted())
        });
        let now = Instant::now();
        let mut last = mon.last_net.lock().unwrap();
        let rate = match *last {
            Some((at, lrx, ltx)) => {
                let secs = now.duration_since(at).as_secs_f64().max(0.2);
                (
                    (rx.saturating_sub(lrx) as f64 / secs) as u64,
                    (tx.saturating_sub(ltx) as f64 / secs) as u64,
                )
            }
            None => (0, 0),
        };
        *last = Some((now, rx, tx));
        rate
    };

    let temp_c = Components::new_with_refreshed_list()
        .iter()
        .filter_map(|c| c.temperature())
        .filter(|t| t.is_finite() && *t > 0.0 && *t < 130.0)
        .fold(None, |m: Option<f32>, t| Some(m.map_or(t, |x| x.max(t))));

    Ok(Snapshot {
        cpu,
        cores,
        cpu_name,
        mem_used,
        mem_total,
        swap_used,
        swap_total,
        disks,
        net_down,
        net_up,
        uptime_secs: System::uptime(),
        temp_c,
        by_cpu,
        by_memory,
    })
}

/// A short line for the tray tooltip: "CPU 12% · Memory 8.1 of 16 GB".
pub fn tooltip_line(cpu: f32, used: u64, total: u64) -> String {
    let gb = |b: u64| b as f64 / 1_073_741_824.0;
    format!("CPU {:.0}% · {:.1} of {:.0} GB", cpu, gb(used), gb(total))
}

/// While it is switched on, puts the processor and memory in the tray tooltip every few seconds.
pub fn spawn_tray(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(4));
        let mut shown = false;
        loop {
            tick.tick().await;
            let on = app.state::<AppState>().settings.get().monitor.tray_tooltip;
            if !on {
                if shown {
                    crate::tray::set_extra(&app, None);
                    shown = false;
                }
                continue;
            }
            let mon = app.state::<MonitorState>();
            let line = {
                let mut sys = mon.sys.lock().unwrap();
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                tooltip_line(
                    sys.global_cpu_usage(),
                    sys.used_memory(),
                    sys.total_memory(),
                )
            };
            crate::tray::set_extra(&app, Some(line));
            shown = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, cpu: f32, memory: u64) -> ProcInfo {
        ProcInfo {
            name: name.into(),
            pid: 1,
            cpu,
            memory,
        }
    }

    #[test]
    fn programs_with_many_processes_become_one_row() {
        let merged = merge_by_name(vec![
            p("chrome.exe", 3.0, 100),
            p("code.exe", 1.0, 50),
            p("Chrome.exe", 2.0, 200),
        ]);
        assert_eq!(merged.len(), 2);
        let chrome = merged
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case("chrome.exe"))
            .unwrap();
        assert_eq!((chrome.cpu, chrome.memory), (5.0, 300));
    }

    #[test]
    fn the_tray_line_is_short_and_readable() {
        assert_eq!(
            tooltip_line(12.4, 8_589_934_592, 17_179_869_184),
            "CPU 12% · 8.0 of 16 GB"
        );
    }
}
