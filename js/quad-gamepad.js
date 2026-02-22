// Gamepad support plugin for miniquad (W3C Gamepad API)
// https://w3c.github.io/gamepad/#remapping

// Controller type detection from gamepad.id string
// Returns: 0=Xbox, 1=PlayStation, 2=Nintendo, 3=Generic
// Browsers embed USB vendor/product IDs in the id string:
//   Chrome: "... (STANDARD GAMEPAD Vendor: 054c Product: 09cc)"
//   Firefox/Safari: "054c-09cc-Wireless Controller"
var VENDOR_TYPES = {"045e": 0, "054c": 1, "057e": 2}; // Xbox, PlayStation, Nintendo
function detectControllerType(name) {
    if (!name) return 3;
    var lower = name.toLowerCase();
    // Try to extract vendor ID
    var match = lower.match(/vendor:?\s*([0-9a-f]{4})/) ||
                lower.match(/^([0-9a-f]{4})-[0-9a-f]{4}-/);
    if (match && match[1] in VENDOR_TYPES) return VENDOR_TYPES[match[1]];
    return 3;
}

params_register_js_plugin = function (importObject) {
    // Returns highest occupied gamepad slot + 1 (not count of connected gamepads)
    importObject.env.sapp_gamepad_count = function() {
        var gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
        var count = 0;
        for (var i = 0; i < gamepads.length && i < 4; i++) {
            if (gamepads[i]) count = i + 1;
        }
        return count;
    };
    importObject.env.sapp_gamepad_connected = function(id) {
        var gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
        if (id < 0 || id >= gamepads.length) return 0;
        var gp = gamepads[id];
        return (gp && gp.connected) ? 1 : 0;
    };
    importObject.env.sapp_gamepad_button = function(id, btn) {
        var gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
        if (id < 0 || id >= gamepads.length) return 0;
        var gp = gamepads[id];
        if (!gp || !gp.connected || btn < 0 || btn >= gp.buttons.length) return 0;
        return gp.buttons[btn].pressed ? 1 : 0;
    };
    importObject.env.sapp_gamepad_axis = function(id, axis) {
        var gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
        if (id < 0 || id >= gamepads.length) return 0.0;
        var gp = gamepads[id];
        if (!gp || !gp.connected || axis < 0 || axis >= gp.axes.length) return 0.0;
        return gp.axes[axis];
    };
    // Returns controller type: 0=Xbox, 1=PlayStation, 2=Nintendo, 3=Generic
    importObject.env.sapp_gamepad_type = function(id) {
        var gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
        if (id < 0 || id >= gamepads.length) return 3;
        var gp = gamepads[id];
        if (!gp || !gp.connected) return 3;
        return detectControllerType(gp.id);
    };
};

miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    name: "quad_gamepad",
    version: 1
});
