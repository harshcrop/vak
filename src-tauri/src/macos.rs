//! Small macOS helpers: permissions, frontmost app, system volume.

use serde::Serialize;
use std::ffi::{c_char, c_void};
use std::process::Command;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: *const c_void;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_cb: *const c_void,
        value_cb: *const c_void,
    ) -> *const c_void;
    fn CFRelease(cf: *const c_void);
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    // Referenced so the linker keeps AVFoundation and `AVCaptureDevice` resolves at runtime.
    static AVMediaTypeAudio: *const c_void;
}

extern "C" {
    static _NSConcreteGlobalBlock: *const c_void;
}

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

/// Like `accessibility_trusted`, but when untrusted macOS shows its "allow Kōe to control
/// this computer" prompt and adds the app to the Accessibility list.
pub fn accessibility_trusted_or_prompt() -> bool {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let opts = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );
        let trusted = AXIsProcessTrustedWithOptions(opts);
        if !opts.is_null() {
            CFRelease(opts);
        }
        trusted
    }
}

/// AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio
pub fn microphone_status() -> String {
    unsafe {
        type MsgObj = extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> isize;
        let send_obj: MsgObj = std::mem::transmute(objc_msgSend as *const ());

        let device = objc_getClass(c"AVCaptureDevice".as_ptr());
        if device.is_null() {
            return "undetermined".into();
        }
        let media = AVMediaTypeAudio as *mut c_void;
        let status = send_obj(device, sel_registerName(c"authorizationStatusForMediaType:".as_ptr()), media);
        match status {
            3 => "granted",
            1 | 2 => "denied",
            _ => "undetermined",
        }
        .into()
    }
}

#[repr(C)]
struct BlockDesc {
    reserved: usize,
    size: usize,
}

#[repr(C)]
struct Block {
    isa: *const c_void,
    flags: i32,
    reserved: i32,
    invoke: extern "C" fn(*mut Block, bool),
    desc: *const BlockDesc,
}

static REQUEST_DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

extern "C" fn request_done(_: *mut Block, _granted: bool) {
    REQUEST_DONE.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// AVCaptureDevice requestAccessForMediaType:completionHandler: — shows the system prompt and
/// blocks until it is answered. Going through AVFoundation (not just opening an input stream)
/// keeps `microphone_status` in step with the answer.
pub fn request_microphone() {
    use std::sync::atomic::Ordering;
    unsafe {
        let device = objc_getClass(c"AVCaptureDevice".as_ptr());
        if device.is_null() {
            return;
        }
        let desc = BlockDesc { reserved: 0, size: std::mem::size_of::<Block>() };
        let mut block = Block { isa: _NSConcreteGlobalBlock, flags: 1 << 28, reserved: 0, invoke: request_done, desc: &desc };
        REQUEST_DONE.store(false, Ordering::SeqCst);
        type Msg = extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut Block);
        let send: Msg = std::mem::transmute(objc_msgSend as *const ());
        send(device, sel_registerName(c"requestAccessForMediaType:completionHandler:".as_ptr()), AVMediaTypeAudio as *mut c_void, &mut block);
        // The block lives on this stack frame, so wait for the answer before returning.
        for _ in 0..1200 {
            if REQUEST_DONE.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

pub fn permissions() -> Permissions {
    Permissions { microphone: microphone_status(), accessibility: accessibility_trusted() }
}

pub fn open_privacy_pane(kind: &str) {
    // Registers Kōe in the Accessibility list; without this the pane opens with no entry to toggle.
    if kind == "accessibility" && accessibility_trusted_or_prompt() {
        return;
    }
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
    parse_app_name(&out).unwrap_or_else(|| "Unknown".into())
}

/// Older macOS prints `"LSDisplayName"="Notion"`; newer prints `"Notion" ASN:0x0-…: (in front)`
/// followed by fields like `Arch=!!none`, so take the name, never the last `=` value.
fn parse_app_name(out: &str) -> Option<String> {
    let quoted = |s: &str| s.trim_start().strip_prefix('"').and_then(|r| r.split('"').next()).map(str::to_string);
    let name = match out.find("\"LSDisplayName\"=") {
        Some(i) => quoted(&out[i + "\"LSDisplayName\"=".len()..]),
        None => quoted(out),
    };
    name.filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::parse_app_name;

    #[test]
    fn app_name_from_lsappinfo() {
        assert_eq!(parse_app_name("\"LSDisplayName\"=\"Notion\"\n").as_deref(), Some("Notion"));
        let new = "\"Slack\" ASN:0x0-0x136b36a: (in front) \n    bundleID=[ NULL ] \n Arch=!!none \n";
        assert_eq!(parse_app_name(new).as_deref(), Some("Slack"));
        assert_eq!(parse_app_name(""), None);
    }
}

/// Let a floating window (menu bar panel, HUD) appear on every Space, including over
/// full-screen apps, instead of macOS switching back to the desktop to show it.
/// Call on the main thread with the window's NSWindow pointer.
pub fn float_over_fullscreen(ns_window: *mut c_void) {
    if ns_window.is_null() {
        return;
    }
    // NSWindowCollectionBehavior: CanJoinAllSpaces | Stationary | FullScreenAuxiliary | IgnoresCycle
    const BEHAVIOR: usize = 1 << 0 | 1 << 4 | 1 << 8 | 1 << 6;
    const NS_STATUS_WINDOW_LEVEL: isize = 25;
    unsafe {
        type MsgUsize = extern "C" fn(*mut c_void, *mut c_void, usize);
        type MsgIsize = extern "C" fn(*mut c_void, *mut c_void, isize);
        let send_u: MsgUsize = std::mem::transmute(objc_msgSend as *const ());
        let send_i: MsgIsize = std::mem::transmute(objc_msgSend as *const ());
        send_u(ns_window, sel_registerName(c"setCollectionBehavior:".as_ptr()), BEHAVIOR);
        send_i(ns_window, sel_registerName(c"setLevel:".as_ptr()), NS_STATUS_WINDOW_LEVEL);
    }
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
