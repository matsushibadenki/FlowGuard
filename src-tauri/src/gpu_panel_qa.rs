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
                for step in 0..10 {
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
                                        let s = platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            1,
                                            Some(b),
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )?;
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
                                        platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            2,
                                            Some(b),
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )?;
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
                                        let s = platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            3,
                                            Some(b),
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )?;
                                        if !s.presented {
                                            return Err("qa_resize_failed".into());
                                        }
                                        platform::wait_for_completion()?;
                                        "resized_and_presented"
                                    }
                                    4 => {
                                        let s = platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            4,
                                            None,
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )?;
                                        if s.presented {
                                            return Err("qa_hidden_presented".into());
                                        }
                                        "hidden_without_rendering"
                                    }
                                    5 => {
                                        platform::close(lease);
                                        if platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            5,
                                            Some(b),
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )
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
                                        let s = platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            1,
                                            Some(b),
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )?;
                                        if !s.presented {
                                            return Err("qa_reopen_failed".into());
                                        }
                                        platform::wait_for_completion()?;
                                        "reopened_with_new_lease"
                                    }
                                    7 => {
                                        use crate::gpu_filter::{Filter,Intent,Source};
                                        let s=platform::sync(&handle2,window.clone(),lease,2,Some(b),Filter{search:"no matching app".into(),hide_loopback:false,..Default::default()})?;
                                        if !s.presented||s.routes!=0||s.matched!=0||s.total!=10001 {return Err("qa_empty_filter".into());}
                                        platform::wait_for_completion()?;
                                        let s=platform::sync(&handle2,window.clone(),lease,3,Some(b),Filter{hide_loopback:false,..Default::default()})?;
                                        if s.routes!=10000||s.matched!=10001 {return Err("qa_filter_cache".into());}
                                        {
                                            let shared=handle2.state::<SharedState>();let mut session=shared.inner.lock().map_err(|_|"state_lock")?;
                                            session.demo[0].destination="127.0.0.1".into();session.demo[1].destination="198.51.100.1".into();let row=session.demo[1].clone();
                                            session.document.rules=vec![crate::model::Rule{
 transport:"TCP".into(),id:"qa-rule".into(),app:row.app,app_key:row.app_key,destination:row.destination,port:row.port,direction:"both".into(),action:"block".into(),source:"demo".into(),updated_at:1,identity:None,system_exception:false,timeout_action:"allow".into()}];
                                            session.revision+=1;
                                        }
                                        let s=platform::sync(&handle2,window.clone(),lease,4,Some(b),Filter::default())?;
                                        if s.routes!=10000||s.matched!=10000||s.truncated {return Err("qa_loopback_filter".into());}
                                        let s=platform::sync(&handle2,window.clone(),lease,5,Some(b),Filter{intent:Intent::Block,..Default::default()})?;
                                        if s.routes!=1||s.matched!=1||s.total!=10001 {return Err("qa_intent_filter".into());}
                                        let s=platform::sync(&handle2,window.clone(),lease,6,Some(b),Filter{intent:Intent::None,..Default::default()})?;
                                        if s.matched!=9999 {return Err("qa_none_filter".into());}
                                        if platform::sync(&handle2,window.clone(),lease,7,Some(b),Filter{source:Source::Live,..Default::default()}).is_ok(){return Err("qa_source_fence".into());}
                                        let s=platform::sync(&handle2,window.clone(),lease,8,Some(b),Filter::default())?;
                                        if s.matched!=10000 {return Err("qa_filter_recovery".into());}
                                        if platform::sync(&handle2,window.clone(),lease,7,Some(b),Filter::default()).is_ok(){return Err("qa_old_filter_restored".into());}
                                        platform::wait_for_completion()?;
                                        platform::verify_interactions(&handle2,window.clone(),lease,b)?;
                                        "filters_search_loopback_intent_empty_cap_source_and_sequence"
                                    }
                                    8 => {
                                        let old=lease;let floating=tauri::WebviewWindowBuilder::new(&handle2,"gpu-map",tauri::WebviewUrl::External("about:blank".parse().unwrap())).title("FlowGuard GPU floating QA").inner_size(1000.0,720.0).build().map_err(|e|e.to_string())?;
                                        let webview:&tauri::Webview=floating.as_ref();let floated=webview.window();
                                        handle2.state::<SharedState>().inner.lock().map_err(|_|"state_lock")?.gpu_owner="gpu-map".into();
                                        lease=platform::open_for("gpu-map");platform::close(old);platform::window_event("main",true);
                                        let status=platform::sync(&handle2,floated.clone(),lease,1,Some(b),crate::gpu_filter::Filter::default())?;
                                        if !status.presented{return Err("qa_float_surface".into());}platform::wait_for_completion()?;
                                        if platform::sync(&handle2,window,lease,2,Some(b),crate::gpu_filter::Filter::default()).is_ok(){return Err("qa_owner_not_checked".into());}
                                        platform::window_event("gpu-map",true);floating.close().map_err(|e|e.to_string())?;
                                        "floating_surface_owner_isolation_labels_zoom_and_shared_selection"
                                    }
                                    _ => {
                                        platform::destroy();
                                        if platform::sync(
                                            &handle2,
                                            window,
                                            lease,
                                            2,
                                            Some(b),
                                            crate::gpu_filter::Filter {
                                                hide_loopback: false,
                                                ..Default::default()
                                            },
                                        )
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
    let code = app.run_return(|_, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            platform::destroy();
        }
    });
    let _ = std::fs::remove_dir_all(path);
    if code == 0 {
        Ok(())
    } else {
        Err("native_surface_check_failed".into())
    }
}
