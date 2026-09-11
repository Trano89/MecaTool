/**
 * Tolérancement géométrique — ISO 1101.
 *
 * Cet écran ne propose aucune valeur, et c'est délibéré : l'ISO 1101 n'en donne
 * pas. Elle définit un vocabulaire — quatorze caractéristiques, leurs symboles,
 * la forme de la zone que chacune délimite — et la valeur relève du concepteur.
 * Un écran qui suggérerait « 0,05 conviendrait ici » inventerait une règle.
 *
 * Ce qu'il fait, en revanche, c'est lire un cadre de tolérance et dire ce qui
 * cloche : une référence manquante, un ø qui n'a pas de sens, une spécification
 * qu'une autre rend déjà inopérante.
 *
 * La réserve de source est affichée dès l'ouverture, avant toute saisie. Les
 * données viennent d'un recueil qui reproduit la norme, pas de la norme : c'est
 * une information dont l'utilisateur a besoin *avant* de s'appuyer sur ce que
 * l'écran lui dira, pas après.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { geometric, geometricCatalogue } from "../api";
import { StatusBox } from "../components/StatusBox";
import { Why } from "../components/Why";
import type {
  AppError,
  Characteristic,
  DatumRule,
  Finding,
  GeometricCatalogue,
  GroupAnalysis,
  ToleranceFamily,
} from "../types";
import { SEVERITY_BADGE, SEVERITY_CLASS, SEVERITY_LABEL } from "../types";

const DATUM_LABEL: Record<DatumRule, string> = {
  none: "aucune",
  required: "requise",
  optional: "au choix",
};

const EXAMPLE = ["// 0.02 A", "⏥ 0.05", "⟂ 0.03"].join("\n");

/** Un constat, pastille et libellé ensemble. */
function FindingLine({ finding }: { finding: Finding }) {
  return (
    <li className={`finding ${SEVERITY_CLASS[finding.severity]}`}>
      <span aria-hidden="true">{SEVERITY_BADGE[finding.severity]}</span>
      <span>
        <strong>{SEVERITY_LABEL[finding.severity]}</strong> — {finding.message}
      </span>
    </li>
  );
}

/** Le catalogue, replié par défaut : c'est une référence, pas l'outil. */
function Catalogue({ catalogue }: { catalogue: GeometricCatalogue }) {
  const byFamily = useMemo(() => {
    const map = new Map<string, Characteristic[]>();
    for (const characteristic of catalogue.characteristics) {
      const list = map.get(characteristic.class) ?? [];
      list.push(characteristic);
      map.set(characteristic.class, list);
    }
    return map;
  }, [catalogue.characteristics]);

  return (
    <details className="card">
      <summary>Les caractéristiques géométriques, et ce que chacune exige</summary>

      <div className="stack" style={{ marginTop: "0.8rem" }}>
        {catalogue.families.map((family) => (
          <section key={family.id}>
            <h3 style={{ textTransform: "capitalize" }}>{family.heading}</h3>
            <p className="hint">{family.note}</p>

            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">Symbole</th>
                    <th scope="col">Caractéristique</th>
                    <th scope="col">Référence spécifiée</th>
                    <th scope="col">TED explicite</th>
                    <th scope="col">Zone</th>
                  </tr>
                </thead>
                <tbody>
                  {(byFamily.get(family.id) ?? []).map((characteristic) => (
                    <tr key={characteristic.id}>
                      <td className="symbol" aria-hidden="true">
                        {characteristic.symbol}
                      </td>
                      <td>{characteristic.name}</td>
                      <td>{DATUM_LABEL[characteristic.datum]}</td>
                      <td>{characteristic.requires_ted ? "oui" : "non"}</td>
                      <td>
                        {characteristic.zones
                          .map((zone) => zone.feature)
                          .join(" ; ")}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        ))}

        <section>
          <h3>Modificateurs</h3>
          <p className="hint">
            MecaTool les restitue mais ne vérifie pas qu'ils s'appliquent à une caractéristique
            donnée : la source consultée les catalogue sans énoncer de règle d'applicabilité
            complète. Tous ne sont pas définis par l'ISO 1101, et la colonne le dit.
          </p>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Symbole</th>
                  <th scope="col">Signification</th>
                  <th scope="col">Défini par</th>
                </tr>
              </thead>
              <tbody>
                {catalogue.modifiers.map((modifier) => (
                  <tr key={modifier.symbol}>
                    <td className="symbol">{modifier.symbol}</td>
                    <td>{modifier.name}</td>
                    <td>{modifier.defined_by}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </details>
  );
}

export function Geometry() {
  const [input, setInput] = useState(EXAMPLE);
  const [catalogue, setCatalogue] = useState<GeometricCatalogue | null>(null);
  const [analysis, setAnalysis] = useState<GroupAnalysis | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  useEffect(() => {
    geometricCatalogue()
      .then(setCatalogue)
      .catch(() => {
        // Le catalogue n'est qu'une référence dépliable : son absence ne doit
        // pas empêcher de contrôler une spécification.
      });
  }, []);

  // Le nom d'une famille vient du catalogue, jamais d'une table écrite ici :
  // le frontend ne porte aucune règle normative, pas même un libellé. Tant que
  // le catalogue n'est pas chargé, la phrase se réduit au nom de la
  // caractéristique plutôt que d'afficher un libellé inventé.
  const familyName = useCallback(
    (id: ToleranceFamily): string =>
      catalogue?.families.find((family) => family.id === id)?.name ?? "",
    [catalogue],
  );

  const run = useCallback(async (text: string) => {
    const lines = text.split("\n").filter((line) => line.trim() !== "");
    if (lines.length === 0) {
      setAnalysis(null);
      setError(null);
      return;
    }
    try {
      setAnalysis(await geometric(lines));
      setError(null);
    } catch (cause) {
      setError(cause as AppError);
      setAnalysis(null);
    }
  }, []);

  return (
    <div className="stack">
      <div>
        <h1>Tolérancement géométrique</h1>
        <p className="muted" style={{ marginTop: "0.35rem" }}>
          Ce que dit un cadre de tolérance, et ce qui lui manque.
        </p>
      </div>

      {catalogue?.warnings.map((warning) => (
        <div key={warning} className="status caution" role="note">
          <span className="status-badge" aria-hidden="true">
            🟠
          </span>
          <div>
            <div className="status-headline">Origine des données</div>
            <div className="status-detail">{warning}</div>
          </div>
        </div>
      ))}

      <div className="card">
        <h2>Ce que cet écran ne fait pas</h2>
        <p className="hint">
          Il ne propose aucune valeur de tolérance. L'ISO 1101 n'en donne pas : elle définit les
          symboles et la forme des zones, et la valeur se choisit en fonction de la pièce, du
          procédé et du besoin fonctionnel. MecaTool lit ce que vous écrivez, il ne le décide pas à
          votre place.
        </p>
      </div>

      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          void run(input);
        }}
      >
        <label htmlFor="specs">Spécifications posées sur un même élément</label>
        <textarea
          id="specs"
          rows={4}
          value={input}
          spellCheck={false}
          autoComplete="off"
          onChange={(event) => setInput(event.target.value)}
        />
        <p className="hint">
          Une par ligne : <code>⟂ 0.05 A</code>, ou en clair <code>perp 0.05 A</code>. Le ø se
          note <code>ø0.2</code>. Une lettre seule est une référence spécifiée ; un modificateur
          s'écrit entouré ou entre parenthèses, <code>(M)</code>. Elles doivent porter sur le
          <strong> même élément</strong> : c'est ce qui donne son sens au contrôle de recouvrement.
        </p>
        <button type="submit" className="btn">
          Contrôler
        </button>
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

      {analysis ? (
        <>
          <StatusBox conclusion={analysis.conclusion} />

          {analysis.specs.map((spec) => (
            <section key={spec.spec.input} className="card">
              <h2 className="designation">{spec.designation}</h2>
              <p className="muted">
                {spec.characteristic.name}
                {familyName(spec.characteristic.class)
                  ? ` — tolérance de ${familyName(spec.characteristic.class)}`
                  : ""}{" "}
                <span className="faint">(page {spec.characteristic.page} de la source)</span>
              </p>

              {spec.findings.length > 0 ? (
                <ul className="findings">
                  {spec.findings.map((finding) => (
                    <FindingLine key={finding.code} finding={finding} />
                  ))}
                </ul>
              ) : (
                <p className="hint">Rien à signaler sur cette spécification.</p>
              )}

              <Why steps={spec.conclusion.why} open={false} />
            </section>
          ))}

          {analysis.overlaps.length > 0 ? (
            <section className="card">
              <h2>Recouvrements</h2>
              <p className="hint">
                Une tolérance d'orientation borne aussi l'écart de forme ; une tolérance de
                position borne forme et orientation. Une spécification plus large que celle qui la
                borne déjà n'ajoute donc rien au dessin.
              </p>
              <ul className="findings">
                {analysis.overlaps.map((overlap) => (
                  <FindingLine
                    key={`${overlap.wider}→${overlap.narrower}`}
                    finding={overlap.finding}
                  />
                ))}
              </ul>
            </section>
          ) : null}
        </>
      ) : null}

      {catalogue ? <Catalogue catalogue={catalogue} /> : null}
    </div>
  );
}
