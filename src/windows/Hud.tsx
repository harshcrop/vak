import { listen } from "@tauri-apps/api/event";
import { AlertCircle, Check, Mic, WandSparkles } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties } from "react";
import { formatDuration, useLevel, type HudState } from "../lib/api";

const glass: CSSProperties = {
  background: "rgba(16,11,9,.78)",
  border: ".5px solid rgba(255,255,255,.1)",
  boxShadow: "0 16px 40px rgba(0,0,0,.42), inset 0 1px 0 rgba(255,255,255,.1)",
  backdropFilter: "blur(34px) saturate(200%)",
  WebkitBackdropFilter: "blur(34px) saturate(200%)",
  animation: "fadein .16s ease-out",
};

const pill: CSSProperties = { ...glass, display: "flex", alignItems: "center", borderRadius: 999 };

const BAR_SHAPE = [0.3, 0.58, 0.92, 0.46, 0.73, 1, 0.38, 0.65, 0.85, 0.35, 0.54, 0.77, 0.27, 0.62, 0.42];

function Waveform({ db }: { db: number }) {
  // Scroll a short history of levels so the bars move like a live waveform.
  const [hist, setHist] = useState<number[]>(() => Array(15).fill(0));
  useEffect(() => {
    const loud = Math.min(1, Math.max(0, (db + 58) / 46));
    setHist((h) => [...h.slice(1), loud]);
  }, [db]);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 3, height: 26 }}>
      {hist.map((v, i) => {
        const h = 5 + 21 * Math.min(1, v * (0.55 + 0.45 * BAR_SHAPE[i]));
        const hot = v > 0.55 && i % 3 === 2;
        return <div key={i} style={{ width: 3, height: h, borderRadius: 2, background: hot ? "#fff" : "rgba(255,236,220,.55)", transition: "height .07s linear" }} />;
      })}
    </div>
  );
}

function Elapsed() {
  const start = useRef(Date.now());
  const [, tick] = useState(0);
  useEffect(() => {
    const t = setInterval(() => tick((n) => n + 1), 250);
    return () => clearInterval(t);
  }, []);
  return <span style={{ fontFamily: "var(--mono)", fontSize: 12, color: "rgba(255,255,255,.6)", minWidth: 30 }}>{formatDuration(Date.now() - start.current)}</span>;
}

export function Hud() {
  const [hud, setHud] = useState<HudState | null>(null);
  const [key, setKey] = useState(0);
  const db = useLevel();

  useEffect(() => {
    document.body.style.background = "transparent";
    const un = listen<HudState>("hud", (e) => {
      setHud(e.payload);
      if (e.payload.state === "listening") setKey((k) => k + 1);
    });
    return () => void un.then((u) => u());
  }, []);

  if (!hud) return null;
  const top = hud.position === "top";

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: top ? "flex-start" : "flex-end",
        padding: "14px 20px 22px",
      }}
    >
      {hud.state === "listening" && (
        <div key={key} style={{ ...pill, gap: 13, padding: "11px 18px", boxShadow: `${glass.boxShadow}, 0 0 0 4px oklch(.68 .19 48 / .22)` }}>
          <div style={{ width: 9, height: 9, borderRadius: "50%", background: "var(--rec)", boxShadow: "0 0 10px oklch(.66 .2 22 / .8)", animation: "koepulse 1.6s ease-in-out infinite" }} />
          <Waveform db={db} />
          <Elapsed />
          {hud.postProcess && <WandSparkles strokeWidth={1.75} style={{ width: 14, height: 14, color: "var(--accent-hi)" }} />}
        </div>
      )}

      {hud.state === "transcribing" && (
        <div style={{ ...glass, width: 420, borderRadius: 20, padding: "14px 18px", display: "flex", flexDirection: "column", gap: 10 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div style={{ display: "flex", gap: 3 }}>
              {[0, 0.18, 0.36].map((d) => (
                <div key={d} style={{ width: 5, height: 5, borderRadius: "50%", background: "var(--accent-hi)", animation: `koepulse 1.1s ease-in-out ${d}s infinite` }} />
              ))}
            </div>
            <span style={{ fontSize: 11.5, letterSpacing: 0.3, color: "rgba(255,255,255,.55)" }}>
              {hud.model} · {formatDuration(hud.durationMs)} of audio{hud.postProcess ? " · then post-processing" : ""}
            </span>
            <span style={{ marginLeft: "auto", fontSize: 11, padding: "3px 8px", borderRadius: 6, color: "rgba(255,255,255,.7)", background: "rgba(255,255,255,.08)" }}>{hud.app}</span>
          </div>
          <div style={{ fontSize: 14, lineHeight: 1.55, color: "rgba(255,255,255,.55)" }}>
            Transcribing on this Mac
            <span style={{ display: "inline-block", width: 2, height: 14, marginLeft: 3, verticalAlign: -2, background: "var(--accent-hi)", animation: "koepulse 1s infinite" }} />
          </div>
        </div>
      )}

      {hud.state === "done" && (
        <div style={{ ...pill, gap: 10, padding: "9px 16px" }}>
          <div style={{ width: 17, height: 17, borderRadius: "50%", background: "var(--ok)", boxShadow: "0 0 12px oklch(.72 .15 160 / .5)", display: "flex", alignItems: "center", justifyContent: "center" }}>
            <Check strokeWidth={2.5} style={{ width: 11, height: 11, color: "#0d1a12" }} />
          </div>
          <span style={{ fontSize: 12.5, color: "rgba(255,255,255,.85)" }}>
            {hud.words} {hud.words === 1 ? "word" : "words"} {hud.method === "clipboard" ? "copied" : "inserted"}
          </span>
          <span style={{ fontFamily: "var(--mono)", fontSize: 11.5, color: "rgba(255,255,255,.45)" }}>
            {hud.method === "clipboard" ? "⌘V to paste" : "⌘Z to undo"}
            {hud.realtime > 0 && ` · ${hud.realtime.toFixed(1)}× realtime`}
          </span>
        </div>
      )}

      {(hud.state === "hint" || hud.state === "error") && (
        <div style={{ ...pill, gap: 10, padding: "10px 16px" }}>
          <span style={{ fontSize: 15, fontWeight: 700, letterSpacing: -0.2, color: "rgba(255,255,255,.85)", lineHeight: 1 }}>
            Kōe
          </span>
          <div style={{ width: 1, height: 14, background: "rgba(255,255,255,.15)" }} />
          {hud.state === "error" ? (
            <AlertCircle strokeWidth={1.75} style={{ width: 14, height: 14, color: "var(--danger)" }} />
          ) : (
            <Mic strokeWidth={1.75} style={{ width: 14, height: 14, color: "rgba(255,255,255,.65)" }} />
          )}
          <span style={{ fontSize: 12.5, color: "rgba(255,255,255,.7)" }}>{hud.message}</span>
        </div>
      )}
    </div>
  );
}
