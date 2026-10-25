use super::{Bounds, Lease, Status};
use crate::{gpu::NativeRenderer, gpu_scene, state::SharedState};
use objc2::{define_class, msg_send, rc::Retained, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSView, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::CAMetalLayer;
use std::cell::RefCell;
use tauri::Manager;
// The drawable must not intercept WebView wheel, pointer or keyboard interactions.
define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    struct PassthroughView;
    impl PassthroughView {
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView { std::ptr::null_mut() }
    }
);
impl PassthroughView {
    fn create(mtm: MainThreadMarker) -> Retained<NSView> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: NSView's designated initializer and argument/return types are correct.
        let this: Retained<Self> =
            unsafe { msg_send![super(this),initWithFrame:frame(0.0,0.0,1.0,1.0)] };
        this.into_super()
    }
}
thread_local! { static CONTROLLER:RefCell<Controller>=RefCell::new(Controller::default()); }
#[derive(Default)]
struct Controller {
    next: u64,
    lease: Lease,
    panel: Option<Panel>,
}
// Main-thread-only native resources. Field order keeps surface alive no longer than its layer.
struct Panel {
    surface: wgpu::Surface<'static>,
    renderer: NativeRenderer,
    _layer: Retained<CAMetalLayer>,
    view: Retained<NSView>,
    container: Retained<NSView>,
    size: (u32, u32),
    revision: Option<u64>,
    filter: Option<crate::gpu_filter::Filter>,
    status: Status,
}
impl Drop for Panel {
    fn drop(&mut self) {
        self.container.removeFromSuperview();
    }
}
pub fn open() -> u64 {
    CONTROLLER.with_borrow_mut(|c| {
        c.panel = None;
        c.next += 1;
        c.lease = Lease {
            id: c.next,
            sequence: 0,
            closed: false,
        };
        c.next
    })
}
pub fn close(id: u64) {
    CONTROLLER.with_borrow_mut(|c| {
        if c.lease.id == id {
            c.lease.closed = true;
            c.panel = None;
        }
    });
}
pub fn destroy() {
    CONTROLLER.with_borrow_mut(|c| {
        c.lease.closed = true;
        c.panel = None;
    });
}
pub fn suspend() {
    CONTROLLER.with_borrow(|c| {
        if let Some(p) = &c.panel {
            p.container.setHidden(true);
        }
    });
}
fn frame(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
impl Panel {
    fn new(host: &NSView, mtm: MainThreadMarker) -> Result<Self, String> {
        let container = PassthroughView::create(mtm);
        container.setWantsLayer(true);
        if let Some(layer) = container.layer() {
            layer.setMasksToBounds(true);
        }
        let view = NSView::initWithFrame(NSView::alloc(mtm), frame(0.0, 0.0, 1.0, 1.0));
        let layer = CAMetalLayer::new();
        view.setWantsLayer(true);
        view.setLayer(Some(&layer));
        container.addSubview(&view);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        });
        // SAFETY: this retained layer outlives the surface, all use/destruction is on AppKit's
        // main thread, and the controller removes the view before releasing its native resources.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                Retained::as_ptr(&layer).cast_mut().cast(),
            ))
        }
        .map_err(|_| "surface_creation_failed")?;
        let renderer = pollster::block_on(NativeRenderer::new_for_surface(
            &instance,
            Some(&surface),
            1,
            1,
        ))?;
        host.addSubview(&container);
        container.setHidden(true);
        Ok(Self {
            surface,
            renderer,
            _layer: layer,
            view,
            container,
            size: (1, 1),
            revision: None,
            filter: None,
            status: Status::default(),
        })
    }
    fn place(&mut self, host: &NSView, b: Bounds, scale: f64) -> Result<(), String> {
        let host_bounds = host.bounds();
        let sx = host_bounds.size.width / b.viewport_width;
        let sy = host_bounds.size.height / b.viewport_height;
        if !sx.is_finite()
            || !sy.is_finite()
            || sx <= 0.0
            || sy <= 0.0
            || !scale.is_finite()
            || scale <= 0.0
        {
            return Err("invalid_native_bounds".into());
        }
        let r = b.rect;
        let clip = b.clip;
        let cy = if host.isFlipped() {
            clip.y * sy
        } else {
            host_bounds.size.height - (clip.y + clip.height) * sy
        };
        self.container
            .setFrame(frame(clip.x * sx, cy, clip.width * sx, clip.height * sy));
        // NSView is unflipped. Keep the full drawable in a clipped container so scrolling crops,
        // rather than squeezing or re-layouting, the map.
        self.view.setFrame(frame(
            (r.x - clip.x) * sx,
            (clip.y + clip.height - r.y - r.height) * sy,
            r.width * sx,
            r.height * sy,
        ));
        self._layer.setContentsScale(scale);
        let size = (
            ((r.width * sx * scale).round() as u32).clamp(1, 4096),
            ((r.height * sy * scale).round() as u32).clamp(1, 4096),
        );
        if size != self.size {
            self.revision = None;
            self.renderer.resize(size.0, size.1);
            self.size = size;
            self.renderer.configure_surface(&self.surface);
        }
        Ok(())
    }
}
pub fn sync(
    app: &tauri::AppHandle,
    window: tauri::Window,
    id: u64,
    sequence: u64,
    bounds: Option<Bounds>,
    filter: crate::gpu_filter::Filter,
) -> Result<Status, String> {
    let mtm = MainThreadMarker::new().ok_or("main_thread_required")?;
    CONTROLLER.with_borrow_mut(|c| {
        c.lease.accept(id, sequence)?;
        if bounds.is_none()
            || !window.is_visible().unwrap_or(false)
            || window.is_minimized().unwrap_or(true)
        {
            if let Some(p) = &c.panel {
                p.container.setHidden(true);
            }
            return Ok(Status {
                lease: id,
                ..Default::default()
            });
        }
        // SAFETY: Window owns this NSWindow and this entire closure runs on the main thread.
        let native = unsafe {
            &*window
                .ns_window()
                .map_err(|_| "native_window_unavailable")?
                .cast::<NSWindow>()
        };
        let host = native.contentView().ok_or("native_view_unavailable")?;
        if c.panel.is_none() {
            c.panel = Some(Panel::new(&host, mtm)?);
        }
        let result = (|| {
            let p = c.panel.as_mut().ok_or("surface_unavailable")?;
            p.place(
                &host,
                bounds.unwrap(),
                window.scale_factor().map_err(|_| "scale_unavailable")?,
            )?;
            let (revision, connections, selected, total) = {
                let shared = app.state::<SharedState>();
                let session = shared.inner.lock().map_err(|_| "state_lock")?;
                if session.document.preferences.view != "map" {
                    return Err("map_view_required".into());
                }
                if !filter.matches_source(&session.document.preferences.mode) {
                    return Err("stale_gpu_source".into());
                }
                if p.revision == Some(session.revision)
                    && p.filter.as_ref() == Some(&filter)
                    && p.status.presented
                {
                    p.container.setHidden(false);
                    return Ok(p.status.clone());
                }
                // Copy only the renderer's inputs, never history, diagnostics or log bodies.
                let visible = session.visible();
                let total = visible.len();
                (
                    session.revision,
                    filter.project(visible, &session.document.rules),
                    session.selected_id.clone(),
                    total,
                )
            };
            let scene = gpu_scene::build(&connections, selected.as_deref());
            let presented = p.renderer.present(&p.surface, &scene.routes)?;
            p.container.setHidden(!presented);
            let status = Status {
                lease: id,
                presented,
                routes: scene.routes.len(),
                total,
                matched: scene.total,
                truncated: scene.total > scene.routes.len(),
            };
            p.revision = presented.then_some(revision);
            p.filter = Some(filter);
            p.status = status.clone();
            Ok(status)
        })();
        if result.is_err() {
            c.panel = None;
        }
        result
    })
}
#[cfg(feature = "gpu-qa")]
pub(super) fn wait_for_completion() -> Result<(), String> {
    CONTROLLER.with_borrow(|c| {
        let p = c.panel.as_ref().ok_or("surface_unavailable")?;
        assert!(p.container.hitTest(NSPoint::new(1.0, 1.0)).is_none());
        p.renderer.wait_for_completion()
    })
}
#[cfg(feature = "gpu-qa")]
pub(super) fn verify_clipped_geometry() -> Result<(), String> {
    CONTROLLER.with_borrow(|c| {
        let p = c.panel.as_ref().ok_or("surface_unavailable")?;
        if p.container.frame().size.height >= p.view.frame().size.height
            || !p.container.layer().is_some_and(|l| l.masksToBounds())
        {
            return Err("qa_clip_geometry_failed".into());
        }
        Ok(())
    })
}
