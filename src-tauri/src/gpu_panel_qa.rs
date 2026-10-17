//! Opt-in local integration harness; no observer, network, real document, extension or updater.
use super::{platform, Bounds, Rect};
use crate::{model::Connection, state::SharedState};
use tauri::Manager;
pub fn run() -> Result<(), String> {
    let path = std::env::temp_dir().join(format!("flowguard-gpu-qa-{}", std::process::id()));
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let state = SharedState::load(path.join("document.json"))?;
    {
        let mut s = state.inner.lock().map_err(|_| "state_lock")?;
        s.document.preferences.mode = "demo".into();
        s.document.preferences.view = "map".into();
        s.demo = (0..10_001)
            .map(|i| Connection {
                identity: None,
                id: format!("qa-{i}"),
                app: format!("App {}", i % 100),
                app_key: format!("/qa/app{}", i % 100),
                pid: 0,
                local: "127.0.0.1".into(),
                destination: format!("192.0.2.{}", i % 255),
                port: 443,
                protocol: "TCP".into(),
                direction: "unknown".into(),
                observed_at: 0,
            })
            .collect();
        s.selected_id = Some("qa-1".into());
    }
    let mut context = crate::app_context();
    context.config_mut().app.windows.clear();
    let app = tauri::Builder::default()
        .setup(move |app| {
            app.manage(state);
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External("about:blank".parse().unwrap()),
            )
            .title("FlowGuard GPU surface QA")
            .inner_size(1000.0, 720.0)
            .build()?;
            window.set_focus()?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let (tx, rx) = std::sync::mpsc::channel();
                let mut lease = 0;
                for step in 0..8 {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    let handle2 = handle.clone();
                    let tx = tx.clone();
                    handle
                        .run_on_main_thread(move || {
                            let result = (|| -> Result<(u64, String), String> {
                                let webview_window =
                                    handle2.get_webview_window("main").ok_or("qa_window")?;
                                let webview: &tauri::Webview = webview_window.as_ref();
                                let window = webview.window();
                                let r = Rect {
                                    x: 40.0,
                                    y: 80.0,
                                    width: 900.0,
                                    height: 500.0,
                                };
                                let mut b = Bounds {
                                    rect: r,
                                    clip: r,
                                    viewport_width: 1000.0,
                                    viewport_height: 720.0,
                                };
                                let message = match step {
                                    0 => {
                                        lease = platform::open();
                                        println!(
                                            "INFO backing_scale={}",
                                            window.scale_factor().map_err(|_| "scale")?
                                        );
                                        let s =
                                            platform::sync(&handle2, window, lease, 1, Some(b))?;
                                        if !s.presented || s.routes != 10000 || !s.truncated {
                                            return Err("qa_present_failed".into());
                                        }
                                        platform::wait_for_completion()?;
                                        "presented_10000_of_10001"
                                    }
                                    1 => {
                                        b.rect.y = -100.0;
                                        b.clip = Rect {
                                            x: 40.0,
                                            y: 0.0,
                                            width: 900.0,
                                            height: 400.0,
                                        };
                                        platform::sync(&handle2, window, lease, 2, Some(b))?;
                                        platform::verify_clipped_geometry()?;
                                        "clipped_without_squeezing"
                                    }
                                    2 => {
                                        window
                                            .set_size(tauri::LogicalSize::new(800.0, 600.0))
                                            .map_err(|_| "resize")?;
                                        platform::suspend();
                                        "resize_suspended"
                                    }
                                    3 => {
                                        b.rect.width = 700.0;
                                        b.clip = b.rect;
                                        b.viewport_width = 800.0;
                                        b.viewport_height = 600.0;
                                        b.rect.height = 440.0;
                                        b.clip.height = 440.0;
                                        let s =
                                            platform::sync(&handle2, window, lease, 3, Some(b))?;
                                        if !s.presented {
                                            return Err("qa_resize_failed".into());
                                        }
                                        platform::wait_for_completion()?;
                                        "resized_and_presented"
                                    }
                                    4 => {
                                        let s = platform::sync(&handle2, window, lease, 4, None)?;
                                        if s.presented {
                                            return Err("qa_hidden_presented".into());
                                        }
                                        "hidden_without_rendering"
                                    }
                                    5 => {
                                        platform::close(lease);
                                        if platform::sync(&handle2, window, lease, 5, Some(b))
                                            .is_ok()
                                        {
                                            return Err("qa_late_restored".into());
                                        }
                                        "closed_lease_rejects_late_sync"
                                    }
                                    6 => {
                                        let old = lease;
                                        lease = platform::open();
                                        platform::close(old);
                                        let s =
                                            platform::sync(&handle2, window, lease, 1, Some(b))?;
                                        if !s.presented {
                                            return Err("qa_reopen_failed".into());
                                        }
                                        platform::wait_for_completion()?;
                                        "reopened_with_new_lease"
                                    }
                                    _ => {
                                        platform::destroy();
                                        if platform::sync(&handle2, window, lease, 2, Some(b))
                                            .is_ok()
                                        {
                                            return Err("qa_destroyed_restored".into());
                                        }
                                        "resources_released_and_destroyed_lease_rejected"
                                    }
                                };
                                Ok((lease, message.into()))
                            })();
                            let _ = tx.send(result);
                        })
                        .expect("main thread");
                    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                        Ok(Ok((id, message))) => {
                            lease = id;
                            println!("PASS {message}");
                        }
                        error => {
                            eprintln!("FAIL {error:?}");
                            handle.exit(1);
                            return;
                        }
                    }
                }
                handle.exit(0);
            });
            Ok(())
        })
        .build(context)
        .map_err(|e| e.to_string())?;
    let code = app.run_return(|_, _| {});
    let _ = std::fs::remove_dir_all(path);
    if code == 0 {
        Ok(())
    } else {
        Err("native_surface_check_failed".into())
    }
}
