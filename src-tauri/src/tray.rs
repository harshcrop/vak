//! Menu bar icon: the ō mark. The macron is the "recording" light: it tracks your voice while you speak.

use parking_lot::Mutex;
use std::sync::OnceLock;
use std::time::Instant;
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition};

const ID: &str = "koe";
/// Macron length (fraction of full) when idle.
const REST: f32 = 1.0;

/// The ō mark as an 18pt template image at @2x, antialiased by 4×4 supersampling.
fn mark_icon(macron: f32) -> Image<'static> {
    let (w, h) = (36u32, 36u32);
    let (cx, cy, r_out, r_in) = (18.0f32, 22.5f32, 11.0f32, 6.8f32);
    let (my0, my1) = (4.0f32, 8.4f32);
    let half = 10.0 * macron.clamp(0.2, 1.0);
    let rad = (my1 - my0) / 2.0;
    let inside = |x: f32, y: f32| -> bool {
        let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        if d <= r_out && d >= r_in {
            return true;
        }
        // Pill: rectangle with round caps.
        let my = (my0 + my1) / 2.0;
        let dx = ((x - cx).abs() - (half - rad)).max(0.0);
        (dx * dx + (y - my).powi(2)).sqrt() <= rad
    };
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let mut hits = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    if inside(px as f32 + (sx as f32 + 0.5) / 4.0, py as f32 + (sy as f32 + 0.5) / 4.0) {
                        hits += 1;
                    }
                }
            }
            rgba[((py * w + px) * 4 + 3) as usize] = (hits * 255 / 16) as u8;
        }
    }
    Image::new_owned(rgba, w, h)
}

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    TrayIconBuilder::with_id(ID)
        .icon(mark_icon(REST))
        .icon_as_template(true)
        .tooltip("Kōe")
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
    // Use the scale of the screen whose menu bar was clicked, not the one the panel was last on.
    let panel_scale = panel.scale_factor().unwrap_or(2.0);
    let pos = rect.position.to_physical::<f64>(panel_scale);
    let scale = app
        .monitor_from_point(pos.x, pos.y)
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(panel_scale);
    let pos = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);
    let pw = 330.0 * scale;
    let x = pos.x + size.width / 2.0 - pw / 2.0;
    let y = pos.y + size.height + 6.0 * scale;
    let _ = panel.set_position(PhysicalPosition::new(x as i32, y as i32));
    let _ = panel.show();
    let _ = panel.set_focus();
}

static LAST: OnceLock<Mutex<Instant>> = OnceLock::new();

/// Called from the level meter while recording; throttled to ~8 fps.
pub fn animate(app: &AppHandle, db: f32) {
    let last = LAST.get_or_init(|| Mutex::new(Instant::now()));
    let mut t = last.lock();
    if t.elapsed().as_millis() < 120 {
        return;
    }
    *t = Instant::now();
    let loud = ((db + 60.0) / 50.0).clamp(0.0, 1.0);
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_icon(Some(mark_icon(0.25 + 0.75 * loud)));
        let _ = tray.set_icon_as_template(true);
    }
}

pub fn reset(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_icon(Some(mark_icon(REST)));
        let _ = tray.set_icon_as_template(true);
    }
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_visible(visible);
    }
}
