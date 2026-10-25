# Testing Kōe

## Automated

```bash
cd src-tauri
cargo test --lib                     # 28 fast tests, no network or model needed
```

Slow tests (network plus a real Whisper model):

```bash
curl -L -o /tmp/ggml-tiny.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
say -o /tmp/e2e.aiff "Um, we should move the gRPC service to Kubernetes on Thursday."
afconvert -f WAVE -d LEI16@16000 -c 1 /tmp/e2e.aiff /tmp/e2e.wav
KOE_TEST_MODEL=/tmp/ggml-tiny.bin KOE_TEST_WAV=/tmp/e2e.wav cargo test --lib -- --include-ignored --test-threads=1
```

Real keystrokes into TextEdit, Terminal and iTerm (needs Accessibility for the terminal you run it from; hands off the keyboard for ~30 s):

```bash
cargo test --lib output_e2e -- --ignored --nocapture --test-threads=1
```

| Area | What's covered |
| --- | --- |
| Settings | Partial/nested updates, bad types rejected, old or partial files load |
| History | Pruning keeps starred, old audio deleted, delete/clear, word and latency stats |
| Audio | 48→16 kHz resample, level meter maths, WAV round trip |
| Text | Filler removal, multi-word corrections, punctuation |
| Output | Every method, Restore/Keep, auto submit, trailing space and paste delay produce the right steps; auto switch to terminal-safe |
| Clipboard | Restore puts back text, rich text, images, files and an empty clipboard (real pasteboard, ignored) |
| Post-process | Request format, API key header, `<think>` stripping, HTTP errors, dead endpoint |
| Models | Catalog sanity; every download URL resolves at the right size (network) |
| Pipeline | WAV → Whisper → cleanup → history entry on disk (model) |
| Shortcuts | Defaults and recorder output parse; ⌥Space ≠ ⌥⇧Space |

UI flows run against mock data in the browser: `pnpm dev`, then open http://localhost:1420.

## Manual (needs a person and macOS permissions)

Run `pnpm tauri dev`. Keep **Diagnostics → Live log** open in a second pass to see each step.

**Onboarding**
- [ ] Dark window with "Speak. Kōe types." appears
- [ ] Allow → macOS mic prompt → step turns green
- [ ] Open Settings → enable Accessibility for your terminal app → step turns green within ~2 s
- [ ] Download Whisper Large V3 Turbo → progress bar moves → "Model ready"

**Dictation (push to talk)**
- [ ] In Notes, hold ⌥Space → chime, red-dot pill with a moving waveform, timer counts
- [ ] Menu bar bars animate while you talk
- [ ] Release → "Transcribing" card → text appears at the cursor → "N words inserted" → pill fades
- [ ] ⌘Z in Notes undoes the paste
- [ ] Your clipboard is back to what it was before
- [ ] Hold ⌥Space, then press Esc → nothing is pasted
- [ ] Tap ⌥Space quickly → "Hold ⌥Space while you speak" hint

**Settings that change behaviour**
- [ ] General → Toggle mode: press once to start, once to stop
- [ ] General → change shortcut (e.g. ⌥D) → the new keys work, the old ones don't
- [ ] General → Test chime plays; volume slider changes loudness
- [ ] Output → Type it out: works in an app that blocks paste
- [ ] Output → Clipboard only: nothing typed, ⌘V pastes it
- [ ] Output → Auto submit Return: message sends in Slack/Messages
- [ ] Output → Auto submit ⌘ Return: sends in apps where Return is a newline (e.g. Slack with that preference)
- [ ] Output → Keep transcript: after dictating, ⌘V pastes the transcript again
- [ ] Output → Restore with an image or Finder file copied: still there after dictating
- [ ] Output → Trailing space off: no space after the text
- [ ] Output → Paste delay 0 ms: the new text lands, not your old clipboard
- [ ] Terminal/iTerm: dictation pastes (terminal-safe ⌃⇧V)
- [ ] Vocabulary → add a correction, then say the misheard word → corrected text is pasted
- [ ] Advanced → Overlay Top / Bottom / Hidden moves or hides the pill
- [ ] Advanced → Show menu bar icon off → icon disappears

**Menu bar panel**
- [ ] Click the bars icon → panel opens under it, input meter moves
- [ ] Toggle off → "Kōe is paused", ⌥Space does nothing; toggle on → works again
- [ ] Click a Recent item → copied; Settings… opens the window; Quit Kōe quits

**History and Models**
- [ ] New take appears at the top with the right app name and word count
- [ ] Play button plays the recording; waveform fills; speed button cycles
- [ ] Copy, star and delete work; search filters
- [ ] Models → Pause mid-download, then Resume continues (doesn't restart)
- [ ] Use a different installed model → status bar updates

**Post-processing (optional, needs Ollama: `ollama pull qwen3:8b`)**
- [ ] Post-process → Run on last transcript shows a cleaned version
- [ ] Hold ⌥⇧Space → dictation is cleaned by the LLM before pasting

**Diagnostics**
- [ ] Mic test records 5 s and plays back
- [ ] Copy report puts a report on the clipboard
- [ ] Reveal log folder opens ~/Library/Logs/Kōe

## Known results

- Warm transcription of ~4 s of audio: about 60 ms on Apple silicon (tiny, debug build).
