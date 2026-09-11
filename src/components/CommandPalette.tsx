/**
 * La palette de commandes.
 *
 * C'est elle qui permet à la barre latérale de rester une **carte du domaine**
 * plutôt qu'une liste de raccourcis : quand douze domaines seront là, on ira
 * les chercher au clavier, pas à la souris.
 *
 * La recherche porte sur le nom **et** sur la question à laquelle le domaine
 * répond. C'est délibéré : un utilisateur cherche « jeu » ou « rugosité » bien
 * avant de connaître le nom du module qui s'en occupe.
 *
 * Les domaines à venir sont listés à part, sous leur propre intertitre, avec la
 * raison de leur absence. Ils ne sont pas atteignables aux flèches — rien à y
 * valider — mais ils se lisent : une recherche infructueuse doit se lire « pas
 * encore », jamais « ça n'existe pas ».
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { Domain } from "../types";
import { HomeIcon, SearchIcon } from "./icons";
import { useListboxKeys } from "./listbox";
import { HOME, iconFor, isAvailable, matches, type Screen } from "./navigation";

interface Props {
  open: boolean;
  domains: Domain[];
  onClose: () => void;
  onNavigate: (screen: Screen) => void;
}

interface Entry {
  id: Screen;
  name: string;
  question: string;
  icon: ReturnType<typeof iconFor>;
  unavailable: string | null;
}

const HOME_ENTRY: Entry = {
  id: HOME,
  name: "Accueil",
  question: "Revenir au choix du domaine.",
  icon: HomeIcon,
  unavailable: null,
};

export function CommandPalette({ open, domains, onClose, onNavigate }: Props) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  // Pour rendre le focus à ce qui l'avait avant l'ouverture.
  const restoreTo = useRef<HTMLElement | null>(null);

  const { reachable, later } = useMemo(() => {
    const entries: Entry[] = [
      HOME_ENTRY,
      ...domains.map((domain) => ({
        id: domain.id,
        name: domain.name,
        question: domain.question,
        icon: iconFor(domain.id),
        unavailable: isAvailable(domain) ? null : domain.unavailable,
      })),
    ];
    const kept = entries.filter(
      (entry) =>
        query.trim() === "" ||
        domains.some((domain) => domain.id === entry.id && matches(domain, query)) ||
        (entry.id === HOME && matches(asDomain(HOME_ENTRY), query)),
    );
    return {
      reachable: kept.filter((entry) => entry.unavailable === null),
      later: kept.filter((entry) => entry.unavailable !== null),
    };
  }, [domains, query]);

  useEffect(() => {
    if (!open) return;
    restoreTo.current = document.activeElement as HTMLElement | null;
    setQuery("");
    setActive(0);
    inputRef.current?.focus();
    return () => restoreTo.current?.focus?.();
  }, [open]);

  useEffect(() => {
    setActive(0);
  }, [query]);

  const choose = useCallback(
    (entry: Entry) => {
      onNavigate(entry.id);
      onClose();
    },
    [onNavigate, onClose],
  );

  // Flèches, Entrée, Échap, Tab : la même mécanique que la liste de sélection,
  // et le même code. Deux listes navigables au clavier dans la même application
  // doivent se comporter à l'identique.
  const onKeyDown = useListboxKeys({
    items: reachable,
    active,
    setActive,
    onChoose: choose,
    onDismiss: onClose,
  });

  if (!open) return null;

  return (
    <div
      className="palette-scrim"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label="Aller à un domaine"
        onKeyDown={onKeyDown}
      >
        <div className="palette-field">
          <span className="nav-icon">
            <SearchIcon />
          </span>
          <input
            ref={inputRef}
            type="text"
            className="palette-input"
            placeholder="Chercher un domaine, ou ce que vous voulez obtenir…"
            value={query}
            autoComplete="off"
            spellCheck={false}
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-list"
            aria-activedescendant={
              reachable[active] ? `palette-option-${reachable[active].id}` : undefined
            }
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>

        {reachable.length === 0 && later.length === 0 ? (
          <p className="palette-empty">Aucun domaine ne répond à « {query} ».</p>
        ) : (
          <ul className="palette-list" id="palette-list" role="listbox">
            {reachable.map((entry, index) => (
              <li
                key={entry.id}
                id={`palette-option-${entry.id}`}
                className="palette-option"
                role="option"
                aria-selected={index === active}
                onMouseEnter={() => setActive(index)}
                onClick={() => choose(entry)}
              >
                <span className="nav-icon">
                  <entry.icon />
                </span>
                <span className="nav-label">{entry.name}</span>
                <span className="palette-hint">{entry.question}</span>
              </li>
            ))}

            {later.length > 0 ? (
              <>
                <li className="palette-group" role="presentation">
                  À venir
                </li>
                {later.map((entry) => (
                  <li
                    key={entry.id}
                    className="palette-option"
                    role="option"
                    aria-selected={false}
                    aria-disabled="true"
                    title={entry.unavailable ?? undefined}
                  >
                    <span className="nav-icon">
                      <entry.icon />
                    </span>
                    <span className="nav-label">{entry.name}</span>
                    <span className="nav-flag">à venir</span>
                  </li>
                ))}
              </>
            ) : null}
          </ul>
        )}

        <div className="palette-foot">
          <span>
            <kbd>↑</kbd> <kbd>↓</kbd> parcourir
          </span>
          <span>
            <kbd>↵</kbd> ouvrir
          </span>
          <span>
            <kbd>Échap</kbd> fermer
          </span>
        </div>
      </div>
    </div>
  );
}

/** L'accueil n'est pas un domaine du registre : on lui en donne la forme pour
 *  le passer au même filtre, plutôt que d'écrire deux fois la recherche. */
function asDomain(entry: Entry): Domain {
  return {
    id: entry.id,
    name: entry.name,
    question: entry.question,
    group: "dimensional",
    group_label: "",
    status: "ready",
    sources: [],
    reserve: null,
    unavailable: null,
    examples: [],
  };
}
