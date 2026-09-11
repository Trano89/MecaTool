/**
 * Chaînes de cotes.
 *
 * Trois choses que cet écran doit rendre évidentes :
 *
 * 1. **La tolérance résultante est la somme de toutes les tolérances**, quel
 *    que soit le sens des maillons. C'est le résultat le plus contre-intuitif
 *    de la cotation, et le plus utile.
 * 2. **Quel maillon pèse le plus.** Resserrer le mauvais maillon ne sert à rien.
 * 3. **Ce que suppose l'estimation statistique.** Elle est plus optimiste que le
 *    pire des cas, et son optimisme repose sur des hypothèses que la géométrie
 *    ne garantit pas. Elle n'est donc jamais calculée par défaut.
 */

import { useCallback, useEffect, useState } from "react";

import { dimensionChain } from "../api";
import { StatusBox } from "../components/StatusBox";
import { Why } from "../components/Why";
import { mm } from "../format";
import type { AppError, ChainReport, ContributionChart } from "../types";

const DEFAULT_CHAIN = "A = 20 ±0.1\nB = 10 ±0.05\n-C = 5 ±0.02";

export function Chain() {
  const [input, setInput] = useState(DEFAULT_CHAIN);
  const [minimum, setMinimum] = useState("");
  const [maximum, setMaximum] = useState("");
  const [statistical, setStatistical] = useState(false);
  const [report, setReport] = useState<ChainReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  const run = useCallback(
    async (chain: string, options: { statistical: boolean; min: string; max: string }) => {
      if (chain.trim() === "") return;
      try {
        setReport(
          await dimensionChain(chain, {
            statistical: options.statistical,
            minimumMm: options.min || undefined,
            maximumMm: options.max || undefined,
          }),
        );
        setError(null);
      } catch (cause) {
        setError(cause as AppError);
        setReport(null);
      }
    },
    [],
  );

  useEffect(() => {
    void run(input, { statistical, min: minimum, max: maximum });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    void run(input, { statistical, min: minimum, max: maximum });
  };

  const toggleStatistical = () => {
    const next = !statistical;
    setStatistical(next);
    void run(input, { statistical: next, min: minimum, max: maximum });
  };

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Chaîne de cotes</h1>
        <p className="lead">
          Ce que devient une dimension qui dépend de plusieurs cotes tolérancées — et quel maillon
          y pèse le plus lourd.
        </p>
      </div>

      <form className="card" onSubmit={submit}>
        <div className="row">
          <div className="field grow">
            <label htmlFor="links">Maillons, un par ligne</label>
            <textarea
              id="links"
              value={input}
              rows={5}
              spellCheck={false}
              onChange={(event) => setInput(event.target.value)}
            />
            <p className="hint">
              « A = 20 ±0.1 » ou « A = 20 +0.1/-0.05 ». Un signe moins devant le repère marque
              un maillon <strong>diminuant</strong> : « -C = 5 ±0.02 ».
            </p>
          </div>

          <div className="field" style={{ minWidth: "170px" }}>
            <label htmlFor="min">Minimum voulu (mm)</label>
            <input
              id="min"
              type="text"
              value={minimum}
              placeholder="24.7"
              autoComplete="off"
              onChange={(event) => setMinimum(event.target.value)}
            />
            <label htmlFor="max" style={{ marginTop: "0.6rem" }}>
              Maximum voulu (mm)
            </label>
            <input
              id="max"
              type="text"
              value={maximum}
              placeholder="25.3"
              autoComplete="off"
              onChange={(event) => setMaximum(event.target.value)}
            />
          </div>

          <div className="field">
            <span className="field-label field-spacer" aria-hidden="true" />
            <button type="submit" className="btn-primary">
              Calculer
            </button>
          </div>
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

      {report ? (
        <>
          <section className="card">
            <header>
              <span className="card-title">Cote résultante</span>
              <strong className="num" style={{ fontSize: "1.05rem" }}>
                {report.designation} mm
              </strong>
            </header>

            <dl className="rows" style={{ maxWidth: "420px" }}>
              <dt>Nominale</dt>
              <dd>{mm(report.analysis.nominal)} mm</dd>
              <dt>Maximale</dt>
              <dd>{mm(report.analysis.limits.max)} mm</dd>
              <dt>Minimale</dt>
              <dd>{mm(report.analysis.limits.min)} mm</dd>
              <dt>Tolérance résultante</dt>
              <dd>{mm(report.analysis.tolerance)} mm</dd>
            </dl>

            <div style={{ marginTop: "0.9rem" }}>
              <Why steps={report.analysis.steps} label="Comment la résultante est calculée" />
            </div>
          </section>

          <section className="card">
            <header>
              <span className="card-title">Contribution de chaque maillon</span>
              {report.analysis.dominant ? (
                <span className="faint">
                  Le maillon {report.analysis.dominant} pèse le plus lourd
                </span>
              ) : null}
            </header>
            <ContributionView chart={report.chart} />
          </section>

          <section className="card">
            <header>
              <span className="card-title">Analyse statistique</span>
              <button
                type="button"
                className="btn-quiet"
                aria-pressed={statistical}
                onClick={toggleStatistical}
              >
                {statistical ? "Estimation affichée" : "Estimer (RSS)"}
              </button>
            </header>

            {report.analysis.statistical ? (
              <>
                <p>
                  <strong>{report.analysis.statistical.method}</strong> —{" "}
                  {report.analysis.statistical.summary}
                </p>
                {/* Les hypothèses ne sont pas repliées : une estimation plus
                    optimiste que le pire des cas ne doit pas circuler sans
                    ce qui la rend possible. */}
                <div className="status caution" role="note" style={{ marginTop: "0.7rem" }}>
                  <span className="status-badge" aria-hidden="true">
                    🟠
                  </span>
                  <div>
                    <div className="status-headline">CE QUE CETTE ESTIMATION SUPPOSE</div>
                    <ul style={{ margin: "0.35rem 0 0", paddingLeft: "1.1rem" }}>
                      {report.analysis.statistical.assumptions.map((assumption) => (
                        <li key={assumption} className="status-detail">
                          {assumption}
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </>
            ) : (
              <p className="muted">
                Le pire des cas ne suppose rien : il vaut quelles que soient les pièces. Une
                estimation statistique donne une tolérance plus large, mais au prix
                d'hypothèses sur la fabrication. Elle n'est donc pas calculée par défaut.
              </p>
            )}
          </section>

          <section className="card">
            <header>
              <span className="card-title">Conclusion</span>
            </header>
            <StatusBox conclusion={report.conclusion} />
          </section>
        </>
      ) : null}
    </div>
  );
}

/**
 * Le graphique de contribution.
 *
 * Comme pour les zones de tolérance, aucune coordonnée n'est calculée ici :
 * elles viennent du moteur, où elles sont testées.
 */
function ContributionView({ chart }: { chart: ContributionChart }) {
  return (
    <figure className="diagram">
      <svg
        viewBox={`0 0 ${chart.width} ${chart.height}`}
        width="100%"
        role="img"
        aria-label={chart.caption}
        preserveAspectRatio="xMidYMin meet"
      >
        {chart.bars.map((bar) => {
          const colour =
            bar.direction === "increasing" ? "var(--hole)" : "var(--shaft)";
          const middle = bar.y + bar.height / 2;
          return (
            <g key={bar.label}>
              <text
                x="0"
                y={middle}
                dy="4"
                fill="var(--ink)"
                fontSize="11"
                fontWeight={bar.dominant ? 700 : 500}
              >
                {bar.direction === "decreasing" ? "− " : ""}
                {bar.label}
              </text>
              <text x="26" y={middle} dy="4" fill="var(--ink-faint)" fontSize="10">
                {bar.designation}
              </text>
              <rect
                x={chart.bar_origin}
                y={bar.y}
                width={Math.max(bar.width, 1)}
                height={bar.height}
                fill={colour}
                opacity={bar.dominant ? 0.85 : 0.45}
                rx="2"
              />
              <text
                x={chart.bar_origin + Math.max(bar.width, 1) + 8}
                y={middle}
                dy="4"
                fill="var(--ink)"
                fontSize="10"
                fontWeight={bar.dominant ? 700 : 400}
              >
                {bar.share_label} — {bar.tolerance_label}
              </text>
            </g>
          );
        })}
      </svg>
      <figcaption>{chart.caption}</figcaption>
    </figure>
  );
}
