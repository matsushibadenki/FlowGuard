use crate::model::now;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameObservation {
    #[serde(default)]
    pub values: Vec<String>,
    pub value: Option<String>,
    pub source: Option<String>,
    pub status: String,
    pub observed_at: Option<u64>,
}
impl Default for NameObservation {
    fn default() -> Self {
        Self {
            values: vec![],
            value: None,
            source: None,
            status: "not_requested".into(),
            observed_at: None,
        }
    }
}
fn valid_name(name: &str) -> Option<String> {
    let name = name.trim().trim_end_matches('.');
    if name.is_empty() || name.len() > 253 || name.chars().any(char::is_control) {
        None
    } else {
        Some(name.into())
    }
}
pub fn eligible(address: &str, mode: &str) -> bool {
    match address.split('%').next().unwrap_or("").parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_private(),
        Ok(IpAddr::V6(ip)) => {
            mode == "mdns" && (ip.is_unicast_link_local() || ip.is_unique_local())
        }
        _ => false,
    }
}
fn observation(value: Option<String>, source: &str, status: &str) -> NameObservation {
    NameObservation {
        values: value.iter().cloned().collect(),
        value,
        source: Some(source.into()),
        status: status.into(),
        observed_at: Some(now()),
    }
}
fn parse_system(output: &str, address: IpAddr) -> Option<String> {
    output.split("\n\n").find_map(|block| {
        let matches = block.lines().any(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                ["ip_address", "ipv6_address"].contains(&key.trim())
                    && value.trim().parse::<IpAddr>().ok() == Some(address)
            })
        });
        if !matches {
            return None;
        }
        block
            .lines()
            .find_map(|line| line.strip_prefix("name:").and_then(valid_name))
    })
}
pub fn system(address: IpAddr) -> NameObservation {
    let value = address.to_string();
    match crate::observer::run_bounded(
        "/usr/bin/dscacheutil",
        &["-q", "host", "-a", "ip_address", &value],
    ) {
        Ok((true, text)) => {
            let name = parse_system(&text, address);
            let status = if name.is_some() {
                "resolved"
            } else {
                "unobserved"
            };
            let mut result = observation(name, "system", status);
            for block in text.split("\n\n") {
                let matches = block.lines().any(|line| {
                    line.split_once(':').is_some_and(|(k, v)| {
                        ["ip_address", "ipv6_address"].contains(&k.trim())
                            && v.trim().parse::<IpAddr>().ok() == Some(address)
                    })
                });
                if matches {
                    for name in block
                        .lines()
                        .filter_map(|l| l.strip_prefix("name:").and_then(valid_name))
                    {
                        if result.values.len() < 8
                            && !result.values.iter().any(|v| v.eq_ignore_ascii_case(&name))
                        {
                            result.values.push(name);
                        }
                    }
                }
            }
            result
        }
        _ => observation(None, "system", "failed"),
    }
}
fn reverse_name(address: IpAddr) -> String {
    match address {
        IpAddr::V4(ip) => {
            let b = ip.octets();
            format!("{}.{}.{}.{}.in-addr.arpa.", b[3], b[2], b[1], b[0])
        }
        IpAddr::V6(ip) => {
            let hex = format!("{:032x}", u128::from(ip));
            format!(
                "{}.ip6.arpa.",
                hex.chars()
                    .rev()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            )
        }
    }
}
// DNS-SD PTR rdata uses uncompressed DNS labels. Reject pointers, truncation and control characters.
pub(crate) fn parse_ptr(bytes: &[u8]) -> Option<String> {
    if bytes.len() > 255 {
        return None;
    }
    let mut index = 0;
    let mut labels = vec![];
    while index < bytes.len() {
        let size = usize::from(bytes[index]);
        index += 1;
        if size == 0 {
            return if index == bytes.len() {
                valid_name(&labels.join("."))
            } else {
                None
            };
        }
        if size > 63 || index + size > bytes.len() {
            return None;
        }
        labels.push(
            std::str::from_utf8(&bytes[index..index + size])
                .ok()?
                .to_string(),
        );
        index += size;
    }
    None
}
#[cfg(target_os = "macos")]
mod mdns {
    use super::*;
    use std::{
        ffi::{c_char, c_void, CString},
        time::{Duration, Instant},
    };
    type Ref = *mut c_void;
    type Reply = unsafe extern "C" fn(
        Ref,
        u32,
        u32,
        i32,
        *const c_char,
        u16,
        u16,
        u16,
        *const c_void,
        u32,
        *mut c_void,
    );
    #[link(name = "System")]
    unsafe extern "C" {
        fn DNSServiceQueryRecord(
            service: *mut Ref,
            flags: u32,
            interface: u32,
            name: *const c_char,
            rrtype: u16,
            rrclass: u16,
            callback: Reply,
            context: *mut c_void,
        ) -> i32;
        fn DNSServiceRefSockFD(service: Ref) -> i32;
        fn DNSServiceProcessResult(service: Ref) -> i32;
        fn DNSServiceRefDeallocate(service: Ref);
    }
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    unsafe extern "C" {
        fn if_nametoindex(name: *const c_char) -> u32;
        fn poll(fds: *mut PollFd, count: u32, timeout: i32) -> i32;
    }
    struct Service(Ref);
    impl Drop for Service {
        fn drop(&mut self) {
            unsafe { DNSServiceRefDeallocate(self.0) }
        }
    }
    struct Context {
        result: NameObservation,
        interface: u32,
        finished: bool,
    }
    unsafe extern "C" fn callback(
        _service: Ref,
        flags: u32,
        interface: u32,
        error: i32,
        _name: *const c_char,
        rrtype: u16,
        rrclass: u16,
        size: u16,
        data: *const c_void,
        ttl: u32,
        context: *mut c_void,
    ) {
        if context.is_null() {
            return;
        }
        let context = &mut *context.cast::<Context>();
        if error != 0 {
            context.result.status = if error == -65554 {
                "unobserved"
            } else {
                "failed"
            }
            .into();
            context.finished = true;
            return;
        }
        if flags & 2 == 0
            || interface != context.interface
            || rrtype != 12
            || rrclass != 1
            || data.is_null()
            || ttl == 0
        {
            return;
        }
        if let Some(name) = parse_ptr(std::slice::from_raw_parts(
            data.cast::<u8>(),
            usize::from(size),
        )) {
            if context.result.values.len() < 8
                && !context
                    .result
                    .values
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(&name))
            {
                if context.result.value.is_none() {
                    context.result.value = Some(name.clone());
                }
                context.result.values.push(name);
                context.result.status = "resolved".into();
                context.result.observed_at = Some(now());
            }
            context.finished = flags & 1 == 0 || context.result.values.len() >= 8;
        }
    }
    pub fn lookup(
        address: IpAddr,
        interface: &str,
        cancelled: impl Fn() -> bool,
    ) -> NameObservation {
        let Ok(interface) = CString::new(interface) else {
            return observation(None, "mdns", "failed");
        };
        let index = unsafe { if_nametoindex(interface.as_ptr()) };
        if index == 0 {
            return observation(None, "mdns", "failed");
        }
        let query = CString::new(reverse_name(address)).unwrap();
        let mut context = Context {
            result: observation(None, "mdns", "unobserved"),
            interface: index,
            finished: false,
        };
        let mut raw = std::ptr::null_mut();
        // Force multicast (0x400), scope the PTR query to the selected interface; no unicast DNS fallback.
        let error = unsafe {
            DNSServiceQueryRecord(
                &mut raw,
                0x400,
                index,
                query.as_ptr(),
                12,
                1,
                callback,
                (&mut context as *mut Context).cast(),
            )
        };
        if error != 0 || raw.is_null() {
            return observation(None, "mdns", "failed");
        }
        let service = Service(raw);
        let fd = unsafe { DNSServiceRefSockFD(service.0) };
        if fd < 0 {
            return observation(None, "mdns", "failed");
        }
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(1) && !context.finished {
            if cancelled() {
                return observation(None, "mdns", "cancelled");
            }
            let mut pollfd = PollFd {
                fd,
                events: 1,
                revents: 0,
            };
            let ready = unsafe { poll(&mut pollfd, 1, 100) };
            if ready < 0 || pollfd.revents & !1 != 0 {
                return observation(None, "mdns", "failed");
            }
            if ready > 0
                && pollfd.revents & 1 != 0
                && unsafe { DNSServiceProcessResult(service.0) } != 0
            {
                return observation(None, "mdns", "failed");
            }
        }
        // Deallocate before stack context drops; callbacks run synchronously during ProcessResult only.
        drop(service);
        context.result.observed_at = Some(now());
        context.result
    }
    struct Records {
        interface: u32,
        kind: u16,
        values: Vec<Vec<u8>>,
        failed: bool,
    }
    unsafe extern "C" fn record_callback(
        _service: Ref,
        flags: u32,
        interface: u32,
        error: i32,
        _name: *const c_char,
        rrtype: u16,
        rrclass: u16,
        size: u16,
        data: *const c_void,
        ttl: u32,
        context: *mut c_void,
    ) {
        if context.is_null() {
            return;
        }
        let c = &mut *context.cast::<Records>();
        if error != 0 {
            c.failed = true;
            return;
        }
        if flags & 2 == 0
            || interface != c.interface
            || rrtype != c.kind
            || rrclass != 1
            || ttl == 0
            || data.is_null()
            || size > 2048
            || c.values.len() >= 16
        {
            return;
        }
        let value = std::slice::from_raw_parts(data.cast::<u8>(), size as usize).to_vec();
        if !c.values.contains(&value) {
            c.values.push(value);
        }
    }
    pub fn records(
        name: &str,
        kind: u16,
        interface: &str,
        cancelled: &(impl Fn() -> bool + Sync),
    ) -> Result<Vec<Vec<u8>>, String> {
        let query = CString::new(name).map_err(|_| "invalid_query")?;
        let iface = CString::new(interface).map_err(|_| "invalid_interface")?;
        let index = unsafe { if_nametoindex(iface.as_ptr()) };
        if index == 0 {
            return Err("invalid_interface".into());
        }
        let mut context = Records {
            interface: index,
            kind,
            values: vec![],
            failed: false,
        };
        let mut raw = std::ptr::null_mut();
        let error = unsafe {
            DNSServiceQueryRecord(
                &mut raw,
                0x400,
                index,
                query.as_ptr(),
                kind,
                1,
                record_callback,
                (&mut context as *mut Records).cast(),
            )
        };
        if error != 0 || raw.is_null() {
            return Err("mdns_unavailable".into());
        }
        let service = Service(raw);
        let fd = unsafe { DNSServiceRefSockFD(service.0) };
        if fd < 0 {
            return Err("mdns_unavailable".into());
        }
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(600)
            && !context.failed
            && context.values.len() < 16
        {
            if cancelled() {
                return Err("cancelled".into());
            }
            let mut pollfd = PollFd {
                fd,
                events: 1,
                revents: 0,
            };
            let ready = unsafe { poll(&mut pollfd, 1, 50) };
            if ready < 0 || pollfd.revents & !1 != 0 {
                return Err("mdns_unavailable".into());
            }
            if ready > 0 && unsafe { DNSServiceProcessResult(service.0) } != 0 {
                return Err("mdns_unavailable".into());
            }
        }
        drop(service);
        if context.failed {
            Err("mdns_unavailable".into())
        } else {
            Ok(context.values)
        }
    }
}
pub(crate) fn mdns_records(
    name: &str,
    kind: u16,
    interface: &str,
    cancelled: &(impl Fn() -> bool + Sync),
) -> Result<Vec<Vec<u8>>, String> {
    #[cfg(target_os = "macos")]
    {
        mdns::records(name, kind, interface, cancelled)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (name, kind, interface, cancelled);
        Err("macos_required".into())
    }
}
pub fn lookup(
    address: &str,
    interface: &str,
    mode: &str,
    cancelled: impl Fn() -> bool,
) -> NameObservation {
    if !eligible(address, mode) {
        let mut result = NameObservation::default();
        result.status = "skipped".into();
        return result;
    }
    let ip = address
        .split('%')
        .next()
        .unwrap_or("")
        .parse::<IpAddr>()
        .unwrap();
    match mode {
        "system" => system(ip),
        #[cfg(target_os = "macos")]
        "mdns" => mdns::lookup(ip, interface, cancelled),
        _ => observation(None, mode, "failed"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ptr_parser_rejects_bad_labels_and_keeps_reported_name() {
        assert_eq!(
            parse_ptr(b"\x07Printer\x05local\0").as_deref(),
            Some("Printer.local")
        );
        for bytes in [
            &b"\xc0\x01"[..],
            &b"\x03ab"[..],
            &b"\x01\n\0"[..],
            &b"\x01a\0x"[..],
        ] {
            assert!(parse_ptr(bytes).is_none());
        }
    }
    #[test]
    fn reverse_queries_cover_both_families_and_do_not_include_scope() {
        assert_eq!(
            reverse_name("192.168.1.2".parse().unwrap()),
            "2.1.168.192.in-addr.arpa."
        );
        assert_eq!(
            reverse_name("::1".parse().unwrap()),
            format!("1.{}ip6.arpa.", "0.".repeat(31))
        );
        assert!(eligible("fe80::1%en0", "mdns"));
        assert!(!eligible("fe80::1%en0", "system"));
        assert!(!eligible("8.8.8.8", "mdns"));
    }
    #[test]
    fn system_name_requires_an_exact_address_record() {
        let text="name: gateway.example\nip_address: 192.168.1.1\n\nname: other.example\nip_address: 192.168.1.2\n";
        assert_eq!(
            parse_system(text, "192.168.1.2".parse().unwrap()).as_deref(),
            Some("other.example")
        );
        assert!(parse_system(text, "192.168.1.3".parse().unwrap()).is_none());
    }
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Resolves loopback via the system resolver; no LAN device query"]
    fn unsigned_system_resolver_reads_localhost() {
        let name = system("127.0.0.1".parse().unwrap());
        assert_eq!(name.value.as_deref(), Some("localhost"));
        assert_eq!(name.source.as_deref(), Some("system"));
    }
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Uses the DNS-SD API on loopback only; no LAN query"]
    fn unsigned_mdns_abi_is_bounded_on_loopback() {
        let start = std::time::Instant::now();
        let result = mdns::lookup("127.0.0.1".parse().unwrap(), "lo0", || false);
        assert_ne!(result.status, "failed");
        assert!(start.elapsed() < std::time::Duration::from_secs(3));
    }
}
