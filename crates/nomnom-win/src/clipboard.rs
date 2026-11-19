//! The Windows side of the clipboard history: noticing that the clipboard changed, leaving out what
//! a password manager marked as private, and pasting into the window that was in front.

use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::DataExchange::{
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, RegisterClipboardFormatW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

/// A number that changes every time anything is copied. Cheap enough to read many times a second.
pub fn sequence() -> u32 {
    // SAFETY: no arguments, no side effects.
    unsafe { GetClipboardSequenceNumber() }
}

/// Did the program that copied this ask not to be recorded (password managers do)?
pub fn private_copy() -> bool {
    // SAFETY: only reads whether a registered format is on the clipboard.
    unsafe {
        let id = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
        id != 0 && IsClipboardFormatAvailable(id).is_ok()
    }
}

/// The window in front now, to give it back the keyboard later.
pub fn foreground() -> isize {
    // SAFETY: returns a handle or null.
    unsafe { GetForegroundWindow().0 as isize }
}

fn key(code: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Presses Ctrl+V in whatever window has the keyboard.
pub fn press_paste() {
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    // SAFETY: the array is valid for the call and the size is that of one INPUT.
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Puts `window` (from `foreground()`) back in front, then pastes into it.
pub fn paste_into(window: isize) {
    if window != 0 {
        // SAFETY: a stale handle just makes the call fail.
        unsafe {
            let _ = SetForegroundWindow(HWND(window as *mut _));
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(120));
    press_paste();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sequence_number_is_readable_and_a_private_copy_is_not_assumed() {
        let a = sequence();
        assert_eq!(a, sequence(), "nothing copied in between");
        let _ = private_copy();
        let _ = foreground();
    }
}
