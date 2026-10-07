/**
 * États de surface.
 *
 * ## Ce que cet écran doit rendre visible
 *
 * Deux questions, dans les deux sens : **que dit cette indication**, et
 * **quel procédé l'obtient**. Le graphique des procédés répond à la seconde
 * avant même toute saisie — c'est la question que l'utilisateur se pose le
 * plus souvent : « quelle rugosité pour ce procédé ? ».
 *
 * ## Ce que cet écran ne fait pas
 *
 * Il ne convertit pas Rz en Ra : le moteur refuse, et l'écran affiche son
 * refus au lieu d'une liste vide qu'on lirait comme « aucun procédé ne
 * convient ». Il ne devine pas non plus de règle d'acceptation ni de longueur
 * de base : elles diffèrent entre les deux générations de normes.
 *
 * Les trois sources du domaine ne sont pas encore confrontées à leur norme. La
 * réserve s'affiche avant la saisie, comme pour la géométrie et les roulements.
 */

import { useCallback, useEffect, useState } from "react";

import { surfaceCatalogue, surfaceRead } from "../api";
import { ErrorBox, Findings } from "../components/Findings";
import { Reserves } from "../components/Reserves";
import { SourceTag, Sources } from "../components/Sources";
import { StatusBox } from "../components/StatusBox";
import { umFine } from "../format";
import type {
  AppError,
  ProcessFit,
  Reach,
  RoughnessChart,
  SurfaceAnalysis,
  SurfaceCatalogue,
} from "../types";

const EXAMPLES: readonly { label: string; why: string }[] = [
  { label: "Ra 0.8", why: "Une exigence simple : quels procédés l'atteignent ?" },
  { label: "MRR Ra 1.6 ⊥ rectifié", why: "Symbole, stries et procédé cités ensemble." },
  { label: "N7", why: "Une classe N d'un plan ancien, lue en Ra." },
  { label: "NMR Ra 25 moulage en sable", why: "Une surface brute, sans enlèvement de matière." },
  { label: "Rz 6.3", why: "Un Rz se lit, mais ne se convertit pas en Ra." },
];

/** L'ordre de lecture des procédés : du plus utile au moins utile. */
const REACH_ORDER: readonly Reach[] = ["usual", "possible", "finer", "out_of_reach"];

const REACH_TITLE: Record<Reach, string> = {
  usual: "Atteinte d'ordinaire",
  possible: "Avec des soins particuliers",
  finer: "Plus fins que nécessaire",
  out_of_reach: "Hors d'atteinte",
};

/** Teinte d'une plage selon la situation du procédé. Toujours doublée d'un libellé. */
const REACH_COLOUR: Record<Reach, string> = {
  usual: "var(--ok-solid)",
  possible: "var(--warn-solid)",
  finer: "var(--info-solid)",
  out_of_reach: "var(--ink-faint)",
};

/** Hauteur réservée aux en-têtes de colonnes, au-dessus des lignes du moteur. */
const HEADER = 34;

/**
 * Les plages de rugosité par procédé.
 *
 * Toutes les coordonnées viennent du moteur, où elles sont testées. L'interface
 * ne fait que décaler le tout sous la ligne d'en-tête.
 */
function RoughnessChartView({ chart }: { chart: RoughnessChart }) {
  return (
    <figure className="diagram">
      <svg
        viewBox={`0 0 ${chart.width} ${chart.height + HEADER}`}
        width="100%"
        role="img"
        aria-label={chart.caption}
        preserveAspectRatio="xMidYMin meet"
      >
        {chart.columns.map((column, index) => (
          <g key={column.grade}>
            <rect
              x={column.x}
              y={0}
              width={column.width}
              height={chart.height + HEADER}
              fill={index % 2 === 0 ? "var(--surface-sunken)" : "transparent"}
              opacity="0.6"
            />
            <text
              x={column.x + column.width / 2}
              y={13}
              textAnchor="middle"
              fontSize="10"
              fontWeight="600"
              fill="var(--ink)"
            >
              {column.grade}
            </text>
            <text
              x={column.x + column.width / 2}
              y={26}
              textAnchor="middle"
              fontSize="9"
              fill="var(--ink-faint)"
            >
              {column.ra_label}
            </text>
          </g>
        ))}

        <g transform={`translate(0 ${HEADER})`}>
          {chart.rows.map((row) => {
            const colour = row.reach ? REACH_COLOUR[row.reach] : "var(--accent-solid)";
            const middle = row.y + row.height / 2;
            return (
              <g key={row.id} opacity={row.excluded ? 0.35 : 1}>
                <text x="0" y={middle} dy="4" fontSize="11" fill="var(--ink)">
                  {row.name}
                </text>
                <rect
                  x={row.possible.x + 1}
                  y={row.y + row.height / 4}
                  width={row.possible.width - 2}
                  height={row.height / 2}
                  fill={colour}
                  opacity="0.25"
                  rx="2"
                />
                <rect
                  x={row.usual.x + 1}
                  y={row.y}
                  width={row.usual.width - 2}
                  height={row.height}
                  fill={colour}
                  opacity="0.85"
                  rx="2"
                />
              </g>
            );
          })}
        </g>

        {chart.marker ? (
          <g>
            <line
              x1={chart.marker.x}
              x2={chart.marker.x}
              y1={HEADER - 4}
              y2={chart.height + HEADER}
              stroke="var(--bad-solid)"
              strokeWidth="2"
              strokeDasharray="4 3"
            />
          </g>
        ) : null}
      </svg>
      <figcaption>
        {chart.marker ? <strong>{chart.marker.label} : trait rouge. </strong> : null}
        {chart.caption}
      </figcaption>
    </figure>
  );
}

/** Les procédés d'une même situation, avec leurs plages. */
function ReachGroup({ reach, fits }: { reach: Reach; fits: ProcessFit[] }) {
  if (fits.length === 0) return null;
  return (
    <section>
      <h3>
        {REACH_TITLE[reach]} <span className="faint">({fits.length})</span>
      </h3>
      <div className="table-scroll">
        <table>
          <caption className="visually-hidden">{REACH_TITLE[reach]}</caption>
          <thead>
            <tr>
              <th scope="col">Procédé</th>
              <th scope="col">Famille</th>
              {/* L'unité reste dans les cellules : un en-tête passé en
                  capitales ferait de « µm » un « MM ». */}
              <th scope="col" className="num">
                Ra usuel
              </th>
              <th scope="col" className="num">
                Ra atteignable
              </th>
            </tr>
          </thead>
          <tbody>
            {fits.map((fit) => (
              <tr key={fit.process.id}>
                <th scope="row">{fit.process.name}</th>
                <td className="muted">
                  {fit.process.removal ? "enlèvement de matière" : "mise en forme"}
                </td>
                <td className="num">
                  {umFine(fit.process.usual.finest)} à {umFine(fit.process.usual.coarsest)} µm
                </td>
                <td className="num">
                  {umFine(fit.process.possible.finest)} à {umFine(fit.process.possible.coarsest)} µm
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function Result({ analysis }: { analysis: SurfaceAnalysis }) {
  const kept = analysis.processes.filter((fit) => fit.excluded === null);
  const excluded = analysis.processes.filter((fit) => fit.excluded !== null);

  return (
    <>
      <section className="card">
        <header>
          <span className="card-title">Lecture de l'indication</span>
          <SourceTag provenance={analysis.provenance} />
        </header>

        <p className="headline">
          <strong className="mono">{analysis.designation}</strong>
        </p>

        <dl className="rows wrap">
          <dt>Symbole</dt>
          <dd>
            {analysis.requirement
              ? `${analysis.requirement.code} — ${analysis.requirement.meaning}`
              : "non précisé : tout procédé est envisagé"}
          </dd>
          <dt>Paramètre</dt>
          <dd>
            <strong>{analysis.parameter.symbol}</strong> — {analysis.parameter.name}.{" "}
            {analysis.parameter.definition}
          </dd>
          <dt>Limite</dt>
          <dd>
            {analysis.indication.limit === "upper" ? "supérieure" : "inférieure"} :{" "}
            {analysis.parameter.symbol} {umFine(analysis.indication.value)} µm
          </dd>
          <dt>Classe N</dt>
          <dd>
            {analysis.grade
              ? `${analysis.grade.grade} (ISO 1302:1992, retirée)`
              : "aucune classe N ne correspond"}
          </dd>
          {analysis.lay ? (
            <>
              <dt>Stries</dt>
              <dd>
                <span className="mono">{analysis.lay.symbol}</span> — {analysis.lay.meaning}
              </dd>
            </>
          ) : null}
          {analysis.stated_process ? (
            <>
              <dt>Procédé indiqué</dt>
              <dd>
                {analysis.stated_process.process.name} : {analysis.stated_process.reach_label}
              </dd>
            </>
          ) : null}
        </dl>

        <div style={{ marginTop: "var(--s-6)" }}>
          <Findings findings={analysis.findings} />
        </div>
      </section>

      <section className="card">
        <header>
          <span className="card-title">Quels procédés l'obtiennent</span>
          <span className="faint">ordres de grandeur, non normatifs</span>
        </header>

        {analysis.process_note ? (
          // Le refus du moteur, en toutes lettres. Une liste vide se lirait
          // « aucun procédé ne convient », ce qui serait faux.
          <div className="status insufficient-data" role="note">
            <span className="status-badge" aria-hidden="true">
              🔵
            </span>
            <div>
              <div className="status-headline">Pas de confrontation au tableau</div>
              <div className="status-detail">{analysis.process_note}</div>
            </div>
          </div>
        ) : (
          <div className="stack">
            <RoughnessChartView chart={analysis.chart} />
            {REACH_ORDER.map((reach) => (
              <ReachGroup
                key={reach}
                reach={reach}
                fits={kept.filter((fit) => fit.reach === reach)}
              />
            ))}
            {excluded.length > 0 ? (
              <p className="hint">
                Écartés par le symbole : {excluded.map((fit) => fit.process.name).join(", ")}.{" "}
                {excluded[0]!.excluded}
              </p>
            ) : null}
          </div>
        )}
      </section>

      <section className="card">
        <header>
          <span className="card-title">Conclusion</span>
        </header>
        <StatusBox conclusion={analysis.conclusion} collapsedWhy={false} />
      </section>

      <Sources provenance={analysis.provenance} label="D'où viennent ces chiffres" />
    </>
  );
}

/** Le vocabulaire, replié : c'est une référence, pas l'outil. */
function Vocabulary({ catalogue }: { catalogue: SurfaceCatalogue }) {
  return (
    <details className="card">
      <summary>Le vocabulaire : symboles, stries, paramètres, classes N</summary>
      <div className="stack">
        <section>
          <h3>Les trois variantes du symbole</h3>
          <dl className="rows wrap">
            {catalogue.symbols.map((symbol) => (
              <div key={symbol.id} style={{ display: "contents" }}>
                <dt className="mono">{symbol.code}</dt>
                <dd>
                  <strong>{symbol.name}</strong> — {symbol.meaning}
                </dd>
              </div>
            ))}
          </dl>
        </section>
        <section>
          <h3>Sens des stries</h3>
          <dl className="rows wrap">
            {catalogue.lays.map((lay) => (
              <div key={lay.symbol} style={{ display: "contents" }}>
                <dt className="mono">{lay.symbol}</dt>
                <dd>{lay.meaning}</dd>
              </div>
            ))}
          </dl>
        </section>
        <section>
          <h3>Paramètres d'amplitude</h3>
          <dl className="rows wrap">
            {catalogue.parameters.map((parameter) => (
              <div key={parameter.symbol} style={{ display: "contents" }}>
                <dt className="mono">{parameter.symbol}</dt>
                <dd>
                  <strong>{parameter.name}</strong> — {parameter.definition}
                  {parameter.note ? <span className="faint"> {parameter.note}</span> : null}
                </dd>
              </div>
            ))}
          </dl>
        </section>
        <section>
          <h3>Classes N (ISO 1302:1992, retirée)</h3>
          <p className="hint">
            À lire sur les plans existants, jamais à écrire sur un plan neuf : on y inscrit la
            valeur Ra.
          </p>
          <div className="table-scroll">
            <table>
              <caption className="visually-hidden">Classes N et valeur Ra</caption>
              <thead>
                <tr>
                  {catalogue.grades.map((grade) => (
                    <th key={grade.grade} scope="col" className="num">
                      {grade.grade}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                <tr>
                  {catalogue.grades.map((grade) => (
                    <td key={grade.grade} className="num">
                      {umFine(grade.ra)}
                    </td>
                  ))}
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </details>
  );
}

export function Surface() {
  const [catalogue, setCatalogue] = useState<SurfaceCatalogue | null>(null);
  const [input, setInput] = useState("Ra 0.8");
  const [analysis, setAnalysis] = useState<SurfaceAnalysis | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  useEffect(() => {
    surfaceCatalogue()
      .then(setCatalogue)
      .catch((cause) => setError(cause as AppError));
  }, []);

  const read = useCallback(async (text: string) => {
    if (text.trim() === "") return;
    try {
      setAnalysis(await surfaceRead(text));
      setError(null);
    } catch (cause) {
      setAnalysis(null);
      setError(cause as AppError);
    }
  }, []);

  return (
    <div className="stack">
      <div className="page-header">
        <h1>États de surface</h1>
        <p className="lead">
          Ce que dit une indication de rugosité, et quels procédés l'obtiennent d'ordinaire.
        </p>
      </div>

      {catalogue ? <Reserves warnings={catalogue.warnings} /> : null}

      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          void read(input);
        }}
      >
        <header>
          <span className="card-title">L'indication</span>
        </header>

        <div className="row">
          <div className="grow">
            <label htmlFor="surface-input">Exigence de rugosité</label>
            <input
              id="surface-input"
              type="text"
              value={input}
              placeholder="MRR Ra 0.8 ⊥"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setInput(event.target.value)}
            />
            <p className="hint">
              Valeur en micromètres. APA, MRR ou NMR pour la variante du symbole, « max » pour la
              règle du maximum, un symbole de stries (= ⊥ X M C R P) et un procédé si besoin.
            </p>
          </div>
          <div className="field">
            <span className="field-label field-spacer" aria-hidden="true" />
            <button type="submit" className="btn">
              Lire l'indication
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
              title={example.why}
              onClick={() => {
                setInput(example.label);
                void read(example.label);
              }}
            >
              {example.label}
            </button>
          ))}
        </div>
      </form>

      {error ? <ErrorBox error={error} /> : null}

      {analysis ? (
        <Result analysis={analysis} />
      ) : catalogue ? (
        <section className="card">
          <header>
            <span className="card-title">Quelle rugosité pour ce procédé ?</span>
            <span className="faint">ordres de grandeur, non normatifs</span>
          </header>
          <RoughnessChartView chart={catalogue.chart} />
        </section>
      ) : null}

      {catalogue ? <Vocabulary catalogue={catalogue} /> : null}
    </div>
  );
}
