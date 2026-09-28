//! whisper.cpp (via whisper-rs, Metal on Apple silicon). Keeps one model resident
//! and unloads it after the configured idle time.

use parking_lot::Mutex;
use std::path::Path;
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::logs::log;

struct Loaded {
    id: String,
    ctx: WhisperContext,
}

pub struct Engine {
    loaded: Mutex<Option<Loaded>>,
    last_used: Mutex<Instant>,
}

impl Default for Engine {
    fn default() -> Self {
        Self { loaded: Mutex::new(None), last_used: Mutex::new(Instant::now()) }
    }
}

impl Engine {
    pub fn loaded_id(&self) -> Option<String> {
        self.loaded.lock().as_ref().map(|l| l.id.clone())
    }

    pub fn ensure_loaded(&self, id: &str, path: &Path) -> Result<(), String> {
        let mut slot = self.loaded.lock();
        if slot.as_ref().map(|l| l.id == id).unwrap_or(false) {
            return Ok(());
        }
        *slot = None;
        let t0 = Instant::now();
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);
        let ctx = WhisperContext::new_with_params(&path.to_string_lossy(), params)
            .map_err(|e| format!("could not load model: {e}"))?;
        log("asr", format!("loaded {id} in {} ms", t0.elapsed().as_millis()));
        *slot = Some(Loaded { id: id.to_string(), ctx });
        *self.last_used.lock() = Instant::now();
        Ok(())
    }

    pub fn transcribe(
        &self,
        id: &str,
        path: &Path,
        samples: &[f32],
        language: &str,
        prompt: &str,
    ) -> Result<String, String> {
        self.ensure_loaded(id, path)?;
        let slot = self.loaded.lock();
        let loaded = slot.as_ref().ok_or("model not loaded")?;
        let mut state = loaded.ctx.create_state().map_err(|e| e.to_string())?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
        params.set_n_threads(threads as i32);
        params.set_language(Some(if language.is_empty() { "auto" } else { language }));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_no_context(true);
        if !prompt.is_empty() {
            params.set_initial_prompt(prompt);
        }

        // Whisper wants at least one second of audio; pad short takes with silence.
        let mut padded;
        let data = if samples.len() < 16_000 + 1_600 {
            padded = samples.to_vec();
            padded.resize(16_000 + 1_600, 0.0);
            &padded[..]
        } else {
            samples
        };

        state.full(params, data).map_err(|e| e.to_string())?;
        let n = state.full_n_segments().map_err(|e| e.to_string())?;
        let mut text = String::new();
        for i in 0..n {
            if let Ok(seg) = state.full_get_segment_text_lossy(i) {
                text.push_str(&seg);
            }
        }
        *self.last_used.lock() = Instant::now();
        Ok(text.trim().to_string())
    }

    pub fn unload(&self) {
        if self.loaded.lock().take().is_some() {
            log("asr", "model unloaded after idle");
        }
    }

    pub fn unload_if_idle(&self, after: Duration) {
        if after.is_zero() {
            return;
        }
        if self.last_used.lock().elapsed() >= after {
            self.unload();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAK_TEST_MODEL=path/to/ggml-tiny.bin VAK_TEST_WAV=speech-16k.wav cargo test -- --ignored
    #[test]
    #[ignore]
    fn transcribes_wav() {
        let model = std::env::var("VAK_TEST_MODEL").expect("VAK_TEST_MODEL");
        let wav = std::env::var("VAK_TEST_WAV").expect("VAK_TEST_WAV");
        let mut r = hound::WavReader::open(wav).unwrap();
        let spec = r.spec();
        let raw: Vec<f32> = r.samples::<i16>().map(|s| s.unwrap() as f32 / i16::MAX as f32).collect();
        let samples = crate::audio::resample(&raw, spec.sample_rate, 16_000);
        let e = Engine::default();
        let t0 = Instant::now();
        let text = e.transcribe("tiny", Path::new(&model), &samples, "en", "Vāk, gRPC").unwrap();
        eprintln!("TRANSCRIPT ({} ms): {text}", t0.elapsed().as_millis());
        assert!(text.to_lowercase().contains("migration"));
    }
}
