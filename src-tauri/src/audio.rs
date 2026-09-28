//! Microphone capture (cpal) and the start/stop chime.
//!
//! cpal streams are not `Send` on macOS, so each capture owns its stream on a
//! dedicated thread and is controlled over a channel.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub const TARGET_RATE: u32 = 16_000;

pub struct Captured {
    /// Mono, 16 kHz, f32 in [-1, 1].
    pub samples: Vec<f32>,
    pub device: String,
    pub source_rate: u32,
}

impl Captured {
    pub fn duration_ms(&self) -> u64 {
        (self.samples.len() as u64 * 1000) / TARGET_RATE as u64
    }
}

struct Active {
    stop_tx: mpsc::Sender<()>,
    join: JoinHandle<Result<Captured, String>>,
}

#[derive(Default)]
pub struct Recorder {
    active: Mutex<Option<Active>>,
}

pub fn input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub fn output_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub fn default_input_name() -> Option<String> {
    cpal::default_host().default_input_device().and_then(|d| d.name().ok())
}

pub fn default_output_name() -> Option<String> {
    cpal::default_host().default_output_device().and_then(|d| d.name().ok())
}

fn find_input(name: &Option<String>) -> Option<cpal::Device> {
    let host = cpal::default_host();
    if let Some(name) = name {
        if let Ok(mut devs) = host.input_devices() {
            if let Some(d) = devs.find(|d| d.name().ok().as_deref() == Some(name)) {
                return Some(d);
            }
        }
    }
    host.default_input_device()
}

fn find_output(name: &Option<String>) -> Option<cpal::Device> {
    let host = cpal::default_host();
    if let Some(name) = name {
        if let Ok(mut devs) = host.output_devices() {
            if let Some(d) = devs.find(|d| d.name().ok().as_deref() == Some(name)) {
                return Some(d);
            }
        }
    }
    host.default_output_device()
}

/// Level in dBFS from an RMS value.
pub fn to_db(rms: f32) -> f32 {
    if rms <= 1e-6 {
        -80.0
    } else {
        (20.0 * rms.log10()).max(-80.0)
    }
}

impl Recorder {
    pub fn is_active(&self) -> bool {
        self.active.lock().is_some()
    }

    /// Start capturing. `on_level` is called ~20×/s with the current level in dBFS.
    /// When `keep` is false the audio is only metered, not stored.
    pub fn start(
        &self,
        device: Option<String>,
        keep: bool,
        on_level: impl Fn(f32) + Send + 'static,
    ) -> Result<String, String> {
        let mut slot = self.active.lock();
        if slot.is_some() {
            return Err("already recording".into());
        }
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<String, String>>();

        let join = std::thread::spawn(move || -> Result<Captured, String> {
            let dev = match find_input(&device) {
                Some(d) => d,
                None => {
                    let _ = ready_tx.send(Err("no input device".into()));
                    return Err("no input device".into());
                }
            };
            let name = dev.name().unwrap_or_else(|_| "Microphone".into());
            let config = match dev.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return Err(e.to_string());
                }
            };
            let rate = config.sample_rate().0;
            let channels = config.channels() as usize;
            let buf = Arc::new(Mutex::new(Vec::<f32>::with_capacity(rate as usize * 30)));
            let level = Arc::new(AtomicU32::new(0));

            let err_fn = |e| eprintln!("audio stream error: {e}");
            let stream_cfg: cpal::StreamConfig = config.clone().into();

            macro_rules! build {
                ($t:ty, $conv:expr) => {{
                    let buf = buf.clone();
                    let level = level.clone();
                    dev.build_input_stream(
                        &stream_cfg,
                        move |data: &[$t], _| {
                            let mut sum = 0.0f32;
                            let mut frames = 0usize;
                            let mut out = if keep { Some(buf.lock()) } else { None };
                            for frame in data.chunks(channels) {
                                let mut s = 0.0f32;
                                for v in frame {
                                    s += $conv(*v);
                                }
                                let s = s / channels as f32;
                                sum += s * s;
                                frames += 1;
                                if let Some(o) = out.as_mut() {
                                    o.push(s);
                                }
                            }
                            if frames > 0 {
                                let rms = (sum / frames as f32).sqrt();
                                level.store(rms.to_bits(), Ordering::Relaxed);
                            }
                        },
                        err_fn,
                        None,
                    )
                }};
            }

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => build!(f32, |v: f32| v),
                cpal::SampleFormat::I16 => build!(i16, |v: i16| v as f32 / i16::MAX as f32),
                cpal::SampleFormat::I32 => build!(i32, |v: i32| v as f32 / i32::MAX as f32),
                cpal::SampleFormat::U16 => {
                    build!(u16, |v: u16| (v as f32 - 32768.0) / 32768.0)
                }
                other => {
                    let msg = format!("unsupported sample format {other:?}");
                    let _ = ready_tx.send(Err(msg.clone()));
                    return Err(msg);
                }
            };
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return Err(e.to_string());
                }
            };
            if let Err(e) = stream.play() {
                let _ = ready_tx.send(Err(e.to_string()));
                return Err(e.to_string());
            }
            let _ = ready_tx.send(Ok(format!("{name} {} kHz {}", rate / 1000, if channels == 1 { "mono".to_string() } else { format!("{channels}ch") })));

            loop {
                match stop_rx.recv_timeout(Duration::from_millis(50)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        on_level(to_db(f32::from_bits(level.load(Ordering::Relaxed))));
                    }
                    _ => break,
                }
            }
            drop(stream);
            let raw = std::mem::take(&mut *buf.lock());
            Ok(Captured {
                samples: resample(&raw, rate, TARGET_RATE),
                device: name,
                source_rate: rate,
            })
        });

        let desc = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "microphone did not start".to_string())
            .and_then(|r| r);
        match desc {
            Ok(d) => {
                *slot = Some(Active { stop_tx, join });
                Ok(d)
            }
            Err(e) => {
                let _ = join.join();
                Err(e)
            }
        }
    }

    pub fn stop(&self) -> Result<Captured, String> {
        let active = self.active.lock().take().ok_or("not recording")?;
        let _ = active.stop_tx.send(());
        active.join.join().map_err(|_| "capture thread panicked".to_string())?
    }

    pub fn cancel(&self) {
        let _ = self.stop();
    }
}

/// Linear resampler; plenty for speech going into Whisper.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = (pos - idx as f64) as f32;
        let a = input[idx];
        let b = *input.get(idx + 1).unwrap_or(&a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Peak envelope for the history waveform.
pub fn peaks(samples: &[f32], buckets: usize) -> Vec<f32> {
    if samples.is_empty() {
        return vec![0.0; buckets];
    }
    let size = (samples.len() / buckets).max(1);
    let mut out: Vec<f32> = samples
        .chunks(size)
        .take(buckets)
        .map(|c| c.iter().fold(0.0f32, |m, v| m.max(v.abs())))
        .collect();
    let max = out.iter().cloned().fold(0.0f32, f32::max).max(1e-4);
    for v in out.iter_mut() {
        *v = (*v / max).clamp(0.08, 1.0);
    }
    out
}

pub fn write_wav(path: &Path, samples: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

/// A short two-note chime on the chosen output device. Rising for start, falling for stop.
pub fn chime(device: Option<String>, volume: f32, rising: bool) {
    std::thread::spawn(move || {
        let Some(dev) = find_output(&device) else { return };
        let Ok(config) = dev.default_output_config() else { return };
        if config.sample_format() != cpal::SampleFormat::F32 {
            return;
        }
        let rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        let (f1, f2) = if rising { (660.0, 990.0) } else { (880.0, 587.0) };
        let note = (rate * 0.07) as usize;
        let total = note * 2;
        let mut t = 0usize;
        let vol = volume.clamp(0.0, 1.0) * 0.25;
        let stream = dev.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(channels) {
                    let v = if t < total {
                        let f = if t < note { f1 } else { f2 };
                        let local = (t % note) as f32 / note as f32;
                        let env = (1.0 - local).powf(1.6) * (local * 40.0).min(1.0);
                        (t as f32 / rate * f * std::f32::consts::TAU).sin() * env * vol
                    } else {
                        0.0
                    };
                    t += 1;
                    for s in frame.iter_mut() {
                        *s = v;
                    }
                }
            },
            |e| eprintln!("chime error: {e}"),
            None,
        );
        if let Ok(stream) = stream {
            if stream.play().is_ok() {
                std::thread::sleep(Duration::from_millis(260));
            }
        }
    });
}
