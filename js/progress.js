// Clipboard plugin for miniquad
// Provides clipboard read/write for progress export/import

var clipboard_key = "infestation_clipboard_data";

progress_register_js_plugin = function (importObject) {
    importObject.env.sapp_clipboard_write = function (buf, len) {
        var text = UTF8ToString(buf, len);
        navigator.clipboard.writeText(text);
    };
    importObject.env.sapp_clipboard_read = function () {
        navigator.clipboard.readText().then(function (text) {
            if (text && text.trim() !== "") {
                localStorage.setItem(clipboard_key, text.trim());
            }
        });
    };
};

miniquad_add_plugin({
    register_plugin: progress_register_js_plugin,
    name: "progress",
    version: 1
});
