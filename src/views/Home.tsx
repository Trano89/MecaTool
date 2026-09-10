/**
 * L'écran d'accueil.
 *
 * Une question, des cartes. L'utilisateur n'a pas à connaître la norme ni le
 * vocabulaire pour démarrer : il choisit ce qu'il veut obtenir, et Mecatol
 * pré-remplit la saisie correspondante.
 *
 * Les modules non encore construits sont montrés **désactivés** plutôt que
 * masqués : cacher la feuille de route ferait chercher en vain une fonction
 * qui n'existe pas.
 */

import type { Query } from "./Calculate";

interface Entry {
  title: string;
  question: string;
  query?: Query;
  later?: string;
}

const ENTRIES: readonly Entry[] = [
  {
    title: "Ajustement",
    question: "Quel jeu ou serrage vais-je obtenir ?",
    query: { input: "Ø20 H7/g6", clearance: "" },
  },
  {
    title: "Trouver une tolérance",
    question: "Je connais mon besoin fonctionnel.",
    query: { input: "Ø20", clearance: "10..30" },
  },
  {
    title: "Vérifier",
    question: "Ma tolérance convient-elle ?",
    query: { input: "Ø20 H7/g6", clearance: "5..50" },
  },
  {
    title: "Tolérances générales",
    question: "Que valent les cotes non tolérancées ?",
    later: "version 0.2",
  },
  {
    title: "Géométrie",
    question: "Quelle tolérance géométrique utiliser ?",
    later: "version 0.3",
  },
  {
    title: "État de surface",
    question: "Quel état de surface demander ?",
    later: "version 0.4",
  },
  {
    title: "Chaîne de cotes",
    question: "Quelle sera la dimension résultante ?",
    later: "version 0.5",
  },
];

interface Props {
  onStart: (query: Query) => void;
}

export function Home({ onStart }: Props) {
  return (
    <div className="stack">
      <div>
        <h1>Que voulez-vous calculer ?</h1>
        <p className="muted" style={{ marginTop: "0.35rem" }}>
          Partez d'une désignation normalisée, ou simplement du jeu dont vous avez besoin.
        </p>
      </div>

      <div className="cards">
        {ENTRIES.map((entry) => (
          <button
            key={entry.title}
            type="button"
            className="entry-card"
            disabled={!entry.query}
            onClick={() => entry.query && onStart(entry.query)}
          >
            <h3>{entry.title}</h3>
            <p>{entry.question}</p>
            {entry.later ? <span className="later">{entry.later}</span> : null}
          </button>
        ))}
      </div>
    </div>
  );
}
