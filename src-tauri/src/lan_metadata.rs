use crate::{
    discovery::{Device, Plan},
    model::now,
};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub services: Vec<Advertisement>,
    pub service_status: String,
    pub smb_names: Vec<String>,
    pub smb_domains: Vec<String>,
    pub smb_status: String,
    pub observed_at: u64,
    pub source: String,
    pub trust: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Advertisement {
    pub service: String,
    pub instance: String,
    pub target: String,
    pub port: u16,
    pub observed_at: u64,
}
fn ip(address: &str) -> Option<IpAddr> {
    address.split('%').next()?.parse().ok()
}
pub fn enrich(
    plan: &Plan,
    devices: &[Device],
    cancelled: &(impl Fn() -> bool + Sync),
) -> BTreeMap<String, Metadata> {
    let mut output = devices
        .iter()
        .take(1024)
        .map(|d| {
            (
                d.id.clone(),
                Metadata {
                    service_status: if plan.options.services {
                        "unobserved"
                    } else {
                        "not_requested"
                    }
                    .into(),
                    smb_status: if plan.options.smb {
                        "skipped"
                    } else {
                        "not_requested"
                    }
                    .into(),
                    observed_at: now(),
                    source: match (plan.options.services, plan.options.smb) {
                        (true, true) => "interface_scoped_mdns_and_unicast_nbstat",
                        (true, false) => "interface_scoped_mdns",
                        (false, true) => "unicast_nbstat",
                        _ => "not_requested",
                    }
                    .into(),
                    trust: "unverified_self_report".into(),
                    ..Default::default()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    if plan.options.services {
        let query = |name: &str, kind| {
            crate::names::mdns_records(name, kind, &plan.local.interface, cancelled)
        };
        let mut remaining = 8;
        for service in [
            "_smb._tcp.local",
            "_ssh._tcp.local",
            "_http._tcp.local",
            "_https._tcp.local",
        ] {
            if cancelled() {
                return output;
            }
            let records = match query(&format!("{service}."), 12) {
                Ok(v) => v,
                Err(_) => {
                    for row in output.values_mut() {
                        row.service_status = "partial".into();
                    }
                    continue;
                }
            };
            for data in records {
                if remaining == 0 || cancelled() {
                    break;
                }
                let Some(instance) = crate::names::parse_ptr(&data) else {
                    continue;
                };
                if !instance
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{service}"))
                {
                    continue;
                }
                remaining -= 1;
                let Ok(records) = query(&instance, 33) else {
                    continue;
                };
                for data in records.into_iter().take(1) {
                    let Some((port, target)) = srv(&data) else {
                        continue;
                    };
                    if !target.to_ascii_lowercase().ends_with(".local") {
                        continue;
                    }
                    let mut addresses = vec![];
                    for kind in [1, 28] {
                        if cancelled() {
                            return output;
                        }
                        if let Ok(records) = query(&target, kind) {
                            addresses.extend(records.into_iter().filter_map(|b| {
                                match (kind, b.len()) {
                                    (1, 4) => {
                                        Some(IpAddr::V4(Ipv4Addr::new(b[0], b[1], b[2], b[3])))
                                    }
                                    (28, 16) => Some(IpAddr::V6(std::net::Ipv6Addr::from(
                                        <[u8; 16]>::try_from(b).ok()?,
                                    ))),
                                    _ => None,
                                }
                            }));
                        }
                    }
                    for device in devices {
                        if ip(&device.address).is_some_and(|a| addresses.contains(&a)) {
                            if let Some(row) = output.get_mut(&device.id) {
                                if row.services.len() < 8 {
                                    row.services.push(Advertisement {
                                        service: service.into(),
                                        instance: instance.clone(),
                                        target: target.clone(),
                                        port,
                                        observed_at: now(),
                                    });
                                    row.service_status = "observed".into();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if plan.options.services && output.values().any(|m| m.services.len() >= 8) {
        for m in output.values_mut() {
            if m.service_status == "unobserved" {
                m.service_status = "limited".into();
            }
        }
    }
    if plan.options.smb {
        let mut eligible_count = 0;
        for d in devices {
            if d.source != "local"
                && ip(&d.address).is_some_and(|ip| matches!(ip,IpAddr::V4(a)if a.is_private()))
            {
                if let Some(row) = output.get_mut(&d.id) {
                    row.smb_status = if eligible_count >= 32 {
                        "limited"
                    } else {
                        "unobserved"
                    }
                    .into();
                }
                eligible_count += 1;
            }
        }
        let candidates = devices
            .iter()
            .filter(|d| {
                d.source != "local"
                    && ip(&d.address).is_some_and(|ip| matches!(ip,IpAddr::V4(a)if a.is_private()))
            })
            .take(32)
            .collect::<Vec<_>>();
        for batch in candidates.chunks(4) {
            if cancelled() {
                break;
            }
            let results = std::thread::scope(|scope| {
                let handles = batch
                    .iter()
                    .map(|d| scope.spawn(move || (&d.id, nbstat(plan, &d.address, cancelled))))
                    .collect::<Vec<_>>();
                handles
                    .into_iter()
                    .filter_map(|h| h.join().ok())
                    .collect::<Vec<_>>()
            });
            if cancelled() {
                break;
            }
            for (id, result) in results {
                if let Some(row) = output.get_mut(id) {
                    match result {
                        Ok((names, domains)) => {
                            row.smb_status = if names.is_empty() && domains.is_empty() {
                                "unobserved"
                            } else {
                                "observed"
                            }
                            .into();
                            row.smb_names = names;
                            row.smb_domains = domains;
                        }
                        Err(status) => row.smb_status = status,
                    }
                    row.observed_at = now();
                }
            }
        }
    }
    output
}
fn srv(bytes: &[u8]) -> Option<(u16, String)> {
    if bytes.len() < 8 {
        return None;
    }
    let port = u16::from_be_bytes([bytes[4], bytes[5]]);
    if port == 0 {
        return None;
    }
    Some((port, crate::names::parse_ptr(&bytes[6..])?))
}
fn nbstat(
    plan: &Plan,
    address: &str,
    cancelled: &(impl Fn() -> bool + Sync),
) -> Result<(Vec<String>, Vec<String>), String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (plan, address, cancelled);
        return Err("unsupported".into());
    }
    #[cfg(target_os = "macos")]
    {
        let target: Ipv4Addr = address.parse().map_err(|_| "skipped")?;
        let local: Ipv4Addr = plan.local.address.parse().map_err(|_| "skipped")?;
        let mask: Ipv4Addr = plan.local.netmask.parse().map_err(|_| "skipped")?;
        if !target.is_private()
            || (u32::from(target) & u32::from(mask)) != (u32::from(local) & u32::from(mask))
        {
            return Err("skipped".into());
        }
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .map_err(|_| "failed")?;
        let iface = std::ffi::CString::new(plan.local.interface.as_str()).map_err(|_| "failed")?;
        let index = unsafe { libc::if_nametoindex(iface.as_ptr()) };
        let index = std::num::NonZeroU32::new(index).ok_or("failed")?;
        socket
            .bind_device_by_index_v4(Some(index))
            .map_err(|_| "failed")?;
        socket
            .bind(&SocketAddr::from((local, 0)).into())
            .map_err(|_| "failed")?;
        let socket: std::net::UdpSocket = socket.into();
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .map_err(|_| "failed")?;
        static ID: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(1);
        let id = ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut request = vec![0u8; 12];
        request[..2].copy_from_slice(&id.to_be_bytes());
        request[5] = 1;
        request.push(32);
        for byte in [b'*'].into_iter().chain([0u8; 15]) {
            request.push(b'A' + (byte >> 4));
            request.push(b'A' + (byte & 15));
        }
        request.extend_from_slice(&[0, 0, 0x21, 0, 1]);
        socket
            .send_to(&request, (target, 137))
            .map_err(|_| "failed")?;
        let start = Instant::now();
        let mut bytes = [0u8; 2048];
        while start.elapsed() < Duration::from_millis(600) {
            if cancelled() {
                return Err("cancelled".into());
            }
            match socket.recv_from(&mut bytes) {
                Ok((len, peer)) if peer == SocketAddr::from((target, 137)) => {
                    return parse_nbstat(&bytes[..len], id).ok_or_else(|| "invalid".into())
                }
                Ok(_) => continue,
                Err(e)
                    if [std::io::ErrorKind::WouldBlock, std::io::ErrorKind::TimedOut]
                        .contains(&e.kind()) =>
                {
                    continue
                }
                Err(_) => return Err("failed".into()),
            }
        }
        Err("unobserved".into())
    }
}
fn skip_dns_name(bytes: &[u8], mut offset: usize) -> Option<usize> {
    for _ in 0..128 {
        let n = *bytes.get(offset)? as usize;
        if n & 0xc0 == 0xc0 {
            return (bytes.get(offset + 1).is_some()).then_some(offset + 2);
        }
        if n > 63 {
            return None;
        }
        offset += 1;
        if n == 0 {
            return Some(offset);
        }
        offset = offset.checked_add(n)?;
        if offset > bytes.len() {
            return None;
        }
    }
    None
}
fn parse_nbstat(bytes: &[u8], id: u16) -> Option<(Vec<String>, Vec<String>)> {
    if bytes.len() < 12
        || u16::from_be_bytes([bytes[0], bytes[1]]) != id
        || bytes[2] & 0x80 == 0
        || bytes[2] & 0x7a != 0
        || bytes[3] & 15 != 0
    {
        return None;
    }
    let q = u16::from_be_bytes([bytes[4], bytes[5]]);
    let a = u16::from_be_bytes([bytes[6], bytes[7]]);
    if q > 1 || a != 1 {
        return None;
    }
    let mut offset = 12;
    for _ in 0..q {
        offset = skip_dns_name(bytes, offset)?.checked_add(4)?;
    }
    offset = skip_dns_name(bytes, offset)?;
    let header = bytes.get(offset..offset + 10)?;
    if header[..4] != [0, 0x21, 0, 1] {
        return None;
    }
    let length = u16::from_be_bytes([header[8], header[9]]) as usize;
    let data = bytes.get(offset + 10..offset + 10 + length)?;
    let count = *data.first()? as usize;
    if count > 32 || data.len() < 1 + count * 18 + 6 {
        return None;
    }
    let mut names = vec![];
    let mut domains = vec![];
    for record in data[1..1 + count * 18].chunks_exact(18) {
        let raw = std::str::from_utf8(&record[..15]).ok()?.trim();
        if raw.is_empty() || raw.chars().any(|c| !c.is_ascii() || c.is_control()) {
            continue;
        }
        let flags = u16::from_be_bytes([record[16], record[17]]);
        let list = if record[15] == 0x20 && flags & 0x8000 == 0 {
            &mut names
        } else if record[15] == 0 && flags & 0x8000 != 0 {
            &mut domains
        } else {
            continue;
        };
        if !list.iter().any(|v| v == raw) && list.len() < 8 {
            list.push(raw.to_owned());
        }
    }
    Some((names, domains))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nbstat_requires_transaction_flags_types_and_complete_payload() {
        let mut bytes = vec![
            0x12, 0x34, 0x80, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0xc0, 0x0c, 0, 0x21, 0, 1, 0, 0, 0, 10, 0,
            43, 2,
        ];
        for (name, suffix, flags) in [("FILESERVER", 0x20, 0u16), ("WORKGROUP", 0, 0x8000u16)] {
            let mut record = [b' '; 15];
            record[..name.len()].copy_from_slice(name.as_bytes());
            bytes.extend(record);
            bytes.push(suffix);
            bytes.extend(flags.to_be_bytes());
        }
        bytes.extend([0u8; 6]);
        assert_eq!(
            parse_nbstat(&bytes, 0x1234),
            Some((vec!["FILESERVER".into()], vec!["WORKGROUP".into()]))
        );
        assert!(parse_nbstat(&bytes, 1).is_none());
        for n in 0..bytes.len() {
            assert!(parse_nbstat(&bytes[..n], 0x1234).is_none());
        }
        let mut bad = bytes.clone();
        bad[2] |= 2;
        assert!(parse_nbstat(&bad, 0x1234).is_none());
        bad = bytes.clone();
        bad[15] = 1;
        assert!(parse_nbstat(&bad, 0x1234).is_none());
    }
    #[test]
    fn parses_srv_and_rejects_bad_nbstat() {
        assert!(srv(&[0; 7]).is_none());
        assert!(parse_nbstat(&[0; 12], 1).is_none());
        assert!(skip_dns_name(&[0xc0], 0).is_none());
        let mut b = vec![0, 0, 0, 0, 1, 0xbb];
        b.extend_from_slice(&[
            4, b'h', b'o', b's', b't', 5, b'l', b'o', b'c', b'a', b'l', 0,
        ]);
        assert_eq!(srv(&b), Some((443, "host.local".into())));
    }
}
