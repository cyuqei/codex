use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCheckProbeReport {
    pub platform: &'static str,
    pub supported: bool,
    pub has_token: bool,
    pub token_length: u64,
    #[serde(skip_serializing)]
    pub token_base64: Option<String>,
    pub error: Option<String>,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct RawDeviceCheckProbeResult {
    supported: bool,
    has_token: bool,
    token_length: std::os::raw::c_ulong,
    token_base64: *mut std::os::raw::c_char,
    error_description: *mut std::os::raw::c_char,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn codex_devicecheck_probe() -> RawDeviceCheckProbeResult;
    fn codex_devicecheck_probe_free(string: *mut std::os::raw::c_char);
}

pub fn probe_devicecheck() -> DeviceCheckProbeReport {
    probe_impl()
}

#[cfg(target_os = "macos")]
fn probe_impl() -> DeviceCheckProbeReport {
    let raw = unsafe { codex_devicecheck_probe() };
    let token_base64 = take_c_string(raw.token_base64);
    let error = take_c_string(raw.error_description);
    DeviceCheckProbeReport {
        platform: "macos",
        supported: raw.supported,
        has_token: raw.has_token,
        token_length: raw.token_length as u64,
        token_base64,
        error,
    }
}

#[cfg(target_os = "macos")]
fn take_c_string(ptr: *mut std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let value = unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { codex_devicecheck_probe_free(ptr) };
    Some(value)
}

#[cfg(not(target_os = "macos"))]
fn probe_impl() -> DeviceCheckProbeReport {
    DeviceCheckProbeReport {
        platform: std::env::consts::OS,
        supported: false,
        has_token: false,
        token_length: 0,
        token_base64: None,
        error: Some("DeviceCheck is only available on macOS in this probe".to_string()),
    }
}
