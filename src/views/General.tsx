import { Mic, Play, RefreshCw, RotateCcw, Volume2 } from "lucide-react";
import { useEffect } from "react";
import { Header, LevelBars, Row, Section, Segmented, Select, ShortcutField, Slider, Toggle } from "../components/ui";
import { api, useLevel, useVak } from "../lib/api";
import type { ViewProps } from "../windows/MainWindow";

function levelWords(db: number) {
  if (db < -58) return "silent";
  if (db < -40) return "quiet";
  if (db > -6) return "too loud";
  return "sounds good";
}

export function GeneralView({ toast }: ViewProps) {
  const { settings: s, update, inputDevices, outputDevices, defaultInput, defaultOutput, refresh } = useVak();
  const db = useLevel();

  useEffect(() => {
    api.startMonitor().catch(() => {});
    return () => void api.stopMonitor();
  }, [s.inputDevice]);

  const restartMonitor = async () => {
    await api.stopMonitor();
    await refresh();
    api.startMonitor().catch(() => {});
  };

  return (
    <>
      <Header title="General" subtitle="How Vāk starts listening, and what it sounds like">
        <button
          className="btn"
          onClick={async () => {
            await api.resetGeneral();
            await refresh();
            toast("General settings reset");
          }}
        >
          <RotateCcw strokeWidth={1.75} />
          Reset defaults
        </button>
      </Header>
      <div className="content-scroll">
        <Section title="Recording">
          <div className="group">
            <Row title="Transcribe shortcut" desc="Click the field, then press the keys you want.">
              <ShortcutField value={s.shortcut} onChange={(shortcut) => update({ shortcut })} />
            </Row>
            {s.experimental && s.postProcess.enabled && (
              <Row title="Transcribe + post-process" desc="Same as above, with an LLM pass before the paste.">
                <ShortcutField value={s.ppShortcut} onChange={(ppShortcut) => update({ ppShortcut })} />
              </Row>
            )}
            <Row title="Recording mode" desc={s.recordingMode === "ptt" ? "Push to talk holds the mic open only while the keys are down." : "Press once to start, again to stop."}>
              <Segmented
                value={s.recordingMode}
                options={[
                  ["ptt", "Push to talk"],
                  ["toggle", "Toggle"],
                ]}
                onChange={(recordingMode) => update({ recordingMode })}
              />
            </Row>
            <Row title="Cancel with Esc" desc="Drop the take without inserting anything.">
              <Toggle on={s.cancelWithEsc} onChange={(cancelWithEsc) => update({ cancelWithEsc })} />
            </Row>
          </div>
        </Section>

        <Section title="Sound">
          <div className="group">
            <Row
              title="Microphone"
              desc={
                <span style={{ display: "flex", alignItems: "center", gap: 9, marginTop: 3 }}>
                  <LevelBars db={db} height={16} />
                  <span className="mono" style={{ fontSize: 11.5, color: "var(--fg-4)" }}>
                    {Math.round(db)} dB · {levelWords(db)}
                  </span>
                </span>
              }
            >
              <Select
                icon={Mic}
                label="Microphone"
                value={s.inputDevice ?? ""}
                options={[["", `System default${defaultInput ? ` (${defaultInput})` : ""}`], ...inputDevices.map((d) => [d, d] as [string, string])]}
                onChange={(v) => update({ inputDevice: v || null })}
              />
              <button className="icon-btn" title="Refresh devices" onClick={restartMonitor}>
                <RefreshCw strokeWidth={1.75} />
              </button>
            </Row>
            <Row title="Mute other audio while recording" desc="Music and calls duck until you release the key.">
              <Toggle on={s.muteOtherAudio} onChange={(muteOtherAudio) => update({ muteOtherAudio })} />
            </Row>
            <Row title="Audio feedback" desc="A short chime when recording starts and stops.">
              <Toggle on={s.audioFeedback} onChange={(audioFeedback) => update({ audioFeedback })} />
            </Row>
            {s.audioFeedback && (
              <div className="row sub" style={{ flexDirection: "column", alignItems: "stretch", gap: 11 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <span style={{ fontSize: 13, color: "var(--fg-2)", width: 104, flex: "none" }}>Output device</span>
                  <Select
                    icon={Volume2}
                    label="Output device"
                    value={s.outputDevice ?? ""}
                    options={[["", `System default${defaultOutput ? ` (${defaultOutput})` : ""}`], ...outputDevices.map((d) => [d, d] as [string, string])]}
                    onChange={(v) => update({ outputDevice: v || null })}
                  />
                  <button className="icon-btn" title="Refresh devices" onClick={() => refresh()}>
                    <RefreshCw strokeWidth={1.75} />
                  </button>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <span style={{ fontSize: 13, color: "var(--fg-2)", width: 104, flex: "none" }}>Chime volume</span>
                  <Slider
                    style={{ flex: 1, maxWidth: 380 }}
                    value={Math.round(s.chimeVolume * 100)}
                    min={0}
                    max={100}
                    onChange={(v) => update({ chimeVolume: v / 100 }).then(() => api.testChime())}
                    format={(v) => `${v}%`}
                  />
                  <button className="btn" onClick={() => api.testChime()}>
                    <Play strokeWidth={1.75} />
                    Test
                  </button>
                </div>
              </div>
            )}
          </div>
        </Section>
      </div>
    </>
  );
}
