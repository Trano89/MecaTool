/**
 * Le socle commun des listes navigables au clavier.
 *
 * Deux composants s'en servent : la palette de commandes (Ctrl+K) et la liste
 * de sélection (`Select`). Ils n'ont pas la même apparence ni le même rôle, mais
 * ils posent exactement les mêmes questions — comment filtrer sans que les
 * accents gênent, quelle touche déplace quoi, que fait Entrée. Les réponses sont
 * ici, une seule fois.
 *
 * Ce module ne rend rien : il ne connaît ni le DOM ni la forme des options. Les
 * deux composants lui passent un tableau et reçoivent un gestionnaire de
 * touches.
 */

import { useCallback } from "react";

/**
 * Neutralise accents et casse.
 *
 * Personne ne tape « tolérancement » avec son accent dans un champ de
 * recherche, et « ø » se cherche aussi bien en écrivant « o ».
 */
export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}

/**
 * Vrai si **chaque mot** de la recherche se trouve dans l'un des textes.
 *
 * Mot à mot, et non sur la chaîne entière : « h7 alesage » doit trouver une
 * option dont le libellé porte « H7 » et la description « alésage », sans que
 * l'utilisateur ait à deviner l'ordre dans lequel elles sont écrites.
 */
export function matchesQuery(haystacks: ReadonlyArray<string | undefined>, query: string): boolean {
  const needle = fold(query);
  if (needle === "") return true;
  const hay = haystacks.filter((text): text is string => typeof text === "string").map(fold);
  return needle.split(/\s+/).every((word) => hay.some((text) => text.includes(word)));
}

interface KeyOptions<T> {
  items: readonly T[];
  active: number;
  setActive: (next: number) => void;
  onChoose: (item: T, index: number) => void;
  /** Échap, ou Tab qui sort de la liste. */
  onDismiss?: (() => void) | undefined;
  /** Flèche bas sur une liste fermée : l'ouvrir plutôt que de déplacer. */
  onOpen?: (() => void) | undefined;
  isOpen?: boolean;
}

/**
 * Le gestionnaire de touches d'une liste.
 *
 * Les flèches **bouclent** : arrivé en bas, on repart en haut. Sur une liste
 * courte, c'est plus rapide que de remonter, et cela évite le cul-de-sac d'une
 * flèche qui ne fait plus rien.
 *
 * `Tab` ferme sans choisir. Une liste ouverte ne doit pas retenir le focus :
 * on doit pouvoir passer au champ suivant sans lever les mains du clavier.
 */
export function useListboxKeys<T>({
  items,
  active,
  setActive,
  onChoose,
  onDismiss,
  onOpen,
  isOpen = true,
}: KeyOptions<T>) {
  return useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === "Escape") {
        if (onDismiss) {
          event.preventDefault();
          onDismiss();
        }
        return;
      }
      if (event.key === "Tab") {
        onDismiss?.();
        return;
      }
      if (!isOpen) {
        if (event.key === "ArrowDown" || event.key === "ArrowUp") {
          event.preventDefault();
          onOpen?.();
        }
        return;
      }
      if (items.length === 0) return;

      switch (event.key) {
        case "ArrowDown":
          event.preventDefault();
          setActive((active + 1) % items.length);
          break;
        case "ArrowUp":
          event.preventDefault();
          setActive((active - 1 + items.length) % items.length);
          break;
        case "Home":
          event.preventDefault();
          setActive(0);
          break;
        case "End":
          event.preventDefault();
          setActive(items.length - 1);
          break;
        case "Enter": {
          const picked = items[active] ?? items[0];
          if (picked !== undefined) {
            event.preventDefault();
            onChoose(picked, items[active] !== undefined ? active : 0);
          }
          break;
        }
        default:
          break;
      }
    },
    [items, active, setActive, onChoose, onDismiss, onOpen, isOpen],
  );
}

/**
 * Ce qu'annonce le lecteur d'écran quand la liste se filtre.
 *
 * Le nombre de résultats est une information que le voyant lit d'un coup d'œil
 * et que l'autre n'a pas. Elle est donc dite, dans une région `aria-live`.
 */
export function resultAnnouncement(count: number): string {
  if (count === 0) return "Aucun résultat.";
  if (count === 1) return "1 résultat.";
  return `${count} résultats.`;
}
