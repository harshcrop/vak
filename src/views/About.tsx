import { RotateCcw } from "lucide-react";
import { Header, Keys, Logo } from "../components/ui";
import { useKoe } from "../lib/api";

export function AboutView() {
  const { build, settings, update } = useKoe();
  return (
    <>
      <Header title="About" />
      <div className="content-scroll" style={{ alignItems: "center", justifyContent: "center", gap: 24 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 18 }}>
          <Logo size={74} radius={20} />
          <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
            <span style={{ fontSize: 34, fontWeight: 700, letterSpacing: -0.8, lineHeight: 1 }}>Kōe</span>
            <span className="mono" style={{ fontSize: 11.5, letterSpacing: 0.4, color: "var(--fg-3)" }}>
              speech, on your Mac only
            </span>
          </div>
        </div>
        <div style={{ fontSize: 13, color: "var(--fg-3)", textAlign: "center", lineHeight: 1.7, maxWidth: 420 }}>
          Kōe (声) is Japanese for voice. Hold <Keys spec={settings.shortcut} size={11.5} /> anywhere and your words land in the app you’re in. Audio is transcribed on
          this machine with whisper.cpp and never uploaded.
        </div>
        <div className="mono" style={{ fontSize: 11.5, color: "var(--fg-4)" }}>
          v{build.version} · {build.arch} · {build.os}
        </div>
        <button className="btn" onClick={() => update({ onboarded: false })}>
          <RotateCcw strokeWidth={1.75} />
          Show onboarding again
        </button>
      </div>
    </>
  );
}
