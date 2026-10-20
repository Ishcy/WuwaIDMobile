use serde_json::{json, Value};
use tauri::{ipc::Channel, AppHandle};

#[cfg(target_os = "android")]
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "android")]
struct AppUpdater<R: Runtime>(PluginHandle<R>);

#[cfg(target_os = "android")]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("app-updater")
        .setup(|app, api| {
            let plugin =
                api.register_android_plugin("com.titotfp.wuwaid.update", "AppUpdatePlugin")?;
            app.manage(AppUpdater(plugin));
            Ok(())
        })
        .build()
}

#[tauri::command]
pub async fn wuwa_app_update(
    app: AppHandle,
    action: String,
    progress: Channel<Value>,
) -> Result<Value, String> {
    if !matches!(action.as_str(), "status" | "check" | "download" | "install") {
        return Err("Tindakan updater tidak dikenal.".into());
    }
    #[cfg(target_os = "android")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            app.state::<AppUpdater<tauri::Wry>>()
                .0
                .run_mobile_plugin("update", json!({ "action": action, "progress": progress }))
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = progress;
        Ok(json!({
            "supported": false,
            "currentVersion": app.package_info().version.to_string(),
            "currentVersionCode": null,
            "availableVersion": null,
            "size": 0,
            "ready": false,
            "checked": false,
            "message": "Updater APK hanya tersedia pada aplikasi Android release."
        }))
    }
}
