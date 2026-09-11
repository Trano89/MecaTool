/**
 * Les classes de tolérance, telles que l'interface a le droit de les proposer.
 *
 * ⚠ **Rien n'est écrit ici.** La liste vient de `tolerance_classes()`, donc du
 * moteur. Les lettres disponibles ne sont pas une décision d'affichage : ce sont
 * celles dont MecaTool possède les écarts fondamentaux — dix sur vingt-huit. Une
 * liste rédigée dans l'interface offrirait des classes que le moteur refuserait
 * ensuite de calculer, et l'utilisateur ne comprendrait pas pourquoi.
 *
 * Deux écrans s'en servent — le calcul et la comparaison — d'où ce module.
 */

import { useEffect, useState } from "react";

import { toleranceClasses } from "../api";
import type { ClassCatalogue, ClassOption } from "../types";
import type { Option } from "./Select";

/**
 * Charge le catalogue une fois.
 *
 * Un échec est silencieux : les listes restent vides, elles le disent, et le
 * champ de saisie libre continue de fonctionner. Une liste de secours écrite
 * dans l'interface serait pire que pas de liste du tout.
 */
export function useToleranceClasses(): ClassCatalogue | null {
  const [catalogue, setCatalogue] = useState<ClassCatalogue | null>(null);

  useEffect(() => {
    let cancelled = false;
    toleranceClasses()
      .then((loaded) => {
        if (!cancelled) setCatalogue(loaded);
      })
      .catch(() => {
        // La saisie libre reste ouverte : le composeur est une aide, pas le
        // seul chemin.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return catalogue;
}

/**
 * Les options d'une liste de sélection.
 *
 * Le degré figure en précision autant pour se lire que pour se chercher : taper
 * « it7 » doit ramener toutes les classes de ce degré, alors que la désignation
 * seule (« H7 ») ne s'y prête pas.
 */
export function classOptions(list: readonly ClassOption[] | undefined): Option[] {
  return (list ?? []).map((option) => ({
    value: option.designation,
    label: option.designation,
    hint: option.grade,
  }));
}

/** La note affichée quand le moteur n'a pas répondu. */
export const CLASSES_UNAVAILABLE =
  "Le moteur n'a pas fourni la liste des classes. La saisie directe reste possible.";
