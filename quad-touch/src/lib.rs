//! Touch device detection for miniquad/macroquad.
//!
//! # WASM Usage
//!
//! For WASM builds, include the JavaScript plugin after miniquad's `gl.js`:
//!
//! ```html
//! <script src="gl.js"></script>
//! <script src="quad-touch.js"></script>
//! <script>load("your-game.wasm");</script>
//! ```

/// Plugin version for miniquad's plugin system.
#[no_mangle]
#[cfg(target_arch = "wasm32")]
pub extern "C" fn quad_touch_crate_version() -> u32 {
    1
}

/// Returns true if the device has touch capability.
#[cfg(target_arch = "wasm32")]
pub fn is_touch_device() -> bool {
    extern "C" {
        fn sapp_is_touch_device() -> i32;
    }
    unsafe { sapp_is_touch_device() != 0 }
}

/// Returns true if the device has touch capability.
/// On native platforms, this always returns false.
#[cfg(not(target_arch = "wasm32"))]
pub fn is_touch_device() -> bool {
    false
}
