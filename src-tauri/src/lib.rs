mod app_icons;
mod asks;
mod destination_names;
mod diagnostics;
mod discovery;
mod enforcement;
mod extension;
mod firewall_control;
mod flow_names;
#[cfg(feature = "gpu")]
pub mod gpu;
mod gpu_filter;
mod gpu_panel;
#[cfg(feature = "gpu")]
mod gpu_scene;
mod history;
mod identity;
#[cfg(target_os = "macos")]
mod instance;
mod inventory;
mod lan_metadata;
mod lan_vendor;
mod menubar;
mod model;
mod names;
mod native_runtime;
mod network;
mod notifications;
mod observer;
mod port_layout;
mod ports;
mod public_network;
mod renderer;
pub mod routers;
mod security;
mod service_comparison;
mod service_notes;
mod services;
mod state;
mod syslog;
mod system_protection;
mod temporary_protection;
mod tls_details;
mod unknown_flows;
mod update_arch;
mod update_cache;
mod update_trust;
mod updates;
use model::{Command, Rule, Snapshot};
use state::SharedState;
use tauri::{Emitter, Manager};

#[tauri::command]
async fn temporary_protection_action(
    app: tauri::AppHandle,
    action: String,
    connection_id: Option<String>,
    id: Option<String>,
    direction: Option<String>,
    peer_evidence: Option<String>,
    seconds: Option<u16>,
    confirmed: bool,
) -> Result<Snapshot, String> {
    let fresh = if action == "add" {
        Some(
            tauri::async_runtime::spawn_blocking(network::observe)
                .await
                .map_err(|_| "network_unavailable")?,
        )
    } else {
        None
    };
    let next = app.state::<SharedState>().temporary_action(
        &action,
        connection_id.as_deref(),
        id.as_deref(),
        direction.as_deref(),
        peer_evidence.as_deref(),
        seconds.unwrap_or(60),
        confirmed,
        fresh,
    )?;
    let _ = app.emit("state-changed", &next);
    Ok(next)
}
async fn poll_temporary(app: tauri::AppHandle) {
    let Ok(Some(ticket)) = app.state::<SharedState>().begin_temporary_sync() else {
        return;
    };
    let work = ticket.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let executable = std::env::current_exe().map_err(|_| "bundle_path")?;
        let contents = executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bundle_path")?;
        temporary_protection::transact(contents, &work)
    })
    .await
    .unwrap_or_else(|_| Err("temporary_unavailable".into()));
    if let Ok(next) = app
        .state::<SharedState>()
        .finish_temporary_sync(&ticket, result)
    {
        let _ = app.emit("state-changed", next);
    }
}
#[tauri::command]
async fn clear_update_download_cache(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().snapshot()?;
    if ["checking", "downloading", "installing"].contains(&snapshot.updates.stage.as_str()) {
        return Err("updater_busy".into());
    }
    let root = app
        .path()
        .app_cache_dir()
        .map_err(|_| "update_cache_failed")?
        .join("updater-v1");
    tauri::async_runtime::spawn_blocking(move || update_cache::Cache::clear(&root))
        .await
        .map_err(|_| "update_cache_failed")??;
    app.state::<SharedState>().snapshot()
}
#[tauri::command]
fn set_flow_names(
    app: tauri::AppHandle,
    enabled: bool,
    search: bool,
    operation: u64,
) -> Result<Snapshot, String> {
    let next = app
        .state::<SharedState>()
        .configure_flow_names(enabled, search, operation)?;
    let _ = app.emit("state-changed", &next);
    Ok(next)
}
#[tauri::command]
async fn collect_flow_names(app: tauri::AppHandle, operation: u64) -> Result<Snapshot, String> {
    let (ticket, next) = app.state::<SharedState>().begin_flow_names(operation)?;
    let _ = app.emit("state-changed", next);
    let work = ticket.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let executable = std::env::current_exe().map_err(|_| "bundle_path")?;
        let contents = executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bundle_path")?;
        flow_names::query(contents, &work)
    })
    .await
    .unwrap_or_else(|_| Err("flow_name_worker".into()));
    let next = app
        .state::<SharedState>()
        .finish_flow_names(&ticket, result)?;
    let _ = app.emit("state-changed", &next);
    Ok(next)
}
#[tauri::command]
async fn set_destination_names(
    app: tauri::AppHandle,
    enabled: bool,
    operation: u64,
) -> Result<Snapshot, String> {
    let fresh = if enabled {
        Some(
            tauri::async_runtime::spawn_blocking(network::observe)
                .await
                .map_err(|_| "network_unavailable")?,
        )
    } else {
        None
    };
    let snapshot = app
        .state::<SharedState>()
        .configure_destination_names(enabled, operation, fresh)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn clear_destination_names(app: tauri::AppHandle, operation: u64) -> Result<Snapshot, String> {
    let snapshot = app
        .state::<SharedState>()
        .clear_destination_names(operation)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn lookup_destination_name(
    app: tauri::AppHandle,
    connection_id: String,
    address: String,
    operation: u64,
) -> Result<Snapshot, String> {
    let fresh = tauri::async_runtime::spawn_blocking(network::observe)
        .await
        .map_err(|_| "network_unavailable")?;
    let (ticket, snapshot) = app.state::<SharedState>().begin_destination_name(
        &connection_id,
        &address,
        operation,
        &fresh,
    )?;
    let _ = app.emit("state-changed", &snapshot);
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = std::panic::catch_unwind(|| {
            if ticket.matches_network(&network::observe()) {
                destination_names::resolve(&ticket)
            } else {
                (None, "failed".into())
            }
        })
        .unwrap_or((None, "failed".into()));
        let fresh = network::observe();
        if let Ok(snapshot) = handle
            .state::<SharedState>()
            .finish_destination_name(&ticket, result.0, &result.1, &fresh)
        {
            let _ = handle.emit("state-changed", snapshot);
        }
    });
    Ok(snapshot)
}
#[tauri::command]
async fn answer_ask(
    app: tauri::AppHandle,
    request_id: String,
    instance_id: String,
    policy_id: String,
    action: String,
) -> Result<Snapshot, String> {
    let executable = std::env::current_exe().map_err(|_| "bundle_path_unavailable")?;
    let contents = executable
        .parent()
        .and_then(|p| p.parent())
        .ok_or("bundle_path_unavailable")?
        .to_path_buf();
    let (op, answer, snapshot) =
        app.state::<SharedState>()
            .claim_ask(&request_id, &instance_id, &policy_id, &action)?;
    let _ = app.emit("state-changed", snapshot);
    let sent = answer.clone();
    let result = tauri::async_runtime::spawn_blocking(move || asks::answer(&contents, &sent))
        .await
        .unwrap_or_else(|_| Err("ask_answer_unknown".into()));
    let snapshot = app
        .state::<SharedState>()
        .finish_ask_answer(op, &answer, result)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> Result<Snapshot, String> {
    updates::check(app, false).await
}
#[tauri::command]
async fn download_update(app: tauri::AppHandle) -> Result<Snapshot, String> {
    updates::download(app).await
}
#[tauri::command]
fn cancel_update(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let (_, s) = app.state::<SharedState>().update_change(|m| m.cancel())?;
    let _ = app.emit("state-changed", &s);
    Ok(s)
}
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let exe = std::env::current_exe().map_err(|_| "install_failed")?;
    let bundle = exe
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or("install_failed")?;
    if bundle.file_name().and_then(|p| p.to_str()) != Some("FlowGuard.app")
        || bundle
            .to_string_lossy()
            .chars()
            .any(|c| c == '\\' || c == '\'' || c == '"' || c.is_control())
    {
        return Err("install_failed".into());
    }
    let (op, update, bytes, s) = app.state::<SharedState>().begin_update_install()?;
    let _ = app.emit("state-changed", s);
    let bundle = bundle.to_path_buf();
    let installed = tauri::async_runtime::spawn_blocking(move || {
        updates::install_with_backup(update, bytes, &bundle)
    })
    .await;
    if !matches!(installed, Ok(Ok(()))) {
        let (_, s) = app.state::<SharedState>().update_change(|m| {
            m.fail(
                op,
                if matches!(&installed,Ok(Err(error))if error=="rollback_failed") {
                    "rollback_failed"
                } else {
                    "install_failed"
                },
            )
        })?;
        let _ = app.emit("state-changed", &s);
        return Ok(s);
    }
    app.restart()
}
#[tauri::command]
async fn control_firewall(
    app: tauri::AppHandle,
    action: firewall_control::Action,
    expected: Option<bool>,
    acknowledged: bool,
) -> Result<Snapshot, String> {
    let executable = std::env::current_exe().map_err(|_| "bundle_path_unavailable")?;
    let contents = executable
        .parent()
        .and_then(|p| p.parent())
        .ok_or("bundle_path_unavailable")?
        .to_path_buf();
    let (op, snapshot) =
        app.state::<SharedState>()
            .begin_firewall_control(action, expected, acknowledged)?;
    let _ = app.emit("state-changed", snapshot);
    let result = tauri::async_runtime::spawn_blocking(move || {
        firewall_control::execute(&contents, action, expected)
    })
    .await
    .unwrap_or_else(|_| Err("result_unknown".into()));
    let snapshot = app
        .state::<SharedState>()
        .finish_firewall_control(op, result)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn start_syslog(app: tauri::AppHandle, config: syslog::Config) -> Result<Snapshot, String> {
    let network = tauri::async_runtime::spawn_blocking(network::observe)
        .await
        .map_err(|_| "address_unavailable")?;
    let (ticket, snapshot) = app.state::<SharedState>().begin_syslog(config, &network)?;
    let _ = app.emit("state-changed", &snapshot);
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        syslog::run(ticket, |result| {
            if let Ok(snapshot) = handle.state::<SharedState>().update_syslog(result) {
                let _ = handle.emit("state-changed", snapshot);
            }
        })
    });
    Ok(snapshot)
}
#[tauri::command]
fn stop_syslog(app: tauri::AppHandle, operation: u64) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().stop_syslog(operation)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn clear_syslog(app: tauri::AppHandle, operation: u64) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().clear_syslog(operation)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn syslog_csv(app: tauri::AppHandle, operation: u64) -> Result<String, String> {
    app.state::<SharedState>().syslog_csv(operation)
}
#[tauri::command]
fn select_syslog(app: tauri::AppHandle, operation: u64, id: u64) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().select_syslog(operation, id)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn start_services(
    app: tauri::AppHandle,
    operation: u64,
    mode: String,
    hostname: Option<String>,
) -> Result<Snapshot, String> {
    let network = tauri::async_runtime::spawn_blocking(network::observe)
        .await
        .map_err(|_| "interfaces_unavailable")?;
    let plan = services::Plan::new(
        &app.state::<SharedState>().snapshot()?,
        &network,
        operation,
        &mode,
    )?
    .with_hostname(hostname.as_deref().unwrap_or(""))?;
    let (op, snapshot) = app.state::<SharedState>().begin_services(&plan)?;
    let _ = app.emit("state-changed", &snapshot);
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        services::run(
            plan,
            |result| {
                if let Ok(s) = handle.state::<SharedState>().update_services(op, result) {
                    let _ = handle.emit("state-changed", s);
                }
            },
            || handle.state::<SharedState>().services_cancelled(op),
        )
    });
    Ok(snapshot)
}
#[tauri::command]
fn stop_services(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let s = app.state::<SharedState>().stop_services()?;
    let _ = app.emit("state-changed", &s);
    Ok(s)
}
#[tauri::command]
fn diagnostic_action(
    app: tauri::AppHandle,
    service_operation: u64,
    report: u64,
    finding: Option<String>,
    action: String,
    disposition: Option<diagnostics::Disposition>,
    reason: Option<String>,
) -> Result<Snapshot, String> {
    let next = app.state::<SharedState>().diagnostic_action(
        service_operation,
        report,
        finding,
        action,
        disposition,
        reason,
    )?;
    let _ = app.emit("state-changed", &next);
    Ok(next)
}
#[tauri::command]
fn diagnostic_export(app: tauri::AppHandle, report: u64, format: String) -> Result<String, String> {
    app.state::<SharedState>()
        .diagnostic_export(report, &format)
}
#[tauri::command]
fn services_csv(app: tauri::AppHandle, operation: u64) -> Result<String, String> {
    app.state::<SharedState>().services_csv(operation)
}
#[tauri::command]
fn resize_evidence_panel(
    app: tauri::AppHandle,
    kind: port_layout::EvidenceKind,
    width: u16,
    expected: u16,
) -> Result<Snapshot, String> {
    let result = app
        .state::<SharedState>()
        .resize_evidence_panel(kind, width, expected);
    if let Ok(snapshot) = app.state::<SharedState>().snapshot() {
        let _ = app.emit("state-changed", snapshot);
    }
    result
}
#[tauri::command]
fn position_port_details(
    app: tauri::AppHandle,
    panel: port_layout::Panel,
) -> Result<Snapshot, String> {
    use port_layout::Panel;
    if panel == Panel::Floating {
        let locale = app
            .state::<SharedState>()
            .snapshot()?
            .document
            .preferences
            .locale;
        let title = match locale.as_str() {
            "ja" => "FlowGuard · TCPポート詳細",
            "zh-CN" => "FlowGuard · TCP端口详情",
            _ => "FlowGuard · TCP port details",
        };
        let window = if let Some(window) = app.get_webview_window("port-details") {
            window
        } else {
            tauri::WebviewWindowBuilder::new(
                &app,
                "port-details",
                tauri::WebviewUrl::App("index.html?panel=port-details".into()),
            )
            .title(title)
            .inner_size(500.0, 680.0)
            .min_inner_size(320.0, 420.0)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?
        };
        window.set_title(title).map_err(|e| e.to_string())?;
        if let Err(error) = window.show().and_then(|_| window.set_focus()) {
            let _ = window.hide();
            if let Ok(snapshot) = app.state::<SharedState>().port_panel_closed() {
                let _ = app.emit("state-changed", snapshot);
            }
            return Err(error.to_string());
        }
    } else if let Some(window) = app.get_webview_window("port-details") {
        window.hide().map_err(|e| e.to_string())?;
    }
    let snapshot = app.state::<SharedState>().port_panel(panel)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn position_service_details(
    app: tauri::AppHandle,
    panel: port_layout::Panel,
) -> Result<Snapshot, String> {
    use port_layout::Panel;
    if panel == Panel::Floating {
        let locale = app
            .state::<SharedState>()
            .snapshot()?
            .document
            .preferences
            .locale;
        let title = match locale.as_str() {
            "ja" => "FlowGuard · サービス詳細",
            "zh-CN" => "FlowGuard · 服务详情",
            _ => "FlowGuard · Service details",
        };
        let window = if let Some(window) = app.get_webview_window("service-details") {
            window
        } else {
            tauri::WebviewWindowBuilder::new(
                &app,
                "service-details",
                tauri::WebviewUrl::App("index.html?panel=service-details".into()),
            )
            .title(title)
            .inner_size(560.0, 800.0)
            .min_inner_size(320.0, 420.0)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?
        };
        window.set_title(title).map_err(|e| e.to_string())?;
        if let Err(error) = window.show().and_then(|_| window.set_focus()) {
            let _ = window.hide();
            if let Ok(snapshot) = app.state::<SharedState>().service_panel_closed() {
                let _ = app.emit("state-changed", snapshot);
            }
            return Err(error.to_string());
        }
    } else if let Some(window) = app.get_webview_window("service-details") {
        window.hide().map_err(|e| e.to_string())?;
    }
    let snapshot = app.state::<SharedState>().service_panel(panel)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn set_port_layout(
    app: tauri::AppHandle,
    expected: port_layout::Layout,
    value: port_layout::Layout,
) -> Result<Snapshot, String> {
    let result = app.state::<SharedState>().set_port_layout(&expected, value);
    if let Ok(current) = app.state::<SharedState>().snapshot() {
        let _ = app.emit("state-changed", current);
    }
    result
}
#[tauri::command]
fn select_port(app: tauri::AppHandle, operation: u64, port: u16) -> Result<Snapshot, String> {
    let next = app.state::<SharedState>().select_port(operation, port)?;
    let _ = app.emit("state-changed", &next);
    Ok(next)
}
#[tauri::command]
fn edit_service_note(
    app: tauri::AppHandle,
    operation: u64,
    command: service_notes::Edit,
) -> Result<Snapshot, String> {
    let result = app
        .state::<SharedState>()
        .edit_service_note(operation, command);
    if let Ok(snapshot) = app.state::<SharedState>().snapshot() {
        let _ = app.emit("state-changed", snapshot);
    }
    result
}
#[tauri::command]
fn select_service(app: tauri::AppHandle, operation: u64, port: u16) -> Result<Snapshot, String> {
    let s = app.state::<SharedState>().select_service(operation, port)?;
    let _ = app.emit("state-changed", &s);
    Ok(s)
}
#[tauri::command]
async fn open_lan_ports(
    app: tauri::AppHandle,
    id: String,
    generation: u64,
) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().open_ports(&id, generation)?;
    let _ = app.emit("state-changed", &snapshot);
    let main = if let Some(main) = app.get_webview_window("main") {
        main
    } else {
        let config = app
            .config()
            .app
            .windows
            .iter()
            .find(|w| w.label == "main")
            .ok_or("main_window_config_missing")?;
        tauri::WebviewWindowBuilder::from_config(&app, config)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?
    };
    main.show().map_err(|e| e.to_string())?;
    main.unminimize().map_err(|e| e.to_string())?;
    main.set_focus().map_err(|e| e.to_string())?;
    Ok(snapshot)
}
#[tauri::command]
async fn start_lan_ports(
    app: tauri::AppHandle,
    id: String,
    generation: u64,
    first: u16,
    last: u16,
    preset: Option<String>,
    settings: Option<ports::ScanSettings>,
) -> Result<Snapshot, String> {
    let network = tauri::async_runtime::spawn_blocking(network::observe)
        .await
        .map_err(|_| "interfaces_unavailable")?;
    let preset = preset.as_deref().unwrap_or("range");
    let (first, last) = if preset == "range" {
        (first, last)
    } else {
        (1, 1)
    };
    let plan = ports::Plan::new(
        &app.state::<SharedState>().snapshot()?,
        &network,
        &id,
        generation,
        first,
        last,
    )?
    .with_preset(preset)?
    .with_settings(settings.unwrap_or_default())?;
    let (op, snapshot) = app.state::<SharedState>().begin_ports(&plan)?;
    let _ = app.emit("state-changed", &snapshot);
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ports::run(
            plan,
            |result| {
                if let Ok(s) = handle.state::<SharedState>().update_ports(op, result) {
                    let _ = handle.emit("state-changed", s);
                }
            },
            || handle.state::<SharedState>().ports_cancelled(op),
        )
    });
    Ok(snapshot)
}
#[tauri::command]
fn lan_ports_csv(app: tauri::AppHandle, operation: u64) -> Result<String, String> {
    app.state::<SharedState>().ports_csv(operation)
}
#[tauri::command]
fn stop_lan_ports(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let s = app.state::<SharedState>().stop_ports()?;
    let _ = app.emit("state-changed", &s);
    Ok(s)
}
#[tauri::command]
fn select_lan_device(app: tauri::AppHandle, id: String) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().lan_selection(&id)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn edit_lan_inventory(
    app: tauri::AppHandle,
    generation: u64,
    command: inventory::Edit,
) -> Result<Snapshot, String> {
    let result = app.state::<SharedState>().lan_edit(generation, command);
    if let Ok(current) = app.state::<SharedState>().snapshot() {
        let _ = app.emit("state-changed", current);
    }
    result
}
#[tauri::command]
async fn copy_lan_selection(app: tauri::AppHandle, generation: u64) -> Result<(), String> {
    let text = app.state::<SharedState>().lan_copy(generation)?;
    if text.len() > 8 * 1024 * 1024 {
        return Err("clipboard_payload_limit".into());
    }
    #[cfg(target_os = "macos")]
    return tauri::async_runtime::spawn_blocking(move || {
        let (ok, _) = observer::run_bounded_input("/usr/bin/pbcopy", &[], Some(text.into_bytes()))?;
        if ok {
            Ok(())
        } else {
            Err("clipboard_unavailable".into())
        }
    })
    .await
    .map_err(|_| "clipboard_unavailable")?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        Err("clipboard_unavailable".into())
    }
}
#[tauri::command]
fn lan_csv(app: tauri::AppHandle, generation: u64) -> Result<String, String> {
    app.state::<SharedState>().lan_csv(generation)
}
#[tauri::command]
async fn float_lan_details(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let locale = app
        .state::<SharedState>()
        .snapshot()?
        .document
        .preferences
        .locale;
    let title = match locale.as_str() {
        "ja" => "FlowGuard · LAN記録の詳細",
        "zh-CN" => "FlowGuard · LAN记录详情",
        _ => "FlowGuard · LAN record details",
    };
    if let Some(window) = app.get_webview_window("lan-details") {
        window.set_title(title).map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "lan-details",
            tauri::WebviewUrl::App("index.html?panel=lan-details".into()),
        )
        .title(title)
        .inner_size(430.0, 800.0)
        .min_inner_size(320.0, 500.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    let snapshot = app.state::<SharedState>().lan_panel("floating")?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn dock_lan_details(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let snapshot = app.state::<SharedState>().lan_panel("docked")?;
    let _ = app.emit("state-changed", &snapshot);
    if let Some(window) = app.get_webview_window("lan-details") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(snapshot)
}
#[tauri::command]
async fn start_lan_discovery(
    app: tauri::AppHandle,
    interface: String,
    address: String,
    active: bool,
    name_mode: Option<String>,
    options: Option<discovery::ScanOptions>,
) -> Result<Snapshot, String> {
    let network = tauri::async_runtime::spawn_blocking(network::observe)
        .await
        .map_err(|_| "interfaces_unavailable")?;
    let context_interface = interface.clone();
    let context =
        tauri::async_runtime::spawn_blocking(move || network::lan_context(&context_interface))
            .await
            .map_err(|_| "context_unavailable")?;
    let plan = discovery::Plan::new(&network, &interface, &address, active)?
        .with_names(name_mode.as_deref().unwrap_or("none"))?
        .with_options(options.unwrap_or_default())?
        .with_context(context);
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
fn get_lan_archive(
    app: tauri::AppHandle,
    generation: u64,
    observed_at: u64,
) -> Result<inventory::Archive, String> {
    app.state::<SharedState>()
        .get_lan_archive(generation, observed_at)
}
#[tauri::command]
fn configure_lan_public(
    app: tauri::AppHandle,
    enabled: bool,
    expected: u64,
) -> Result<Snapshot, String> {
    let snapshot = app
        .state::<SharedState>()
        .configure_lan_public(enabled, expected)?;
    let _ = app.emit("state-changed", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn query_lan_public(app: tauri::AppHandle, expected: u64) -> Result<Snapshot, String> {
    let (op, snapshot) = app.state::<SharedState>().begin_lan_public(expected)?;
    let _ = app.emit("state-changed", &snapshot);
    let result = public_network::fetch().await;
    let snapshot = app.state::<SharedState>().finish_lan_public(op, result)?;
    let _ = app.emit("state-changed", &snapshot);
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
        let mut admitted = false;
        if let Ok(snapshot) = app.state::<SharedState>().extension_change(|extension, _| {
            admitted = extension.lifecycle(operation, "initial_sync");
        }) {
            let _ = app.emit("state-changed", snapshot);
        }
        if !admitted {
            return;
        }
        tauri::async_runtime::spawn(async move {
            let initial_handle = app.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                let executable = std::env::current_exe().map_err(|e| e.to_string())?;
                let contents = executable
                    .parent()
                    .and_then(|p| p.parent())
                    .ok_or("bundle_path_unavailable")?;
                // Readiness probes are retryable; never retry an uncertain policy update.
                for _ in 0..10 {
                    let current = || {
                        initial_handle
                            .state::<SharedState>()
                            .inner
                            .lock()
                            .is_ok_and(|s| s.extension.generation() == operation)
                    };
                    if !current() {
                        return Err("operation_cancelled".into());
                    }
                    if extension::bridge_status(contents).is_ok_and(|s| s.status == "ready") {
                        if !current() {
                            return Err("operation_cancelled".into());
                        }
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
    runtime.activate(&payload, operation, lifecycle_callback, token);
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
    runtime.disable(remove, operation, lifecycle_callback, token);
    app.state::<SharedState>().snapshot()
}
#[tauri::command]
async fn sync_extension_rules(app: tauri::AppHandle) -> Result<Snapshot, String> {
    sync_extension(app, false).await
}
async fn sync_extension(app: tauri::AppHandle, recovery: bool) -> Result<Snapshot, String> {
    let state = app.state::<SharedState>();
    let (operation, rules, snapshot) = if recovery {
        state.begin_recovery(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        )?
    } else {
        state.begin_extension("applying")?
    };
    let _ = app.emit("state-changed", &snapshot);
    let sent = rules.clone();
    let runtime = app.state::<HostRuntime>().0.clone();
    let operation_handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let contents = executable
            .parent()
            .and_then(|p| p.parent())
            .ok_or("bundle_path_unavailable")?;
        let runtime = runtime.ok_or("native_runtime_unavailable")?;
        runtime.persist_blocking(&extension::policy_payload(&sent)?, operation)?;
        {
            let state = operation_handle.state::<SharedState>();
            let session = state.inner.lock().map_err(|_| "state_unavailable")?;
            if session.extension.generation() != operation {
                return Err("operation_cancelled".into());
            }
        }
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
    let reachable = result.as_ref().is_ok_and(|reply| reply.status == "ready");
    let snapshot = app
        .state::<SharedState>()
        .extension_change(|extension, _| {
            if extension.generation() != generation {
                return;
            }
            match result {
                Ok(reply) => {
                    extension.observed(&reply.instance_id, reply.revision, reply.status == "ready");
                    if let Some(expected) = extension.expected_digest() {
                        if reply.policy_digest.as_deref() != Some(expected.as_str()) {
                            extension.mismatch();
                        }
                    }
                }
                Err(error) => extension.disconnected(&error),
            }
        })?;
    let _ = app.emit("state-changed", &snapshot);
    if reachable
        && app.state::<SharedState>().inner.lock().is_ok_and(|s| {
            s.extension.recovery_due(
                &s.document.rules,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            )
        })
    {
        return sync_extension(app, true).await;
    }
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
    let mute = matches!(
        &command,
        Command::SetAskNotifications { enabled: false }
            | Command::SetUnknownNotifications { enabled: false }
    );
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
    if mute {
        notifications::clear(&app, snap.revision);
    }
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("document-v1.json");
            #[cfg(target_os = "macos")]
            let activation_path = instance::socket_path(&path)
                .unwrap_or_else(|_| path.with_file_name("activation-v1.sock"));
            #[cfg(target_os = "macos")]
            if instance::request(&activation_path).unwrap_or(false) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
                app.handle().exit(0);
                return Ok(());
            }
            let state = match SharedState::load(path) {
                Ok(state) => state,
                Err(error) => {
                    #[cfg(target_os = "macos")]
                    if error == state::ALREADY_RUNNING {
                        // The owner may still be creating its listener after acquiring the lock.
                        for _ in 0..10 {
                            if instance::request(&activation_path).unwrap_or(false) {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                                app.handle().exit(0);
                                return Ok(());
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                    return Err(std::io::Error::other(error).into());
                }
            };
            app.manage(state);
            #[cfg(target_os = "macos")]
            {
                let handle = app.handle().clone();
                match instance::start(activation_path, move || {
                    if handle.get_webview_window("main").is_none() {
                        return false;
                    }
                    let app = handle.clone();
                    handle
                        .run_on_main_thread(move || {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        })
                        .is_ok()
                }) {
                    Ok(server) => {
                        app.manage(server);
                    }
                    Err(error) => eprintln!("Activation IPC unavailable: {error}"),
                }
            }
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
            if let Err(error) = menubar::install(app.handle()) {
                eprintln!("Menu bar unavailable: {error}");
            }
            updates::schedule(app.handle().clone());
            let ask_handle = app.handle().clone();
            let ask_contents = contents.to_path_buf();
            std::thread::spawn(move || {
                let mut planner = notifications::Planner::default();
                loop {
                    notifications::poll(&ask_handle, &mut planner);
                    menubar::refresh(&ask_handle);
                    if let Ok((op, instance)) = ask_handle.state::<SharedState>().begin_ask_poll() {
                        let result = asks::poll(&ask_contents);
                        if let Ok(snapshot) = ask_handle
                            .state::<SharedState>()
                            .finish_ask_poll(op, &instance, result)
                        {
                            let _ = ask_handle.emit("state-changed", snapshot);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
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
            let temporary_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    poll_temporary(temporary_app.clone()).await;
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if ["main", "gpu-map"].contains(&window.label())
                && matches!(
                    event,
                    tauri::WindowEvent::Resized(_)
                        | tauri::WindowEvent::CloseRequested { .. }
                        | tauri::WindowEvent::Destroyed
                )
            {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    gpu_panel::window_event(window.label(), true);
                } else {
                    gpu_panel::window_event(window.label(), false);
                }
            }
            if window.label() == "main"
                && window
                    .app_handle()
                    .try_state::<menubar::MenuBar>()
                    .is_some()
            {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if window.hide().is_ok() {
                        api.prevent_close();
                    }
                }
            }
            if window.label() == "gpu-map" && matches!(event, tauri::WindowEvent::Destroyed) {
                let shared = window.app_handle().state::<SharedState>();
                if let Ok(mut s) = shared.inner.lock() {
                    if s.gpu_owner == "gpu-map" {
                        s.gpu_owner = "main".into();
                        s.revision += 1;
                    }
                }
                if let Ok(snapshot) = shared.snapshot() {
                    let _ = window.emit("state-changed", snapshot);
                }
            }
            if window.label() == "port-details" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        if window.hide().is_ok() {
                            api.prevent_close();
                            let app = window.app_handle();
                            if let Ok(snapshot) = app.state::<SharedState>().port_panel_closed() {
                                let _ = app.emit("state-changed", snapshot);
                            }
                        }
                    }
                    tauri::WindowEvent::Destroyed => {
                        let app = window.app_handle();
                        if let Ok(snapshot) = app.state::<SharedState>().port_panel_closed() {
                            let _ = app.emit("state-changed", snapshot);
                        }
                    }
                    _ => {}
                }
            }
            if window.label() == "service-details" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        if window.hide().is_ok() {
                            api.prevent_close();
                            let app = window.app_handle();
                            if let Ok(snapshot) = app.state::<SharedState>().service_panel_closed()
                            {
                                let _ = app.emit("state-changed", snapshot);
                            }
                        }
                    }
                    tauri::WindowEvent::Destroyed => {
                        let app = window.app_handle();
                        if let Ok(snapshot) = app.state::<SharedState>().service_panel_closed() {
                            let _ = app.emit("state-changed", snapshot);
                        }
                    }
                    _ => {}
                }
            }
            if window.label() == "lan-details" && matches!(event, tauri::WindowEvent::Destroyed) {
                let app = window.app_handle();
                if let Ok(snapshot) = app.state::<SharedState>().lan_panel("docked") {
                    let _ = app.emit("state-changed", snapshot);
                }
            }
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
            edit_lan_inventory,
            copy_lan_selection,
            notifications::notification_permission,
            answer_ask,
            check_update,
            download_update,
            clear_update_download_cache,
            cancel_update,
            install_update,
            start_services,
            gpu_panel::gpu_panel_available,
            gpu_panel::open_gpu_panel,
            gpu_panel::float_gpu_panel,
            gpu_panel::dock_gpu_panel,
            gpu_panel::sync_gpu_panel,
            gpu_panel::close_gpu_panel,
            control_firewall,
            start_syslog,
            stop_syslog,
            clear_syslog,
            select_syslog,
            syslog_csv,
            stop_services,
            services_csv,
            diagnostic_action,
            temporary_protection_action,
            set_flow_names,
            collect_flow_names,
            diagnostic_export,
            select_service,
            edit_service_note,
            select_port,
            set_port_layout,
            position_port_details,
            resize_evidence_panel,
            position_service_details,
            start_lan_ports,
            open_lan_ports,
            lan_ports_csv,
            stop_lan_ports,
            set_destination_names,
            clear_destination_names,
            lookup_destination_name,
            app_icons::application_icon,
            get_snapshot,
            select_lan_device,
            lan_csv,
            float_lan_details,
            dock_lan_details,
            start_lan_discovery,
            configure_lan_public,
            get_lan_archive,
            query_lan_public,
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
        .build(app_context())
        .expect("FlowGuard could not start")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                gpu_panel::destroy();
            }
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::RunEvent::Reopen { .. }) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}

#[cfg(all(feature = "gpu-qa", target_os = "macos"))]
pub fn run_gpu_panel_check() -> Result<(), String> {
    gpu_panel::run_check()
}

fn app_context() -> tauri::Context<tauri::Wry> {
    tauri::generate_context!()
}

/// Local release tooling only; no OS installation or network access.
pub fn verify_update_artifact(
    path: &std::path::Path,
    version: &str,
    channel: &str,
) -> Result<(), String> {
    updates::verify_artifact(path, version, channel)
}

pub fn verify_update_artifact_for_arch(
    path: &std::path::Path,
    version: &str,
    channel: &str,
    arch: &str,
) -> Result<(), String> {
    updates::verify_artifact_for_arch(path, version, channel, arch)
}
