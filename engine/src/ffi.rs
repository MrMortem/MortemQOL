//! C ABI used by the Lua bridge. See the plan's Shared Contracts.

use std::panic::{catch_unwind, AssertUnwindSafe};

pub const ABI_VERSION: u32 = 1;
pub const STATUS_RUNNING: i32 = 0;
pub const STATUS_FINISHED: i32 = 1;
pub const STATUS_ERROR: i32 = -1;
pub const STATUS_TOO_SMALL: i32 = -2;
pub const STATUS_UNKNOWN_ID: i32 = -3;

/// Runs `f`, turning a panic into `fallback` so that it can never unwind into LuaJIT.
fn guarded<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

/// Copies `bytes` to `out`, reporting the size through `out_len`.
///
/// # Safety
/// `out` must be valid for `cap` bytes, `out_len` valid for one write; either may be null.
unsafe fn write_out(bytes: &[u8], out: *mut u8, cap: usize, out_len: *mut usize, ok: i32) -> i32 {
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    if bytes.is_empty() {
        return ok;
    }
    if out.is_null() || bytes.len() > cap {
        return STATUS_TOO_SMALL;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
    ok
}

#[no_mangle]
pub extern "C" fn mqol_abi_version() -> u32 {
    ABI_VERSION
}

/// Starts a search and returns its id, or 0 on failure.
///
/// # Safety
/// `json` must be valid for `len` bytes, or null.
#[no_mangle]
pub unsafe extern "C" fn mqol_search_start(json: *const u8, len: usize) -> u64 {
    guarded(0, || {
        let _ = (json, len);
        0
    })
}

/// Polls a search. See the status constants.
///
/// # Safety
/// `out` must be valid for `cap` bytes and `out_len` for one write; either may be null.
#[no_mangle]
pub unsafe extern "C" fn mqol_search_poll(
    id: u64,
    out: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> i32 {
    guarded(STATUS_ERROR, || {
        let _ = (id, out, cap, out_len);
        STATUS_UNKNOWN_ID
    })
}

/// Scores one play synchronously.
///
/// # Safety
/// `json` must be valid for `len` bytes, `out` for `cap` bytes and `out_len` for one write;
/// any of them may be null.
#[no_mangle]
pub unsafe extern "C" fn mqol_score(
    json: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> i32 {
    guarded(STATUS_ERROR, || {
        if json.is_null() {
            return write_out(b"null request", out, cap, out_len, STATUS_ERROR);
        }
        let request = std::slice::from_raw_parts(json, len);
        match crate::score::score_request(request) {
            Ok(result) => write_out(&result, out, cap, out_len, STATUS_FINISHED),
            Err(message) => write_out(message.as_bytes(), out, cap, out_len, STATUS_ERROR),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_is_1() {
        assert_eq!(mqol_abi_version(), 1);
    }

    #[test]
    fn score_with_null_pointer_returns_error_not_crash() {
        let mut out = [0u8; 64];
        let mut len = 0usize;
        let status =
            unsafe { mqol_score(std::ptr::null(), 0, out.as_mut_ptr(), out.len(), &mut len) };
        assert_eq!(status, STATUS_ERROR);
    }

    #[test]
    fn poll_unknown_id_returns_minus_3() {
        let mut out = [0u8; 64];
        let mut len = 0usize;
        let status = unsafe { mqol_search_poll(u64::MAX, out.as_mut_ptr(), out.len(), &mut len) };
        assert_eq!(status, STATUS_UNKNOWN_ID);
    }
}
