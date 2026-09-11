/**
 * L'écran de calcul.
 *
 * Un seul champ de désignation, un champ de jeu facultatif. C'est la saisie qui
 * détermine ce que MecaTool fait — calculer, vérifier ou chercher — et seul le
 * moteur sait la lire. Faire choisir un mode à l'utilisateur avant qu'il ait
 * tapé quoi que ce soit lui demanderait de savoir d'avance ce qu'il cherche.
 */

import { useCallback, useEffect, useRef, useState } from "react";

import { analyse, rescaleDiagram } from "../api";
import { DiagramView } from "../components/DiagramView";
import { FeatureBlock } from "../components/FeatureBlock";
import { SolutionsTable } from "../components/SolutionsTable";
import { StatusBox } from "../components/StatusBox";
import { Why } from "../components/Why";
import { umLabel } from "../format";
import type { AppError, Diagram, Report } from "../types";

export interface Query {
  input: string;
  clearance: string;
}

interface Props {
  query: Query;
  onQueryChange: (query: Query) => void;
}

const EXAMPLES: ReadonlyArray<{ label: string; query: Query; why: string }> = [
  { label: "Ø10 H7/g6", query: { input: "Ø10 H7/g6", clearance: "" }, why: "un ajustement courant" },
  {
    label: "Ø20 H7/g6, jeu 5–50 µm",
    query: { input: "Ø20 H7/g6", clearance: "5..50" },
    why: "vérifier une exigence",
  },
  { label: "Ø20, jeu 10–30 µm", query: { input: "Ø20", clearance: "10..30" }, why: "chercher une solution" },
  { label: "Ø20 H7/p6", query: { input: "Ø20 H7/p6", clearance: "" }, why: "un serrage" },
];

export function Calculate({ query, onQueryChange }: Props) {
  const [report, setReport] = useState<Report | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  const [trueToScale, setTrueToScale] = useState(false);
  const [diagram, setDiagram] = useState<Diagram | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const run = useCallback(
    async (next: Query) => {
      if (next.input.trim() === "") {
        setReport(null);
        setError(null);
        setDiagram(null);
        return;
      }
      setBusy(true);
      try {
        const result = await analyse(next.input, next.clearance || undefined, trueToScale);
        setReport(result);
        setDiagram(result.kind === "fit" ? result.diagram : null);
        setError(null);
      } catch (cause) {
        setError(cause as AppError);
        setReport(null);
        setDiagram(null);
      } finally {
        setBusy(false);
      }
    },
    [trueToScale],
  );

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    void run(query);
  };

  const pick = (example: Query) => {
    onQueryChange(example);
    void run(example);
  };

  const toggleScale = async () => {
    const next = !trueToScale;
    setTrueToScale(next);
    if (report?.kind === "fit") {
      try {
        setDiagram(await rescaleDiagram(query.input, next));
      } catch {
        // Le diagramme précédent reste affiché : il n'est pas faux, seulement
        // dans l'autre mode. Mieux vaut cela qu'un écran vide.
      }
    }
  };

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Calculer un ajustement</h1>
        <p className="lead">
          Une seule saisie. C'est elle qui détermine si MecaTool calcule, vérifie ou cherche.
        </p>
      </div>

      <form onSubmit={submit} className="card">
        <div className="row">
          <div className="field grow">
            <label htmlFor="designation">Désignation</label>
            <input
              id="designation"
              ref={inputRef}
              type="text"
              value={query.input}
              placeholder="Ø20 H7/g6"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => onQueryChange({ ...query, input: event.target.value })}
            />
            <p className="hint">
              Un ajustement (« Ø20 H7/g6 »), un élément seul (« 20 H7 ») ou une dimension nue
              (« Ø20 ») pour chercher des solutions.
            </p>
          </div>

          <div className="field" style={{ minWidth: "190px" }}>
            <label htmlFor="clearance">Jeu recherché (facultatif)</label>
            <input
              id="clearance"
              type="text"
              value={query.clearance}
              placeholder="10..30"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => onQueryChange({ ...query, clearance: event.target.value })}
            />
            <p className="hint">En µm, ou avec unité : « 0.01..0.03 mm ».</p>
          </div>

          <div className="field">
            <span className="field-label field-spacer" aria-hidden="true" />
            <button type="submit" className="btn-primary" disabled={busy}>
              {busy ? "Calcul…" : "Calculer"}
            </button>
          </div>
        </div>

        <div className="chips">
          <span className="chips-label">Exemples</span>
          {EXAMPLES.map((example) => (
            <button
              key={example.label}
              type="button"
              className="chip"
              onClick={() => pick(example.query)}
              title={example.why}
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

      {report?.kind === "fit" ? (
        <>
          <div className="grid-2">
            <FeatureBlock tolerance={report.analysis.fit.hole} />
            <FeatureBlock tolerance={report.analysis.fit.shaft} />
          </div>

          <section className="card">
            <header>
              <span className="card-title">Ajustement</span>
              <strong>{report.classification}</strong>
            </header>

            <dl className="rows" style={{ maxWidth: "420px" }}>
              <dt>Jeu minimum</dt>
              <dd>{umLabel(report.analysis.fit.min_clearance)}</dd>
              <dt>Jeu maximum</dt>
              <dd>{umLabel(report.analysis.fit.max_clearance)}</dd>
              <dt>Dispersion à tenir</dt>
              <dd>
                {umLabel(
                  report.analysis.fit.max_clearance - report.analysis.fit.min_clearance,
                )}
              </dd>
            </dl>

            <div style={{ marginTop: "0.9rem" }}>
              <Why steps={report.analysis.fit_steps} label="Comment ce jeu est calculé" />
            </div>
          </section>

          <section className="card">
            <header>
              <span className="card-title">Zones de tolérance</span>
              <button
                type="button"
                className="btn-quiet"
                aria-pressed={trueToScale}
                onClick={() => void toggleScale()}
              >
                {trueToScale ? "Échelle fidèle" : "Écarts amplifiés"}
              </button>
            </header>
            {diagram ? <DiagramView diagram={diagram} /> : null}
          </section>

          <section className="card">
            <header>
              <span className="card-title">Conclusion</span>
            </header>
            <StatusBox conclusion={report.verification?.conclusion ?? report.conclusion} />
          </section>

          <details className="why">
            <summary>Détail du calcul, élément par élément</summary>
            <div className="steps">
              <Why steps={report.analysis.hole_steps} label="Alésage" open />
              <Why steps={report.analysis.shaft_steps} label="Arbre" open />
            </div>
          </details>
        </>
      ) : null}

      {report?.kind === "feature" ? (
        <>
          <div className="grid-2">
            <FeatureBlock tolerance={report.analysis.tolerance} />
          </div>
          <section className="card">
            <header>
              <span className="card-title">Détail du calcul</span>
            </header>
            <Why steps={report.analysis.steps} open />
          </section>
        </>
      ) : null}

      {report?.kind === "search" ? (
        <>
          {report.diagnosis ? (
            <div className="status caution" role="status">
              <span className="status-badge" aria-hidden="true">
                🟠
              </span>
              <div>
                <div className="status-headline">AUCUNE SOLUTION EXACTE</div>
                <div className="status-detail">{report.diagnosis}</div>
              </div>
            </div>
          ) : null}

          <section className="card">
            <header>
              <span className="card-title">Solutions</span>
              <span className="faint">
                {report.result.solutions.length} retenues sur {report.result.examined} combinaisons
                examinées
              </span>
            </header>
            <SolutionsTable
              solutions={report.result.solutions}
              onSelect={(designation) => pick({ input: designation, clearance: query.clearance })}
            />
          </section>

          <section className="card">
            <header>
              <span className="card-title">Périmètre de la recherche</span>
            </header>
            {/* Une recherche infructueuse doit se lire « aucune solution ici »,
                jamais « aucune solution n'existe ». */}
            {report.result.notes.map((note) => (
              <p key={note} className="muted">
                {note}
              </p>
            ))}
          </section>
        </>
      ) : null}
    </div>
  );
}
