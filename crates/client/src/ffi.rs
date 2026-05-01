//! C-FFI bindings for mobile platforms (Swift/Kotlin)
//!
//! # Safety
//! All pointers returned by FFI functions must be freed using the corresponding `_free` function.
//! String pointers are null-terminated UTF-8.

use crate::client::FerroClient;
use crate::error::ClientError;
use crate::types::FileEntry;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

pub struct FerroClientHandle {
    client: FerroClient,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FerroResult {
    Success = 0,
    ErrorNetwork = 1,
    ErrorAuth = 2,
    ErrorNotFound = 3,
    ErrorHttp = 4,
    ErrorXml = 5,
    ErrorInvalidArg = 6,
    ErrorUnknown = 99,
}

impl From<&ClientError> for FerroResult {
    fn from(err: &ClientError) -> Self {
        match err {
            ClientError::AuthFailed => FerroResult::ErrorAuth,
            ClientError::NotFound(_) => FerroResult::ErrorNotFound,
            ClientError::Http { .. } => FerroResult::ErrorHttp,
            ClientError::XmlParse(_) => FerroResult::ErrorXml,
            ClientError::Network(_) => FerroResult::ErrorNetwork,
            _ => FerroResult::ErrorUnknown,
        }
    }
}

#[repr(C)]
pub struct FerroFileEntry {
    pub name: *mut c_char,
    pub path: *mut c_char,
    pub size: u64,
    pub is_dir: bool,
    pub modified: *mut c_char,
    pub etag: *mut c_char,
}

#[repr(C)]
pub struct FerroFileList {
    pub entries: *mut FerroFileEntry,
    pub count: usize,
    pub result: FerroResult,
}

#[repr(C)]
pub struct FerroBytes {
    pub data: *mut u8,
    pub len: usize,
    pub result: FerroResult,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferro_client_new(
    server_url: *const c_char,
    token: *const c_char,
) -> *mut FerroClientHandle {
    if server_url.is_null() || token.is_null() {
        return ptr::null_mut();
    }

    let url = match unsafe { CStr::from_ptr(server_url) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let token = match unsafe { CStr::from_ptr(token) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let client = FerroClient::new(url, token);
    Box::into_raw(Box::new(FerroClientHandle { client }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferro_client_free(handle: *mut FerroClientHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferro_test_connection(handle: *mut FerroClientHandle) -> FerroResult {
    if handle.is_null() {
        return FerroResult::ErrorInvalidArg;
    }

    let client = &unsafe { &*handle }.client;
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return FerroResult::ErrorUnknown,
    };

    match rt.block_on(client.test_connection()) {
        Ok(_) => FerroResult::Success,
        Err(e) => FerroResult::from(&e),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferro_file_list_free(list: *mut FerroFileList) {
    if list.is_null() {
        return;
    }

    let list = unsafe { Box::from_raw(list) };
    for i in 0..list.count {
        let entry = unsafe { &*list.entries.add(i) };
        if !entry.name.is_null() {
            unsafe { drop(CString::from_raw(entry.name)) };
        }
        if !entry.path.is_null() {
            unsafe { drop(CString::from_raw(entry.path)) };
        }
        if !entry.modified.is_null() {
            unsafe { drop(CString::from_raw(entry.modified)) };
        }
        if !entry.etag.is_null() {
            unsafe { drop(CString::from_raw(entry.etag)) };
        }
    }
    if !list.entries.is_null() && list.count > 0 {
        unsafe { drop(Vec::from_raw_parts(list.entries, list.count, list.count)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferro_bytes_free(bytes: *mut FerroBytes) {
    if bytes.is_null() {
        return;
    }

    let bytes = unsafe { Box::from_raw(bytes) };
    if !bytes.data.is_null() && bytes.len > 0 {
        unsafe { drop(Vec::from_raw_parts(bytes.data, bytes.len, bytes.len)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferro_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

#[allow(dead_code)]
fn entry_to_ffi(entry: FileEntry) -> FerroFileEntry {
    let etag = match entry.etag.and_then(|e| CString::new(e).ok()) {
        Some(cs) => cs.into_raw(),
        None => ptr::null_mut(),
    };
    FerroFileEntry {
        name: CString::new(entry.name).unwrap_or_default().into_raw(),
        path: CString::new(entry.path).unwrap_or_default().into_raw(),
        size: entry.size,
        is_dir: entry.is_dir,
        modified: CString::new(entry.modified).unwrap_or_default().into_raw(),
        etag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_from_error() {
        assert_eq!(
            FerroResult::from(&ClientError::AuthFailed),
            FerroResult::ErrorAuth
        );
        assert_eq!(
            FerroResult::from(&ClientError::NotFound("/x".into())),
            FerroResult::ErrorNotFound
        );
        assert_eq!(
            FerroResult::from(&ClientError::XmlParse("bad".into())),
            FerroResult::ErrorXml
        );
        assert_eq!(
            FerroResult::from(&ClientError::Http {
                status: 500,
                body: "err".into()
            }),
            FerroResult::ErrorHttp
        );
    }

    #[test]
    fn test_entry_to_ffi() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: "/test.txt".to_string(),
            size: 42,
            is_dir: false,
            modified: "Wed, 01 Jan 2024".to_string(),
            etag: Some("\"abc\"".to_string()),
            content_type: None,
        };

        let ffi = entry_to_ffi(entry);
        assert_eq!(ffi.size, 42);
        assert!(!ffi.is_dir);

        unsafe {
            if !ffi.name.is_null() {
                drop(CString::from_raw(ffi.name));
            }
            if !ffi.path.is_null() {
                drop(CString::from_raw(ffi.path));
            }
            if !ffi.modified.is_null() {
                drop(CString::from_raw(ffi.modified));
            }
            if !ffi.etag.is_null() {
                drop(CString::from_raw(ffi.etag));
            }
        }
    }

    #[test]
    fn test_client_new_null() {
        unsafe {
            let handle = ferro_client_new(ptr::null(), ptr::null());
            assert!(handle.is_null());
        }
    }
}
