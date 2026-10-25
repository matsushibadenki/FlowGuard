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
    owner: String,
    camera: gpu_scene::Camera,
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
    camera: gpu_scene::Camera,
    scene: Option<gpu_scene::Scene>,
    labels: Vec<Retained<objc2_app_kit::NSTextField>>,
}
impl Drop for Panel {
    fn drop(&mut self) {
        self.container.removeFromSuperview();
    }
}
#[cfg(feature = "gpu-qa")]
pub fn open() -> u64 {
    open_for("main")
}
pub fn open_for(owner: &str) -> u64 {
    CONTROLLER.with_borrow_mut(|c| {
        if let Some(p) = c.panel.as_ref() {
            c.camera = p.camera;
        }
        c.panel = None;
        c.owner = owner.into();
        c.next += 1;
        c.lease = Lease {
            id: c.next,
            sequence: 0,
            closed: false,
        };
        c.next
    })
}
#[cfg(feature = "gpu-qa")]
pub fn close(id: u64) {
    close_for(id, "main")
}
pub fn close_for(id: u64, owner: &str) {
    CONTROLLER.with_borrow_mut(|c| {
        if c.lease.id == id && c.owner == owner {
            c.lease.closed = true;
            if let Some(p) = c.panel.as_ref() {
                c.camera = p.camera;
            }
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
#[cfg(feature = "gpu-qa")]
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
            camera: Default::default(),
            scene: None,
            labels: Vec::new(),
        })
    }
    fn show_labels(&mut self, scene: &gpu_scene::Scene, mtm: MainThreadMarker) {
        use objc2_app_kit::{NSColor, NSFont, NSTextField};
        use objc2_foundation::NSString;
        for label in self.labels.drain(..) {
            label.removeFromSuperview();
        }
        let bounds = self.view.bounds();
        let width = bounds.size.width;
        let height = bounds.size.height;
        let mut positions: [Vec<f32>; 2] = [vec![], vec![]];
        for label in &scene.labels {
            let y = label.position[1];
            let side = usize::from(label.right);
            if !(0.02..0.98).contains(&y)
                || positions[side].len() >= 32
                || positions[side]
                    .iter()
                    .any(|v| ((v - y) as f64 * height).abs() < 18.0)
            {
                continue;
            }
            positions[side].push(y);
            let text = NSString::from_str(&label.text.chars().take(160).collect::<String>());
            let field = NSTextField::labelWithString(&text, mtm);
            field.setFont(Some(&NSFont::systemFontOfSize(12.0)));
            field.setTextColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(
                0.12, 0.18, 0.22, 1.0,
            )));
            field.setUsesSingleLineMode(true);
            field.setToolTip(Some(&NSString::from_str(&label.text)));
            field.setFrame(frame(
                label.position[0] as f64 * width,
                (1.0 - y as f64) * height - 8.0,
                width * 0.39,
                18.0,
            ));
            self.view.addSubview(&field);
            self.labels.push(field);
        }
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
#[cfg(feature = "gpu-qa")]
pub fn sync(
    app: &tauri::AppHandle,
    window: tauri::Window,
    id: u64,
    sequence: u64,
    bounds: Option<Bounds>,
    filter: crate::gpu_filter::Filter,
) -> Result<Status, String> {
    sync_action(app, window, id, sequence, bounds, filter, None)
}
pub fn sync_action(
    app: &tauri::AppHandle,
    window: tauri::Window,
    id: u64,
    sequence: u64,
    bounds: Option<Bounds>,
    filter: crate::gpu_filter::Filter,
    interaction: Option<super::Interaction>,
) -> Result<Status, String> {
    let mtm = MainThreadMarker::new().ok_or("main_thread_required")?;
    CONTROLLER.with_borrow_mut(|c| {
        if c.owner != window.label() {
            return Err("stale_gpu_owner".into());
        }
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
            let mut panel = Panel::new(&host, mtm)?;
            panel.camera = c.camera;
            c.panel = Some(panel);
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
                let mut session = shared.inner.lock().map_err(|_| "state_lock")?;
                if session.gpu_owner != window.label() {
                    return Err("stale_gpu_owner".into());
                }
                if window.label() == "main" && session.document.preferences.view != "map" {
                    return Err("map_view_required".into());
                }
                if !filter.matches_source(&session.document.preferences.mode) {
                    return Err("stale_gpu_source".into());
                }
                if let Some(action) = interaction.as_ref() {
                    if let super::Interaction::Pick { x, y } = action {
                        if p.revision == Some(session.revision)
                            && p.filter.as_ref() == Some(&filter)
                        {
                            if let Some(id) = p.scene.as_ref().and_then(|scene| {
                                gpu_scene::pick(
                                    scene,
                                    *x,
                                    *y,
                                    bounds.unwrap().rect.width as f32,
                                    bounds.unwrap().rect.height as f32,
                                )
                            }) {
                                if session.visible().iter().any(|c| c.id == id) {
                                    session.selected_id = Some(id.into());
                                    session.revision += 1;
                                    let _ = tauri::Emitter::emit(
                                        app,
                                        "state-changed",
                                        session.snapshot_for_gpu(),
                                    );
                                }
                            }
                        }
                    } else {
                        p.camera.change(action.kind());
                        p.revision = None;
                    }
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
                    {
                        let names = session.flow_name_snapshot();
                        filter.project_with_names(
                            visible,
                            &session.document.rules,
                            if names.search_enabled {
                                &names.rows
                            } else {
                                &[]
                            },
                        )
                    },
                    session.selected_id.clone(),
                    total,
                )
            };
            let mut scene = gpu_scene::build(&connections, selected.as_deref());
            gpu_scene::camera(&mut scene, p.camera);
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
            p.show_labels(&scene, mtm);
            p.scene = Some(scene);
            p.status = status.clone();
            Ok(status)
        })();
        if result.is_err() {
            if let Some(p) = c.panel.as_ref() {
                c.camera = p.camera;
            }
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

pub fn window_event(owner: &str, destroyed: bool) {
    CONTROLLER.with_borrow_mut(|c| {
        if c.owner == owner {
            if destroyed {
                c.lease.closed = true;
                if let Some(p) = c.panel.as_ref() {
                    c.camera = p.camera;
                }
                c.panel = None;
            } else if let Some(p) = &c.panel {
                p.container.setHidden(true);
            }
        }
    });
}

#[cfg(feature = "gpu-qa")]
pub fn verify_interactions(
    app: &tauri::AppHandle,
    window: tauri::Window,
    lease: u64,
    b: Bounds,
) -> Result<(), String> {
    sync_action(
        app,
        window.clone(),
        lease,
        9,
        Some(b),
        crate::gpu_filter::Filter::default(),
        Some(super::Interaction::ZoomIn),
    )?;
    CONTROLLER.with_borrow(|c| {
        let p = c.panel.as_ref().ok_or("qa_panel")?;
        if p.camera.zoom <= 1.0 || p.labels.is_empty() {
            return Err("qa_labels_or_zoom".into());
        }
        Ok::<_, String>(())
    })?;
    sync_action(
        app,
        window.clone(),
        lease,
        10,
        Some(b),
        crate::gpu_filter::Filter::default(),
        Some(super::Interaction::Reset),
    )?;
    let (x, y, expected) = CONTROLLER.with_borrow(|c| {
        let p = c.panel.as_ref().ok_or("qa_panel")?;
        let scene = p.scene.as_ref().ok_or("qa_scene")?;
        let route = scene.routes.last().ok_or("qa_route")?;
        let x = 0.5;
        let y = (route.from[1] + route.to[1]) * 0.5;
        let expected = gpu_scene::pick(scene, x, y, b.rect.width as f32, b.rect.height as f32)
            .ok_or("qa_pick")?
            .to_owned();
        Ok::<_, String>((x, y, expected))
    })?;
    sync_action(
        app,
        window,
        lease,
        11,
        Some(b),
        crate::gpu_filter::Filter::default(),
        Some(super::Interaction::Pick { x, y }),
    )?;
    if app
        .state::<SharedState>()
        .snapshot()?
        .selected_id
        .as_deref()
        != Some(&expected)
    {
        return Err("qa_selection_not_shared".into());
    }
    Ok(())
}
