import { Check } from "lucide-react";
import { useEffect, useState } from "react";
import { Keys, Logo } from "../components/ui";
import { api, formatBytes, shortcutText, useKoe } from "../lib/api";

type StepState = "done" | "current" | "todo";

function Step({ state, title, desc, action }: { state: StepState; title: string; desc: string; action?: React.ReactNode }) {
  const hot = state === "current";
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 13,
        padding: "14px 16px",
        borderRadius: 12,
        textAlign: "left",
        background: hot ? "linear-gradient(150deg,rgba(52,36,26,.74),rgba(26,20,16,.62))" : state === "done" ? "rgba(22,17,14,.72)" : "rgba(22,17,14,.52)",
        border: hot ? "1px solid oklch(.68 .19 48 / .45)" : ".5px solid rgba(255,255,255,.16)",
        boxShadow: hot ? "0 6px 20px rgba(180,70,10,.16)" : undefined,
        opacity: state === "todo" ? 0.85 : 1,
      }}
    >
      {state === "done" ? (
        <div style={{ width: 26, height: 26, flex: "none", borderRadius: "50%", background: "var(--ok)", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <Check strokeWidth={2.5} style={{ width: 15, height: 15, color: "#0d1a12" }} />
        </div>
      ) : (
        <div style={{ width: 26, height: 26, flex: "none", borderRadius: "50%", border: hot ? "2px solid var(--accent)" : "2px dashed rgba(255,246,238,.22)" }} />
      )}
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 13.5, fontWeight: 500, color: "var(--fg)" }}>{title}</div>
        <div style={{ fontSize: 12, color: "var(--fg-3)" }}>{desc}</div>
      </div>
      {action}
    </div>
  );
}

export function Onboarding() {
  const { permissions, setPermissions, models, progress, settings, update } = useKoe();
  const [busy, setBusy] = useState(false);

  // Accessibility is granted in System Settings; poll until it flips.
  useEffect(() => {
    if (permissions.accessibility && permissions.microphone === "granted") return;
    const t = setInterval(() => api.permissions().then(setPermissions), 1500);
    return () => clearInterval(t);
  }, [permissions.accessibility, permissions.microphone, setPermissions]);

  const mic = permissions.microphone;
  const firstModel = models.find((m) => m.recommended && m.available) ?? models.find((m) => m.available)!;
  const anyInstalled = models.some((m) => m.installed);
  const p = progress[firstModel.id];
  const downloading = firstModel.downloading || p?.state === "downloading";
  const pct = p ? (p.downloaded / (p.total || 1)) * 100 : (firstModel.downloaded / firstModel.total) * 100;

  const micState: StepState = mic === "granted" ? "done" : "current";
  const axState: StepState = permissions.accessibility ? "done" : mic === "granted" ? "current" : "todo";
  const modelState: StepState = anyInstalled ? "done" : mic === "granted" && permissions.accessibility ? "current" : "todo";

  return (
    <div data-tauri-drag-region style={{ height: "100%", display: "flex", alignItems: "center", justifyContent: "center", padding: 40 }}>
      <div
        style={{
          width: 600,
          borderRadius: 16,
          padding: "30px 40px 32px",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          textAlign: "center",
          background: "rgba(22,17,14,.72)",
          border: ".5px solid rgba(255,255,255,.13)",
          boxShadow: "0 24px 60px rgba(0,0,0,.4), inset 0 1px 0 rgba(255,255,255,.12)",
        }}
      >
        <div style={{ marginBottom: 16 }}>
          <Logo size={66} radius={19} />
        </div>
        <div style={{ fontSize: 26, fontWeight: 600, letterSpacing: -0.4, marginBottom: 6 }}>Speak. Kōe types.</div>
        <div style={{ fontSize: 14, lineHeight: 1.55, color: "var(--fg-3)", maxWidth: 420, marginBottom: 26, textWrap: "pretty" } as React.CSSProperties}>
          Hold one key anywhere on your Mac and your words appear in the app you are already in. Everything runs on this machine.
        </div>
        <div style={{ width: "100%", display: "flex", flexDirection: "column", gap: 10, marginBottom: 24 }}>
          <Step
            state={micState}
            title="Microphone access"
            desc={mic === "granted" ? "Granted" : mic === "denied" ? "Denied — turn it on in System Settings" : "So Kōe can hear you while you hold the key"}
            action={
              mic !== "granted" && (
                <button
                  className="btn primary lg"
                  disabled={busy}
                  onClick={async () => {
                    if (mic === "denied") return api.openPrivacy("microphone");
                    setBusy(true);
                    setPermissions(await api.requestMicrophone());
                    setBusy(false);
                  }}
                >
                  {mic === "denied" ? "Open Settings" : "Allow"}
                </button>
              )
            }
          />
          <Step
            state={axState}
            title="Accessibility access"
            desc={permissions.accessibility ? "Granted" : "Lets Kōe type into other apps"}
            action={
              !permissions.accessibility && (
                <button className={`btn lg${axState === "current" ? " primary" : ""}`} onClick={() => api.openPrivacy("accessibility")}>
                  Open Settings
                </button>
              )
            }
          />
          <Step
            state={modelState}
            title={anyInstalled ? "Model ready" : `Download ${firstModel.name}`}
            desc={
              anyInstalled
                ? `${models.find((m) => m.installed)?.name} is on this Mac`
                : downloading
                  ? `${Math.round(pct)}% · ${formatBytes(p?.downloaded ?? firstModel.downloaded)} of ${formatBytes(firstModel.sizeBytes)}`
                  : `${formatBytes(firstModel.sizeBytes)} · one-time download`
            }
            action={
              !anyInstalled &&
              (downloading ? (
                <div className="progress" style={{ width: 90, flex: "none" }}>
                  <div style={{ width: `${pct}%` }} />
                </div>
              ) : (
                <button className={`btn lg${modelState === "current" ? " primary" : ""}`} onClick={() => api.downloadModel(firstModel.id)}>
                  Download
                </button>
              ))
            }
          />
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "9px 16px", borderRadius: 999, whiteSpace: "nowrap", background: "rgba(255,246,238,.07)", border: ".5px solid rgba(255,255,255,.14)" }}>
            <span style={{ fontSize: 12.5, color: "var(--fg-3)" }}>Your shortcut</span>
            <Keys spec={settings.shortcut} />
            <span style={{ fontSize: 12, color: "var(--fg-4)" }}>change in General</span>
          </div>
          <button className="btn primary lg" onClick={() => update({ onboarded: true })}>
            {anyInstalled && mic === "granted" ? `Start — hold ${shortcutText(settings.shortcut)}` : "Skip for now"}
          </button>
        </div>
      </div>
    </div>
  );
}
