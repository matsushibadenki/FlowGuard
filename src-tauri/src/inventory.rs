use crate::discovery::{Device, DiscoverySnapshot, Plan};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub device: Device,
    pub previous: Option<Device>,
    pub kind: String,
    pub fields: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySnapshot {
    pub generation: u64,
    pub local_address: String,
    pub netmask: String,
    pub status: String,
    pub baseline_at: Option<u64>,
    pub rows: Vec<Change>,
    pub selected_id: Option<String>,
    pub panel: String,
}
impl Default for InventorySnapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            local_address: String::new(),
            netmask: String::new(),
            status: "first".into(),
            baseline_at: None,
            rows: vec![],
            selected_id: None,
            panel: "docked".into(),
        }
    }
}
#[derive(Default)]
pub struct Inventory {
    pub snapshot: InventorySnapshot,
    baseline: Option<(String, DiscoverySnapshot)>,
    scope: String,
}
fn scope(plan: &Plan) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        plan.local.interface,
        plan.local.address,
        plan.local.netmask,
        plan.range,
        plan.active,
        plan.name_mode
    )
}
fn fields(a: &Device, b: &Device) -> Vec<String> {
    let mut keys = vec![];
    if a.mac != b.mac {
        keys.push("mac");
    }
    if a.name.value != b.name.value
        || a.name.source != b.name.source
        || a.name.status != b.name.status
    {
        keys.push("name");
    }
    if a.response != b.response {
        keys.push("response");
    }
    if a.source != b.source {
        keys.push("source");
    }
    keys.into_iter().map(str::to_string).collect()
}
impl Inventory {
    pub fn begin(&mut self, plan: &Plan, generation: u64) {
        self.scope = scope(plan);
        self.snapshot.generation = generation;
        self.snapshot.local_address = plan.local.address.clone();
        self.snapshot.netmask = plan.local.netmask.clone();
        self.snapshot.status = "running".into();
        self.snapshot.rows.clear();
        self.snapshot.selected_id = None;
        self.snapshot.baseline_at = None;
    }
    pub fn update(&mut self, result: &DiscoverySnapshot) {
        let full = result.stage == "completed" && result.errors.is_empty();
        let prior = self
            .baseline
            .as_ref()
            .filter(|(scope, _)| *scope == self.scope);
        self.snapshot.status = if ["running", "cancelling"].contains(&result.stage.as_str()) {
            "running"
        } else if !full {
            "incomplete"
        } else if prior.is_some() {
            "ready"
        } else if self.baseline.is_some() {
            "scope_changed"
        } else {
            "first"
        }
        .into();
        self.snapshot.baseline_at = if full {
            prior.and_then(|(_, p)| p.observed_at)
        } else {
            None
        };
        self.snapshot.rows = result
            .devices
            .iter()
            .map(|device| Change {
                device: device.clone(),
                previous: None,
                kind: "uncompared".into(),
                fields: vec![],
            })
            .collect();
        if full {
            if let Some((_, before)) = prior {
                let mut old: BTreeMap<_, _> =
                    before.devices.iter().map(|d| (d.id.clone(), d)).collect();
                for row in &mut self.snapshot.rows {
                    if let Some(previous) = old.remove(&row.device.id) {
                        row.fields = fields(previous, &row.device);
                        row.kind = if row.fields.is_empty() {
                            "unchanged"
                        } else {
                            "changed"
                        }
                        .into();
                        row.previous = Some(previous.clone());
                    } else {
                        row.kind = "added".into();
                    }
                }
                self.snapshot.rows.extend(old.into_values().map(|d| Change {
                    device: d.clone(),
                    previous: Some(d.clone()),
                    kind: "not_seen".into(),
                    fields: vec![],
                }));
            }
            self.baseline = Some((self.scope.clone(), result.clone()));
        }
        if !self
            .snapshot
            .rows
            .iter()
            .any(|r| Some(&r.device.id) == self.snapshot.selected_id.as_ref())
        {
            self.snapshot.selected_id = None;
        }
    }
    pub fn invalidate(&mut self, result: &DiscoverySnapshot) {
        self.baseline = None;
        self.update(result);
    }
    pub fn select(&mut self, id: &str) -> Result<(), String> {
        if !self.snapshot.rows.iter().any(|r| r.device.id == id) {
            return Err("device_not_available".into());
        }
        self.snapshot.selected_id = Some(id.into());
        Ok(())
    }
}
fn cell(value: &str) -> String {
    let dangerous = value.starts_with(['\t', '\r', '\n'])
        || value.trim_start().starts_with(['=', '+', '-', '@']);
    format!(
        "\"{}{}\"",
        if dangerous { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}
pub fn csv(inventory: &InventorySnapshot, job: &DiscoverySnapshot) -> String {
    let headers = [
        "generation",
        "result_stage",
        "comparison_status",
        "baseline_at_unix",
        "interface",
        "local_address",
        "netmask",
        "range",
        "active_ping",
        "name_mode",
        "change",
        "changed_fields",
        "address",
        "family",
        "mac",
        "reported_name",
        "name_source",
        "name_status",
        "name_collected_at_unix",
        "response",
        "evidence_source",
        "collected_at_unix",
        "previous_mac",
        "previous_name",
        "previous_name_source",
        "previous_name_status",
        "previous_name_collected_at_unix",
        "previous_response",
        "previous_evidence_source",
        "previous_collected_at_unix",
        "result_errors",
    ];
    let mut text = String::from("\u{feff}");
    text.push_str(&headers.map(cell).join(","));
    text.push_str("\r\n");
    for row in &inventory.rows {
        let d = &row.device;
        let p = row.previous.as_ref();
        let values = vec![
            inventory.generation.to_string(),
            job.stage.clone(),
            inventory.status.clone(),
            inventory
                .baseline_at
                .map(|t| t.to_string())
                .unwrap_or_default(),
            d.interface.clone(),
            inventory.local_address.clone(),
            inventory.netmask.clone(),
            job.range.clone(),
            job.active.to_string(),
            job.name_mode.clone(),
            row.kind.clone(),
            row.fields.join("|"),
            d.address.clone(),
            d.family.clone(),
            d.mac.clone().unwrap_or_default(),
            d.name.value.clone().unwrap_or_default(),
            d.name.source.clone().unwrap_or_default(),
            d.name.status.clone(),
            d.name
                .observed_at
                .map(|t| t.to_string())
                .unwrap_or_default(),
            d.response.clone(),
            d.source.clone(),
            d.observed_at.to_string(),
            p.and_then(|d| d.mac.clone()).unwrap_or_default(),
            p.and_then(|d| d.name.value.clone()).unwrap_or_default(),
            p.and_then(|d| d.name.source.clone()).unwrap_or_default(),
            p.map(|d| d.name.status.clone()).unwrap_or_default(),
            p.and_then(|d| d.name.observed_at)
                .map(|t| t.to_string())
                .unwrap_or_default(),
            p.map(|d| d.response.clone()).unwrap_or_default(),
            p.map(|d| d.source.clone()).unwrap_or_default(),
            p.map(|d| d.observed_at.to_string()).unwrap_or_default(),
            job.errors.join("|"),
        ];
        text.push_str(&values.iter().map(|v| cell(v)).collect::<Vec<_>>().join(","));
        text.push_str("\r\n");
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> Plan {
        Plan::new(
            &crate::network::NetworkSnapshot {
                addresses: vec![crate::network::InterfaceAddress {
                    interface: "en0".into(),
                    address: "192.168.1.20".into(),
                    family: "IPv4".into(),
                    netmask: "255.255.255.0".into(),
                    up: true,
                }],
                ..Default::default()
            },
            "en0",
            "192.168.1.20",
            false,
        )
        .unwrap()
    }
    fn device(address: &str) -> Device {
        Device {
            id: format!("en0|{address}"),
            interface: "en0".into(),
            address: address.into(),
            family: "IPv4".into(),
            mac: None,
            source: "arp".into(),
            response: "unverified".into(),
            observed_at: 1,
            name: Default::default(),
        }
    }
    fn result(devices: Vec<Device>) -> DiscoverySnapshot {
        DiscoverySnapshot {
            devices,
            stage: "completed".into(),
            observed_at: Some(2),
            ..Default::default()
        }
    }
    #[test]
    fn compares_records_without_treating_timestamps_as_changes() {
        let mut i = Inventory::default();
        i.begin(&plan(), 1);
        i.update(&result(vec![
            device("192.168.1.1"),
            device("192.168.1.2"),
            device("192.168.1.3"),
        ]));
        i.begin(&plan(), 2);
        let mut same = device("192.168.1.1");
        same.observed_at = 10;
        let mut changed = device("192.168.1.2");
        changed.mac = Some("02:00:00:00:00:02".into());
        i.update(&result(vec![same, changed, device("192.168.1.4")]));
        assert_eq!(
            i.snapshot
                .rows
                .iter()
                .map(|r| r.kind.as_str())
                .collect::<Vec<_>>(),
            ["unchanged", "changed", "added", "not_seen"]
        );
        assert_eq!(i.snapshot.rows[1].fields, ["mac"]);
        assert_eq!(i.snapshot.rows[0].previous.as_ref().unwrap().observed_at, 1);
        assert_eq!(i.snapshot.rows[3].device.observed_at, 1);
        if let Ok(path) = std::env::var("FLOWGUARD_CSV_TEST_OUTPUT") {
            std::fs::write(path, csv(&i.snapshot, &result(vec![]))).unwrap();
        }
    }
    #[test]
    fn incomplete_results_never_promote_a_baseline_or_infer_missing_records() {
        let mut i = Inventory::default();
        i.begin(&plan(), 1);
        i.update(&result(vec![device("192.168.1.1")]));
        for stage in ["partial", "failed", "cancelled"] {
            i.begin(&plan(), 2);
            let mut partial = result(vec![]);
            partial.stage = stage.into();
            i.update(&partial);
            assert_eq!(i.snapshot.status, "incomplete");
            assert!(i.snapshot.rows.is_empty());
        }
        i.begin(&plan(), 3);
        i.update(&result(vec![]));
        assert_eq!(i.snapshot.status, "ready");
        assert_eq!(i.snapshot.rows[0].kind, "not_seen");
        let different = plan().with_names("mdns").unwrap();
        i.begin(&different, 4);
        i.update(&result(vec![device("192.168.1.2")]));
        assert_eq!(i.snapshot.status, "scope_changed");
        assert_eq!(i.snapshot.rows[0].kind, "uncompared");
    }
    #[test]
    fn csv_quotes_and_neutralizes_formula_cells_without_mutating_records() {
        assert_eq!(cell(" =SUM(1,2)"), "\"' =SUM(1,2)\"");
        assert_eq!(cell("a,\"b\""), "\"a,\"\"b\"\"\"");
        let mut d = device("192.168.1.2");
        d.name.value = Some("=悪意,\"名前\"".into());
        let mut i = Inventory::default();
        i.begin(&plan(), 1);
        i.update(&result(vec![d]));
        let text = csv(&i.snapshot, &result(vec![]));
        assert!(text.starts_with('\u{feff}'));
        assert!(text.contains("\"'=悪意,\"\"名前\"\"\""));
        assert!(text.ends_with("\r\n"));
        assert_eq!(
            i.snapshot.rows[0].device.name.value.as_deref(),
            Some("=悪意,\"名前\"")
        );
    }
}
