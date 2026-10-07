/**
 * Soudure.
 *
 * ## Ce que cet écran doit rendre visible
 *
 * Un symbole de soudure se lit élément par élément : le symbole élémentaire et
 * son côté, la cote, la discontinuité, les symboles supplémentaires, puis la
 * queue — procédé et niveau de qualité. L'écran construit le symbole avec ces
 * mêmes éléments, et le moteur le restitue en clair, phrase par phrase, avec
 * ce qui cloche.
 *
 * Le niveau de qualité ne vaut que chiffré : « h ≤ 1 mm + 0,15 b, max. 7 mm »
 * n'aide personne devant un cordon. L'écran montre donc la limite telle que la
 * norme l'écrit **et** sa valeur pour la géométrie saisie — ou la grandeur qui
 * manque pour la chiffrer.
 *
 * ## Ce que cet écran ne fait pas
 *
 * Il ne recommande pas de niveau de qualité : c'est l'affaire de la norme
 * d'application ou du concepteur. Aucune option n'est présélectionnée.
 *
 * Aucune liste n'est écrite ici : symboles, procédés et niveaux viennent du
 * moteur, dont les trois sources ne sont pas encore confrontées à leur norme.
 */

import { useCallback, useEffect, useState } from "react";

import { weldingCatalogue, weldingProcess, weldingRead } from "../api";
import { ErrorBox, Findings } from "../components/Findings";
import { Reserves } from "../components/Reserves";
import { Select } from "../components/Select";
import { SourceTag, Sources } from "../components/Sources";
import { StatusBox } from "../components/StatusBox";
import type {
  AppError,
  ProcessReading,
  QualityAssessment,
  Side,
  WeldingCatalogue,
  WeldReading,
  WeldRequest,
} from "../types";
import { LIMIT_STATUS_LABEL } from "../types";

const SIDES: readonly { id: Side; label: string }[] = [
  { id: "arrow", label: "Côté flèche" },
  { id: "other", label: "Côté opposé" },
  { id: "both", label: "Des deux côtés" },
];

const PROCESS_EXAMPLES = ["135", "MAG", "141", "111", "21"];

/** Le texte saisi, ou `null` : le moteur distingue « vide » de « absent ». */
function orNull(text: string): string | null {
  return text.trim() === "" ? null : text;
}

function ProcessReadings({ readings }: { readings: ProcessReading[] }) {
  return (
    <section className="card">
      <header>
        <span className="card-title">Lecture du procédé</span>
        <span className="faint">
          {readings.length} lecture{readings.length > 1 ? "s" : ""}
        </span>
      </header>
      {readings.length > 1 ? (
        <p className="hint">
          Ce nom d'atelier désigne plusieurs numéros. Un symbole porte un numéro : à vous de
          reconnaître le vôtre.
        </p>
      ) : null}
      <div className="table-scroll">
        <table>
          <caption className="visually-hidden">Numéros de procédé et leur hiérarchie</caption>
          <thead>
            <tr>
              <th scope="col">Numéro</th>
              <th scope="col">Procédé</th>
              <th scope="col">Hiérarchie</th>
              <th scope="col">ISO 5817</th>
            </tr>
          </thead>
          <tbody>
            {readings.map((reading) => (
              <tr key={reading.process.number}>
                <th scope="row" className="mono">
                  {reading.process.number}
                </th>
                <td>
                  {reading.process.name}
                  {reading.is_group ? (
                    <span className="faint">
                      {" "}
                      — groupe : {reading.children.map((c) => c.number).join(", ")}
                    </span>
                  ) : null}
                </td>
                <td className="muted">
                  {reading.lineage.map((step) => step.number).join(" › ")}
                </td>
                <td className="muted">
                  {reading.quality_scope.kind === "in_scope"
                    ? "soudage par fusion : visé"
                    : reading.quality_scope.kind === "excluded"
                      ? `non visé — ${reading.quality_scope.reason}`
                      : "MecaTool ne sait pas"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function QualityTable({ quality }: { quality: QualityAssessment }) {
  return (
    <section className="card">
      <header>
        <span className="card-title">
          Niveau de qualité {quality.level.id} — {quality.level.name}
        </span>
        <span className="faint">ISO 5817, sélection</span>
      </header>
      <p className="hint">
        Ce que ce niveau tolère, pour la géométrie saisie. Le choix du niveau revient à la norme
        d'application ou au concepteur : MecaTool ne le recommande pas.
      </p>
      <div className="table-scroll">
        <table>
          <caption className="visually-hidden">Limites des imperfections pour ce niveau</caption>
          <thead>
            <tr>
              <th scope="col">Imperfection</th>
              <th scope="col">Épaisseur</th>
              <th scope="col">Limite de la norme</th>
              <th scope="col">Pour cette soudure</th>
            </tr>
          </thead>
          <tbody>
            {quality.limits.map((limit) => (
              <tr key={`${limit.reference}-${limit.name}`}>
                <th scope="row">
                  {limit.name} <span className="faint mono">({limit.iso6520})</span>
                  {limit.remark ? <div className="faint">{limit.remark}</div> : null}
                </th>
                <td className="muted">{limit.thickness_label}</td>
                <td className="mono">{limit.formula}</td>
                <td>
                  {limit.value_label ? (
                    <strong className="mono">{limit.value_label}</strong>
                  ) : (
                    <span>{LIMIT_STATUS_LABEL[limit.status]}</span>
                  )}
                  {limit.missing ? (
                    <span className="faint"> — indiquez {limit.missing} pour chiffrer</span>
                  ) : null}
                  {limit.rounded ? <span className="faint"> (arrondi par défaut)</span> : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {quality.notes.map((note) => (
        <p key={note} className="faint">
          {note}
        </p>
      ))}
    </section>
  );
}

function Reading({ reading }: { reading: WeldReading }) {
  return (
    <>
      <section className="card">
        <header>
          <span className="card-title">Lecture du symbole</span>
          <SourceTag provenance={reading.provenance} />
        </header>
        <p className="headline">
          <strong>{reading.designation}</strong>
        </p>
        <ol className="stack" style={{ gap: "var(--s-3)", paddingLeft: "1.2rem" }}>
          {reading.sentences.map((sentence) => (
            <li key={sentence}>{sentence}</li>
          ))}
        </ol>
        {reading.equivalent ? (
          <p className="hint">
            {reading.equivalent.label} : relation géométrique d'un cordon à côtés égaux, arrondie
            au centième — pas une valeur de la norme.
          </p>
        ) : null}
        <div style={{ marginTop: "var(--s-6)" }}>
          <Findings findings={reading.findings} />
        </div>
      </section>

      {reading.quality ? <QualityTable quality={reading.quality} /> : null}

      <section className="card">
        <header>
          <span className="card-title">Conclusion</span>
        </header>
        <StatusBox conclusion={reading.conclusion} collapsedWhy={false} />
      </section>

      <Sources provenance={reading.provenance} label="D'où viennent ces chiffres" />
    </>
  );
}

export function Welding() {
  const [catalogue, setCatalogue] = useState<WeldingCatalogue | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  const [processInput, setProcessInput] = useState("135");
  const [readings, setReadings] = useState<ProcessReading[] | null>(null);

  const [symbol, setSymbol] = useState("fillet");
  const [side, setSide] = useState<Side>("arrow");
  const [sizeLetter, setSizeLetter] = useState<string | null>("a");
  const [size, setSize] = useState("5");
  const [count, setCount] = useState("");
  const [length, setLength] = useState("");
  const [spacing, setSpacing] = useState("");
  const [staggered, setStaggered] = useState(false);
  const [supplementary, setSupplementary] = useState<string[]>([]);
  const [allAround, setAllAround] = useState(false);
  const [fieldWeld, setFieldWeld] = useState(false);
  const [process, setProcess] = useState("");
  const [level, setLevel] = useState<string | null>(null);
  const [thickness, setThickness] = useState("");
  const [width, setWidth] = useState("");
  const [reading, setReading] = useState<WeldReading | null>(null);

  useEffect(() => {
    weldingCatalogue()
      .then(setCatalogue)
      .catch((cause) => setError(cause as AppError));
  }, []);

  const chosen = catalogue?.elementary.find((option) => option.id === symbol);

  const readProcess = useCallback(async (text: string) => {
    if (text.trim() === "") return;
    try {
      setReadings(await weldingProcess(text));
      setError(null);
    } catch (cause) {
      setReadings(null);
      setError(cause as AppError);
    }
  }, []);

  const readWeld = useCallback(async () => {
    const request: WeldRequest = {
      symbol,
      side,
      size_letter: sizeLetter,
      size_mm: sizeLetter ? orNull(size) : null,
      count: orNull(count),
      length_mm: orNull(length),
      spacing_mm: orNull(spacing),
      staggered,
      supplementary,
      all_around: allAround,
      field_weld: fieldWeld,
      process: orNull(process),
      level,
      thickness_mm: orNull(thickness),
      width_mm: orNull(width),
    };
    try {
      setReading(await weldingRead(request));
      setError(null);
    } catch (cause) {
      setReading(null);
      setError(cause as AppError);
    }
  }, [
    symbol,
    side,
    sizeLetter,
    size,
    count,
    length,
    spacing,
    staggered,
    supplementary,
    allAround,
    fieldWeld,
    process,
    level,
    thickness,
    width,
  ]);

  const toggle = (id: string) =>
    setSupplementary((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Soudure</h1>
        <p className="lead">
          Ce que dit un symbole de soudure, et ce que tolère le niveau de qualité qu'il porte.
        </p>
      </div>

      {catalogue ? <Reserves warnings={catalogue.warnings} /> : null}

      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          void readProcess(processInput);
        }}
      >
        <header>
          <span className="card-title">Un procédé</span>
        </header>
        <div className="row">
          <div className="grow">
            <label htmlFor="weld-process-lookup">Numéro ISO 4063 ou nom d'atelier</label>
            <input
              id="weld-process-lookup"
              type="text"
              value={processInput}
              placeholder="135"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setProcessInput(event.target.value)}
            />
          </div>
          <div className="field">
            <span className="field-label field-spacer" aria-hidden="true" />
            <button type="submit" className="btn">
              Lire le procédé
            </button>
          </div>
        </div>
        <div className="chips">
          <span className="chips-label">Exemples</span>
          {PROCESS_EXAMPLES.map((example) => (
            <button
              key={example}
              type="button"
              className="chip"
              onClick={() => {
                setProcessInput(example);
                void readProcess(example);
              }}
            >
              {example}
            </button>
          ))}
        </div>
      </form>

      {readings ? <ProcessReadings readings={readings} /> : null}

      {catalogue ? (
        <form
          className="card"
          onSubmit={(event) => {
            event.preventDefault();
            void readWeld();
          }}
        >
          <header>
            <span className="card-title">Un symbole</span>
          </header>

          <div className="stack" style={{ gap: "var(--s-6)" }}>
            <div className="row">
              <div className="grow" style={{ maxWidth: "420px" }}>
                <Select
                  id="weld-symbol"
                  label="Symbole élémentaire"
                  options={catalogue.elementary.map((option) => ({
                    value: option.id,
                    label: option.name,
                  }))}
                  value={symbol}
                  onChange={(value) => {
                    setSymbol(value);
                    const next = catalogue.elementary.find((option) => option.id === value);
                    setSizeLetter(next?.sizes[0] ?? null);
                  }}
                />
              </div>
              <fieldset className="field" style={{ border: 0, padding: 0, margin: 0 }}>
                <legend className="field-label" style={{ padding: 0 }}>
                  Côté
                </legend>
                <div className="segmented">
                  {SIDES.map((option) => (
                    <button
                      key={option.id}
                      type="button"
                      aria-pressed={side === option.id}
                      onClick={() => setSide(option.id)}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              </fieldset>
            </div>

            {chosen && chosen.sizes.length > 0 ? (
              <div className="row">
                <fieldset className="field" style={{ border: 0, padding: 0, margin: 0 }}>
                  <legend className="field-label" style={{ padding: 0 }}>
                    Cote principale
                  </legend>
                  <div className="segmented">
                    {chosen.sizes.map((letter) => (
                      <button
                        key={letter}
                        type="button"
                        aria-pressed={sizeLetter === letter}
                        title={catalogue.sizes.find((s) => s.letter === letter)?.meaning}
                        onClick={() => setSizeLetter(letter)}
                      >
                        {letter} — {catalogue.sizes.find((s) => s.letter === letter)?.name}
                      </button>
                    ))}
                    <button
                      type="button"
                      aria-pressed={sizeLetter === null}
                      onClick={() => setSizeLetter(null)}
                    >
                      Sans cote
                    </button>
                  </div>
                </fieldset>
                {sizeLetter ? (
                  <div style={{ minWidth: "140px" }}>
                    <label htmlFor="weld-size">{sizeLetter} (mm)</label>
                    <input
                      id="weld-size"
                      type="text"
                      value={size}
                      autoComplete="off"
                      onChange={(event) => setSize(event.target.value)}
                    />
                  </div>
                ) : null}
              </div>
            ) : null}

            <div className="row">
              <div style={{ minWidth: "120px" }}>
                <label htmlFor="weld-count">Éléments n</label>
                <input
                  id="weld-count"
                  type="text"
                  value={count}
                  placeholder="—"
                  autoComplete="off"
                  onChange={(event) => setCount(event.target.value)}
                />
              </div>
              <div style={{ minWidth: "120px" }}>
                <label htmlFor="weld-length">Longueur l (mm)</label>
                <input
                  id="weld-length"
                  type="text"
                  value={length}
                  placeholder="—"
                  autoComplete="off"
                  onChange={(event) => setLength(event.target.value)}
                />
              </div>
              <div style={{ minWidth: "120px" }}>
                <label htmlFor="weld-spacing">Intervalle e (mm)</label>
                <input
                  id="weld-spacing"
                  type="text"
                  value={spacing}
                  placeholder="—"
                  autoComplete="off"
                  onChange={(event) => setSpacing(event.target.value)}
                />
              </div>
              <p className="hint grow">
                Soudure discontinue, notée n × l (e). Laissez vide pour une soudure continue.
              </p>
            </div>

            <div className="field">
              <span className="field-label" id="weld-extras-label">
                Symboles supplémentaires et indications
              </span>
              <div
                className="chips"
                role="group"
                aria-labelledby="weld-extras-label"
                style={{ marginTop: 0, paddingTop: 0, borderTop: 0 }}
              >
                {catalogue.supplementary.map((extra) => (
                  <button
                    key={extra.id}
                    type="button"
                    className="chip"
                    aria-pressed={supplementary.includes(extra.id)}
                    title={extra.meaning}
                    onClick={() => toggle(extra.id)}
                  >
                    {supplementary.includes(extra.id) ? "✓ " : ""}
                    {extra.name}
                  </button>
                ))}
                <button
                  type="button"
                  className="chip"
                  aria-pressed={allAround}
                  onClick={() => setAllAround((value) => !value)}
                >
                  {allAround ? "✓ " : ""}sur tout le pourtour
                </button>
                <button
                  type="button"
                  className="chip"
                  aria-pressed={fieldWeld}
                  onClick={() => setFieldWeld((value) => !value)}
                >
                  {fieldWeld ? "✓ " : ""}sur chantier
                </button>
                <button
                  type="button"
                  className="chip"
                  aria-pressed={staggered}
                  onClick={() => setStaggered((value) => !value)}
                >
                  {staggered ? "✓ " : ""}alternée
                </button>
              </div>
            </div>

            <div className="row">
              <div style={{ minWidth: "160px" }}>
                <label htmlFor="weld-process">Procédé (ISO 4063)</label>
                <input
                  id="weld-process"
                  type="text"
                  value={process}
                  placeholder="135"
                  autoComplete="off"
                  spellCheck={false}
                  onChange={(event) => setProcess(event.target.value)}
                />
              </div>
              <fieldset className="field" style={{ border: 0, padding: 0, margin: 0 }}>
                <legend className="field-label" style={{ padding: 0 }}>
                  Niveau de qualité (ISO 5817)
                </legend>
                <div className="segmented">
                  <button
                    type="button"
                    aria-pressed={level === null}
                    onClick={() => setLevel(null)}
                  >
                    Aucun
                  </button>
                  {catalogue.levels.map((option) => (
                    <button
                      key={option.id}
                      type="button"
                      aria-pressed={level === option.id}
                      title={option.meaning}
                      onClick={() => setLevel(option.id)}
                    >
                      {option.id} — {option.name}
                    </button>
                  ))}
                </div>
              </fieldset>
            </div>

            {level ? (
              <div className="row">
                <div style={{ minWidth: "160px" }}>
                  <label htmlFor="weld-thickness">Épaisseur t (mm)</label>
                  <input
                    id="weld-thickness"
                    type="text"
                    value={thickness}
                    placeholder="10"
                    autoComplete="off"
                    onChange={(event) => setThickness(event.target.value)}
                  />
                </div>
                <div style={{ minWidth: "160px" }}>
                  <label htmlFor="weld-width">Largeur b (mm)</label>
                  <input
                    id="weld-width"
                    type="text"
                    value={width}
                    placeholder="—"
                    autoComplete="off"
                    onChange={(event) => setWidth(event.target.value)}
                  />
                </div>
                <p className="hint grow">
                  t, la pièce la plus mince ; b, la largeur de la surépaisseur. Une limite qui en
                  dépend reste écrite sous sa forme littérale tant qu'elle manque.
                </p>
              </div>
            ) : null}

            <div>
              <button type="submit" className="btn">
                Lire le symbole
              </button>
            </div>
          </div>
        </form>
      ) : null}

      {error ? <ErrorBox error={error} /> : null}

      {reading ? <Reading reading={reading} /> : null}
    </div>
  );
}
