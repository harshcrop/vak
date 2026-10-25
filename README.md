# Kōe

On-device speech-to-text for macOS, built with Tauri 2 (Rust) and React. Hold ⌥Space anywhere, talk, let go, and the text lands in the app in front. Implements the "Kōe Mac App" design: main window (sidebar + detail), dictation HUD, menu bar panel, onboarding, and the settings screens.

## Install (macOS 12+, Intel and Apple Silicon)

1. Download `Koe-0.1.0-universal.dmg` from the Releases page and drag Kōe into Applications.
2. The app isn't notarized yet, so macOS blocks the first launch. Right-click Kōe, choose **Open**, then **Open** again. Or run `xattr -cr /Applications/Kōe.app` once.
3. Grant Microphone and Accessibility access when asked, then pick a speech model on first run (it downloads once, then works offline).

## Run

```bash
pnpm install
pnpm tauri dev
```

Build a `.app` / `.dmg`:

```bash
pnpm tauri build
```

Release build for every Mac (Intel + Apple Silicon, needs `rustup target add x86_64-apple-darwin` once):

```bash
pnpm build:mac
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
| Menu bar icon (the ō mark; the macron tracks your voice) | `src-tauri/src/tray.rs` |
| Windows: `main`, `hud` (click-through overlay), `panel` (tray popover) | `src/windows/` |

Data lives in `~/Library/Application Support/app.koe.desktop/` (settings, history, models, recordings), and logs go to `~/Library/Logs/Kōe/koe.log`.

## Permissions

- **Microphone**: prompted on first capture.
- **Accessibility**: needed to send ⌘V into other apps. Grant it in System Settings › Privacy & Security › Accessibility. During `pnpm tauri dev`, macOS attributes this to your terminal app.

## Tests

```bash
cd src-tauri && cargo test --lib
# real transcription (needs a ggml model and a 16-bit WAV):
KOE_TEST_MODEL=ggml-tiny.bin KOE_TEST_WAV=speech.wav cargo test --lib -- --ignored
```

## Not done yet

- No live partial text in the HUD: Whisper runs once, after you release the key.
- Post-processing API keys are stored in plain text in `settings.json`.
