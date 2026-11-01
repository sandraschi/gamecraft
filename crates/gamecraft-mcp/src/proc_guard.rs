//! Child-process hygiene for anything that launches Godot (or another engine).
//!
//! Two leaks this prevents, both seen on the dev box (2026-10-31: 21 game windows from
//! the viewer and headless test engines whose launcher was long gone):
//!
//! * [`kill_children_on_exit`] puts the *current* process in a Windows Job Object with
//!   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Children inherit the job, so when this process
//!   exits for any reason (normal exit, crash, killed by a test runner) Windows kills
//!   every process it started. No-op elsewhere.
//! * [`run_with_timeout`] runs a command to completion, capturing output, and kills it
//!   when it exceeds a wall-clock budget instead of waiting forever.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
static JOB: std::sync::OnceLock<Option<win32job::Job>> = std::sync::OnceLock::new();

/// Make every child this process starts die with it (Windows Job Object).
///
/// Idempotent. Returns `false` when the job could not be created or assigned (for
/// example when an outer job forbids nesting); callers should carry on, the processes
/// just lose this safety net. Always `false` on non-Windows targets.
pub fn kill_children_on_exit() -> bool {
    #[cfg(windows)]
    {
        JOB.get_or_init(|| {
            let job = win32job::Job::create().ok()?;
            let mut info = job.query_extended_limit_info().ok()?;
            info.limit_kill_on_job_close();
            job.set_extended_limit_info(&info).ok()?;
            job.assign_current_process().ok()?;
            Some(job)
        })
        .is_some()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Result of [`run_with_timeout`].
#[derive(Debug)]
pub struct TimedOutput {
    /// Exit status, `None` when the process was killed for exceeding the budget.
    pub status: Option<ExitStatus>,
    /// Captured stdout (lossy UTF-8).
    pub stdout: String,
    /// Captured stderr (lossy UTF-8).
    pub stderr: String,
    /// True when the budget ran out and the process was killed.
    pub timed_out: bool,
    /// Wall-clock time spent.
    pub elapsed: Duration,
}

fn drain<R: Read + Send + 'static>(reader: Option<R>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = reader {
            let _ = r.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).to_string()
    })
}

/// Run `command` to completion with captured output, killing it after `budget`.
///
/// Stdout and stderr are drained on their own threads so a chatty child cannot
/// deadlock on a full pipe.
///
/// # Errors
///
/// Returns the spawn error when the program cannot be started.
pub fn run_with_timeout(command: &mut Command, budget: Duration) -> std::io::Result<TimedOutput> {
    let started = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let mut timed_out = false;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break Some(status),
            None if started.elapsed() >= budget => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    Ok(TimedOutput {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
        timed_out,
        elapsed: started.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn sleeper(seconds: u32) -> Command {
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-Command",
            &format!("Start-Sleep -Seconds {seconds}; 'done'"),
        ]);
        c
    }

    #[cfg(not(windows))]
    fn sleeper(seconds: u32) -> Command {
        let mut c = Command::new("sh");
        c.args(["-c", &format!("sleep {seconds}; echo done")]);
        c
    }

    #[test]
    fn finishes_within_budget() {
        let out = run_with_timeout(&mut sleeper(0), Duration::from_secs(30));
        assert!(out.is_ok());
        if let Ok(out) = out {
            assert!(!out.timed_out);
            assert!(out.stdout.contains("done"));
        }
    }

    #[test]
    fn kills_on_timeout() {
        let out = run_with_timeout(&mut sleeper(30), Duration::from_millis(1500));
        assert!(out.is_ok());
        if let Ok(out) = out {
            assert!(out.timed_out);
            assert!(out.status.is_none());
            assert!(out.elapsed < Duration::from_secs(20));
        }
    }

    #[test]
    fn job_guard_is_idempotent() {
        let first = kill_children_on_exit();
        assert_eq!(first, kill_children_on_exit());
    }
}
