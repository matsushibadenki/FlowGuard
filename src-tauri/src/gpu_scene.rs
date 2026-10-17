//! A deterministic, bounded overview of Rust-owned connections. Lines indicate observation only.
use crate::{gpu::Route, model::Connection};
use std::collections::BTreeMap;
pub const MAX_ROUTES: usize = 10_000;
pub struct Scene {
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
        routes,
        total: connections.len(),
    }
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
