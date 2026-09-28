//! Optional LLM pass through any OpenAI-compatible chat endpoint
//! (Ollama / LM Studio / mlx_lm.server locally, or a hosted API).

use crate::settings::{PostProcess, Provider};
use serde_json::json;
use std::time::Duration;

pub fn provider(pp: &PostProcess) -> &Provider {
    match pp.provider.as_str() {
        "hosted" => &pp.hosted,
        "custom" => &pp.custom,
        _ => &pp.local,
    }
}

pub fn run(pp: &PostProcess, prompt_template: &str, transcript: &str) -> Result<String, String> {
    let p = provider(pp);
    let prompt = if prompt_template.contains("${transcript}") {
        prompt_template.replace("${transcript}", transcript)
    } else {
        format!("{prompt_template}\n{transcript}")
    };
    let url = format!("{}/chat/completions", p.base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.post(&url).json(&json!({
        "model": p.model,
        "temperature": 0.2,
        "stream": false,
        "messages": [{ "role": "user", "content": prompt }],
    }));
    if !p.api_key.is_empty() {
        req = req.bearer_auth(&p.api_key);
    }
    let resp = req.send().map_err(|e| format!("{url}: {e}"))?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", body["error"]["message"].as_str().unwrap_or("request failed")));
    }
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("empty response")?;
    Ok(strip_thinking(content).trim().to_string())
}

/// Reasoning models (Qwen3 etc.) may prefix a <think>…</think> block.
fn strip_thinking(s: &str) -> String {
    match (s.find("<think>"), s.find("</think>")) {
        (Some(a), Some(b)) if b > a => format!("{}{}", &s[..a], &s[b + "</think>".len()..]),
        _ => s.to_string(),
    }
}
