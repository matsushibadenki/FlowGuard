#[cfg(feature = "gpu")]
fn main() {
    let renderer = pollster::block_on(flowguard_lib::gpu::NativeRenderer::new(800, 600))
        .expect("Metal adapter");
    renderer
        .render(&[flowguard_lib::gpu::Route {
            from: [0.1, 0.2],
            to: [0.9, 0.8],
            color: [0.1, 0.4, 0.3, 1.0],
        }])
        .expect("render");
    println!("Metal pipeline initialized; route submitted without frame readback.");
}
#[cfg(not(feature = "gpu"))]
fn main() {
    eprintln!("Run with --features gpu");
}
