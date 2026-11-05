//! Safe subprocess execution. Programs are launched with an argument vector (never through a
//! shell), without a console window, and inside a Job Object so that cancelling kills the whole
//! process tree and nothing outlives NOMNOM.

use crate::{Error, Result};
use std::ffi::OsStr;
use std::process::Stdio;
use tokio::process::{Child, Command};

#[cfg(windows)]
mod job {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    /// Owns a Job Object handle. Dropping it terminates every process still in the job.
    pub struct Job(HANDLE);
    // SAFETY: a Job Object handle can be used and closed from any thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn new() -> windows::core::Result<Job> {
            // SAFETY: plain Win32 calls with valid, fully initialised arguments.
            unsafe {
                let handle = CreateJobObjectW(None, None)?;
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )?;
                Ok(Job(handle))
            }
        }

        pub fn assign(&self, process: HANDLE) -> windows::core::Result<()> {
            // SAFETY: both handles are valid for the duration of the call.
            unsafe { AssignProcessToJobObject(self.0, process) }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: we own the handle and close it exactly once.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

/// A running child process bound to its own Job Object.
pub struct ManagedChild {
    pub child: Child,
    #[cfg(windows)]
    _job: job::Job,
}

impl ManagedChild {
    /// Kills the process and everything it started, then reaps it.
    pub async fn kill_tree(mut self) {
        #[cfg(windows)]
        drop(std::mem::replace(&mut self._job, job::Job::new().expect("job object")));
        let _ = self.child.kill().await;
    }
}

/// Spawns `program` with `args`. The caller is responsible for resolving `program` to a trusted
/// path (see the tool manager); it is never taken from user input.
pub fn spawn<I, S>(program: &std::path::Path, args: I, piped_output: bool) -> Result<ManagedChild>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = Command::new(program);
    cmd.args(args).stdin(Stdio::null()).kill_on_drop(true);
    if piped_output {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd.spawn().map_err(|e| Error::io(program, e))?;

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HANDLE;
        let job = job::Job::new().map_err(|e| Error::Other(format!("job object: {e}")))?;
        if let Some(raw) = child.raw_handle() {
            // There is a very small window between spawn and assignment in which the child could
            // start a grandchild that escapes the job. Acceptable for yt-dlp/ffmpeg.
            job.assign(HANDLE(raw as _))
                .map_err(|e| Error::Other(format!("assigning process to job: {e}")))?;
        }
        Ok(ManagedChild { child, _job: job })
    }
    #[cfg(not(windows))]
    {
        Ok(ManagedChild { child })
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn kill_tree_stops_long_running_child_and_its_children() {
        // cmd.exe starts ping.exe as a grandchild; both must die.
        let cmd = Path::new("C:\\Windows\\System32\\cmd.exe");
        let mc = spawn(cmd, ["/c", "ping -n 60 127.0.0.1 >nul"], false).unwrap();
        let pid = mc.child.id().unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        let started = Instant::now();
        mc.kill_tree().await;
        assert!(started.elapsed() < Duration::from_secs(5));
        tokio::time::sleep(Duration::from_millis(300)).await;
        let out = std::process::Command::new("C:\\Windows\\System32\\tasklist.exe")
            .args(["/FI", "IMAGENAME eq ping.exe", "/FO", "CSV", "/NH"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(!text.contains("ping.exe"), "grandchild survived (cmd pid {pid}): {text}");
    }

    #[tokio::test]
    async fn runs_without_a_shell() {
        let cmd = Path::new("C:\\Windows\\System32\\cmd.exe");
        // A hostile-looking argument must be passed as data, not interpreted.
        let mut mc = spawn(cmd, ["/c", "echo", "a&calc"], true).unwrap();
        let status = mc.child.wait().await.unwrap();
        assert!(status.success());
    }
}
