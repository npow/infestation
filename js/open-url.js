// URL opening plugin for miniquad
// Opens a URL in the browser from WASM

open_url_register_js_plugin = function (importObject) {
    importObject.env.sapp_open_url = function(buf, len) {
        var url = UTF8ToString(buf, len);
        window.open(url, '_self');
    };
};

miniquad_add_plugin({
    register_plugin: open_url_register_js_plugin,
    name: "open_url",
    version: 1
});
