mod discovery;
mod enforcement;
mod extension;
#[cfg(feature = "gpu")]
pub mod gpu;
mod model;
mod names;
mod native_runtime;
mod network;
mod observer;
mod renderer;
mod state;
use model::{Command, Rule, Snapshot};
use state::SharedState;
use tauri::{Emitter, Manager};

#[tauri::command]
async fn start_lan_discovery(
    app: tauri::AppHandle,
    interface: String,
    address: String,
    active: bool,
    name_mode: Option<String>,
) -> Result<Snapshot, String> {
    let network = tauri::async_runtime::spawn_blocking(network::observe)
        .await
        .map_err(|_| "interfaces_unavailable")?;
    let plan = discovery::Plan::new(&network, &interface, &address, active)?
        .with_names(name_mode.as_deref().unwrap_or("none"))?;
    let (operation, snapshot) = app.state::<SharedState>().begin_discovery(&plan)?;
    let _ = app.emit("state-changed", &snapshot);
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        discovery::run(
            plan,
            |result| {
                if let Ok(snapshot) = handle
                    .state::<SharedState>()
                    .update_discovery(operation, result)
                {
                    let _ = handle.emit("state-changed", snapshot);
                }
            },
            || handle.state::<SharedState>().discovery_cancelled(operation),
        );
    });
    Ok(snapshot)
}
#[tauri::command]
fn stop_lan_discovery(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().stop_discovery()?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn extension_package_status() -> Result<extension::ExtensionPackageStatus, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let contents = executable
        .parent()
        .and_then(|p| p.parent())
        .ok_or("bundle_path_unavailable")?;
    Ok(extension::package_status(contents))
}
#[tauri::command]
async fn extension_bridge_status() -> Result<extension::BridgeStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let contents = executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bundle_path_unavailable")?;
        extension::bridge_status(contents)
    })
    .await
    .map_err(|e| e.to_string())?
}
struct HostRuntime(Option<std::sync::Arc<native_runtime::NativeRuntime>>);
static ACTIVATIONS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<u64, (tauri::AppHandle, u64, Vec<Rule>)>>,
> = std::sync::OnceLock::new();
static NEXT_ACTIVATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
extern "C" fn lifecycle_callback(status: *const std::ffi::c_char, token: u64) {
    let Some(status) = native_runtime::callback_status(status) else {
        return;
    };
    let registry = ACTIVATIONS.get_or_init(Default::default);
    let target = {
        let Ok(mut entries) = registry.lock() else {
            return;
        };
        let Some(target) = entries.get(&token).cloned() else {
            return;
        };
        if status != "approval_required" {
            entries.remove(&token);
        }
        target
    };
    let (app, operation, rules) = target;
    if status == "configuration_saved" {
        if let Ok(snapshot) = app.state::<SharedState>().extension_change(|extension, _| {
            extension.lifecycle(operation, "initial_sync");
        }) {
            let _ = app.emit("state-changed", snapshot);
        }
        tauri::async_runtime::spawn(async move {
            let result = tauri::async_runtime::spawn_blocking(move || {
                let executable = std::env::current_exe().map_err(|e| e.to_string())?;
                let contents = executable
                    .parent()
                    .and_then(|p| p.parent())
                    .ok_or("bundle_path_unavailable")?;
                // Readiness probes are retryable; never retry an uncertain policy update.
                for _ in 0..10 {
                    if extension::bridge_status(contents).is_ok_and(|s| s.status == "ready") {
                        return extension::sync_rules(contents, &rules).map(|reply| (reply, rules));
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
                Err("initial_policy_sync_timeout".into())
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
            if let Ok(snapshot) =
                app.state::<SharedState>()
                    .extension_change(|extension, current| match result {
                        Ok((reply, rules)) => extension.accepted(
                            operation,
                            reply.instance_id,
                            reply.revision,
                            rules,
                            current,
                        ),
                        Err(error) => extension.fail(operation, error),
                    })
            {
                let _ = app.emit("state-changed", snapshot);
            }
        });
        return;
    }
    if let Ok(snapshot) = app.state::<SharedState>().extension_change(|extension, _| {
        extension.lifecycle(operation, &status);
    }) {
        let _ = app.emit("state-changed", snapshot);
    }
}
#[tauri::command]
fn activate_extension(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let runtime = app
        .state::<HostRuntime>()
        .0
        .clone()
        .ok_or("native_runtime_unavailable")?;
    let (operation, rules, snapshot) = app.state::<SharedState>().begin_extension("activating")?;
    let payload = extension::policy_payload(&rules)?;
    let _ = app.emit("state-changed", &snapshot);
    let token = NEXT_ACTIVATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    ACTIVATIONS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "state_unavailable")?
        .insert(token, (app.clone(), operation, rules));
    runtime.activate(&payload, lifecycle_callback, token);
    app.state::<SharedState>().snapshot()
}
#[tauri::command]
fn disable_extension(app: tauri::AppHandle, remove: bool) -> Result<Snapshot, String> {
    let runtime = app
        .state::<HostRuntime>()
        .0
        .clone()
        .ok_or("native_runtime_unavailable")?;
    let (operation, _, snapshot) = app.state::<SharedState>().begin_extension(if remove {
        "removing"
    } else {
        "disabling"
    })?;
    let _ = app.emit("state-changed", &snapshot);
    let token = NEXT_ACTIVATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    ACTIVATIONS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "state_unavailable")?
        .insert(token, (app.clone(), operation, vec![]));
    runtime.disable(remove, lifecycle_callback, token);
    app.state::<SharedState>().snapshot()
}
#[tauri::command]
async fn sync_extension_rules(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let (operation, rules, snapshot) = app.state::<SharedState>().begin_extension("applying")?;
    let _ = app.emit("state-changed", &snapshot);
    let sent = rules.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let contents = executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bundle_path_unavailable")?;
        extension::sync_rules(contents, &sent)
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|value| value);
    let snapshot =
        app.state::<SharedState>()
            .extension_change(|extension, current| match result {
                Ok(reply) => {
                    extension.accepted(operation, reply.instance_id, reply.revision, rules, current)
                }
                Err(error) => extension.fail(operation, error),
            })?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn refresh_extension(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let (signed, generation) = {
        let state = app.state::<SharedState>();
        let s = state.inner.lock().map_err(|_| "state_unavailable")?;
        (
            s.extension.status.signed
                && !s.extension.status.busy
                && !["restart_required", "disabled", "removed"]
                    .contains(&s.extension.status.stage.as_str()),
            s.extension.generation(),
        )
    };
    if !signed {
        return app.state::<SharedState>().snapshot();
    }
    let result = tauri::async_runtime::spawn_blocking(|| {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        extension::bridge_status(
            executable
                .parent()
                .and_then(|p| p.parent())
                .ok_or("bundle_path_unavailable")?,
        )
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|value| value);
    let snapshot = app
        .state::<SharedState>()
        .extension_change(|extension, _| {
            if extension.generation() != generation {
                return;
            }
            match result {
                Ok(reply) => {
                    extension.observed(&reply.instance_id, reply.revision, reply.status == "ready")
                }
                Err(error) => extension.disconnected(&error),
            }
        })?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn get_snapshot(state: tauri::State<SharedState>) -> Result<Snapshot, String> {
    state.snapshot()
}
#[tauri::command]
fn dispatch(
    app: tauri::AppHandle,
    state: tauri::State<SharedState>,
    command: Command,
    expected_rules: Vec<Rule>,
) -> Result<Snapshot, String> {
    if matches!(&command,Command::SetPanel{panel} if panel=="floating") {
        return Err("use_float_panel".into());
    }
    let snap = match state.dispatch_checked(command, Some(&expected_rules)) {
        Ok(snap) => snap,
        Err(error) => {
            if error == "edit_conflict" {
                if let Ok(current) = state.snapshot() {
                    let _ = app.emit("state-changed", current);
                }
            }
            return Err(error);
        }
    };
    app.emit("state-changed", &snap)
        .map_err(|e| e.to_string())?;
    Ok(snap)
}
#[tauri::command]
async fn float_panel(app: tauri::AppHandle) -> Result<Snapshot, String> {
    if let Some(window) = app.get_webview_window("inspector") {
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "inspector",
            tauri::WebviewUrl::App("index.html?panel=inspector".into()),
        )
        .title("FlowGuard · Connection details")
        .inner_size(380.0, 840.0)
        .min_inner_size(320.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    let snap = app.state::<SharedState>().dispatch(Command::SetPanel {
        panel: "floating".into(),
    })?;
    app.emit("state-changed", &snap)
        .map_err(|e| e.to_string())?;
    Ok(snap)
}
#[tauri::command]
async fn dock_panel(app: tauri::AppHandle, panel: String) -> Result<Snapshot, String> {
    if !["left", "right"].contains(&panel.as_str()) {
        return Err("invalid_panel".into());
    }
    let snap = app
        .state::<SharedState>()
        .dispatch(Command::SetPanel { panel })?;
    app.emit("state-changed", &snap)
        .map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("inspector") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(snap)
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("document-v1.json");
            app.manage(SharedState::load(path).map_err(std::io::Error::other)?);
            let executable = std::env::current_exe()?;
            let contents = executable
                .parent()
                .and_then(|p| p.parent())
                .ok_or_else(|| std::io::Error::other("bundle_path_unavailable"))?;
            let package = extension::package_status(contents);
            let runtime = native_runtime::NativeRuntime::load(contents)
                .ok()
                .map(std::sync::Arc::new);
            let signed = runtime.as_ref().is_some_and(|runtime| runtime.signed());
            app.state::<SharedState>()
                .extension_change(|extension, _| {
                    extension.configure(
                        package.packaged && package.bridge_available,
                        runtime.is_some(),
                        signed,
                    )
                })
                .map_err(std::io::Error::other)?;
            app.manage(HostRuntime(runtime));
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let (connections, firewall) = observer::observe();
                if let Ok(snapshot) = handle.state::<SharedState>().observed(
                    connections,
                    firewall,
                    network::observe(),
                ) {
                    let _ = handle.emit("state-changed", snapshot);
                }
                let should_poll = handle.state::<SharedState>().snapshot().is_ok_and(|s| {
                    s.extension.signed
                        && !s.extension.busy
                        && !["restart_required", "disabled", "removed"]
                            .contains(&s.extension.stage.as_str())
                });
                if should_poll {
                    let _ = tauri::async_runtime::block_on(refresh_extension(handle.clone()));
                }
                std::thread::sleep(std::time::Duration::from_secs(5));
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "inspector" && matches!(event, tauri::WindowEvent::Destroyed) {
                let app = window.app_handle();
                if let Ok(current) = app.state::<SharedState>().snapshot() {
                    if current.document.preferences.panel == "floating" {
                        if let Ok(snapshot) =
                            app.state::<SharedState>().dispatch(Command::SetPanel {
                                panel: "right".into(),
                            })
                        {
                            let _ = app.emit("state-changed", snapshot);
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            start_lan_discovery,
            stop_lan_discovery,
            extension_package_status,
            extension_bridge_status,
            sync_extension_rules,
            activate_extension,
            disable_extension,
            refresh_extension,
            dispatch,
            float_panel,
            dock_panel
        ])
        .run(tauri::generate_context!())
        .expect("FlowGuard could not start");
}
