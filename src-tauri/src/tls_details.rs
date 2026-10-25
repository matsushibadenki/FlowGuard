//! Display-only certificate metadata. Trust decisions remain entirely in rustls/webpki.
use serde::Serialize;
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub server_name: String,
    pub verification: String,
    pub reason: String,
    pub certificate: Option<Certificate>,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Certificate {
    pub issuer: Option<String>,
    pub not_before: Option<i64>,
    pub not_after: Option<i64>,
}
// ASCII A-labels only: no URLs, IP literals, wildcards, ports, control characters or DNS lookups.
pub fn hostname(value: &str) -> Result<Option<String>, String> {
    if value.chars().any(char::is_control) {
        return Err("service_hostname".into());
    }
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 253
        || !value.is_ascii()
        || value.parse::<std::net::IpAddr>().is_ok()
        || !value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("service_hostname".into());
    }
    Ok(Some(value.to_ascii_lowercase()))
}
pub fn reason(error: &std::io::Error) -> &'static str {
    use rustls::{CertificateError as C, Error as E};
    match error.get_ref().and_then(|e| e.downcast_ref::<E>()) {
        Some(E::InvalidCertificate(c)) => match c {
            C::Expired | C::ExpiredContext { .. } => "expired",
            C::NotValidYet | C::NotValidYetContext { .. } => "not_yet_valid",
            C::NotValidForName | C::NotValidForNameContext { .. } => "name_mismatch",
            C::UnknownIssuer => "unknown_issuer",
            C::Revoked => "revoked",
            C::BadSignature => "bad_signature",
            C::BadEncoding => "bad_certificate",
            _ => "certificate_rejected",
        },
        _ if matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) =>
        {
            "timeout"
        }
        _ => "handshake_failed",
    }
}
// A bounded, fixed-depth DER reader for issuer/validity display only; never a certificate validator.
fn element<'a>(input: &mut &'a [u8]) -> Option<(u8, &'a [u8])> {
    let (&tag, rest) = input.split_first()?;
    let (&first, mut rest) = rest.split_first()?;
    let len = if first < 128 {
        first as usize
    } else {
        let count = (first & 127) as usize;
        if count == 0 || count > 4 || rest.len() < count || rest[0] == 0 {
            return None;
        }
        let mut len = 0usize;
        for &byte in &rest[..count] {
            len = len.checked_mul(256)?.checked_add(byte as usize)?;
        }
        rest = &rest[count..];
        if len < 128 {
            return None;
        }
        len
    };
    if rest.len() < len {
        return None;
    }
    let (value, tail) = rest.split_at(len);
    *input = tail;
    Some((tag, value))
}
fn take<'a>(input: &mut &'a [u8], tag: u8) -> Option<&'a [u8]> {
    let (actual, value) = element(input)?;
    (actual == tag).then_some(value)
}
fn timestamp(tag: u8, value: &[u8]) -> Option<i64> {
    if !value.is_ascii() {
        return None;
    }
    let text = std::str::from_utf8(value).ok()?;
    let (year, offset) = match (tag, text.len()) {
        (0x17, 13) => {
            let y = text.get(..2)?.parse::<i32>().ok()?;
            (if y >= 50 { 1900 + y } else { 2000 + y }, 2)
        }
        (0x18, 15) => (text.get(..4)?.parse::<i32>().ok()?, 4),
        _ => return None,
    };
    if !text.ends_with('Z') || !text[..text.len() - 1].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = |i: usize| {
        text.get(offset as usize + i..offset as usize + i + 2)?
            .parse::<u8>()
            .ok()
    };
    let date =
        time::Date::from_calendar_date(year, time::Month::try_from(n(0)?).ok()?, n(2)?).ok()?;
    Some(
        date.with_hms(n(4)?, n(6)?, n(8)?)
            .ok()?
            .assume_utc()
            .unix_timestamp(),
    )
}
fn issuer(mut data: &[u8]) -> Option<String> {
    let mut values = vec![];
    for _ in 0..32 {
        if data.is_empty() {
            return (!values.is_empty()).then(|| values.join(", "));
        }
        let mut set = take(&mut data, 0x31)?;
        for _ in 0..16 {
            if set.is_empty() {
                break;
            }
            let mut attr = take(&mut set, 0x30)?;
            let oid = take(&mut attr, 6)?;
            let key = match oid {
                [0x55, 4, 3] => "CN",
                [0x55, 4, 10] => "O",
                [0x55, 4, 11] => "OU",
                [0x55, 4, 6] => "C",
                [0x55, 4, 7] => "L",
                [0x55, 4, 8] => "ST",
                _ => continue,
            };
            let (tag, value) = element(&mut attr)?;
            if !attr.is_empty() {
                return None;
            }
            let value = match tag {
                0x0c | 0x13 | 0x16 => std::str::from_utf8(value).ok()?.to_string(),
                0x1e if value.len() % 2 == 0 => String::from_utf16(
                    &value
                        .chunks_exact(2)
                        .map(|p| u16::from_be_bytes([p[0], p[1]]))
                        .collect::<Vec<_>>(),
                )
                .ok()?,
                _ => continue,
            };
            let safe: String = value
                .chars()
                .filter(|c| {
                    !c.is_control() && !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
                })
                .take(256)
                .collect();
            values.push(format!("{key}={safe}"));
        }
        if !set.is_empty() {
            return None;
        }
    }
    None
}
pub fn certificate(der: &[u8]) -> Option<Certificate> {
    if der.len() > 64 * 1024 {
        return None;
    }
    let mut outer = der;
    let mut cert = take(&mut outer, 0x30)?;
    if !outer.is_empty() {
        return None;
    }
    let mut tbs = take(&mut cert, 0x30)?;
    if tbs.first() == Some(&0xa0) {
        take(&mut tbs, 0xa0)?;
    }
    take(&mut tbs, 2)?;
    take(&mut tbs, 0x30)?;
    let issuer = issuer(take(&mut tbs, 0x30)?);
    let mut validity = take(&mut tbs, 0x30)?;
    let (tag, value) = element(&mut validity)?;
    let not_before = timestamp(tag, value);
    let (tag, value) = element(&mut validity)?;
    let not_after = timestamp(tag, value);
    if !validity.is_empty() {
        return None;
    }
    Some(Certificate {
        issuer,
        not_before,
        not_after,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_name_is_not_a_destination_or_header_injection() {
        assert_eq!(
            hostname(" Example.COM ").unwrap().as_deref(),
            Some("example.com")
        );
        assert_eq!(hostname("").unwrap(), None);
        assert!(hostname("xn--bcher-kva.example").is_ok());
        for n in [
            "https://example.com",
            "*.example.com",
            "x:443",
            "127.0.0.1",
            "::1",
            "a..b",
            "-a.b",
            "a-.b",
            "a.b.",
            "a_b",
            "例.example",
            "x\r\nHost: evil",
            "example.com\r\n",
        ] {
            assert!(hostname(n).is_err(), "{n}");
        }
        assert!(hostname(&"a".repeat(64)).is_err());
    }
    #[test]
    fn date_and_der_limits_do_not_invent_metadata() {
        assert_eq!(timestamp(0x17, b"700101000000Z"), Some(0));
        assert_eq!(timestamp(0x18, b"19700101000000Z"), Some(0));
        for date in [
            b"260230000000Z",
            b"261001250000Z",
            b"261001000000+",
            b"26100100x000Z",
        ] {
            assert_eq!(timestamp(0x17, date), None);
        }
        for value in [
            vec![],
            vec![0x30, 0x80],
            vec![0x30, 0x84, 255, 255, 255, 255],
            vec![0; 65537],
        ] {
            assert!(certificate(&value).is_none());
        }
        let mut input = &b"\x30\x81\x01\x00"[..];
        assert!(element(&mut input).is_none());
    }
    #[test]
    fn validation_reasons_are_structured() {
        for (error, expected) in [
            (rustls::CertificateError::UnknownIssuer, "unknown_issuer"),
            (rustls::CertificateError::NotValidForName, "name_mismatch"),
            (rustls::CertificateError::Expired, "expired"),
            (rustls::CertificateError::NotValidYet, "not_yet_valid"),
        ] {
            let e = std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                rustls::Error::InvalidCertificate(error),
            );
            assert_eq!(reason(&e), expected);
        }
    }
}
