//! Other programs' windows: keeping one above the rest.

use windows::core::PWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowTextW, SetWindowPos, GWL_EXSTYLE,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
};

/// What `toggle_topmost` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pinned {
    pub title: String,
    /// True when the window now stays on top.
    pub on_top: bool,
}

fn text_of(f: impl Fn(PWSTR, i32) -> i32) -> String {
    let mut buf = [0u16; 256];
    let n = f(PWSTR(buf.as_mut_ptr()), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// The desktop and the taskbar can't be pinned.
fn is_shell(class: &str) -> bool {
    matches!(
        class,
        "Progman"
            | "WorkerW"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "NotifyIconOverflowWindow"
    )
}

/// Puts the window in front of the rest, or lets it go back to normal. Acts on the window that has
/// the keyboard now. `None` when there is none, or it is part of the desktop.
pub fn toggle_topmost() -> Option<Pinned> {
    // SAFETY: plain window calls on the handle Windows gave; a window that closes meanwhile just fails.
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let class =
            text_of(|p, n| GetClassNameW(hwnd, std::slice::from_raw_parts_mut(p.0, n as usize)));
        if is_shell(&class) {
            return None;
        }
        let title =
            text_of(|p, n| GetWindowTextW(hwnd, std::slice::from_raw_parts_mut(p.0, n as usize)));
        let now = (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0) != 0;
        SetWindowPos(
            hwnd,
            if now { HWND_NOTOPMOST } else { HWND_TOPMOST },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .ok()?;
        Some(Pinned {
            title: if title.trim().is_empty() {
                class
            } else {
                title
            },
            on_top: !now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_and_taskbar_are_not_windows_to_pin() {
        assert!(is_shell("Progman"));
        assert!(is_shell("Shell_TrayWnd"));
        assert!(!is_shell("Notepad"));
    }
}
