//! Gives memory that nobody is using back to Windows.
//!
//! A program keeps pages it touched once (start-up code, drawing buffers, caches) in its working
//! set long after it needs them. Trimming the working set moves the idle pages out of the
//! "Memory" column of Task Manager; any page that is used again comes back on demand, a few
//! microseconds each. NOMNOM and the WebView2 processes it starts are trimmed together, since
//! the web view is most of the memory.

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

/// `root` and every process started by it, directly or through others.
pub fn family(root: u32) -> Vec<u32> {
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    // SAFETY: the snapshot handle is closed below, and `entry` is sized as the API requires.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return vec![root];
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                pairs.push((entry.th32ProcessID, entry.th32ParentProcessID));
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let parent = out[i];
        for &(pid, ppid) in &pairs {
            if ppid == parent && pid != parent && !out.contains(&pid) {
                out.push(pid);
            }
        }
        i += 1;
    }
    out
}

/// Trims the working set of one process. Returns whether it worked.
pub fn trim(pid: u32) -> bool {
    // SAFETY: the handle is closed before returning.
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, false, pid)
        else {
            return false;
        };
        let ok = EmptyWorkingSet(handle).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

/// Trims `root` and everything it started. Returns how many processes were trimmed.
pub fn trim_family(root: u32) -> usize {
    family(root).into_iter().filter(|&pid| trim(pid)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_process_and_what_it_started_are_found_and_trimmed() {
        let me = std::process::id();
        assert!(family(me).contains(&me));
        assert!(trim(me));

        let mut child = std::process::Command::new("cmd")
            .args(["/C", "ping -n 4 127.0.0.1 >nul"])
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let all = family(me);
        assert!(all.contains(&child.id()), "the child is part of the family");
        assert!(trim_family(me) >= 2, "this process and the child");
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn a_process_that_is_not_there_is_not_trimmed() {
        assert!(!trim(0xFFFF_FFF0));
    }
}
