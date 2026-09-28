//! Small macOS helpers: permissions, frontmost app, system volume.

use serde::Serialize;
use std::ffi::{c_char, c_void};
use std::process::Command;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// "granted" | "denied" | "undetermined"
    pub microphone: String,
    pub accessibility: bool,
}

pub fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio
pub fn microphone_status() -> String {
    unsafe {
        type MsgStr = extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        type MsgObj = extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> isize;
        let send_str: MsgStr = std::mem::transmute(objc_msgSend as *const ());
        let send_obj: MsgObj = std::mem::transmute(objc_msgSend as *const ());

        let ns_string = objc_getClass(c"NSString".as_ptr());
        let device = objc_getClass(c"AVCaptureDevice".as_ptr());
        if ns_string.is_null() || device.is_null() {
            return "undetermined".into();
        }
        let media = send_str(ns_string, sel_registerName(c"stringWithUTF8String:".as_ptr()), c"soun".as_ptr());
        let status = send_obj(device, sel_registerName(c"authorizationStatusForMediaType:".as_ptr()), media);
        match status {
            3 => "granted",
            1 | 2 => "denied",
            _ => "undetermined",
        }
        .into()
    }
}

pub fn permissions() -> Permissions {
    Permissions { microphone: microphone_status(), accessibility: accessibility_trusted() }
}

pub fn open_privacy_pane(kind: &str) {
    let anchor = match kind {
        "microphone" => "Privacy_Microphone",
        _ => "Privacy_Accessibility",
    };
    let _ = Command::new("open")
        .arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}"))
        .spawn();
}

/// Name of the app in front, via `lsappinfo` (needs no extra permission).
pub fn frontmost_app() -> String {
    let asn = Command::new("lsappinfo").arg("front").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let Some(asn) = asn.filter(|s| !s.is_empty()) else { return "Unknown".into() };
    let out = Command::new("lsappinfo")
        .args(["info", "-only", "name", &asn])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    // "LSDisplayName"="Notion"
    out.rsplit('=')
        .next()
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".into())
}

pub fn os_version() -> String {
    Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn osascript(script: &str) -> Option<String> {
    Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn output_volume() -> Option<u32> {
    osascript("output volume of (get volume settings)")?.parse().ok()
}

pub fn set_output_volume(v: u32) {
    let _ = osascript(&format!("set volume output volume {}", v.min(100)));
}

pub fn reveal(path: &std::path::Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = Command::new("open").arg(path).spawn();
}
