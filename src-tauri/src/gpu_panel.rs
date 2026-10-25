//! Geometry/state-only IPC for one Rust-owned docked or floating native map panel.
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.x.abs() <= 16384.0
            && self.y.abs() <= 16384.0
            && (1.0..=4096.0).contains(&self.width)
            && (1.0..=4096.0).contains(&self.height)
    }
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bounds {
    pub rect: Rect,
    pub clip: Rect,
    pub viewport_width: f64,
    pub viewport_height: f64,
}
impl Bounds {
    pub fn valid(self) -> bool {
        let r = self.rect;
        let c = self.clip;
        r.valid()
            && c.valid()
            && self.viewport_width.is_finite()
            && self.viewport_height.is_finite()
            && (1.0..=16384.0).contains(&self.viewport_width)
            && (1.0..=16384.0).contains(&self.viewport_height)
            && c.x >= 0.0
            && c.y >= 0.0
            && c.x >= r.x
            && c.y >= r.y
            && c.x + c.width <= (r.x + r.width).min(self.viewport_width) + 0.01
            && c.y + c.height <= (r.y + r.height).min(self.viewport_height) + 0.01
    }
}
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub lease: u64,
    pub presented: bool,
    pub routes: usize,
    pub total: usize,
    pub matched: usize,
    pub truncated: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Interaction {
    ZoomIn,
    ZoomOut,
    PanUp,
    PanDown,
    Reset,
    Pick { x: f32, y: f32 },
}
impl Interaction {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::PanUp => "pan_up",
            Self::PanDown => "pan_down",
            Self::Reset => "reset",
            Self::Pick { .. } => "pick",
        }
    }
}
pub struct Lease {
    pub id: u64,
    pub sequence: u64,
    pub closed: bool,
}
impl Default for Lease {
    fn default() -> Self {
        Self {
            id: 0,
            sequence: 0,
            closed: true,
        }
    }
}
impl Lease {
    fn accept(&mut self, id: u64, sequence: u64) -> Result<(), String> {
        if self.closed || id != self.id || sequence <= self.sequence {
            return Err("stale_gpu_panel".into());
        }
        self.sequence = sequence;
        Ok(())
    }
}
#[cfg(all(feature = "gpu", target_os = "macos"))]
#[path = "gpu_panel_macos.rs"]
mod platform;

#[tauri::command]
pub fn gpu_panel_available() -> bool {
    cfg!(all(feature = "gpu", target_os = "macos"))
}
#[tauri::command]
pub async fn open_gpu_panel(window: tauri::Window, app: tauri::AppHandle) -> Result<u64, String> {
    if !["main", "gpu-map"].contains(&window.label()) {
        return Err("main_window_required".into());
    }
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let allowed = handle
                .state::<crate::state::SharedState>()
                .inner
                .lock()
                .map(|s| s.gpu_owner == window.label())
                .unwrap_or(false);
            let _ = tx.send(if allowed {
                Ok(platform::open_for(window.label()))
            } else {
                Err("stale_gpu_owner".to_owned())
            });
        })
        .map_err(|_| "main_thread_unavailable")?;
        rx.await.map_err(|_| "main_thread_unavailable".to_owned())?
    }
    #[cfg(not(all(feature = "gpu", target_os = "macos")))]
    {
        let _ = app;
        Err("gpu_unavailable".into())
    }
}
#[tauri::command]
pub async fn sync_gpu_panel(
    window: tauri::Window,
    app: tauri::AppHandle,
    lease: u64,
    sequence: u64,
    bounds: Option<Bounds>,
    filter: crate::gpu_filter::Filter,
    interaction: Option<Interaction>,
) -> Result<Status, String> {
    if !["main", "gpu-map"].contains(&window.label()) {
        return Err("main_window_required".into());
    }
    if let Some(Interaction::Pick { x, y }) = interaction.as_ref() {
        if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(x) || !(0.0..=1.0).contains(y)
        {
            return Err("invalid_gpu_pick".into());
        }
    }
    if !filter.valid() {
        return Err("invalid_gpu_filter".into());
    }
    if bounds.is_some_and(|b| !b.valid()) {
        return Err("invalid_gpu_bounds".into());
    }
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(platform::sync_action(
                &handle,
                window,
                lease,
                sequence,
                bounds,
                filter,
                interaction,
            ));
        })
        .map_err(|_| "main_thread_unavailable")?;
        rx.await.map_err(|_| "main_thread_unavailable")?
    }
    #[cfg(not(all(feature = "gpu", target_os = "macos")))]
    {
        let _ = (app, lease, sequence, filter, interaction);
        Err("gpu_unavailable".into())
    }
}
#[tauri::command]
pub async fn close_gpu_panel(
    window: tauri::Window,
    app: tauri::AppHandle,
    lease: u64,
) -> Result<(), String> {
    if !["main", "gpu-map"].contains(&window.label()) {
        return Err("main_window_required".into());
    }
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            platform::close_for(lease, window.label());
            let _ = tx.send(());
        })
        .map_err(|_| "main_thread_unavailable")?;
        rx.await.map_err(|_| "main_thread_unavailable".into())
    }
    #[cfg(not(all(feature = "gpu", target_os = "macos")))]
    {
        let _ = (app, lease);
        Ok(())
    }
}
pub fn destroy() {
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    platform::destroy();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_geometry_never_reaches_native_view() {
        let r = Rect {
            x: 16.0,
            y: 100.0,
            width: 800.0,
            height: 320.0,
        };
        let valid = Bounds {
            rect: r,
            clip: r,
            viewport_width: 1200.0,
            viewport_height: 900.0,
        };
        assert!(valid.valid());
        for b in [
            Bounds {
                clip: Rect { x: -1.0, ..r },
                ..valid
            },
            Bounds {
                viewport_width: f64::NAN,
                ..valid
            },
            Bounds {
                rect: Rect { width: 0.0, ..r },
                ..valid
            },
            Bounds {
                rect: Rect {
                    height: 4097.0,
                    ..r
                },
                ..valid
            },
        ] {
            assert!(!b.valid());
        }
        assert!(Bounds {
            rect: Rect { y: -100.0, ..r },
            clip: Rect {
                y: 0.0,
                height: 220.0,
                ..r
            },
            ..valid
        }
        .valid());
    }
    #[test]
    fn closed_replaced_and_out_of_order_leases_cannot_restore_surface() {
        assert!(Lease::default().accept(0, 1).is_err());
        let mut l = Lease {
            id: 7,
            closed: false,
            ..Default::default()
        };
        assert!(l.accept(7, 2).is_ok());
        assert!(l.accept(7, 1).is_err());
        assert!(l.accept(6, 3).is_err());
        l.closed = true;
        assert!(l.accept(7, 3).is_err());
    }
}
#[cfg(all(feature = "gpu-qa", target_os = "macos"))]
#[path = "gpu_panel_qa.rs"]
mod qa;
#[cfg(all(feature = "gpu-qa", target_os = "macos"))]
pub fn run_check() -> Result<(), String> {
    qa::run()
}

pub fn window_event(owner: &str, destroyed: bool) {
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    platform::window_event(owner, destroyed);
    #[cfg(not(all(feature = "gpu", target_os = "macos")))]
    let _ = (owner, destroyed);
}
#[tauri::command]
pub fn float_gpu_panel(
    app: tauri::AppHandle,
    window: tauri::Window,
    filter: crate::gpu_filter::Filter,
) -> Result<(), String> {
    if window.label() != "main" || !filter.valid() {
        return Err("invalid_gpu_filter".into());
    }
    let shared = app.state::<crate::state::SharedState>();
    let operation = {
        let mut s = shared.inner.lock().map_err(|_| "state_lock")?;
        if !filter.matches_source(&s.document.preferences.mode) {
            return Err("stale_gpu_source".into());
        }
        s.gpu_filter = filter;
        s.gpu_owner = "gpu-map".into();
        s.gpu_float_operation += 1;
        s.revision += 1;
        s.gpu_float_operation
    };
    let _ = app.emit("state-changed", shared.snapshot()?);
    if let Some(w) = app.get_webview_window("gpu-map") {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    let result = tauri::WebviewWindowBuilder::new(
        &app,
        "gpu-map",
        tauri::WebviewUrl::App("index.html?panel=gpu".into()),
    )
    .title("FlowGuard · GPU map")
    .inner_size(1000.0, 720.0)
    .min_inner_size(360.0, 400.0)
    .build();
    if let Err(error) = result {
        if app.get_webview_window("gpu-map").is_none() {
            let mut s = shared.inner.lock().map_err(|_| "state_lock")?;
            if s.gpu_float_operation == operation && s.gpu_owner == "gpu-map" {
                s.gpu_owner = "main".into();
                s.revision += 1;
            }
            drop(s);
            let _ = app.emit("state-changed", shared.snapshot()?);
            return Err(error.to_string());
        }
    }
    Ok(())
}
#[tauri::command]
pub fn dock_gpu_panel(app: tauri::AppHandle, window: tauri::Window) -> Result<(), String> {
    if window.label() != "gpu-map" {
        return Err("gpu_window_required".into());
    }
    let shared = app.state::<crate::state::SharedState>();
    let _ = shared.dispatch(crate::model::Command::SetView { view: "map".into() })?;
    {
        let mut s = shared.inner.lock().map_err(|_| "state_lock")?;
        s.gpu_owner = "main".into();
        s.gpu_dock_operation += 1;
        s.revision += 1;
    }
    let _ = app.emit("state-changed", shared.snapshot()?);
    if let Some(main) = app.get_webview_window("main") {
        main.unminimize().map_err(|e| e.to_string())?;
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
    }
    window.close().map_err(|e| e.to_string())
}
