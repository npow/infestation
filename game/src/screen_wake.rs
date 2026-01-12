//! Cross-platform screen wake lock to prevent display sleep during gameplay.
//!
//! - Native (Windows/macOS/Linux): Uses the `keepawake` crate
//! - WASM: Uses the Screen Wake Lock API via JavaScript

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::sync::OnceLock;

    static WAKE_LOCK: OnceLock<Option<keepawake::KeepAwake>> = OnceLock::new();

    pub(crate) fn request() {
        WAKE_LOCK.get_or_init(|| {
            keepawake::Builder::default()
                .display(true)
                .reason("Playing Infestation")
                .app_name("Infestation")
                .app_reverse_domain("com.infestation.game")
                .create()
                .ok()
        });
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    unsafe extern "C" {
        fn sapp_request_wake_lock();
    }

    pub(crate) fn request() {
        unsafe { sapp_request_wake_lock() };
    }
}

/// Request a screen wake lock to prevent the display from sleeping.
/// Call this once at app startup. The lock is held for the lifetime of the app.
pub(crate) fn request() {
    #[cfg(not(target_arch = "wasm32"))]
    native::request();

    #[cfg(target_arch = "wasm32")]
    wasm::request();
}
