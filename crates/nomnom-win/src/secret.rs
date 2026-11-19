//! Keeps a secret (a service key) on this computer so only this Windows user can read it back.
//! It uses the Windows data protection of the signed-in account; the bytes saved to disk mean
//! nothing to another user or another computer.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
};

fn take(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    // SAFETY: Windows allocated `pbData` with `cbData` bytes, and it must be freed with LocalFree.
    unsafe {
        let v = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(blob.pbData as *mut _));
        v
    }
}

/// Locks `data` for the current Windows user.
pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` points at `data` for the call; `out` is filled by Windows.
    unsafe { CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut out).ok()? };
    Some(take(out))
}

/// Opens what `protect` locked, for the same Windows user. `None` for anything else.
pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as in `protect`.
    unsafe { CryptUnprotectData(&input, None, None, None, None, 0, &mut out).ok()? };
    Some(take(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_comes_back_and_is_not_stored_in_the_clear() {
        let secret = b"1234-abcd:fx";
        let locked = protect(secret).expect("lock");
        assert!(!locked.windows(secret.len()).any(|w| w == secret));
        assert_eq!(unprotect(&locked).as_deref(), Some(&secret[..]));
        assert_eq!(unprotect(b"not protected data"), None);
    }
}
