// Browser-only stand-in for the Rust backend, so the UI can be previewed with `pnpm dev`
// outside Tauri. Never loaded inside the app.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

export function installMock() {
  const now = Date.now();
  const settings: any = {
    onboarded: new URLSearchParams(location.search).get("onboarding") !== "1",
    enabled: true,
    shortcut: "Alt+Space",
    ppShortcut: "Alt+Shift+Space",
    recordingMode: "ptt",
    cancelWithEsc: true,
    inputDevice: null,
    outputDevice: null,
    muteOtherAudio: false,
    audioFeedback: true,
    chimeVolume: 0.64,
    activeModel: "whisper-large-v3-turbo",
    language: "auto",
    pasteMethod: "paste",
    autoTerminal: true,
    clipboardHandling: "restore",
    autoSubmit: "off",
    trailingSpace: true,
    pasteDelayMs: 60,
    vocabulary: ["Kōe", "Parakeet", "Anika Rao", "Kubernetes", "SOC 2", "gRPC"],
    removeFillers: true,
    smartPunctuation: true,
    matchAppStyle: false,
    corrections: [
      { heard: "koey / koi", written: "Kōe" },
      { heard: "grpc / gee are pee see", written: "gRPC" },
      { heard: "anika rao", written: "Anika Rao" },
    ],
    correctionEagerness: 0.45,
    launchAtLogin: true,
    startHidden: false,
    showTrayIcon: true,
    overlayPosition: "bottom",
    unloadAfterMin: 5,
    experimental: true,
    keepTranscripts: 500,
    deleteAudioAfterDays: 3,
    postProcess: {
      enabled: true,
      provider: "local",
      local: { baseUrl: "http://localhost:11434/v1", model: "qwen3:8b", apiKey: "" },
      hosted: { baseUrl: "https://api.openai.com/v1", model: "gpt-4o-mini", apiKey: "" },
      custom: { baseUrl: "http://localhost:8080/v1", model: "default", apiKey: "" },
      recipes: [
        { name: "Tidy up", prompt: "Fix grammar and punctuation, drop filler words, keep my tone and every technical term exactly as spoken.\n${transcript}" },
        { name: "Bullets", prompt: "Turn this dictation into a short bulleted list.\n${transcript}" },
        { name: "Email", prompt: "Rewrite this dictation as a concise, friendly email body.\n${transcript}" },
        { name: "Translate", prompt: "Translate this into English.\n${transcript}" },
      ],
      activeRecipe: 0,
    },
  };
  const m = (id: string, name: string, description: string, sizeBytes: number, speed: number, accuracy: number, extra: any = {}) => ({
    id, name, description, sizeBytes, languages: 99, speed, accuracy, latencyMs: 200, recommended: false,
    installed: false, available: true, downloading: false, downloaded: 0, total: sizeBytes, updated: null, ...extra,
  });
  const models = [
    m("whisper-large-v3-turbo", "Whisper Large V3 Turbo", "Highest accuracy for accents, technical vocabulary and noisy rooms.", 1.62e9, 3, 5, { installed: true, recommended: true, downloaded: 1.62e9, updated: "12 Jul" }),
    m("whisper-medium", "Whisper Medium", "Balanced accuracy and speed. A good default on Apple silicon.", 1.53e9, 3, 4, { downloading: true, downloaded: 1.0e9 }),
    m("whisper-hindi2hinglish-apex", "Whisper Hindi2Hinglish Apex", "Hindi and Hinglish speech written out in Roman script (\"kaise ho bhai\"). Oriserve's fine-tune of Large V3 Turbo, 8-bit.", 8.74e8, 3, 4, { languages: 2 }),
  ];
  const peaks = Array.from({ length: 48 }, (_, i) => 0.2 + Math.abs(Math.sin(i * 1.7)) * 0.8);
  const history = [
    { id: "1", created: now - 3 * 60e3, app: "Linear", durationMs: 41000, words: 137, text: "Ship the migration behind a flag on Thursday, then let support know before we flip it for everyone. If the error rate holds under half a percent through the weekend we can drop the flag on Monday.", raw: "um so basically we should ship the migration behind a flag on thursday and uh tell support before we flip it", postProcessed: false, starred: true, audio: "1.wav", peaks, model: "Whisper Large V3 Turbo", latencyMs: 178 },
    { id: "2", created: now - 90 * 60e3, app: "Mail", durationMs: 18000, words: 54, text: "Reply to Anika — the invoice went out Monday and the PO number is on page two. Happy to walk through it on a call if that is easier.", raw: "", postProcessed: false, starred: false, audio: "2.wav", peaks, model: "Whisper Large V3 Turbo", latencyMs: 182 },
    { id: "3", created: now - 26 * 3600e3, app: "Obsidian", durationMs: 112000, words: 302, text: "Notes from the pricing call. Two tiers, no seat minimum, and the on-device story is the whole pitch for regulated teams.", raw: "", postProcessed: true, starred: false, audio: null, peaks, model: "Whisper Medium", latencyMs: 410 },
  ];
  const logs = [
    { ts: "12:38:04", tag: "hotkey", msg: "⌥Space down · capture started" },
    { ts: "12:38:04", tag: "audio", msg: "device=MacBook Pro Microphone 48 kHz mono" },
    { ts: "12:38:45", tag: "asr", msg: "41.2 s audio → 137 words in 178 ms" },
    { ts: "12:38:45", tag: "paste", msg: "frontmost=Linear · method=⌘V · delay=60 ms" },
    { ts: "12:38:45", tag: "ok", msg: "clipboard restored in 12 ms" },
  ];
  const merge = (a: any, b: any) => {
    for (const k of Object.keys(b)) a[k] = b[k] && typeof b[k] === "object" && !Array.isArray(b[k]) ? merge(a[k] ?? {}, b[k]) : b[k];
    return a;
  };

  // Stand-in for the desktop behind the translucent window.
  document.documentElement.style.background = "linear-gradient(150deg,#0a0908 0%,#241610 38%,#6e3410 72%,#e07b28 100%)";
  mockWindows(new URLSearchParams(location.search).get("w") ?? "main");
  mockIPC((cmd, args: any) => {
    switch (cmd) {
      case "bootstrap":
        return {
          settings, models, history, logs,
          permissions: { microphone: "granted", accessibility: settings.onboarded },
          inputDevices: ["MacBook Pro Microphone", "AirPods Pro"],
          outputDevices: ["MacBook Pro Speakers", "AirPods Pro"],
          defaultInput: "MacBook Pro Microphone",
          defaultOutput: "MacBook Pro Speakers",
          loadedModel: "whisper-large-v3-turbo",
          stats: { wordsWeek: 7412, medianLatency: 182, diskBytes: 184e6, transcripts: 1284 },
          build: { version: "0.1.0", arch: "aarch64", os: "macOS 26.1" },
          recordingsDir: "/tmp",
        };
      case "update_settings":
        return merge(settings, args.patch);
      case "list_models":
        return models;
      case "list_history":
        return history;
      case "get_stats":
        return { wordsWeek: 7412, medianLatency: 182, diskBytes: 184e6, transcripts: 1284 };
      case "permissions":
        return { microphone: "granted", accessibility: settings.onboarded };
      case "run_postprocess":
        return "We should ship the migration behind a flag on Thursday, and tell support before we flip it.";
      default:
        return null;
    }
  }, { shouldMockEvents: true });
  // Lets you drive the HUD from devtools: __koeEmit("hud", { state: "listening", position: "bottom" })
  (window as any).__koeEmit = emit;
  (window as any).__TAURI_INTERNALS__.convertFileSrc = (p: string) => p;
}
