//! Deterministic active-time state machine: counts foreground seconds only while the user is not
//! AFK, splits intervals across local hour/day boundaries, guards against OS sleep/hibernation
//! jumps, and batches flushes cleanly.

use crate::classify::{classify_app, AppIdentity};
use crate::probe::ForegroundSample;
use chrono::{Local, TimeZone};
use serde::Serialize;

/// Maximum elapsed wall-clock seconds credited to a single polling tick. If the computer sleeps or
/// hibernates for hours between two 2-second polls, the gap is discarded rather than counted.
const MAX_TICK_CREDIT_SECS: i64 = 15;

/// Flush accumulated active seconds at least every 20 seconds even if the user stays in one app.
const FLUSH_INTERVAL_SECS: i64 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourBucket {
    pub day: String,
    pub hour: u8,
    pub secs: u32,
    pub sessions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFlush {
    pub app: AppIdentity,
    /// Present when a completed session ended (`started_at`, `ended_at`, `active_secs`).
    pub finished_session: Option<(i64, i64, u32)>,
    pub hourly: Vec<HourBucket>,
    pub delta_secs: u32,
    pub seen_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LiveFocus {
    pub app_id: String,
    pub name: String,
    pub category: String,
    pub session_secs: u32,
    pub started_at: i64,
    pub is_afk: bool,
}

struct ActiveRun {
    app: AppIdentity,
    session_start: i64,
    last_seen: i64,
    total_session_secs: u32,
    unflushed_secs: u32,
    unflushed_buckets: Vec<HourBucket>,
    last_flush_at: i64,
    session_counted_in_hourly: bool,
}

pub struct ActiveTracker {
    last_tick_at: Option<i64>,
    current: Option<ActiveRun>,
    last_afk: bool,
}

impl Default for ActiveTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ActiveTracker {
    pub fn new() -> Self {
        Self {
            last_tick_at: None,
            current: None,
            last_afk: false,
        }
    }

    pub fn live(&self) -> Option<LiveFocus> {
        let run = self.current.as_ref()?;
        Some(LiveFocus {
            app_id: run.app.id.clone(),
            name: run.app.name.clone(),
            category: run.app.category.as_str().to_string(),
            session_secs: run.total_session_secs,
            started_at: run.session_start,
            is_afk: self.last_afk,
        })
    }

    /// Processes one foreground observation at `now` (Unix seconds) and returns any flushes ready
    /// to be persisted to SQLite.
    pub fn tick(
        &mut self,
        now: i64,
        sample: ForegroundSample,
        idle_limit_secs: u32,
        self_pid: u32,
        track_self: bool,
    ) -> Vec<SessionFlush> {
        let prev_tick = self.last_tick_at.replace(now);
        let raw_dt = prev_tick.map(|t| (now - t).max(0)).unwrap_or(0);
        let sleep_gap = raw_dt > MAX_TICK_CREDIT_SECS;
        let credit_dt = if sleep_gap { 0 } else { raw_dt as u32 };

        let mut flushes = Vec::new();
        if sleep_gap {
            if let Some(done) = self.finish_current() {
                flushes.push(done);
            }
        }

        let effective_limit = idle_limit_secs.clamp(30, 7200);
        let active_app = match sample {
            ForegroundSample::LockedOrNone => {
                self.last_afk = false;
                None
            }
            ForegroundSample::Idle { .. } => {
                self.last_afk = true;
                None
            }
            ForegroundSample::Active {
                raw,
                idle_secs,
                fullscreen_or_gamepad,
            } => {
                if !fullscreen_or_gamepad && idle_secs >= effective_limit {
                    self.last_afk = true;
                    None
                } else {
                    self.last_afk = false;
                    if !track_self && raw.pid == self_pid {
                        None
                    } else {
                        classify_app(&raw).filter(|a| track_self || a.id != "app:nomnom")
                    }
                }
            }
        };

        let Some(app) = active_app else {
            if let Some(done) = self.finish_current() {
                flushes.push(done);
            }
            return flushes;
        };

        // Did the focused app change?
        let same_app = self.current.as_ref().is_some_and(|r| r.app.id == app.id);
        if !same_app {
            if let Some(done) = self.finish_current() {
                flushes.push(done);
            }
            self.current = Some(ActiveRun {
                app,
                session_start: now,
                last_seen: now,
                total_session_secs: 0,
                unflushed_secs: 0,
                unflushed_buckets: Vec::new(),
                last_flush_at: now,
                session_counted_in_hourly: false,
            });
            return flushes;
        }

        if let Some(run) = self.current.as_mut() {
            if credit_dt > 0 {
                let interval_start = now - i64::from(credit_dt);
                let mark_session = !run.session_counted_in_hourly;
                add_interval_buckets(
                    &mut run.unflushed_buckets,
                    interval_start,
                    credit_dt,
                    mark_session,
                );
                run.session_counted_in_hourly = true;
                run.total_session_secs = run.total_session_secs.saturating_add(credit_dt);
                run.unflushed_secs = run.unflushed_secs.saturating_add(credit_dt);
            }
            run.last_seen = now;
            if run.unflushed_secs > 0 && (now - run.last_flush_at) >= FLUSH_INTERVAL_SECS {
                flushes.push(SessionFlush {
                    app: run.app.clone(),
                    finished_session: None,
                    hourly: std::mem::take(&mut run.unflushed_buckets),
                    delta_secs: std::mem::replace(&mut run.unflushed_secs, 0),
                    seen_at: now,
                });
                run.last_flush_at = now;
            }
        }

        flushes
    }

    /// Flushes any active session on app shutdown so nothing in memory is lost.
    pub fn flush_all(&mut self) -> Option<SessionFlush> {
        self.finish_current()
    }

    fn finish_current(&mut self) -> Option<SessionFlush> {
        let mut run = self.current.take()?;
        let finished_session = (run.total_session_secs >= 2).then_some((
            run.session_start,
            run.last_seen.max(run.session_start),
            run.total_session_secs,
        ));
        if run.unflushed_secs == 0 && finished_session.is_none() {
            return None;
        }
        Some(SessionFlush {
            app: run.app,
            finished_session,
            hourly: std::mem::take(&mut run.unflushed_buckets),
            delta_secs: std::mem::replace(&mut run.unflushed_secs, 0),
            seen_at: run.last_seen,
        })
    }
}

fn add_interval_buckets(
    buckets: &mut Vec<HourBucket>,
    start_unix: i64,
    mut remaining: u32,
    mut first_session_flag: bool,
) {
    let mut cursor = start_unix;
    while remaining > 0 {
        let dt = Local
            .timestamp_opt(cursor, 0)
            .single()
            .unwrap_or_else(Local::now);
        let day = dt.format("%Y-%m-%d").to_string();
        let hour = dt.format("%H").to_string().parse::<u8>().unwrap_or(0);
        let secs_into_hour = dt.format("%M").to_string().parse::<u32>().unwrap_or(0) * 60
            + dt.format("%S").to_string().parse::<u32>().unwrap_or(0);
        let room = (3600u32.saturating_sub(secs_into_hour)).max(1);
        let chunk = remaining.min(room);
        let add_sess = u32::from(first_session_flag);
        first_session_flag = false;

        if let Some(existing) = buckets.iter_mut().find(|b| b.day == day && b.hour == hour) {
            existing.secs = existing.secs.saturating_add(chunk);
            existing.sessions = existing.sessions.saturating_add(add_sess);
        } else {
            buckets.push(HourBucket {
                day,
                hour,
                secs: chunk,
                sessions: add_sess,
            });
        }
        remaining -= chunk;
        cursor += i64::from(chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::RawForeground;

    fn sample(exe: &str, idle_secs: u32) -> ForegroundSample {
        ForegroundSample::Active {
            raw: RawForeground {
                pid: 500,
                exe_path: Some(exe.into()),
                wm_class: None,
                desktop_name: None,
            },
            idle_secs,
            fullscreen_or_gamepad: false,
        }
    }

    #[test]
    fn counts_only_active_non_idle_foreground_time_and_discards_sleep_gaps() {
        let mut tr = ActiveTracker::new();
        assert!(tr
            .tick(1_700_000_000, sample("Discord.exe", 0), 300, 1, false)
            .is_empty());
        assert!(tr
            .tick(1_700_000_002, sample("Discord.exe", 1), 300, 1, false)
            .is_empty());
        assert_eq!(tr.live().unwrap().session_secs, 2);

        // User walks away (idle 305s >= 300s limit): session finishes without adding idle time.
        let out = tr.tick(1_700_000_004, sample("Discord.exe", 305), 300, 1, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].delta_secs, 2);
        assert_eq!(
            out[0].finished_session,
            Some((1_700_000_000, 1_700_000_002, 2))
        );
        assert!(tr.live().is_none());

        // User returns, works 2s, then PC sleeps for 3600s: the 3600s sleep jump must NOT count.
        tr.tick(
            1_700_001_000,
            sample("CLIPStudioPaint.exe", 0),
            300,
            1,
            false,
        );
        tr.tick(
            1_700_001_002,
            sample("CLIPStudioPaint.exe", 0),
            300,
            1,
            false,
        );
        let wake = tr.tick(
            1_700_004_602,
            sample("CLIPStudioPaint.exe", 0),
            300,
            1,
            false,
        );
        assert_eq!(wake.len(), 1);
        assert_eq!(wake[0].delta_secs, 2);
        assert_eq!(wake[0].app.name, "Clip Studio Paint");
    }
}
