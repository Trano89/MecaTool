/**
 * Une liste de sélection avec recherche.
 *
 * ## Pourquoi une liste plutôt qu'un champ libre
 *
 * Les classes de tolérance sont deux cents par élément, les caractéristiques
 * géométriques portent des symboles (⏥ ⟂ ⌭ ⌖) qu'aucun clavier ne compose. Une
 * liste montre ce qui existe — et surtout, elle ne montre **que** ce que le
 * moteur sait calculer : ses options viennent toujours de lui, jamais d'un
 * tableau écrit dans l'interface. Dix lettres sur vingt-huit sont embarquées ;
 * une liste écrite ici proposerait des classes que le moteur refuserait ensuite.
 *
 * ## Pourquoi avec un champ de recherche
 *
 * Deux cents options ne se parcourent pas à la souris. Le champ **est** la
 * liste : on y tape pour filtrer, les flèches déplacent, Entrée choisit, Échap
 * ferme. Celui qui connaît sa classe tape « h7 » et valide sans jamais voir la
 * liste ; celui qui cherche la déroule.
 *
 * ## Accessibilité
 *
 * Motif `combobox` : le champ porte `role="combobox"`, `aria-expanded`,
 * `aria-controls` et `aria-activedescendant` — c'est ce dernier qui fait
 * annoncer l'option survolée au clavier sans jamais déplacer le focus. Le
 * nombre de résultats est annoncé dans une région `aria-live` : c'est une
 * information que le voyant lit d'un coup d'œil et que l'autre n'aurait pas.
 *
 * L'option retenue n'est jamais signalée par la seule couleur : elle porte une
 * coche, et `aria-selected`.
 */

import { useEffect, useId, useMemo, useRef, useState } from "react";

import { matchesQuery, resultAnnouncement, useListboxKeys } from "./listbox";

export interface Option {
  /** Ce que le composant rend à l'appelant. */
  value: string;
  /** Ce qui s'affiche, et ce sur quoi porte la recherche. */
  label: string;
  /** Précision affichée à droite, également cherchable. */
  hint?: string | undefined;
  /**
   * Un glyphe affiché devant le libellé — un symbole ISO 1101, par exemple.
   * Décoratif : le libellé reste seul porteur du sens, car la police peut ne
   * pas connaître le glyphe.
   */
  symbol?: string | undefined;
}

interface Props {
  id: string;
  /** L'intitulé visible. Toujours présent : pas de champ sans étiquette. */
  label: string;
  options: readonly Option[];
  value: string | null;
  onChange: (value: string) => void;
  placeholder?: string;
  /** L'aide de saisie, sous le champ. */
  hint?: string | undefined;
  /** Ce qu'on lit quand le moteur n'a pas (encore) fourni d'options. */
  emptyNote?: string;
  disabled?: boolean;
  /** Masque l'intitulé sans le retirer de l'arbre d'accessibilité. */
  hideLabel?: boolean;
}

export function Select({
  id,
  label,
  options,
  value,
  onChange,
  placeholder,
  hint,
  emptyNote = "Le moteur n'a pas fourni de liste.",
  disabled = false,
  hideLabel = false,
}: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const listId = `${useId().replace(/:/g, "")}-list`;

  const selected = useMemo(
    () => options.find((option) => option.value === value) ?? null,
    [options, value],
  );

  const filtered = useMemo(
    () => options.filter((option) => matchesQuery([option.label, option.hint], query)),
    [options, query],
  );

  // L'option retenue est celle qu'on met en avant à l'ouverture : on rouvre une
  // liste là où on l'avait laissée, pas au début.
  useEffect(() => {
    if (!open) return;
    const index = filtered.findIndex((option) => option.value === value);
    setActive(index >= 0 ? index : 0);
  }, [open, value, filtered]);

  useEffect(() => {
    if (open) setActive(0);
  }, [query, open]);

  // Garder l'option active visible quand on la déplace au clavier.
  //
  // `scrollIntoView` n'existe pas partout — jsdom ne l'implémente pas, et c'est
  // un confort, pas une fonction. L'appeler sans garde ferait tomber tout
  // l'écran là où il manque, ce qui est hors de proportion avec ce qu'il rend.
  useEffect(() => {
    if (!open) return;
    const target = listRef.current?.querySelector<HTMLElement>('[aria-selected="true"]');
    target?.scrollIntoView?.({ block: "nearest" });
  }, [active, open]);

  const close = () => {
    setOpen(false);
    setQuery("");
  };

  const choose = (option: Option) => {
    onChange(option.value);
    close();
    inputRef.current?.focus();
  };

  const onKeyDown = useListboxKeys({
    items: filtered,
    active,
    setActive,
    onChoose: choose,
    onDismiss: close,
    onOpen: () => setOpen(true),
    isOpen: open,
  });

  const activeId = filtered[active] ? `${listId}-${active}` : undefined;

  return (
    <div className="field">
      <label htmlFor={id} className={hideLabel ? "visually-hidden" : undefined}>
        {label}
      </label>

      <div
        className="combo"
        ref={rootRef}
        onBlur={(event) => {
          // Ne fermer que si le focus quitte vraiment le composant : passer du
          // champ à une option ne doit pas refermer la liste.
          if (!rootRef.current?.contains(event.relatedTarget as Node | null)) close();
        }}
      >
        <input
          id={id}
          ref={inputRef}
          type="text"
          className="combo-input"
          role="combobox"
          autoComplete="off"
          spellCheck={false}
          disabled={disabled}
          aria-expanded={open}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={activeId}
          placeholder={placeholder ?? "Choisir…"}
          value={open ? query : (selected?.label ?? "")}
          onChange={(event) => {
            setOpen(true);
            setQuery(event.target.value);
          }}
          onMouseDown={() => {
            if (!disabled) setOpen((current) => !current);
          }}
          onKeyDown={onKeyDown}
        />

        <span className="combo-caret" aria-hidden="true">
          ▾
        </span>

        {open ? (
          <ul className="combo-list" id={listId} role="listbox" aria-label={label} ref={listRef}>
            {filtered.length === 0 ? (
              <li className="combo-empty" role="presentation">
                {options.length === 0 ? emptyNote : `Aucune option ne correspond à « ${query} ».`}
              </li>
            ) : (
              filtered.map((option, index) => (
                <li
                  key={option.value}
                  id={`${listId}-${index}`}
                  className="combo-option"
                  role="option"
                  aria-selected={index === active}
                  onMouseEnter={() => setActive(index)}
                  // `mousedown` et non `click` : le `blur` du champ partirait
                  // avant que le clic n'aboutisse, et refermerait la liste.
                  onMouseDown={(event) => {
                    event.preventDefault();
                    choose(option);
                  }}
                >
                  {option.symbol ? (
                    <span className="combo-symbol" aria-hidden="true">
                      {option.symbol}
                    </span>
                  ) : null}
                  <span className="combo-label">{option.label}</span>
                  {option.hint ? <span className="combo-hint">{option.hint}</span> : null}
                  {/* La coche double `aria-selected` : le choix courant se
                      reconnaît sans distinguer les couleurs. */}
                  {option.value === value ? (
                    <span className="combo-check" aria-hidden="true">
                      ✓
                    </span>
                  ) : null}
                </li>
              ))
            )}
          </ul>
        ) : null}
      </div>

      {/*
        `aria-live` sans `role="status"` : le rôle serait redondant — il ne fait
        qu'impliquer `aria-live="polite"` — et il ferait de chaque liste de
        sélection un « statut » supplémentaire sur la page, là où ce rôle est
        réservé aux verdicts du moteur.
      */}
      <span className="visually-hidden" aria-live="polite" aria-atomic="true">
        {open ? resultAnnouncement(filtered.length) : ""}
      </span>

      {hint ? <p className="hint">{hint}</p> : null}
    </div>
  );
}
