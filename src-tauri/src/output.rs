//! Getting text into the app in front: paste, type, terminal-safe paste, or clipboard only.
//!
//! `plan` turns settings into a list of steps (pure, unit tested); `run` carries them out.

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

use crate::settings::Settings;

const TERMINALS: &[&str] = &["Terminal", "iTerm2", "iTerm", "Ghostty", "Warp", "Alacritty", "kitty", "WezTerm"];
const KEY_V: u32 = 9; // kVK_ANSI_V
/// How long the target app gets to read the clipboard before the old contents go back.
const RESTORE_AFTER_MS: u64 = 200;
/// Gap between the text landing and the submit key, so the app sees them in order.
const SUBMIT_AFTER_MS: u64 = 40;

#[derive(Debug, Clone, PartialEq)]
pub enum Keys {
    Paste { terminal: bool },
    Type(String),
    Submit { cmd: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    SaveClipboard,
    SetClipboard(String),
    Wait(u64),
    Press(Keys),
    RestoreClipboard,
}

/// Whatever was on the clipboard before we borrowed it.
pub enum Saved {
    Files(Vec<PathBuf>),
    Text { text: String, html: Option<String> },
    Image(ImageData<'static>),
    Nothing,
}

impl Saved {
    pub fn take(c: &mut Clipboard) -> Saved {
        if let Ok(files) = c.get().file_list() {
            if !files.is_empty() {
                return Saved::Files(files);
            }
        }
        if let Ok(text) = c.get_text() {
            return Saved::Text { html: c.get().html().ok(), text };
        }
        if let Ok(img) = c.get_image() {
            return Saved::Image(img);
        }
        Saved::Nothing
    }

    pub fn put_back(self, c: &mut Clipboard) -> Result<(), arboard::Error> {
        match self {
            Saved::Files(f) => c.set().file_list(&f),
            Saved::Text { text, html: Some(html) } => c.set_html(html, Some(text)),
            Saved::Text { text, html: None } => c.set_text(text),
            Saved::Image(img) => c.set_image(img),
            Saved::Nothing => c.clear(),
        }
    }
}

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

pub fn press(e: &mut Enigo, k: Keys) -> Result<(), String> {
    match k {
        Keys::Paste { terminal: true } => chord(e, &[Key::Control, Key::Shift], Key::Other(KEY_V)),
        Keys::Paste { terminal: false } => chord(e, &[Key::Meta], Key::Other(KEY_V)),
        Keys::Type(t) => e.text(&t).map_err(|x| x.to_string()),
        Keys::Submit { cmd: true } => chord(e, &[Key::Meta], Key::Return),
        Keys::Submit { cmd: false } => chord(e, &[], Key::Return),
    }
}

pub fn resolve_method(s: &Settings, front_app: &str) -> String {
    if s.auto_terminal && s.paste_method == "paste" && TERMINALS.iter().any(|t| front_app.eq_ignore_ascii_case(t)) {
        "terminal".into()
    } else {
        s.paste_method.clone()
    }
}

/// What delivering `text` will do, step by step. Returns the method actually used too.
pub fn plan(text: &str, s: &Settings, front_app: &str) -> (String, Vec<Step>) {
    let method = resolve_method(s, front_app);
    let mut text = text.to_string();
    if s.trailing_space && !text.is_empty() && method != "clipboard" {
        text.push(' ');
    }
    let mut steps = Vec::new();
    match method.as_str() {
        "clipboard" => steps.push(Step::SetClipboard(text)),
        "type" => steps.push(Step::Press(Keys::Type(text))),
        _ => {
            let restore = s.clipboard_handling == "restore";
            if restore {
                steps.push(Step::SaveClipboard);
            }
            steps.push(Step::SetClipboard(text));
            steps.push(Step::Wait(s.paste_delay_ms as u64));
            steps.push(Step::Press(Keys::Paste { terminal: method == "terminal" }));
            if restore {
                steps.push(Step::Wait(RESTORE_AFTER_MS));
                steps.push(Step::RestoreClipboard);
            }
        }
    }
    let submit = match s.auto_submit.as_str() {
        "return" => Some(false),
        "cmdReturn" => Some(true),
        _ => None,
    };
    if let (Some(cmd), true) = (submit, method != "clipboard") {
        steps.push(Step::Wait(SUBMIT_AFTER_MS));
        steps.push(Step::Press(Keys::Submit { cmd }));
    }
    (method, steps)
}

/// Carry out `steps`. Returns how long the clipboard restore took, if there was one.
pub fn run(steps: Vec<Step>, mut keys: impl FnMut(Keys) -> Result<(), String>) -> Result<Option<u128>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let mut saved = None;
    let mut restore_ms = None;
    for step in steps {
        match step {
            Step::SaveClipboard => saved = Some(Saved::take(&mut clipboard)),
            Step::SetClipboard(t) => clipboard.set_text(t).map_err(|e| e.to_string())?,
            Step::Wait(ms) => std::thread::sleep(Duration::from_millis(ms)),
            Step::Press(k) => keys(k)?,
            Step::RestoreClipboard => {
                let t0 = Instant::now();
                if let Some(prev) = saved.take() {
                    let _ = prev.put_back(&mut clipboard);
                }
                restore_ms = Some(t0.elapsed().as_millis());
            }
        }
    }
    Ok(restore_ms)
}

/// Deliver `text`. Returns the method actually used and how long clipboard restore took.
pub fn deliver(app: &AppHandle, text: &str, s: &Settings, front_app: &str) -> Result<(String, Option<u128>), String> {
    let (method, steps) = plan(text, s, front_app);
    let restore_ms = run(steps, |k| on_main(app, move |e| press(e, k)))?;
    Ok((method, restore_ms))
}

pub fn copy(text: &str) -> Result<(), String> {
    Clipboard::new()
        .and_then(|mut c| c.set_text(text.to_string()))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Keys::*;
    use Step::*;

    fn with(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings::default();
        f(&mut s);
        s
    }

    #[test]
    fn terminal_apps_switch_to_terminal_safe() {
        let s = Settings::default();
        assert_eq!(resolve_method(&s, "Ghostty"), "terminal");
        assert_eq!(resolve_method(&s, "iTerm2"), "terminal");
        assert_eq!(resolve_method(&s, "Notion"), "paste");
        let off = Settings { auto_terminal: false, ..Settings::default() };
        assert_eq!(resolve_method(&off, "Terminal"), "paste");
        let clip = Settings { paste_method: "clipboard".into(), ..Settings::default() };
        assert_eq!(resolve_method(&clip, "Terminal"), "clipboard");
        // Auto switch only overrides plain paste, never a method the user picked on purpose.
        let typed = Settings { paste_method: "type".into(), ..Settings::default() };
        assert_eq!(resolve_method(&typed, "Warp"), "type");
    }

    #[test]
    fn paste_with_restore() {
        let s = with(|s| s.paste_delay_ms = 10);
        let (m, steps) = plan("hi", &s, "Notes");
        assert_eq!(m, "paste");
        assert_eq!(
            steps,
            vec![SaveClipboard, SetClipboard("hi ".into()), Wait(10), Press(Paste { terminal: false }), Wait(200), RestoreClipboard]
        );
    }

    #[test]
    fn keep_transcript_skips_save_and_restore() {
        let s = with(|s| s.clipboard_handling = "keep".into());
        let (_, steps) = plan("hi", &s, "Notes");
        assert!(!steps.contains(&SaveClipboard) && !steps.contains(&RestoreClipboard));
        assert_eq!(steps[0], SetClipboard("hi ".into()));
    }

    #[test]
    fn terminal_safe_uses_ctrl_shift_v() {
        let (m, steps) = plan("ls", &Settings::default(), "Ghostty");
        assert_eq!(m, "terminal");
        assert!(steps.contains(&Press(Paste { terminal: true })));
        let picked = with(|s| s.paste_method = "terminal".into());
        assert!(plan("ls", &picked, "Notes").1.contains(&Press(Paste { terminal: true })));
    }

    #[test]
    fn type_never_touches_clipboard() {
        let s = with(|s| s.paste_method = "type".into());
        let (m, steps) = plan("hi", &s, "Notes");
        assert_eq!(m, "type");
        assert_eq!(steps, vec![Press(Type("hi ".into()))]);
    }

    #[test]
    fn clipboard_only_sends_no_keys_and_no_space() {
        let s = with(|s| {
            s.paste_method = "clipboard".into();
            s.auto_submit = "return".into();
        });
        let (m, steps) = plan("hi", &s, "Terminal");
        assert_eq!(m, "clipboard");
        assert_eq!(steps, vec![SetClipboard("hi".into())]);
    }

    #[test]
    fn auto_submit_comes_last() {
        for (opt, cmd) in [("return", false), ("cmdReturn", true)] {
            for method in ["paste", "type", "terminal"] {
                let s = with(|s| {
                    s.auto_submit = opt.into();
                    s.paste_method = method.into();
                });
                let steps = plan("hi", &s, "Slack").1;
                assert_eq!(steps.last(), Some(&Press(Submit { cmd })), "{opt} with {method}");
                assert_eq!(steps[steps.len() - 2], Wait(40));
            }
        }
        let off = plan("hi", &Settings::default(), "Slack").1;
        assert!(!off.iter().any(|s| matches!(s, Press(Submit { .. }))));
    }

    #[test]
    fn trailing_space() {
        let off = with(|s| s.trailing_space = false);
        assert_eq!(plan("hi", &off, "Notes").1[1], SetClipboard("hi".into()));
        assert_eq!(plan("hi", &Settings::default(), "Notes").1[1], SetClipboard("hi ".into()));
        // Nothing to say, nothing to pad.
        assert_eq!(plan("", &Settings::default(), "Notes").1[1], SetClipboard("".into()));
    }

    #[test]
    fn paste_delay_sits_between_clipboard_and_keys() {
        for ms in [0, 10, 60, 400] {
            let s = with(|s| s.paste_delay_ms = ms);
            let steps = plan("hi", &s, "Notes").1;
            let i = steps.iter().position(|x| *x == Press(Paste { terminal: false })).unwrap();
            assert_eq!(steps[i - 1], Wait(ms as u64));
            assert!(matches!(steps[i - 2], SetClipboard(_)));
        }
    }

    // Uses the real pasteboard (overwrites it). `cargo test --lib clipboard_round_trip -- --ignored`
    #[test]
    #[ignore]
    fn clipboard_round_trip() {
        let mut c = Clipboard::new().unwrap();
        let mut check = |label: &str, setup: &dyn Fn(&mut Clipboard), verify: &dyn Fn(&mut Clipboard)| {
            setup(&mut c);
            let saved = Saved::take(&mut c);
            c.set_text("TRANSCRIPT").unwrap();
            saved.put_back(&mut c).unwrap();
            verify(&mut c);
            println!("ok: {label}");
        };
        check("text", &|c| c.set_text("old text").unwrap(), &|c| assert_eq!(c.get_text().unwrap(), "old text"));
        check(
            "html",
            &|c| c.set_html("<b>bold</b>", Some("bold")).unwrap(),
            &|c| {
                assert_eq!(c.get_text().unwrap(), "bold");
                assert!(c.get().html().unwrap().contains("<b>bold</b>"));
            },
        );
        let px: Vec<u8> = (0..4 * 4 * 4).map(|i| (i * 7) as u8).collect();
        let img = ImageData { width: 4, height: 4, bytes: px.clone().into() };
        check(
            "image",
            &|c| c.set_image(img.clone()).unwrap(),
            &|c| {
                let got = c.get_image().unwrap();
                assert_eq!((got.width, got.height), (4, 4));
                assert!(c.get_text().is_err(), "transcript must not linger");
            },
        );
        let file = std::env::temp_dir().join("koe-clip-test.txt");
        std::fs::write(&file, "x").unwrap();
        let f2 = file.clone();
        check(
            "files",
            &move |c| c.set().file_list(&[&f2]).unwrap(),
            &|c| assert_eq!(c.get().file_list().unwrap()[0].file_name(), file.file_name()),
        );
        check("empty", &|c| c.clear().unwrap(), &|c| assert!(c.get_text().is_err()));
    }

    fn osa(script: &str) -> String {
        let out = std::process::Command::new("osascript").arg("-e").arg(script).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string()
    }

    fn deliver_here(text: &str, s: &Settings, front_app: &str) -> String {
        let (method, steps) = plan(text, s, front_app);
        let mut e = Enigo::new(&EnigoSettings::default()).unwrap();
        run(steps, |k| press(&mut e, k)).unwrap();
        method
    }

    fn clip() -> String {
        Clipboard::new().unwrap().get_text().unwrap_or_default()
    }

    fn set_clip(t: &str) {
        Clipboard::new().unwrap().set_text(t).unwrap();
    }

    /// Real keystrokes into TextEdit, Terminal and iTerm. Takes over the keyboard for ~30 s:
    /// don't touch anything while it runs. Needs Accessibility for the terminal you run it from.
    /// `cargo test --lib output_e2e -- --ignored --nocapture --test-threads=1`
    #[test]
    #[ignore]
    fn output_e2e() {
        assert!(crate::macos::accessibility_trusted(), "give this terminal Accessibility access first");
        let wait = |ms| std::thread::sleep(Duration::from_millis(ms));
        let mut results: Vec<(String, bool, String)> = Vec::new();

        // TextEdit: one fresh document per case, read back with AppleScript.
        let textedit = |s: Settings, old_clip: &str, text: &str| -> (String, String) {
            osa(r#"tell application "TextEdit" to activate
                   tell application "TextEdit" to make new document"#);
            wait(800);
            set_clip(old_clip);
            deliver_here(text, &s, "TextEdit");
            wait(400);
            let doc = osa(r#"tell application "TextEdit" to get text of document 1"#);
            let c = clip();
            osa(r#"tell application "TextEdit" to close document 1 saving no"#);
            (doc, c)
        };
        let mut case = |name: &str, ok: bool, detail: String| results.push((name.into(), ok, detail));

        let (doc, c) = textedit(Settings::default(), "OLD", "hello");
        case("Paste ⌘V + Restore + trailing space", doc == "hello " && c == "OLD", format!("doc={doc:?} clip={c:?}"));

        let (doc, c) = textedit(with(|s| s.clipboard_handling = "keep".into()), "OLD", "kept");
        case("Keep transcript", doc == "kept " && c == "kept ", format!("doc={doc:?} clip={c:?}"));

        let (doc, c) = textedit(with(|s| s.paste_method = "type".into()), "OLD", "typed out");
        case("Type it out (clipboard untouched)", doc == "typed out " && c == "OLD", format!("doc={doc:?} clip={c:?}"));

        let (doc, c) = textedit(with(|s| s.paste_method = "clipboard".into()), "OLD", "only");
        case("Clipboard only (nothing typed)", doc.is_empty() && c == "only", format!("doc={doc:?} clip={c:?}"));

        let (doc, _) = textedit(with(|s| s.trailing_space = false), "OLD", "nospace");
        case("Trailing space off", doc == "nospace", format!("doc={doc:?}"));

        let (doc, _) = textedit(with(|s| s.auto_submit = "return".into()), "OLD", "line");
        case("Auto submit Return", doc == "line \n", format!("doc={doc:?}"));

        for ms in [0, 10] {
            let (doc, c) = textedit(with(|s| s.paste_delay_ms = ms), "STALE", "fresh");
            case(&format!("Paste delay {ms} ms (no stale text)"), doc == "fresh " && c == "STALE", format!("doc={doc:?} clip={c:?}"));
        }

        // Terminals: type an echo, auto submit, and look for its output.
        let marker = format!("koe{}", std::process::id());
        let mut term = |app: &str, front: &str, open: &str, read: &str, close: &str, s: Settings| {
            osa(open);
            wait(1500);
            let cmd = format!("echo {marker}-$((40+2))");
            let method = deliver_here(&cmd, &s, front);
            wait(800);
            let out = osa(read);
            osa(close);
            let ok = out.contains(&format!("{marker}-42"));
            let label = format!("{app} via {} (auto terminal {})", pipeline_label(&method), if s.auto_terminal { "on" } else { "off" });
            results.push((label, ok, format!("last lines: {:?}", out.lines().rev().take(3).collect::<Vec<_>>())));
        };
        let submit = || with(|s| s.auto_submit = "return".into());
        let off = || with(|s| {
            s.auto_submit = "return".into();
            s.auto_terminal = false;
        });
        let t_open = r#"tell application "Terminal" to activate
                        tell application "Terminal" to do script """#;
        let t_read = r#"tell application "Terminal" to get contents of selected tab of front window"#;
        let t_close = r#"tell application "Terminal" to do script "exit" in front window"#;
        term("Terminal", "Terminal", t_open, t_read, t_close, submit());
        term("Terminal", "Terminal", t_open, t_read, t_close, off());
        if std::path::Path::new("/Applications/iTerm.app").exists() {
            let i_open = r#"tell application "iTerm" to activate
                            tell application "iTerm" to create window with default profile"#;
            let i_read = r#"tell application "iTerm" to tell current session of current window to get contents"#;
            let i_close = r#"tell application "iTerm" to close current window"#;
            term("iTerm", "iTerm2", i_open, i_read, i_close, submit());
            term("iTerm", "iTerm2", i_open, i_read, i_close, off());
        }

        println!();
        for (name, ok, detail) in &results {
            println!("{} {name}  {}", if *ok { "PASS" } else { "FAIL" }, if *ok { String::new() } else { detail.clone() });
        }
        assert!(results.iter().all(|r| r.1), "some output cases failed");
    }

    fn pipeline_label(m: &str) -> &'static str {
        crate::pipeline::method_label(m)
    }
}
