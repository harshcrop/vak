import { convertFileSrc } from "@tauri-apps/api/core";
import { Activity, AlertTriangle, Check, ChevronRight, FolderOpen, Share } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Header, LevelBars, Section } from "../components/ui";
import { api, formatBytes, useLevel, useVak } from "../lib/api";
import type { ViewProps } from "../windows/MainWindow";

const TAG_COLOR: Record<string, string> = {
  hotkey: "oklch(.78 .16 62)",
  ok: "oklch(.78 .16 62)",
  paste: "rgb(255,180,120)",
};

function isWarning(msg: string) {
  return /fail|could not|error|unavailable|invalid|denied/i.test(msg);
}

export function DiagnosticsView({ toast }: ViewProps) {
  const { models, loadedModel, stats, permissions, build, logs, settings } = useVak();
  const [filter, setFilter] = useState<"all" | "warn" | "audio">("all");
  const [testing, setTesting] = useState(false);
  const db = useLevel();
  const logRef = useRef<HTMLDivElement>(null);
  const loaded = models.find((m) => m.id === loadedModel);
  const active = models.find((m) => m.id === settings.activeModel);
  const allGranted = permissions.microphone === "granted" && permissions.accessibility;

  const shown = logs.filter((l) => (filter === "all" ? true : filter === "audio" ? l.tag === "audio" : isWarning(l.msg)));

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [shown.length]);

  const micTest = async () => {
    setTesting(true);
    try {
      const path = await api.micTest();
      await new Audio(convertFileSrc(path)).play();
    } catch (e) {
      toast(`Mic test failed: ${e}`);
    } finally {
      setTesting(false);
    }
  };

  return (
    <>
      <Header title="Diagnostics" subtitle="For when it misbehaves">
        <button className="btn" onClick={() => api.copyReport().then(() => toast("Report copied to clipboard"))}>
          <Share strokeWidth={1.75} />
          Copy report
        </button>
      </Header>
      <div className="content-scroll" style={{ gap: 14 }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 10 }}>
          <StatCard label="Model" value={loaded?.name ?? active?.name ?? "—"} sub={loaded ? `loaded · ${formatBytes(loaded.sizeBytes)}` : active?.installed ? "not in memory" : "not downloaded"} />
          <StatCard label="Median latency" value={stats.medianLatency != null ? `${stats.medianLatency} ms` : "—"} sub={`last ${Math.min(50, stats.transcripts)} takes`} />
          <div className="group" style={{ padding: "12px 13px" }}>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              Permissions
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              {allGranted ? (
                <Check strokeWidth={2.25} style={{ width: 13, height: 13, color: "var(--accent-hi)" }} />
              ) : (
                <AlertTriangle strokeWidth={2} style={{ width: 13, height: 13, color: "var(--danger)" }} />
              )}
              <span style={{ fontSize: 13, fontWeight: 500 }}>{allGranted ? "All granted" : "Needs attention"}</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--fg-4)" }}>
              mic {permissions.microphone === "granted" ? "✓" : "✕"} · accessibility {permissions.accessibility ? "✓" : "✕"}
            </div>
          </div>
          <StatCard label="Build" value={build.version} sub={`${build.arch} · ${build.os}`} />
        </div>

        <Section
          title="Live log"
          right={
            <>
              <span style={{ fontSize: 11.5, color: "var(--fg-4)", marginRight: 6 }}>tail · vak.log</span>
              {(
                [
                  ["all", "All"],
                  ["warn", "Warnings"],
                  ["audio", "Audio"],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  onClick={() => setFilter(id)}
                  style={{
                    padding: "4px 10px",
                    borderRadius: 7,
                    fontSize: 11.5,
                    color: filter === id ? "#fff" : "var(--fg-4)",
                    background: filter === id ? "rgba(255,255,255,.14)" : undefined,
                  }}
                >
                  {label}
                </button>
              ))}
            </>
          }
        >
          <div
            ref={logRef}
            className="selectable mono"
            style={{
              padding: "13px 14px",
              borderRadius: 12,
              background: "rgba(0,0,0,.45)",
              border: ".5px solid rgba(255,255,255,.12)",
              fontSize: 11.5,
              lineHeight: 1.85,
              color: "var(--fg-3)",
              height: 260,
              overflowY: "auto",
            }}
          >
            {shown.length === 0 && <div style={{ color: "var(--fg-5)" }}>Nothing logged yet.</div>}
            {shown.map((l, i) => (
              <div key={i}>
                <span style={{ color: "var(--fg-5)" }}>{l.ts}</span>{" "}
                <span style={{ color: isWarning(l.msg) ? "var(--danger)" : TAG_COLOR[l.tag] ?? "var(--fg-4)" }}>{l.tag}</span> {l.msg}
              </div>
            ))}
          </div>
        </Section>

        <div style={{ display: "flex", gap: 10 }}>
          <button className="group" style={{ flex: 1, display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", textAlign: "left" }} onClick={micTest} disabled={testing}>
            <Activity strokeWidth={1.75} style={{ width: 15, height: 15, color: "var(--fg-3)" }} />
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 12.5, color: "var(--fg-2)" }}>{testing ? "Recording… speak now" : "Mic test"}</div>
              <div style={{ fontSize: 11, color: "var(--fg-4)" }}>Record five seconds and play it back</div>
            </div>
            {testing ? <LevelBars db={db} /> : <ChevronRight strokeWidth={1.75} style={{ width: 15, height: 15, color: "var(--fg-5)" }} />}
          </button>
          <button className="group" style={{ flex: 1, display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", textAlign: "left" }} onClick={() => api.reveal("logs")}>
            <FolderOpen strokeWidth={1.75} style={{ width: 15, height: 15, color: "var(--fg-3)" }} />
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 12.5, color: "var(--fg-2)" }}>Reveal log folder</div>
              <div style={{ fontSize: 11, color: "var(--fg-4)" }}>~/Library/Logs/Vāk</div>
            </div>
            <ChevronRight strokeWidth={1.75} style={{ width: 15, height: 15, color: "var(--fg-5)" }} />
          </button>
        </div>
        {!allGranted && (
          <div className="tip">
            <AlertTriangle strokeWidth={1.75} style={{ color: "var(--danger)" }} />
            <span style={{ flex: 1 }}>
              {permissions.microphone !== "granted" ? "Microphone access is missing. " : ""}
              {!permissions.accessibility ? "Accessibility access is needed to paste into other apps." : ""}
            </span>
            {permissions.microphone !== "granted" && (
              <button className="btn" onClick={() => api.openPrivacy("microphone")}>
                Microphone…
              </button>
            )}
            {!permissions.accessibility && (
              <button className="btn primary" onClick={() => api.openPrivacy("accessibility")}>
                Accessibility…
              </button>
            )}
          </div>
        )}
      </div>
    </>
  );
}

function StatCard({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div className="group" style={{ padding: "12px 13px" }}>
      <div className="eyebrow" style={{ marginBottom: 6 }}>
        {label}
      </div>
      <div style={{ fontSize: 13, fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{value}</div>
      <div style={{ fontSize: 11, color: "var(--fg-4)" }}>{sub}</div>
    </div>
  );
}
