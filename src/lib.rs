mod scene;
pub mod ui_app;

pub fn run_desktop() -> Result<(), slint::PlatformError> {
    ui_app::run()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Slint Android backend initialization failed");
    let _ = ui_app::run();
}
