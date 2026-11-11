//! Native OS sleep prevention RAII guard.
//! Prevents system idle sleep during long-running batch video compression runs.
//! Releases all assertions automatically when dropped.

use std::sync::atomic::{AtomicBool, Ordering};

static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

const REASON_STR: &str = "mar2i: Processing Hajj & Umrah video batch\0";

/// RAII guard that inhibits OS idle sleep.
/// When dropped, normal system sleep behavior is restored.
pub struct KeepAwake {
    #[cfg(target_os = "macos")]
    assertion_id: Option<u32>,
    #[cfg(windows)]
    previous_state: Option<u32>,
}

impl KeepAwake {
    /// Acquire an idle sleep assertion.
    pub fn acquire() -> Self {
        #[cfg(target_os = "macos")]
        {
            let id = macos::create_assertion();
            if id.is_some() {
                IS_ACTIVE.store(true, Ordering::SeqCst);
            }
            Self { assertion_id: id }
        }

        #[cfg(windows)]
        {
            let prev = windows::inhibit_sleep();
            if prev.is_some() {
                IS_ACTIVE.store(true, Ordering::SeqCst);
            }
            Self { previous_state: prev }
        }

        #[cfg(not(any(target_os = "macos", windows)))]
        {
            IS_ACTIVE.store(true, Ordering::SeqCst);
            Self {}
        }
    }

    /// Whether a sleep prevention assertion is currently active.
    pub fn is_active() -> bool {
        IS_ACTIVE.load(Ordering::SeqCst)
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Some(id) = self.assertion_id {
                macos::release_assertion(id);
            }
        }

        #[cfg(windows)]
        {
            if let Some(prev) = self.previous_state {
                windows::restore_sleep(prev);
            }
        }

        IS_ACTIVE.store(false, Ordering::SeqCst);
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::REASON_STR;
    use std::ffi::c_void;
    use std::ptr;

    type CFStringRef = *const c_void;
    type IOReturn = i32;

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;
    const K_IOR_RETURN_SUCCESS: i32 = 0;

    const ASSERTION_TYPE_STR: &str = "PreventUserIdleSystemSleep\0";

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const i8,
            encoding: u32,
        ) -> CFStringRef;
        fn CFRelease(cf: *const c_void);
    }

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOPMAssertionCreateWithName(
            assertion_type: CFStringRef,
            level: u32,
            name: CFStringRef,
            assertion_id: *mut u32,
        ) -> IOReturn;
        fn IOPMAssertionRelease(assertion_id: u32) -> IOReturn;
    }

    pub fn create_assertion() -> Option<u32> {
        unsafe {
            let assertion_type_cf = CFStringCreateWithCString(
                ptr::null(),
                ASSERTION_TYPE_STR.as_ptr() as *const i8,
                K_CF_STRING_ENCODING_UTF8,
            );
            if assertion_type_cf.is_null() {
                return None;
            }

            let reason_cf = CFStringCreateWithCString(
                ptr::null(),
                REASON_STR.as_ptr() as *const i8,
                K_CF_STRING_ENCODING_UTF8,
            );
            if reason_cf.is_null() {
                CFRelease(assertion_type_cf);
                return None;
            }

            let mut assertion_id = 0u32;
            let status = IOPMAssertionCreateWithName(
                assertion_type_cf,
                K_IOPM_ASSERTION_LEVEL_ON,
                reason_cf,
                &mut assertion_id,
            );

            CFRelease(assertion_type_cf);
            CFRelease(reason_cf);

            if status == K_IOR_RETURN_SUCCESS {
                Some(assertion_id)
            } else {
                None
            }
        }
    }

    pub fn release_assertion(id: u32) {
        unsafe {
            let _ = IOPMAssertionRelease(id);
        }
    }
}

#[cfg(windows)]
mod windows {
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

    unsafe extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }

    pub fn inhibit_sleep() -> Option<u32> {
        unsafe {
            let prev = SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
            if prev != 0 { Some(prev) } else { None }
        }
    }

    pub fn restore_sleep(previous: u32) {
        unsafe {
            let _ = SetThreadExecutionState(previous);
        }
    }
}
