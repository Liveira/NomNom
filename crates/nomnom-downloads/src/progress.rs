//! Parsing of the machine-readable lines we ask yt-dlp to print.

use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub finished: bool,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: Option<u64>,
    pub eta_secs: Option<u64>,
}

impl Progress {
    pub fn fraction(&self) -> Option<f32> {
        let total = self.total.filter(|t| *t > 0)?;
        Some((self.downloaded as f64 / total as f64).clamp(0.0, 1.0) as f32)
    }
}

#[derive(Debug, PartialEq)]
pub enum OutputLine {
    Progress(Progress),
    FinalPath(PathBuf),
}

fn as_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64))
}

/// Understands `NOMNOM {json}` progress lines and `NOMNOM_PATH <path>` lines. Everything else is
/// ignored.
pub fn parse_line(line: &str) -> Option<OutputLine> {
    let line = line.trim();
    if let Some(path) = line.strip_prefix("NOMNOM_PATH ") {
        let path = path.trim();
        return (!path.is_empty()).then(|| OutputLine::FinalPath(PathBuf::from(path)));
    }
    let json = line.strip_prefix("NOMNOM ")?;
    let v: Value = serde_json::from_str(json).ok()?;
    let status = v.get("status")?.as_str()?;
    let downloaded = v.get("downloaded_bytes").and_then(as_u64).unwrap_or(0);
    let total = v
        .get("total_bytes")
        .and_then(as_u64)
        .or_else(|| v.get("total_bytes_estimate").and_then(as_u64));
    Some(OutputLine::Progress(Progress {
        finished: status == "finished",
        downloaded,
        total,
        speed_bps: v.get("speed").and_then(as_u64),
        eta_secs: v.get("eta").and_then(as_u64),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_with_exact_total() {
        let line = r#"NOMNOM {"status":"downloading","downloaded_bytes":500,"total_bytes":1000,"speed":2048.5,"eta":12}"#;
        let Some(OutputLine::Progress(p)) = parse_line(line) else {
            panic!("not progress")
        };
        assert_eq!(p.fraction(), Some(0.5));
        assert_eq!(p.speed_bps, Some(2048));
        assert_eq!(p.eta_secs, Some(12));
        assert!(!p.finished);
    }

    #[test]
    fn falls_back_to_estimated_total_and_tolerates_nulls() {
        let line = r#"NOMNOM {"status":"downloading","downloaded_bytes":10,"total_bytes":null,"total_bytes_estimate":40.0,"speed":null,"eta":null}"#;
        let Some(OutputLine::Progress(p)) = parse_line(line) else {
            panic!()
        };
        assert_eq!(p.fraction(), Some(0.25));
        assert_eq!(p.speed_bps, None);
    }

    #[test]
    fn unknown_total_gives_no_fraction() {
        let line = r#"NOMNOM {"status":"downloading","downloaded_bytes":10}"#;
        let Some(OutputLine::Progress(p)) = parse_line(line) else {
            panic!()
        };
        assert_eq!(p.fraction(), None);
    }

    #[test]
    fn finished_status_and_overshoot_are_handled() {
        let line = r#"NOMNOM {"status":"finished","downloaded_bytes":1200,"total_bytes":1000}"#;
        let Some(OutputLine::Progress(p)) = parse_line(line) else {
            panic!()
        };
        assert!(p.finished);
        assert_eq!(p.fraction(), Some(1.0));
    }

    #[test]
    fn parses_final_path_with_spaces() {
        assert_eq!(
            parse_line("NOMNOM_PATH C:\\stage\\My Video.mp4\r\n"),
            Some(OutputLine::FinalPath(PathBuf::from(
                "C:\\stage\\My Video.mp4"
            )))
        );
    }

    #[test]
    fn ignores_noise_and_garbage() {
        for l in [
            "",
            "[download] 10%",
            "NOMNOM {not json",
            "NOMNOM {}",
            "NOMNOM_PATH   ",
        ] {
            assert_eq!(parse_line(l), None, "{l:?}");
        }
    }
}
