//! Decorative app icons only. Never an input to identity, policy or trust decisions.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{sync_channel, SyncSender},
        OnceLock,
    },
    time::{Duration, Instant},
};
use tauri::Manager;
pub const SIDE: u32 = 96;
const MAX_PNG: usize = 48 * 1024;
const CACHE_SIZE: usize = 128;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub source: &'static str,
}
fn png_asset(bytes: &[u8]) -> Option<Asset> {
    if bytes.len() < 24
        || bytes.len() > MAX_PNG
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || &bytes[12..16] != b"IHDR"
        || u32::from_be_bytes(bytes[16..20].try_into().ok()?) != SIDE
        || u32::from_be_bytes(bytes[20..24].try_into().ok()?) != SIDE
    {
        return None;
    }
    Some(Asset {
        data_url: format!("data:image/png;base64,{}", STANDARD.encode(bytes)),
        width: SIDE,
        height: SIDE,
        source: "macos_workspace",
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    path: PathBuf,
    length: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    change: (i64, i64),
}
fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Some(Stamp {
        path: path.into(),
        length: m.len(),
        modified: m.modified().ok(),
        #[cfg(unix)]
        inode: m.ino(),
        #[cfg(unix)]
        change: (m.ctime(), m.ctime_nsec()),
    })
}
struct Bundle {
    path: PathBuf,
    stamps: Vec<Stamp>,
}
fn local_file(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let Ok(c) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        let mut fs = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: valid path C string and correctly allocated statfs output.
        unsafe {
            libc::statfs(c.as_ptr(), fs.as_mut_ptr()) == 0
                && fs.assume_init().f_flags & libc::MNT_LOCAL as u32 != 0
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}
fn bundle_for_executable(executable: &Path) -> Option<Bundle> {
    if !executable.is_absolute()
        || executable
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    let executable = executable.canonicalize().ok()?;
    if !executable.is_file() || !local_file(&executable) {
        return None;
    }
    let mut chosen = None;
    for path in executable
        .ancestors()
        .filter(|p| p.extension().is_some_and(|e| e == "app"))
    {
        if !executable.starts_with(path.join("Contents")) {
            continue;
        }
        let info = path.join("Contents/Info.plist");
        let info_path = info.canonicalize().ok()?;
        if !info_path.starts_with(path) || std::fs::metadata(&info_path).ok()?.len() > 65536 {
            continue;
        }
        use std::io::Read;
        let value = plist::Value::from_reader(std::io::Cursor::new({
            let mut data = Vec::new();
            std::fs::File::open(&info_path)
                .ok()?
                .take(65537)
                .read_to_end(&mut data)
                .ok()?;
            data
        }))
        .ok()?;
        if value
            .as_dictionary()?
            .get("CFBundlePackageType")
            .and_then(|v| v.as_string())
            != Some("APPL")
        {
            continue;
        }
        let mut stamps = vec![stamp(path)?, stamp(&info_path)?];
        let resources = path.join("Contents/Resources");
        if let Some(s) = stamp(&resources.join("Assets.car")) {
            stamps.push(s);
        }
        if let Some(name) = value
            .as_dictionary()?
            .get("CFBundleIconFile")
            .and_then(|v| v.as_string())
        {
            if name.len() <= 255
                && !name.contains('/')
                && !name.contains('\\')
                && !name.contains("..")
            {
                let file = resources.join(if name.ends_with(".icns") {
                    name.to_string()
                } else {
                    format!("{name}.icns")
                });
                if let Some(s) = stamp(&file) {
                    stamps.push(s);
                }
            }
        }
        // An enclosing application supplies the icon for a nested helper; no name search.
        chosen = Some(Bundle {
            path: path.into(),
            stamps,
        });
    }
    chosen
}
struct Entry {
    path: PathBuf,
    stamps: Vec<Stamp>,
    value: Option<Asset>,
    at: Instant,
}
#[derive(Default)]
struct Cache {
    entries: Vec<Entry>,
}
impl Cache {
    fn get(&mut self, bundle: &Bundle) -> Option<Option<Asset>> {
        let i = self.entries.iter().position(|e| e.path == bundle.path)?;
        let e = self.entries.remove(i);
        let ttl = if e.value.is_some() { 60 } else { 15 };
        if e.stamps != bundle.stamps || e.at.elapsed() >= Duration::from_secs(ttl) {
            return None;
        }
        let value = e.value.clone();
        self.entries.push(e);
        Some(value)
    }
    fn put(&mut self, bundle: Bundle, value: Option<Asset>) {
        self.entries.retain(|e| e.path != bundle.path);
        self.entries.push(Entry {
            path: bundle.path,
            stamps: bundle.stamps,
            value,
            at: Instant::now(),
        });
        if self.entries.len() > CACHE_SIZE {
            self.entries.remove(0);
        }
    }
}
struct Job {
    app: tauri::AppHandle,
    subject: String,
    path: PathBuf,
    deadline: Instant,
    reply: tokio::sync::oneshot::Sender<Option<Asset>>,
}
static WORKER: OnceLock<SyncSender<Job>> = OnceLock::new();
fn worker() -> &'static SyncSender<Job> {
    WORKER.get_or_init(|| {
        let (tx, rx) = sync_channel::<Job>(16);
        std::thread::spawn(move || {
            let mut cache = Cache::default();
            for job in rx {
                let result = (|| {
                    if Instant::now() > job.deadline {
                        return None;
                    }
                    if job
                        .app
                        .state::<crate::state::SharedState>()
                        .icon_subject(&job.subject)
                        .as_deref()
                        != Some(job.path.as_path())
                    {
                        return None;
                    }
                    let bundle = bundle_for_executable(&job.path)?;
                    if Instant::now() > job.deadline {
                        return None;
                    }
                    let value = if let Some(cached) = cache.get(&bundle) {
                        cached
                    } else {
                        let value = platform_icon(&bundle.path).and_then(|png| png_asset(&png));
                        if bundle_for_executable(&job.path).as_ref().map(|b| &b.stamps)
                            != Some(&bundle.stamps)
                        {
                            return None;
                        }
                        cache.put(bundle, value.clone());
                        value
                    };
                    if Instant::now() > job.deadline
                        || job
                            .app
                            .state::<crate::state::SharedState>()
                            .icon_subject(&job.subject)
                            .as_deref()
                            != Some(job.path.as_path())
                    {
                        return None;
                    }
                    value
                })();
                let _ = job.reply.send(result);
            }
        });
        tx
    })
}
#[tauri::command]
pub async fn application_icon(app: tauri::AppHandle, subject: String) -> Option<Asset> {
    if subject.len() > 4096 || subject.contains('\0') {
        return None;
    }
    let path = app
        .state::<crate::state::SharedState>()
        .icon_subject(&subject)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    worker()
        .try_send(Job {
            app,
            subject,
            path,
            deadline: Instant::now() + Duration::from_secs(4),
            reply: tx,
        })
        .ok()?;
    tokio::time::timeout(Duration::from_secs(4), rx)
        .await
        .ok()?
        .ok()?
}
#[cfg(not(target_os = "macos"))]
fn platform_icon(_: &Path) -> Option<Vec<u8>> {
    None
}
#[cfg(target_os = "macos")]
fn platform_icon(path: &Path) -> Option<Vec<u8>> {
    use objc2::AnyThread;
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
        NSGraphicsContext, NSWorkspace,
    };
    use objc2_foundation::{NSCopying, NSDictionary, NSPoint, NSRect, NSSize, NSString};
    objc2::rc::autoreleasepool(|_| {
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path.to_str()?));
        let image = image.copy();
        // SAFETY: null planes asks AppKit to allocate a bounded, owned RGBA bitmap.
        let bitmap = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(NSBitmapImageRep::alloc(),std::ptr::null_mut(),SIDE as isize,SIDE as isize,8,4,true,false,NSDeviceRGBColorSpace,(SIDE*4) as isize,32)
        }?;
        let pixels = bitmap.bitmapData();
        if pixels.is_null() || bitmap.bytesPerRow() != (SIDE * 4) as isize {
            return None;
        }
        // SAFETY: the exclusively owned, non-planar RGBA bitmap has exactly 96 rows
        // of 384 bytes. Clear transparent margins before any drawing or PNG encoding.
        unsafe {
            std::ptr::write_bytes(pixels, 0, (SIDE * SIDE * 4) as usize);
        }
        let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
        NSGraphicsContext::saveGraphicsState_class();
        struct Restore;
        impl Drop for Restore {
            fn drop(&mut self) {
                NSGraphicsContext::restoreGraphicsState_class();
            }
        }
        let _restore = Restore;
        NSGraphicsContext::setCurrentContext(Some(&context));
        image.drawInRect_fromRect_operation_fraction(
            NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(SIDE as f64, SIDE as f64),
            ),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            NSCompositingOperation::Copy,
            1.0,
        );
        context.flushGraphics();
        // SAFETY: an empty property dictionary supplies no incorrectly typed values. NSData
        // remains retained and immutable while copied; no AppKit objects leave this worker.
        let png = unsafe {
            bitmap.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &NSDictionary::new(),
            )
        }?;
        if png.len() > MAX_PNG {
            return None;
        }
        Some(unsafe { png.as_bytes_unchecked() }.to_vec())
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_icons_and_oversized_pngs() {
        assert!(png_asset(b"not an image").is_none());
        let mut p = vec![0u8; 24];
        p[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        p[12..16].copy_from_slice(b"IHDR");
        p[16..20].copy_from_slice(&SIDE.to_be_bytes());
        p[20..24].copy_from_slice(&SIDE.to_be_bytes());
        assert!(png_asset(&p).is_some());
        p[16..20].copy_from_slice(&4096u32.to_be_bytes());
        assert!(png_asset(&p).is_none());
        p.resize(MAX_PNG + 1, 0);
        assert!(png_asset(&p).is_none());
    }
    #[test]
    fn cache_is_bounded_and_fingerprints_expire_old_content() {
        let mut c = Cache::default();
        for n in 0..130 {
            c.put(
                Bundle {
                    path: format!("/{n}.app").into(),
                    stamps: vec![],
                },
                None,
            );
        }
        assert_eq!(c.entries.len(), 128);
        let b = Bundle {
            path: "/129.app".into(),
            stamps: vec![],
        };
        assert!(c.get(&b).is_some());
        let changed = Bundle {
            path: b.path.clone(),
            stamps: vec![Stamp {
                path: "/129.app/Contents/Info.plist".into(),
                length: 10,
                modified: None,
                #[cfg(unix)]
                inode: 1,
                #[cfg(unix)]
                change: (0, 0),
            }],
        };
        assert!(c.get(&changed).is_none());
        c.put(
            Bundle {
                path: b.path.clone(),
                stamps: vec![],
            },
            None,
        );
        c.entries.last_mut().unwrap().at = Instant::now() - Duration::from_secs(16);
        assert!(c.get(&b).is_none());
        assert!(c
            .get(&Bundle {
                path: "/0.app".into(),
                stamps: vec![]
            })
            .is_none());
    }
    #[test]
    fn non_app_and_missing_paths_do_not_produce_icons() {
        assert!(bundle_for_executable(Path::new("relative/app")).is_none());
        assert!(bundle_for_executable(Path::new("/usr/bin/true")).is_none());
        assert!(bundle_for_executable(Path::new(
            "/definitely-missing/Test.app/Contents/MacOS/test"
        ))
        .is_none());
    }
    #[test]
    #[ignore = "reads local AppKit icons; requires a macOS GUI session"]
    fn actual_workspace_icon_is_a_bounded_png() {
        let path = Path::new("/System/Applications/TextEdit.app/Contents/MacOS/TextEdit");
        let bundle = bundle_for_executable(path).expect("local TextEdit bundle");
        let bytes = platform_icon(&bundle.path).expect("AppKit icon");
        let asset = png_asset(&bytes).expect("bounded 96px PNG");
        assert_eq!(asset.width, 96);
        std::fs::write("/tmp/flowguard-native-app-icon.png", bytes).unwrap();
    }
}
