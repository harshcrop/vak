import { getCurrentWindow } from "@tauri-apps/api/window";
import { ClipboardCopy, Mic, Power, Settings, WandSparkles, type LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { LevelBars, Logo, Toggle } from "../components/ui";
import { api, shortcutText, useLevel, useKoe, KoeProvider } from "../lib/api";

export function Panel() {
  return (
    <KoeProvider>
      <PanelBody />
    </KoeProvider>
  );
}

function Item({ icon: Icon, label, keys, onClick, disabled }: { icon: LucideIcon; label: string; keys?: string; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      className="panel-item"
      onClick={onClick}
      disabled={disabled}
      style={{ display: "flex", alignItems: "center", gap: 9, padding: "8px 9px", borderRadius: 8, fontSize: 13, color: "rgba(255,247,240,.88)", width: "100%", textAlign: "left", opacity: disabled ? 0.45 : 1 }}
    >
      <Icon strokeWidth={1.75} style={{ width: 14, height: 14, color: "var(--fg-3)" }} />
      {label}
      {keys && (
        <span className="mono" style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--fg-4)" }}>
          {keys}
        </span>
      )}
    </button>
  );
}

function PanelBody() {
  const { settings: s, update, models, history, defaultInput, permissions } = useKoe();
  const db = useLevel();
  const [status, setStatus] = useState<string | null>(null);
  const active = models.find((m) => m.id === s.activeModel);
  const ready = !!active?.installed && permissions.microphone !== "denied";
  const pp = s.experimental && s.postProcess.enabled;

  // Meter only while the panel is open.
  useEffect(() => {
    document.body.style.background = "transparent";
    const win = getCurrentWindow();
    const sync = (focused: boolean) => (focused ? api.startMonitor().catch(() => {}) : api.stopMonitor());
    sync(true);
    const un = win.onFocusChanged(({ payload }) => sync(payload));
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === ",") api.showMain("general");
      if (e.metaKey && e.key.toLowerCase() === "q") api.quit();
      if (e.key === "Escape") win.hide();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      api.stopMonitor();
      un.then((u) => u());
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const title = !s.enabled ? "Kōe is paused" : ready ? "Kōe is ready" : active?.installed ? "Microphone blocked" : "No model yet";

  return (
    <div style={{ height: "100%", borderRadius: 14, overflow: "hidden", background: "rgba(22,17,14,.62)", border: ".5px solid rgba(255,255,255,.14)", display: "flex", flexDirection: "column" }}>
      <style>{`.panel-item:hover:not(:disabled){background:rgba(255,255,255,.07)}`}</style>
      <div style={{ padding: "16px 16px 14px", display: "flex", flexDirection: "column", gap: 12, borderBottom: ".5px solid rgba(255,246,238,.1)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <Logo size={34} radius={10} />
          <div style={{ display: "flex", flexDirection: "column", gap: 2, minWidth: 0 }}>
            <span style={{ fontSize: 14, fontWeight: 600 }}>{title}</span>
            <span style={{ fontSize: 11.5, color: "var(--fg-3)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {active?.name ?? "—"} · {s.inputDevice ?? defaultInput ?? "Default mic"}
            </span>
          </div>
          <div style={{ marginLeft: "auto" }}>
            <Toggle on={s.enabled} onChange={(enabled) => update({ enabled })} label="Enable Kōe" />
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "11px 13px", borderRadius: 11, background: "rgba(22,17,14,.72)", border: ".5px solid rgba(255,255,255,.17)" }}>
          <LevelBars db={db} />
          <span style={{ fontSize: 12, color: "var(--fg-3)" }}>Input level</span>
          <span className="mono" style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--fg-4)" }}>
            {Math.round(db)} dB
          </span>
        </div>
      </div>
      <div style={{ padding: 8, display: "flex", flexDirection: "column", gap: 1, flex: 1, minHeight: 0 }}>
        <Item icon={Mic} label="Start dictation" keys={shortcutText(s.shortcut)} disabled={!s.enabled} onClick={() => api.startDictation(false)} />
        {pp && <Item icon={WandSparkles} label="Dictate with post-processing" keys={shortcutText(s.ppShortcut)} disabled={!s.enabled} onClick={() => api.startDictation(true)} />}
        {pp && (
          <Item
            icon={ClipboardCopy}
            label={status ?? "Clean up last transcript"}
            disabled={!history.length}
            onClick={async () => {
              setStatus("Cleaning up…");
              try {
                await api.cleanupLast();
                setStatus("Copied to clipboard");
              } catch (e) {
                setStatus("Post-processing failed");
              }
              setTimeout(() => setStatus(null), 1800);
            }}
          />
        )}
        <div style={{ height: 1, background: "rgba(255,246,238,.1)", margin: "6px 4px" }} />
        <div className="eyebrow" style={{ padding: "4px 9px 6px" }}>
          Recent
        </div>
        {history.length === 0 && <div style={{ padding: "7px 9px", fontSize: 12.5, color: "var(--fg-4)" }}>Nothing yet — hold {shortcutText(s.shortcut)} and talk.</div>}
        {history.slice(0, 2).map((e, i) => (
          <button
            key={e.id}
            className="panel-item"
            title="Copy"
            onClick={() => api.copy(e.text).then(() => getCurrentWindow().hide())}
            style={{
              padding: "7px 9px",
              borderRadius: 8,
              fontSize: 12.5,
              lineHeight: 1.4,
              textAlign: "left",
              color: i === 0 ? "rgba(255,247,240,.76)" : "rgba(255,247,240,.68)",
              background: i === 0 ? "rgba(22,17,14,.62)" : undefined,
              width: "100%",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            “{e.text}”
          </button>
        ))}
        <div style={{ height: 1, background: "rgba(255,246,238,.1)", margin: "6px 4px" }} />
        <Item icon={Settings} label="Settings…" keys="⌘," onClick={() => api.showMain()} />
        <Item icon={Power} label="Quit Kōe" keys="⌘Q" onClick={() => api.quit()} />
      </div>
    </div>
  );
}
