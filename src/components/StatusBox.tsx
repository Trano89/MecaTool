/**
 * Une conclusion, avec son verdict.
 *
 * La pastille **accompagne** toujours le libellé, elle ne le remplace jamais :
 * un utilisateur qui ne distingue pas les couleurs lit « NON COMPATIBLE » aussi
 * bien que les autres. Le libellé et la pastille viennent du moteur.
 */

import type { Conclusion } from "../types";
import { VERDICT_BADGE, VERDICT_CLASS, VERDICT_LABEL } from "../types";
import { Why } from "./Why";

interface Props {
  conclusion: Conclusion;
  /** Replier le raisonnement plutôt que de l'afficher. */
  collapsedWhy?: boolean;
}

export function StatusBox({ conclusion, collapsedWhy = true }: Props) {
  const { verdict } = conclusion;

  return (
    <div className="stack" style={{ gap: "0.6rem" }}>
      <div className={`status ${VERDICT_CLASS[verdict]}`} role="status">
        <span className="status-badge" aria-hidden="true">
          {VERDICT_BADGE[verdict]}
        </span>
        <div>
          <div className="status-headline">{VERDICT_LABEL[verdict]}</div>
          <div className="status-detail">{conclusion.detail}</div>
        </div>
      </div>

      <Why steps={conclusion.why} open={!collapsedWhy} />

      {conclusion.warnings.map((warning) => (
        <p key={warning} className="faint">
          {warning}
        </p>
      ))}
    </div>
  );
}
