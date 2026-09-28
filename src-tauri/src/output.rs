//! Getting text into the app in front: paste, type, terminal-safe paste, or clipboard only.

use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use std::sync::mpsc;
use std::time::Duration;
use tauri::AppHandle;

use crate::settings::Settings;

const TERMINALS: &[&str] = &["Terminal", "iTerm2", "iTerm", "Ghostty", "Warp", "Alacritty", "kitty", "WezTerm"];
const KEY_V: u32 = 9; // kVK_ANSI_V

/// enigo's macOS backend must run on the main thread; run it there and wait.
fn on_main<F>(app: &AppHandle, f: F) -> Result<(), String>
where
    F: FnOnce(&mut Enigo) -> Result<(), String> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let res = Enigo::new(&EnigoSettings::default())
            .map_err(|e| e.to_string())
            .and_then(|mut e| f(&mut e));
        let _ = tx.send(res);
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(10)).map_err(|_| "keyboard timed out".to_string())?
}

fn chord(e: &mut Enigo, mods: &[Key], key: Key) -> Result<(), String> {
    let err = |x: enigo::InputError| x.to_string();
    for m in mods {
        e.key(*m, Direction::Press).map_err(err)?;
    }
    e.key(key, Direction::Click).map_err(err)?;
    for m in mods.iter().rev() {
        e.key(*m, Direction::Release).map_err(err)?;
    }
    Ok(())
}

pub fn resolve_method(s: &Settings, front_app: &str) -> String {
    if s.auto_terminal && s.paste_method == "paste" && TERMINALS.iter().any(|t| front_app.eq_ignore_ascii_case(t)) {
        "terminal".into()
    } else {
        s.paste_method.clone()
    }
}

/// Deliver `text`. Returns the method actually used and how long clipboard restore took.
pub fn deliver(app: &AppHandle, text: &str, s: &Settings, front_app: &str) -> Result<(String, Option<u128>), String> {
    let method = resolve_method(s, front_app);
    let mut text = text.to_string();
    if s.trailing_space && !text.is_empty() && method != "clipboard" {
        text.push(' ');
    }
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let mut restore_ms = None;

    match method.as_str() {
        "clipboard" => {
            clipboard.set_text(text).map_err(|e| e.to_string())?;
        }
        "type" => {
            let t = text.clone();
            on_main(app, move |e| e.text(&t).map_err(|x| x.to_string()))?;
        }
        _ => {
            let previous = clipboard.get_text().ok();
            clipboard.set_text(text.clone()).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(s.paste_delay_ms as u64));
            let terminal = method == "terminal";
            on_main(app, move |e| {
                if terminal {
                    chord(e, &[Key::Control, Key::Shift], Key::Other(KEY_V))
                } else {
                    chord(e, &[Key::Meta], Key::Other(KEY_V))
                }
            })?;
            if s.clipboard_handling == "restore" {
                std::thread::sleep(Duration::from_millis(200));
                let t0 = std::time::Instant::now();
                if let Some(prev) = previous {
                    let _ = clipboard.set_text(prev);
                } else {
                    let _ = clipboard.clear();
                }
                restore_ms = Some(t0.elapsed().as_millis());
            }
        }
    }

    match s.auto_submit.as_str() {
        "return" if method != "clipboard" => {
            std::thread::sleep(Duration::from_millis(40));
            on_main(app, |e| chord(e, &[], Key::Return))?;
        }
        "cmdReturn" if method != "clipboard" => {
            std::thread::sleep(Duration::from_millis(40));
            on_main(app, |e| chord(e, &[Key::Meta], Key::Return))?;
        }
        _ => {}
    }
    Ok((method, restore_ms))
}

pub fn copy(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text.to_string()))
        .map_err(|e| e.to_string())
}
