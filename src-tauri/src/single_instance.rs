#[cfg(not(windows))]
pub fn ensure_single_instance() -> bool {
    true
}

#[cfg(windows)]
pub fn ensure_single_instance() -> bool {
    use std::ffi::c_void;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    static OWNED_MUTEX: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    let name: Vec<u16> = OsStr::new("Local\\rest-reminder.single-instance.v1")
        .encode_wide()
        .chain(Some(0))
        .collect();

    let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
    if handle.is_null() {
        // Creating the mutex should not normally fail. Fail open so startup is never blocked.
        return true;
    }

    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if already_exists {
        unsafe {
            CloseHandle(handle);
        }
        return false;
    }

    OWNED_MUTEX.store(handle, Ordering::SeqCst);
    true
}
