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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// One-shot OpenAI-compatible server; returns the request body it received.
    fn serve(reply: &'static str, status: &'static str) -> (String, mpsc::Receiver<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", l.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut len = 0usize;
            let mut head = String::new();
            loop {
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                if line.to_lowercase().starts_with("content-length:") {
                    len = line[15..].trim().parse().unwrap();
                }
                head.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0; len];
            r.read_exact(&mut body).unwrap();
            tx.send(format!("{head}{}", String::from_utf8_lossy(&body))).unwrap();
            write!(s, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{reply}", reply.len()).unwrap();
        });
        (url, rx)
    }

    fn pp(url: &str) -> PostProcess {
        let mut p = PostProcess::default();
        p.provider = "custom".into();
        p.custom.base_url = url.into();
        p.custom.api_key = "secret".into();
        p
    }

    #[test]
    fn sends_prompt_and_strips_thinking() {
        let (url, rx) = serve(r#"{"choices":[{"message":{"content":"<think>hmm</think>We ship Thursday."}}]}"#, "200 OK");
        let out = run(&pp(&url), "Tidy: ${transcript}", "um we ship thursday").unwrap();
        assert_eq!(out, "We ship Thursday.");
        let req = rx.recv().unwrap();
        assert!(req.starts_with("POST /v1/chat/completions"));
        assert!(req.to_lowercase().contains("authorization: bearer secret"));
        assert!(req.contains("Tidy: um we ship thursday"));
    }

    #[test]
    fn appends_transcript_when_placeholder_missing() {
        let (url, rx) = serve(r#"{"choices":[{"message":{"content":"ok"}}]}"#, "200 OK");
        run(&pp(&url), "Summarise", "hello there").unwrap();
        assert!(rx.recv().unwrap().contains(r#"Summarise\nhello there"#));
    }

    #[test]
    fn surfaces_http_errors() {
        let (url, _rx) = serve(r#"{"error":{"message":"bad key"}}"#, "401 Unauthorized");
        let err = run(&pp(&url), "x ${transcript}", "y").unwrap_err();
        assert!(err.contains("401") && err.contains("bad key"), "{err}");
    }

    #[test]
    fn unreachable_endpoint_is_an_error_not_a_hang() {
        let err = run(&pp("http://127.0.0.1:9/v1"), "x", "y").unwrap_err();
        assert!(err.contains("127.0.0.1:9"), "{err}");
    }
}
