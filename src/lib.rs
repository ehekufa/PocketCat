// eframe's NativeActivity entry point for cargo-apk.
#[path = "main.rs"]
mod app;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    app::run_android(app);
}
