/**
 * Les solutions d'une recherche, classées.
 *
 * L'ordre vient du moteur : celles qui satisfont entièrement l'exigence
 * d'abord, puis celles qui s'en approchent le plus. L'interface ne reclasse
 * rien — elle afficherait sinon un ordre que le « pourquoi » ne justifierait
 * plus.
 */

import { umLabel } from "../format";
import type { Solution } from "../types";
import { VERDICT_BADGE, VERDICT_LABEL } from "../types";

interface Props {
  solutions: Solution[];
  /** Nombre de lignes affichées avant repli. */
  limit?: number;
  onSelect?: (designation: string) => void;
}

function designationOf(solution: Solution): string {
  const { hole, shaft } = solution.fit;
  const holeName = `${hole.class.letter}${hole.class.grade.replace("IT", "")}`;
  const shaftName = `${shaft.class.letter}${shaft.class.grade.replace("IT", "")}`.toLowerCase();
  return `${holeName}/${shaftName}`;
}

export function SolutionsTable({ solutions, limit = 15, onSelect }: Props) {
  const shown = solutions.slice(0, limit);

  return (
    <div className="table-scroll">
      <table>
        <caption className="visually-hidden">
          Ajustements normalisés répondant au besoin, du plus recommandable au moins
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
            <th scope="col">Verdict</th>
          </tr>
        </thead>
        <tbody>
          {shown.map((solution) => {
            const designation = designationOf(solution);
            const { verdict } = solution.verification;
            return (
              <tr key={designation}>
                <th scope="row" className="num" style={{ fontWeight: 600 }}>
                  {onSelect ? (
                    <button
                      type="button"
                      className="btn-quiet"
                      onClick={() => onSelect(designation)}
                      title={`Ouvrir ${designation}`}
                    >
                      {designation}
                    </button>
                  ) : (
                    designation
                  )}
                </th>
                <td className="num">{umLabel(solution.fit.min_clearance)}</td>
                <td className="num">{umLabel(solution.fit.max_clearance)}</td>
                <td className="num">{umLabel(solution.total_tolerance)}</td>
                <td>
                  <span aria-hidden="true">{VERDICT_BADGE[verdict]} </span>
                  {VERDICT_LABEL[verdict]}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>

      {solutions.length > shown.length ? (
        <p className="faint" style={{ marginTop: "0.5rem" }}>
          {solutions.length - shown.length} autres solutions moins bien classées ne sont pas
          affichées.
        </p>
      ) : null}
    </div>
  );
}
