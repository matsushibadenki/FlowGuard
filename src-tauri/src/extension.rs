use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPackageStatus {
    pub packaged: bool,
    pub bridge_available: bool,
    pub status: &'static str,
}
pub fn package_status(contents: &Path) -> ExtensionPackageStatus {
    let extension =
        contents.join("Library/SystemExtensions/com.flowguard.filter.systemextension/Contents");
    let packaged = extension.join("Info.plist").is_file()
        && extension.join("MacOS/FlowGuardSystemExtension").is_file();
    let bridge_available = contents.join("MacOS/FlowGuardBridge").is_file();
    ExtensionPackageStatus {
        packaged,
        bridge_available,
        // Package presence never means signed, approved, running or applied.
        status: if packaged && bridge_available {
            "packaged_unverified"
        } else {
            "not_packaged"
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_swift_acknowledgement_field_names() {
        let reply: BridgeStatus = serde_json::from_str(r#"{"protocolVersion":3,"requestID":"request","instanceID":"instance","revision":1,"status":"accepted","ruleCount":0}"#).unwrap();
        assert_eq!(reply.request_id, "request");
        assert_eq!(reply.instance_id, "instance");
        assert!(reply.error.is_none());
    }
    #[test]
    fn absent_and_partial_bundles_cannot_claim_readiness() {
        let root =
            std::env::temp_dir().join(format!("flowguard-bundle-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("MacOS")).unwrap();
        std::fs::write(root.join("MacOS/FlowGuardBridge"), "test").unwrap();
        let status = package_status(&root);
        assert!(!status.packaged);
        assert!(status.bridge_available);
        assert_eq!(status.status, "not_packaged");
        let extension =
            root.join("Library/SystemExtensions/com.flowguard.filter.systemextension/Contents");
        std::fs::create_dir_all(extension.join("MacOS")).unwrap();
        std::fs::write(extension.join("Info.plist"), "test").unwrap();
        std::fs::write(extension.join("MacOS/FlowGuardSystemExtension"), "test").unwrap();
        assert_eq!(package_status(&root).status, "packaged_unverified");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub protocol_version: u32,
    #[serde(rename = "requestID")]
    pub request_id: String,
    #[serde(rename = "instanceID")]
    pub instance_id: String,
    pub revision: u64,
    pub status: String,
    pub rule_count: usize,
    pub error: Option<String>,
    #[serde(default)]
    pub policy_digest: Option<String>,
}
// Fixed bundled helper and read-only operation; caller cannot choose a executable or service.
pub fn bridge_status(contents: &Path) -> Result<BridgeStatus, String> {
    let package = package_status(contents);
    if !package.packaged || !package.bridge_available {
        return Err("extension_not_packaged".into());
    }
    let binary = contents.join("MacOS/FlowGuardBridge");
    let binary = binary.to_str().ok_or("invalid_bundle_path")?;
    let (success, output) = crate::observer::run_bounded(binary, &["--status"])?;
    if !success {
        return Err("signed_host_or_active_extension_required".into());
    }
    let status: BridgeStatus = serde_json::from_str(&output).map_err(|_| "invalid_bridge_reply")?;
    if status.protocol_version != 3
        || !["ready", "unavailable"].contains(&status.status.as_str())
        || status.instance_id.len() != 36
        || status.rule_count > 2000
        || !status.request_id.is_empty()
        || status.error.is_some()
        || (status.status == "ready"
            && !status
                .policy_digest
                .as_ref()
                .is_some_and(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit())))
    {
        return Err("invalid_bridge_reply".into());
    }
    Ok(status)
}

pub fn sync_rules(contents: &Path, rules: &[crate::model::Rule]) -> Result<BridgeStatus, String> {
    let package = package_status(contents);
    if !package.packaged || !package.bridge_available {
        return Err("extension_not_packaged".into());
    }
    let payload = policy_payload(rules)?;
    let live: Vec<_> = rules
        .iter()
        .filter(|r| crate::model::enforceable(r))
        .collect();
    // Authenticated status plus an exact digest resolves an acknowledgement lost to timeout.
    let status = bridge_status(contents)?;
    if status.status == "ready"
        && status.error.is_none()
        && status.policy_digest.as_deref() == Some(&policy_digest(rules))
    {
        return Ok(status);
    }
    let helper = contents.join("MacOS/FlowGuardBridge");
    let (success, output) = crate::observer::run_bounded_input(
        helper.to_str().ok_or("invalid_bundle_path")?,
        &["--sync"],
        Some(payload),
    )?;
    if !success {
        return Err("signed_host_or_active_extension_required".into());
    }
    let reply: BridgeStatus = serde_json::from_str(&output).map_err(|_| "invalid_bridge_reply")?;
    // Swift host validates request ID, provider instance ID and exact next revision.
    if reply.protocol_version != 3
        || reply.status != "accepted"
        || reply.error.is_some()
        || reply.rule_count != live.len()
        || reply.policy_digest.as_deref() != Some(&policy_digest(rules))
    {
        return Err("invalid_bridge_reply".into());
    }
    Ok(reply)
}

/// Validation runs before activation can reach any OS lifecycle API.
pub fn policy_payload(rules: &[crate::model::Rule]) -> Result<Vec<u8>, String> {
    if rules
        .iter()
        .any(|r| r.source == "live" && !crate::model::validate_rule(r))
    {
        return Err("unsupported_or_invalid_rules".into());
    }
    // Demo intentions never enter the provider. Ask is rejected until decision UI is connected.
    let live: Vec<_> = rules
        .iter()
        .filter(|rule| crate::model::enforceable(rule))
        .collect();
    if live.iter().any(|rule| !crate::model::validate_rule(rule)) {
        return Err("unsupported_or_invalid_rules".into());
    }
    let payload =
        serde_json::to_vec(&serde_json::json!({ "rules": live })).map_err(|e| e.to_string())?;
    if payload.len() > 4 * 1024 * 1024 {
        return Err("rule_payload_too_large".into());
    }
    Ok(payload)
}

pub fn policy_digest(rules: &[crate::model::Rule]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"FlowGuardPolicy/v3");
    for rule in rules.iter().filter(|r| crate::model::enforceable(r)) {
        for field in [
            &rule.id,
            &rule.app_key,
            &rule.app,
            &rule.destination,
            &rule.port.to_string(),
            &rule.direction,
            &rule.action,
            &rule.updated_at.to_string(),
            &rule.source,
            &rule.identity.as_ref().unwrap().authority,
            &rule.identity.as_ref().unwrap().team_id,
            &rule.identity.as_ref().unwrap().identifier,
            &rule.timeout_action,
        ] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod digest_tests {
    #[test]
    fn empty_policy_matches_swift_contract() {
        assert_eq!(
            super::policy_digest(&[]),
            "dc4588f5d4949ea04f5b1c296dad6f24a990246a4e3139191bed4789eabc4e32"
        );
    }
    #[test]
    fn nonempty_policy_matches_swift_and_changes_with_content() {
        let rules: Vec<crate::model::Rule> = serde_json::from_str(r#"[{"id": "one", "appKey": "signed:v1:developer:ABCDE12345:com.test.app", "identity":{"authority":"developer","teamId":"ABCDE12345","identifier":"com.test.app"}, "app": "Test", "destination": "1.2.3.4", "port": 443, "direction": "both", "action": "block", "updatedAt": 1, "source": "live"}]"#).unwrap();
        assert_eq!(
            super::policy_digest(&rules),
            "c9e3e7608370bec8f372ebd9a7da81df695a2bebe2bd5651bc476b38a617ba60"
        );
        let mut changed = rules.clone();
        changed[0].action = "allow".into();
        assert_ne!(super::policy_digest(&rules), super::policy_digest(&changed));
    }
}
