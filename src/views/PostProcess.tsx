import { Bookmark, Check, Cloud, Cpu, Info, Play, Plus, Server, type LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { Header, Keys, Section, Toggle } from "../components/ui";
import { api, useKoe, type Settings } from "../lib/api";
import type { ViewProps } from "../windows/MainWindow";

type ProviderId = Settings["postProcess"]["provider"];
const PROVIDERS: { id: ProviderId; title: string; icon: LucideIcon; hint: string }[] = [
  { id: "local", title: "On this Mac", icon: Cpu, hint: "Ollama, LM Studio or mlx_lm.server" },
  { id: "hosted", title: "Hosted model", icon: Cloud, hint: "Bring your own API key" },
  { id: "custom", title: "Custom endpoint", icon: Server, hint: "Any OpenAI-compatible URL" },
];

const SAMPLE = "um so basically we should ship the migration behind a flag on thursday and uh tell support before we flip it";

export function PostProcessView({ toast }: ViewProps) {
  const { settings: s, update, history } = useKoe();
  const pp = s.postProcess;
  const recipe = pp.recipes[pp.activeRecipe] ?? pp.recipes[0];
  const [prompt, setPrompt] = useState(recipe?.prompt ?? "");
  const [output, setOutput] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [took, setTook] = useState<number | null>(null);
  const provider = pp[pp.provider];
  const heard = history[0]?.raw || history[0]?.text || SAMPLE;

  useEffect(() => setPrompt(recipe?.prompt ?? ""), [recipe?.prompt, pp.activeRecipe]);

  const run = async () => {
    setRunning(true);
    setError(null);
    const t0 = performance.now();
    try {
      setOutput(await api.runPostprocess(heard, prompt));
      setTook(Math.round(performance.now() - t0));
    } catch (e) {
      setError(String(e));
      setOutput(null);
    } finally {
      setRunning(false);
    }
  };

  const saveRecipe = () => {
    const recipes = pp.recipes.map((r, i) => (i === pp.activeRecipe ? { ...r, prompt } : r));
    update({ postProcess: { recipes } }).then(() => toast(`Saved “${recipe.name}”`));
  };

  const newRecipe = () => {
    const name = `Recipe ${pp.recipes.length + 1}`;
    const recipes = [...pp.recipes, { name, prompt: "Describe what to do with the transcript.\n${transcript}" }];
    update({ postProcess: { recipes, activeRecipe: recipes.length - 1 } });
  };

  return (
    <>
      <Header title="Post-processing" subtitle="An LLM pass before the paste">
        <span className="badge soft">Alpha</span>
        <span style={{ fontSize: 12, color: "var(--fg-3)", display: "flex", alignItems: "center", gap: 6 }}>
          Runs on <Keys spec={s.ppShortcut} size={11.5} />
        </span>
        <Toggle on={pp.enabled} onChange={(enabled) => update({ postProcess: { enabled } })} label="Enable post-processing" />
      </Header>
      <div style={{ flex: 1, minHeight: 0, display: "flex" }}>
        <div style={{ width: 260, flex: "none", padding: 16, borderRight: ".5px solid rgba(255,255,255,.09)", display: "flex", flexDirection: "column", gap: 8, overflowY: "auto" }}>
          <span className="eyebrow">Provider</span>
          {PROVIDERS.map(({ id, title, icon: Icon, hint }) => {
            const on = pp.provider === id;
            return (
              <button key={id} className={`choice${on ? " on" : ""}`} style={{ display: "flex", alignItems: "center", gap: 10, padding: "10px 12px" }} onClick={() => update({ postProcess: { provider: id } })}>
                <Icon strokeWidth={1.75} style={{ width: 15, height: 15, flex: "none", color: on ? "var(--accent-hi)" : "var(--fg-3)" }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: on ? 600 : 500 }}>{title}</div>
                  <div style={{ fontSize: 11, color: "var(--fg-4)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{on ? pp[id].model : hint}</div>
                </div>
                {on && <Check strokeWidth={2.25} style={{ width: 14, height: 14, color: "var(--accent-hi)" }} />}
              </button>
            );
          })}
          <div style={{ display: "flex", flexDirection: "column", gap: 7, marginTop: 6 }}>
            <label className="eyebrow">Base URL</label>
            <input
              className="field mono"
              style={{ fontSize: 11.5 }}
              defaultValue={provider.baseUrl}
              key={`${pp.provider}-url`}
              onBlur={(e) => update({ postProcess: { [pp.provider]: { baseUrl: e.target.value.trim() } } })}
            />
            <label className="eyebrow">Model</label>
            <input
              className="field mono"
              style={{ fontSize: 11.5 }}
              defaultValue={provider.model}
              key={`${pp.provider}-model`}
              onBlur={(e) => update({ postProcess: { [pp.provider]: { model: e.target.value.trim() } } })}
            />
            {pp.provider !== "local" && (
              <>
                <label className="eyebrow">API key</label>
                <input
                  className="field mono"
                  type="password"
                  style={{ fontSize: 11.5 }}
                  placeholder="sk-…"
                  defaultValue={provider.apiKey}
                  key={`${pp.provider}-key`}
                  onBlur={(e) => update({ postProcess: { [pp.provider]: { apiKey: e.target.value.trim() } } })}
                />
              </>
            )}
          </div>
          <div style={{ marginTop: 6, padding: "11px 12px", borderRadius: 11, background: "rgba(255,255,255,.04)", border: ".5px solid var(--line)" }}>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              {pp.provider === "local" ? "Cost" : "Privacy"}
            </div>
            <div style={{ fontSize: 11.5, lineHeight: 1.45, color: "var(--fg-3)" }}>
              {pp.provider === "local"
                ? "Local runs add a few hundred ms and nothing leaves the machine."
                : "The transcript is sent to this endpoint. Audio never is."}
            </div>
          </div>
        </div>

        <div className="content-scroll" style={{ flex: 1, padding: 16, gap: 14 }}>
          <Section
            title="Recipe"
            right={
              <>
                {pp.recipes.map((r, i) => (
                  <button key={r.name + i} className={`pill-tab${i === pp.activeRecipe ? " on" : ""}`} onClick={() => update({ postProcess: { activeRecipe: i } })}>
                    {r.name}
                  </button>
                ))}
                <button className="pill-tab" onClick={newRecipe} aria-label="New recipe">
                  <Plus strokeWidth={2} style={{ width: 11, height: 11, verticalAlign: -1 }} />
                </button>
              </>
            }
          >
            <textarea className="field" rows={5} value={prompt} onChange={(e) => setPrompt(e.target.value)} spellCheck={false} />
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Info strokeWidth={1.75} style={{ width: 13, height: 13, color: "var(--fg-5)" }} />
              <span style={{ fontSize: 11.5, color: "var(--fg-4)" }}>
                <span className="mono" style={{ color: "var(--accent-text)" }}>
                  {"${transcript}"}
                </span>{" "}
                is replaced with the raw text before the model sees it.
              </span>
            </div>
          </Section>

          <Section title="Preview">
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
              <div style={{ padding: "12px 14px", borderRadius: 12, background: "rgba(255,255,255,.04)", border: ".5px solid rgba(255,255,255,.11)" }}>
                <div className="eyebrow" style={{ marginBottom: 7 }}>
                  Heard {history[0] ? "· last take" : "· sample"}
                </div>
                <div className="selectable" style={{ fontSize: 12.5, lineHeight: 1.6, color: "var(--fg-3)" }}>
                  {heard}
                </div>
              </div>
              <div className="card hot" style={{ padding: "12px 14px", borderRadius: 12 }}>
                <div className="eyebrow" style={{ marginBottom: 7, color: "var(--accent-text)" }}>
                  Pasted
                </div>
                <div className="selectable" style={{ fontSize: 12.5, lineHeight: 1.6, color: output ? "var(--fg-2)" : "var(--fg-4)", whiteSpace: "pre-wrap" }}>
                  {running ? "Running…" : output ?? "Run the recipe to see what would be pasted."}
                </div>
              </div>
            </div>
            {error && <div className="error-text">{error}</div>}
          </Section>

          <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: "auto" }}>
            <button className="btn primary lg" onClick={run} disabled={running}>
              <Play strokeWidth={2} />
              Run on {history[0] ? "last transcript" : "sample"}
            </button>
            <button className="btn lg" onClick={saveRecipe} disabled={prompt === recipe?.prompt}>
              <Bookmark strokeWidth={1.75} />
              Save recipe
            </button>
            {took != null && (
              <span className="mono" style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--fg-4)" }}>
                +{took} ms
              </span>
            )}
          </div>
        </div>
      </div>
    </>
  );
}
