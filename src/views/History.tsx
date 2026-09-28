import { convertFileSrc } from "@tauri-apps/api/core";
import { Copy, FolderOpen, Pause, Play, Search, Star, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Header } from "../components/ui";
import { api, formatDuration, formatWhen, useVak, type Entry } from "../lib/api";
import type { ViewProps } from "../windows/MainWindow";

export function HistoryView({ toast }: ViewProps) {
  const { history, recordingsDir } = useVak();
  const [q, setQ] = useState("");
  const [starred, setStarred] = useState(false);
  const [playing, setPlaying] = useState<string | null>(null);

  const list = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return history.filter((e) => (!starred || e.starred) && (!needle || e.text.toLowerCase().includes(needle) || e.app.toLowerCase().includes(needle)));
  }, [history, q, starred]);

  return (
    <>
      <Header title="History" subtitle={`${history.length.toLocaleString()} transcripts, kept on this Mac`}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, width: 220, padding: "6px 11px", borderRadius: 8, background: "var(--well)", border: ".5px solid var(--line-2)" }}>
          <Search strokeWidth={1.75} style={{ width: 13, height: 13, color: "var(--fg-4)", flex: "none" }} />
          <input
            placeholder="Search transcripts"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            style={{ background: "transparent", border: 0, outline: "none", fontSize: 12, width: "100%" }}
          />
        </div>
        <button className="btn" onClick={() => setStarred((v) => !v)} style={starred ? { borderColor: "var(--accent-line)", color: "var(--accent-text)" } : undefined}>
          <Star strokeWidth={1.75} fill={starred ? "currentColor" : "none"} />
          Starred
        </button>
        <button className="btn" onClick={() => api.reveal("recordings")}>
          <FolderOpen strokeWidth={1.75} />
          Recordings
        </button>
      </Header>
      <div className="content-scroll" style={{ gap: 11 }}>
        {list.length === 0 && (
          <div className="empty">
            {history.length === 0 ? (
              <>
                Nothing here yet.
                <br />
                Hold your shortcut anywhere on your Mac and start talking.
              </>
            ) : (
              "No transcripts match."
            )}
          </div>
        )}
        {list.map((e, i) => (
          <HistoryCard
            key={e.id}
            e={e}
            hot={i === 0 && !q && !starred}
            src={e.audio ? convertFileSrc(`${recordingsDir}/${e.audio}`) : null}
            playing={playing === e.id}
            onPlay={(p) => setPlaying(p ? e.id : null)}
            toast={toast}
          />
        ))}
      </div>
    </>
  );
}

const SPEEDS = [1, 1.25, 1.5, 2];

function HistoryCard({
  e,
  hot,
  src,
  playing,
  onPlay,
  toast,
}: {
  e: Entry;
  hot: boolean;
  src: string | null;
  playing: boolean;
  onPlay: (p: boolean) => void;
  toast: (m: string) => void;
}) {
  const audio = useRef<HTMLAudioElement | null>(null);
  const [t, setT] = useState(0);
  const [speed, setSpeed] = useState(1);
  const total = e.durationMs / 1000;

  useEffect(() => {
    if (!playing) audio.current?.pause();
  }, [playing]);

  const toggle = () => {
    if (!src) return;
    if (!audio.current) {
      const a = new Audio(src);
      a.ontimeupdate = () => setT(a.currentTime);
      a.onended = () => {
        onPlay(false);
        setT(0);
      };
      audio.current = a;
    }
    const a = audio.current;
    a.playbackRate = speed;
    if (playing) {
      a.pause();
      onPlay(false);
    } else {
      a.play().then(() => onPlay(true)).catch(() => toast("Couldn’t play this recording"));
    }
  };

  const seek = (frac: number) => {
    if (!audio.current) toggle();
    const a = audio.current;
    if (a) {
      a.currentTime = frac * (a.duration || total);
      setT(a.currentTime);
    }
  };

  const played = total ? t / total : 0;

  return (
    <div className={`card${hot ? " hot" : ""}`} style={{ padding: "15px 17px", background: hot ? undefined : "var(--fill)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 9 }}>
        <span style={{ fontSize: 13.5, fontWeight: 600, color: hot ? "var(--fg)" : "var(--fg-2)" }}>{formatWhen(e.created)}</span>
        <span className={`tag${hot ? " accent" : ""}`}>{e.app}</span>
        {e.postProcessed && <span className="tag accent">Post-processed</span>}
        <span className="mono" style={{ fontSize: 11, color: "var(--fg-4)" }}>
          {e.words} words · {e.model}
        </span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button className="ghost-icon" title="Copy" onClick={() => api.copy(e.text).then(() => toast("Copied to clipboard"))}>
            <Copy strokeWidth={1.75} />
          </button>
          <button className="ghost-icon" title={e.starred ? "Unstar" : "Star"} onClick={() => api.starEntry(e.id)}>
            <Star strokeWidth={1.75} fill={e.starred ? "currentColor" : "none"} style={e.starred ? { color: "var(--accent-hi)" } : undefined} />
          </button>
          <button className="ghost-icon" title="Delete" onClick={() => api.deleteEntry(e.id)}>
            <Trash2 strokeWidth={1.75} />
          </button>
        </div>
      </div>
      <div className="selectable" style={{ fontSize: 14, lineHeight: 1.6, color: hot ? "var(--fg-2)" : "rgba(255,247,240,.72)", marginBottom: 13, textWrap: "pretty" } as React.CSSProperties}>
        {e.text}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <button
          onClick={toggle}
          disabled={!src}
          aria-label={playing ? "Pause" : "Play"}
          style={{
            width: 30,
            height: 30,
            flex: "none",
            borderRadius: "50%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: playing ? "var(--accent-grad)" : "rgba(255,255,255,.09)",
            border: playing ? 0 : ".5px solid rgba(255,255,255,.14)",
            boxShadow: playing ? "0 2px 8px rgba(180,70,10,.4)" : undefined,
            opacity: src ? 1 : 0.4,
          }}
        >
          {playing ? <Pause strokeWidth={2.25} style={{ width: 13, height: 13, color: "#fff" }} /> : <Play strokeWidth={2.25} style={{ width: 13, height: 13, color: "var(--fg-2)" }} />}
        </button>
        <span className="mono" style={{ fontSize: 11.5, color: "var(--fg-3)", width: 32 }}>
          {formatDuration(t * 1000)}
        </span>
        {src ? (
          <div
            style={{ flex: 1, display: "flex", alignItems: "center", gap: 2, height: 22 }}
            onClick={(ev) => {
              const r = ev.currentTarget.getBoundingClientRect();
              seek((ev.clientX - r.left) / r.width);
            }}
          >
            {(e.peaks.length ? e.peaks : Array(48).fill(0.3)).map((p, i, arr) => (
              <div
                key={i}
                style={{
                  flex: 1,
                  height: Math.max(3, p * 22),
                  borderRadius: 2,
                  background: i / arr.length < played ? "oklch(.74 .17 62)" : "rgba(255,247,240,.2)",
                }}
              />
            ))}
          </div>
        ) : (
          <span style={{ flex: 1, fontSize: 11.5, color: "var(--fg-5)" }}>Audio deleted</span>
        )}
        <span className="mono" style={{ fontSize: 11.5, color: "var(--fg-4)" }}>
          {formatDuration(e.durationMs)}
        </span>
        <button
          className="tag"
          style={{ padding: "3px 8px", border: ".5px solid rgba(255,255,255,.12)" }}
          onClick={() => {
            const next = SPEEDS[(SPEEDS.indexOf(speed) + 1) % SPEEDS.length];
            setSpeed(next);
            if (audio.current) audio.current.playbackRate = next;
          }}
        >
          {speed.toFixed(speed % 1 ? 2 : 1)}×
        </button>
      </div>
    </div>
  );
}
