import { ClipboardPaste, Copy, Keyboard, Lightbulb, Terminal, type LucideIcon } from "lucide-react";
import { Header, Row, Section, Segmented, Select, Slider, Toggle } from "../components/ui";
import { useKoe, type Settings } from "../lib/api";

const METHODS: { id: Settings["pasteMethod"]; title: string; desc: string; icon: LucideIcon }[] = [
  { id: "paste", title: "Paste (⌘V)", desc: "Fastest and safest. Works in almost every Mac app.", icon: ClipboardPaste },
  { id: "type", title: "Type it out", desc: "Character by character, for apps that refuse a paste.", icon: Keyboard },
  { id: "terminal", title: "Terminal safe (⌃⇧V)", desc: "For terminals and SSH sessions that swallow ⌘V.", icon: Terminal },
  { id: "clipboard", title: "Clipboard only", desc: "Nothing is typed — you paste it yourself.", icon: Copy },
];

export function OutputView() {
  const { settings: s, update } = useKoe();
  return (
    <>
      <Header title="Output" subtitle="How text lands in the app you are in" />
      <div className="content-scroll">
        <Section title="Insertion method">
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 9 }}>
            {METHODS.map(({ id, title, desc, icon: Icon }) => {
              const on = s.pasteMethod === id;
              return (
                <button key={id} className={`choice${on ? " on" : ""}`} onClick={() => update({ pasteMethod: id })}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 5 }}>
                    <Icon strokeWidth={1.75} style={{ width: 15, height: 15, color: on ? "var(--accent-hi)" : "var(--fg-3)" }} />
                    <span style={{ fontSize: 13, fontWeight: 600, color: on ? "var(--fg)" : "var(--fg-2)" }}>{title}</span>
                    {on && (
                      <span className="badge accent" style={{ marginLeft: "auto" }}>
                        In use
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: 12, lineHeight: 1.45, color: "var(--fg-3)" }}>{desc}</div>
                </button>
              );
            })}
          </div>
          <div className="tip">
            <Lightbulb strokeWidth={1.75} />
            <span style={{ flex: 1 }}>Switch to Terminal safe automatically when Terminal, iTerm, Ghostty or Warp is in front.</span>
            <Toggle on={s.autoTerminal} onChange={(autoTerminal) => update({ autoTerminal })} />
          </div>
        </Section>

        <Section title="After pasting">
          <div className="group">
            <Row title="Clipboard" desc="Put your old clipboard back once the text has landed.">
              <Segmented
                value={s.clipboardHandling}
                options={[
                  ["restore", "Restore"],
                  ["keep", "Keep transcript"],
                ]}
                onChange={(clipboardHandling) => update({ clipboardHandling })}
              />
            </Row>
            <Row title="Auto submit" desc="Send the message as soon as the text is in.">
              <Select
                label="Auto submit"
                value={s.autoSubmit}
                options={[
                  ["off", "Off"],
                  ["return", "Return"],
                  ["cmdReturn", "⌘ Return"],
                ]}
                onChange={(autoSubmit) => update({ autoSubmit })}
              />
            </Row>
            <Row title="Trailing space" desc="Handy when you dictate in bursts.">
              <Toggle on={s.trailingSpace} onChange={(trailingSpace) => update({ trailingSpace })} />
            </Row>
            <Row title="Paste delay" desc="Raise it if the wrong text shows up.">
              <Slider
                style={{ width: 230, flex: "none" }}
                value={s.pasteDelayMs}
                min={0}
                max={400}
                step={10}
                onChange={(pasteDelayMs) => update({ pasteDelayMs })}
                format={(v) => `${v} ms`}
              />
            </Row>
          </div>
        </Section>
      </div>
    </>
  );
}
