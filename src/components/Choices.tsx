/**
 * Un choix parmi des valeurs connues, par boutons.
 *
 * ## Pourquoi des boutons plutôt qu'un champ
 *
 * Une tolérance, une classe, un pas, une valeur de rugosité ne se tapent pas :
 * ils se **choisissent** parmi ce que le moteur connaît. Un champ libre laisse
 * écrire « 0,08 » là où la série ne connaît que 0,8, et ne le découvre qu'au
 * calcul. Un bouton ne propose que ce qui existe — et ses options viennent
 * toujours du moteur, jamais d'une liste écrite ici.
 *
 * Deux dispositions, une seule logique : `segmented` pour quelques options à
 * voir d'un coup d'œil, `chips` pour une série plus longue qui passe à la
 * ligne. Le bouton retenu porte `aria-pressed` **et** une coche : la couleur
 * seule ne porte jamais l'information.
 */

export interface Choice {
  value: string;
  label: string;
  /** Précision affichée au survol. */
  title?: string | undefined;
}

interface Props {
  legend: string;
  options: readonly Choice[];
  value: string | null;
  onChange: (value: string | null) => void;
  /** Ajoute un bouton qui désélectionne, avec ce libellé. */
  noneLabel?: string | undefined;
  layout?: "segmented" | "chips";
  /** Masque la légende visuellement, sans la retirer aux lecteurs d'écran. */
  hideLegend?: boolean;
}

export function ChoiceGroup({
  legend,
  options,
  value,
  onChange,
  noneLabel,
  layout = "segmented",
  hideLegend = false,
}: Props) {
  const all: Choice[] = noneLabel ? [{ value: "", label: noneLabel }, ...options] : [...options];
  return (
    <fieldset className="field choice-group" style={{ border: 0, padding: 0, margin: 0 }}>
      <legend className={hideLegend ? "visually-hidden" : "field-label"} style={{ padding: 0 }}>
        {legend}
      </legend>
      <div className={layout === "chips" ? "chips choice-chips" : "segmented"}>
        {all.map((option) => {
          const pressed = option.value === "" ? value === null : value === option.value;
          return (
            <button
              key={option.value === "" ? "__none" : option.value}
              type="button"
              className={layout === "chips" ? "chip" : undefined}
              aria-pressed={pressed}
              title={option.title}
              onClick={() => onChange(option.value === "" ? null : option.value)}
            >
              {layout === "chips" && pressed ? "✓ " : ""}
              {option.label}
            </button>
          );
        })}
      </div>
    </fieldset>
  );
}
