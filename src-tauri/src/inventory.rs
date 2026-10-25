use crate::discovery::{Device, DiscoverySnapshot, Plan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub device: Device,
    pub previous: Option<Device>,
    pub kind: String,
    pub fields: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySnapshot {
    #[serde(skip_deserializing)]
    pub public_info: crate::public_network::Snapshot,
    pub vendors: BTreeMap<String, crate::lan_vendor::Vendor>,
    pub widths: BTreeMap<String, u16>,
    pub labels: BTreeMap<String, String>,
    pub first_seen: BTreeMap<String, u64>,
    pub archives: Vec<ArchiveSummary>,
    pub archive_error: bool,
    pub candidates: BTreeMap<String, Vec<String>>,
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
            public_info: Default::default(),
            vendors: BTreeMap::new(),
            widths: COLUMNS
                .iter()
                .map(|c| (c.to_string(), default_width(c)))
                .collect(),
            labels: BTreeMap::new(),
            first_seen: BTreeMap::new(),
            archives: vec![],
            archive_error: false,
            candidates: BTreeMap::new(),
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
    "address",
    "inspect",
    "change",
    "family",
    "interface",
    "mac",
    "name",
    "nameSource",
    "response",
    "source",
    "observed",
    "comment",
    "label",
    "scope",
    "latency",
    "firstSeen",
    "vendor",
    "services",
    "smb",
];
fn default_width(column: &str) -> u16 {
    match column {
        "inspect" | "family" => 80,
        "change" | "latency" => 120,
        _ => 160,
    }
}
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Edit {
    Width {
        column: String,
        value: u16,
        expected: u16,
    },
    Label {
        id: String,
        value: String,
        expected: String,
    },
    Archive {},
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
    pub public: crate::public_network::Machine,
    archives: Vec<Archive>,
    pub snapshot: InventorySnapshot,
    baseline: Option<(String, DiscoverySnapshot)>,
    scope: String,
    ledger: BTreeMap<String, u64>,
    network_scope: String,
}
fn scope(plan: &Plan) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        plan.local.interface,
        plan.local.address,
        plan.local.netmask,
        plan.range,
        plan.active,
        format!("{}|{:?}|{:?}", plan.name_mode, plan.options, plan.context)
    )
}
fn fields(a: &Device, b: &Device) -> Vec<String> {
    let mut keys = vec![];
    if a.mac != b.mac {
        keys.push("mac");
    }
    let names = |d: &Device| {
        let mut values = d
            .name
            .values
            .iter()
            .map(|v| v.to_ascii_lowercase())
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
    };
    if names(a) != names(b)
        || a.name.value.as_ref().map(|v| v.to_ascii_lowercase())
            != b.name.value.as_ref().map(|v| v.to_ascii_lowercase())
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
        let network_scope = format!(
            "{}|{}|{}|{:?}",
            plan.local.interface, plan.local.address, plan.local.netmask, plan.context
        );
        if self.network_scope != network_scope {
            self.ledger.clear();
            self.network_scope = network_scope;
        }
        self.snapshot.first_seen.clear();
        self.snapshot.candidates.clear();
        self.snapshot.labels.clear();
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
        for row in &self.snapshot.rows {
            if row.kind == "not_seen" {
                continue;
            }
            let key = format!(
                "{}|{}",
                row.device.id,
                row.device.mac.as_deref().unwrap_or("")
            );
            if self.ledger.len() < 2048 || self.ledger.contains_key(&key) {
                let first = *self.ledger.entry(key).or_insert(row.device.observed_at);
                self.snapshot
                    .first_seen
                    .insert(row.device.id.clone(), first);
            }
        }
        self.snapshot.vendors = self
            .snapshot
            .rows
            .iter()
            .filter_map(|r| {
                Some((
                    r.device.id.clone(),
                    crate::lan_vendor::candidate(r.device.mac.as_deref()?)?,
                ))
            })
            .collect();
        self.snapshot.candidates.clear();
        if full {
            if let Some(previous) = self.archives.iter().find(|a| {
                a.interface == result.interface
                    && a.local_address == self.snapshot.local_address
                    && a.netmask == self.snapshot.netmask
                    && a.context == result.context
            }) {
                for row in &self.snapshot.rows {
                    if row.kind == "not_seen" {
                        continue;
                    }
                    let Some(mac) = row.device.mac.as_deref() else {
                        continue;
                    };
                    // Locally administered/randomized MACs are excluded; candidates never merge records.
                    if u8::from_str_radix(mac.split(':').next().unwrap_or(""), 16)
                        .map_or(true, |b| b & 3 != 0)
                    {
                        continue;
                    }
                    let candidates = previous
                        .rows
                        .iter()
                        .filter(|old| {
                            old.kind != "not_seen"
                                && old.device.mac.as_deref() == Some(mac)
                                && old.device.address != row.device.address
                        })
                        .take(4)
                        .map(|old| old.device.address.clone())
                        .collect::<Vec<_>>();
                    if !candidates.is_empty() {
                        self.snapshot
                            .candidates
                            .insert(row.device.id.clone(), candidates);
                    }
                }
            }
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
            Edit::Archive {} => return Err("archive_requires_snapshot".into()),
            Edit::Width {
                column,
                value,
                expected,
            } => {
                if !COLUMNS.contains(&column.as_str()) || !(80..=480).contains(&value) {
                    return Err("lan_width_invalid".into());
                }
                if self
                    .snapshot
                    .widths
                    .get(&column)
                    .copied()
                    .unwrap_or(default_width(&column))
                    != expected
                {
                    return Err("lan_conflict".into());
                }
                self.snapshot.widths.insert(column, value);
            }
            Edit::Label {
                id,
                value,
                expected,
            } => {
                if !self.snapshot.rows.iter().any(|r| r.device.id == id)
                    || value.chars().count() > 80
                    || value.chars().any(char::is_control)
                {
                    return Err("lan_label_invalid".into());
                }
                if self
                    .snapshot
                    .labels
                    .get(&id)
                    .map(String::as_str)
                    .unwrap_or("")
                    != expected
                {
                    return Err("lan_conflict".into());
                }
                if value.is_empty() {
                    self.snapshot.labels.remove(&id);
                } else {
                    self.snapshot.labels.insert(id, value);
                }
            }

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
        "display_label",
        "first_seen_unix",
        "ping_rtt_ms",
        "address_candidates_unverified",
        "oui_assignment_holder",
        "oui_source",
        "oui_retrieved_date",
        "reported_names",
        "advertised_services_unverified",
        "smb_names_unverified",
        "smb_domains_unverified",
        "service_metadata_status",
        "smb_metadata_status",
        "metadata_source",
        "metadata_at_unix",
        "gateway",
        "ssid",
        "concurrency",
        "timeout_ms",
        "interval_ms",
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
            inventory.labels.get(&d.id).cloned().unwrap_or_default(),
            inventory
                .first_seen
                .get(&d.id)
                .map(u64::to_string)
                .unwrap_or_default(),
            job.response_ms
                .get(&d.id)
                .map(f64::to_string)
                .unwrap_or_default(),
            inventory
                .candidates
                .get(&d.id)
                .map(|v| v.join("|"))
                .unwrap_or_default(),
            inventory
                .vendors
                .get(&d.id)
                .map(|v| v.organization.clone())
                .unwrap_or_default(),
            inventory
                .vendors
                .get(&d.id)
                .map(|v| v.source.clone())
                .unwrap_or_default(),
            inventory
                .vendors
                .get(&d.id)
                .map(|v| v.retrieved_at.clone())
                .unwrap_or_default(),
            d.name.values.join("|"),
            job.metadata
                .get(&d.id)
                .map(|m| {
                    m.services
                        .iter()
                        .map(|s| format!("{}:{}:{}:{}", s.service, s.instance, s.target, s.port))
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .unwrap_or_default(),
            job.metadata
                .get(&d.id)
                .map(|m| m.smb_names.join("|"))
                .unwrap_or_default(),
            job.metadata
                .get(&d.id)
                .map(|m| m.smb_domains.join("|"))
                .unwrap_or_default(),
            job.metadata
                .get(&d.id)
                .map(|m| m.service_status.clone())
                .unwrap_or_default(),
            job.metadata
                .get(&d.id)
                .map(|m| m.smb_status.clone())
                .unwrap_or_default(),
            job.metadata
                .get(&d.id)
                .map(|m| m.source.clone())
                .unwrap_or_default(),
            job.metadata
                .get(&d.id)
                .map(|m| m.observed_at.to_string())
                .unwrap_or_default(),
            job.context.gateway.clone().unwrap_or_default(),
            job.context.ssid.clone().unwrap_or_default(),
            job.options.concurrency.to_string(),
            job.options.timeout_ms.to_string(),
            job.options.interval_ms.to_string(),
        ];
        text.push_str(&values.iter().map(|v| cell(v)).collect::<Vec<_>>().join(","));
        text.push_str("\r\n");
    }
    text
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Archive {
    #[serde(default)]
    pub metadata: BTreeMap<String, crate::lan_metadata::Metadata>,
    #[serde(default)]
    pub options: crate::discovery::ScanOptions,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub name_mode: String,
    pub generation: u64,
    pub observed_at: u64,
    pub interface: String,
    pub local_address: String,
    pub netmask: String,
    pub range: String,
    #[serde(default)]
    pub context: crate::network::LanContext,
    pub rows: Vec<Change>,
    pub labels: BTreeMap<String, String>,
    pub comments: BTreeMap<String, String>,
    pub response_ms: BTreeMap<String, f64>,
    #[serde(default)]
    pub vendors: BTreeMap<String, crate::lan_vendor::Vendor>,
    #[serde(default)]
    pub first_seen: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSummary {
    pub generation: u64,
    pub observed_at: u64,
    pub interface: String,
    pub local_address: String,
    pub range: String,
    pub row_count: usize,
}
impl Inventory {
    fn sync_archives(&mut self) {
        self.snapshot.archives = self
            .archives
            .iter()
            .map(|a| ArchiveSummary {
                generation: a.generation,
                observed_at: a.observed_at,
                interface: a.interface.clone(),
                local_address: a.local_address.clone(),
                range: a.range.clone(),
                row_count: a.rows.len(),
            })
            .collect();
    }
    pub fn get_archive(&self, generation: u64, observed_at: u64) -> Result<Archive, String> {
        self.archives
            .iter()
            .find(|a| a.generation == generation && a.observed_at == observed_at)
            .cloned()
            .ok_or_else(|| "lan_archive_unavailable".into())
    }
    pub fn load_archives(path: &std::path::Path) -> Self {
        let mut value = Self::default();
        let path = path.with_extension("lan-history.json");
        if !path.exists() {
            return value;
        }
        let loaded = (|| -> Option<Vec<Archive>> {
            if std::fs::metadata(&path).ok()?.len() > 8 * 1024 * 1024 {
                return None;
            }
            let records: Vec<Archive> = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
            if records.len() > 8
                || records.iter().any(|a| {
                    a.rows.len() > 2048
                        || a.labels.len() > 2048
                        || a.comments.len() > 2048
                        || a.metadata.len() > 1024
                        || a.vendors.len() > 2048
                        || a.first_seen.len() > 2048
                        || a.response_ms.len() > 1024
                        || a.labels
                            .values()
                            .any(|v| v.chars().count() > 80 || v.chars().any(char::is_control))
                        || a.comments.values().any(|v| {
                            v.chars().count() > 500
                                || v.len() > 2000
                                || v.chars().any(|c| c.is_control() && c != '\n')
                        })
                        || a.metadata.values().any(|m| {
                            m.services.len() > 8 || m.smb_names.len() > 8 || m.smb_domains.len() > 8
                        })
                        || a.response_ms
                            .values()
                            .any(|v| !v.is_finite() || !(0.0..=60000.0).contains(v))
                        || a.rows.iter().any(|r| {
                            r.device.id.len() > 512
                                || r.device.name.values.len() > 8
                                || r.device
                                    .name
                                    .values
                                    .iter()
                                    .any(|v| v.len() > 253 || v.chars().any(char::is_control))
                                || r.device.address.len() > 128
                                || r.device.name.value.as_ref().is_some_and(|v| v.len() > 1024)
                        })
                })
            {
                return None;
            }
            Some(records)
        })();
        if let Some(records) = loaded {
            value.archives = records;
            value.sync_archives();
        } else {
            value.snapshot.archive_error = true;
        }
        value
    }
    pub fn archive(
        &mut self,
        job: &DiscoverySnapshot,
        path: &std::path::Path,
    ) -> Result<(), String> {
        if job.stage != "completed" || !job.errors.is_empty() || self.snapshot.rows.is_empty() {
            return Err("lan_archive_incomplete".into());
        }
        let mut records = self.archives.clone();
        // Generations restart after app exit, so a saved run uses its timestamp as well.
        records.retain(|a| {
            a.generation != self.snapshot.generation
                || a.observed_at != job.observed_at.unwrap_or(0)
        });
        records.insert(
            0,
            Archive {
                metadata: job.metadata.clone(),
                options: job.options.clone(),
                active: job.active,
                name_mode: job.name_mode.clone(),
                generation: self.snapshot.generation,
                observed_at: job.observed_at.ok_or("lan_archive_incomplete")?,
                interface: job.interface.clone(),
                local_address: self.snapshot.local_address.clone(),
                netmask: self.snapshot.netmask.clone(),
                range: job.range.clone(),
                context: job.context.clone(),
                rows: self.snapshot.rows.clone(),
                labels: self.snapshot.labels.clone(),
                comments: self.snapshot.comments.clone(),
                response_ms: job.response_ms.clone(),
                vendors: self.snapshot.vendors.clone(),
                first_seen: self.snapshot.first_seen.clone(),
            },
        );
        records.truncate(8);
        let bytes = serde_json::to_vec(&records).map_err(|_| "lan_archive_save")?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("lan_archive_limit".into());
        }
        let target = path.with_extension("lan-history.json");
        let temp = path.with_extension("lan-history.tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        let mut file = options.open(&temp).map_err(|_| "lan_archive_save")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "lan_archive_save")?;
        std::fs::rename(temp, target).map_err(|_| "lan_archive_save")?;
        self.archives = records;
        self.sync_archives();
        self.snapshot.archive_error = false;
        Ok(())
    }
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
    fn extended_csv_has_matching_provenance_columns_and_preserves_zero_rtt() {
        let mut i = Inventory::default();
        i.begin(&plan(), 1);
        let mut job = result(vec![device("192.168.1.2")]);
        let id = job.devices[0].id.clone();
        job.response_ms.insert(id.clone(), 0.0);
        job.context.gateway = Some("192.168.1.1".into());
        i.update(&job);
        let text = csv(&i.snapshot, &job);
        let lines = text.trim_end().split("\r\n").collect::<Vec<_>>();
        let headers = lines[0].split(',').collect::<Vec<_>>();
        let values = lines[1].split(',').collect::<Vec<_>>();
        assert_eq!(headers.len(), values.len());
        let latency = headers
            .iter()
            .position(|v| v.contains("ping_rtt_ms"))
            .unwrap();
        assert_eq!(values[latency], "\"0\"");
        let gateway = headers.iter().position(|v| v.contains("gateway")).unwrap();
        assert_eq!(values[gateway], "\"192.168.1.1\"");
    }
    #[test]
    fn archive_survives_restart_is_bounded_and_does_not_merge_addresses() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("document.json");
        let mut i = Inventory::default();
        i.begin(&plan(), 1);
        let mut a = device("192.168.1.2");
        a.mac = Some("00:0c:29:00:00:01".into());
        let job = result(vec![a.clone()]);
        i.update(&job);
        let id = a.id.clone();
        i.edit(
            1,
            Edit::Label {
                id: id.clone(),
                value: "Router".into(),
                expected: String::new(),
            },
        )
        .unwrap();
        i.edit(
            1,
            Edit::Width {
                column: "address".into(),
                value: 220,
                expected: 160,
            },
        )
        .unwrap();
        assert!(i
            .edit(
                1,
                Edit::Width {
                    column: "address".into(),
                    value: 120,
                    expected: 160
                }
            )
            .is_err());
        i.archive(&job, &path).unwrap();
        let mut restored = Inventory::load_archives(&path);
        assert_eq!(restored.archives[0].labels[&id], "Router");
        assert!(restored.snapshot.rows.is_empty());
        restored.begin(&plan(), 2);
        a.address = "192.168.1.3".into();
        a.id = "en0|192.168.1.3".into();
        restored.update(&result(vec![a.clone()]));
        assert_eq!(restored.snapshot.candidates[&a.id], vec!["192.168.1.2"]);
        assert!(restored.snapshot.labels.is_empty());
        for n in 2..13 {
            restored.snapshot.generation = n;
            restored.archive(&result(vec![a.clone()]), &path).unwrap();
        }
        assert_eq!(restored.snapshot.archives.len(), 8);
        std::fs::create_dir(path.with_extension("lan-history.tmp")).unwrap();
        let saved = std::fs::read(path.with_extension("lan-history.json")).unwrap();
        assert!(restored.archive(&result(vec![a.clone()]), &path).is_err());
        assert_eq!(
            std::fs::read(path.with_extension("lan-history.json")).unwrap(),
            saved
        );
        std::fs::remove_dir(path.with_extension("lan-history.tmp")).unwrap();
        let before = restored.snapshot.archives.len();
        let mut stopped = result(vec![a]);
        stopped.stage = "cancelled".into();
        assert!(restored.archive(&stopped, &path).is_err());
        assert_eq!(restored.snapshot.archives.len(), before);
        std::fs::write(path.with_extension("lan-history.json"), b"invalid").unwrap();
        assert!(Inventory::load_archives(&path).snapshot.archive_error);
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
