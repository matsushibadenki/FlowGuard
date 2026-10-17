use crate::model::{Connection, NodeLayout};
// Rust owns the bounded scene description. WebViews receive identifiers and positions,
// never packet contents, image buffers, or frame data. This small map is UI chrome;
// the dense-map panel separately reads shared connections into a bounded Rust/wgpu scene.
pub const MAX_VISIBLE_NODES: usize = 6;
pub fn layout(connections: &[Connection]) -> Vec<NodeLayout> {
    connections
        .iter()
        .take(MAX_VISIBLE_NODES)
        .enumerate()
        .map(|(i, c)| NodeLayout {
            id: c.id.clone(),
            y: 72.0 + i as f32 * 100.0,
        })
        .collect()
}
