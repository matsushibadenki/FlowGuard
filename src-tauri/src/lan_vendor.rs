use std::{collections::BTreeMap, sync::LazyLock};
static REGISTRY: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../resources/oui-ma-l.json")).unwrap_or_default()
});
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vendor {
    pub organization: String,
    pub prefix: String,
    pub source: String,
    pub retrieved_at: String,
    pub trust: String,
}
pub fn candidate(mac: &str) -> Option<Vendor> {
    let parts = mac.split(':').collect::<Vec<_>>();
    if parts.len() != 6 || parts.iter().any(|p| p.len() != 2) {
        return None;
    }
    let bytes = parts
        .iter()
        .map(|p| u8::from_str_radix(p, 16))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if bytes[0] & 3 != 0 || bytes.iter().all(|b| *b == 0) {
        return None;
    }
    let prefix = bytes[..3]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<String>();
    let organization = REGISTRY.get(&prefix)?.clone();
    Some(Vendor {
        organization,
        prefix,
        source: "IEEE MA-L /24".into(),
        retrieved_at: "2026-10-26".into(),
        trust: "assignment_holder_candidate".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_random_multicast_and_bad_addresses() {
        assert!(candidate("02:00:00:00:00:01").is_none());
        assert!(candidate("01:00:5e:00:00:01").is_none());
        assert!(candidate("00:00:00:00:00:00").is_none());
        assert!(candidate("00:0c:29:00:00:01").is_some());
        assert!(candidate("zz:00:00:00:00:01").is_none());
    }
}
