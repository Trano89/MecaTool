/**
 * Des constats du moteur, pastille et libellé ensemble.
 *
 * La pastille accompagne toujours le libellé — « À corriger », « À vérifier »,
 * « Pour information » — et ne le remplace jamais : la couleur seule ne porte
 * pas l'information.
 */

import type { Finding } from "../types";
import { SEVERITY_BADGE, SEVERITY_CLASS, SEVERITY_LABEL } from "../types";

export function Findings({ findings }: { findings: Finding[] }) {
  if (findings.length === 0) return null;
  return (
    <ul className="findings">
      {findings.map((finding) => (
        <li key={finding.code} className={`finding ${SEVERITY_CLASS[finding.severity]}`}>
          <span aria-hidden="true">{SEVERITY_BADGE[finding.severity]}</span>
          <span>
            <strong>{SEVERITY_LABEL[finding.severity]}</strong> — {finding.message}
          </span>
        </li>
      ))}
    </ul>
  );
}

/** Une erreur du moteur, avec sa piste d'action. */
export function ErrorBox({ error }: { error: { message: string; hint: string | null } }) {
  return (
    <div className="status incompatible" role="alert">
      <span className="status-badge" aria-hidden="true">
        🔴
      </span>
      <div>
        <div className="status-headline">{error.message}</div>
        {error.hint ? <div className="status-detail">{error.hint}</div> : null}
      </div>
    </div>
  );
}
