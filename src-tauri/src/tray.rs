//! Menu bar icon: the five voice bars (logo 2c), animated while you speak.

use parking_lot::Mutex;
use std::sync::OnceLock;
use std::time::Instant;
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition};

const ID: &str = "vak";
const REST: [f32; 5] = [0.34, 0.62, 1.0, 0.52, 0.26];

fn bars_icon(levels: [f32; 5]) -> Image<'static> {
    // 18pt template image at @2x.
    let (w, h) = (36u32, 36u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let bar_w = 4u32;
    let gap = 3u32;
    let total = bar_w * 5 + gap * 4;
    let x0 = (w - total) / 2;
    for (i, lv) in levels.iter().enumerate() {
        let bh = ((lv.clamp(0.15, 1.0)) * 28.0).round() as u32;
        let top = (h - bh) / 2;
        let left = x0 + i as u32 * (bar_w + gap);
        for y in top..top + bh {
            for x in left..left + bar_w {
                // Soften the corners of each bar.
                let corner = (y == top || y == top + bh - 1) && (x == left || x == left + bar_w - 1);
                let idx = ((y * w + x) * 4) as usize;
                rgba[idx + 3] = if corner { 110 } else { 255 };
            }
        }
    }
    Image::new_owned(rgba, w, h)
}

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    TrayIconBuilder::with_id(ID)
        .icon(bars_icon(REST))
        .icon_as_template(true)
        .tooltip("Vāk")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = event {
                toggle_panel(tray.app_handle(), rect);
            }
        })
        .build(app)
}

fn toggle_panel(app: &AppHandle, rect: tauri::Rect) {
    let Some(panel) = app.get_webview_window("panel") else { return };
    if panel.is_visible().unwrap_or(false) {
        let _ = panel.hide();
        return;
    }
    let scale = panel.scale_factor().unwrap_or(2.0);
    let pos = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);
    let pw = 330.0 * scale;
    let x = pos.x + size.width / 2.0 - pw / 2.0;
    let y = pos.y + size.height + 6.0 * scale;
    let _ = panel.set_position(PhysicalPosition::new(x as i32, y as i32));
    let _ = panel.show();
    let _ = panel.set_focus();
}

struct Anim {
    last: Instant,
    phase: usize,
}

static ANIM: OnceLock<Mutex<Anim>> = OnceLock::new();

/// Called from the level meter while recording; throttled to ~8 fps.
pub fn animate(app: &AppHandle, db: f32) {
    let anim = ANIM.get_or_init(|| Mutex::new(Anim { last: Instant::now(), phase: 0 }));
    let mut a = anim.lock();
    if a.last.elapsed().as_millis() < 120 {
        return;
    }
    a.last = Instant::now();
    a.phase = (a.phase + 1) % 5;
    let loud = ((db + 60.0) / 50.0).clamp(0.0, 1.0);
    let mut levels = [0f32; 5];
    for (i, l) in levels.iter_mut().enumerate() {
        let wobble = ((i + a.phase) % 5) as f32 / 4.0;
        *l = 0.2 + loud * (0.45 + 0.55 * (REST[i] * 0.6 + wobble * 0.4));
    }
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_icon(Some(bars_icon(levels)));
        let _ = tray.set_icon_as_template(true);
    }
}

pub fn reset(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_icon(Some(bars_icon(REST)));
        let _ = tray.set_icon_as_template(true);
    }
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_visible(visible);
    }
}
