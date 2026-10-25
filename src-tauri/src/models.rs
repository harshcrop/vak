//! Model catalog and downloads (resumable, with progress events).

use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::logs::log;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub size_bytes: u64,
    pub languages: u32,
    pub speed: u8,
    pub accuracy: u8,
    pub latency_ms: u32,
    pub recommended: bool,
    /// None when the engine isn't available in this build yet.
    pub file: Option<&'static str>,
    pub url: Option<&'static str>,
    /// Decode language the model was fine-tuned for; overrides the user's setting.
    #[serde(skip)]
    pub language: Option<&'static str>,
}


pub fn catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "whisper-large-v3-turbo",
            name: "Whisper Large V3 Turbo",
            description: "Highest accuracy for accents, technical vocabulary and noisy rooms.",
            size_bytes: 1_624_555_275,
            languages: 99,
            speed: 3,
            accuracy: 5,
            latency_ms: 420,
            recommended: true,
            file: Some("ggml-large-v3-turbo.bin"),
            url: Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"),
            language: None,
        },
        ModelInfo {
            id: "whisper-medium",
            name: "Whisper Medium",
            description: "Balanced accuracy and speed. A good default on Apple silicon.",
            size_bytes: 1_533_763_059,
            languages: 99,
            speed: 3,
            accuracy: 4,
            latency_ms: 360,
            recommended: false,
            file: Some("ggml-medium.bin"),
            url: Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"),
            language: None,
        },
        ModelInfo {
            id: "whisper-hindi2hinglish-apex",
            name: "Whisper Hindi2Hinglish Apex",
            description: "Hindi and Hinglish speech written out in Roman script (\"kaise ho bhai\"). Oriserve's fine-tune of Large V3 Turbo, 8-bit.",
            size_bytes: 874_188_075,
            languages: 2,
            speed: 3,
            accuracy: 4,
            latency_ms: 400,
            recommended: false,
            file: Some("ggml-apex-hinglish-q8_0.bin"),
            url: Some("https://huggingface.co/Marquestra/Whisper-Hindi2Hinglish-Apex-GGML/resolve/main/ggml-apex-hinglish-q8_0.bin"),
            language: Some("en"),
        },
    ]
}

pub fn find(id: &str) -> Option<ModelInfo> {
    catalog().into_iter().find(|m| m.id == id)
}

pub fn model_path(dir: &Path, m: &ModelInfo) -> Option<PathBuf> {
    m.file.map(|f| dir.join(f))
}

pub fn is_installed(dir: &Path, m: &ModelInfo) -> bool {
    model_path(dir, m).map(|p| p.exists()).unwrap_or(false)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    #[serde(flatten)]
    pub info: ModelInfo,
    pub installed: bool,
    pub available: bool,
    pub downloading: bool,
    pub downloaded: u64,
    pub total: u64,
    pub updated: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub downloaded: u64,
    pub total: u64,
    /// "downloading" | "paused" | "done" | "error"
    pub state: String,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Downloads {
    cancel: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Downloads {
    pub fn is_active(&self, id: &str) -> bool {
        self.cancel.lock().contains_key(id)
    }
    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.cancel.lock().get(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

pub fn statuses(dir: &Path, downloads: &Downloads) -> Vec<ModelStatus> {
    catalog()
        .into_iter()
        .map(|m| {
            let path = model_path(dir, &m);
            let meta = path.as_ref().and_then(|p| std::fs::metadata(p).ok());
            let partial = path
                .as_ref()
                .and_then(|p| std::fs::metadata(p.with_extension("part")).ok())
                .map(|m| m.len())
                .unwrap_or(0);
            let updated = meta.as_ref().and_then(|m| m.modified().ok()).map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%-d %b")
                    .to_string()
            });
            ModelStatus {
                installed: meta.is_some(),
                available: m.file.is_some(),
                downloading: downloads.is_active(m.id),
                downloaded: if meta.is_some() { m.size_bytes } else { partial },
                total: m.size_bytes,
                updated,
                info: m,
            }
        })
        .collect()
}

pub async fn download(app: AppHandle, dir: PathBuf, id: String, downloads: Arc<Downloads>) -> Result<(), String> {
    let m = find(&id).ok_or("unknown model")?;
    let (Some(file), Some(url)) = (m.file, m.url) else {
        return Err("this engine isn't available yet".into());
    };
    let flag = Arc::new(AtomicBool::new(false));
    {
        let mut map = downloads.cancel.lock();
        if map.contains_key(&id) {
            return Ok(());
        }
        map.insert(id.clone(), flag.clone());
    }
    let result = run_download(&app, &dir, &id, file, url, m.size_bytes, &flag).await;
    downloads.cancel.lock().remove(&id);
    let (state, error) = match &result {
        Ok(true) => ("done", None),
        Ok(false) => ("paused", None),
        Err(e) => ("error", Some(e.clone())),
    };
    let part = dir.join(file).with_extension("part");
    let downloaded = std::fs::metadata(dir.join(file))
        .or_else(|_| std::fs::metadata(&part))
        .map(|m| m.len())
        .unwrap_or(0);
    let _ = app.emit(
        "model-progress",
        Progress { id: id.clone(), downloaded, total: m.size_bytes, state: state.into(), error },
    );
    match result {
        Ok(true) => log("model", format!("{} downloaded", m.name)),
        Ok(false) => log("model", format!("{} paused", m.name)),
        Err(e) => log("model", format!("{} failed: {e}", m.name)),
    }
    Ok(())
}

/// Returns Ok(true) when complete, Ok(false) when paused by the user.
async fn run_download(
    app: &AppHandle,
    dir: &Path,
    id: &str,
    file: &str,
    url: &str,
    expected: u64,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let final_path = dir.join(file);
    let part = final_path.with_extension("part");
    let mut have = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let client = reqwest::Client::new();
    let mut req = client.get(url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    if status != reqwest::StatusCode::PARTIAL_CONTENT {
        have = 0; // server ignored the range; start over
    }
    let total = resp.content_length().map(|l| l + have).unwrap_or(expected);
    let mut out = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(have > 0)
        .truncate(have == 0)
        .open(&part)
        .await
        .map_err(|e| e.to_string())?;

    log("model", format!("downloading {file} from {}", if have > 0 { "partial" } else { "start" }));
    let mut stream = resp.bytes_stream();
    let mut last = Instant::now();
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            out.flush().await.ok();
            return Ok(false);
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        out.write_all(&chunk).await.map_err(|e| e.to_string())?;
        have += chunk.len() as u64;
        if last.elapsed().as_millis() > 200 {
            last = Instant::now();
            let _ = app.emit(
                "model-progress",
                Progress { id: id.into(), downloaded: have, total, state: "downloading".into(), error: None },
            );
        }
    }
    out.flush().await.map_err(|e| e.to_string())?;
    drop(out);
    std::fs::rename(&part, &final_path).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        let c = catalog();
        let mut ids: Vec<_> = c.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len(), "duplicate ids");
        assert!(find("whisper-large-v3-turbo").unwrap().recommended);
        assert!(find("whisper-small").is_none() && find("whisper-base").is_none() && find("whisper-tiny").is_none());
        assert!(find("parakeet-v3").is_none());
        let dir = std::env::temp_dir().join("koe-models-empty");
        let st = statuses(&dir, &Downloads::default());
        assert!(st.iter().all(|m| !m.installed && !m.downloading));
    }

    /// Network: every download URL resolves and matches the size we show.
    #[test]
    #[ignore]
    fn download_urls_resolve() {
        let client = reqwest::blocking::Client::new();
        for m in catalog().into_iter().filter(|m| m.url.is_some()) {
            let url = m.url.unwrap();
            let r = client.head(url).send().unwrap();
            assert!(r.status().is_success(), "{} → {}", url, r.status());
            let len = r
                .headers()
                .get("x-linked-size")
                .or_else(|| r.headers().get(reqwest::header::CONTENT_LENGTH))
                .and_then(|v| v.to_str().ok()?.parse::<u64>().ok())
                .unwrap_or(0);
            let diff = (len as i64 - m.size_bytes as i64).abs() as f64 / m.size_bytes as f64;
            eprintln!("{:<24} {:>12} bytes (catalog {:>12})", m.id, len, m.size_bytes);
            assert!(diff < 0.02, "{} size off by {:.1}%", m.id, diff * 100.0);
        }
    }
}
