#[cfg(feature = "gpu")]
pub mod gpu;
mod model;
mod observer;
mod renderer;
mod state;
use model::{Command, Snapshot};
use state::SharedState;
use tauri::{Emitter, Manager};

#[tauri::command]
fn get_snapshot(state: tauri::State<SharedState>) -> Result<Snapshot, String> {
    state.snapshot()
}
#[tauri::command]
fn dispatch(
    app: tauri::AppHandle,
    state: tauri::State<SharedState>,
    command: Command,
) -> Result<Snapshot, String> {
    if matches!(&command,Command::SetPanel{panel} if panel=="floating") {
        return Err("use_float_panel".into());
    }
    let snap = state.dispatch(command)?;
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
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let (connections, firewall) = observer::observe();
                if let Ok(snapshot) = handle
                    .state::<SharedState>()
                    .observed(connections, firewall)
                {
                    let _ = handle.emit("state-changed", snapshot);
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
            dispatch,
            float_panel,
            dock_panel
        ])
        .run(tauri::generate_context!())
        .expect("FlowGuard could not start");
}
