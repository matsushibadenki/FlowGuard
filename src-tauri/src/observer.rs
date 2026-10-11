use crate::model::{now, Connection, FirewallStatus};
use std::{
    collections::HashMap,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

// Fixed, read-only tools. No shell, sudo, arbitrary arguments, DNS lookups or packet payloads.
fn run_bounded(binary: &str, args: &[&str]) -> Result<(bool, String), String> {
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let pipe = child.stdout.take().ok_or("missing stdout")?;
    // Read concurrently so a full pipe cannot deadlock the observer.
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut text = String::new();
        pipe.take(4 * 1024 * 1024)
            .read_to_string(&mut text)
            .map(|_| text)
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("observer_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let text = reader
        .join()
        .map_err(|_| "observer_reader_failed")?
        .map_err(|e| e.to_string())?;
    Ok((status.success(), text))
}
fn executable(pid: u32) -> String {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::{c_char, c_int, c_void, CStr};
        unsafe extern "C" {
            fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
        }
        let mut path = [0u8; 4096];
        let n = unsafe { proc_pidpath(pid as c_int, path.as_mut_ptr().cast(), path.len() as u32) };
        if n > 0 {
            return unsafe { CStr::from_ptr(path.as_ptr() as *const c_char) }
                .to_string_lossy()
                .into_owned();
        }
    }
    let _ = pid;
    String::new()
}
fn split_endpoint(value: &str) -> Option<(String, u16)> {
    let (host, port) = value.rsplit_once(':')?;
    Some((
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .to_string(),
        port.parse().ok()?,
    ))
}
pub fn parse_lsof(text: &str, timestamp: u64) -> Vec<Connection> {
    let mut pid = 0;
    let mut app = String::new();
    let mut protocol = "TCP".to_string();
    let mut result = Vec::new();
    for line in text.lines() {
        let Some(kind) = line.chars().next() else {
            continue;
        };
        let value = &line[1..];
        match kind {
            'p' => {
                pid = value.parse().unwrap_or(0);
                app.clear();
                protocol = "TCP".into();
            }
            'c' => app = value.into(),
            'P' => protocol = value.into(),
            'n' => {
                if let Some((local, remote)) = value.split_once("->") {
                    if let Some((destination, port)) = split_endpoint(remote) {
                        if pid > 0 && !app.is_empty() {
                            result.push(Connection {
                                id: format!("{pid}:{protocol}:{value}"),
                                app: app.clone(),
                                app_key: String::new(),
                                pid,
                                local: local.into(),
                                destination,
                                port,
                                protocol: protocol.clone(),
                                direction: "unknown".into(),
                                observed_at: timestamp,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    result.sort_by(|a, b| a.app.cmp(&b.app).then(a.id.cmp(&b.id)));
    result.dedup_by(|a, b| a.id == b.id);
    result
}
pub fn observe() -> (Vec<Connection>, FirewallStatus) {
    if !cfg!(target_os = "macos") {
        return (
            vec![],
            FirewallStatus {
                enabled: None,
                enforcement: "unavailable".into(),
                observer: "unsupported".into(),
                error: Some("macos_required".into()),
            },
        );
    }
    let enabled = run_bounded(
        "/usr/libexec/ApplicationFirewall/socketfilterfw",
        &["--getglobalstate"],
    )
    .ok()
    .and_then(|(success, text)| {
        if !success {
            None
        } else if text.contains("State = 1") {
            Some(true)
        } else if text.contains("State = 0") {
            Some(false)
        } else {
            None
        }
    });
    let scan = run_bounded(
        "/usr/sbin/lsof",
        &["-nP", "-iTCP", "-sTCP:ESTABLISHED", "-FpcnPt"],
    );
    let (mut connections, error) = match scan {
        Ok((success, text)) if success || text.is_empty() => (parse_lsof(&text, now()), None),
        Ok(_) => (vec![], Some("observer_failed".into())),
        Err(_) => (vec![], Some("observer_failed".into())),
    };
    let mut paths = HashMap::new();
    for c in &mut connections {
        c.app_key = paths
            .entry(c.pid)
            .or_insert_with(|| executable(c.pid))
            .clone();
    }
    (
        connections,
        FirewallStatus {
            enabled,
            enforcement: "unavailable".into(),
            observer: if error.is_some() { "error" } else { "limited" }.into(),
            error,
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_ipv6_multiple_processes_and_ignores_listeners() {
        let input="p42\ncMail\nPTCP\nn[::1]:5000->[2001:db8::2]:993\np55\ncSafari\nn127.0.0.1:6000->17.1.2.3:443\nn*:80\n";
        let rows = parse_lsof(input, 100);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].destination, "2001:db8::2");
        assert_eq!(rows[0].port, 993);
        assert_eq!(rows[1].pid, 55);
        assert_eq!(rows[1].direction, "unknown");
    }
    #[test]
    fn malformed_output_is_not_a_connection() {
        assert!(parse_lsof(
            "pbad\ncName\nnlocalhost:1->invalid\np23\nn127.0.0.1:2->1.2.3.4:3",
            0
        )
        .is_empty());
    }
}
