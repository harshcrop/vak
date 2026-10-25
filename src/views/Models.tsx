import { ArrowDownToLine, Check, CirclePlay, FolderOpen, Globe, Pause, Trash2 } from "lucide-react";
import { Header, Meter, Select } from "../components/ui";
import { api, formatBytes, LANGUAGES, useKoe, type ModelStatus } from "../lib/api";
import type { ViewProps } from "../windows/MainWindow";

export function ModelsView({ toast }: ViewProps) {
  const { models, settings, update, progress, loadedModel } = useKoe();
  const active = models.find((m) => m.id === settings.activeModel);
  const rest = models.filter((m) => m.id !== settings.activeModel);

  const download = (m: ModelStatus) =>
    api.downloadModel(m.id).catch((e) => toast(`Download failed: ${e}`));

  return (
    <>
      <Header title="Models" subtitle="Pick the engine Kōe uses to turn speech into text">
        <Select
          icon={Globe}
          label="Language"
          value={settings.language}
          options={LANGUAGES}
          onChange={(language) => update({ language })}
        />
        <button className="btn" onClick={() => api.reveal("models")}>
          <FolderOpen strokeWidth={1.75} />
          Models folder
        </button>
      </Header>
      <div className="content-scroll" style={{ gap: 12 }}>
        {active && (
          <ActiveCard m={active} loaded={loadedModel === active.id} progress={progress[active.id]} onDownload={() => download(active)} />
        )}
        {rest.map((m) => (
          <ModelCard
            key={m.id}
            m={m}
            progress={progress[m.id]}
            onUse={() => update({ activeModel: m.id }).then(() => toast(`${m.name} is now active`))}
            onDownload={() => download(m)}
            onDelete={() => api.deleteModel(m.id).then(() => toast(`${m.name} removed`))}
          />
        ))}
      </div>
    </>
  );
}

function progressOf(m: ModelStatus, p?: { downloaded: number; total: number; state: string }) {
  const downloaded = p?.downloaded ?? m.downloaded;
  const total = p?.total || m.total;
  const downloading = m.downloading || p?.state === "downloading";
  return { downloaded, total, downloading, pct: total ? Math.min(100, (downloaded / total) * 100) : 0 };
}

function ActiveCard({ m, loaded, progress, onDownload }: { m: ModelStatus; loaded: boolean; progress?: any; onDownload: () => void }) {
  const p = progressOf(m, progress);
  return (
    <div className="card hot" style={{ padding: "16px 18px" }}>
      <div style={{ display: "flex", alignItems: "flex-start", gap: 14 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 9, marginBottom: 5 }}>
            <span style={{ fontSize: 15, fontWeight: 600 }}>{m.name}</span>
            <span className="badge accent">Active</span>
            {m.recommended && <span className="badge green">Recommended</span>}
          </div>
          <div style={{ fontSize: 12.5, lineHeight: 1.5, color: "var(--fg-3)", maxWidth: 520 }}>{m.description}</div>
          <div style={{ display: "flex", gap: 22, marginTop: 13, flexWrap: "wrap" }}>
            <Stat label="Speed">
              <Meter value={m.speed} />
            </Stat>
            <Stat label="Accuracy">
              <Meter value={m.accuracy} />
            </Stat>
            <Stat label="Size">{formatBytes(m.sizeBytes)}</Stat>
            <Stat label="Latency">~{m.latencyMs} ms</Stat>
            <Stat label="Languages">{m.languages}</Stat>
          </div>
          {!m.installed && m.available && (
            <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 14 }}>
              <div className="progress">
                <div style={{ width: `${p.pct}%` }} />
              </div>
              <span className="mono" style={{ fontSize: 11.5, color: "var(--fg-3)" }}>
                {p.downloading ? `${Math.round(p.pct)}% · ${formatBytes(p.downloaded)} of ${formatBytes(p.total)}` : "Not downloaded yet"}
              </span>
            </div>
          )}
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 7, alignItems: "flex-end" }}>
          {m.installed ? (
            <>
              <div className="btn">
                <Check strokeWidth={1.75} style={{ color: "var(--accent-hi)" }} />
                In use
              </div>
              <span style={{ fontSize: 11, color: "var(--fg-4)" }}>
                {loaded ? "Loaded in memory" : "Loads on first use"}
                {m.updated ? ` · ${m.updated}` : ""}
              </span>
            </>
          ) : !m.available ? (
            <span className="badge muted">Coming soon</span>
          ) : p.downloading ? (
            <button className="btn" onClick={() => api.pauseDownload(m.id)}>
              <Pause strokeWidth={1.75} />
              Pause
            </button>
          ) : (
            <button className="btn primary" onClick={onDownload}>
              <ArrowDownToLine strokeWidth={1.75} />
              {p.downloaded > 0 ? "Resume" : "Download"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function Stat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
      <span className="eyebrow" style={{ fontSize: 10 }}>
        {label}
      </span>
      <span style={{ fontSize: 12.5, fontWeight: 500, color: "rgba(255,247,240,.78)" }}>{children}</span>
    </div>
  );
}

function ModelCard({
  m,
  progress,
  onUse,
  onDownload,
  onDelete,
}: {
  m: ModelStatus;
  progress?: any;
  onUse: () => void;
  onDownload: () => void;
  onDelete: () => void;
}) {
  const p = progressOf(m, progress);
  const showBar = !m.installed && (p.downloading || p.downloaded > 0);
  return (
    <div className={`card${m.available ? "" : " dim"}`} style={{ display: "flex", alignItems: "center", gap: 14 }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 9, marginBottom: 4, flexWrap: "wrap" }}>
          <span style={{ fontSize: 14.5, fontWeight: 600 }}>{m.name}</span>
          <span style={{ fontSize: 10.5, color: "var(--fg-4)" }}>
            {formatBytes(m.sizeBytes)} · {m.languages} languages
          </span>
          {m.installed && <span className="badge muted">Installed</span>}
          {m.recommended && <span className="badge green">Recommended</span>}
          {!m.available && <span className="badge muted">Coming soon</span>}
        </div>
        <div style={{ fontSize: 12.5, color: "var(--fg-3)", marginBottom: showBar ? 9 : 0 }}>{m.description}</div>
        {showBar && (
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div className="progress">
              <div style={{ width: `${p.pct}%` }} />
            </div>
            <span className="mono" style={{ fontSize: 11.5, color: "var(--fg-3)" }}>
              {Math.round(p.pct)}% · {formatBytes(p.downloaded)} of {formatBytes(p.total)}
              {!p.downloading && " · paused"}
            </span>
          </div>
        )}
      </div>
      {m.installed ? (
        <div style={{ display: "flex", gap: 6 }}>
          <button className="icon-btn" title="Delete model" onClick={onDelete}>
            <Trash2 strokeWidth={1.75} />
          </button>
          <button className="btn" onClick={onUse}>
            <CirclePlay strokeWidth={1.75} />
            Use
          </button>
        </div>
      ) : !m.available ? null : p.downloading ? (
        <button className="btn" onClick={() => api.pauseDownload(m.id)}>
          <Pause strokeWidth={1.75} />
          Pause
        </button>
      ) : (
        <button className={`btn${m.recommended ? " primary" : ""}`} onClick={onDownload}>
          <ArrowDownToLine strokeWidth={1.75} />
          {p.downloaded > 0 ? "Resume" : "Download"}
        </button>
      )}
    </div>
  );
}
