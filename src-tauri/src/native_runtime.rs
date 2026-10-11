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
    #[cfg(target_os = "macos")]
    handle: *mut c_void,
    preflight: unsafe extern "C" fn() -> i32,
    activate: unsafe extern "C" fn(*const u8, u64, Callback, u64),
    disable: unsafe extern "C" fn(i32, Callback, u64),
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
                } != 2
            {
                unsafe {
                    dlclose(handle);
                }
                return Err("invalid_native_runtime".into());
            }
            let preflight = unsafe { dlsym(handle, c"flowguard_host_can_activate".as_ptr()) };
            let activate = unsafe { dlsym(handle, c"flowguard_host_activate".as_ptr()) };
            let disable = unsafe { dlsym(handle, c"flowguard_host_disable".as_ptr()) };
            if preflight.is_null() || activate.is_null() || disable.is_null() {
                unsafe {
                    dlclose(handle);
                }
                return Err("invalid_native_runtime".into());
            }
            Ok(Self {
                handle,
                preflight: unsafe {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> i32>(preflight)
                },
                activate: unsafe {
                    std::mem::transmute::<
                        *mut c_void,
                        unsafe extern "C" fn(*const u8, u64, Callback, u64),
                    >(activate)
                },
                disable: unsafe {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i32, Callback, u64)>(
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
    pub fn signed(&self) -> bool {
        unsafe { (self.preflight)() == 1 }
    }
    pub fn activate(&self, policy: &[u8], callback: Callback, token: u64) {
        unsafe { (self.activate)(policy.as_ptr(), policy.len() as u64, callback, token) }
    }
    pub fn disable(&self, remove: bool, callback: Callback, token: u64) {
        unsafe { (self.disable)(i32::from(remove), callback, token) }
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
        runtime.activate(br#"{"rules":[]}"#, receive, 7);
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            "signature_required"
        );
        for remove in [false, true] {
            runtime.disable(remove, receive, 7);
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
