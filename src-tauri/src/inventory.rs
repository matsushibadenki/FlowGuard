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
    pub checked_ids: Vec<String>,
    pub comments: BTreeMap<String, String>,
    pub columns: Vec<String>,
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
            checked_ids: vec![],
            comments: BTreeMap::new(),
            columns: COLUMNS.iter().map(|s| s.to_string()).collect(),
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
pub const COLUMNS: &[&str] = &[
    "change",
    "inspect",
    "address",
    "family",
    "interface",
    "mac",
    "name",
    "nameSource",
    "response",
    "source",
    "observed",
    "comment",
];
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Edit {
    Selection {
        ids: Vec<String>,
        expected: Vec<String>,
    },
    Comment {
        id: String,
        value: String,
        expected: String,
    },
    Columns {
        values: Vec<String>,
        expected: Vec<String>,
    },
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
        self.snapshot.checked_ids.clear();
        self.snapshot.comments.clear();
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
        self.snapshot
            .checked_ids
            .retain(|id| self.snapshot.rows.iter().any(|r| &r.device.id == id));
        self.snapshot
            .comments
            .retain(|id, _| self.snapshot.rows.iter().any(|r| &r.device.id == id));
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
    pub fn edit(&mut self, generation: u64, command: Edit) -> Result<(), String> {
        if self.snapshot.generation != generation || self.snapshot.status == "running" {
            return Err("lan_stale".into());
        }
        match command {
            Edit::Selection { mut ids, expected } => {
                if expected != self.snapshot.checked_ids {
                    return Err("lan_conflict".into());
                }
                if ids.len() > 2048
                    || ids
                        .iter()
                        .any(|id| !self.snapshot.rows.iter().any(|r| &r.device.id == id))
                {
                    return Err("lan_stale".into());
                }
                ids.sort();
                ids.dedup();
                self.snapshot.checked_ids = ids;
            }
            Edit::Comment {
                id,
                value,
                expected,
            } => {
                if !self.snapshot.rows.iter().any(|r| r.device.id == id) {
                    return Err("lan_stale".into());
                }
                if self
                    .snapshot
                    .comments
                    .get(&id)
                    .map(String::as_str)
                    .unwrap_or("")
                    != expected
                {
                    return Err("lan_conflict".into());
                }
                if value.chars().count() > 500
                    || value.len() > 2000
                    || value.chars().any(|c| c.is_control() && c != '\n')
                {
                    return Err("lan_comment_invalid".into());
                }
                if value.is_empty() {
                    self.snapshot.comments.remove(&id);
                } else {
                    self.snapshot.comments.insert(id, value);
                }
            }
            Edit::Columns { values, expected } => {
                if expected != self.snapshot.columns {
                    return Err("lan_conflict".into());
                }
                let unique: std::collections::HashSet<_> = values.iter().collect();
                if values.len() > COLUMNS.len()
                    || unique.len() != values.len()
                    || !values.iter().any(|v| v == "address")
                    || !values.iter().any(|v| v == "inspect")
                    || values.iter().any(|v| !COLUMNS.contains(&v.as_str()))
                {
                    return Err("lan_columns_invalid".into());
                }
                self.snapshot.columns = values;
            }
        }
        Ok(())
    }
    pub fn selected_csv(&self, job: &DiscoverySnapshot) -> Result<String, String> {
        if self.snapshot.checked_ids.is_empty() {
            return Err("lan_selection_empty".into());
        }
        let mut selected = self.snapshot.clone();
        selected
            .rows
            .retain(|r| self.snapshot.checked_ids.contains(&r.device.id));
        if selected.rows.len() != self.snapshot.checked_ids.len() {
            return Err("lan_stale".into());
        }
        Ok(csv(&selected, job))
    }
    pub fn select(&mut self, id: &str) -> Result<(), String> {
        if !self.snapshot.rows.iter().any(|r| r.device.id == id) {
            return Err("device_not_available".into());
        }
        self.snapshot.selected_id = Some(id.into());
        Ok(())
    }
}
pub(crate) fn cell(value: &str) -> String {
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
        "comment",
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
            inventory.comments.get(&d.id).cloned().unwrap_or_default(),
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
    fn workbench_selection_comments_and_columns_are_scoped_and_conflict_checked() {
        let mut i = Inventory::default();
        i.begin(&plan(), 7);
        i.update(&result(vec![device("192.168.1.2"), device("192.168.1.3")]));
        let id = i.snapshot.rows[0].device.id.clone();
        i.edit(
            7,
            Edit::Selection {
                ids: vec![id.clone(), id.clone()],
                expected: vec![],
            },
        )
        .unwrap();
        assert_eq!(i.snapshot.checked_ids, vec![id.clone()]);
        assert!(i
            .edit(
                7,
                Edit::Selection {
                    ids: vec![],
                    expected: vec![]
                }
            )
            .is_err());
        i.edit(
            7,
            Edit::Comment {
                id: id.clone(),
                value: "=注記,\"a\"\n次の行".into(),
                expected: "".into(),
            },
        )
        .unwrap();
        assert!(i
            .edit(
                7,
                Edit::Comment {
                    id: id.clone(),
                    value: "stale".into(),
                    expected: "".into()
                }
            )
            .is_err());
        let text = i.selected_csv(&result(vec![])).unwrap();
        assert!(text.contains("192.168.1.2"));
        assert!(!text.contains("192.168.1.3"));
        assert!(text.contains("comment"));
        assert!(text.contains("name_status"));
        assert!(text.contains("collected_at_unix"));
        assert!(text.contains("'="));
        assert_eq!(i.snapshot.comments[&id], "=注記,\"a\"\n次の行");
        let columns = i.snapshot.columns.clone();
        i.edit(
            7,
            Edit::Columns {
                values: vec!["address".into(), "inspect".into()],
                expected: columns.clone(),
            },
        )
        .unwrap();
        assert!(i
            .edit(
                7,
                Edit::Columns {
                    values: columns.clone(),
                    expected: columns
                }
            )
            .is_err());
        i.begin(&plan(), 8);
        assert!(i.snapshot.checked_ids.is_empty());
        assert!(i.snapshot.comments.is_empty());
        assert_eq!(i.snapshot.columns, vec!["address", "inspect"]);
        assert!(i
            .edit(
                7,
                Edit::Selection {
                    ids: vec![],
                    expected: vec![]
                }
            )
            .is_err());
        assert!(i.selected_csv(&result(vec![])).is_err());
    }
    #[test]
    fn workbench_rejects_running_invalid_columns_and_oversized_comments() {
        let mut i = Inventory::default();
        i.begin(&plan(), 1);
        assert!(i
            .edit(
                1,
                Edit::Selection {
                    ids: vec![],
                    expected: vec![]
                }
            )
            .is_err());
        i.update(&result(vec![device("192.168.1.2")]));
        let id = i.snapshot.rows[0].device.id.clone();
        for values in [
            vec!["address".into()],
            vec!["address".into(), "inspect".into(), "address".into()],
            vec!["address".into(), "inspect".into(), "bogus".into()],
        ] {
            assert!(i
                .edit(
                    1,
                    Edit::Columns {
                        values,
                        expected: i.snapshot.columns.clone()
                    }
                )
                .is_err());
        }
        for value in ["a".repeat(501), "tab\t".into(), "bad\0".into()] {
            assert!(i
                .edit(
                    1,
                    Edit::Comment {
                        id: id.clone(),
                        value,
                        expected: "".into()
                    }
                )
                .is_err());
        }
        assert!(i
            .edit(
                1,
                Edit::Selection {
                    ids: vec!["missing".into()],
                    expected: vec![]
                }
            )
            .is_err());
        assert!(i.snapshot.comments.is_empty());
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
