#[cfg(windows)]
use std::{ffi::c_void, ptr, slice};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_NOT_FOUND},
    Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    },
};

#[cfg(windows)]
pub fn set(reference: &str, secret: &str) -> Result<(), String> {
    let target: Vec<u16> = reference.encode_utf16().chain([0]).collect();
    let mut blob = secret.as_bytes().to_vec();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: target.as_ptr() as *mut u16,
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };
    let result = unsafe { CredWriteW(&credential, 0) };
    if result == 0 {
        return Err(format!("CREDENTIAL_STORE_WRITE_FAILED: {}", unsafe {
            GetLastError()
        }));
    }
    Ok(())
}

#[cfg(windows)]
pub fn get(reference: &str) -> Result<Option<String>, String> {
    let target: Vec<u16> = reference.encode_utf16().chain([0]).collect();
    let mut credential = ptr::null_mut();
    let result = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if result == 0 {
        let error = unsafe { GetLastError() };
        if error == ERROR_NOT_FOUND {
            return Ok(None);
        }
        return Err(format!("CREDENTIAL_STORE_READ_FAILED: {error}"));
    }
    if credential.is_null() {
        return Ok(None);
    }
    let value = unsafe {
        let record = &*credential;
        let bytes =
            slice::from_raw_parts(record.CredentialBlob, record.CredentialBlobSize as usize);
        String::from_utf8(bytes.to_vec())
    }
    .map_err(|error| format!("CREDENTIAL_STORE_INVALID: {error}"));
    unsafe { CredFree(credential as *const c_void) };
    value.map(Some)
}

#[cfg(windows)]
pub fn remove(reference: &str) -> Result<(), String> {
    let target: Vec<u16> = reference.encode_utf16().chain([0]).collect();
    let result = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
    if result == 0 {
        let error = unsafe { GetLastError() };
        if error != ERROR_NOT_FOUND {
            return Err(format!("CREDENTIAL_STORE_DELETE_FAILED: {error}"));
        }
    }
    Ok(())
}
