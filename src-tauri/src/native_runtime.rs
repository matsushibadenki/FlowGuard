use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::Path,
};
pub type Callback = extern "C" fn(*const c_char, u64);
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}
pub struct NativeRuntime {
    notification: Option<unsafe extern "C" fn(i32, u64, *const u8, u64, Callback, u64)>,
    #[cfg(target_os = "macos")]
    handle: *mut c_void,
    preflight: unsafe extern "C" fn() -> i32,
    activate: unsafe extern "C" fn(*const u8, u64, u64, Callback, u64),
    persist: unsafe extern "C" fn(*const u8, u64, u64, Callback, u64),
    disable: unsafe extern "C" fn(i32, u64, Callback, u64),
}
// Symbols are immutable; Swift dispatches signed lifecycle requests onto the main queue.
unsafe impl Send for NativeRuntime {}
unsafe impl Sync for NativeRuntime {}
impl NativeRuntime {
    pub fn load(contents: &Path) -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            let path = CString::new(
                contents
                    .join("Frameworks/libFlowGuardHostRuntime.dylib")
                    .to_string_lossy()
                    .as_bytes(),
            )
            .map_err(|_| "invalid_bundle_path")?;
            let handle = unsafe { dlopen(path.as_ptr(), 2) }; // RTLD_NOW, fixed bundled path only
            if handle.is_null() {
                return Err("native_runtime_unavailable".into());
            }
            let abi = unsafe { dlsym(handle, c"flowguard_host_abi_version".as_ptr()) };
            if abi.is_null()
                || unsafe {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(abi)()
                } != 5
            {
                unsafe {
                    dlclose(handle);
                }
                return Err("invalid_native_runtime".into());
            }
            let notification = unsafe { dlsym(handle, c"flowguard_host_notification".as_ptr()) };
            let preflight = unsafe { dlsym(handle, c"flowguard_host_can_activate".as_ptr()) };
            let activate = unsafe { dlsym(handle, c"flowguard_host_activate".as_ptr()) };
            let persist = unsafe { dlsym(handle, c"flowguard_host_persist_policy".as_ptr()) };
            let disable = unsafe { dlsym(handle, c"flowguard_host_disable".as_ptr()) };
            if preflight.is_null() || activate.is_null() || disable.is_null() || persist.is_null() {
                unsafe {
                    dlclose(handle);
                }
                return Err("invalid_native_runtime".into());
            }
            Ok(Self {
                notification: if notification.is_null() {
                    None
                } else {
                    Some(unsafe {
                        std::mem::transmute::<
                            *mut c_void,
                            unsafe extern "C" fn(i32, u64, *const u8, u64, Callback, u64),
                        >(notification)
                    })
                },
                handle,
                preflight: unsafe {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(preflight)
                },
                activate: unsafe {
                    std::mem::transmute::<
                        *mut c_void,
                        unsafe extern "C" fn(*const u8, u64, u64, Callback, u64),
                    >(activate)
                },
                persist: unsafe {
                    std::mem::transmute::<
                        *mut c_void,
                        unsafe extern "C" fn(*const u8, u64, u64, Callback, u64),
                    >(persist)
                },
                disable: unsafe {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i32, u64, Callback, u64)>(
                        disable,
                    )
                },
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = contents;
            Err("macos_required".into())
        }
    }
    pub fn notification(
        &self,
        operation: i32,
        revision: u64,
        payload: &[u8],
        callback: Callback,
        token: u64,
    ) {
        if let Some(call) = self.notification {
            unsafe {
                call(
                    operation,
                    revision,
                    payload.as_ptr(),
                    payload.len() as u64,
                    callback,
                    token,
                )
            }
        } else {
            callback(c"unavailable".as_ptr(), token)
        }
    }
    pub fn signed(&self) -> bool {
        unsafe { (self.preflight)() == 1 }
    }
    pub fn activate(&self, policy: &[u8], generation: u64, callback: Callback, token: u64) {
        unsafe {
            (self.activate)(
                policy.as_ptr(),
                policy.len() as u64,
                generation,
                callback,
                token,
            )
        }
    }
    pub fn persist(&self, policy: &[u8], generation: u64, callback: Callback, token: u64) {
        unsafe {
            (self.persist)(
                policy.as_ptr(),
                policy.len() as u64,
                generation,
                callback,
                token,
            )
        }
    }
    pub fn disable(&self, remove: bool, generation: u64, callback: Callback, token: u64) {
        unsafe { (self.disable)(i32::from(remove), generation, callback, token) }
    }
}
static PERSIST_REPLIES: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<u64, std::sync::mpsc::Sender<String>>>,
> = std::sync::OnceLock::new();
static PERSIST_TOKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
extern "C" fn persisted(status: *const c_char, token: u64) {
    if let Some(sender) = PERSIST_REPLIES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&token)
    {
        let _ = sender.send(callback_status(status).unwrap_or_else(|| "failed".into()));
    }
}
impl NativeRuntime {
    /// Called on a Rust blocking worker; Swift preference operations remain on the main queue.
    pub fn persist_blocking(&self, policy: &[u8], generation: u64) -> Result<(), String> {
        let token = PERSIST_TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, receiver) = std::sync::mpsc::channel();
        PERSIST_REPLIES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .insert(token, sender);
        self.persist(policy, generation, persisted, token);
        let status = receiver.recv_timeout(std::time::Duration::from_secs(30));
        PERSIST_REPLIES
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .remove(&token);
        match status {
            Ok(status) if status == "policy_persisted" => Ok(()),
            Ok(status) => Err(format!("startup_policy_{status}")),
            Err(_) => Err("startup_policy_timeout".into()),
        }
    }
}
// Runtime stays loaded for the app lifetime: pending Swift callbacks must not point into unloaded code.
impl Drop for NativeRuntime {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let _ = self.handle;
        }
    }
}
pub fn callback_status(status: *const c_char) -> Option<String> {
    if status.is_null() {
        return None;
    }
    // Our bundled Swift export supplies a valid, temporary, NUL-terminated status string.
    Some(
        unsafe { CStr::from_ptr(status) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    static REPLY: std::sync::OnceLock<std::sync::Mutex<Option<std::sync::mpsc::Sender<String>>>> =
        std::sync::OnceLock::new();
    extern "C" fn receive(status: *const c_char, token: u64) {
        assert_eq!(token, 7);
        if let Some(status) = callback_status(status) {
            if let Some(sender) = REPLY.get_or_init(Default::default).lock().unwrap().as_ref() {
                let _ = sender.send(status);
            }
        }
    }
    #[test]
    #[ignore = "Requires npm run native:package; exercises the real Swift dylib without signing or OS mutation"]
    fn unsigned_runtime_gate_crosses_real_swift_ffi() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("native/build/FlowGuard.app/Contents");
        let runtime = NativeRuntime::load(&root).unwrap();
        assert!(!runtime.signed());
        let (sender, receiver) = std::sync::mpsc::channel();
        *REPLY.get_or_init(Default::default).lock().unwrap() = Some(sender);
        runtime.activate(br#"{"rules":[]}"#, 1, receive, 7);
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            "signature_required"
        );
        runtime.persist(br#"{"rules":[]}"#, 1, receive, 7);
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            "signature_required"
        );
        assert_eq!(
            runtime.persist_blocking(br#"{"rules":[]}"#, 1).unwrap_err(),
            "startup_policy_signature_required"
        );
        for remove in [false, true] {
            runtime.disable(remove, 1, receive, 7);
            assert_eq!(
                receiver
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap(),
                "signature_required"
            );
        }
        *REPLY.get().unwrap().lock().unwrap() = None;
    }
}
