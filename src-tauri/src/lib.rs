mod audio;
mod engine;
mod history;
mod logs;
mod macos;
mod models;
mod output;
mod pipeline;
mod postprocess;
mod settings;
mod text;
mod tray;

use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

use logs::log;
use settings::Settings;

pub struct Paths {
    pub data: PathBuf,
    pub models: PathBuf,
    pub settings: PathBuf,
}

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub paths: Paths,
    pub recorder: audio::Recorder,
    pub monitor: audio::Recorder,
    pub engine: engine::Engine,
    pub downloads: Arc<models::Downloads>,
    pub session: Mutex<pipeline::Session>,
    pub history: Mutex<history::History>,
}

type Res<T> = Result<T, String>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    words_week: u32,
    median_latency: Option<u64>,
    disk_bytes: u64,
    transcripts: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildInfo {
    version: String,
    arch: String,
    os: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    settings: Settings,
    models: Vec<models::ModelStatus>,
    history: Vec<history::Entry>,
    permissions: macos::Permissions,
    input_devices: Vec<String>,
    output_devices: Vec<String>,
    default_input: Option<String>,
    default_output: Option<String>,
    loaded_model: Option<String>,
    stats: Stats,
    build: BuildInfo,
    logs: Vec<logs::LogLine>,
    recordings_dir: String,
}

fn stats(st: &AppState) -> Stats {
    let h = st.history.lock();
    let week_ago = chrono::Utc::now().timestamp_millis() - 7 * 86_400_000;
    Stats {
        words_week: h.words_since(week_ago),
        median_latency: h.median_latency(),
        disk_bytes: h.disk_bytes(),
        transcripts: h.entries.len(),
    }
}

#[tauri::command]
fn bootstrap(app: AppHandle, st: State<AppState>) -> Bootstrap {
    // Take each lock in its own statement: a guard held inside the struct literal
    // lives until the literal ends and would deadlock the later locks.
    let (history, recordings_dir) = {
        let h = st.history.lock();
        (h.entries.clone(), h.recordings.to_string_lossy().to_string())
    };
    let stats = stats(&st);
    Bootstrap {
        settings: st.settings.read().clone(),
        models: models::statuses(&st.paths.models, &st.downloads),
        history,
        permissions: macos::permissions(),
        input_devices: audio::input_devices(),
        output_devices: audio::output_devices(),
        default_input: audio::default_input_name(),
        default_output: audio::default_output_name(),
        loaded_model: st.engine.loaded_id(),
        stats,
        build: BuildInfo {
            version: app.package_info().version.to_string(),
            arch: std::env::consts::ARCH.into(),
            os: format!("macOS {}", macos::os_version()),
        },
        logs: logs::recent(),
        recordings_dir,
    }
}

fn apply_settings(app: &AppHandle, old: &Settings, new: Settings) {
    let st = app.state::<AppState>();
    new.save(&st.paths.settings);
    *st.settings.write() = new.clone();

    if old.shortcut != new.shortcut
        || old.enabled != new.enabled
        || old.pp_shortcut != new.pp_shortcut
        || old.experimental != new.experimental
        || old.post_process.enabled != new.post_process.enabled
    {
        pipeline::register_shortcuts(app);
    }
    if old.show_tray_icon != new.show_tray_icon {
        tray::set_visible(app, new.show_tray_icon);
    }
    if old.launch_at_login != new.launch_at_login {
        let al = app.autolaunch();
        let r = if new.launch_at_login { al.enable() } else { al.disable() };
        if let Err(e) = r {
            log("app", format!("launch at login: {e}"));
        }
    }
    if old.active_model != new.active_model {
        preload(app);
    }
    let _ = app.emit("settings", new);
}

#[tauri::command]
fn update_settings(app: AppHandle, st: State<AppState>, patch: serde_json::Value) -> Res<Settings> {
    let old = st.settings.read().clone();
    let new = old.merged(patch)?;
    apply_settings(&app, &old, new.clone());
    Ok(new)
}

#[tauri::command]
fn reset_general(app: AppHandle, st: State<AppState>) -> Settings {
    let old = st.settings.read().clone();
    let d = Settings::default();
    let new = Settings {
        shortcut: d.shortcut,
        recording_mode: d.recording_mode,
        cancel_with_esc: d.cancel_with_esc,
        input_device: None,
        output_device: None,
        mute_other_audio: d.mute_other_audio,
        audio_feedback: d.audio_feedback,
        chime_volume: d.chime_volume,
        ..old.clone()
    };
    apply_settings(&app, &old, new.clone());
    new
}

fn preload(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let s = app.state::<AppState>().settings.read().clone();
        if let Some((id, path)) = pipeline::resolve_model(&app, &s) {
            if let Err(e) = app.state::<AppState>().engine.ensure_loaded(&id, &path) {
                log("asr", e);
            }
            let _ = app.emit("models-changed", ());
        }
    });
}

#[tauri::command]
fn list_models(st: State<AppState>) -> Vec<models::ModelStatus> {
    models::statuses(&st.paths.models, &st.downloads)
}

#[tauri::command]
async fn download_model(app: AppHandle, id: String) -> Res<()> {
    let (dir, downloads) = {
        let st = app.state::<AppState>();
        (st.paths.models.clone(), st.downloads.clone())
    };
    let _ = app.emit("models-changed", ());
    models::download(app.clone(), dir, id.clone(), downloads).await?;
    let _ = app.emit("models-changed", ());
    // First model on a fresh install becomes the active one.
    let st = app.state::<AppState>();
    let s = st.settings.read().clone();
    let active_installed = models::find(&s.active_model).map(|m| models::is_installed(&st.paths.models, &m)).unwrap_or(false);
    if !active_installed && models::find(&id).map(|m| models::is_installed(&st.paths.models, &m)).unwrap_or(false) {
        let new = Settings { active_model: id, ..s.clone() };
        apply_settings(&app, &s, new);
    } else {
        preload(&app);
    }
    Ok(())
}

#[tauri::command]
fn pause_download(st: State<AppState>, id: String) {
    st.downloads.cancel(&id);
}

#[tauri::command]
fn delete_model(app: AppHandle, st: State<AppState>, id: String) -> Res<()> {
    let m = models::find(&id).ok_or("unknown model")?;
    if st.engine.loaded_id().as_deref() == Some(id.as_str()) {
        st.engine.unload();
    }
    if let Some(p) = models::model_path(&st.paths.models, &m) {
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(p.with_extension("part"));
    }
    log("model", format!("{} deleted", m.name));
    let _ = app.emit("models-changed", ());
    Ok(())
}

#[tauri::command]
fn list_history(st: State<AppState>) -> Vec<history::Entry> {
    st.history.lock().entries.clone()
}

#[tauri::command]
fn delete_entry(app: AppHandle, st: State<AppState>, id: String) {
    st.history.lock().delete(&id);
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
fn star_entry(app: AppHandle, st: State<AppState>, id: String) {
    st.history.lock().toggle_star(&id);
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
fn clear_audio(app: AppHandle, st: State<AppState>) {
    st.history.lock().clear_audio();
    log("app", "recordings cleared");
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
fn get_stats(st: State<AppState>) -> Stats {
    stats(&st)
}

#[tauri::command]
fn copy_text(text: String) -> Res<()> {
    output::copy(&text)
}

#[tauri::command]
fn reveal(st: State<AppState>, what: String) {
    match what.as_str() {
        "logs" => macos::reveal(&logs::log_dir()),
        "models" => macos::reveal(&st.paths.models),
        _ => macos::reveal(&st.history.lock().recordings),
    }
}

#[tauri::command]
fn permissions() -> macos::Permissions {
    macos::permissions()
}

#[tauri::command]
async fn request_microphone() -> macos::Permissions {
    // Opening an input stream is what triggers the system prompt.
    let _ = tauri::async_runtime::spawn_blocking(|| {
        let r = audio::Recorder::default();
        if r.start(None, false, |_| {}).is_ok() {
            std::thread::sleep(Duration::from_millis(400));
            let _ = r.stop();
        }
    })
    .await;
    macos::permissions()
}

#[tauri::command]
fn open_privacy(kind: String) {
    macos::open_privacy_pane(&kind);
}

#[tauri::command]
fn start_monitor(app: AppHandle, st: State<AppState>) -> Res<String> {
    if st.monitor.is_active() {
        return Ok(String::new());
    }
    let device = st.settings.read().input_device.clone();
    let handle = app.clone();
    st.monitor.start(device, false, move |db| {
        let _ = handle.emit("level", db);
    })
}

#[tauri::command]
fn stop_monitor(st: State<AppState>) {
    if st.monitor.is_active() {
        st.monitor.cancel();
    }
}

#[tauri::command]
fn test_chime(st: State<AppState>) {
    let s = st.settings.read();
    audio::chime(s.output_device.clone(), s.chime_volume, true);
}

#[tauri::command]
async fn mic_test(app: AppHandle) -> Res<String> {
    let (device, dir) = {
        let st = app.state::<AppState>();
        let d = st.settings.read().input_device.clone();
        let r = st.history.lock().recordings.clone();
        (d, r)
    };
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let r = audio::Recorder::default();
        let h2 = handle.clone();
        let desc = r.start(device, true, move |db| {
            let _ = h2.emit("level", db);
        })?;
        log("audio", format!("mic test · {desc}"));
        std::thread::sleep(Duration::from_secs(5));
        let c = r.stop()?;
        let path = dir.join("mic-test.wav");
        audio::write_wav(&path, &c.samples)?;
        let peak = c.samples.iter().fold(0f32, |m, v| m.max(v.abs()));
        log("audio", format!("mic test done · peak {:.0} dB", audio::to_db(peak)));
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn diagnostics_report(app: AppHandle, st: State<AppState>) -> Res<()> {
    let s = st.settings.read().clone();
    let p = macos::permissions();
    let stats = stats(&st);
    let mut r = String::new();
    r.push_str(&format!("Vāk {} · {} · macOS {}\n", app.package_info().version, std::env::consts::ARCH, macos::os_version()));
    r.push_str(&format!("model: {} (loaded: {:?})\n", s.active_model, st.engine.loaded_id()));
    r.push_str(&format!("permissions: mic={} accessibility={}\n", p.microphone, p.accessibility));
    r.push_str(&format!("median latency: {:?} ms · transcripts: {}\n", stats.median_latency, stats.transcripts));
    r.push_str(&format!("shortcut: {} · mode: {} · paste: {} · delay: {} ms\n\n", s.shortcut, s.recording_mode, s.paste_method, s.paste_delay_ms));
    for l in logs::recent().iter().rev().take(80).collect::<Vec<_>>().into_iter().rev() {
        r.push_str(&format!("{} {} {}\n", l.ts, l.tag, l.msg));
    }
    output::copy(&r)
}

#[tauri::command]
async fn run_postprocess(app: AppHandle, text: String, prompt: String) -> Res<String> {
    let pp = app.state::<AppState>().settings.read().post_process.clone();
    tauri::async_runtime::spawn_blocking(move || postprocess::run(&pp, &prompt, &text))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn cleanup_last(app: AppHandle) -> Res<String> {
    let st = app.state::<AppState>();
    let pp = st.settings.read().post_process.clone();
    let last = st.history.lock().entries.first().map(|e| e.text.clone()).ok_or("no transcripts yet")?;
    let prompt = pp.recipes.get(pp.active_recipe).map(|r| r.prompt.clone()).unwrap_or_default();
    let out = tauri::async_runtime::spawn_blocking(move || postprocess::run(&pp, &prompt, &last))
        .await
        .map_err(|e| e.to_string())??;
    output::copy(&out)?;
    log("llm", "cleaned up last transcript · copied to clipboard");
    Ok(out)
}

#[tauri::command]
fn start_dictation(app: AppHandle, post_process: bool) {
    if let Some(p) = app.get_webview_window("panel") {
        let _ = p.hide();
    }
    // Give focus back to the previous app before capturing the frontmost name.
    // Tapping the shortcut (or Esc) ends a take started from here.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        pipeline::start(&app, post_process);
    });
}

#[tauri::command]
fn stop_dictation(app: AppHandle) {
    pipeline::finish(&app);
}

#[tauri::command]
fn show_main(app: AppHandle, view: Option<String>) {
    if let Some(p) = app.get_webview_window("panel") {
        let _ = p.hide();
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    if let Some(v) = view {
        let _ = app.emit("navigate", v);
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| pipeline::on_shortcut(app, shortcut, event.state()))
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            let handle = app.handle().clone();
            logs::init(handle.clone());
            whisper_rs::install_logging_hooks();

            let data = app.path().app_data_dir()?;
            let models_dir = data.join("models");
            std::fs::create_dir_all(&models_dir)?;
            let settings_path = data.join("settings.json");
            let s = Settings::load(&settings_path);
            let mut history = history::History::load(&data);
            history.prune(s.keep_transcripts, Some(s.delete_audio_after_days));
            history.save();

            app.manage(AppState {
                settings: RwLock::new(s.clone()),
                paths: Paths { data, models: models_dir, settings: settings_path },
                recorder: Default::default(),
                monitor: Default::default(),
                engine: Default::default(),
                downloads: Default::default(),
                session: Default::default(),
                history: Mutex::new(history),
            });

            tray::build(&handle)?;
            tray::set_visible(&handle, s.show_tray_icon);
            pipeline::register_shortcuts(&handle);
            preload(&handle);

            if !s.onboarded || !s.start_hidden {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                }
            }

            // Unload the model after the configured idle time.
            let h = handle.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(30));
                let st = h.state::<AppState>();
                let mins = st.settings.read().unload_after_min;
                if st.session.lock().phase == pipeline::Phase::Idle {
                    st.engine.unload_if_idle(Duration::from_secs(mins as u64 * 60));
                }
            });

            log("app", format!("Vāk {} started", handle.package_info().version));
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            ("panel", WindowEvent::Focused(false)) => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            update_settings,
            reset_general,
            list_models,
            download_model,
            pause_download,
            delete_model,
            list_history,
            delete_entry,
            star_entry,
            clear_audio,
            get_stats,
            copy_text,
            reveal,
            permissions,
            request_microphone,
            open_privacy,
            start_monitor,
            stop_monitor,
            test_chime,
            mic_test,
            diagnostics_report,
            run_postprocess,
            cleanup_last,
            start_dictation,
            stop_dictation,
            show_main,
            quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Vāk")
        .run(|app, event| {
            if let RunEvent::Reopen { .. } = event {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        });
}
