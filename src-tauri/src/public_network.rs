use serde::Serialize;
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub enabled: bool,
    pub busy: bool,
    pub operation: u64,
    pub ip: Option<String>,
    pub organization: Option<String>,
    pub country: Option<String>,
    pub observed_at: Option<u64>,
    pub error: bool,
    pub source: &'static str,
}
#[derive(Default)]
pub struct Machine {
    pub snapshot: Snapshot,
    active: Option<u64>,
    last: Option<std::time::Instant>,
}
impl Machine {
    pub fn configure(&mut self, enabled: bool, expected: u64) -> Result<(), String> {
        if expected != self.snapshot.operation {
            return Err("public_network_stale".into());
        }
        let operation = self.snapshot.operation + 1;
        self.snapshot = Snapshot {
            enabled,
            operation,
            busy: self.active.is_some(),
            source: "https://ipinfo.io/json",
            ..Default::default()
        };
        Ok(())
    }
    pub fn begin(&mut self, expected: u64) -> Result<u64, String> {
        if !self.snapshot.enabled
            || expected != self.snapshot.operation
            || self.active.is_some()
            || self
                .last
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30))
        {
            return Err("public_network_unavailable".into());
        }
        self.snapshot.operation += 1;
        let op = self.snapshot.operation;
        self.active = Some(op);
        self.last = Some(std::time::Instant::now());
        self.snapshot.busy = true;
        self.snapshot.error = false;
        self.snapshot.ip = None;
        self.snapshot.organization = None;
        self.snapshot.country = None;
        self.snapshot.observed_at = None;
        Ok(op)
    }
    pub fn finish(&mut self, op: u64, result: Result<Snapshot, String>) {
        if self.active != Some(op) {
            return;
        }
        self.active = None;
        self.snapshot.busy = false;
        if !self.snapshot.enabled || self.snapshot.operation != op {
            return;
        }
        match result {
            Ok(value) => {
                self.snapshot = Snapshot {
                    enabled: true,
                    busy: false,
                    operation: op,
                    ..value
                };
            }
            Err(_) => {
                self.snapshot.error = true;
            }
        }
    }
}
pub async fn fetch() -> Result<Snapshot, String> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(6))
        .user_agent("FlowGuard/0.1")
        .build()
        .map_err(|_| "public_network_failed")?;
    let mut response = client
        .get("https://ipinfo.io/json")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "public_network_failed")?;
    if !response.status().is_success() || response.content_length().is_some_and(|n| n > 8192) {
        return Err("public_network_failed".into());
    }
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "public_network_failed")?
    {
        if bytes.len() + chunk.len() > 8192 {
            return Err("public_network_limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    parse(&bytes)
}
fn parse(bytes: &[u8]) -> Result<Snapshot, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "public_network_invalid")?;
    let ip = value
        .get("ip")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<std::net::IpAddr>().ok())
        .filter(|ip| match ip {
            std::net::IpAddr::V4(a) => {
                !a.is_private()
                    && !a.is_loopback()
                    && !a.is_link_local()
                    && !a.is_unspecified()
                    && !a.is_multicast()
                    && !a.is_broadcast()
            }
            std::net::IpAddr::V6(a) => {
                !a.is_unique_local()
                    && !a.is_unicast_link_local()
                    && !a.is_unspecified()
                    && !a.is_loopback()
                    && !a.is_multicast()
            }
        })
        .ok_or("public_network_invalid")?;
    let clean = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && s.len() <= 160 && !s.chars().any(char::is_control))
            .map(str::to_owned)
    };
    let country =
        clean("country").filter(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase()));
    Ok(Snapshot {
        ip: Some(ip.to_string()),
        organization: clean("org"),
        country,
        observed_at: Some(crate::model::now()),
        source: "https://ipinfo.io/json",
        ..Default::default()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opt_in_only_revocation_fences_replies_and_throttles() {
        let mut m = Machine::default();
        assert!(m.begin(0).is_err());
        m.configure(true, 0).unwrap();
        let op = m.begin(1).unwrap();
        m.configure(false, op).unwrap();
        assert!(m.active.is_some());
        m.finish(
            op,
            parse(br#"{"ip":"1.1.1.1","org":"AS123 Candidate","country":"JP","loc":"discarded"}"#),
        );
        assert!(!m.snapshot.enabled);
        assert!(m.snapshot.ip.is_none());
        assert!(!m.active.is_some());
        m.configure(true, m.snapshot.operation).unwrap();
        assert!(m.begin(m.snapshot.operation).is_err());
        assert!(parse(br#"{"ip":"127.0.0.1"}"#).is_err());
        assert!(parse(br#"{"ip":"bad"}"#).is_err());
    }
}
