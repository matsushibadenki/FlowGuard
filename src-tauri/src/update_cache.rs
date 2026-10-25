//! One bounded resumable artifact, always re-verified against the current signed release.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub const LIMIT: usize = 200 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub version: String,
    pub url: String,
    pub signature: String,
    pub architecture: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Meta {
    identity: Identity,
    etag: String,
}
pub struct Cache {
    _lock: std::fs::File,
    root: PathBuf,
    identity: Identity,
    pub etag: Option<String>,
}
fn regular(path: &Path, limit: u64) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() <= limit)
}
fn open_private(path: &Path, truncate: bool) -> Result<std::fs::File, String> {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create(true)
        .truncate(truncate)
        .append(!truncate);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path).map_err(|_| "update_cache_failed".into())
}
impl Cache {
    pub fn open(root: PathBuf, identity: Identity) -> Result<(Self, Vec<u8>), String> {
        if identity.version.len() > 64
            || identity.url.len() > 2048
            || identity.signature.len() > 8192
            || identity.architecture.len() > 32
        {
            return Err("update_cache_failed".into());
        }
        if let Ok(m) = fs::symlink_metadata(&root) {
            if !m.is_dir() || m.file_type().is_symlink() {
                return Err("update_cache_failed".into());
            }
        }
        fs::create_dir_all(&root).map_err(|_| "update_cache_failed")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|_| "update_cache_failed")?;
        }
        let lock = open_private(&root.join("artifact.lock"), false)?;
        lock.try_lock().map_err(|_| "update_cache_busy")?;
        let mut cache = Self {
            _lock: lock,
            root,
            identity,
            etag: None,
        };
        let meta = cache.root.join("artifact.json");
        let part = cache.root.join("artifact.part");
        if regular(&meta, 32768) && regular(&part, LIMIT as u64) {
            let m = fs::read(&meta)
                .ok()
                .and_then(|b| serde_json::from_slice::<Meta>(&b).ok());
            if let Some(m) = m.filter(|m| m.identity == cache.identity && strong_etag(&m.etag)) {
                let mut b = vec![];
                fs::File::open(&part)
                    .map_err(|_| "update_cache_failed")?
                    .take(LIMIT as u64 + 1)
                    .read_to_end(&mut b)
                    .map_err(|_| "update_cache_failed")?;
                if b.len() <= LIMIT {
                    cache.etag = Some(m.etag);
                    return Ok((cache, b));
                }
            }
        }
        cache.reset(None)?;
        Ok((cache, vec![]))
    }
    pub fn reset(&mut self, etag: Option<String>) -> Result<(), String> {
        self.etag = etag.filter(|s| strong_etag(s));
        open_private(&self.root.join("artifact.part"), true)?;
        let meta = Meta {
            identity: self.identity.clone(),
            etag: self.etag.clone().unwrap_or_default(),
        };
        let bytes = serde_json::to_vec(&meta).map_err(|_| "update_cache_failed")?;
        let mut f = open_private(&self.root.join("artifact.json"), true)?;
        f.write_all(&bytes).map_err(|_| "update_cache_failed")?;
        f.sync_all().map_err(|_| "update_cache_failed")?;
        Ok(())
    }
    pub fn append(&self, data: &[u8], offset: usize) -> Result<(), String> {
        let path = self.root.join("artifact.part");
        if offset > LIMIT
            || data.len() > LIMIT - offset
            || !regular(&path, offset as u64)
            || fs::metadata(&path)
                .map_err(|_| "update_cache_failed")?
                .len()
                != offset as u64
        {
            return Err("update_cache_failed".into());
        }
        let mut f = open_private(&path, false)?;
        f.write_all(data).map_err(|_| "update_cache_failed")?;
        Ok(())
    }
    pub fn clear(root: &Path) -> Result<(), String> {
        if !root.exists() {
            return Ok(());
        }
        let metadata = fs::symlink_metadata(root).map_err(|_| "update_cache_failed")?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("update_cache_failed".into());
        }
        let lock = open_private(&root.join("artifact.lock"), false)?;
        lock.try_lock().map_err(|_| "update_cache_busy")?;

        for name in ["artifact.part", "artifact.json"] {
            let p = root.join(name);
            if p.exists() || p.is_symlink() {
                fs::remove_file(p).map_err(|_| "update_cache_failed")?;
            }
        }
        Ok(())
    }
}
pub fn strong_etag(value: &str) -> bool {
    value.len() >= 2
        && value.len() <= 512
        && value.starts_with('"')
        && value.ends_with('"')
        && !value.chars().any(char::is_control)
}
pub fn range_total(header: &str, offset: usize) -> Option<usize> {
    let v = header.strip_prefix("bytes ")?;
    let (range, total) = v.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let start = start.parse::<usize>().ok()?;
    let end = end.parse::<usize>().ok()?;
    let total = total.parse::<usize>().ok()?;
    (start == offset && end >= start && end < total && total <= LIMIT).then_some(total)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Identity {
        Identity {
            version: "1.0.0".into(),
            url: "https://example.invalid/update".into(),
            signature: "public-signature".into(),
            architecture: "aarch64".into(),
        }
    }
    #[test]
    fn active_cache_is_locked_and_clear_waits_for_the_owner() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        let (c, _) = Cache::open(root.clone(), identity()).unwrap();
        assert!(Cache::open(root.clone(), identity()).is_err());
        assert!(Cache::clear(&root).is_err());
        drop(c);
        Cache::clear(&root).unwrap();
        assert!(!root.join("artifact.part").exists());
    }
    #[test]
    fn only_exact_strong_etag_artifacts_resume_and_ranges_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let (mut c, b) = Cache::open(dir.path().join("cache"), identity()).unwrap();
        assert!(b.is_empty());
        c.reset(Some("\"v1\"".into())).unwrap();
        c.append(b"abc", 0).unwrap();
        assert!(c.append(b"x", 0).is_err());
        drop(c);
        let (c, b) = Cache::open(dir.path().join("cache"), identity()).unwrap();
        assert_eq!(b, b"abc");
        drop(c);
        let mut different = identity();
        different.signature = "changed".into();
        assert!(Cache::open(dir.path().join("cache"), different)
            .unwrap()
            .1
            .is_empty());
        assert_eq!(range_total("bytes 3-9/10", 3), Some(10));
        for h in [
            "bytes 2-9/10",
            "bytes 3-10/10",
            "bytes 3-9/*",
            "bytes 3-9/999999999",
        ] {
            assert_eq!(range_total(h, 3), None);
        }
        assert!(!strong_etag("W/\"weak\""));
    }
    #[test]
    fn cache_symlinks_and_oversize_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let (c, _) = Cache::open(dir.path().join("cache"), identity()).unwrap();
        assert!(c.append(&[], LIMIT + 1).is_err());
        #[cfg(unix)]
        {
            fs::remove_file(c.root.join("artifact.part")).unwrap();
            std::os::unix::fs::symlink(dir.path().join("victim"), c.root.join("artifact.part"))
                .unwrap();
            let path = c.root.clone();
            drop(c);
            assert!(Cache::open(path, identity()).is_err());
        }
    }
}
