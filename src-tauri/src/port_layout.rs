use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
    Left,
    Right,
    #[default]
    Bottom,
    Floating,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceKind {
    Ports,
    Services,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Geometry {
    pub width: u16,
}
impl Default for Geometry {
    fn default() -> Self {
        Self { width: 380 }
    }
}
impl Geometry {
    pub fn resize(&mut self, width: u16, expected: u16) -> Result<(), String> {
        if !(280..=600).contains(&width) {
            return Err("invalid_evidence_width".into());
        }
        if self.width != expected {
            return Err("evidence_width_conflict".into());
        }
        self.width = width;
        Ok(())
    }
}

pub const COLUMNS: [&str; 8] = [
    "target", "port", "change", "status", "previous", "duration", "time", "inspect",
];
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layout {
    pub columns: Vec<String>,
    pub widths: BTreeMap<String, u16>,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            columns: COLUMNS.iter().map(|c| c.to_string()).collect(),
            widths: COLUMNS
                .iter()
                .map(|c| {
                    (
                        c.to_string(),
                        match *c {
                            "target" | "previous" | "time" => 190,
                            "inspect" => 80,
                            _ => 130,
                        },
                    )
                })
                .collect(),
        }
    }
}
impl Layout {
    pub fn validate(&self) -> Result<(), String> {
        let ordered: Vec<String> = COLUMNS
            .iter()
            .filter(|c| self.columns.iter().any(|s| s == **c))
            .map(|c| c.to_string())
            .collect();
        if self.columns != ordered
            || !["port", "status", "inspect"]
                .iter()
                .all(|c| self.columns.iter().any(|s| s == c))
            || self.widths.len() != COLUMNS.len()
            || !COLUMNS
                .iter()
                .all(|c| self.widths.get(*c).is_some_and(|w| (80..=400).contains(w)))
        {
            return Err("invalid_port_layout".into());
        }
        Ok(())
    }
    pub fn replace(&mut self, expected: &Self, value: Self) -> Result<(), String> {
        value.validate()?;
        if self != expected {
            return Err("port_layout_conflict".into());
        }
        *self = value;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_width_is_bounded_and_stale_edits_never_overwrite() {
        let mut value = Geometry::default();
        assert_eq!(value.width, 380);
        for width in [0, 279, 601, u16::MAX] {
            assert!(value.resize(width, 380).is_err());
            assert_eq!(value.width, 380);
        }
        value.resize(600, 380).unwrap();
        assert_eq!(
            value.resize(280, 380).unwrap_err(),
            "evidence_width_conflict"
        );
        assert_eq!(value.width, 600);
        value.resize(280, 600).unwrap();
        assert!(serde_json::from_str::<EvidenceKind>("\"other\"").is_err());
    }
    #[test]
    fn bounded_canonical_columns_keep_result_and_selection_available() {
        let layout = Layout::default();
        assert!(layout.validate().is_ok());
        let mut compact = layout.clone();
        compact.columns = vec!["port".into(), "status".into(), "inspect".into()];
        assert!(compact.validate().is_ok());
        for columns in [
            vec![],
            vec!["port", "inspect"],
            vec!["status", "port", "inspect"],
            vec!["port", "status", "inspect", "inspect"],
            vec!["port", "status", "inspect", "unknown"],
        ] {
            let mut bad = layout.clone();
            bad.columns = columns.iter().map(|s| s.to_string()).collect();
            assert!(bad.validate().is_err());
        }
        for width in [0, 79, 401, u16::MAX] {
            let mut bad = layout.clone();
            bad.widths.insert("port".into(), width);
            assert!(bad.validate().is_err());
        }
        let mut bad = layout.clone();
        bad.widths.remove("time");
        assert!(bad.validate().is_err());
        let mut bad = layout.clone();
        bad.widths.insert("unknown".into(), 100);
        assert!(bad.validate().is_err());
    }
    #[test]
    fn stale_window_cannot_overwrite_a_newer_layout() {
        let mut layout = Layout::default();
        let before = layout.clone();
        let mut next = before.clone();
        next.widths.insert("target".into(), 400);
        layout.replace(&before, next.clone()).unwrap();
        assert_eq!(
            layout.replace(&before, before.clone()).unwrap_err(),
            "port_layout_conflict"
        );
        assert_eq!(layout, next);
        let mut invalid = next.clone();
        invalid.columns.clear();
        assert!(layout.replace(&next, invalid).is_err());
        assert_eq!(layout, next);
    }
}
