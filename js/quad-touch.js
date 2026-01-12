// Touch device detection plugin for miniquad

params_register_js_plugin = function (importObject) {
    importObject.env.sapp_is_touch_device = function() {
        return navigator.maxTouchPoints > 0 ? 1 : 0;
    };
};

miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    name: "quad_touch",
    version: 1
});
