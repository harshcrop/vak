import { ChevronsUpDown, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { prettyShortcut } from "../lib/api";

export function Logo({ size = 30, radius }: { size?: number; radius?: number }) {
  return (
    <div className="logo" style={{ width: size, height: size, borderRadius: radius ?? size * 0.29 }}>
      <span style={{ fontSize: size * 0.52 }}>वा</span>
    </div>
  );
}

export function Toggle({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <button role="switch" aria-checked={on} aria-label={label} className={`toggle${on ? " on" : ""}`} onClick={() => onChange(!on)}>
      <div className="knob" />
    </button>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  quiet,
}: {
  value: T;
  options: [T, string][];
  onChange: (v: T) => void;
  quiet?: boolean;
}) {
  return (
    <div className={`seg${quiet ? " quiet" : ""}`} role="radiogroup">
      {options.map(([v, label]) => (
        <button key={v} role="radio" aria-checked={v === value} className={v === value ? "on" : ""} onClick={() => onChange(v)}>
          {label}
        </button>
      ))}
    </div>
  );
}

export function Select<T extends string | number>({
  value,
  options,
  onChange,
  icon: Icon,
  style,
  label,
}: {
  value: T;
  options: [T, string][];
  onChange: (v: T) => void;
  icon?: LucideIcon;
  style?: CSSProperties;
  label?: string;
}) {
  return (
    <div className={`select-wrap${Icon ? " has-lead" : ""}`}>
      {Icon && <Icon className="lead" strokeWidth={1.75} />}
      <select
        aria-label={label}
        style={style}
        value={String(value)}
        onChange={(e) => {
          const raw = e.target.value;
          const match = options.find(([v]) => String(v) === raw);
          if (match) onChange(match[0]);
        }}
      >
        {options.map(([v, l]) => (
          <option key={String(v)} value={String(v)}>
            {l}
          </option>
        ))}
      </select>
      <ChevronsUpDown className="chev" strokeWidth={1.75} />
    </div>
  );
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  format,
  style,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  format: (v: number) => string;
  style?: CSSProperties;
}) {
  // Local state while dragging; commit on release so we don't write settings 60×/s.
  const [local, setLocal] = useState(value);
  useEffect(() => setLocal(value), [value]);
  const pct = ((local - min) / (max - min)) * 100;
  return (
    <div className="slider" style={style}>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={local}
        style={{ ["--p" as string]: `${pct}%` }}
        onChange={(e) => setLocal(Number(e.target.value))}
        onPointerUp={() => local !== value && onChange(local)}
        onKeyUp={() => local !== value && onChange(local)}
      />
      <span className="val">{format(local)}</span>
    </div>
  );
}

export function Section({ title, children, right }: { title: string; children: ReactNode; right?: ReactNode }) {
  return (
    <div className="section">
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span className="eyebrow">{title}</span>
        {right && <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>{right}</div>}
      </div>
      {children}
    </div>
  );
}

export function Row({
  title,
  desc,
  icon: Icon,
  children,
  sub,
}: {
  title: ReactNode;
  desc?: ReactNode;
  icon?: LucideIcon;
  children?: ReactNode;
  sub?: boolean;
}) {
  return (
    <div className={`row${sub ? " sub" : ""}`}>
      {Icon && <Icon className="row-icon" strokeWidth={1.75} />}
      <div className="row-main">
        <div className="row-title">{title}</div>
        {desc && <div className="row-desc">{desc}</div>}
      </div>
      {children}
    </div>
  );
}

export function Header({ title, subtitle, children }: { title: string; subtitle?: string; children?: ReactNode }) {
  return (
    <div className="content-head" data-tauri-drag-region>
      <div data-tauri-drag-region>
        <div className="content-title" data-tauri-drag-region>
          {title}
        </div>
        {subtitle && (
          <div className="content-sub" data-tauri-drag-region>
            {subtitle}
          </div>
        )}
      </div>
      {children && <div style={{ display: "flex", gap: 8, alignItems: "center" }}>{children}</div>}
    </div>
  );
}

export function Keys({ spec, size = 13 }: { spec: string; size?: number }) {
  return (
    <span style={{ display: "inline-flex", gap: 5 }}>
      {prettyShortcut(spec).map((k, i) => (
        <span key={i} className="kbd" style={{ fontSize: size }}>
          {k}
        </span>
      ))}
    </span>
  );
}

const PATTERN = [0.35, 0.7, 1, 0.5, 0.82, 0.4];

/** Six little bars that follow the input level. */
export function LevelBars({ db, height = 18 }: { db: number; height?: number }) {
  const loud = Math.min(1, Math.max(0, (db + 62) / 50));
  return (
    <div className="levelbars" style={{ height }}>
      {PATTERN.map((p, i) => (
        <i key={i} style={{ height: Math.max(3, height * (0.18 + 0.82 * loud * p)), opacity: 0.45 + 0.55 * p }} />
      ))}
    </div>
  );
}

export function Meter({ value }: { value: number }) {
  return (
    <div className="meter">
      {[1, 2, 3, 4, 5].map((i) => (
        <i key={i} className={i <= value ? "on" : ""} />
      ))}
    </div>
  );
}

export function Toast({ message, onDone }: { message: string | null; onDone: () => void }) {
  useEffect(() => {
    if (!message) return;
    const t = setTimeout(onDone, 2200);
    return () => clearTimeout(t);
  }, [message, onDone]);
  return message ? <div className="toast">{message}</div> : null;
}

const MOD_ORDER = ["Control", "Alt", "Shift", "Super"];

/** Click, then press the combination. Esc cancels. */
export function ShortcutField({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const ref = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") {
        setRecording(false);
        setPreview(null);
        return;
      }
      const mods = [e.ctrlKey && "Control", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean) as string[];
      const isMod = ["Alt", "Shift", "Control", "Meta", "OS"].some((m) => e.key === m || e.code.startsWith(m));
      if (isMod) {
        setPreview(mods.sort((a, b) => MOD_ORDER.indexOf(a) - MOD_ORDER.indexOf(b)).join("+"));
        return;
      }
      const isF = /^F\d{1,2}$/.test(e.code);
      if (mods.length === 0 && !isF) return; // need a modifier for anything but F-keys
      const spec = [...mods.sort((a, b) => MOD_ORDER.indexOf(a) - MOD_ORDER.indexOf(b)), e.code].join("+");
      onChange(spec);
      setRecording(false);
      setPreview(null);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <button
      ref={ref}
      onClick={() => setRecording((r) => !r)}
      onBlur={() => {
        setRecording(false);
        setPreview(null);
      }}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "7px 12px",
        borderRadius: 10,
        flex: "none",
        background: "linear-gradient(150deg,rgba(52,36,26,.9),rgba(26,20,16,.8))",
        border: `1px solid oklch(.68 .19 48 / ${recording ? 0.9 : 0.55})`,
        boxShadow: `0 0 0 3px oklch(.68 .19 48 / ${recording ? 0.3 : 0.16})`,
        minWidth: 120,
        justifyContent: "center",
      }}
    >
      {recording ? (
        preview ? (
          <Keys spec={preview} />
        ) : (
          <span style={{ fontSize: 12.5, color: "var(--accent-text)" }}>Press keys…</span>
        )
      ) : (
        <Keys spec={value} />
      )}
    </button>
  );
}
