# Vāk

On-device speech-to-text for macOS, built with Tauri 2 (Rust) and React. Hold ⌥Space anywhere, talk, let go, and the text lands in the app in front. Implements the "Vāk Mac App" design: main window (sidebar + detail), dictation HUD, menu bar panel, onboarding, and the settings screens.

## Run

```bash
pnpm install
pnpm tauri dev
```

Build a `.app` / `.dmg`:

```bash
pnpm tauri build
```

Browser-only UI preview with mock data (no Rust needed): `pnpm dev`, then open `http://localhost:1420` (add `?w=panel`, `?w=hud` or `?onboarding=1`).

Requirements: Rust (rustup), Node 20+, pnpm, cmake and the Xcode command line tools (whisper.cpp is compiled with Metal).

## How it works

| Piece | Where |
| --- | --- |
| Global shortcut, push-to-talk / toggle, Esc to cancel | `src-tauri/src/pipeline.rs` |
| Mic capture (cpal), level meter, resample to 16 kHz, chime | `src-tauri/src/audio.rs` |
| whisper.cpp via `whisper-rs` (Metal), idle unload | `src-tauri/src/engine.rs` |
| Model catalog and resumable downloads from Hugging Face | `src-tauri/src/models.rs` |
| Fillers, word corrections, punctuation | `src-tauri/src/text.rs` |
| Paste ⌘V / type / terminal-safe ⌃⇧V / clipboard, restore, auto-submit | `src-tauri/src/output.rs` |
| Optional LLM pass (any OpenAI-compatible endpoint, e.g. Ollama) | `src-tauri/src/postprocess.rs` |
| History, audio retention | `src-tauri/src/history.rs` |
| Menu bar icon (the five bars, animated while you speak) | `src-tauri/src/tray.rs` |
| Windows: `main`, `hud` (click-through overlay), `panel` (tray popover) | `src/windows/` |

Data lives in `~/Library/Application Support/app.vak.desktop/` (settings, history, models, recordings), and logs go to `~/Library/Logs/Vāk/vak.log`.

## Permissions

- **Microphone**: prompted on first capture.
- **Accessibility**: needed to send ⌘V into other apps. Grant it in System Settings › Privacy & Security › Accessibility. During `pnpm tauri dev`, macOS attributes this to your terminal app.

## Tests

```bash
cd src-tauri && cargo test --lib
# real transcription (needs a ggml model and a 16-bit WAV):
VAK_TEST_MODEL=ggml-tiny.bin VAK_TEST_WAV=speech.wav cargo test --lib -- --ignored
```

## Not done yet

- Parakeet V3 is listed but has no engine in this build (Whisper models only).
- No live partial text in the HUD: Whisper runs once, after you release the key.
- Post-processing API keys are stored in plain text in `settings.json`.
