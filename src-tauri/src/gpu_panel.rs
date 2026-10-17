//! Geometry/state-only IPC for a single main-window native map panel.
use serde::{Deserialize, Serialize};
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
    pub truncated: bool,
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
    if window.label() != "main" {
        return Err("main_window_required".into());
    }
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(platform::open());
        })
        .map_err(|_| "main_thread_unavailable")?;
        rx.await.map_err(|_| "main_thread_unavailable".into())
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
) -> Result<Status, String> {
    if window.label() != "main" {
        return Err("main_window_required".into());
    }
    if bounds.is_some_and(|b| !b.valid()) {
        return Err("invalid_gpu_bounds".into());
    }
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(platform::sync(&handle, window, lease, sequence, bounds));
        })
        .map_err(|_| "main_thread_unavailable")?;
        rx.await.map_err(|_| "main_thread_unavailable")?
    }
    #[cfg(not(all(feature = "gpu", target_os = "macos")))]
    {
        let _ = (app, lease, sequence);
        Err("gpu_unavailable".into())
    }
}
#[tauri::command]
pub async fn close_gpu_panel(
    window: tauri::Window,
    app: tauri::AppHandle,
    lease: u64,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("main_window_required".into());
    }
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            platform::close(lease);
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
pub fn suspend() {
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    platform::suspend();
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
