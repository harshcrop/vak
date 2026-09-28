//! The dictation flow: shortcut → capture → transcribe → clean up → (LLM) → paste → history.

use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

use crate::logs::log;
use crate::settings::Settings;
use crate::{audio, history, macos, models, output, postprocess, text, tray, AppState};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Phase {
    #[default]
    Idle,
    Recording,
    Transcribing,
}

#[derive(Default)]
pub struct Session {
    pub phase: Phase,
    pub started: Option<Instant>,
    pub post_process: bool,
    pub front_app: String,
    pub prev_volume: Option<u32>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Hud {
    #[serde(rename_all = "camelCase")]
    Listening { position: String, post_process: bool },
    #[serde(rename_all = "camelCase")]
    Transcribing { position: String, model: String, app: String, duration_ms: u64, post_process: bool },
    #[serde(rename_all = "camelCase")]
    Done { position: String, words: u32, text: String, method: String, realtime: f32 },
    #[serde(rename_all = "camelCase")]
    Error { position: String, message: String },
    #[serde(rename_all = "camelCase")]
    Hint { position: String, message: String },
}

fn escape() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

pub fn register_shortcuts(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let s = app.state::<AppState>().settings.read().clone();
    if !s.enabled {
        log("hotkey", "shortcuts paused");
        return;
    }
    for (label, spec) in [("transcribe", &s.shortcut), ("post-process", &s.pp_shortcut)] {
        if label == "post-process" && !(s.experimental && s.post_process.enabled) {
            continue;
        }
        match spec.parse::<Shortcut>() {
            Ok(sc) => match gs.register(sc) {
                Ok(_) => log("hotkey", format!("{label} shortcut {spec} registered")),
                Err(e) => log("hotkey", format!("could not register {spec}: {e}")),
            },
            Err(e) => log("hotkey", format!("invalid shortcut {spec}: {e}")),
        }
    }
}

pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) {
    let s = app.state::<AppState>().settings.read().clone();
    if *shortcut == escape() {
        if state == ShortcutState::Pressed {
            cancel(app);
        }
        return;
    }
    let is_main = s.shortcut.parse::<Shortcut>().map(|m| m == *shortcut).unwrap_or(false);
    let is_pp = s.pp_shortcut.parse::<Shortcut>().map(|m| m == *shortcut).unwrap_or(false);
    if !is_main && !is_pp {
        return;
    }
    let phase = app.state::<AppState>().session.lock().phase;
    match (s.recording_mode.as_str(), state) {
        ("toggle", ShortcutState::Pressed) => {
            if phase == Phase::Idle {
                start(app, is_pp);
            } else if phase == Phase::Recording {
                finish(app);
            }
        }
        ("toggle", _) => {}
        (_, ShortcutState::Pressed) => {
            if phase == Phase::Idle {
                start(app, is_pp);
            }
        }
        (_, ShortcutState::Released) => {
            if phase == Phase::Recording {
                finish(app);
            }
        }
    }
}

pub fn emit_hud(app: &AppHandle, hud: Hud) {
    let _ = app.emit("hud", hud);
}

pub fn show_hud(app: &AppHandle, position: &str) {
    let Some(win) = app.get_webview_window("hud") else { return };
    if position == "hidden" {
        return;
    }
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let (mw, mh) = (m.size().width as f64, m.size().height as f64);
        let (w, h) = (480.0 * scale, 180.0 * scale);
        let x = m.position().x as f64 + (mw - w) / 2.0;
        let y = if position == "top" {
            m.position().y as f64 + 34.0 * scale
        } else {
            m.position().y as f64 + mh - h - 64.0 * scale
        };
        let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
    let _ = win.set_ignore_cursor_events(true);
    let _ = win.show();
}

fn hide_hud_later(app: &AppHandle, after: Duration) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(after);
        if app.state::<AppState>().session.lock().phase == Phase::Idle {
            if let Some(w) = app.get_webview_window("hud") {
                let _ = w.hide();
            }
        }
    });
}

pub fn start(app: &AppHandle, post_process: bool) {
    let st = app.state::<AppState>();
    let s = st.settings.read().clone();
    {
        let mut sess = st.session.lock();
        if sess.phase != Phase::Idle {
            return;
        }
        sess.phase = Phase::Recording;
    }
    let front = macos::frontmost_app();
    log("hotkey", format!("{} down · capture started{}", pretty(&s.shortcut), if post_process { " · post-process" } else { "" }));

    if s.audio_feedback {
        audio::chime(s.output_device.clone(), s.chime_volume, true);
    }
    let handle = app.clone();
    let result = st.recorder.start(s.input_device.clone(), true, move |db| {
        let _ = handle.emit("level", db);
        tray::animate(&handle, db);
    });
    match result {
        Ok(desc) => log("audio", format!("device={desc}")),
        Err(e) => {
            log("audio", format!("could not open microphone: {e}"));
            st.session.lock().phase = Phase::Idle;
            show_hud(app, &s.overlay_position);
            emit_hud(app, Hud::Error { position: s.overlay_position.clone(), message: format!("Microphone unavailable — {e}") });
            hide_hud_later(app, Duration::from_millis(2500));
            return;
        }
    }
    let prev_volume = if s.mute_other_audio {
        let v = macos::output_volume();
        macos::set_output_volume(v.unwrap_or(50) / 5);
        v
    } else {
        None
    };
    {
        let mut sess = st.session.lock();
        sess.started = Some(Instant::now());
        sess.post_process = post_process;
        sess.front_app = front;
        sess.prev_volume = prev_volume;
    }
    if s.cancel_with_esc {
        let _ = app.global_shortcut().register(escape());
    }
    show_hud(app, &s.overlay_position);
    emit_hud(app, Hud::Listening { position: s.overlay_position.clone(), post_process });
}

fn end_capture(app: &AppHandle) -> Option<(audio::Captured, Session)> {
    let st = app.state::<AppState>();
    let s = st.settings.read().clone();
    let _ = app.global_shortcut().unregister(escape());
    let captured = st.recorder.stop().ok();
    let sess = std::mem::take(&mut *st.session.lock());
    if let Some(v) = sess.prev_volume {
        macos::set_output_volume(v);
    }
    if s.audio_feedback {
        audio::chime(s.output_device.clone(), s.chime_volume, false);
    }
    tray::reset(app);
    captured.map(|c| (c, sess))
}

pub fn cancel(app: &AppHandle) {
    if app.state::<AppState>().session.lock().phase != Phase::Recording {
        return;
    }
    end_capture(app);
    log("hotkey", "cancelled with Esc · nothing inserted");
    if let Some(w) = app.get_webview_window("hud") {
        let _ = w.hide();
    }
}

pub fn finish(app: &AppHandle) {
    let st = app.state::<AppState>();
    if st.session.lock().phase != Phase::Recording {
        return;
    }
    let s = st.settings.read().clone();
    let Some((captured, sess)) = end_capture(app) else {
        return;
    };
    let pos = s.overlay_position.clone();
    let dur = captured.duration_ms();
    if dur < 300 {
        let hint = if s.recording_mode == "ptt" { format!("Hold {} while you speak", pretty(&s.shortcut)) } else { "Too short — try again".into() };
        emit_hud(app, Hud::Hint { position: pos, message: hint });
        hide_hud_later(app, Duration::from_millis(1400));
        return;
    }
    let Some((model_id, model_path)) = resolve_model(app, &s) else {
        emit_hud(app, Hud::Error { position: pos, message: "No model downloaded — open Vāk › Models".into() });
        hide_hud_later(app, Duration::from_millis(2600));
        return;
    };
    st.session.lock().phase = Phase::Transcribing;
    let model_name = models::find(&model_id).map(|m| m.name).unwrap_or("Whisper");
    emit_hud(
        app,
        Hud::Transcribing {
            position: pos.clone(),
            model: model_name.to_string(),
            app: sess.front_app.clone(),
            duration_ms: dur,
            post_process: sess.post_process,
        },
    );

    let app = app.clone();
    std::thread::spawn(move || {
        let st = app.state::<AppState>();
        let t0 = Instant::now();
        let prompt = s.vocabulary.join(", ");
        let result = st.engine.transcribe(&model_id, &model_path, &captured.samples, &s.language, &prompt);
        let latency = t0.elapsed().as_millis() as u64;
        let raw = match result {
            Ok(t) => t,
            Err(e) => {
                log("asr", format!("failed: {e}"));
                st.session.lock().phase = Phase::Idle;
                emit_hud(&app, Hud::Error { position: pos, message: e });
                hide_hud_later(&app, Duration::from_millis(2600));
                return;
            }
        };
        let mut text = text::clean(&raw, &s, &sess.front_app);
        let words_raw = text::word_count(&text);
        log("asr", format!("{:.1} s audio → {words_raw} words in {latency} ms", dur as f32 / 1000.0));

        let mut post_processed = false;
        if sess.post_process && !text.is_empty() {
            let t1 = Instant::now();
            let recipe = s.post_process.recipes.get(s.post_process.active_recipe).map(|r| r.prompt.clone()).unwrap_or_default();
            match postprocess::run(&s.post_process, &recipe, &text) {
                Ok(t) if !t.is_empty() => {
                    log("llm", format!("post-processed in {} ms", t1.elapsed().as_millis()));
                    text = t;
                    post_processed = true;
                }
                Ok(_) => log("llm", "empty response · using raw transcript"),
                Err(e) => log("llm", format!("failed · using raw transcript: {e}")),
            }
        }

        if text.is_empty() {
            st.session.lock().phase = Phase::Idle;
            emit_hud(&app, Hud::Hint { position: pos, message: "Didn’t catch anything".into() });
            hide_hud_later(&app, Duration::from_millis(1400));
            return;
        }

        let method = match output::deliver(&app, &text, &s, &sess.front_app) {
            Ok((method, restore)) => {
                log("paste", format!("frontmost={} · method={} · delay={} ms", sess.front_app, method_label(&method), s.paste_delay_ms));
                if let Some(ms) = restore {
                    log("ok", format!("clipboard restored in {ms} ms"));
                }
                method
            }
            Err(e) => {
                log("paste", format!("failed: {e} · text left on clipboard"));
                let _ = output::copy(&text);
                "clipboard".into()
            }
        };

        // History + audio.
        let id = chrono::Utc::now().timestamp_millis();
        let audio_file = if s.delete_audio_after_days > 0 {
            let name = format!("{id}.wav");
            let path = st.history.lock().recordings.join(&name);
            audio::write_wav(&path, &captured.samples).ok().map(|_| name)
        } else {
            None
        };
        let words = text::word_count(&text);
        let entry = history::Entry {
            id: id.to_string(),
            created: id,
            app: sess.front_app.clone(),
            duration_ms: dur,
            words,
            text: text.clone(),
            raw,
            post_processed,
            starred: false,
            audio: audio_file,
            peaks: audio::peaks(&captured.samples, 48),
            model: model_name.to_string(),
            latency_ms: latency,
        };
        st.history.lock().push(entry, s.keep_transcripts);
        let _ = app.emit("history-changed", ());

        st.session.lock().phase = Phase::Idle;
        let realtime = if latency > 0 { dur as f32 / latency as f32 } else { 0.0 };
        emit_hud(&app, Hud::Done { position: pos, words, text, method, realtime });
        hide_hud_later(&app, Duration::from_millis(1800));
    });
}

/// The active model if it's on disk, otherwise Tiny, otherwise anything installed.
pub fn resolve_model(app: &AppHandle, s: &Settings) -> Option<(String, std::path::PathBuf)> {
    let dir = app.state::<AppState>().paths.models.clone();
    let installed = |id: &str| {
        models::find(id).filter(|m| models::is_installed(&dir, m)).and_then(|m| models::model_path(&dir, &m).map(|p| (m.id.to_string(), p)))
    };
    installed(&s.active_model)
        .or_else(|| installed("whisper-tiny"))
        .or_else(|| models::catalog().iter().find_map(|m| installed(m.id)))
}

pub fn method_label(m: &str) -> &'static str {
    match m {
        "type" => "typed",
        "terminal" => "⌃⇧V",
        "clipboard" => "clipboard",
        _ => "⌘V",
    }
}

pub fn pretty(spec: &str) -> String {
    spec.split('+')
        .map(|p| match p.to_ascii_lowercase().as_str() {
            "alt" | "option" => "⌥".to_string(),
            "shift" => "⇧".to_string(),
            "control" | "ctrl" => "⌃".to_string(),
            "super" | "cmd" | "command" | "meta" | "commandorcontrol" => "⌘".to_string(),
            _ => p.to_string(),
        })
        .collect()
}
