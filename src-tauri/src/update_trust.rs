//! Build-pinned, root-authorized updater-key rotation. Feed-supplied keys are never trusted.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Policy {
    version: u32,
    root_valid_until: Option<String>,
    delegations: Vec<Delegation>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Delegation {
    id: String,
    public_key: String,
    min_version: String,
    channels: Vec<String>,
    authorization: String,
}
fn decode(s: &str) -> Option<String> {
    if s.len() > 8192 {
        return None;
    }
    String::from_utf8(STANDARD.decode(s).ok()?).ok()
}
pub fn keys(policy: &str, root: &str, version: &str, channel: &str) -> Result<Vec<String>, String> {
    let p: Policy = serde_json::from_str(policy).map_err(|_| "update_trust_policy")?;
    let version = semver::Version::parse(version).map_err(|_| "invalid_version")?;
    if p.version != 1 || p.delegations.len() > 8 || !["stable", "beta"].contains(&channel) {
        return Err("update_trust_policy".into());
    }
    let root_key = minisign_verify::PublicKey::decode(&decode(root).ok_or("update_trust_policy")?)
        .map_err(|_| "update_trust_policy")?;
    let mut result = vec![];
    if p.root_valid_until
        .as_ref()
        .map(|v| semver::Version::parse(v).map(|last| version <= last))
        .transpose()
        .map_err(|_| "update_trust_policy")?
        .unwrap_or(true)
    {
        result.push(root.into());
    }
    let mut ids = std::collections::HashSet::new();
    for g in p.delegations {
        if g.id.is_empty()
            || g.id.len() > 64
            || !g
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !ids.insert(g.id.clone())
            || g.public_key.len() > 1024
            || g.channels.is_empty()
            || g.channels.len() > 2
            || g.channels
                .iter()
                .any(|c| !["stable", "beta"].contains(&c.as_str()))
        {
            return Err("update_trust_policy".into());
        }
        let mut channels = g.channels.clone();
        channels.sort();
        channels.dedup();
        if channels.len() != g.channels.len() {
            return Err("update_trust_policy".into());
        }
        let payload = format!(
            "FlowGuardUpdaterKey/v1\n{}\n{}\n{}\n{}\n",
            g.id,
            g.public_key,
            g.min_version,
            channels.join(",")
        );
        let authorization = minisign_verify::Signature::decode(
            &decode(&g.authorization).ok_or("update_trust_policy")?,
        )
        .map_err(|_| "update_trust_policy")?;
        root_key
            .verify(payload.as_bytes(), &authorization, false)
            .map_err(|_| "update_trust_policy")?;
        minisign_verify::PublicKey::decode(&decode(&g.public_key).ok_or("update_trust_policy")?)
            .map_err(|_| "update_trust_policy")?;
        let first = semver::Version::parse(&g.min_version).map_err(|_| "update_trust_policy")?;
        if version >= first && channels.iter().any(|c| c == channel) {
            result.push(g.public_key);
        }
    }
    if result.is_empty() {
        return Err("update_key_unavailable".into());
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authorized_rotation_retires_root_and_tampering_cannot_enroll_a_key() {
        let root = include_str!("../tests/fixtures/key-rotation/root.pub").trim();
        let policy = include_str!("../tests/fixtures/key-rotation/grant.json");
        let old = keys(policy, root, "1.1.0", "stable").unwrap();
        assert_eq!(old, vec![root]);
        let new = keys(policy, root, "1.2.0", "stable").unwrap();
        assert_eq!(new.len(), 1);
        assert_ne!(new[0], root);
        assert_eq!(keys(policy, root, "1.2.0", "beta").unwrap(), new);
        let mut tampered: serde_json::Value = serde_json::from_str(policy).unwrap();
        tampered["delegations"][0]["minVersion"] = "1.0.0".into();
        assert!(keys(&tampered.to_string(), root, "1.2.0", "stable").is_err());
    }
    #[test]
    fn only_pinned_root_or_authorized_keys_are_candidates() {
        let root = include_str!("../tests/fixtures/updater-test.pub")
            .trim()
            .to_owned();
        assert_eq!(
            keys(
                include_str!("../update-trust.json"),
                &root,
                "1.0.0",
                "stable"
            )
            .unwrap(),
            vec![root.clone()]
        );
        assert!(keys(
            r#"{"version":1,"rootValidUntil":"0.9.0","delegations":[]}"#,
            &root,
            "1.0.0",
            "stable"
        )
        .is_err());
        assert!(keys(
            include_str!("../update-trust.json"),
            &root,
            "1.0.0",
            "unknown"
        )
        .is_err());
        let invalid = serde_json::json!({"version":1,"rootValidUntil":null,"delegations":[{"id":"next","publicKey":root,"minVersion":"1.0.0","channels":["stable"],"authorization":STANDARD.encode(include_str!("../tests/fixtures/updater-message.txt.sig"))}]});
        assert!(keys(&invalid.to_string(), &root, "1.0.0", "stable").is_err());
    }
}
