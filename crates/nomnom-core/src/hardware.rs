//! Reading the machine's own report of itself: the graphics card (from `nvidia-smi`), the state of
//! each disk (from Windows' storage reliability counters) and a verdict on whether a disk needs
//! attention. Running the tools is the app's job; this file only reads what they print.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gpu {
    pub name: String,
    /// Percent of the card's time that was busy.
    pub load: Option<f32>,
    pub temp_c: Option<f32>,
    pub mem_used_mb: Option<u64>,
    pub mem_total_mb: Option<u64>,
    /// The fan's speed as a percent of its top speed.
    pub fan_percent: Option<f32>,
    pub power_w: Option<f32>,
}

fn num(s: &str) -> Option<f32> {
    let t = s.trim();
    // "[N/A]" and "N/A" are what the tool prints for what a card doesn't report.
    if t.is_empty() || t.contains("N/A") || t.contains("Not Supported") {
        return None;
    }
    t.parse::<f32>().ok()
}

/// What `nvidia-smi --query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total,fan.speed,power.draw
/// --format=csv,noheader,nounits` prints: one line per card.
pub fn parse_nvidia_smi(output: &str) -> Vec<Gpu> {
    output
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            if f.len() < 7 || f[0].is_empty() {
                return None;
            }
            Some(Gpu {
                name: f[0].to_string(),
                load: num(f[1]),
                temp_c: num(f[2]),
                mem_used_mb: num(f[3]).map(|v| v as u64),
                mem_total_mb: num(f[4]).map(|v| v as u64),
                fan_percent: num(f[5]),
                power_w: num(f[6]),
            })
        })
        .collect()
}

pub const NVIDIA_QUERY: &str =
    "name,utilization.gpu,temperature.gpu,memory.used,memory.total,fan.speed,power.draw";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DiskHealth {
    pub name: String,
    /// "SSD", "HDD" or "Unspecified".
    pub media: String,
    /// "Healthy", "Warning" or "Unhealthy".
    pub health: String,
    pub status: String,
    pub size: u64,
    pub temp_c: Option<f32>,
    /// How worn out an SSD is, 0 to 100 (100 means used up).
    pub wear: Option<f32>,
    pub hours: Option<u64>,
    pub read_errors: Option<u64>,
    pub write_errors: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Good,
    /// Worth keeping an eye on, and having a backup of.
    Watch,
    /// Copy the important things off it now.
    Bad,
}

/// Is this disk fine, to be watched, or failing? And why, in a sentence for people.
pub fn verdict(d: &DiskHealth) -> (Verdict, Vec<String>) {
    let mut why = Vec::new();
    let mut v = Verdict::Good;
    let mut raise = |to: Verdict, text: String, v: &mut Verdict| {
        if to == Verdict::Bad || *v == Verdict::Good {
            *v = to;
        }
        why.push(text);
    };
    match d.health.as_str() {
        "Healthy" | "" => {}
        "Warning" => raise(
            Verdict::Watch,
            "Windows says this disk has a warning.".into(),
            &mut v,
        ),
        _ => raise(
            Verdict::Bad,
            "Windows says this disk is not healthy.".into(),
            &mut v,
        ),
    }
    if let Some(w) = d.wear {
        if w >= 95.0 {
            raise(Verdict::Bad, format!("It is {w:.0}% worn out."), &mut v);
        } else if w >= 80.0 {
            raise(Verdict::Watch, format!("It is {w:.0}% worn out."), &mut v);
        }
    }
    if let Some(t) = d.temp_c {
        if t >= 70.0 {
            raise(
                Verdict::Watch,
                format!("It is running hot ({t:.0} °C)."),
                &mut v,
            );
        }
    }
    let errors = d.read_errors.unwrap_or(0) + d.write_errors.unwrap_or(0);
    if errors > 0 {
        raise(
            if errors >= 100 {
                Verdict::Bad
            } else {
                Verdict::Watch
            },
            format!("It has reported {errors} read or write errors."),
            &mut v,
        );
    }
    (v, why)
}

/// What the PowerShell script printed: an array (or one object) of disks.
pub fn parse_disk_health(json: &str) -> Vec<DiskHealth> {
    #[derive(Deserialize)]
    struct Row {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        media: Option<String>,
        #[serde(default)]
        health: Option<String>,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        size: Option<f64>,
        #[serde(default)]
        temp: Option<f64>,
        #[serde(default)]
        wear: Option<f64>,
        #[serde(default)]
        hours: Option<f64>,
        #[serde(default, rename = "readErrors")]
        read_errors: Option<f64>,
        #[serde(default, rename = "writeErrors")]
        write_errors: Option<f64>,
    }
    let text = json.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Row> = serde_json::from_str::<Vec<Row>>(text)
        .or_else(|_| serde_json::from_str::<Row>(text).map(|r| vec![r]))
        .unwrap_or_default();
    rows.into_iter()
        .map(|r| DiskHealth {
            name: r.name.unwrap_or_else(|| "Disk".into()),
            media: r.media.unwrap_or_default(),
            health: r.health.unwrap_or_default(),
            status: r.status.unwrap_or_default(),
            size: r.size.map(|s| s.max(0.0) as u64).unwrap_or(0),
            // A temperature of 0 means "not reported".
            temp_c: r.temp.filter(|t| *t > 0.0).map(|t| t as f32),
            wear: r
                .wear
                .filter(|w| (0.0..=100.0).contains(w))
                .map(|w| w as f32),
            hours: r.hours.filter(|h| *h >= 0.0).map(|h| h as u64),
            read_errors: r.read_errors.filter(|e| *e >= 0.0).map(|e| e as u64),
            write_errors: r.write_errors.filter(|e| *e >= 0.0).map(|e| e as u64),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_report_is_read_and_missing_parts_are_left_out() {
        let out = "NVIDIA GeForce RTX 3050 Laptop GPU, 12, 47, 812, 4096, [N/A], 14.82\nNVIDIA Other, 0, 35, 100, 2048, 30, [Not Supported]\n";
        let g = parse_nvidia_smi(out);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].name, "NVIDIA GeForce RTX 3050 Laptop GPU");
        assert_eq!(g[0].load, Some(12.0));
        assert_eq!(g[0].temp_c, Some(47.0));
        assert_eq!(g[0].mem_total_mb, Some(4096));
        assert_eq!(g[0].fan_percent, None);
        assert_eq!(g[0].power_w, Some(14.82));
        assert_eq!(g[1].power_w, None);
        assert_eq!(g[1].fan_percent, Some(30.0));
    }

    #[test]
    fn nonsense_from_the_tool_gives_no_cards() {
        assert!(parse_nvidia_smi("").is_empty());
        assert!(
            parse_nvidia_smi("NVIDIA-SMI has failed because it couldn't communicate").is_empty()
        );
        assert!(parse_nvidia_smi(", 1, 2, 3, 4, 5, 6").is_empty());
    }

    #[test]
    fn disks_are_read_from_the_script_output() {
        let json = r#"[{"name":"Samsung SSD 970","media":"SSD","health":"Healthy","status":"OK","size":512110190592,"temp":41,"wear":3,"hours":1200,"readErrors":0,"writeErrors":0},
                       {"name":"Old HDD","media":"HDD","health":"Warning","status":"OK","size":1000204886016,"temp":0,"wear":null,"hours":null,"readErrors":null,"writeErrors":null}]"#;
        let d = parse_disk_health(json);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].temp_c, Some(41.0));
        assert_eq!(d[0].wear, Some(3.0));
        assert_eq!(d[1].temp_c, None);
        assert_eq!(d[1].wear, None);
        assert_eq!(
            parse_disk_health(r#"{"name":"One","health":"Healthy"}"#).len(),
            1
        );
        assert!(parse_disk_health("").is_empty());
        assert!(parse_disk_health("oops").is_empty());
    }

    #[test]
    fn the_verdict_says_why() {
        let good = DiskHealth {
            health: "Healthy".into(),
            wear: Some(5.0),
            temp_c: Some(40.0),
            ..Default::default()
        };
        assert_eq!(verdict(&good), (Verdict::Good, vec![]));
        let watch = DiskHealth {
            health: "Healthy".into(),
            wear: Some(85.0),
            ..Default::default()
        };
        let (v, why) = verdict(&watch);
        assert_eq!(v, Verdict::Watch);
        assert!(why[0].contains("85%"));
        let bad = DiskHealth {
            health: "Unhealthy".into(),
            temp_c: Some(75.0),
            ..Default::default()
        };
        let (v, why) = verdict(&bad);
        assert_eq!(v, Verdict::Bad);
        assert_eq!(why.len(), 2);
        let worn = DiskHealth {
            health: "Healthy".into(),
            wear: Some(97.0),
            ..Default::default()
        };
        assert_eq!(verdict(&worn).0, Verdict::Bad);
        let errors = DiskHealth {
            health: "Healthy".into(),
            read_errors: Some(3),
            ..Default::default()
        };
        assert_eq!(verdict(&errors).0, Verdict::Watch);
        let many = DiskHealth {
            health: "Healthy".into(),
            read_errors: Some(80),
            write_errors: Some(40),
            ..Default::default()
        };
        assert_eq!(verdict(&many).0, Verdict::Bad);
    }
}
