//! For the automatic mode: a new file is only touched once it has stopped changing. Events from
//! the file system say "something happened"; this decides when it is safe to look at the file.

use crate::model::Fingerprint;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The file disappeared (renamed, deleted, or finished downloading under another name).
    Gone,
    /// Still changing, or not quiet for long enough yet.
    Waiting,
    /// Unchanged for the whole quiet period.
    Ready,
}

pub struct Settler {
    quiet: Duration,
    seen: HashMap<PathBuf, (Fingerprint, Instant)>,
    /// Never track more files than this; a flood of events must not grow memory without limit.
    max: usize,
}

impl Settler {
    pub fn new(quiet: Duration) -> Self {
        Settler {
            quiet,
            seen: HashMap::new(),
            max: 500,
        }
    }

    pub fn pending(&self) -> usize {
        self.seen.len()
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.seen.keys().cloned().collect()
    }

    /// Records what the file looks like right now (`None` if it is gone) and says what to do.
    pub fn observe(&mut self, path: &Path, now_fp: Option<Fingerprint>, now: Instant) -> Verdict {
        let Some(fp) = now_fp else {
            self.seen.remove(path);
            return Verdict::Gone;
        };
        match self.seen.get(path) {
            Some((old, since)) if *old == fp => {
                if now.duration_since(*since) >= self.quiet {
                    self.seen.remove(path);
                    Verdict::Ready
                } else {
                    Verdict::Waiting
                }
            }
            _ => {
                if self.seen.len() >= self.max && !self.seen.contains_key(path) {
                    return Verdict::Waiting; // over the limit: ignore until something clears
                }
                self.seen.insert(path.to_path_buf(), (fp, now));
                Verdict::Waiting
            }
        }
    }

    /// Stops tracking a file (it was handled, or isn't something to organize).
    pub fn forget(&mut self, path: &Path) {
        self.seen.remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(size: u64) -> Fingerprint {
        Fingerprint {
            size,
            mtime: 1,
            volume: 1,
            index: 1,
        }
    }

    #[test]
    fn a_file_is_ready_only_after_it_has_been_unchanged_for_the_whole_quiet_period() {
        let mut s = Settler::new(Duration::from_secs(5));
        let p = Path::new(r"C:\d\a.mp4");
        let t = Instant::now();
        assert_eq!(s.observe(p, Some(fp(10)), t), Verdict::Waiting);
        assert_eq!(
            s.observe(p, Some(fp(10)), t + Duration::from_secs(4)),
            Verdict::Waiting
        );
        assert_eq!(
            s.observe(p, Some(fp(10)), t + Duration::from_secs(5)),
            Verdict::Ready
        );
        assert_eq!(s.pending(), 0);
    }

    #[test]
    fn a_file_that_keeps_growing_never_becomes_ready() {
        let mut s = Settler::new(Duration::from_secs(5));
        let p = Path::new(r"C:\d\big.mkv");
        let t = Instant::now();
        for i in 0..20u64 {
            let v = s.observe(p, Some(fp(i * 1000)), t + Duration::from_secs(i * 3));
            assert_eq!(v, Verdict::Waiting, "step {i}");
        }
    }

    #[test]
    fn a_change_restarts_the_wait() {
        let mut s = Settler::new(Duration::from_secs(5));
        let p = Path::new(r"C:\d\a.mp4");
        let t = Instant::now();
        s.observe(p, Some(fp(1)), t);
        s.observe(p, Some(fp(2)), t + Duration::from_secs(4)); // changed
        assert_eq!(
            s.observe(p, Some(fp(2)), t + Duration::from_secs(8)),
            Verdict::Waiting
        );
        assert_eq!(
            s.observe(p, Some(fp(2)), t + Duration::from_secs(9)),
            Verdict::Ready
        );
    }

    #[test]
    fn a_file_that_vanishes_is_forgotten() {
        let mut s = Settler::new(Duration::from_secs(5));
        let p = Path::new(r"C:\d\a.crdownload");
        let t = Instant::now();
        s.observe(p, Some(fp(1)), t);
        assert_eq!(
            s.observe(p, None, t + Duration::from_secs(1)),
            Verdict::Gone
        );
        assert_eq!(s.pending(), 0);
    }

    #[test]
    fn a_flood_of_events_cannot_grow_the_list_without_limit() {
        let mut s = Settler::new(Duration::from_secs(5));
        let t = Instant::now();
        for i in 0..2_000 {
            s.observe(Path::new(&format!(r"C:\d\f{i}.mp4")), Some(fp(1)), t);
        }
        assert!(s.pending() <= 500);
    }
}
