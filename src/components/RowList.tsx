/**
 * Une liste de lignes qu'on ajoute et qu'on retire.
 *
 * Partout où l'application demande « plusieurs » de quelque chose — plusieurs
 * ajustements à comparer, plusieurs maillons d'une chaîne, plusieurs
 * spécifications sur un même élément — c'est ce composant qui porte le motif.
 * Il ne sait rien du contenu d'une ligne : l'appelant le rend, lui tient les
 * champs, et reçoit le tableau mis à jour.
 *
 * ## Retirer la dernière ligne
 *
 * Une liste vide n'est pas un état mort : c'est un état **nommé**. La liste
 * affiche alors ce qu'il faut faire pour en sortir, et le bouton d'ajout reste
 * à sa place. L'alternative — interdire le retrait de la dernière ligne —
 * laisserait un bouton qui ne fait rien, ce qui se lit comme une panne.
 *
 * ## Pourquoi un identifiant par ligne
 *
 * React réutilise les nœuds d'une liste d'après leur clé. Avec l'indice pour
 * clé, retirer la deuxième de trois lignes ferait glisser le contenu des champs
 * d'une ligne à l'autre — le curseur et la sélection avec. Chaque ligne porte
 * donc une identité stable, que `nextRowId()` fabrique.
 */

import type { ReactNode } from "react";

/** Une ligne, quelle qu'elle soit, porte une identité stable. */
export interface Row {
  id: string;
}

let counter = 0;

/**
 * Un identifiant de ligne.
 *
 * Un simple compteur : ces identifiants ne servent qu'à React, le temps d'une
 * session. Ils ne sont ni transmis au moteur, ni conservés.
 */
export function nextRowId(): string {
  counter += 1;
  return `row-${counter}`;
}

interface Props<T extends Row> {
  /** L'intitulé du groupe. Il nomme les lignes : « Maillons », « Ajustements ». */
  legend: string;
  items: readonly T[];
  onChange: (items: T[]) => void;
  /** Fabrique une ligne vierge. */
  create: () => T;
  /** Rend les champs d'une ligne. `update` remplace la ligne entière. */
  renderRow: (item: T, index: number, update: (next: T) => void) => ReactNode;
  /** Le libellé du bouton d'ajout : « Ajouter un maillon ». */
  addLabel: string;
  /**
   * Comment nommer une ligne dans le bouton « Retirer ».
   *
   * Sans cela, une page de cinq lignes offrirait cinq boutons nommés
   * « Retirer », que rien ne distinguerait à l'oreille.
   */
  describe: (item: T, index: number) => string;
  /** Ce qui s'affiche quand il ne reste aucune ligne. */
  emptyNote: string;
  /** Le maximum que le moteur accepte, le cas échéant. */
  max?: number | undefined;
  /** Pourquoi ce maximum. Affiché quand il est atteint. */
  maxNote?: string | undefined;
  /** Aide de saisie, sous les lignes. */
  hint?: ReactNode;
}

export function RowList<T extends Row>({
  legend,
  items,
  onChange,
  create,
  renderRow,
  addLabel,
  describe,
  emptyNote,
  max,
  maxNote,
  hint,
}: Props<T>) {
  const full = max !== undefined && items.length >= max;

  const replace = (index: number, next: T) => {
    onChange(items.map((item, position) => (position === index ? next : item)));
  };

  const remove = (index: number) => {
    onChange(items.filter((_, position) => position !== index));
  };

  return (
    <div className="rowlist">
      <div className="rowlist-head">
        <span className="field-label" id={`${legend}-legend`}>
          {legend}
        </span>
        <span className="faint">
          {items.length} ligne{items.length > 1 ? "s" : ""}
        </span>
      </div>

      {items.length === 0 ? (
        <p className="rowlist-empty">{emptyNote}</p>
      ) : (
        <ul className="rowlist-items" aria-labelledby={`${legend}-legend`}>
          {items.map((item, index) => (
            <li className="rowlist-item" key={item.id}>
              {/* Le numéro repère la ligne dans les messages du moteur. Il est
                  décoratif : le bouton « Retirer » porte, lui, un nom complet. */}
              <span className="rowlist-index" aria-hidden="true">
                {index + 1}
              </span>
              <div className="rowlist-fields">
                {renderRow(item, index, (next) => replace(index, next))}
              </div>
              <button
                type="button"
                className="rowlist-remove"
                aria-label={`Retirer ${describe(item, index)}`}
                title={`Retirer ${describe(item, index)}`}
                onClick={() => remove(index)}
              >
                <span aria-hidden="true">✕</span>
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="rowlist-foot">
        <button
          type="button"
          className="btn"
          disabled={full}
          onClick={() => onChange([...items, create()])}
        >
          <span aria-hidden="true">+</span> {addLabel}
        </button>
        {full && maxNote ? <span className="hint">{maxNote}</span> : null}
      </div>

      {hint ? <div className="hint">{hint}</div> : null}
    </div>
  );
}
