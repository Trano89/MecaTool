/**
 * Matières.
 *
 * ## Ce que cet écran doit rendre visible
 *
 * Trois questions, dans l'ordre où elles se posent :
 *
 *  1. **Que dit cette désignation ?** Le moteur la décompose morceau par
 *     morceau, en appliquant les règles de l'EN 10027-1.
 *  2. **Que vaut-elle ?** Pour un acier de construction, la limite d'élasticité
 *     dépend de l'épaisseur : l'épaisseur se choisit par bouton, parmi les
 *     échelons de la table, et non au clavier.
 *  3. **Que devient mon ajustement à chaud ?** Un alésage en aluminium sur un
 *     arbre en acier ne se dilate pas comme lui. C'est là que ce domaine
 *     alimente celui des ajustements.
 *
 * Rien n'est tapé qui puisse se choisir : nuances d'exemple, épaisseurs,
 * familles, écarts de température et classes de tolérance sont des boutons ou
 * des listes, dont les options viennent du moteur.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { materialsCatalogue, materialsRead, thermalFit, toleranceClasses } from "../api";
import { type Choice, ChoiceGroup } from "../components/Choices";
import { ErrorBox, Findings } from "../components/Findings";
import { Reserves } from "../components/Reserves";
import { type Option, Select } from "../components/Select";
import { SourceTag, Sources } from "../components/Sources";
import { StatusBox } from "../components/StatusBox";
import { deviation, nominal } from "../format";
import type {
  AppError,
  ClassCatalogue,
  ClassOption,
  FitKind,
  GradeRow,
  MaterialFamily,
  MaterialsCatalogue,
  SteelReading,
  ThermalFit,
} from "../types";

/** Désignations d'autres catégories, pour qu'on voie ce que le moteur sait lire. */
const OTHER_EXAMPLES = ["C45E", "42CrMo4", "X5CrNi18-10", "HS6-5-2", "1.4301"];

/**
 * Les écarts de température proposés.
 *
 * Ce ne sont pas des valeurs normatives, seulement des points de départ : un
 * champ libre reste disponible à côté pour toute autre valeur.
 */
const DELTA_T: readonly Choice[] = [-40, -20, 20, 50, 80, 100, 150].map((value) => ({
  value: String(value),
  label: `${value > 0 ? "+" : ""}${value} K`,
}));

const KIND_LABEL: Record<FitKind, string> = {
  clearance: "avec jeu",
  transition: "incertain",
  interference: "serré",
};

function bandLabel(row: GradeRow): string {
  return row.band.above === 0
    ? `jusqu'à ${nominal(row.band.to)} mm`
    : `${nominal(row.band.above)} à ${nominal(row.band.to)} mm`;
}

function decimal(value: number, divisor: number): string {
  return String(value / divisor);
}

function classOptions(list: ClassOption[] | undefined): Option[] {
  return (list ?? []).map((option) => ({
    value: option.designation,
    label: option.designation,
    hint: option.grade,
  }));
}

function FamilyProperties({ family }: { family: MaterialFamily }) {
  return (
    <dl className="rows">
      <dt>Module d'élasticité E</dt>
      <dd>{family.e_gpa} GPa</dd>
      <dt>Coefficient de Poisson ν</dt>
      <dd>{decimal(family.poisson_milli, 1000)}</dd>
      <dt>Masse volumique ρ</dt>
      <dd>{family.density} kg/m³</dd>
      <dt>Dilatation α</dt>
      <dd>{decimal(family.alpha_tenths, 10)} µm/(m·K)</dd>
    </dl>
  );
}

function Reading({
  reading,
  catalogue,
}: {
  reading: SteelReading;
  catalogue: MaterialsCatalogue;
}) {
  const [band, setBand] = useState<string | null>("0");
  const [family, setFamily] = useState<string | null>(reading.family?.id ?? null);
  useEffect(() => {
    setBand("0");
    setFamily(reading.family?.id ?? null);
  }, [reading]);

  const structural = reading.structural;
  const chosenRow = structural && band !== null ? structural.rows[Number(band)] : undefined;
  const chosenFamily = catalogue.families.find((option) => option.id === family);

  return (
    <>
      <section className="card">
        <header>
          <span className="card-title">Lecture de la désignation</span>
          <SourceTag provenance={reading.provenance} />
        </header>
        <p className="headline">
          <strong className="mono">{reading.input}</strong>
        </p>
        <p className="muted">{reading.kind_label}</p>
        <div className="table-scroll">
          <table>
            <caption className="visually-hidden">La désignation, morceau par morceau</caption>
            <thead>
              <tr>
                <th scope="col">Morceau</th>
                <th scope="col">Ce qu'il dit</th>
              </tr>
            </thead>
            <tbody>
              {reading.parts.map((part, index) => (
                <tr key={`${part.text}-${index}`}>
                  <th scope="row" className="mono">
                    {part.text}
                  </th>
                  <td>{part.meaning}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div style={{ marginTop: "var(--s-6)" }}>
          <Findings findings={reading.findings} />
        </div>
      </section>

      {structural ? (
        <section className="card">
          <header>
            <span className="card-title">{structural.grade} selon l'épaisseur</span>
            <span className="faint">EN 10025-2</span>
          </header>
          <p className="hint">
            Le nombre de la désignation ne vaut que pour la plus petite épaisseur. Choisissez celle
            de votre pièce.
          </p>
          <ChoiceGroup
            legend="Épaisseur nominale"
            layout="chips"
            options={structural.rows.map((row, index) => ({
              value: String(index),
              label: bandLabel(row),
            }))}
            value={band}
            onChange={setBand}
          />
          {chosenRow ? (
            <dl className="rows" style={{ marginTop: "var(--s-6)" }}>
              <dt>Limite d'élasticité minimale ReH</dt>
              <dd>
                <strong>{chosenRow.yield_mpa} MPa</strong>
              </dd>
              <dt>Résistance à la traction Rm</dt>
              <dd>
                {chosenRow.tensile_min_mpa} à {chosenRow.tensile_max_mpa} MPa
                {chosenRow.band.above === 0 ? (
                  <span className="faint"> (à partir de 3 mm)</span>
                ) : null}
              </dd>
              <dt>Qualités embarquées</dt>
              <dd>{structural.qualities.join(", ")}</dd>
            </dl>
          ) : null}
        </section>
      ) : null}

      <section className="card">
        <header>
          <span className="card-title">Propriétés physiques</span>
          <span className="faint">ordres de grandeur d'une famille</span>
        </header>
        <p className="hint">{reading.family_reason}</p>
        <ChoiceGroup
          legend="Famille"
          layout="chips"
          options={catalogue.families.map((option) => ({ value: option.id, label: option.name }))}
          value={family}
          onChange={setFamily}
        />
        {chosenFamily ? (
          <div style={{ marginTop: "var(--s-6)" }}>
            <FamilyProperties family={chosenFamily} />
          </div>
        ) : null}
      </section>

      <section className="card">
        <header>
          <span className="card-title">Conclusion</span>
        </header>
        <StatusBox conclusion={reading.conclusion} collapsedWhy={false} />
      </section>
    </>
  );
}

function ThermalResult({ result }: { result: ThermalFit }) {
  return (
    <>
      <section className="card">
        <header>
          <span className="card-title">À ΔT = {result.delta_t} K</span>
          <SourceTag provenance={result.provenance} />
        </header>
        <p className="headline">
          <strong className="num">Jeu {result.clearance_shift_label}</strong>
        </p>
        <dl className="rows">
          <dt>Dilatation de l'alésage ({result.hole_family.name})</dt>
          <dd>{deviation(result.hole_growth)}</dd>
          <dt>Dilatation de l'arbre ({result.shaft_family.name})</dt>
          <dd>{deviation(result.shaft_growth)}</dd>
        </dl>
        {result.fit ? (
          <div className="table-scroll" style={{ marginTop: "var(--s-6)" }}>
            <table>
              <caption className="visually-hidden">Jeu à froid et à chaud</caption>
              <thead>
                <tr>
                  <th scope="col">{result.fit.designation}</th>
                  <th scope="col" className="num">
                    Jeu minimal
                  </th>
                  <th scope="col" className="num">
                    Jeu maximal
                  </th>
                  <th scope="col">Nature</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <th scope="row">À la température de référence</th>
                  <td className="num">{deviation(result.fit.cold_min)}</td>
                  <td className="num">{deviation(result.fit.cold_max)}</td>
                  <td>{KIND_LABEL[result.fit.cold_kind]}</td>
                </tr>
                <tr>
                  <th scope="row">À ΔT = {result.delta_t} K</th>
                  <td className="num">{deviation(result.fit.hot_min)}</td>
                  <td className="num">{deviation(result.fit.hot_max)}</td>
                  <td>{KIND_LABEL[result.fit.hot_kind]}</td>
                </tr>
              </tbody>
            </table>
          </div>
        ) : null}
        <div style={{ marginTop: "var(--s-6)" }}>
          <Findings findings={result.findings} />
        </div>
      </section>
      <section className="card">
        <header>
          <span className="card-title">Conclusion</span>
        </header>
        <StatusBox conclusion={result.conclusion} collapsedWhy={false} />
      </section>
      <Sources provenance={result.provenance} label="D'où viennent ces chiffres" />
    </>
  );
}

export function Materials() {
  const [catalogue, setCatalogue] = useState<MaterialsCatalogue | null>(null);
  const [classes, setClasses] = useState<ClassCatalogue | null>(null);
  const [input, setInput] = useState("S355J2");
  const [reading, setReading] = useState<SteelReading | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  const [holeFamily, setHoleFamily] = useState<string | null>("aluminium");
  const [shaftFamily, setShaftFamily] = useState<string | null>("steel");
  const [diameter, setDiameter] = useState("50");
  const [hole, setHole] = useState<string | null>("H7");
  const [shaft, setShaft] = useState<string | null>("p6");
  const [deltaT, setDeltaT] = useState("80");
  const [thermal, setThermal] = useState<ThermalFit | null>(null);

  useEffect(() => {
    materialsCatalogue()
      .then(setCatalogue)
      .catch((cause) => setError(cause as AppError));
    toleranceClasses()
      .then(setClasses)
      .catch(() => {
        // Sans catalogue de classes, le calcul à chaud reste possible sans
        // ajustement : la variation du jeu ne dépend que des familles.
      });
  }, []);

  const read = useCallback(async (text: string) => {
    if (text.trim() === "") return;
    try {
      setReading(await materialsRead(text));
      setError(null);
    } catch (cause) {
      setReading(null);
      setError(cause as AppError);
    }
  }, []);

  const computeThermal = useCallback(async () => {
    if (holeFamily === null || shaftFamily === null) return;
    try {
      setThermal(
        await thermalFit({
          nominalMm: diameter,
          deltaT,
          holeFamily,
          shaftFamily,
          holeClass: hole ?? undefined,
          shaftClass: shaft ?? undefined,
        }),
      );
      setError(null);
    } catch (cause) {
      setThermal(null);
      setError(cause as AppError);
    }
  }, [diameter, deltaT, holeFamily, shaftFamily, hole, shaft]);

  // Les nuances d'aciers de construction embarquées, avec leurs qualités : on
  // les choisit, on ne les tape pas.
  const grades = useMemo<Choice[]>(
    () =>
      (catalogue?.structural_grades ?? []).flatMap((grade) =>
        grade.qualities.map((quality) => ({
          value: `${grade.grade}${quality}`,
          label: `${grade.grade}${quality}`,
        })),
      ),
    [catalogue],
  );
  const families = useMemo<Choice[]>(
    () => (catalogue?.families ?? []).map((option) => ({ value: option.id, label: option.name })),
    [catalogue],
  );

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Matières</h1>
        <p className="lead">
          Ce que dit une désignation d'acier, ce que vaut la nuance, et ce que devient un
          ajustement quand la température change.
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
          <span className="card-title">La nuance</span>
        </header>
        <div className="stack" style={{ gap: "var(--s-6)" }}>
          <ChoiceGroup
            legend="Aciers de construction embarqués"
            layout="chips"
            options={grades}
            value={grades.some((g) => g.value === input) ? input : null}
            onChange={(value) => {
              if (value === null) return;
              setInput(value);
              void read(value);
            }}
          />
          <ChoiceGroup
            legend="Autres catégories"
            layout="chips"
            options={OTHER_EXAMPLES.map((example) => ({ value: example, label: example }))}
            value={OTHER_EXAMPLES.includes(input) ? input : null}
            onChange={(value) => {
              if (value === null) return;
              setInput(value);
              void read(value);
            }}
          />
          <div className="row">
            <div className="grow">
              <label htmlFor="steel-input">Ou une autre désignation</label>
              <input
                id="steel-input"
                type="text"
                value={input}
                placeholder="S355J2"
                autoComplete="off"
                spellCheck={false}
                onChange={(event) => setInput(event.target.value)}
              />
            </div>
            <div className="field">
              <span className="field-label field-spacer" aria-hidden="true" />
              <button type="submit" className="btn">
                Lire la désignation
              </button>
            </div>
          </div>
        </div>
      </form>

      {error ? <ErrorBox error={error} /> : null}

      {reading && catalogue ? <Reading reading={reading} catalogue={catalogue} /> : null}

      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          void computeThermal();
        }}
      >
        <header>
          <span className="card-title">Un ajustement à chaud</span>
          <span className="faint">matières → ajustements</span>
        </header>
        <div className="stack" style={{ gap: "var(--s-6)" }}>
          <ChoiceGroup
            legend="Matière de l'alésage"
            layout="chips"
            options={families}
            value={holeFamily}
            onChange={setHoleFamily}
          />
          <ChoiceGroup
            legend="Matière de l'arbre"
            layout="chips"
            options={families}
            value={shaftFamily}
            onChange={setShaftFamily}
          />
          <ChoiceGroup
            legend="Écart de température"
            layout="chips"
            options={DELTA_T}
            value={DELTA_T.some((choice) => choice.value === deltaT) ? deltaT : null}
            onChange={(value) => {
              if (value !== null) setDeltaT(value);
            }}
          />
          <div className="row">
            <div style={{ minWidth: "120px" }}>
              {/* Le diamètre reste libre, comme sur l'écran des ajustements : une
                  liste de diamètres « usuels » serait une valeur inventée. */}
              <label htmlFor="thermal-diameter">Diamètre (mm)</label>
              <input
                id="thermal-diameter"
                type="text"
                value={diameter}
                autoComplete="off"
                onChange={(event) => setDiameter(event.target.value)}
              />
            </div>
            <div style={{ minWidth: "120px" }}>
              <label htmlFor="thermal-delta">Autre ΔT (K)</label>
              <input
                id="thermal-delta"
                type="text"
                value={deltaT}
                autoComplete="off"
                onChange={(event) => setDeltaT(event.target.value)}
              />
            </div>
            <Select
              id="thermal-hole"
              label="Classe d'alésage"
              options={classOptions(classes?.hole)}
              value={hole}
              onChange={setHole}
              placeholder="H7…"
            />
            <Select
              id="thermal-shaft"
              label="Classe d'arbre"
              options={classOptions(classes?.shaft)}
              value={shaft}
              onChange={setShaft}
              placeholder="p6…"
            />
            <div className="field">
              <span className="field-label field-spacer" aria-hidden="true" />
              <button
                type="button"
                className="btn-quiet"
                onClick={() => {
                  setHole(null);
                  setShaft(null);
                }}
              >
                Sans classe
              </button>
            </div>
          </div>
          <div>
            <button type="submit" className="btn">
              Calculer à chaud
            </button>
          </div>
        </div>
      </form>

      {thermal ? <ThermalResult result={thermal} /> : null}

      {reading && !thermal ? (
        <Sources provenance={reading.provenance} label="D'où viennent ces chiffres" />
      ) : null}
    </div>
  );
}
