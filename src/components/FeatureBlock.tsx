/**
 * Le bloc « ALÉSAGE H7 » ou « ARBRE g6 ».
 *
 * Les symboles d'écart suivent la norme : majuscules pour l'alésage (`EI`,
 * `ES`), minuscules pour l'arbre (`ei`, `es`). Un lecteur habitué aux tables
 * doit retrouver exactement ses repères.
 */

import { deviation, mm, nominal } from "../format";
import type { FeatureTolerance } from "../types";

interface Props {
  tolerance: FeatureTolerance;
}

export function FeatureBlock({ tolerance }: Props) {
  const isHole = tolerance.feature === "hole";
  const heading = isHole ? "Alésage" : "Arbre";
  const lower = isHole ? "EI" : "ei";
  const upper = isHole ? "ES" : "es";
  const designation = `${tolerance.class.letter}${tolerance.class.grade.replace("IT", "")}`;

  // Le nom accessible identifie la région : un lecteur d'écran annonce
  // « Alésage H7 » en y entrant, plutôt qu'une suite de cotes sans contexte.
  return (
    <section className="card" aria-label={`${heading} ${isHole ? designation : designation.toLowerCase()}`}>
      <header>
        <span className="card-title">{heading}</span>
        <strong
          className="num"
          style={{ color: isHole ? "var(--hole)" : "var(--shaft)", fontSize: "1.05rem" }}
        >
          {isHole ? designation : designation.toLowerCase()}
        </strong>
      </header>

      <dl className="rows">
        <dt>Dimension nominale</dt>
        <dd>{nominal(tolerance.nominal)} mm</dd>

        <dt>Écart supérieur {upper}</dt>
        <dd>{deviation(tolerance.deviations.upper)}</dd>

        <dt>Écart inférieur {lower}</dt>
        <dd>{deviation(tolerance.deviations.lower)}</dd>

        <dt>Dimension maximale</dt>
        <dd>{mm(tolerance.limits.max)} mm</dd>

        <dt>Dimension minimale</dt>
        <dd>{mm(tolerance.limits.min)} mm</dd>

        <dt>Tolérance {tolerance.class.grade}</dt>
        <dd>{mm(tolerance.it, 3)} mm</dd>
      </dl>

      <p className="faint" style={{ marginTop: "0.7rem" }}>
        Échelon des tables : au-dessus de {nominal(tolerance.size_range.above)} jusqu'à{" "}
        {nominal(tolerance.size_range.up_to)} mm
      </p>
    </section>
  );
}
