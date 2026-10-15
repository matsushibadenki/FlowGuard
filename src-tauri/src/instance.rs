//! Same-user local activation only: no paths, rules, URLs or OS operations cross IPC.
#[cfg(target_os = "macos")]
mod local {
    use std::{
        fs,
        io::{self, Read, Write},
        os::unix::{
            fs::{FileTypeExt, MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };
    const REQUEST: &[u8; 8] = b"FGSHOW01";
    const ACK: &[u8; 8] = b"FGACK001";
    fn owner(path: &Path, socket: bool) -> io::Result<fs::Metadata> {
        let meta = fs::symlink_metadata(path)?;
        if meta.uid() != unsafe { libc::geteuid() }
            || (socket && !meta.file_type().is_socket())
            || (!socket && !meta.is_dir())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Untrusted activation path",
            ));
        }
        Ok(meta)
    }
    fn peer(stream: &UnixStream) -> bool {
        let (mut uid, mut gid) = (0, 0);
        unsafe {
            libc::getpeereid(std::os::fd::AsRawFd::as_raw_fd(stream), &mut uid, &mut gid) == 0
                && uid == libc::geteuid()
        }
    }
    fn bounds(stream: &UnixStream) -> io::Result<()> {
        let deadline = Some(Duration::from_millis(200));
        stream.set_read_timeout(deadline)?;
        stream.set_write_timeout(deadline)
    }
    pub fn socket_path(document: &Path) -> io::Result<PathBuf> {
        use std::os::unix::ffi::OsStrExt;
        let direct = document.with_file_name("activation-v1.sock");
        if direct.as_os_str().as_bytes().len() <= 100 {
            return Ok(direct);
        }
        // macOS sockaddr_un paths are short; use a private same-user directory and full document hash.
        use sha2::{Digest, Sha256};
        use std::os::unix::fs::DirBuilderExt;
        let root = PathBuf::from(format!("/tmp/fg-{}", unsafe { libc::geteuid() }));
        match fs::DirBuilder::new().mode(0o700).create(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        let meta = owner(&root, false)?;
        if meta.mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Activation directory must be private",
            ));
        }
        let digest = Sha256::digest(document.as_os_str().as_bytes());
        Ok(root.join(format!("{digest:x}.sock")))
    }
    pub fn request(path: &Path) -> io::Result<bool> {
        match owner(path, true) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        }
        let mut stream = match UnixStream::connect(path) {
            Ok(s) => s,
            Err(e)
                if [io::ErrorKind::ConnectionRefused, io::ErrorKind::NotFound]
                    .contains(&e.kind()) =>
            {
                return Ok(false)
            }
            Err(e) => return Err(io::Error::new(e.kind(), format!("Activation connect: {e}"))),
        };
        bounds(&stream)
            .map_err(|e| io::Error::new(e.kind(), format!("Activation timeout setup: {e}")))?;
        if !peer(&stream) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Activation peer rejected",
            ));
        }
        stream
            .write_all(REQUEST)
            .map_err(|e| io::Error::new(e.kind(), format!("Activation write: {e}")))?;
        let mut reply = [0; 8];
        stream
            .read_exact(&mut reply)
            .map_err(|e| io::Error::new(e.kind(), format!("Activation acknowledgement: {e}")))?;
        Ok(&reply == ACK)
    }
    pub struct Server {
        path: PathBuf,
        ino: u64,
        dev: u64,
        stop: Arc<AtomicBool>,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Ok(meta) = fs::symlink_metadata(&self.path) {
                if meta.ino() == self.ino && meta.dev() == self.dev {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
    }
    /// Call only after acquiring the document lock. Never replaces a regular file/symlink.
    pub fn start(path: PathBuf, show: impl Fn() -> bool + Send + 'static) -> io::Result<Server> {
        owner(
            path.parent()
                .ok_or_else(|| io::Error::other("Missing parent"))?,
            false,
        )?;
        match owner(&path, true) {
            Ok(_) => fs::remove_file(&path)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        };
        let listener = UnixListener::bind(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let meta = owner(&path, true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        std::thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if bounds(&stream).is_err() || !peer(&stream) {
                            continue;
                        }
                        let mut command = [0; 8];
                        if stream.read_exact(&mut command).is_ok()
                            && &command == REQUEST
                            && !stopped.load(Ordering::Acquire)
                            && show()
                        {
                            let _ = stream.write_all(ACK);
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Server {
            path,
            ino: meta.ino(),
            dev: meta.dev(),
            stop,
        })
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn rejects_regular_files_and_symlinks() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("focus.sock");
            fs::write(&path, "preserved").unwrap();
            assert!(start(path.clone(), || true).is_err());
            assert!(request(&path).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), "preserved");
            fs::remove_file(&path).unwrap();
            std::os::unix::fs::symlink(dir.path().join("other"), &path).unwrap();
            assert!(start(path, || true).is_err());
        }
        #[test]
        #[ignore = "same-user Unix socket only; no OS configuration"]
        fn host_focus_roundtrip_and_malformed_request() {
            use std::sync::atomic::AtomicUsize;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("focus.sock");
            assert!(!request(&path).unwrap());
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let server = start(path.clone(), move || {
                count.fetch_add(1, Ordering::SeqCst);
                true
            })
            .unwrap();
            assert!(request(&path).unwrap());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            // A real second process uses the same fixed wire protocol; no app/OS state is touched.
            let status=std::process::Command::new("/usr/bin/python3")
                .args(["-c", "import socket,sys; s=socket.socket(socket.AF_UNIX); s.settimeout(2); s.connect(sys.argv[1]); s.sendall(b'FGSHOW01'); assert s.recv(8)==b'FGACK001'",path.to_str().unwrap()])
                .status().unwrap();
            assert!(status.success());
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            let mut bad = UnixStream::connect(&path).unwrap();
            bounds(&bad).unwrap();
            bad.write_all(b"INVALID!").unwrap();
            let mut reply = [0; 8];
            assert!(bad.read_exact(&mut reply).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            drop(server);
            assert!(!path.exists());
        }
        #[test]
        #[ignore = "same-user Unix socket only; no OS configuration"]
        fn host_stale_socket_and_document_lock_are_preserved() {
            let dir = tempfile::tempdir().unwrap();
            let long_document = dir.path().join("long-".repeat(50)).join("doc.json");
            let fallback = socket_path(&long_document).unwrap();
            assert!(fallback.as_os_str().as_encoded_bytes().len() <= 100);
            let long_server = start(fallback.clone(), || true).unwrap();
            assert!(request(&fallback).unwrap());
            drop(long_server);
            let doc = dir.path().join("doc.json");
            let primary = crate::state::SharedState::load(doc.clone()).unwrap();
            let path = dir.path().join("focus.sock");
            let stale = UnixListener::bind(&path).unwrap();
            drop(stale);
            assert!(!request(&path).unwrap());
            let server = start(path.clone(), || true).unwrap();
            assert!(request(&path).unwrap());
            assert!(crate::state::SharedState::load(doc.clone()).is_err());
            drop(server);
            drop(primary);
            assert!(crate::state::SharedState::load(doc).is_ok());
        }
    }
}
#[cfg(target_os = "macos")]
pub use local::*;
