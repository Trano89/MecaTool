/**
 * Visserie.
 *
 * ## Ce que cet écran doit rendre visible
 *
 * Une désignation comme « M10 » porte bien plus qu'un diamètre : un pas, un
 * profil, une section résistante, un perçage avant taraudage, et trois trous
 * de passage possibles. L'écran les déplie dans cet ordre.
 *
 * Comme pour les roulements, **la frontière entre deux natures de source passe
 * au milieu du résultat** : le diamètre d'un trou de passage et sa classe
 * viennent de l'ISO 273, saisie sans document ouvert ; ses écarts viennent du
 * moteur ISO 286, confronté à la norme. L'écran tient les deux colonnes
 * séparées, chacune avec son étiquette d'état.
 *
 * ## Ce que cet écran ne fait pas
 *
 * Il n'affiche aucun effort admissible : la classe de qualité ne donne que des
 * valeurs nominales, et les minimums garantis ne sont pas embarqués. Il ne
 * choisit pas de foret à la place de l'utilisateur : D − P est une règle
 * d'atelier, présentée comme telle.
 */

import { useCallback, useEffect, useState } from "react";

import { fastenerCatalogue, fastenerRead } from "../api";
import { ErrorBox, Findings } from "../components/Findings";
import { Reserves } from "../components/Reserves";
import { SourceTag, Sources } from "../components/Sources";
import { StatusBox } from "../components/StatusBox";
import { Why } from "../components/Why";
import { deviation, mm, nominal } from "../format";
import type { AppError, FastenerCatalogue, Provenance, ThreadReport } from "../types";

const EXAMPLES: readonly { label: string; why: string }[] = [
  { label: "M10", why: "Pas gros, sous-entendu." },
  { label: "M12 x 1.5", why: "Un pas fin, qui s'écrit toujours." },
  { label: "M8 8.8", why: "Avec sa classe de qualité." },
  { label: "M20 9.8", why: "Une classe qui n'est pas définie à ce diamètre." },
  { label: "M10-6H", why: "Une classe de tolérance de filetage, lue mais non calculée." },
];

/**
 * Les seules références dont l'identifiant commence par l'un des préfixes.
 *
 * C'est un tri d'affichage, pas une règle : chaque étiquette d'état doit porter
 * sur la partie du résultat qu'elle surmonte, et non sur le résultat entier.
 */
function only(provenance: Provenance, ...prefixes: string[]): Provenance {
  return {
    references: provenance.references.filter((reference) =>
      prefixes.some((prefix) => reference.id.startsWith(prefix)),
    ),
  };
}

function Report({ report }: { report: ThreadReport }) {
  const firstTolerance = report.clearance_holes.find((hole) => hole.tolerance)?.tolerance;

  return (
    <>
      <section className="card">
        <header>
          <span className="card-title">1 — Le filetage</span>
          <SourceTag provenance={only(report.provenance, "ISO 261", "ISO 68")} />
        </header>

        <p className="headline">
          <strong className="mono">{report.normalised}</strong>
        </p>

        <dl className="rows">
          <dt>Diamètre nominal d</dt>
          <dd>{nominal(report.designation.d)} mm</dd>
          <dt>Pas P</dt>
          <dd>
            {nominal(report.designation.pitch)} mm —{" "}
            {report.designation.pitch_kind === "coarse"
              ? "pas gros"
              : report.designation.pitch_kind === "fine"
                ? "pas fin"
                : "pas non listé"}
          </dd>
          <dt>Choix</dt>
          <dd>{report.thread.choice === 1 ? "premier choix" : "deuxième choix"}</dd>
          <dt>{report.h.label.split(" = ")[0]}</dt>
          <dd>{mm(report.h.value)} mm</dd>
          {report.dimensions.map((dimension) => (
            <div key={dimension.symbol} style={{ display: "contents" }}>
              <dt>
                {dimension.symbol} — {dimension.name}
              </dt>
              <dd>{mm(dimension.value)} mm</dd>
            </div>
          ))}
          <dt>Section résistante</dt>
          <dd>{report.stress_area_label}</dd>
          <dt>Perçage avant taraudage</dt>
          <dd>
            {report.tap_drill_label} <span className="faint">règle d'atelier D − P</span>
          </dd>
        </dl>

        <p className="hint">
          Les diamètres de base se déduisent du pas par le profil ISO 68-1 ; √3 n'étant pas
          décimal, ils sont arrondis au micromètre, comme dans les tableaux de l'ISO 724.
        </p>

        <div style={{ marginTop: "var(--s-6)" }}>
          <Findings findings={report.findings} />
        </div>
      </section>

      <section className="card">
        <header>
          <span className="card-title">2 — Trous de passage</span>
        </header>
        <p className="hint">
          Le diamètre et la classe viennent de l'ISO 273 ; les écarts de la classe, du moteur
          ISO 286. Ce sont deux natures de source, et chaque colonne porte la sienne.
        </p>
        <div className="table-scroll">
          <table>
            <caption className="visually-hidden">
              Trous de passage des trois séries, et leurs écarts
            </caption>
            <thead>
              <tr>
                <th scope="col">Série</th>
                <th scope="col">
                  Trou <SourceTag provenance={only(report.provenance, "ISO 273")} />
                </th>
                <th scope="col" className="num">
                  ES
                </th>
                <th scope="col" className="num">
                  EI
                </th>
                <th scope="col" className="num">
                  Min.
                </th>
                <th scope="col" className="num">
                  Max.{" "}
                  {firstTolerance ? <SourceTag provenance={firstTolerance.provenance} /> : null}
                </th>
              </tr>
            </thead>
            <tbody>
              {report.clearance_holes.map((hole) => (
                <tr key={hole.series}>
                  <th scope="row">{hole.series_name}</th>
                  <td className="mono">{hole.designation}</td>
                  {hole.tolerance ? (
                    <>
                      <td className="num">
                        {deviation(hole.tolerance.tolerance.deviations.upper)}
                      </td>
                      <td className="num">
                        {deviation(hole.tolerance.tolerance.deviations.lower)}
                      </td>
                      <td className="num">{mm(hole.tolerance.tolerance.limits.min)}</td>
                      <td className="num">{mm(hole.tolerance.tolerance.limits.max)}</td>
                    </>
                  ) : (
                    <td colSpan={4} className="muted">
                      {hole.unavailable}
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {report.strength ? (
        <section className="card">
          <header>
            <span className="card-title">3 — Classe de qualité {report.strength.class.class}</span>
            <SourceTag provenance={only(report.provenance, "ISO 898")} />
          </header>
          <dl className="rows">
            <dt>Résistance nominale Rm</dt>
            <dd>{report.strength.class.tensile_mpa} MPa</dd>
            <dt>Limite d'élasticité nominale</dt>
            <dd>{report.strength.class.yield_mpa} MPa</dd>
            <dt>Écrou associé</dt>
            <dd>
              {report.strength.nut_class !== null
                ? `classe ${report.strength.nut_class} au moins`
                : "aucune classe d'écrou embarquée ne convient"}
            </dd>
          </dl>
          <p className="hint">{report.strength.explanation}</p>
        </section>
      ) : null}

      <section className="card">
        <header>
          <span className="card-title">Conclusion</span>
        </header>
        <StatusBox conclusion={report.conclusion} collapsedWhy={false} />
        {report.clearance_holes[1]?.tolerance ? (
          <div style={{ marginTop: "var(--s-6)" }}>
            <Why
              steps={report.clearance_holes[1].tolerance.steps}
              label="Comment les écarts du trou moyen sont obtenus"
            />
          </div>
        ) : null}
      </section>

      <Sources provenance={report.provenance} label="D'où viennent ces chiffres" />
    </>
  );
}

/** Les filetages embarqués, repliés : c'est une référence, pas l'outil. */
function Threads({ catalogue }: { catalogue: FastenerCatalogue }) {
  return (
    <details className="card">
      <summary>Les filetages embarqués, et leurs pas</summary>
      <div className="stack">
        <div className="table-scroll">
          <table>
            <caption className="visually-hidden">Filetages métriques ISO embarqués</caption>
            <thead>
              <tr>
                <th scope="col">Filetage</th>
                <th scope="col">Choix</th>
                <th scope="col" className="num">
                  Pas gros
                </th>
                <th scope="col">Pas fins</th>
              </tr>
            </thead>
            <tbody>
              {catalogue.threads.map((thread) => (
                <tr key={thread.d}>
                  <th scope="row" className="mono">
                    M{nominal(thread.d)}
                  </th>
                  <td className="muted">{thread.choice === 1 ? "1er" : "2e"}</td>
                  <td className="num">{nominal(thread.coarse)}</td>
                  <td className="muted">{thread.fine.map((pitch) => nominal(pitch)).join(" · ")}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </details>
  );
}

export function Fasteners() {
  const [catalogue, setCatalogue] = useState<FastenerCatalogue | null>(null);
  const [input, setInput] = useState("M10");
  const [report, setReport] = useState<ThreadReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  useEffect(() => {
    fastenerCatalogue()
      .then(setCatalogue)
      .catch((cause) => setError(cause as AppError));
  }, []);

  const read = useCallback(async (text: string) => {
    if (text.trim() === "") return;
    try {
      setReport(await fastenerRead(text));
      setError(null);
    } catch (cause) {
      setReport(null);
      setError(cause as AppError);
    }
  }, []);

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Visserie</h1>
        <p className="lead">
          Ce que porte une désignation de filetage : son pas, son profil, sa section résistante et
          ses trous de passage.
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
          <span className="card-title">Le filetage</span>
        </header>
        <div className="row">
          <div className="grow">
            <label htmlFor="fastener-input">Désignation</label>
            <input
              id="fastener-input"
              type="text"
              value={input}
              placeholder="M10"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setInput(event.target.value)}
            />
            <p className="hint">
              « M10 » pour le pas gros, « M10 x 1.25 » pour un pas fin, suivi au besoin de la
              classe de qualité : « M10 8.8 ».
            </p>
          </div>
          <div className="field">
            <span className="field-label field-spacer" aria-hidden="true" />
            <button type="submit" className="btn">
              Lire le filetage
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

      {report ? <Report report={report} /> : null}

      {catalogue ? <Threads catalogue={catalogue} /> : null}
    </div>
  );
}
