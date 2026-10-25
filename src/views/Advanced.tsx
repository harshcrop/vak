import { EyeOff, LayoutPanelTop, MemoryStick, Power, Trash2, Info, EyeOff as Hidden, Monitor, FlaskConical } from "lucide-react";
import { Header, Row, Section, Select, Toggle } from "../components/ui";
import { api, formatBytes, useKoe, type Settings } from "../lib/api";
import type { ViewProps } from "../windows/MainWindow";

const POSITIONS: { id: Settings["overlayPosition"]; label: string }[] = [
  { id: "top", label: "Top" },
  { id: "bottom", label: "Bottom" },
  { id: "hidden", label: "Hidden" },
];

function PositionTile({ id, on, onClick }: { id: Settings["overlayPosition"]; on: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      title={POSITIONS.find((p) => p.id === id)?.label}
      style={{
        width: 46,
        height: 30,
        borderRadius: 7,
        background: "rgba(0,0,0,.35)",
        border: on ? "1px solid oklch(.68 .19 48 / .7)" : ".5px solid rgba(255,255,255,.12)",
        boxShadow: on ? "0 0 0 3px oklch(.68 .19 48 / .16)" : undefined,
        display: "flex",
        alignItems: id === "top" ? "flex-start" : id === "bottom" ? "flex-end" : "center",
        justifyContent: "center",
        padding: 4,
      }}
    >
      {id === "hidden" ? (
        <EyeOff strokeWidth={1.75} style={{ width: 13, height: 13, color: "var(--fg-5)" }} />
      ) : (
        <div style={{ width: 22, height: 5, borderRadius: 3, background: on ? "oklch(.74 .17 62)" : "rgba(255,247,240,.35)" }} />
      )}
    </button>
  );
}

export function AdvancedView({ toast }: ViewProps) {
  const { settings: s, update, stats, models, refreshHistory } = useKoe();
  const active = models.find((m) => m.id === s.activeModel);

  return (
    <>
      <Header title="Advanced" subtitle="Behaviour, memory and retention" />
      <div className="content-scroll">
        <Section title="App">
          <div className="group">
            <Row icon={Power} title="Launch at login" desc="Starts straight to the menu bar.">
              <Toggle on={s.launchAtLogin} onChange={(launchAtLogin) => update({ launchAtLogin })} />
            </Row>
            <Row icon={Hidden} title="Start hidden" desc="Don’t open this window when Kōe launches.">
              <Toggle on={s.startHidden} onChange={(startHidden) => update({ startHidden })} />
            </Row>
            <Row icon={Monitor} title="Show menu bar icon" desc="The ō mark. Its macron moves while you speak.">
              <Toggle on={s.showTrayIcon} onChange={(showTrayIcon) => update({ showTrayIcon })} />
            </Row>
            <Row icon={LayoutPanelTop} title="Overlay position" desc="Where the listening pill sits while you talk.">
              <div style={{ display: "flex", gap: 7, flex: "none" }}>
                {POSITIONS.map((p) => (
                  <PositionTile key={p.id} id={p.id} on={s.overlayPosition === p.id} onClick={() => update({ overlayPosition: p.id })} />
                ))}
              </div>
            </Row>
            <Row
              icon={MemoryStick}
              title="Unload model when idle"
              desc={`Frees ${active ? formatBytes(active.sizeBytes) : "the model’s"} memory. Reloads in about a second.`}
            >
              <Select
                label="Unload model"
                value={s.unloadAfterMin}
                options={[
                  [0, "Never"],
                  [1, "After 1 min"],
                  [5, "After 5 min"],
                  [15, "After 15 min"],
                  [60, "After 1 hour"],
                ]}
                onChange={(unloadAfterMin) => update({ unloadAfterMin })}
              />
            </Row>
            <Row
              icon={FlaskConical}
              title={
                <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                  Experimental features {s.experimental && <span className="tag accent">Post-process unlocked</span>}
                </span>
              }
              desc="Shows the LLM post-processing screen and its shortcut."
            >
              <Toggle on={s.experimental} onChange={(experimental) => update({ experimental })} />
            </Row>
          </div>
        </Section>

        <Section title="Retention">
          <div style={{ display: "flex", gap: 10 }}>
            <div className="group" style={{ flex: 1, padding: "13px 15px" }}>
              <div style={{ fontSize: 12.5, color: "var(--fg-2)", marginBottom: 8 }}>Keep transcripts</div>
              <Select
                label="Keep transcripts"
                value={s.keepTranscripts}
                options={[
                  [50, "50 entries"],
                  [100, "100 entries"],
                  [500, "500 entries"],
                  [2000, "2,000 entries"],
                  [10000, "10,000 entries"],
                ]}
                onChange={(keepTranscripts) => update({ keepTranscripts })}
              />
            </div>
            <div className="group" style={{ flex: 1, padding: "13px 15px" }}>
              <div style={{ fontSize: 12.5, color: "var(--fg-2)", marginBottom: 8 }}>Keep audio for</div>
              <Select
                label="Keep audio"
                value={s.deleteAudioAfterDays}
                options={[
                  [0, "Don’t keep audio"],
                  [1, "1 day"],
                  [3, "3 days"],
                  [7, "7 days"],
                  [30, "30 days"],
                ]}
                onChange={(deleteAudioAfterDays) => update({ deleteAudioAfterDays })}
              />
            </div>
            <div className="group" style={{ flex: 1, padding: "13px 15px", display: "flex", flexDirection: "column", justifyContent: "space-between", gap: 8 }}>
              <div style={{ fontSize: 12.5, color: "var(--fg-2)" }}>On disk now</div>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
                <span style={{ fontSize: 22, fontWeight: 600, letterSpacing: -0.4 }}>{formatBytes(stats.diskBytes)}</span>
                <button
                  className="btn danger"
                  onClick={async () => {
                    await api.clearAudio();
                    await refreshHistory();
                    toast("Recordings deleted — transcripts kept");
                  }}
                >
                  <Trash2 strokeWidth={1.75} />
                  Clear audio
                </button>
              </div>
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 11.5, color: "var(--fg-4)" }}>
            <Info strokeWidth={1.75} style={{ width: 13, height: 13 }} />
            Starred transcripts are never pruned. Older audio is removed on launch.
          </div>
        </Section>
      </div>
    </>
  );
}
