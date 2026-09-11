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
import { CLASSES_UNAVAILABLE, classOptions, useToleranceClasses } from "../components/classes";
import { DiagramView } from "../components/DiagramView";
import { nextRowId, type Row, RowList } from "../components/RowList";
import { Select } from "../components/Select";
import { umLabel } from "../format";
import type { AppError, FitComparison } from "../types";
import { VERDICT_BADGE, VERDICT_LABEL } from "../types";

/** Un ajustement en cours de composition : deux classes, et rien d'autre. */
interface FitRow extends Row {
  hole: string | null;
  shaft: string | null;
}

/**
 * Assemble la saisie à partir des lignes.
 *
 * Mise en forme de chaîne : la dimension, puis les ajustements séparés par des
 * virgules — la forme que le parseur du moteur attend déjà. Les lignes
 * incomplètes sont ignorées plutôt que rendues sous une forme bancale, et
 * **aucune limite de nombre n'est appliquée ici** : le maximum d'ajustements
 * comparables est une règle du moteur, qui la dit lui-même quand elle est
 * franchie. La recopier dans l'interface la ferait exister à deux endroits.
 */
function composeComparison(diameter: string, rows: readonly FitRow[]): string {
  const size = diameter.trim();
  if (size === "") return "";
  const fits = rows
    .map((row) =>
      [row.hole, row.shaft]
        .filter((part): part is string => part !== null && part !== "")
        .join("/"),
    )
    .filter((fit) => fit !== "");
  return fits.length === 0 ? `Ø${size}` : `Ø${size} ${fits.join(", ")}`;
}

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
  // Le composeur, comme sur l'écran de calcul : il écrit dans la saisie, il ne
  // la remplace pas. Son état est un brouillon, pas le reflet du champ.
  const catalogue = useToleranceClasses();
  const [diameter, setDiameter] = useState("20");
  const [rows, setRows] = useState<FitRow[]>([{ id: nextRowId(), hole: null, shaft: null }]);
  const [result, setResult] = useState<FitComparison | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  // Le composeur écrit dans la saisie, il ne la reflète pas. Remonter du texte
  // vers les listes demanderait un parseur dans l'interface, ce que ce projet
  // refuse : le moteur est seul juge de ce qu'une désignation veut dire.
  const compose = (next: { diameter?: string; rows?: FitRow[] }) => {
    const size = next.diameter ?? diameter;
    const lines = next.rows ?? rows;
    if (next.diameter !== undefined) setDiameter(next.diameter);
    if (next.rows !== undefined) setRows(next.rows);
    setInput(composeComparison(size, lines));
  };

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
      <div className="page-header">
        <h1>Comparer des ajustements</h1>
        <p className="lead">
          Plusieurs solutions sur la même dimension, sur une échelle unique. Le dessin porte les
          positions relatives, le tableau porte les valeurs.
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
          <div className="field grow">
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

          <div className="field" style={{ minWidth: "180px" }}>
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

          <div className="field">
            <span className="field-label field-spacer" aria-hidden="true" />
            <button type="submit" className="btn-primary" disabled={busy}>
              {busy ? "Calcul…" : "Comparer"}
            </button>
          </div>
        </div>

        <div className="composer">
          <div className="composer-head">
            <span className="composer-title">Composer la comparaison</span>
            <span className="faint">Écrit dans le champ ci-dessus</span>
          </div>

          <div className="composer-row">
            <div className="field" style={{ minWidth: "120px" }}>
              {/* Comme sur l'écran de calcul : le diamètre reste libre. Une
                  liste de dimensions « usuelles » serait une valeur inventée. */}
              <label htmlFor="compare-diameter">Diamètre (mm)</label>
              <input
                id="compare-diameter"
                type="text"
                value={diameter}
                placeholder="20"
                autoComplete="off"
                spellCheck={false}
                onChange={(event) => compose({ diameter: event.target.value })}
              />
            </div>
          </div>

          <RowList
            legend="Ajustements"
            items={rows}
            onChange={(next) => compose({ rows: next })}
            create={() => ({ id: nextRowId(), hole: null, shaft: null })}
            addLabel="Ajouter un ajustement"
            emptyNote="Aucun ajustement composé. Ajoutez une ligne, ou écrivez directement dans le champ ci-dessus."
            describe={(row, index) =>
              row.hole || row.shaft
                ? `l'ajustement ${[row.hole, row.shaft].filter(Boolean).join("/")}`
                : `l'ajustement ${index + 1}`
            }
            renderRow={(row, index, update) => (
              <>
                <Select
                  id={`compare-hole-${row.id}`}
                  label={`Alésage, ajustement ${index + 1}`}
                  hideLabel
                  options={classOptions(catalogue?.hole)}
                  value={row.hole}
                  onChange={(value) => update({ ...row, hole: value })}
                  placeholder="H7…"
                  emptyNote={CLASSES_UNAVAILABLE}
                />
                <span className="composer-sep" aria-hidden="true">
                  /
                </span>
                <Select
                  id={`compare-shaft-${row.id}`}
                  label={`Arbre, ajustement ${index + 1}`}
                  hideLabel
                  options={classOptions(catalogue?.shaft)}
                  value={row.shaft}
                  onChange={(value) => update({ ...row, shaft: value })}
                  placeholder="g6…"
                  emptyNote={CLASSES_UNAVAILABLE}
                />
              </>
            )}
            hint="Une ligne incomplète est ignorée : elle n'apparaît pas dans la saisie tant que ses deux classes ne sont pas choisies."
          />
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
                      {/* Une désignation n'est pas un nombre : elle se compose
                          en chasse fixe et se cale à gauche, comme son
                          en-tête. La caler à droite avec les jeux ferait
                          croire à une colonne de valeurs. */}
                      <th scope="row" className="mono">
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
              L'ordre est celui de votre saisie. MecaTool ne reclasse pas : « g6, k6, p6 » se lit
              comme une progression, pas comme un classement.
            </p>
          </section>
        </>
      ) : null}
    </div>
  );
}
