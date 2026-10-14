// パス: src-tauri/src/identity.rs
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigningIdentity {
    pub authority: String,
    pub team_id: String,
    pub identifier: String,
}
impl SigningIdentity {
    pub fn valid(&self) -> bool {
        !self.identifier.is_empty()
            && self.identifier.len() <= 256
            && self
                .identifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
            && match self.authority.as_str() {
                "apple" => self.team_id.is_empty(),
                "developer" => {
                    self.team_id.len() == 10
                        && self
                            .team_id
                            .bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
                }
                _ => false,
            }
    }
    pub fn requirement(&self) -> Option<String> {
        self.valid().then(|| if self.authority == "apple" {
            format!("anchor apple and identifier \"{}\"", self.identifier)
        } else {
            format!("anchor apple generic and certificate leaf[subject.OU] = \"{}\" and identifier \"{}\"", self.team_id, self.identifier)
        })
    }
    pub fn key(&self) -> String {
        format!(
            "signed:v1:{}:{}:{}",
            self.authority, self.team_id, self.identifier
        )
    }
}
pub fn for_pid(pid: u32) -> Option<SigningIdentity> {
    #[cfg(target_os = "macos")]
    {
        mac::for_pid(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}
#[cfg(target_os = "macos")]
mod mac {
    use super::SigningIdentity;
    use std::{
        ffi::{c_char, c_void},
        ptr,
    };
    type Ref = *const c_void;
    struct Owned(Ref);
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
    fn owned(status: i32, value: Ref) -> Option<Owned> {
        if value.is_null() {
            return None;
        }
        let value = Owned(value);
        if status == 0 {
            Some(value)
        } else {
            None
        }
    }
    #[test]
    fn failed_or_empty_api_result_never_releases_null() {
        assert!(owned(-1, ptr::null()).is_none());
        assert!(owned(0, ptr::null()).is_none());
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: Ref);
        fn CFNumberCreate(allocator: Ref, kind: isize, value: Ref) -> Ref;
        fn CFDictionaryCreate(
            allocator: Ref,
            keys: *const Ref,
            values: *const Ref,
            count: isize,
            key_callbacks: Ref,
            value_callbacks: Ref,
        ) -> Ref;
        fn CFDictionaryGetValue(dictionary: Ref, key: Ref) -> Ref;
        fn CFStringGetCString(string: Ref, buffer: *mut c_char, size: isize, encoding: u32)
            -> bool;
        fn CFStringCreateWithCString(allocator: Ref, string: *const c_char, encoding: u32) -> Ref;
        fn CFGetTypeID(value: Ref) -> usize;
        fn CFStringGetTypeID() -> usize;
    }
    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        static kSecGuestAttributePid: Ref;
        static kSecCodeInfoIdentifier: Ref;
        static kSecCodeInfoTeamIdentifier: Ref;
        fn SecCodeCopyGuestWithAttributes(
            host: Ref,
            attributes: Ref,
            flags: u32,
            code: *mut Ref,
        ) -> i32;
        fn SecCodeCheckValidity(code: Ref, flags: u32, requirement: Ref) -> i32;
        fn SecCodeCopyStaticCode(code: Ref, flags: u32, static_code: *mut Ref) -> i32;
        fn SecCodeCopySigningInformation(code: Ref, flags: u32, information: *mut Ref) -> i32;
        fn SecRequirementCreateWithString(text: Ref, flags: u32, requirement: *mut Ref) -> i32;
    }
    unsafe fn string(dictionary: Ref, key: Ref) -> Option<String> {
        let value = unsafe { CFDictionaryGetValue(dictionary, key) };
        if value.is_null() || unsafe { CFGetTypeID(value) != CFStringGetTypeID() } {
            return None;
        }
        let mut bytes = [0u8; 1024];
        if !unsafe {
            CFStringGetCString(
                value,
                bytes.as_mut_ptr().cast(),
                bytes.len() as isize,
                0x08000100,
            )
        } {
            return None;
        }
        let end = bytes.iter().position(|b| *b == 0)?;
        String::from_utf8(bytes[..end].to_vec()).ok()
    }
    unsafe fn satisfies(code: Ref, identity: &SigningIdentity) -> bool {
        let Some(text) = identity
            .requirement()
            .and_then(|s| std::ffi::CString::new(s).ok())
        else {
            return false;
        };
        let text = unsafe { CFStringCreateWithCString(ptr::null(), text.as_ptr(), 0x08000100) };
        if text.is_null() {
            return false;
        }
        let text = Owned(text);
        let mut requirement = ptr::null();
        let status = unsafe { SecRequirementCreateWithString(text.0, 0, &mut requirement) };
        let Some(requirement) = owned(status, requirement) else {
            return false;
        };
        unsafe { SecCodeCheckValidity(code, 0, requirement.0) == 0 }
    }
    pub fn for_pid(pid: u32) -> Option<SigningIdentity> {
        if pid == 0 || pid > i32::MAX as u32 {
            return None;
        }
        // No filesystem path is trusted. The kernel resolves a running guest and Security validates it.
        unsafe {
            let pid = pid as i32;
            let number = CFNumberCreate(ptr::null(), 3, (&pid as *const i32).cast());
            if number.is_null() {
                return None;
            }
            let number = Owned(number);
            let attributes = CFDictionaryCreate(
                ptr::null(),
                &kSecGuestAttributePid,
                &number.0,
                1,
                ptr::null(),
                ptr::null(),
            );
            if attributes.is_null() {
                return None;
            }
            let attributes = Owned(attributes);
            let mut code = ptr::null();
            let code = owned(
                SecCodeCopyGuestWithAttributes(ptr::null(), attributes.0, 0, &mut code),
                code,
            )?;
            if SecCodeCheckValidity(code.0, 0, ptr::null()) != 0 {
                return None;
            }
            let mut static_code = ptr::null();
            let static_code = owned(
                SecCodeCopyStaticCode(code.0, 0, &mut static_code),
                static_code,
            )?;
            let mut info = ptr::null();
            let info = owned(
                SecCodeCopySigningInformation(static_code.0, 2, &mut info),
                info,
            )?;
            let identifier = string(info.0, kSecCodeInfoIdentifier)?;
            let apple = SigningIdentity {
                authority: "apple".into(),
                team_id: String::new(),
                identifier: identifier.clone(),
            };
            if satisfies(code.0, &apple) {
                return Some(apple);
            }
            let developer = SigningIdentity {
                authority: "developer".into(),
                team_id: string(info.0, kSecCodeInfoTeamIdentifier)?,
                identifier,
            };
            satisfies(code.0, &developer).then_some(developer)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requirements_cannot_inject_or_ignore_publisher() {
        let mut identity = SigningIdentity {
            authority: "developer".into(),
            team_id: "ABCDE12345".into(),
            identifier: "com.test.app".into(),
        };
        assert!(identity
            .requirement()
            .unwrap()
            .contains("anchor apple generic"));
        identity.identifier = "x\" or true".into();
        assert!(identity.requirement().is_none());
        identity.identifier = "com.test.app".into();
        identity.team_id.clear();
        assert!(!identity.valid());
        identity.authority = "apple".into();
        assert!(identity.valid());
        assert!(for_pid(0).is_none());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn adhoc_test_process_is_not_a_trusted_publisher() {
        assert!(for_pid(std::process::id()).is_none());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod system_tests {
    #[test]
    #[ignore = "requires host Security guest access to an Apple-signed child"]
    fn signed_guest_apple_tool_has_a_verified_identity() {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let identity = super::for_pid(child.id());
        let _ = child.kill();
        let _ = child.wait();
        let identity = identity.expect("Apple-signed running guest must validate");
        assert_eq!(identity.authority, "apple");
        assert!(identity.requirement().unwrap().contains("anchor apple"));
    }
}
