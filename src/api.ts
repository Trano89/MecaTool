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
  BearingCatalogue,
  ChainReport,
  ClassCatalogue,
  ClassComparison,
  DesignationReading,
  Diagram,
  Domain,
  EngineInfo,
  FastenerCatalogue,
  FitComparison,
  MaterialsCatalogue,
  GeometricCatalogue,
  GroupAnalysis,
  MeasureKind,
  MountingAdvice,
  MountingOption,
  ProcessReading,
  Report,
  SteelReading,
  SurfaceAnalysis,
  SurfaceCatalogue,
  ThermalFit,
  ThreadReport,
  WeldingCatalogue,
  WeldReading,
  WeldRequest,
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

/**
 * Le registre des domaines, lu une fois au démarrage.
 *
 * C'est le moteur qui dit ce que l'application sait faire. La barre latérale,
 * l'accueil et la palette de commandes se construisent à partir de cette liste
 * et d'elle seule : ajouter un domaine ne touche aucun composant.
 */
export function domains(): Promise<Domain[]> {
  return call<Domain[]>("domains", {});
}

/**
 * Le catalogue des roulements : familles, régimes de charge, cas d'emploi.
 *
 * Rendu d'un bloc avant toute saisie, comme le catalogue géométrique — et pour
 * la même raison : ses `warnings` portent la réserve qui dit que le tableau des
 * classes de montage n'est **pas** normatif. C'est à savoir avant de s'en
 * servir, pas après.
 */
export function bearingCatalogue(): Promise<BearingCatalogue> {
  return call<BearingCatalogue>("bearing_catalogue", {});
}

/**
 * Les lectures possibles d'une désignation de roulement.
 *
 * Plusieurs, et non une : la source ne dit pas comment découper une
 * désignation, et le moteur refuse de trancher à la place de l'utilisateur.
 */
export function bearingRead(designation: string): Promise<DesignationReading[]> {
  return call<DesignationReading[]>("bearing_read", { designation });
}

/** Les cas d'emploi d'un régime, avec ce que chacun donnerait. */
export function bearingOptions(
  regime: string,
  family: string,
  boreMm: string,
): Promise<MountingOption[]> {
  return call<MountingOption[]>("bearing_options", { regime, family, boreMm });
}

/**
 * Le conseil complet : la classe recommandée, et les écarts qui en découlent.
 *
 * Deux natures de source dans un seul résultat — une recommandation de
 * fabricant, puis un calcul normatif ISO 286. L'écran doit les tenir séparées.
 */
export function bearingAdvise(
  regime: string,
  condition: string,
  family: string,
  boreMm: string,
): Promise<MountingAdvice> {
  return call<MountingAdvice>("bearing_advise", { regime, condition, family, boreMm });
}

/**
 * Les classes de tolerance que l'interface peut proposer.
 *
 * Lue une fois par ecran qui en a besoin. Le moteur ne rend que les lettres
 * dont il possede les ecarts fondamentaux : proposer les autres reviendrait a
 * offrir un calcul qui echouera.
 */
export function toleranceClasses(): Promise<ClassCatalogue> {
  return call<ClassCatalogue>("tolerance_classes", {});
}

/**
 * Le catalogue des états de surface : symboles, stries, paramètres, classes N,
 * procédés et leur graphique.
 *
 * Ses `warnings` sont à lire avant toute saisie : les trois sources du domaine
 * ne sont pas encore confrontées à leur norme.
 */
export function surfaceCatalogue(): Promise<SurfaceCatalogue> {
  return call<SurfaceCatalogue>("surface_catalogue", {});
}

/** Lit une indication d'état de surface : « Ra 0.8 », « MRR Ra 1.6 ⊥ », « N7 ». */
export function surfaceRead(input: string): Promise<SurfaceAnalysis> {
  return call<SurfaceAnalysis>("surface_read", { input });
}

/** Le catalogue de la soudure : procédés, symboles, niveaux de qualité. */
export function weldingCatalogue(): Promise<WeldingCatalogue> {
  return call<WeldingCatalogue>("welding_catalogue", {});
}

/**
 * Les lectures d'un numéro de procédé ou d'un nom d'atelier.
 *
 * Plusieurs, parfois : « MAG » désigne 135, 136 et 138.
 */
export function weldingProcess(input: string): Promise<ProcessReading[]> {
  return call<ProcessReading[]>("welding_process", { input });
}

/** Lit un symbole de soudure complet. Les grandeurs restent en texte. */
export function weldingRead(request: WeldRequest): Promise<WeldReading> {
  return call<WeldReading>("welding_read", { request });
}

/** Le catalogue de la visserie : filetages, séries de trous, classes de qualité. */
export function fastenerCatalogue(): Promise<FastenerCatalogue> {
  return call<FastenerCatalogue>("fastener_catalogue", {});
}

/** Lit une désignation de filetage : « M10 », « M12 x 1.5 », « M8 8.8 ». */
export function fastenerRead(input: string): Promise<ThreadReport> {
  return call<ThreadReport>("fastener_read", { input });
}

/** Le catalogue des matières : groupes d'emploi, aciers de construction, familles. */
export function materialsCatalogue(): Promise<MaterialsCatalogue> {
  return call<MaterialsCatalogue>("materials_catalogue", {});
}

/** Décompose une désignation d'acier : « S355J2 », « 42CrMo4 », « X5CrNi18-10 ». */
export function materialsRead(input: string): Promise<SteelReading> {
  return call<SteelReading>("materials_read", { input });
}

/**
 * Ce que devient un ajustement quand la température change, alésage et arbre
 * de familles différentes. Les classes sont facultatives, mais vont par deux.
 */
export function thermalFit(request: {
  nominalMm: string;
  deltaT: string;
  holeFamily: string;
  shaftFamily: string;
  holeClass?: string | undefined;
  shaftClass?: string | undefined;
}): Promise<ThermalFit> {
  return call<ThermalFit>("thermal_fit", { ...request });
}
