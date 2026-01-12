// Screen Wake Lock plugin for miniquad
// Prevents display from sleeping during gameplay using the Screen Wake Lock API

var wake_lock = null;

screen_wake_register_js_plugin = function (importObject) {
    importObject.env.sapp_request_wake_lock = function() {
        if ('wakeLock' in navigator && wake_lock === null) {
            navigator.wakeLock.request('screen').then(lock => {
                wake_lock = lock;
                // Re-acquire lock if released (e.g., tab becomes visible again)
                document.addEventListener('visibilitychange', async () => {
                    if (wake_lock !== null && document.visibilityState === 'visible') {
                        wake_lock = await navigator.wakeLock.request('screen');
                    }
                });
            }).catch(err => {
                console.warn('Wake Lock request failed:', err);
            });
        }
    };
};

miniquad_add_plugin({
    register_plugin: screen_wake_register_js_plugin,
    name: "screen_wake",
    version: 1
});
