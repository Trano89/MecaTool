/**
 * Comparaison d'ajustements.
 *
 * Le dessin porte les **positions relatives** — c'est ce qu'on ne peut pas lire
 * dans un tableau. Le tableau porte les **valeurs** — c'est ce qu'on ne peut pas
 * lire sur un dessin. Les deux se complètent, aucun ne double l'autre.
 *
 * L'ordre est celui de la saisie. Reclasser ferait perdre l'intention : quand on
 * écrit « g6, k6, p6 », on veut voir la progression du jeu vers le serrage.
 */

import { useCallback, useEffect, useState } from "react";

import { compare } from "../api";
import { DiagramView } from "../components/DiagramView";
import { umLabel } from "../format";
import type { AppError, FitComparison } from "../types";
import { VERDICT_BADGE, VERDICT_LABEL } from "../types";

const EXAMPLES: ReadonlyArray<{ label: string; input: string; why: string }> = [
  {
    label: "Ø20 H7/g6, H7/h6, H7/k6, H7/p6",
    input: "Ø20 H7/g6, H7/h6, H7/k6, H7/p6",
    why: "du jeu au serrage, sur la même dimension",
  },
  {
    label: "Ø50 H7/f7, H8/f7, H7/g6",
    input: "Ø50 H7/f7, H8/f7, H7/g6",
    why: "trois façons d'obtenir du jeu",
  },
  {
    label: "Ø10 H6/h5, H7/h6, H8/h7",
    input: "Ø10 H6/h5, H7/h6, H8/h7",
    why: "le même ajustement à trois degrés de précision",
  },
];

export function Compare() {
  const [input, setInput] = useState("Ø20 H7/g6, H7/h6, H7/k6, H7/p6");
  const [clearance, setClearance] = useState("");
  const [result, setResult] = useState<FitComparison | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  const run = useCallback(async (text: string, window: string) => {
    if (text.trim() === "") return;
    setBusy(true);
    try {
      setResult(await compare(text, window || undefined));
      setError(null);
    } catch (cause) {
      setError(cause as AppError);
      setResult(null);
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void run(input, clearance);
    // Lancé une fois à l'ouverture, avec l'exemple par défaut : un écran vide
    // n'apprendrait rien sur ce que fait cet outil.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const hasRequirement = result?.requirement != null;

  return (
    <div className="stack">
      <div>
        <h1>Comparer des ajustements</h1>
        <p className="muted" style={{ marginTop: "0.35rem" }}>
          Plusieurs solutions sur la même dimension, sur une échelle unique.
        </p>
      </div>

      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          void run(input, clearance);
        }}
      >
        <div className="row">
          <div className="grow">
            <label htmlFor="fits">Ajustements à comparer</label>
            <input
              id="fits"
              type="text"
              value={input}
              placeholder="Ø20 H7/g6, H7/k6"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setInput(event.target.value)}
            />
            <p className="hint">
              La dimension, puis les ajustements séparés par des virgules. Six au maximum :
              au-delà, les zones deviennent trop étroites pour être lues.
            </p>
          </div>

          <div style={{ minWidth: "170px" }}>
            <label htmlFor="window">Jeu voulu (facultatif)</label>
            <input
              id="window"
              type="text"
              value={clearance}
              placeholder="5..50"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setClearance(event.target.value)}
            />
            <p className="hint">Ajoute un verdict par ligne.</p>
          </div>

          <button type="submit" className="btn-primary" disabled={busy}>
            {busy ? "Calcul…" : "Comparer"}
          </button>
        </div>

        <div className="row" style={{ marginTop: "0.9rem", gap: "0.4rem" }}>
          <span className="faint" style={{ alignSelf: "center" }}>
            Exemples :
          </span>
          {EXAMPLES.map((example) => (
            <button
              key={example.label}
              type="button"
              className="btn-quiet"
              title={example.why}
              onClick={() => {
                setInput(example.input);
                void run(example.input, clearance);
              }}
            >
              {example.label}
            </button>
          ))}
        </div>
      </form>

      {error ? (
        <div className="status incompatible" role="alert">
          <span className="status-badge" aria-hidden="true">
            🔴
          </span>
          <div>
            <div className="status-headline">{error.message}</div>
            {error.hint ? <div className="status-detail">{error.hint}</div> : null}
          </div>
        </div>
      ) : null}

      {result ? (
        <>
          <section className="card">
            <header>
              <span className="card-title">Zones de tolérance</span>
              <span className="faint">{result.summary}</span>
            </header>
            <DiagramView diagram={result.diagram} />
          </section>

          <section className="card">
            <header>
              <span className="card-title">Valeurs</span>
            </header>
            <div className="table-scroll">
              <table>
                <caption className="visually-hidden">
                  Jeux et dispersions des ajustements comparés
                </caption>
                <thead>
                  <tr>
                    <th scope="col">Ajustement</th>
                    <th scope="col" className="num">
                      Jeu min
                    </th>
                    <th scope="col" className="num">
                      Jeu max
                    </th>
                    <th scope="col" className="num">
                      Dispersion
                    </th>
                    <th scope="col">Type</th>
                    {hasRequirement ? <th scope="col">Verdict</th> : null}
                  </tr>
                </thead>
                <tbody>
                  {result.entries.map((entry) => (
                    <tr key={entry.designation}>
                      <th scope="row" className="num" style={{ fontWeight: 600 }}>
                        {entry.designation}
                      </th>
                      <td className="num">{umLabel(entry.fit.min_clearance)}</td>
                      <td className="num">{umLabel(entry.fit.max_clearance)}</td>
                      <td className="num">{umLabel(entry.span)}</td>
                      <td>{entry.classification}</td>
                      {hasRequirement ? (
                        <td>
                          {entry.verification ? (
                            <>
                              <span aria-hidden="true">
                                {VERDICT_BADGE[entry.verification.verdict]}{" "}
                              </span>
                              {VERDICT_LABEL[entry.verification.verdict]}
                            </>
                          ) : (
                            "—"
                          )}
                        </td>
                      ) : null}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <p className="faint" style={{ marginTop: "0.8rem" }}>
              L'ordre est celui de votre saisie. Mecatol ne reclasse pas : « g6, k6, p6 » se lit
              comme une progression, pas comme un classement.
            </p>
          </section>
        </>
      ) : null}
    </div>
  );
}
