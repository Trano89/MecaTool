/**
 * Appels au moteur.
 *
 * L'interface ne connaît que ces fonctions. Elle n'invoque jamais Tauri
 * directement, ce qui garde en un seul endroit la traduction des erreurs et
 * permet de faire tourner les composants sous test sans Tauri.
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  AppError,
  ChainReport,
  ClassComparison,
  Diagram,
  EngineInfo,
  FitComparison,
  GeometricCatalogue,
  GroupAnalysis,
  MeasureKind,
  Report,
} from "./types";

/** Vrai lorsque l'application tourne dans la fenêtre Tauri. */
export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Normalise ce que rejette `invoke`.
 *
 * Le moteur rejette un `AppError` sérialisé ; une panne d'un autre ordre
 * (commande absente, fenêtre fermée) rejette une chaîne ou une `Error`. Les
 * trois doivent arriver au composant sous la même forme.
 */
function toAppError(cause: unknown): AppError {
  if (
    typeof cause === "object" &&
    cause !== null &&
    "message" in cause &&
    typeof (cause as AppError).message === "string"
  ) {
    const error = cause as AppError;
    return { message: error.message, hint: error.hint ?? null };
  }
  if (typeof cause === "string") {
    return { message: cause, hint: null };
  }
  return {
    message: "Le moteur n'a pas répondu.",
    hint: cause instanceof Error ? cause.message : null,
  };
}

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isDesktop()) {
    throw {
      message: "Le moteur de calcul n'est pas accessible.",
      hint: "MecaTool doit être lancé comme application de bureau : « npm run app ».",
    } satisfies AppError;
  }
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    throw toAppError(cause);
  }
}

/**
 * Analyse une saisie, avec ou sans fenêtre de jeu.
 *
 * Une seule commande couvre le calcul, la vérification et la recherche : c'est
 * la saisie qui les distingue, et seul le moteur sait la lire.
 */
export function analyse(
  input: string,
  clearance?: string,
  trueToScale?: boolean,
): Promise<Report> {
  return call<Report>("analyse", { input, clearance, trueToScale });
}

/**
 * Compare plusieurs ajustements sur une même dimension.
 *
 * `input` s'écrit « Ø20 H7/g6, H7/h6, H7/k6, H7/p6 » : la virgule sépare les
 * ajustements, la barre sépare l'alésage de l'arbre. L'ordre de saisie est
 * conservé jusqu'à l'affichage.
 */
export function compare(
  input: string,
  clearance?: string,
  trueToScale?: boolean,
): Promise<FitComparison> {
  return call<FitComparison>("compare", { input, clearance, trueToScale });
}

/**
 * Calcule une chaîne de cotes.
 *
 * `input` liste les maillons, un par ligne : « A = 20 ±0.1 ». Un signe moins
 * devant le repère ou le nominal marque un maillon diminuant.
 *
 * `statistical` doit être demandé explicitement : l'estimation RSS est une
 * hypothèse sur la fabrication, pas une propriété de la géométrie.
 */
export function dimensionChain(
  input: string,
  // `| undefined` explicite : sous `exactOptionalPropertyTypes`, « absent » et
  // « présent mais indéfini » ne sont pas la même chose, et l'appelant a
  // naturellement le second.
  options: {
    statistical?: boolean | undefined;
    minimumMm?: string | undefined;
    maximumMm?: string | undefined;
  } = {},
): Promise<ChainReport> {
  return call<ChainReport>("dimension_chain", {
    input,
    statistical: options.statistical,
    minimumMm: options.minimumMm,
    maximumMm: options.maximumMm,
  });
}

/** Redessine un ajustement dans l'autre mode d'échelle, sans tout recalculer. */
export function rescaleDiagram(input: string, trueToScale: boolean): Promise<Diagram> {
  return call<Diagram>("rescale_diagram", { input, trueToScale });
}

/** État du moteur et de ses données, lu une fois au démarrage. */
export function engineInfo(): Promise<EngineInfo> {
  return call<EngineInfo>("engine_info", {});
}

/**
 * Les tolérances générales d'une cote, pour les quatre classes à la fois.
 *
 * Le comparatif est rendu d'un bloc plutôt que classe par classe : on choisit
 * une classe en voyant ce que les autres donneraient, pas en les interrogeant
 * une par une.
 *
 * Pour une cote angulaire, `nominalMm` est la **longueur du côté le plus court**
 * de l'angle — c'est ce que la norme prend en entrée.
 */
export function generalTolerances(
  kind: MeasureKind,
  nominalMm: string,
): Promise<ClassComparison> {
  return call<ClassComparison>("general_tolerances", { kind, nominalMm });
}

/**
 * Le catalogue des caractéristiques géométriques.
 *
 * Rendu d'un bloc plutôt qu'interrogé caractéristique par caractéristique : on
 * choisit une tolérance géométrique en voyant les autres, comme on choisit une
 * classe de tolérance générale.
 */
export function geometricCatalogue(): Promise<GeometricCatalogue> {
  return call<GeometricCatalogue>("geometric_catalogue", {});
}

/**
 * Lit et contrôle des spécifications géométriques posées sur un même élément.
 *
 * C'est ce « même élément » qui donne son sens au contrôle de recouvrement :
 * une tolérance d'orientation ne borne la forme que de l'élément qu'elle vise.
 */
export function geometric(specs: string[]): Promise<GroupAnalysis> {
  return call<GroupAnalysis>("geometric", { specs });
}
