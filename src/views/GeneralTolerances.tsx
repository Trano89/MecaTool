/**
 * Tolérances générales — ISO 2768-1.
 *
 * Les quatre classes sont montrées ensemble, pas une à la fois. Choisir une
 * classe suppose de voir ce que les autres donneraient : afficher `m` seule
 * répondrait à la question « que vaut m ? » alors que la vraie question est
 * « laquelle prendre ? ».
 *
 * Une classe que la norme ne définit pas reste dans le tableau, avec la raison.
 * La retirer laisserait croire qu'elle n'existe pas, alors qu'elle n'est
 * simplement pas définie à cette dimension.
 */

import { useCallback, useEffect, useState } from "react";

import { generalTolerances } from "../api";
import { mm, nominal as formatNominal } from "../format";
import type { AppError, ClassComparison, MeasureKind } from "../types";
import { MEASURE_KIND_LABEL } from "../types";

const KINDS: readonly MeasureKind[] = ["linear", "broken_edge", "angular"];

const KIND_HINT: Record<MeasureKind, string> = {
  linear: "La dimension nominale de la cote.",
  broken_edge: "Le rayon de courbure ou la hauteur de chanfrein.",
  angular: "La longueur du côté le plus court de l'angle — pas la valeur de l'angle.",
};

export function GeneralTolerances() {
  const [kind, setKind] = useState<MeasureKind>("linear");
  const [size, setSize] = useState("50");
  const [comparison, setComparison] = useState<ClassComparison | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  const run = useCallback(async (nextKind: MeasureKind, nextSize: string) => {
    if (nextSize.trim() === "") return;
    try {
      setComparison(await generalTolerances(nextKind, nextSize));
      setError(null);
    } catch (cause) {
      setError(cause as AppError);
      setComparison(null);
    }
  }, []);

  useEffect(() => {
    void run(kind, size);
    // Relancé au changement de type : c'est un changement de table, pas un
    // simple filtre d'affichage.
  }, [kind, run, size]);

  const isAngular = kind === "angular";

  return (
    <div className="stack">
      <div>
        <h1>Tolérances générales</h1>
        <p className="muted" style={{ marginTop: "0.35rem" }}>
          Ce que valent les cotes qui ne portent pas de tolérance individuelle, selon la classe
          inscrite au cartouche.
        </p>
      </div>

      <form className="card" onSubmit={(event) => event.preventDefault()}>
        {/* Les deux groupes n'ont pas la même hauteur : les caler en haut aligne
            leurs libellés, ce que l'alignement en bas par défaut ne fait pas. */}
        <div className="row" style={{ alignItems: "flex-start" }}>
          <div>
            <label htmlFor="kind">Type de cote</label>
            <div style={{ display: "flex", gap: "0.3rem" }}>
              {KINDS.map((option) => (
                <button
                  key={option}
                  type="button"
                  id={option === kind ? "kind" : undefined}
                  className="btn-quiet"
                  aria-pressed={option === kind}
                  onClick={() => setKind(option)}
                >
                  {MEASURE_KIND_LABEL[option]}
                </button>
              ))}
            </div>
          </div>

          <div className="grow" style={{ maxWidth: "260px" }}>
            <label htmlFor="size">
              {isAngular ? "Longueur du côté le plus court" : "Dimension nominale"}
            </label>
            <input
              id="size"
              type="text"
              value={size}
              placeholder="50"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setSize(event.target.value)}
            />
            <p className="hint">{KIND_HINT[kind]} En millimètres.</p>
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

      {comparison ? (
        <section className="card">
          <header>
            <span className="card-title">
              {MEASURE_KIND_LABEL[comparison.kind]} de {formatNominal(comparison.nominal)} mm
            </span>
            <span className="faint">ISO 2768-1</span>
          </header>

          <div className="table-scroll">
            <table>
              <caption className="visually-hidden">
                Écarts limites généraux pour les quatre classes de tolérance
              </caption>
              <thead>
                <tr>
                  <th scope="col">Classe</th>
                  <th scope="col">Désignation</th>
                  <th scope="col" className="num">
                    Écart limite
                  </th>
                  {!isAngular ? (
                    <>
                      <th scope="col" className="num">
                        Minimum
                      </th>
                      <th scope="col" className="num">
                        Maximum
                      </th>
                    </>
                  ) : null}
                </tr>
              </thead>
              <tbody>
                {comparison.rows.map((row) => (
                  <tr key={row.symbol}>
                    <th scope="row">
                      <strong className="num">{row.symbol}</strong>{" "}
                      <span className="muted">{row.name}</span>
                    </th>
                    <td className="num">ISO 2768-{row.symbol}</td>

                    {row.deviation_label ? (
                      <>
                        <td className="num">{row.deviation_label}</td>
                        {!isAngular ? (
                          <>
                            <td className="num">
                              {row.limits ? `${mm(row.limits.min)} mm` : "—"}
                            </td>
                            <td className="num">
                              {row.limits ? `${mm(row.limits.max)} mm` : "—"}
                            </td>
                          </>
                        ) : null}
                      </>
                    ) : (
                      <td colSpan={isAngular ? 1 : 3} className="muted">
                        {/* La norme ne définit rien ici : le dire, plutôt que
                            d'afficher un tiret muet. */}
                        Non définie — {row.unavailable}
                      </td>
                    )}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {isAngular ? (
            <p className="faint" style={{ marginTop: "0.8rem" }}>
              L'écart s'applique à l'angle porté par ce côté. Les tolérances angulaires se
              resserrent quand la pièce grandit : le même écart linéaire, rapporté à un bras
              plus long, donne un angle plus petit.
            </p>
          ) : null}
        </section>
      ) : null}

      {comparison ? (
        <section className="card">
          <header>
            <span className="card-title">Norme utilisée</span>
          </header>
          {comparison.provenance.references.map((reference) => (
            <div key={reference.id}>
              <strong>
                {reference.id}:{reference.edition}
              </strong>
              <p className="muted" style={{ marginTop: "0.2rem" }}>
                {reference.title}
              </p>
              <details className="why">
                <summary>Ce que dit la norme, et ce qu'elle ne dit pas</summary>
                <div className="steps">
                  {reference.notes.map((note) => (
                    <p key={note} className="step-expression" style={{ margin: 0 }}>
                      {note}
                    </p>
                  ))}
                </div>
              </details>
            </div>
          ))}
        </section>
      ) : null}
    </div>
  );
}
