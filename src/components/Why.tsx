/**
 * Le « pourquoi » d'un résultat.
 *
 * Le raisonnement n'est pas un texte rédigé après coup : ce sont les grandeurs
 * que le moteur a effectivement comparées, dans l'ordre où il les a comparées.
 * Replié par défaut, il ne charge pas l'écran du mode simple, et reste à un
 * clic pour qui veut vérifier.
 */

import type { ReasoningStep } from "../types";

interface Props {
  steps: ReasoningStep[];
  label?: string;
  open?: boolean;
}

export function Why({ steps, label = "Pourquoi ?", open = false }: Props) {
  if (steps.length === 0) return null;

  return (
    <details className="why" open={open}>
      <summary>{label}</summary>
      <div className="steps">
        {steps.map((step, index) => (
          <div key={`${step.label}-${index}`}>
            <div className="step-label">{step.label}</div>
            {step.expression ? <div className="step-expression">{step.expression}</div> : null}
            {step.value ? <div className="step-value">= {step.value}</div> : null}
          </div>
        ))}
      </div>
    </details>
  );
}
