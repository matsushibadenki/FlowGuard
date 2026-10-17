#[cfg(all(feature = "gpu-qa", target_os = "macos"))]
fn main() {
    flowguard_lib::run_gpu_panel_check().expect("native panel check");
}
#[cfg(not(all(feature = "gpu-qa", target_os = "macos")))]
fn main() {
    eprintln!("Run on macOS with --features gpu-qa");
    std::process::exit(2);
}
