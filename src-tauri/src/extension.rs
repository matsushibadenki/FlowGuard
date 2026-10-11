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
        let reply: BridgeStatus = serde_json::from_str(r#"{"protocolVersion":1,"requestID":"request","instanceID":"instance","revision":1,"status":"accepted","ruleCount":0}"#).unwrap();
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
    if status.protocol_version != 1
        || !["ready", "unavailable"].contains(&status.status.as_str())
        || status.instance_id.len() != 36
        || status.rule_count > 2000
        || !status.request_id.is_empty()
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
    // Demo intentions never enter the provider. Ask is rejected until decision UI is connected.
    let live: Vec<_> = rules.iter().filter(|rule| rule.source == "live").collect();
    if live
        .iter()
        .any(|rule| !crate::model::validate_rule(rule) || rule.action == "ask")
    {
        return Err("unsupported_or_invalid_rules".into());
    }
    let payload =
        serde_json::to_vec(&serde_json::json!({ "rules": live })).map_err(|e| e.to_string())?;
    if payload.len() > 4 * 1024 * 1024 {
        return Err("rule_payload_too_large".into());
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
    if reply.protocol_version != 1
        || reply.status != "accepted"
        || reply.error.is_some()
        || reply.rule_count != live.len()
    {
        return Err("invalid_bridge_reply".into());
    }
    Ok(reply)
}
