#[cfg(target_os = "android")]
mod android;
mod native;
mod pak;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(std::sync::Arc::new(native::NativeState::default()))
        .invoke_handler(tauri::generate_handler![
            native::wuwa_load,
            native::wuwa_run,
            native::wuwa_open_setup,
            native::wuwa_diagnostics
        ]);
    #[cfg(target_os = "android")]
    let builder = builder.plugin(android::init());
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
