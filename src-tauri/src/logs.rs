use parking_lot::Mutex;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

const MAX_LINES: usize = 400;

#[derive(Serialize, Clone)]
pub struct LogLine {
    pub ts: String,
    pub tag: String,
    pub msg: String,
}

struct Logs {
    lines: Mutex<VecDeque<LogLine>>,
    file: Mutex<Option<File>>,
    app: AppHandle,
}

static LOGS: OnceLock<Logs> = OnceLock::new();

pub fn log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Logs/Vāk")
}

pub fn init(app: AppHandle) {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("vak.log"))
        .ok();
    let _ = LOGS.set(Logs {
        lines: Mutex::new(VecDeque::with_capacity(MAX_LINES)),
        file: Mutex::new(file),
        app,
    });
}

/// Append a line to the in-memory ring, the log file and the Diagnostics live view.
pub fn log(tag: &str, msg: impl Into<String>) {
    let line = LogLine {
        ts: chrono::Local::now().format("%H:%M:%S").to_string(),
        tag: tag.to_string(),
        msg: msg.into(),
    };
    eprintln!("[{}] {} {}", line.ts, line.tag, line.msg);
    let Some(logs) = LOGS.get() else { return };
    if let Some(f) = logs.file.lock().as_mut() {
        let _ = writeln!(
            f,
            "{} {} {} {}",
            chrono::Local::now().format("%Y-%m-%d"),
            line.ts,
            line.tag,
            line.msg
        );
    }
    {
        let mut lines = logs.lines.lock();
        if lines.len() >= MAX_LINES {
            lines.pop_front();
        }
        lines.push_back(line.clone());
    }
    let _ = logs.app.emit("log", line);
}

pub fn recent() -> Vec<LogLine> {
    LOGS.get()
        .map(|l| l.lines.lock().iter().cloned().collect())
        .unwrap_or_default()
}
