use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// Unix ms.
    pub created: i64,
    pub app: String,
    pub duration_ms: u64,
    pub words: u32,
    pub text: String,
    pub raw: String,
    pub post_processed: bool,
    pub starred: bool,
    /// File name inside the recordings folder.
    pub audio: Option<String>,
    pub peaks: Vec<f32>,
    pub model: String,
    pub latency_ms: u64,
}

pub struct History {
    path: PathBuf,
    pub recordings: PathBuf,
    pub entries: Vec<Entry>,
}

impl History {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("history.json");
        let recordings = data_dir.join("recordings");
        let _ = std::fs::create_dir_all(&recordings);
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, recordings, entries }
    }

    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string(&self.entries) {
            let _ = std::fs::write(&self.path, s);
        }
    }

    pub fn push(&mut self, e: Entry, keep: u32) {
        self.entries.insert(0, e);
        self.prune(keep, None);
        self.save();
    }

    /// Keep at most `keep` entries (starred ones survive) and delete audio older than `audio_days`.
    pub fn prune(&mut self, keep: u32, audio_days: Option<u32>) {
        let mut kept = 0u32;
        let recordings = self.recordings.clone();
        self.entries.retain(|e| {
            if e.starred {
                return true;
            }
            kept += 1;
            if kept <= keep.max(1) {
                true
            } else {
                if let Some(a) = &e.audio {
                    let _ = std::fs::remove_file(recordings.join(a));
                }
                false
            }
        });
        if let Some(days) = audio_days {
            let cutoff = chrono::Utc::now().timestamp_millis() - days as i64 * 86_400_000;
            for e in self.entries.iter_mut() {
                if e.created < cutoff {
                    if let Some(a) = e.audio.take() {
                        let _ = std::fs::remove_file(recordings.join(a));
                    }
                }
            }
        }
    }

    pub fn delete(&mut self, id: &str) {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            let e = self.entries.remove(pos);
            if let Some(a) = e.audio {
                let _ = std::fs::remove_file(self.recordings.join(a));
            }
            self.save();
        }
    }

    pub fn toggle_star(&mut self, id: &str) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.starred = !e.starred;
            self.save();
        }
    }

    pub fn clear_audio(&mut self) {
        for e in self.entries.iter_mut() {
            e.audio = None;
        }
        if let Ok(rd) = std::fs::read_dir(&self.recordings) {
            for f in rd.flatten() {
                let _ = std::fs::remove_file(f.path());
            }
        }
        self.save();
    }

    pub fn disk_bytes(&self) -> u64 {
        let audio: u64 = std::fs::read_dir(&self.recordings)
            .map(|rd| rd.flatten().filter_map(|f| f.metadata().ok()).map(|m| m.len()).sum())
            .unwrap_or(0);
        audio + std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    pub fn words_since(&self, since_ms: i64) -> u32 {
        self.entries.iter().filter(|e| e.created >= since_ms).map(|e| e.words).sum()
    }

    pub fn median_latency(&self) -> Option<u64> {
        let mut v: Vec<u64> = self.entries.iter().take(50).map(|e| e.latency_ms).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_unstable();
        Some(v[v.len() / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: i64, words: u32, audio: Option<&str>) -> Entry {
        Entry {
            id: id.to_string(),
            created: id,
            app: "Notes".into(),
            duration_ms: 1000,
            words,
            text: "hello".into(),
            raw: "hello".into(),
            post_processed: false,
            starred: false,
            audio: audio.map(String::from),
            peaks: vec![],
            model: "tiny".into(),
            latency_ms: id as u64 % 1000,
        }
    }

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("koe-hist-{}-{}", std::process::id(), chrono::Utc::now().timestamp_nanos_opt().unwrap()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn prune_keeps_starred_and_deletes_audio() {
        let d = tmp();
        let mut h = History::load(&d);
        let now = chrono::Utc::now().timestamp_millis();
        std::fs::write(h.recordings.join("old.wav"), b"x").unwrap();
        h.push(entry(now - 3, 1, Some("old.wav")), 10);
        h.toggle_star(&(now - 3).to_string());
        h.push(entry(now - 2, 1, None), 10);
        h.push(entry(now - 1, 1, None), 1);
        // keep=1: newest plus the starred one survive.
        assert_eq!(h.entries.len(), 2);
        assert!(h.entries.iter().any(|e| e.starred));
        // Reload from disk.
        let h2 = History::load(&d);
        assert_eq!(h2.entries.len(), 2);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn old_audio_is_removed_but_transcript_kept() {
        let d = tmp();
        let mut h = History::load(&d);
        std::fs::write(h.recordings.join("a.wav"), b"x").unwrap();
        let ten_days_ago = chrono::Utc::now().timestamp_millis() - 10 * 86_400_000;
        h.push(entry(ten_days_ago, 5, Some("a.wav")), 100);
        h.prune(100, Some(3));
        assert_eq!(h.entries.len(), 1);
        assert!(h.entries[0].audio.is_none());
        assert!(!h.recordings.join("a.wav").exists());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn delete_clear_and_stats() {
        let d = tmp();
        let mut h = History::load(&d);
        let now = chrono::Utc::now().timestamp_millis();
        std::fs::write(h.recordings.join("b.wav"), b"xyz").unwrap();
        h.push(entry(now - 100, 7, Some("b.wav")), 100);
        h.push(entry(now - 50, 3, None), 100);
        assert_eq!(h.words_since(now - 1000), 10);
        assert!(h.median_latency().is_some());
        assert!(h.disk_bytes() >= 3);
        h.clear_audio();
        assert!(!h.recordings.join("b.wav").exists());
        h.delete(&(now - 50).to_string());
        assert_eq!(h.entries.len(), 1);
        std::fs::remove_dir_all(d).ok();
    }
}
