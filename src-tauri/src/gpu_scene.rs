//! A deterministic, bounded overview of Rust-owned connections. Lines indicate observation only.
use crate::{gpu::Route, model::Connection};
use std::collections::BTreeMap;
pub const MAX_ROUTES: usize = 10_000;
#[derive(Clone)]
pub struct Label {
    pub position: [f32; 2],
    pub text: String,
    pub right: bool,
}
pub struct Scene {
    pub ids: Vec<String>,
    pub labels: Vec<Label>,
    pub routes: Vec<Route>,
    pub total: usize,
}
pub fn build(connections: &[Connection], selected: Option<&str>) -> Scene {
    let mut ordered: Vec<_> = connections.iter().collect();
    ordered.sort_unstable_by(|a, b| {
        (&a.app_key, &a.destination, a.port, &a.id).cmp(&(
            &b.app_key,
            &b.destination,
            b.port,
            &b.id,
        ))
    });
    ordered.truncate(MAX_ROUTES);
    if let Some(selected) = selected.and_then(|id| connections.iter().find(|c| c.id == id)) {
        if !ordered.iter().any(|c| c.id == selected.id) {
            if ordered.len() == MAX_ROUTES {
                ordered.pop();
            }
            ordered.push(selected);
        }
    }
    let mut apps = BTreeMap::new();
    let mut destinations = BTreeMap::new();
    for c in &ordered {
        apps.insert(&c.app_key, 0usize);
        destinations.insert((&c.destination, c.port), 0usize);
    }
    for (i, n) in apps.values_mut().enumerate() {
        *n = i;
    }
    for (i, n) in destinations.values_mut().enumerate() {
        *n = i;
    }
    let y = |i: usize, total: usize| 0.05 + 0.9 * (i as f32 + 0.5) / total.max(1) as f32;
    let mut labels = Vec::new();
    for (key, index) in &apps {
        if let Some(c) = ordered.iter().find(|c| &c.app_key == *key) {
            labels.push(Label {
                position: [0.04, y(*index, apps.len())],
                text: c.app.clone(),
                right: false,
            });
        }
    }
    for ((address, port), index) in &destinations {
        labels.push(Label {
            position: [0.56, y(*index, destinations.len())],
            text: format!("{address}:{port}"),
            right: true,
        });
    }
    let mut pairs: Vec<_> = ordered
        .iter()
        .map(|c| (c.id.clone(), Some(c.id.as_str()) == selected))
        .collect();
    pairs.sort_by_key(|(_, selected)| *selected);
    let mut routes: Vec<_> = ordered
        .iter()
        .map(|c| Route {
            from: [0.04, y(apps[&c.app_key], apps.len())],
            to: [
                0.96,
                y(destinations[&(&c.destination, c.port)], destinations.len()),
            ],
            color: if Some(c.id.as_str()) == selected {
                [0.0, 0.42, 0.58, 1.0]
            } else {
                [0.36, 0.44, 0.50, 0.28]
            },
        })
        .collect();
    // Selected line last, so dense bundles cannot paint over it.
    routes.sort_by_key(|r| if r.color[3] >= 1.0 { 1 } else { 0 });
    Scene {
        ids: pairs.into_iter().map(|(id, _)| id).collect(),
        labels,
        routes,
        total: connections.len(),
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub zoom: f32,
    pub pan: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: 0.0,
        }
    }
}
impl Camera {
    pub fn change(&mut self, action: &str) {
        match action {
            "zoom_in" => self.zoom = (self.zoom * 1.5).min(8.0),
            "zoom_out" => self.zoom = (self.zoom / 1.5).max(1.0),
            "pan_up" => self.pan += 0.15,
            "pan_down" => self.pan -= 0.15,
            "reset" => *self = Self::default(),
            _ => {}
        }
        self.pan = self
            .pan
            .clamp(-(self.zoom - 1.0) / 2.0, (self.zoom - 1.0) / 2.0);
    }
}
pub fn camera(scene: &mut Scene, c: Camera) {
    let y = |v: f32| (v - 0.5) * c.zoom + 0.5 + c.pan;
    for r in &mut scene.routes {
        r.from[1] = y(r.from[1]);
        r.to[1] = y(r.to[1]);
    }
    for label in &mut scene.labels {
        label.position[1] = y(label.position[1]);
    }
}
pub fn pick(scene: &Scene, x: f32, y: f32, width: f32, height: f32) -> Option<&str> {
    if ![x, y, width, height].iter().all(|v| v.is_finite())
        || !(0.0..=1.0).contains(&x)
        || !(0.0..=1.0).contains(&y)
        || width < 1.0
        || height < 1.0
    {
        return None;
    }
    let mut best = 100.0;
    let mut selected = None;
    for (r, id) in scene.routes.iter().zip(&scene.ids).rev() {
        let center = (r.from[0] + r.to[0]) * 0.5;
        let point = |t: f32| {
            let u = 1.0 - t;
            [
                (u * u * u * r.from[0]
                    + 3.0 * u * u * t * center
                    + 3.0 * u * t * t * center
                    + t * t * t * r.to[0])
                    * width,
                ((u * u * u + 3.0 * u * u * t) * r.from[1]
                    + (3.0 * u * t * t + t * t * t) * r.to[1])
                    * height,
            ]
        };
        let q = [x * width, y * height];
        for i in 0..32 {
            let a = point(i as f32 / 32.0);
            let b = point((i + 1) as f32 / 32.0);
            let d = [b[0] - a[0], b[1] - a[1]];
            let len = d[0] * d[0] + d[1] * d[1];
            let t = if len > 0.0 {
                ((q[0] - a[0]) * d[0] + (q[1] - a[1]) * d[1]) / len
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let distance = (q[0] - a[0] - t * d[0]).powi(2) + (q[1] - a[1] - t * d[1]).powi(2);
            if distance < best {
                best = distance;
                selected = Some(id.as_str());
            }
        }
    }
    selected
}
#[cfg(test)]
mod tests {
    use super::*;
    fn connection(i: usize) -> Connection {
        Connection {
            identity: None,
            id: format!("{i:05}"),
            app: "Test".into(),
            app_key: format!("app{}", i % 3),
            pid: 0,
            local: "127.0.0.1".into(),
            destination: format!("192.0.2.{}", i % 255),
            port: 443,
            protocol: "TCP".into(),
            direction: "unknown".into(),
            observed_at: 0,
        }
    }
    #[test]
    fn labels_zoom_and_hit_test_remain_bounded_and_select_rendered_ids() {
        let c = connection(1);
        let mut scene = build(&[c.clone()], Some(&c.id));
        assert_eq!(scene.ids, vec![c.id.clone()]);
        assert_eq!(scene.labels.len(), 2);
        assert_eq!(pick(&scene, 0.5, 0.5, 1000.0, 500.0), Some(c.id.as_str()));
        assert!(pick(&scene, f32::NAN, 0.0, 1.0, 1.0).is_none());
        let mut view = Camera::default();
        for _ in 0..100 {
            view.change("zoom_in");
            view.change("pan_up");
        }
        assert_eq!(view.zoom, 8.0);
        assert!(view.pan <= 3.5);
        camera(&mut scene, view);
        assert!(scene.routes.iter().all(|r| r
            .from
            .iter()
            .chain(r.to.iter())
            .all(|v| v.is_finite())));
        view.change("reset");
        assert_eq!(view.zoom, 1.0);
        assert_eq!(view.pan, 0.0);
    }
    #[test]
    fn selection_beyond_the_cap_remains_visible() {
        let mut connections: Vec<_> = (0..10_001).map(connection).collect();
        for c in &mut connections {
            c.app_key = "same".into();
            c.destination = "192.0.2.1".into();
        }
        let scene = build(&connections, Some("10000"));
        assert_eq!(scene.routes.len(), MAX_ROUTES);
        assert_eq!(scene.routes.last().unwrap().color[3], 1.0);
    }
    #[test]
    fn bounded_deterministic_and_selected_drawn_last() {
        let mut c: Vec<_> = (0..10_001).map(connection).collect();
        let a = build(&c, Some("00000"));
        c.reverse();
        let b = build(&c, Some("00000"));
        assert_eq!(a.total, 10_001);
        assert_eq!(a.routes.len(), 10_000);
        assert_eq!(a.routes.last().unwrap().color[3], 1.0);
        for (a, b) in a.routes.iter().zip(b.routes) {
            assert_eq!(a.from, b.from);
            assert_eq!(a.to, b.to);
            assert!(a
                .from
                .iter()
                .chain(a.to.iter())
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0));
        }
        assert!(build(&[], None).routes.is_empty());
    }
}
