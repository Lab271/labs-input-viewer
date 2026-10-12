// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Persistent app log: one file per day, kept for a week. A port of
//! `src/main/file-log.js` (ADR 0044), with its tests.
//!
//! The wall came up with a dark capture card, someone restarted the app ten
//! minutes later, and there was no way to tell afterwards what it had seen.
//! This is the record, and the operators read it: the file names, the line
//! format and the `[Health]` lines are an interface, not an implementation
//! detail.
//!
//! Bounded three ways, because the wall runs for months:
//! - files older than `retention_days` are deleted (on start and at each day
//!   change), by the date in the name, not the mtime;
//! - a day's file stops growing at `max_bytes_per_day`, with one line saying so;
//! - more than `max_lines_per_minute` lines in a minute are dropped and
//!   counted, so a log loop cannot fill the disk before the cap matters.
//!
//! Worst case on disk is 7 x 20 MB. Appends are synchronous on purpose: a
//! crash or a hard power-off is exactly when the last lines matter. A log that
//! cannot be written disables itself rather than ever take the app down.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub struct Limits {
    pub retention_days: i64,
    pub max_bytes_per_day: u64,
    pub max_lines_per_minute: u32,
    pub max_line_length: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self { retention_days: 7, max_bytes_per_day: 20 * 1024 * 1024, max_lines_per_minute: 600, max_line_length: 2000 }
    }
}

/// Local-date stamp, so a file is one wall-clock day where the wall is.
pub fn day_stamp(d: NaiveDate) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

fn time_stamp(t: NaiveDateTime) -> String {
    format!("{:02}:{:02}:{:02}.{:03}", t.hour(), t.minute(), t.second(), t.and_utc().timestamp_subsec_millis())
}

/// The day in a log file's name, or None for anything else in the folder.
fn file_day(name: &str) -> Option<NaiveDate> {
    let stamp = name.strip_prefix("input-viewer-")?.strip_suffix(".log")?;
    NaiveDate::parse_from_str(stamp, "%Y-%m-%d").ok()
}

pub struct FileLog {
    dir: PathBuf,
    limits: Limits,
    day: Option<NaiveDate>,
    file: PathBuf,
    bytes: u64,
    capped: bool,
    minute: Option<i64>,
    lines_this_minute: u32,
    dropped: u32,
    broken: bool,
}

impl FileLog {
    pub fn new(dir: impl Into<PathBuf>, limits: Limits) -> Self {
        Self {
            dir: dir.into(),
            limits,
            day: None,
            file: PathBuf::new(),
            bytes: 0,
            capped: false,
            minute: None,
            lines_this_minute: 0,
            dropped: 0,
            broken: false,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Delete files past retention. Returns the names removed.
    pub fn prune(&self, today: NaiveDate) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.dir) else { return Vec::new() };
        // Compare calendar days, not mtimes: a file touched late is still that day.
        let cutoff = today - Duration::days(self.limits.retention_days - 1);
        let mut removed = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if file_day(&name).is_some_and(|d| d < cutoff) && fs::remove_file(entry.path()).is_ok() {
                // In use or gone: try again tomorrow.
                removed.push(name);
            }
        }
        removed
    }

    fn roll_if_needed(&mut self, now: NaiveDateTime) {
        let today = now.date();
        if self.day == Some(today) {
            return;
        }
        self.day = Some(today);
        self.file = self.dir.join(format!("input-viewer-{}.log", day_stamp(today)));
        self.capped = false;
        self.bytes = fs::create_dir_all(&self.dir)
            .ok()
            .and_then(|()| fs::metadata(&self.file).ok())
            .map_or(0, |m| m.len());
        self.prune(today);
    }

    fn append(&mut self, text: &str) {
        if self.broken {
            return;
        }
        let ok = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
            .and_then(|mut f| f.write_all(text.as_bytes()));
        match ok {
            Ok(()) => self.bytes += text.len() as u64,
            // Stop trying rather than fail on every line.
            Err(_) => self.broken = true,
        }
    }

    /// Today's file.
    #[cfg(test)]
    pub fn path(&mut self, now: NaiveDateTime) -> PathBuf {
        self.roll_if_needed(now);
        self.file.clone()
    }

    pub fn write(&mut self, level: Level, source: &str, message: &str, now: NaiveDateTime) {
        self.roll_if_needed(now);

        let minute = now.and_utc().timestamp().div_euclid(60);
        if self.minute != Some(minute) {
            if self.dropped > 0 && !self.capped {
                let line = format!(
                    "{} WARN  [log] dropped {} line(s) over {}/min in the previous minute\n",
                    time_stamp(now), self.dropped, self.limits.max_lines_per_minute
                );
                self.append(&line);
            }
            self.minute = Some(minute);
            self.lines_this_minute = 0;
            self.dropped = 0;
        }
        if self.lines_this_minute >= self.limits.max_lines_per_minute {
            self.dropped += 1;
            return;
        }
        self.lines_this_minute += 1;

        if self.capped {
            return;
        }
        if self.bytes >= self.limits.max_bytes_per_day {
            self.capped = true;
            let line = format!(
                "{} WARN  [log] {} byte daily cap reached; nothing more is written until tomorrow\n",
                time_stamp(now), self.limits.max_bytes_per_day
            );
            self.append(&line);
            return;
        }

        let mut text: String = message.chars().take(self.limits.max_line_length).collect();
        if message.chars().count() > self.limits.max_line_length {
            text.push_str("...(truncated)");
        }
        // One record per line, so a multi-line message does not interleave.
        let text = text.replace("\r\n", "\n").replace('\n', "\n    ");
        let line = format!("{} {:<5} [{source}] {text}\n", time_stamp(now), level.label());
        self.append(&line);
    }
}

// ---------------------------------------------------------------------------
// The app's one log: stderr and the file, from any thread.
// ---------------------------------------------------------------------------

static LOG: OnceLock<Mutex<FileLog>> = OnceLock::new();

/// Start writing to `<dir>/input-viewer-YYYY-MM-DD.log`. Lines logged before
/// this go to stderr only.
pub fn init(dir: PathBuf) {
    let _ = LOG.set(Mutex::new(FileLog::new(dir, Limits::default())));
}

pub fn dir() -> Option<PathBuf> {
    LOG.get().map(|l| l.lock().unwrap_or_else(|e| e.into_inner()).dir().to_owned())
}

pub fn line(level: Level, message: &str) {
    eprintln!("{message}");
    if let Some(log) = LOG.get() {
        // A panic while holding the lock poisons it; logging must still work,
        // the panic hook in particular.
        let mut log = log.lock().unwrap_or_else(|e| e.into_inner());
        log.write(level, "app", message, Local::now().naive_local());
    }
}

/// `info!`/`warn!`/`error!` for this crate, named so they cannot be confused
/// with the `log` crate's.
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::filelog::line($crate::filelog::Level::Info, &format!($($arg)*)) };
}
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::filelog::line($crate::filelog::Level::Warn, &format!($($arg)*)) };
}
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::filelog::line($crate::filelog::Level::Error, &format!($($arg)*)) };
}
pub(crate) use {log_error, log_info, log_warn};

#[cfg(test)]
mod tests {
    //! Ported from test/file-log.test.js, case for case.
    use super::*;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("iv-log-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }

    fn limits() -> Limits {
        Limits::default()
    }

    #[test]
    fn one_timestamped_line_per_record_in_a_file_named_for_the_day() {
        let dir = scratch("line");
        let mut log = FileLog::new(&dir, limits());
        let now = at("2026-09-23T08:40:00");
        log.write(Level::Info, "renderer", "[Health] left ok", now);
        log.write(Level::Warn, "main", "two\nlines", now);
        let path = log.path(now);
        assert_eq!(path.file_name().unwrap(), "input-viewer-2026-09-23.log");
        let text = fs::read_to_string(path).unwrap();
        assert!(text.starts_with("08:40:00.000 INFO  [renderer] [Health] left ok\n"), "{text}");
        assert!(text.contains("WARN  [main] two\n    lines\n"), "{text}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rolls_to_a_new_file_at_midnight() {
        let dir = scratch("roll");
        let mut log = FileLog::new(&dir, limits());
        log.write(Level::Info, "main", "before", at("2026-09-23T23:59:59"));
        log.write(Level::Info, "main", "after", at("2026-09-24T00:00:01"));
        assert_eq!(listing(&dir), ["input-viewer-2026-09-23.log", "input-viewer-2026-09-24.log"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn deletes_files_older_than_a_week_and_leaves_other_files_alone() {
        let dir = scratch("prune");
        for d in ["2026-09-10", "2026-09-16", "2026-09-17", "2026-09-22"] {
            fs::write(dir.join(format!("input-viewer-{d}.log")), "x").unwrap();
        }
        fs::write(dir.join("settings.json"), "{}").unwrap();
        let mut log = FileLog::new(&dir, limits());
        log.write(Level::Info, "main", "start", at("2026-09-23T09:00:00"));
        // Seven calendar days kept: 17th..23rd.
        assert_eq!(listing(&dir), [
            "input-viewer-2026-09-17.log", "input-viewer-2026-09-22.log",
            "input-viewer-2026-09-23.log", "settings.json",
        ]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn prunes_again_when_the_day_changes() {
        let dir = scratch("reprune");
        let mut log = FileLog::new(&dir, limits());
        log.write(Level::Info, "main", "day 0", at("2026-09-23T12:00:00"));
        log.write(Level::Info, "main", "day 8", at("2026-10-01T00:00:01"));
        assert_eq!(listing(&dir), ["input-viewer-2026-10-01.log"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stops_growing_at_the_daily_cap_and_says_so_once() {
        let dir = scratch("cap");
        let mut log = FileLog::new(&dir, Limits { max_bytes_per_day: 200, ..limits() });
        let now = at("2026-09-23T09:00:00");
        for i in 0..50 {
            log.write(Level::Info, "main", &format!("line {i}"), now);
        }
        let text = fs::read_to_string(log.path(now)).unwrap();
        assert!(text.len() < 400, "{}", text.len());
        assert_eq!(text.matches("daily cap reached").count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rate_limits_a_flood_and_reports_how_much_was_dropped() {
        let dir = scratch("rate");
        let mut log = FileLog::new(&dir, Limits { max_lines_per_minute: 5, ..limits() });
        for i in 0..20 {
            log.write(Level::Info, "renderer", &format!("spam {i}"), at("2026-09-23T09:00:00"));
        }
        log.write(Level::Info, "renderer", "next minute", at("2026-09-23T09:01:00"));
        let text = fs::read_to_string(log.path(at("2026-09-23T09:01:00"))).unwrap();
        assert_eq!(text.matches("spam").count(), 5);
        assert!(text.contains("dropped 15 line(s)"));
        assert!(text.contains("next minute"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn truncates_a_runaway_line() {
        let dir = scratch("trunc");
        let mut log = FileLog::new(&dir, Limits { max_line_length: 10, ..limits() });
        let now = at("2026-09-23T09:00:00");
        log.write(Level::Info, "main", &"x".repeat(100), now);
        assert!(fs::read_to_string(log.path(now)).unwrap().contains("xxxxxxxxxx...(truncated)"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn never_panics_when_the_directory_cannot_be_written() {
        let dir = scratch("blocked");
        let blocker = dir.join("not-a-dir");
        fs::write(&blocker, "").unwrap();
        let mut log = FileLog::new(&blocker, limits());
        log.write(Level::Error, "main", "still fine", at("2026-09-23T09:00:00"));
        log.write(Level::Error, "main", "and again", at("2026-09-23T09:00:00"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stamps_days_from_the_local_date() {
        assert_eq!(day_stamp(NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()), "2026-01-05");
    }
}
