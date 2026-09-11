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
import { nextRowId, type Row, RowList } from "../components/RowList";
import { Select } from "../components/Select";
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

/** Une ligne de la saisie, telle que le composeur et la liste la manipulent. */
interface SpecRow extends Row {
  text: string;
}

/**
 * Découpe la saisie en lignes.
 *
 * Découper sur les retours à la ligne n'est **pas** lire une spécification :
 * c'est exactement ce que `run()` fait déjà avant d'appeler le moteur. Le
 * contenu d'une ligne reste opaque à l'interface, qui ne l'interprète jamais.
 *
 * L'identité d'une ligne est son rang. Chaque ligne étant une chaîne entière et
 * contrôlée, un rang qui glisse au retrait d'une ligne ne fait pas glisser le
 * contenu : seule la position du curseur peut sauter, ce qui est sans effet
 * après un clic sur « Retirer ».
 */
function toRows(input: string): SpecRow[] {
  return input.split("\n").map((text, index) => ({ id: `line-${index}`, text }));
}

/**
 * Assemble une ligne à partir de ce qui a été choisi.
 *
 * Mise en forme de chaîne, et rien d'autre : l'ordre suit celui qu'emploie la
 * source pour un cadre de tolérance — le symbole, la zone, puis les références.
 * C'est le moteur qui dira si la ligne a un sens.
 */
function composeSpec(parts: {
  symbol: string | null;
  diameter: boolean;
  value: string;
  modifier: string | null;
  datums: string;
}): string {
  const zone = `${parts.diameter ? "ø" : ""}${parts.value.trim()}`;
  return [parts.symbol, zone, parts.modifier, parts.datums.trim()]
    .filter((piece): piece is string => typeof piece === "string" && piece !== "")
    .join(" ");
}

export function Geometry() {
  const [input, setInput] = useState(EXAMPLE);
  const [catalogue, setCatalogue] = useState<GeometricCatalogue | null>(null);
  const [analysis, setAnalysis] = useState<GroupAnalysis | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  // Le composeur. Son état n'est qu'un brouillon : il ne devient une ligne que
  // lorsqu'on l'ajoute, et la saisie reste la seule chose que le moteur lit.
  const [symbol, setSymbol] = useState<string | null>(null);
  const [value, setValue] = useState("");
  const [diameter, setDiameter] = useState(false);
  const [modifier, setModifier] = useState<string | null>(null);
  const [datums, setDatums] = useState("");

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

  const draft = composeSpec({ symbol, diameter, value, modifier, datums });

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
      <div className="page-header">
        <h1>Tolérancement géométrique</h1>
        {/* Attention en modifiant ce chapeau : un test vérifie que la phrase
            « ne propose aucune valeur » n'apparaît qu'une fois dans l'écran,
            sur le panneau qui l'explique. */}
        <p className="lead">
          Ce que dit un cadre de tolérance, et ce qui lui manque. L'ISO 1101 définit un
          vocabulaire, pas des chiffres.
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

        {/*
          Les mêmes lignes, une par une. C'est ici qu'on en retire une sans avoir
          à sélectionner du texte au clavier — et c'est le même contenu : la
          zone de saisie reste la seule chose que le moteur lit.
        */}
        <div style={{ marginTop: "var(--s-6)" }}>
          <RowList<SpecRow>
            legend="Lignes"
            items={toRows(input)}
            onChange={(rows) => setInput(rows.map((row) => row.text).join("\n"))}
            create={() => ({ id: nextRowId(), text: "" })}
            addLabel="Ajouter une ligne vide"
            emptyNote="Aucune spécification. Composez-en une ci-dessous, ou tapez-la directement."
            describe={(row, index) =>
              row.text.trim() === "" ? `la ligne ${index + 1}` : `« ${row.text.trim()} »`
            }
            renderRow={(row, _index, update) => (
              <div className="field grow">
                <label htmlFor={`spec-${row.id}`} className="visually-hidden">
                  Spécification
                </label>
                <input
                  id={`spec-${row.id}`}
                  type="text"
                  value={row.text}
                  autoComplete="off"
                  spellCheck={false}
                  onChange={(event) => update({ ...row, text: event.target.value })}
                />
              </div>
            )}
          />
        </div>

        {/*
          Le composeur. Les symboles de l'ISO 1101 ne se tapent pas : ⏥ ⌭ ⌖ ⌰
          n'ont aucune touche, et leur nom en clair ne s'invente pas non plus.
          C'est l'écran où la liste apporte le plus — et ses options viennent du
          catalogue du moteur, jamais d'une table écrite ici.
        */}
        <div className="composer">
          <div className="composer-head">
            <span className="composer-title">Composer une spécification</span>
            <span className="faint">{draft === "" ? "—" : draft}</span>
          </div>

          <div className="composer-row">
            <Select
              id="spec-characteristic"
              label="Caractéristique"
              options={(catalogue?.characteristics ?? []).map((characteristic) => ({
                value: characteristic.symbol,
                label: characteristic.name,
                hint: characteristic.symbol,
                symbol: characteristic.symbol,
              }))}
              value={symbol}
              onChange={setSymbol}
              placeholder="perpendicularité…"
              emptyNote="Le catalogue n'a pas été chargé. La saisie directe reste possible."
            />

            <div className="field" style={{ minWidth: "110px" }}>
              <label htmlFor="spec-value">Valeur</label>
              <input
                id="spec-value"
                type="text"
                value={value}
                placeholder="0.05"
                autoComplete="off"
                spellCheck={false}
                onChange={(event) => setValue(event.target.value)}
              />
            </div>

            <div className="field">
              <span className="field-label field-spacer" aria-hidden="true" />
              <button
                type="button"
                className="btn-quiet"
                aria-pressed={diameter}
                onClick={() => setDiameter((current) => !current)}
              >
                Zone ø {diameter ? "oui" : "non"}
              </button>
            </div>

            <div className="field" style={{ minWidth: "120px" }}>
              <label htmlFor="spec-datums">Références</label>
              <input
                id="spec-datums"
                type="text"
                value={datums}
                placeholder="A B C"
                autoComplete="off"
                spellCheck={false}
                onChange={(event) => setDatums(event.target.value)}
              />
            </div>

            <Select
              id="spec-modifier"
              label="Modificateur"
              options={(catalogue?.modifiers ?? []).map((entry) => ({
                value: entry.symbol,
                label: entry.symbol,
                hint: entry.name,
                symbol: entry.symbol,
              }))}
              value={modifier}
              onChange={setModifier}
              placeholder="aucun"
              emptyNote="Le catalogue n'a pas été chargé."
            />

            <div className="field">
              <span className="field-label field-spacer" aria-hidden="true" />
              <button
                type="button"
                className="btn"
                disabled={draft === ""}
                onClick={() => {
                  setInput((current) => (current.trim() === "" ? draft : `${current}\n${draft}`));
                  setValue("");
                  setDatums("");
                  setModifier(null);
                  setDiameter(false);
                }}
              >
                Ajouter cette spécification
              </button>
            </div>
          </div>
        </div>

        <button type="submit" className="btn-primary" style={{ marginTop: "var(--s-6)" }}>
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
