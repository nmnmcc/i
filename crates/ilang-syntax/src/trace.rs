//! A hidden trace of the compiler's stages, for measuring where time and
//! memory go. It is off unless `I_TRACE_STAGES` is set (to anything but
//! `0` or empty); then, at the end of each stage, one line goes to the
//! standard error:
//!
//! ```text
//! i-trace <stage> wall 1.234 s cpu 2.345 s rss 812 MB peak 1530 MB
//! ```
//!
//! `cpu` is the processor time of the whole process during the stage (user
//! and system, all threads). `rss` is the resident memory at the end of the
//! stage, `peak` the most resident memory during it: on Linux the peak is
//! reset (through `/proc/self/clear_refs`) whenever a stage starts or ends,
//! and an inner stage's peak counts for the stages around it. Elsewhere the
//! memory columns are left out. Nothing the compiler computes depends on it.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("I_TRACE_STAGES").is_some_and(|v| !v.is_empty() && v != "0"))
}

/// The peaks (kB) of the open stages, innermost last.
static OPEN: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// A running stage; the line is printed when it is dropped.
pub struct Stage {
    name: Option<&'static str>,
    wall: Instant,
    cpu: f64,
}

/// Starts the stage `name`. Does nothing unless the trace is on.
pub fn stage(name: &'static str) -> Stage {
    if !enabled() {
        return Stage {
            name: None,
            wall: Instant::now(),
            cpu: 0.0,
        };
    }
    let mut open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
    fold_peak(&mut open);
    open.push(memory().map_or(0, |(rss, _)| rss));
    Stage {
        name: Some(name),
        wall: Instant::now(),
        cpu: cpu_seconds().unwrap_or(0.0),
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let Some(name) = self.name else { return };
        let wall = self.wall.elapsed().as_secs_f64();
        let cpu = cpu_seconds().map_or(0.0, |c| c - self.cpu);
        let mut open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
        fold_peak(&mut open);
        let peak = open.pop().unwrap_or(0);
        let memory = match memory() {
            Some((rss, _)) => format!(" rss {} MB peak {} MB", rss / 1024, peak / 1024),
            None => String::new(),
        };
        eprintln!("i-trace {name} wall {wall:.3} s cpu {cpu:.3} s{memory}");
    }
}

/// Counts the peak since the last reset for every open stage, then resets it.
fn fold_peak(open: &mut [u64]) {
    if let Some((_, hwm)) = memory() {
        for p in open.iter_mut() {
            *p = (*p).max(hwm);
        }
        // "5" resets the peak resident size of the process (proc(5)).
        let _ = std::fs::write("/proc/self/clear_refs", "5");
    }
}

/// The current and peak resident memory in kB, from `/proc/self/status`.
fn memory() -> Option<(u64, u64)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let field = |key: &str| -> Option<u64> {
        let line = status.lines().find(|l| l.starts_with(key))?;
        line[key.len()..].trim().trim_end_matches("kB").trim().parse().ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

/// The processor time of the process so far, user and system.
fn cpu_seconds() -> Option<f64> {
    // Fields 14 and 15 of /proc/self/stat, in clock ticks of 1/100 s.
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let rest = &stat[stat.rfind(')')? + 2..];
    let mut fields = rest.split(' ').skip(11);
    let user: f64 = fields.next()?.parse().ok()?;
    let system: f64 = fields.next()?.parse().ok()?;
    Some((user + system) / 100.0)
}
