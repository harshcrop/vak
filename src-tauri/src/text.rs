//! Text cleanup between the recogniser and the paste.

use crate::settings::Settings;

const FILLERS: &[&str] = &["um", "umm", "uh", "uhh", "erm", "er", "hmm", "mm", "ah"];
const CHAT_APPS: &[&str] = &["Slack", "Messages", "Discord", "WhatsApp", "Telegram", "Signal"];

pub fn clean(raw: &str, s: &Settings, front_app: &str) -> String {
    let mut text = strip_annotations(raw);
    if s.remove_fillers {
        text = remove_fillers(&text);
    }
    text = apply_corrections(&text, s);
    if s.smart_punctuation {
        text = punctuate(&text);
    }
    if s.match_app_style && CHAT_APPS.iter().any(|a| front_app.eq_ignore_ascii_case(a)) {
        // Chat apps: no full stop at the end of a one-liner.
        if text.ends_with('.') && !text[..text.len() - 1].contains(['.', '!', '?']) {
            text.pop();
        }
    }
    text
}

/// Drops whisper's non-speech markers like "[BLANK_AUDIO]" or "(music)".
fn strip_annotations(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    collapse_spaces(&out)
}

fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_fillers(s: &str) -> String {
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| {
            let bare: String = w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            !FILLERS.contains(&bare.as_str())
        })
        .collect();
    let joined = words.join(" ");
    // Fix dangling ", ," left behind.
    joined.replace(" ,", ",").replace(",,", ",").trim_start_matches([',', ' ']).to_string()
}

/// Replace each "heard" alternative (split on " / ") with the written form.
/// Eagerness < 0.33 is case-sensitive; above 0.66 it also ignores spacing inside the phrase.
fn apply_corrections(s: &str, st: &Settings) -> String {
    let mut text = s.to_string();
    for c in &st.corrections {
        for alt in c.heard.split('/').map(str::trim).filter(|a| !a.is_empty()) {
            text = replace_phrase(&text, alt, &c.written, st.correction_eagerness);
        }
    }
    text
}

fn norm(w: &str, case_insensitive: bool) -> String {
    let t: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
    if case_insensitive { t.to_lowercase() } else { t }
}

fn replace_phrase(text: &str, phrase: &str, written: &str, eagerness: f32) -> String {
    let ci = eagerness >= 0.33;
    let target: Vec<String> = phrase.split_whitespace().map(|w| norm(w, ci)).collect();
    if target.is_empty() {
        return text.to_string();
    }
    let joined_target: String = target.concat();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let mut matched = 0;
        if i + target.len() <= words.len()
            && (0..target.len()).all(|k| norm(words[i + k], ci) == target[k])
        {
            matched = target.len();
        } else if eagerness > 0.66 {
            // Eager: the phrase may have been heard with different spacing.
            let mut acc = String::new();
            for k in i..words.len().min(i + target.len() + 2) {
                acc.push_str(&norm(words[k], ci));
                if acc == joined_target {
                    matched = k - i + 1;
                    break;
                }
                if !joined_target.starts_with(&acc) {
                    break;
                }
            }
        }
        if matched > 0 {
            let last = words[i + matched - 1];
            let trailing: String = last.chars().rev().take_while(|c| !c.is_alphanumeric()).collect::<Vec<_>>().into_iter().rev().collect();
            out.push(format!("{written}{trailing}"));
            i += matched;
        } else {
            out.push(words[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

fn punctuate(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    let mut out: String = first.to_uppercase().chain(chars).collect();
    if !out.ends_with(['.', '!', '?', ':', ';', '…', '"', '”']) && out.split_whitespace().count() > 2 {
        out.push('.');
    }
    out
}

pub fn word_count(s: &str) -> u32 {
    s.split_whitespace().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_fillers_and_corrections() {
        let s = Settings::default();
        let out = clean("um so we should ship the grpc thing with vaak uh [BLANK_AUDIO]", &s, "Notes");
        assert_eq!(out, "So we should ship the gRPC thing with Vāk.");
    }

    #[test]
    fn multiword_correction() {
        let s = Settings::default();
        assert_eq!(clean("use gee are pee see here", &s, ""), "Use gRPC here.");
    }
}
