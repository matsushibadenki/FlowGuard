//! Session-only connection history. This does not control active observations or other logs.
use crate::model::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Retention {
    #[default]
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "15m")]
    Minutes15,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "24h")]
    Hours24,
}
impl Retention {
    fn seconds(self) -> Option<u64> {
        match self {
            Self::Session => None,
            Self::Off => Some(0),
            Self::Minutes15 => Some(900),
            Self::Hour1 => Some(3600),
            Self::Hours24 => Some(86400),
        }
    }
    pub fn prune(self, rows: &mut Vec<Connection>, now: u64) {
        if self == Self::Off {
            rows.clear();
        } else {
            if let Some(seconds) = self.seconds() {
                rows.retain(|c| c.observed_at <= now && now - c.observed_at < seconds);
            }
            rows.truncate(500);
        }
    }
    pub fn visible(self, rows: &[Connection], now: u64) -> Vec<Connection> {
        let mut rows = rows.to_vec();
        self.prune(&mut rows, now);
        rows
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(at: u64) -> Connection {
        Connection {
            observed_at: at,
            ..crate::model::demo_connections().remove(0)
        }
    }
    #[test]
    fn bounds_expiry_future_times_and_disabled_history() {
        let rows = vec![row(1000), row(101), row(100), row(1001)];
        assert_eq!(Retention::Minutes15.visible(&rows, 1000).len(), 2);
        assert!(Retention::Off.visible(&rows, 1000).is_empty());
        assert_eq!(
            Retention::Session.visible(&vec![row(0); 600], 1000).len(),
            500
        );
        for (policy, span) in [(Retention::Hour1, 3600), (Retention::Hours24, 86400)] {
            assert_eq!(policy.visible(&[row(1), row(2)], span + 1).len(), 1);
        }
        assert!(Retention::Minutes15.visible(&[row(u64::MAX)], 0).is_empty());
    }
    #[test]
    fn serialized_choices_are_closed_and_round_trip() {
        for choice in ["session", "off", "15m", "1h", "24h"] {
            let p: Retention = serde_json::from_value(serde_json::json!(choice)).unwrap();
            assert_eq!(serde_json::to_value(p).unwrap(), choice);
        }
        for value in [
            serde_json::json!("forever"),
            serde_json::json!(0),
            serde_json::Value::Null,
        ] {
            assert!(serde_json::from_value::<Retention>(value).is_err());
        }
    }
}
