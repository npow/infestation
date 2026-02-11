//! Open a URL in the browser.
//!
//! - Native: no-op (this feature is WASM-only)
//! - WASM: Uses window.open() via JavaScript plugin

unsafe extern "C" {
    fn sapp_open_url(buf: *const u8, len: u32);
}

pub(crate) fn open(url: &str) {
    unsafe { sapp_open_url(url.as_ptr(), url.len() as u32) };
}
