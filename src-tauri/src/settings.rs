use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    pub heard: String,
    pub written: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub name: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct PostProcess {
    pub enabled: bool,
    /// "local" | "hosted" | "custom"
    pub provider: String,
    pub local: Provider,
    pub hosted: Provider,
    pub custom: Provider,
    pub recipes: Vec<Recipe>,
    pub active_recipe: usize,
}

impl Default for PostProcess {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "local".into(),
            local: Provider {
                base_url: "http://localhost:11434/v1".into(),
                model: "qwen3:8b".into(),
                api_key: String::new(),
            },
            hosted: Provider {
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-mini".into(),
                api_key: String::new(),
            },
            custom: Provider {
                base_url: "http://localhost:8080/v1".into(),
                model: "default".into(),
                api_key: String::new(),
            },
            recipes: vec![
                Recipe {
                    name: "Tidy up".into(),
                    prompt: "Fix grammar and punctuation, drop filler words, keep my tone and every technical term exactly as spoken. Reply with the cleaned text only.\n${transcript}".into(),
                },
                Recipe {
                    name: "Bullets".into(),
                    prompt: "Turn this dictation into a short bulleted list. Keep every fact, drop filler. Reply with the list only.\n${transcript}".into(),
                },
                Recipe {
                    name: "Email".into(),
                    prompt: "Rewrite this dictation as a concise, friendly email body. No subject line. Reply with the email only.\n${transcript}".into(),
                },
                Recipe {
                    name: "Translate".into(),
                    prompt: "Translate this into English, keeping names and technical terms as they are. Reply with the translation only.\n${transcript}".into(),
                },
            ],
            active_recipe: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub onboarded: bool,
    /// Master switch for the shortcuts (menu bar toggle).
    pub enabled: bool,
    // General
    pub shortcut: String,
    pub pp_shortcut: String,
    /// "ptt" | "toggle"
    pub recording_mode: String,
    pub cancel_with_esc: bool,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub mute_other_audio: bool,
    pub audio_feedback: bool,
    pub chime_volume: f32,
    // Models
    pub active_model: String,
    pub language: String,
    // Output
    /// "paste" | "type" | "terminal" | "clipboard"
    pub paste_method: String,
    pub auto_terminal: bool,
    /// "restore" | "keep"
    pub clipboard_handling: String,
    /// "off" | "return" | "cmdReturn"
    pub auto_submit: String,
    pub trailing_space: bool,
    pub paste_delay_ms: u32,
    // Vocabulary & cleanup
    pub vocabulary: Vec<String>,
    pub remove_fillers: bool,
    pub smart_punctuation: bool,
    pub match_app_style: bool,
    pub corrections: Vec<Correction>,
    pub correction_eagerness: f32,
    // App
    pub launch_at_login: bool,
    pub start_hidden: bool,
    pub show_tray_icon: bool,
    /// "top" | "bottom" | "hidden"
    pub overlay_position: String,
    /// Minutes of idle before the model is unloaded. 0 = never.
    pub unload_after_min: u32,
    pub experimental: bool,
    // Retention
    pub keep_transcripts: u32,
    /// 0 = don't keep audio at all.
    pub delete_audio_after_days: u32,
    pub post_process: PostProcess,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarded: false,
            enabled: true,
            shortcut: "Alt+Space".into(),
            pp_shortcut: "Alt+Shift+Space".into(),
            recording_mode: "ptt".into(),
            cancel_with_esc: true,
            input_device: None,
            output_device: None,
            mute_other_audio: false,
            audio_feedback: true,
            chime_volume: 0.64,
            active_model: "whisper-small".into(),
            language: "auto".into(),
            paste_method: "paste".into(),
            auto_terminal: true,
            clipboard_handling: "restore".into(),
            auto_submit: "off".into(),
            trailing_space: true,
            paste_delay_ms: 60,
            vocabulary: vec!["Vāk".into(), "Kubernetes".into(), "gRPC".into()],
            remove_fillers: true,
            smart_punctuation: true,
            match_app_style: false,
            corrections: vec![
                Correction { heard: "vaak / walk".into(), written: "Vāk".into() },
                Correction { heard: "grpc / gee are pee see".into(), written: "gRPC".into() },
            ],
            correction_eagerness: 0.45,
            launch_at_login: false,
            start_hidden: false,
            show_tray_icon: true,
            overlay_position: "bottom".into(),
            unload_after_min: 5,
            experimental: true,
            keep_transcripts: 500,
            delete_audio_after_days: 3,
            post_process: PostProcess::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, s);
        }
    }

    /// Merge a partial JSON object into the settings (nested objects merge recursively).
    pub fn merged(&self, patch: serde_json::Value) -> Result<Self, String> {
        let mut base = serde_json::to_value(self).map_err(|e| e.to_string())?;
        merge(&mut base, patch);
        serde_json::from_value(base).map_err(|e| e.to_string())
    }
}

fn merge(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                match b.get_mut(&k) {
                    Some(existing) if existing.is_object() && v.is_object() => merge(existing, v),
                    _ => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (b, p) => *b = p,
    }
}
