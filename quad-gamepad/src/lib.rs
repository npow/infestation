//! Cross-platform gamepad support for miniquad/macroquad.
//!
//! Provides unified access to gamepads on native (via gilrs) and web (via W3C Gamepad API).
//!
//! # WASM Usage
//!
//! For WASM builds, include the JavaScript plugin after miniquad's `gl.js`:
//!
//! ```html
//! <script src="gl.js"></script>
//! <script src="quad-gamepad.js"></script>
//! <script>load("your-game.wasm");</script>
//! ```
//!
//! Required JS files:
//! - `gl.js` from [miniquad](https://github.com/not-fl3/miniquad/blob/master/js/gl.js)
//! - `quad-gamepad.js` from this crate's `js/` directory

use std::collections::{HashMap, HashSet};

/// Plugin version for miniquad's plugin system.
#[no_mangle]
#[cfg(target_arch = "wasm32")]
pub extern "C" fn quad_gamepad_crate_version() -> u32 {
    1
}

/// Gamepad button identifiers following the "Standard Gamepad" layout.
/// See: <https://w3c.github.io/gamepad/#remapping>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    /// Bottom button in right cluster (A on Xbox, X on PlayStation)
    South,
    /// Right button in right cluster (B on Xbox, Circle on PlayStation)
    East,
    /// Left button in right cluster (X on Xbox, Square on PlayStation)
    West,
    /// Top button in right cluster (Y on Xbox, Triangle on PlayStation)
    North,
    /// Top left shoulder button (LB/L1)
    LeftShoulder,
    /// Top right shoulder button (RB/R1)
    RightShoulder,
    /// Bottom left shoulder trigger (LT/L2)
    LeftTrigger,
    /// Bottom right shoulder trigger (RT/R2)
    RightTrigger,
    /// Left button in center cluster (Back/Select/Share)
    Select,
    /// Right button in center cluster (Start/Options)
    Start,
    /// Left stick pressed in
    LeftStick,
    /// Right stick pressed in
    RightStick,
    /// D-pad up
    DPadUp,
    /// D-pad down
    DPadDown,
    /// D-pad left
    DPadLeft,
    /// D-pad right
    DPadRight,
    /// Center button (Xbox button, PS button, Home)
    Home,
}

/// Controller type detected from device name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControllerType {
    Xbox,
    PlayStation,
    Nintendo,
    #[default]
    Generic,
}

impl ControllerType {
    /// Detect controller type from device name string.
    pub fn from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();
        if name_lower.contains("xbox")
            || name_lower.contains("xinput")
            || name_lower.contains("microsoft")
        {
            ControllerType::Xbox
        } else if name_lower.contains("playstation")
            || name_lower.contains("dualshock")
            || name_lower.contains("dualsense")
            || name_lower.contains("sony")
            || name_lower.contains("ps4")
            || name_lower.contains("ps5")
        {
            ControllerType::PlayStation
        } else if name_lower.contains("nintendo")
            || name_lower.contains("switch")
            || name_lower.contains("joy-con")
            || name_lower.contains("pro controller")
        {
            ControllerType::Nintendo
        } else {
            ControllerType::Generic
        }
    }
}

/// Gamepad axis identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    /// Left stick horizontal (-1 = left, 1 = right)
    LeftX,
    /// Left stick vertical (-1 = up, 1 = down)
    LeftY,
    /// Right stick horizontal (-1 = left, 1 = right)
    RightX,
    /// Right stick vertical (-1 = up, 1 = down)
    RightY,
}

/// State for a single gamepad.
#[derive(Clone, Default)]
pub struct GamepadState {
    buttons_down: HashSet<GamepadButton>,
    buttons_pressed: HashSet<GamepadButton>,
    buttons_released: HashSet<GamepadButton>,
    axes: HashMap<GamepadAxis, f32>,
    connected: bool,
    controller_type: ControllerType,
}

impl GamepadState {
    /// Creates a new disconnected gamepad state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if this gamepad is connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Returns the detected controller type.
    pub fn controller_type(&self) -> ControllerType {
        self.controller_type
    }

    /// Returns true if the button is currently held down.
    pub fn is_button_down(&self, button: GamepadButton) -> bool {
        self.buttons_down.contains(&button)
    }

    /// Returns true if the button was pressed this frame.
    pub fn is_button_pressed(&self, button: GamepadButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    /// Returns true if the button was released this frame.
    pub fn is_button_released(&self, button: GamepadButton) -> bool {
        self.buttons_released.contains(&button)
    }

    /// Returns the value of an axis (-1.0 to 1.0).
    pub fn axis(&self, axis: GamepadAxis) -> f32 {
        self.axes.get(&axis).copied().unwrap_or(0.0)
    }

    pub(crate) fn end_frame(&mut self) {
        self.buttons_pressed.clear();
        self.buttons_released.clear();
    }

    pub(crate) fn set_button(&mut self, button: GamepadButton, pressed: bool) {
        if pressed {
            if !self.buttons_down.contains(&button) {
                self.buttons_pressed.insert(button);
            }
            self.buttons_down.insert(button);
        } else {
            if self.buttons_down.contains(&button) {
                self.buttons_released.insert(button);
            }
            self.buttons_down.remove(&button);
        }
    }

    pub(crate) fn set_axis(&mut self, axis: GamepadAxis, value: f32) {
        self.axes.insert(axis, value);
    }

    pub(crate) fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
        if !connected {
            self.buttons_down.clear();
        }
    }

    pub(crate) fn set_controller_type(&mut self, controller_type: ControllerType) {
        self.controller_type = controller_type;
    }
}

/// Global gamepad context managing all connected gamepads.
pub struct GamepadContext {
    gamepads: [GamepadState; 4],
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
    #[cfg(not(target_arch = "wasm32"))]
    gilrs_mapping: HashMap<gilrs::GamepadId, usize>,
}

impl Default for GamepadContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadContext {
    /// Creates a new gamepad context and initializes the backend.
    #[allow(unused_mut)]
    pub fn new() -> Self {
        let mut ctx = Self {
            gamepads: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: gilrs::Gilrs::new().ok(),
            #[cfg(not(target_arch = "wasm32"))]
            gilrs_mapping: HashMap::new(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref gilrs) = ctx.gilrs {
            for (id, gamepad) in gilrs.gamepads() {
                if gamepad.is_connected() {
                    let slot = ctx.gilrs_mapping.len();
                    if slot < 4 {
                        ctx.gilrs_mapping.insert(id, slot);
                        ctx.gamepads[slot].connected = true;
                        ctx.gamepads[slot].controller_type =
                            ControllerType::from_name(gamepad.name());
                    }
                }
            }
        }

        ctx
    }

    /// Returns a reference to the gamepad state at the given index (0-3).
    pub fn gamepad(&self, index: usize) -> Option<&GamepadState> {
        self.gamepads.get(index)
    }

    /// Polls for gamepad events. Call this once per frame.
    pub fn poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_native();

        #[cfg(target_arch = "wasm32")]
        self.poll_wasm();
    }

    /// Clears per-frame state (pressed/released). Call this at the end of each frame.
    pub fn end_frame(&mut self) {
        for state in &mut self.gamepads {
            state.end_frame();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_native(&mut self) {
        use gilrs::{Event, EventType};

        let Some(ref mut gilrs) = self.gilrs else {
            return;
        };

        while let Some(Event { id, event, .. }) = gilrs.next_event() {
            match event {
                EventType::Connected => {
                    if !self.gilrs_mapping.contains_key(&id) {
                        let slot = self.gilrs_mapping.len();
                        if slot < 4 {
                            self.gilrs_mapping.insert(id, slot);
                            self.gamepads[slot].set_connected(true);
                            let gp = gilrs.gamepad(id);
                            self.gamepads[slot]
                                .set_controller_type(ControllerType::from_name(gp.name()));
                        }
                    }
                }
                EventType::Disconnected => {
                    if let Some(&slot) = self.gilrs_mapping.get(&id) {
                        self.gamepads[slot].set_connected(false);
                    }
                }
                _ => {}
            }

            let Some(&slot) = self.gilrs_mapping.get(&id) else {
                continue;
            };
            let state = &mut self.gamepads[slot];

            match event {
                EventType::ButtonPressed(btn, _) => {
                    if let Some(button) = map_gilrs_button(btn) {
                        state.set_button(button, true);
                    }
                }
                EventType::ButtonReleased(btn, _) => {
                    if let Some(button) = map_gilrs_button(btn) {
                        state.set_button(button, false);
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    if let Some(ax) = map_gilrs_axis(axis) {
                        state.set_axis(ax, value);
                    }
                }
                _ => {}
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_wasm(&mut self) {
        extern "C" {
            fn sapp_gamepad_count() -> i32;
            fn sapp_gamepad_connected(id: i32) -> i32;
            fn sapp_gamepad_button(id: i32, btn: i32) -> i32;
            fn sapp_gamepad_axis(id: i32, axis: i32) -> f32;
            fn sapp_gamepad_type(id: i32) -> i32;
        }

        let count = unsafe { sapp_gamepad_count() }.min(4);

        for i in 0..4 {
            let state = &mut self.gamepads[i];

            if i >= count as usize {
                if state.connected {
                    state.set_connected(false);
                }
                continue;
            }

            let connected = unsafe { sapp_gamepad_connected(i as i32) } != 0;
            if !connected {
                if state.connected {
                    state.set_connected(false);
                }
                continue;
            }

            let was_connected = state.connected;
            state.set_connected(true);

            // Detect controller type when newly connected
            if !was_connected {
                let type_id = unsafe { sapp_gamepad_type(i as i32) };
                state.set_controller_type(match type_id {
                    0 => ControllerType::Xbox,
                    1 => ControllerType::PlayStation,
                    2 => ControllerType::Nintendo,
                    _ => ControllerType::Generic,
                });
            }

            for (idx, btn) in WASM_BUTTON_MAP.iter().enumerate() {
                if let Some(button) = btn {
                    let pressed = unsafe { sapp_gamepad_button(i as i32, idx as i32) } != 0;
                    state.set_button(*button, pressed);
                }
            }

            state.set_axis(GamepadAxis::LeftX, unsafe {
                sapp_gamepad_axis(i as i32, 0)
            });
            state.set_axis(GamepadAxis::LeftY, unsafe {
                sapp_gamepad_axis(i as i32, 1)
            });
            state.set_axis(GamepadAxis::RightX, unsafe {
                sapp_gamepad_axis(i as i32, 2)
            });
            state.set_axis(GamepadAxis::RightY, unsafe {
                sapp_gamepad_axis(i as i32, 3)
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_gilrs_button(btn: gilrs::Button) -> Option<GamepadButton> {
    use gilrs::Button;
    match btn {
        Button::South => Some(GamepadButton::South),
        Button::East => Some(GamepadButton::East),
        Button::West => Some(GamepadButton::West),
        Button::North => Some(GamepadButton::North),
        Button::LeftTrigger => Some(GamepadButton::LeftShoulder),
        Button::RightTrigger => Some(GamepadButton::RightShoulder),
        Button::LeftTrigger2 => Some(GamepadButton::LeftTrigger),
        Button::RightTrigger2 => Some(GamepadButton::RightTrigger),
        Button::Select => Some(GamepadButton::Select),
        Button::Start => Some(GamepadButton::Start),
        Button::LeftThumb => Some(GamepadButton::LeftStick),
        Button::RightThumb => Some(GamepadButton::RightStick),
        Button::DPadUp => Some(GamepadButton::DPadUp),
        Button::DPadDown => Some(GamepadButton::DPadDown),
        Button::DPadLeft => Some(GamepadButton::DPadLeft),
        Button::DPadRight => Some(GamepadButton::DPadRight),
        Button::Mode => Some(GamepadButton::Home),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_gilrs_axis(axis: gilrs::Axis) -> Option<GamepadAxis> {
    use gilrs::Axis;
    match axis {
        Axis::LeftStickX => Some(GamepadAxis::LeftX),
        Axis::LeftStickY => Some(GamepadAxis::LeftY),
        Axis::RightStickX => Some(GamepadAxis::RightX),
        Axis::RightStickY => Some(GamepadAxis::RightY),
        _ => None,
    }
}

/// Standard Gamepad button mapping for WASM (indices match W3C spec)
#[cfg(target_arch = "wasm32")]
const WASM_BUTTON_MAP: [Option<GamepadButton>; 17] = [
    Some(GamepadButton::South),         // 0
    Some(GamepadButton::East),          // 1
    Some(GamepadButton::West),          // 2
    Some(GamepadButton::North),         // 3
    Some(GamepadButton::LeftShoulder),  // 4
    Some(GamepadButton::RightShoulder), // 5
    Some(GamepadButton::LeftTrigger),   // 6
    Some(GamepadButton::RightTrigger),  // 7
    Some(GamepadButton::Select),        // 8
    Some(GamepadButton::Start),         // 9
    Some(GamepadButton::LeftStick),     // 10
    Some(GamepadButton::RightStick),    // 11
    Some(GamepadButton::DPadUp),        // 12
    Some(GamepadButton::DPadDown),      // 13
    Some(GamepadButton::DPadLeft),      // 14
    Some(GamepadButton::DPadRight),     // 15
    Some(GamepadButton::Home),          // 16
];
