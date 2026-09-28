import { ArrowRight, Plus, X } from "lucide-react";
import { useState } from "react";
import { Header, Row, Section, Slider, Toggle } from "../components/ui";
import { useVak } from "../lib/api";

function eagerLabel(v: number) {
  return v < 0.33 ? "strict" : v > 0.66 ? "eager" : "balanced";
}

export function VocabularyView() {
  const { settings: s, update } = useVak();
  const [word, setWord] = useState("");
  const [heard, setHeard] = useState("");
  const [written, setWritten] = useState("");

  const addWord = () => {
    const w = word.trim();
    if (w && !s.vocabulary.includes(w)) update({ vocabulary: [...s.vocabulary, w] });
    setWord("");
  };
  const addCorrection = () => {
    if (!heard.trim() || !written.trim()) return;
    update({ corrections: [...s.corrections, { heard: heard.trim(), written: written.trim() }] });
    setHeard("");
    setWritten("");
  };

  return (
    <>
      <Header title="Vocabulary" subtitle="Names and jargon Vāk should never mishear" />
      <div className="content-scroll">
        <Section title="Words">
          <div className="group" style={{ padding: 14 }}>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 7 }}>
              {s.vocabulary.map((w) => (
                <span key={w} className="chip">
                  {w}
                  <button aria-label={`Remove ${w}`} onClick={() => update({ vocabulary: s.vocabulary.filter((x) => x !== w) })}>
                    <X strokeWidth={1.75} />
                  </button>
                </span>
              ))}
              <form
                style={{ display: "flex", gap: 6 }}
                onSubmit={(e) => {
                  e.preventDefault();
                  addWord();
                }}
              >
                <input className="field" style={{ width: 160, borderRadius: 999, padding: "5px 12px", fontSize: 12 }} placeholder="Add a word…" value={word} onChange={(e) => setWord(e.target.value)} />
                <button type="submit" className="chip primary" disabled={!word.trim()}>
                  <Plus strokeWidth={1.75} />
                  Add word
                </button>
              </form>
            </div>
            <div style={{ fontSize: 11.5, color: "var(--fg-4)", marginTop: 12 }}>These are passed to the model as context, so it leans towards your spelling.</div>
          </div>
        </Section>

        <Section title="Text cleanup">
          <div className="group">
            <Row title="Remove filler words" desc="Drops um, uh, er and friends.">
              <Toggle on={s.removeFillers} onChange={(removeFillers) => update({ removeFillers })} />
            </Row>
            <Row title="Smart punctuation" desc="Capitalise the first word and close the sentence.">
              <Toggle on={s.smartPunctuation} onChange={(smartPunctuation) => update({ smartPunctuation })} />
            </Row>
            <Row title="Match app writing style" desc="Skip the full stop on one-liners in Slack, Messages and other chat apps.">
              <Toggle on={s.matchAppStyle} onChange={(matchAppStyle) => update({ matchAppStyle })} />
            </Row>
          </div>
        </Section>

        <Section title="Word corrections">
          <div className="group">
            <div style={{ display: "grid", gridTemplateColumns: "1fr 24px 1fr 32px", gap: 10, alignItems: "center", padding: "11px 16px" }} className="eyebrow">
              <span>Heard as</span>
              <span />
              <span>Written as</span>
              <span />
            </div>
            {s.corrections.map((c, i) => (
              <div
                key={i}
                style={{ display: "grid", gridTemplateColumns: "1fr 24px 1fr 32px", gap: 10, alignItems: "center", padding: "9px 16px", borderTop: ".5px solid rgba(255,255,255,.07)" }}
              >
                <span className="mono" style={{ fontSize: 12.5, color: "var(--fg-3)" }}>
                  {c.heard}
                </span>
                <ArrowRight strokeWidth={1.75} style={{ width: 14, height: 14, color: "var(--fg-5)" }} />
                <span className="mono" style={{ fontSize: 12.5, color: "var(--fg)" }}>
                  {c.written}
                </span>
                <button className="ghost-icon" aria-label="Remove correction" onClick={() => update({ corrections: s.corrections.filter((_, j) => j !== i) })}>
                  <X strokeWidth={1.75} style={{ width: 14, height: 14 }} />
                </button>
              </div>
            ))}
            <form
              style={{ display: "grid", gridTemplateColumns: "1fr 24px 1fr 32px", gap: 10, alignItems: "center", padding: "9px 16px", borderTop: ".5px solid rgba(255,255,255,.07)" }}
              onSubmit={(e) => {
                e.preventDefault();
                addCorrection();
              }}
            >
              <input className="field mono" placeholder="vaak / walk" value={heard} onChange={(e) => setHeard(e.target.value)} />
              <ArrowRight strokeWidth={1.75} style={{ width: 14, height: 14, color: "var(--fg-5)" }} />
              <input className="field mono" placeholder="Vāk" value={written} onChange={(e) => setWritten(e.target.value)} />
              <span />
            </form>
            <div style={{ display: "flex", alignItems: "center", gap: 14, padding: "12px 16px", borderTop: ".5px solid rgba(255,255,255,.07)", background: "rgba(255,255,255,.03)" }}>
              <button className="btn primary" onClick={addCorrection} disabled={!heard.trim() || !written.trim()}>
                <Plus strokeWidth={2} />
                Add correction
              </button>
              <span style={{ fontSize: 12, color: "var(--fg-4)", whiteSpace: "nowrap" }}>How eager</span>
              <Slider
                style={{ flex: 1, maxWidth: 260 }}
                value={Math.round(s.correctionEagerness * 100)}
                min={0}
                max={100}
                onChange={(v) => update({ correctionEagerness: v / 100 })}
                format={(v) => eagerLabel(v / 100)}
              />
            </div>
          </div>
          <div style={{ fontSize: 11.5, color: "var(--fg-4)" }}>
            Separate alternatives with “/”. Strict matches case exactly; eager also catches words heard with different spacing.
          </div>
        </Section>
      </div>
    </>
  );
}
