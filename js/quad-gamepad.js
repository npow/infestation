// Gamepad support plugin for miniquad (W3C Gamepad API)
// https://w3c.github.io/gamepad/#remapping

// Controller type detection from gamepad.id string
// Returns: 0=Xbox, 1=PlayStation, 2=Nintendo, 3=Generic
function detectControllerType(name) {
    if (!name) return 3;
    var lower = name.toLowerCase();
    if (lower.includes("xbox") || lower.includes("xinput") || lower.includes("microsoft")) {
        return 0;
    } else if (lower.includes("playstation") || lower.includes("dualshock") ||
               lower.includes("dualsense") || lower.includes("sony") ||
               lower.includes("ps4") || lower.includes("ps5")) {
        return 1;
    } else if (lower.includes("nintendo") || lower.includes("switch") ||
               lower.includes("joy-con") || lower.includes("pro controller")) {
        return 2;
    }
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
