import { listen } from "@tauri-apps/api/event";
import {
  BookA,
  BrainCircuit,
  FlaskConical,
  History,
  Info,
  SendHorizontal,
  SlidersHorizontal,
  WandSparkles,
  Wrench,
  type LucideIcon,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Logo, Toast } from "../components/ui";
import { KoeProvider, useKoe } from "../lib/api";
import { AboutView } from "../views/About";
import { AdvancedView } from "../views/Advanced";
import { DiagnosticsView } from "../views/Diagnostics";
import { GeneralView } from "../views/General";
import { HistoryView } from "../views/History";
import { ModelsView } from "../views/Models";
import { Onboarding } from "../views/Onboarding";
import { OutputView } from "../views/Output";
import { PostProcessView } from "../views/PostProcess";
import { VocabularyView } from "../views/Vocabulary";

export type ViewProps = { toast: (m: string) => void };

const VIEWS: { id: string; label: string; icon: LucideIcon; C: (p: ViewProps) => JSX.Element; experimental?: boolean }[] = [
  { id: "general", label: "General", icon: SlidersHorizontal, C: GeneralView },
  { id: "models", label: "Models", icon: BrainCircuit, C: ModelsView },
  { id: "output", label: "Output", icon: SendHorizontal, C: OutputView },
  { id: "vocabulary", label: "Vocabulary", icon: BookA, C: VocabularyView },
  { id: "postprocess", label: "Post-process", icon: WandSparkles, C: PostProcessView, experimental: true },
  { id: "history", label: "History", icon: History, C: HistoryView },
  { id: "advanced", label: "Advanced", icon: Wrench, C: AdvancedView },
  { id: "diagnostics", label: "Diagnostics", icon: FlaskConical, C: DiagnosticsView },
  { id: "about", label: "About", icon: Info, C: AboutView },
];

export function MainWindow() {
  return (
    <KoeProvider fallback={<div className="shell" data-tauri-drag-region />}>
      <Shell />
    </KoeProvider>
  );
}

function Shell() {
  const { settings, models, stats, build, update, loadedModel } = useKoe();
  const [view, setView] = useState("general");
  const [toast, setToast] = useState<string | null>(null);
  const clearToast = useCallback(() => setToast(null), []);

  useEffect(() => {
    const un = listen<string>("navigate", (e) => setView(e.payload));
    return () => void un.then((u) => u());
  }, []);

  const visible = VIEWS.filter((v) => !v.experimental || settings.experimental);
  const Current = (visible.find((v) => v.id === view) ?? visible[0]).C;
  const installed = models.filter((m) => m.installed);
  const active = models.find((m) => m.id === settings.activeModel);
  const ready = !!active?.installed;

  if (!settings.onboarded) {
    return (
      <div className="shell">
        <Onboarding />
      </div>
    );
  }

  return (
    <div className="shell">
      <div className="shell-body">
        <aside className="sidebar">
          <div className="sidebar-drag" data-tauri-drag-region />
          <div className="brand" data-tauri-drag-region>
            <Logo size={34} />
            <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
              <span className="brand-name">Kōe</span>
              <span className="brand-sub">voice, on-device</span>
            </div>
          </div>
          <nav className="nav">
            {visible.map(({ id, label, icon: Icon }) => (
              <button key={id} className={`nav-item${id === view ? " active" : ""}`} onClick={() => setView(id)}>
                <Icon strokeWidth={1.75} />
                {label}
              </button>
            ))}
          </nav>
          <div className="side-card">
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              This week
            </div>
            <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
              <span style={{ fontSize: 19, fontWeight: 600, letterSpacing: -0.3 }}>{stats.wordsWeek.toLocaleString()}</span>
              <span style={{ fontSize: 11.5, color: "var(--fg-4)" }}>words spoken</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--fg-4)", marginTop: 8, lineHeight: 1.45 }}>Nothing leaves this Mac.</div>
          </div>
        </aside>
        <main className="content">
          <Current toast={setToast} />
        </main>
      </div>
      <footer className="statusbar">
        <div className={`dot ${!settings.enabled ? "off" : ready ? "ok" : "warn"}`} />
        {installed.length > 0 ? (
          <select
            className="status-select"
            value={settings.activeModel}
            onChange={(e) => update({ activeModel: e.target.value })}
            aria-label="Active model"
          >
            {!active?.installed && <option value={settings.activeModel}>{active?.name ?? settings.activeModel} (not downloaded)</option>}
            {installed.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </select>
        ) : (
          <button className="status-select" onClick={() => setView("models")}>
            No model yet — download one
          </button>
        )}
        {loadedModel && <span style={{ color: "var(--fg-4)" }}>· in memory</span>}
        {!settings.enabled && <span style={{ color: "var(--fg-4)" }}>· shortcuts paused</span>}
        <span style={{ marginLeft: "auto", color: "var(--fg-4)" }}>v{build.version}</span>
      </footer>
      <Toast message={toast} onDone={clearToast} />
    </div>
  );
}
