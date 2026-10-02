use std::path::Path;

use serde_json::{json, Value};
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

use crate::pak::{
    model::{DeviceProbe, Entry, FileStat, PakResult},
    storage::Storage,
};

pub struct MobileBridge<R: Runtime> {
    plugin: PluginHandle<R>,
}

impl<R: Runtime> MobileBridge<R> {
    pub fn probe(&self) -> PakResult<DeviceProbe> {
        self.invoke("probe", json!({}))
    }

    pub fn control(&self, action: &str) -> PakResult<()> {
        if !matches!(
            action,
            "request_permission" | "launch_game" | "open_shizuku" | "open_store"
        ) {
            return Err("Unsupported app control action".into());
        }
        self.invoke("control", json!({ "action": action }))
    }

    fn invoke<T: serde::de::DeserializeOwned>(
        &self,
        command: &str,
        payload: Value,
    ) -> PakResult<T> {
        self.plugin
            .run_mobile_plugin(command, payload)
            .map_err(|error| format!("Android {command} failed: {error}"))
    }

    fn storage<T: serde::de::DeserializeOwned>(&self, payload: Value) -> PakResult<T> {
        self.invoke("storage", payload)
    }

    fn source_path(source: &Path) -> PakResult<String> {
        source
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| "Android cache path is not valid UTF-8".to_string())
    }
}

impl<R: Runtime> Storage for MobileBridge<R> {
    fn list(&self, path: &str) -> PakResult<Vec<Entry>> {
        self.storage(json!({ "operation": "list", "path": path }))
    }

    fn stat(&self, path: &str) -> PakResult<FileStat> {
        self.storage(json!({ "operation": "stat", "path": path }))
    }

    fn read_text(&self, path: &str) -> PakResult<String> {
        self.storage(json!({ "operation": "read_text", "path": path }))
    }

    fn hash(&self, path: &str, algorithm: &str) -> PakResult<String> {
        self.storage(json!({
            "operation": "hash",
            "path": path,
            "algorithm": algorithm
        }))
    }

    fn mkdir(&self, path: &str) -> PakResult<()> {
        self.storage(json!({ "operation": "mkdir", "path": path }))
    }

    fn copy(&self, source: &str, target: &str) -> PakResult<()> {
        self.storage(json!({
            "operation": "copy",
            "source": source,
            "target": target
        }))
    }

    fn move_file(&self, source: &str, target: &str) -> PakResult<()> {
        self.storage(json!({
            "operation": "move_file",
            "source": source,
            "target": target
        }))
    }

    fn remove(&self, path: &str) -> PakResult<()> {
        self.storage(json!({ "operation": "remove", "path": path }))
    }

    fn write_text(&self, path: &str, content: &str) -> PakResult<()> {
        self.storage(json!({
            "operation": "write_text",
            "path": path,
            "content": content
        }))
    }

    fn write_from_file(&self, path: &str, source: &Path) -> PakResult<()> {
        let source = Self::source_path(source)?;
        self.storage(json!({
            "operation": "write_from_file",
            "path": path,
            "source": source
        }))
    }

    fn force_stop(&self) -> PakResult<()> {
        self.storage(json!({ "operation": "force_stop" }))
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("android-storage")
        .setup(|app, api| {
            let plugin =
                api.register_android_plugin("com.titotfp.wuwaid.storage", "GameStoragePlugin")?;
            app.manage(MobileBridge { plugin });
            Ok(())
        })
        .build()
}
