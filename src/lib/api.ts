import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createContext, createElement, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export type Provider = { baseUrl: string; model: string; apiKey: string };
export type Recipe = { name: string; prompt: string };
export type Correction = { heard: string; written: string };

export type Settings = {
  onboarded: boolean;
  enabled: boolean;
  shortcut: string;
  ppShortcut: string;
  recordingMode: "ptt" | "toggle";
  cancelWithEsc: boolean;
  inputDevice: string | null;
  outputDevice: string | null;
  muteOtherAudio: boolean;
  audioFeedback: boolean;
  chimeVolume: number;
  activeModel: string;
  language: string;
  pasteMethod: "paste" | "type" | "terminal" | "clipboard";
  autoTerminal: boolean;
  clipboardHandling: "restore" | "keep";
  autoSubmit: "off" | "return" | "cmdReturn";
  trailingSpace: boolean;
  pasteDelayMs: number;
  vocabulary: string[];
  removeFillers: boolean;
  smartPunctuation: boolean;
  matchAppStyle: boolean;
  corrections: Correction[];
  correctionEagerness: number;
  launchAtLogin: boolean;
  startHidden: boolean;
  showTrayIcon: boolean;
  overlayPosition: "top" | "bottom" | "hidden";
  unloadAfterMin: number;
  experimental: boolean;
  keepTranscripts: number;
  deleteAudioAfterDays: number;
  postProcess: {
    enabled: boolean;
    provider: "local" | "hosted" | "custom";
    local: Provider;
    hosted: Provider;
    custom: Provider;
    recipes: Recipe[];
    activeRecipe: number;
  };
};

export type ModelStatus = {
  id: string;
  name: string;
  description: string;
  sizeBytes: number;
  languages: number;
  speed: number;
  accuracy: number;
  latencyMs: number;
  recommended: boolean;
  installed: boolean;
  available: boolean;
  downloading: boolean;
  downloaded: number;
  total: number;
  updated: string | null;
};

export type Progress = { id: string; downloaded: number; total: number; state: "downloading" | "paused" | "done" | "error"; error: string | null };

export type Entry = {
  id: string;
  created: number;
  app: string;
  durationMs: number;
  words: number;
  text: string;
  raw: string;
  postProcessed: boolean;
  starred: boolean;
  audio: string | null;
  peaks: number[];
  model: string;
  latencyMs: number;
};

export type Permissions = { microphone: "granted" | "denied" | "undetermined"; accessibility: boolean };
export type Stats = { wordsWeek: number; medianLatency: number | null; diskBytes: number; transcripts: number };
export type LogLine = { ts: string; tag: string; msg: string };

export type Bootstrap = {
  settings: Settings;
  models: ModelStatus[];
  history: Entry[];
  permissions: Permissions;
  inputDevices: string[];
  outputDevices: string[];
  defaultInput: string | null;
  defaultOutput: string | null;
  loadedModel: string | null;
  stats: Stats;
  build: { version: string; arch: string; os: string };
  logs: LogLine[];
  recordingsDir: string;
};

export type HudState =
  | { state: "listening"; position: string; postProcess: boolean }
  | { state: "transcribing"; position: string; model: string; app: string; durationMs: number; postProcess: boolean }
  | { state: "done"; position: string; words: number; text: string; method: string; realtime: number }
  | { state: "error"; position: string; message: string }
  | { state: "hint"; position: string; message: string };

type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? (T[K] extends unknown[] ? T[K] : DeepPartial<T[K]>) : T[K] };

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  updateSettings: (patch: DeepPartial<Settings>) => invoke<Settings>("update_settings", { patch }),
  resetGeneral: () => invoke<Settings>("reset_general"),
  listModels: () => invoke<ModelStatus[]>("list_models"),
  downloadModel: (id: string) => invoke<void>("download_model", { id }),
  pauseDownload: (id: string) => invoke<void>("pause_download", { id }),
  deleteModel: (id: string) => invoke<void>("delete_model", { id }),
  listHistory: () => invoke<Entry[]>("list_history"),
  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),
  starEntry: (id: string) => invoke<void>("star_entry", { id }),
  clearAudio: () => invoke<void>("clear_audio"),
  stats: () => invoke<Stats>("get_stats"),
  copy: (text: string) => invoke<void>("copy_text", { text }),
  reveal: (what: "logs" | "models" | "recordings") => invoke<void>("reveal", { what }),
  permissions: () => invoke<Permissions>("permissions"),
  requestMicrophone: () => invoke<Permissions>("request_microphone"),
  openPrivacy: (kind: "microphone" | "accessibility") => invoke<void>("open_privacy", { kind }),
  startMonitor: () => invoke<string>("start_monitor"),
  stopMonitor: () => invoke<void>("stop_monitor"),
  testChime: () => invoke<void>("test_chime"),
  micTest: () => invoke<string>("mic_test"),
  copyReport: () => invoke<void>("diagnostics_report"),
  runPostprocess: (text: string, prompt: string) => invoke<string>("run_postprocess", { text, prompt }),
  cleanupLast: () => invoke<string>("cleanup_last"),
  startDictation: (postProcess: boolean) => invoke<void>("start_dictation", { postProcess }),
  stopDictation: () => invoke<void>("stop_dictation"),
  showMain: (view?: string) => invoke<void>("show_main", { view }),
  quit: () => invoke<void>("quit"),
};

// ---------- Shared app store (one per window) ----------

type Store = Bootstrap & {
  progress: Record<string, Progress>;
  update: (patch: DeepPartial<Settings>) => Promise<void>;
  refresh: () => Promise<void>;
  refreshModels: () => Promise<void>;
  refreshHistory: () => Promise<void>;
  setPermissions: (p: Permissions) => void;
};

const Ctx = createContext<Store | null>(null);

export function KoeProvider({ children, fallback }: { children: ReactNode; fallback?: ReactNode }) {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [progress, setProgress] = useState<Record<string, Progress>>({});

  const refresh = useCallback(async () => setBoot(await api.bootstrap()), []);
  const refreshModels = useCallback(async () => {
    const models = await api.listModels();
    setBoot((b) => (b ? { ...b, models } : b));
  }, []);
  const refreshHistory = useCallback(async () => {
    const [history, stats] = await Promise.all([api.listHistory(), api.stats()]);
    setBoot((b) => (b ? { ...b, history, stats } : b));
  }, []);

  useEffect(() => {
    refresh();
    const subs = [
      listen<Settings>("settings", (e) => setBoot((b) => (b ? { ...b, settings: e.payload } : b))),
      listen("models-changed", () => refreshModels()),
      listen("history-changed", () => refreshHistory()),
      listen<LogLine>("log", (e) => setBoot((b) => (b ? { ...b, logs: [...b.logs.slice(-399), e.payload] } : b))),
      listen<Progress>("model-progress", (e) => {
        setProgress((p) => ({ ...p, [e.payload.id]: e.payload }));
        if (e.payload.state !== "downloading") refreshModels();
      }),
    ];
    const onFocus = () => api.permissions().then((permissions) => setBoot((b) => (b ? { ...b, permissions } : b)));
    window.addEventListener("focus", onFocus);
    return () => {
      subs.forEach((p) => p.then((u) => u()));
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh, refreshModels, refreshHistory]);

  const update = useCallback(async (patch: DeepPartial<Settings>) => {
    const settings = await api.updateSettings(patch);
    setBoot((b) => (b ? { ...b, settings } : b));
  }, []);

  const value = useMemo<Store | null>(
    () =>
      boot && {
        ...boot,
        progress,
        update,
        refresh,
        refreshModels,
        refreshHistory,
        setPermissions: (permissions) => setBoot((b) => (b ? { ...b, permissions } : b)),
      },
    [boot, progress, update, refresh, refreshModels, refreshHistory],
  );

  if (!value) return fallback ?? null;
  return createElement(Ctx.Provider, { value }, children);
}

export function useKoe(): Store {
  const v = useContext(Ctx);
  if (!v) throw new Error("useKoe outside KoeProvider");
  return v;
}

/** Live input level in dBFS from the capture or monitor stream. */
export function useLevel(): number {
  const [db, setDb] = useState(-80);
  useEffect(() => {
    const un = listen<number>("level", (e) => setDb(e.payload));
    return () => void un.then((u) => u());
  }, []);
  return db;
}

// ---------- Formatting ----------

export function prettyShortcut(spec: string): string[] {
  return spec.split("+").map((p) => {
    const k = p.toLowerCase();
    if (k === "alt" || k === "option") return "⌥";
    if (k === "shift") return "⇧";
    if (k === "control" || k === "ctrl") return "⌃";
    if (k === "super" || k === "cmd" || k === "command" || k === "meta") return "⌘";
    if (p.startsWith("Key")) return p.slice(3);
    if (p.startsWith("Digit")) return p.slice(5);
    return p;
  });
}

export const shortcutText = (spec: string) => prettyShortcut(spec).join("");

export function formatBytes(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${Math.round(n / 1e6)} MB`;
  if (n >= 1e3) return `${Math.round(n / 1e3)} KB`;
  return `${n} B`;
}

export function formatDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export function formatWhen(ms: number): string {
  const d = new Date(ms);
  const today = new Date();
  const yest = new Date();
  yest.setDate(today.getDate() - 1);
  const time = d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  if (d.toDateString() === today.toDateString()) return `Today at ${time}`;
  if (d.toDateString() === yest.toDateString()) return `Yesterday at ${time}`;
  return `${d.toLocaleDateString([], { weekday: "short", day: "numeric", month: "short" })} at ${time}`;
}

export const LANGUAGES: [string, string][] = [
  ["auto", "All languages"],
  ["en", "English"],
  ["hi", "Hindi"],
  ["es", "Spanish"],
  ["fr", "French"],
  ["de", "German"],
  ["pt", "Portuguese"],
  ["it", "Italian"],
  ["nl", "Dutch"],
  ["ja", "Japanese"],
  ["zh", "Chinese"],
  ["ko", "Korean"],
  ["ru", "Russian"],
  ["ar", "Arabic"],
  ["ta", "Tamil"],
  ["bn", "Bengali"],
  ["mr", "Marathi"],
];
