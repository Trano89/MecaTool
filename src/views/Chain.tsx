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

import { dimensionChain, toleranceClasses } from "../api";
import { ChoiceGroup } from "../components/Choices";
import { nextRowId, type Row, RowList } from "../components/RowList";
import { type Option, Select } from "../components/Select";
import { StatusBox } from "../components/StatusBox";
import { Why } from "../components/Why";
import { mm } from "../format";
import type { AppError, ChainReport, ClassCatalogue, ContributionChart } from "../types";

const DEFAULT_CHAIN = "A = 20 ±0.1\nB = 10 ±0.05\n-C = 5 ±0.02";

/**
 * D'où viennent les écarts d'un maillon.
 *
 * Une tolérance ne se tape pas quand une norme la donne : une cote sans
 * tolérance relève de l'ISO 2768, une cote tolérancée par classe de l'ISO 286.
 * Le moteur lit alors les écarts lui-même. La saisie d'écarts reste possible,
 * en dernier recours, pour une tolérance que le plan porte en clair.
 */
type ToleranceMode = "iso2768" | "iso286" | "deviations";

const MODES = [
  { value: "iso2768", label: "Tolérance générale" },
  { value: "iso286", label: "Classe ISO 286" },
  { value: "deviations", label: "Écarts du plan" },
] as const;

/** Un maillon en cours de composition. */
interface LinkRow extends Row {
  label: string;
  nominal: string;
  mode: ToleranceMode;
  /** Écarts écrits, pour le mode « écarts du plan ». */
  tolerance: string;
  /** Classe ISO 286 : `h11`, `H7`. */
  cls: string | null;
  /** Classe générale : `ISO 2768-m`. */
  general: string | null;
  decreasing: boolean;
}

/** Les écarts tels que le moteur les lit, selon le mode retenu. */
function toleranceText(row: LinkRow): string {
  switch (row.mode) {
    case "iso2768":
      return row.general ?? "";
    case "iso286":
      return row.cls ?? "";
    case "deviations":
      return row.tolerance.trim();
  }
}

/** Les trois maillons de la chaîne d'exemple, déjà composés. */
const DEFAULT_ROWS: LinkRow[] = [
  { label: "A", nominal: "20", tolerance: "±0.1", decreasing: false },
  { label: "B", nominal: "10", tolerance: "±0.05", decreasing: false },
  { label: "C", nominal: "5", tolerance: "±0.02", decreasing: true },
].map((row) => ({
  ...row,
  id: nextRowId(),
  mode: "deviations",
  cls: null,
  general: null,
}));

/**
 * Le sens d'un maillon.
 *
 * Deux valeurs seulement, mais une liste plutôt qu'une case à cocher : « le
 * signe moins devant le repère marque un maillon diminuant » est une convention
 * d'écriture qu'il faut connaître. Deux libellés en toutes lettres la rendent
 * inutile à connaître.
 */
const DIRECTIONS: ReadonlyArray<Option> = [
  { value: "increasing", label: "Croissant", hint: "s'ajoute" },
  { value: "decreasing", label: "Diminuant", hint: "se retranche" },
];

/**
 * Assemble la saisie à partir des maillons.
 *
 * Mise en forme de chaîne, rien d'autre : la grammaire est celle que le parseur
 * du moteur attend déjà. Un maillon sans nominal est ignoré plutôt que rendu
 * sous une forme que le moteur refuserait.
 */
function composeChain(rows: readonly LinkRow[]): string {
  return rows
    .filter((row) => row.nominal.trim() !== "")
    .map((row) => {
      const sign = row.decreasing ? "-" : "";
      const name = row.label.trim();
      const head = name === "" ? "" : `${sign}${name} = `;
      // Sans repère, le signe se porte sur le nominal : le moteur nomme alors
      // le maillon lui-même, A, B, C…
      const nominal = name === "" ? `${sign}${row.nominal.trim()}` : row.nominal.trim();
      const tolerance = toleranceText(row);
      return tolerance === "" ? `${head}${nominal}` : `${head}${nominal} ${tolerance}`;
    })
    .join("\n");
}

export function Chain() {
  const [input, setInput] = useState(DEFAULT_CHAIN);
  // Le composeur est ouvert par defaut, deja garni de la chaine d'exemple : on
  // choisit une tolerance plutot que de la taper. La saisie directe reste
  // au-dessus, pour qui connait la grammaire.
  const [composing, setComposing] = useState(true);
  const [rows, setRows] = useState<LinkRow[]>(DEFAULT_ROWS);
  const [classes, setClasses] = useState<ClassCatalogue | null>(null);
  const [minimum, setMinimum] = useState("");
  const [maximum, setMaximum] = useState("");
  const [statistical, setStatistical] = useState(false);
  const [report, setReport] = useState<ChainReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  // Le composeur écrit dans la saisie, il ne la reflète pas : remonter du texte
  // vers les lignes demanderait un parseur dans l'interface.
  //
  // Mais il n'écrit **rien tant qu'il n'a rien à dire**. Une ligne qu'on vient
  // d'ajouter n'a pas encore de cote nominale : sans cette garde, ouvrir le
  // composeur et cliquer « Ajouter » effacerait la chaîne déjà saisie, au
  // moment précis où l'utilisateur croit l'enrichir.
  const compose = (next: LinkRow[]) => {
    setRows(next);
    const composed = composeChain(next);
    if (composed !== "") setInput(composed);
  };

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
    toleranceClasses()
      .then(setClasses)
      .catch(() => {
        // Sans catalogue, seuls les écarts du plan restent proposés : le
        // composeur ne propose jamais une classe que le moteur n'a pas donnée.
      });
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
              « A = 20 ±0.1 », « A = 20 h11 » ou « A = 20 ISO 2768-m ». Un signe moins devant le
              repère marque un maillon <strong>diminuant</strong> : « -C = 5 ±0.02 ».
            </p>
          </div>

          <div className="field" style={{ minWidth: "170px" }}>
            <span className="field-label field-spacer" aria-hidden="true" />
            <button
              type="button"
              className="btn-quiet"
              aria-expanded={composing}
              onClick={() => setComposing((open) => !open)}
            >
              {composing ? "Masquer le composeur" : "Composer les maillons"}
            </button>
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

        {composing ? (
          <div className="composer">
            <div className="composer-head">
              <span className="composer-title">Composer les maillons</span>
              <span className="faint">Écrit dans le champ ci-dessus</span>
            </div>

            <RowList
              legend="Maillons"
              items={rows}
              onChange={compose}
              create={() => ({
                id: nextRowId(),
                label: "",
                nominal: "",
                mode: (classes ? "iso2768" : "deviations") as ToleranceMode,
                tolerance: "±0.1",
                cls: null,
                general: classes?.general.find((option) => option.symbol === "m")?.designation ?? null,
                decreasing: false,
              })}
              addLabel="Ajouter un maillon"
              emptyNote="Aucun maillon composé. Ajoutez une ligne, ou écrivez directement dans le champ ci-dessus."
              describe={(row, index) =>
                row.label.trim() === ""
                  ? `le maillon ${index + 1}`
                  : `le maillon ${row.label.trim()}`
              }
              renderRow={(row, index, update) => (
                <>
                  <div className="field" style={{ maxWidth: "90px" }}>
                    <label htmlFor={`link-label-${row.id}`} className="visually-hidden">
                      Repère du maillon {index + 1}
                    </label>
                    <input
                      id={`link-label-${row.id}`}
                      type="text"
                      value={row.label}
                      placeholder="A"
                      autoComplete="off"
                      spellCheck={false}
                      onChange={(event) => update({ ...row, label: event.target.value })}
                    />
                  </div>

                  <div className="field" style={{ maxWidth: "120px" }}>
                    <label htmlFor={`link-nominal-${row.id}`} className="visually-hidden">
                      Cote nominale du maillon {index + 1}
                    </label>
                    <input
                      id={`link-nominal-${row.id}`}
                      type="text"
                      value={row.nominal}
                      placeholder="20"
                      autoComplete="off"
                      spellCheck={false}
                      onChange={(event) => update({ ...row, nominal: event.target.value })}
                    />
                  </div>

                  <ChoiceGroup
                    legend={`Origine de la tolérance du maillon ${index + 1}`}
                    hideLegend
                    options={MODES.filter((mode) => classes !== null || mode.value === "deviations")}
                    value={row.mode}
                    onChange={(value) => {
                      if (value !== null) update({ ...row, mode: value as ToleranceMode });
                    }}
                  />

                  {row.mode === "iso2768" && classes ? (
                    <ChoiceGroup
                      legend={`Classe générale du maillon ${index + 1}`}
                      hideLegend
                      options={classes.general.map((option) => ({
                        value: option.designation,
                        label: option.symbol,
                        title: `${option.designation} — ${option.name}`,
                      }))}
                      value={row.general}
                      onChange={(value) => update({ ...row, general: value })}
                    />
                  ) : null}

                  {row.mode === "iso286" && classes ? (
                    <Select
                      id={`link-class-${row.id}`}
                      label={`Classe du maillon ${index + 1}`}
                      hideLabel
                      options={[...classes.shaft, ...classes.hole].map((option) => ({
                        value: option.designation,
                        label: option.designation,
                        hint: option.grade,
                      }))}
                      value={row.cls}
                      onChange={(value) => update({ ...row, cls: value })}
                      placeholder="h11…"
                    />
                  ) : null}

                  {row.mode === "deviations" ? (
                    <div className="field" style={{ maxWidth: "150px" }}>
                      <label htmlFor={`link-tol-${row.id}`} className="visually-hidden">
                        Tolérance du maillon {index + 1}
                      </label>
                      <input
                        id={`link-tol-${row.id}`}
                        type="text"
                        value={row.tolerance}
                        placeholder="±0.1"
                        autoComplete="off"
                        spellCheck={false}
                        onChange={(event) => update({ ...row, tolerance: event.target.value })}
                      />
                    </div>
                  ) : null}

                  <Select
                    id={`link-dir-${row.id}`}
                    label={`Sens du maillon ${index + 1}`}
                    hideLabel
                    options={DIRECTIONS}
                    value={row.decreasing ? "decreasing" : "increasing"}
                    onChange={(value) => update({ ...row, decreasing: value === "decreasing" })}
                  />
                </>
              )}
              hint="Repère, cote nominale, origine de la tolérance, sens. La tolérance se choisit : générale (ISO 2768) ou par classe (ISO 286), et le moteur lit les écarts lui-même. Un maillon sans cote nominale est ignoré."
            />
          </div>
        ) : null}
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
