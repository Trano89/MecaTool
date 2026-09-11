/**
 * D'où viennent les chiffres.
 *
 * Chaque référence porte son **état de source** en toutes lettres, pas
 * seulement en couleur : « Vérifié sur la norme », « Source secondaire »,
 * « Pratique recommandée », « Non vérifié ». La distinction n'est pas
 * décorative — entre une valeur confrontée à l'ISO 286 et une classe de montage
 * que des fabricants recommandent, il y a la différence entre ce qui s'impose et
 * ce dont on peut s'écarter en connaissance de cause.
 *
 * Les notes de chaque source restent repliées : elles éclairent, elles
 * n'encombrent pas.
 */

import type { Provenance, StandardReference, VerificationStatus } from "../types";
import { VERIFICATION_DETAIL, VERIFICATION_LABEL } from "../types";

interface Props {
  provenance: Provenance;
  /** L'intitulé du panneau. */
  label?: string;
}

export function Sources({ provenance, label = "Sources" }: Props) {
  if (provenance.references.length === 0) return null;

  return (
    <section className="card">
      <header>
        <span className="card-title">{label}</span>
        <span className="faint">
          {provenance.references.length} référence
          {provenance.references.length > 1 ? "s" : ""}
        </span>
      </header>

      <div className="stack" style={{ gap: "var(--s-6)" }}>
        {provenance.references.map((reference, index) => (
          <Reference key={`${reference.id}-${reference.scope ?? index}`} reference={reference} />
        ))}
      </div>
    </section>
  );
}

function Reference({ reference }: { reference: StandardReference }) {
  const { state } = reference.verification;

  return (
    <div className="source">
      <div className="source-head">
        <strong className="mono">
          {reference.id}
          {reference.edition ? `:${reference.edition}` : ""}
        </strong>
        <span className={`source-tag ${state}`}>{VERIFICATION_LABEL[state]}</span>
      </div>

      <p className="muted">{reference.title}</p>
      {reference.scope ? <p className="faint">Portée : {reference.scope}</p> : null}

      <p className="faint">{VERIFICATION_DETAIL(reference.verification)}</p>

      {reference.notes.length > 0 ? (
        <details className="why">
          <summary>Ce que dit la source, et ce qu'elle ne dit pas</summary>
          <div className="steps">
            {reference.notes.map((note) => (
              <div key={note}>
                <p className="step-expression" style={{ margin: 0 }}>
                  {note}
                </p>
              </div>
            ))}
          </div>
        </details>
      ) : null}
    </div>
  );
}

/**
 * L'état d'une provenance, résumé en une étiquette.
 *
 * C'est **le plus faible** des états de ses références qui est retenu : une
 * seule source recommandée dans un résultat par ailleurs normatif suffit à ce
 * que le résultat entier ne soit plus normatif. L'étiquette porte aussi les
 * normes citées, pour que l'étiquette et la source se lisent d'un seul coup.
 */
export function SourceTag({ provenance }: { provenance: Provenance }) {
  const state = weakest(provenance);
  const ids = [...new Set(provenance.references.map((reference) => reference.id))];

  return (
    <span className={`source-tag ${state}`}>
      {VERIFICATION_LABEL[state]}
      {ids.length > 0 ? <span className="source-tag-refs">{ids.join(" · ")}</span> : null}
    </span>
  );
}

/** Du plus faible au plus fort. Le premier rencontré l'emporte. */
const WEAKEST_FIRST: ReadonlyArray<VerificationStatus["state"]> = [
  "unverified",
  "recommended",
  "secondary",
  "verified",
];

function weakest(provenance: Provenance): VerificationStatus["state"] {
  const states = new Set(provenance.references.map((reference) => reference.verification.state));
  return WEAKEST_FIRST.find((state) => states.has(state)) ?? "verified";
}
