use crate::identity::SigningIdentity;
/// Verified Apple signing authority, exact code identifier and outbound TCP service.
/// No path, app name, port alone or endpoint-domain inference is trusted.
pub fn preset(
    identity: Option<&SigningIdentity>,
    port: u16,
    direction: &str,
    protocol: &str,
) -> Option<&'static str> {
    if protocol != "TCP" {
        return None;
    }
    let identity = identity.filter(|i| i.valid() && i.authority == "apple")?;
    if !["outbound", "both", "unknown"].contains(&direction) {
        return None;
    }
    match (identity.identifier.as_str(), port) {
        ("com.apple.mDNSResponder", 53) => Some("systemDNS"),
        ("com.apple.softwareupdated", 443) => Some("systemUpdates"),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_verified_apple_subject_service_and_direction() {
        let mut identity = SigningIdentity {
            authority: "apple".into(),
            team_id: "".into(),
            identifier: "com.apple.mDNSResponder".into(),
        };
        assert_eq!(
            preset(Some(&identity), 53, "outbound", "TCP"),
            Some("systemDNS")
        );
        assert_eq!(
            preset(Some(&identity), 53, "both", "TCP"),
            Some("systemDNS")
        );
        for protocol in ["UDP", "unknown", "", "tcp", "ICMP"] {
            assert_eq!(preset(Some(&identity), 53, "outbound", protocol), None);
        }
        assert_eq!(preset(Some(&identity), 5353, "outbound", "UDP"), None);
        assert_eq!(preset(Some(&identity), 53, "inbound", "TCP"), None);
        assert_eq!(preset(Some(&identity), 443, "outbound", "TCP"), None);
        assert_eq!(preset(None, 53, "outbound", "TCP"), None);
        identity.authority = "developer".into();
        identity.team_id = "ABCDE12345".into();
        assert_eq!(preset(Some(&identity), 53, "outbound", "TCP"), None);
        identity.authority = "apple".into();
        identity.team_id = "".into();
        identity.identifier = "com.apple.softwareupdated".into();
        assert_eq!(
            preset(Some(&identity), 443, "outbound", "TCP"),
            Some("systemUpdates")
        );
    }
}
