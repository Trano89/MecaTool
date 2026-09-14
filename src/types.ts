/**
 * Types de la frontière Tauri.
 *
 * Ils décrivent exactement ce que sérialise le moteur Rust, en `snake_case`.
 * Aucune conversion n'a lieu ici : toutes les valeurs arrivent déjà calculées et
 * déjà mises en forme.
 *
 * ## Comment la dérive est empêchée
 *
 * Un test Rust (`exporte_les_echantillons_pour_typescript`) écrit un échantillon
 * de chaque rapport dans `src/fixtures/`. `src/types.test.ts` les relit en les
 * typant avec les types ci-dessous. Si un champ est renommé, supprimé ou change
 * de type côté Rust, `tsc` échoue sur l'échantillon. Les deux côtés ne peuvent
 * donc pas diverger silencieusement.
 */

/**
 * Une longueur, en **nanomètres entiers**.
 *
 * Le moteur ne transmet jamais de millimètres flottants : `10.015 mm` arrive
 * comme `10015000`. Les valeurs prêtes à afficher voyagent à côté, sous forme
 * de chaînes (`upper_label`, `it_label`, …), déjà formatées par le moteur.
 */
export type Nanometres = number;

export const NM_PER_MM = 1_000_000;
export const NM_PER_UM = 1_000;

export type Feature = "hole" | "shaft";

export type FitKind = "clearance" | "transition" | "interference";

export type Verdict =
  | "compatible"
  | "caution"
  | "incompatible"
  | "insufficient_data";

export interface ToleranceClass {
  feature: Feature;
  /** Lettre en majuscule, quelle que soit la casse d'origine : `"H"`, `"G"`, `"JS"`. */
  letter: string;
  /** Degré normalisé : `"IT7"`. */
  grade: string;
}

export interface Deviations {
  lower: Nanometres;
  upper: Nanometres;
}

export interface LimitsOfSize {
  min: Nanometres;
  max: Nanometres;
}

export interface SizeRange {
  above: Nanometres;
  up_to: Nanometres;
}

export interface FeatureTolerance {
  feature: Feature;
  class: ToleranceClass;
  nominal: Nanometres;
  deviations: Deviations;
  limits: LimitsOfSize;
  it: Nanometres;
  size_range: SizeRange;
  fundamental_deviation: Nanometres;
}

export interface Fit {
  hole: FeatureTolerance;
  shaft: FeatureTolerance;
  /** `EI − es`. Négatif si l'ajustement peut serrer. */
  min_clearance: Nanometres;
  /** `ES − ei`. Négatif si l'ajustement serre toujours. */
  max_clearance: Nanometres;
  kind: FitKind;
}

export interface ReasoningStep {
  label: string;
  expression?: string;
  value?: string;
}

export interface Conclusion {
  verdict: Verdict;
  detail: string;
  why: ReasoningStep[];
  warnings: string[];
}

/**
 * Quatre états, pas deux.
 *
 * Entre « confronté à la norme » et « pas encore vérifié » il existe deux cas
 * intermédiaires, et les confondre tromperait dans les deux sens :
 *
 * - `secondary` — la donnée vient d'un recueil technique qui reproduit la
 *   norme, lu avec soin, mais qui n'est pas la norme.
 * - `recommended` — la donnée n'est pas normative du tout : c'est une pratique
 *   que des fabricants recommandent. La présenter comme une norme durcirait une
 *   recommandation ; la présenter comme une donnée douteuse banaliserait une
 *   pratique établie.
 *
 * Aucun des deux ne compte comme vérifié à l'affichage.
 */
export type VerificationStatus =
  | { state: "verified"; against: string; on: string }
  | { state: "secondary"; from: string; reproduces: string; on: string }
  | { state: "recommended"; by: string; on: string }
  | { state: "unverified"; pending: string };

export interface StandardReference {
  id: string;
  edition: string;
  title: string;
  scope: string | null;
  source: string;
  verification: VerificationStatus;
  notes: string[];
}

export interface Provenance {
  references: StandardReference[];
}

/* ---------- Diagramme ---------- */

export type DiagramMode = "deviations" | "true_to_scale";

export type ClearanceKind = "minimum" | "maximum";

export interface Band {
  label: string;
  /**
   * L'ajustement auquel cette zone appartient, dans un comparatif.
   *
   * Nul lorsque le diagramme n'en montre qu'un : le titre suffit alors.
   */
  group: string | null;
  feature: Feature;
  x: number;
  width: number;
  /** Bord supérieur, correspondant à l'écart supérieur. */
  top: number;
  /** Bord inférieur, correspondant à l'écart inférieur. */
  bottom: number;
  upper_deviation: Nanometres;
  lower_deviation: Nanometres;
  upper_label: string;
  lower_label: string;
  it_label: string;
}

export interface ClearanceMarker {
  kind: ClearanceKind;
  hole_y: number;
  shaft_y: number;
  value: Nanometres;
  label: string;
  is_interference: boolean;
}

export interface AxisTick {
  y: number;
  deviation: Nanometres;
  label: string;
  is_zero: boolean;
}

export interface Diagram {
  width: number;
  height: number;
  mode: DiagramMode;
  zero_line_y: number;
  pixels_per_micrometre: number;
  bands: Band[];
  clearances: ClearanceMarker[];
  axis_ticks: AxisTick[];
  /** Annonce d'échelle. Jamais vide, jamais à masquer. */
  scale_note: string;
  title: string;
}

/* ---------- Analyses ---------- */

export interface FeatureAnalysis {
  tolerance: FeatureTolerance;
  steps: ReasoningStep[];
  provenance: Provenance;
}

export interface FitAnalysis {
  fit: Fit;
  hole_steps: ReasoningStep[];
  shaft_steps: ReasoningStep[];
  fit_steps: ReasoningStep[];
  provenance: Provenance;
}

export interface Margins {
  /** `jeu minimal calculé − jeu minimal demandé`. Négatif = dépassement. */
  lower: Nanometres | null;
  /** `jeu maximal demandé − jeu maximal calculé`. Négatif = dépassement. */
  upper: Nanometres | null;
}

export interface Verification {
  verdict: Verdict;
  margins: Margins;
  conclusion: Conclusion;
}

export interface ClearanceRequirement {
  nominal: Nanometres;
  min_clearance: Nanometres | null;
  max_clearance: Nanometres | null;
}

export type Basis = "Hole" | "Shaft";

export interface Solution {
  fit: Fit;
  verification: Verification;
  /** `IT(alésage) + IT(arbre)` : la dispersion à tenir en fabrication. */
  total_tolerance: Nanometres;
  basis: Basis;
}

export interface SearchResult {
  solutions: Solution[];
  examined: number;
  narrowest_span: Nanometres | null;
  provenance: Provenance;
  /** Périmètre exploré. À afficher : une absence de résultat n'est pas une impossibilité. */
  notes: string[];
}

/* ---------- Rapports ---------- */

export interface FitReport {
  kind: "fit";
  analysis: FitAnalysis;
  diagram: Diagram;
  classification: string;
  conclusion: Conclusion;
  verification: Verification | null;
}

export interface FeatureReport {
  kind: "feature";
  analysis: FeatureAnalysis;
}

export interface SearchReport {
  kind: "search";
  result: SearchResult;
  requirement: ClearanceRequirement;
  diagnosis: string | null;
}

export type Report = FitReport | FeatureReport | SearchReport;

/* ---------- Chaînes de cotes ---------- */

export type LinkDirection = "increasing" | "decreasing";

export interface Link {
  /** Repère porté sur le plan : `A`, `B`, `L1`… */
  label: string;
  nominal: Nanometres;
  deviations: Deviations;
  direction: LinkDirection;
}

export interface Contribution {
  link: Link;
  tolerance: Nanometres;
  /** Part de la tolérance résultante, en millièmes. Entier, donc exact. */
  share_per_mille: number;
  share_label: string;
  designation: string;
}

/**
 * L'estimation statistique, quand elle est demandée.
 *
 * Elle repose sur des hypothèses que la géométrie ne garantit pas. Elles
 * voyagent avec le résultat et ne doivent jamais être affichées séparément.
 */
export interface StatisticalEstimate {
  method: string;
  tolerance: Nanometres;
  share_of_worst_case_per_mille: number;
  summary: string;
  assumptions: string[];
}

export interface ChainAnalysis {
  contributions: Contribution[];
  nominal: Nanometres;
  /** Limites au pire des cas. */
  limits: LimitsOfSize;
  /** Tolérance résultante : la somme de toutes les tolérances, quel que soit leur sens. */
  tolerance: Nanometres;
  deviations: Deviations;
  statistical: StatisticalEstimate | null;
  steps: ReasoningStep[];
  /** Le maillon qui pèse le plus lourd. */
  dominant: string | null;
}

export interface ContributionBar {
  label: string;
  designation: string;
  direction: LinkDirection;
  y: number;
  height: number;
  /** Largeur proportionnelle à la part du maillon. */
  width: number;
  share_label: string;
  tolerance_label: string;
  dominant: boolean;
}

export interface ContributionChart {
  width: number;
  height: number;
  /** Abscisse où commencent les barres, après la colonne des repères. */
  bar_origin: number;
  bars: ContributionBar[];
  caption: string;
}

export interface ChainReport {
  analysis: ChainAnalysis;
  chart: ContributionChart;
  designation: string;
  conclusion: Conclusion;
}

/* ---------- Comparaison d'ajustements ---------- */

export interface ComparedFit {
  fit: Fit;
  designation: string;
  /** Type d'ajustement, pastille comprise. */
  classification: string;
  /** `IT(alésage) + IT(arbre)` : la dispersion à tenir en fabrication. */
  span: Nanometres;
  /** Présent seulement si une exigence de jeu a été fournie. */
  verification: Verification | null;
}

export interface FitComparison {
  nominal: Nanometres;
  /** Dans l'ordre de la saisie — l'interface ne reclasse pas. */
  entries: ComparedFit[];
  /** Toutes les zones sur une échelle unique. */
  diagram: Diagram;
  requirement: ClearanceRequirement | null;
  provenance: Provenance;
  /** Ce que le comparatif fait ressortir, en une phrase. */
  summary: string;
}

/* ---------- Tolérances générales (ISO 2768-1) ---------- */

/**
 * Un angle, en **millisecondes d'arc entières**.
 *
 * `0°20′` vaut un tiers de degré : inexprimable en degrés décimaux, exact ici
 * (`1 200 000`). Les libellés prêts à afficher voyagent à côté, déjà formatés
 * par le moteur — l'interface n'a jamais à convertir un angle.
 */
export type Milliarcseconds = number;

export type GeneralClass = "fine" | "medium" | "coarse" | "very_coarse";

export type MeasureKind = "linear" | "broken_edge" | "angular";

export type GeneralDeviation =
  | { kind: "linear"; magnitude: Nanometres }
  | { kind: "angular"; magnitude: Milliarcseconds };

export interface ClassRow {
  class: GeneralClass;
  /** Symbole porté sur le dessin : `f`, `m`, `c`, `v`. */
  symbol: string;
  name: string;
  designation: string;
  /** Absent lorsque la norme ne définit rien pour cette combinaison. */
  deviation: GeneralDeviation | null;
  deviation_label: string | null;
  limits: LimitsOfSize | null;
  /** Pourquoi la norme ne définit rien. Présent si et seulement si `deviation` est nul. */
  unavailable: string | null;
}

export interface ClassComparison {
  kind: MeasureKind;
  nominal: Nanometres;
  /** Une entrée par classe, du plus fin au plus grossier. */
  rows: ClassRow[];
  provenance: Provenance;
}

export const MEASURE_KIND_LABEL: Record<MeasureKind, string> = {
  linear: "Dimension linéaire",
  broken_edge: "Arête abattue",
  angular: "Dimension angulaire",
};

export interface EngineInfo {
  app_version: string;
  available_letters: string[];
  max_nominal_mm: string;
  provenance: Provenance;
  warnings: string[];
}

/** Ce que le moteur renvoie quand il refuse de conclure. */
export interface AppError {
  message: string;
  hint: string | null;
}

/* ---------- Aides d'affichage ---------- */

/** Le libellé d'un verdict, tel que le moteur l'écrit. */
export const VERDICT_LABEL: Record<Verdict, string> = {
  compatible: "COMPATIBLE",
  caution: "ATTENTION",
  incompatible: "NON COMPATIBLE",
  insufficient_data: "INFORMATIONS INSUFFISANTES",
};

/**
 * La pastille d'un verdict.
 *
 * Toujours accompagnée de son libellé : la couleur seule ne doit jamais porter
 * l'information.
 */
export const VERDICT_BADGE: Record<Verdict, string> = {
  compatible: "🟢",
  caution: "🟠",
  incompatible: "🔴",
  insufficient_data: "🔵",
};

/** Classe CSS d'un verdict, alignée sur les noms des tokens de statut. */
export const VERDICT_CLASS: Record<Verdict, string> = {
  compatible: "compatible",
  caution: "caution",
  incompatible: "incompatible",
  insufficient_data: "insufficient-data",
};

/* ---------- Tolérancement géométrique (ISO 1101) ---------- */

/** Les quatre familles de tolérances géométriques. */
export type ToleranceFamily = "form" | "orientation" | "location" | "runout";

/** Ce que la caractéristique attend en matière de référence spécifiée. */
export type DatumRule = "none" | "required" | "optional";

/** Géométrie de la zone : c'est elle qui décide de la présence du « ø ». */
export type ZoneGeometry =
  | "two_parallel_straight_lines"
  | "two_parallel_planes"
  | "two_concentric_circles"
  | "two_coaxial_cylinders"
  | "two_equidistant_lines"
  | "two_equidistant_surfaces"
  | "cylinder"
  | "measured_per_section";

export type Severity = "error" | "caution" | "note";

export interface ZoneDefinition {
  feature: string;
  geometry: ZoneGeometry;
  /** La phrase d'interprétation, reprise au plus près de la source. */
  definition: string;
}

export interface Characteristic {
  id: string;
  name: string;
  symbol: string;
  aliases: string[];
  /** Le nom du champ vient de la source : c'est bien la famille. */
  class: ToleranceFamily;
  datum: DatumRule;
  requires_ted: boolean;
  zones: ZoneDefinition[];
  /** Page de la source où la caractéristique est définie. */
  page: number;
}

export interface FamilyDefinition {
  id: ToleranceFamily;
  name: string;
  heading: string;
  /** Les familles que celle-ci borne aussi. */
  limits: ToleranceFamily[];
  /** La phrase de la source qui énonce cet emboîtement. */
  note: string;
}

export interface Modifier {
  symbol: string;
  name: string;
  family: string;
  /** Toutes ne viennent pas de l'ISO 1101. */
  defined_by: string;
  page: number;
}

export interface GeometricCatalogue {
  families: FamilyDefinition[];
  characteristics: Characteristic[];
  modifiers: Modifier[];
  provenance: Provenance;
  warnings: string[];
}

export interface GeometricSpec {
  characteristic: string;
  value: Nanometres;
  diametral: boolean;
  datums: string[];
  modifiers: string[];
  input: string;
}

export interface Finding {
  code: string;
  severity: Severity;
  message: string;
}

export interface SpecAnalysis {
  spec: GeometricSpec;
  characteristic: Characteristic;
  /** Le cadre reconstitué, normalisé. */
  designation: string;
  findings: Finding[];
  conclusion: Conclusion;
  provenance: Provenance;
}

export interface Overlap {
  wider: string;
  narrower: string;
  finding: Finding;
}

export interface GroupAnalysis {
  specs: SpecAnalysis[];
  overlaps: Overlap[];
  conclusion: Conclusion;
  provenance: Provenance;
}

/**
 * La pastille d'un constat.
 *
 * Comme pour les verdicts, elle accompagne toujours un libellé : la couleur
 * seule ne doit jamais porter l'information.
 */
export const SEVERITY_BADGE: Record<Severity, string> = {
  error: "🔴",
  caution: "🟠",
  note: "🔵",
};

export const SEVERITY_LABEL: Record<Severity, string> = {
  error: "À corriger",
  caution: "À vérifier",
  note: "Pour information",
};

export const SEVERITY_CLASS: Record<Severity, string> = {
  error: "incompatible",
  caution: "caution",
  note: "insufficient-data",
};

/* ---------- Registre des domaines ---------- */

/**
 * L'état d'un domaine.
 *
 * `blocked` ne veut pas dire « caché ». Un domaine prévu dont la source n'est
 * pas exploitable reste visible et désactivé, avec `unavailable` en toutes
 * lettres : masquer la feuille de route ferait chercher en vain une fonction
 * qui n'existe pas, et laisserait croire qu'elle n'est pas prévue.
 */
export type DomainStatus = "ready" | "reserved" | "blocked";

/** Le groupe de navigation auquel un domaine appartient. */
export type DomainGroup = "dimensional" | "geometry" | "components" | "materials";

/**
 * Un domaine du registre.
 *
 * C'est le moteur qui décide ce que l'application sait faire. La navigation,
 * l'accueil et la palette de commandes se construisent à partir de cette liste
 * et d'elle seule : ajouter un domaine ne doit toucher aucun composant.
 */
export interface Domain {
  /** Identifiant stable, celui que l'interface emploie pour router. */
  id: string;
  /** Le nom court, celui de la navigation. */
  name: string;
  /** La question à laquelle le domaine répond. C'est elle qui aide à choisir. */
  question: string;
  group: DomainGroup;
  /**
   * Le libellé du groupe, déjà traduit par le moteur.
   *
   * Redondant avec `group` en apparence, mais c'est ce qui évite que
   * l'interface tienne sa propre table de traduction : un libellé est un texte
   * métier, et le frontend n'en écrit aucun.
   */
  group_label: string;
  status: DomainStatus;
  /** Les normes ou sources mobilisées, citation courte. */
  sources: string[];
  /** La réserve à afficher, quand il y en a une. */
  reserve: string | null;
  /** Pourquoi le domaine n'est pas disponible, le cas échéant. */
  unavailable: string | null;
  /** Exemples de saisie, pour la recherche et l'accueil. */
  examples: string[];
}

/* ---------- Roulements ---------- */

export interface BearingFamily {
  id: string;
  name: string;
  note: string | null;
}

/** Le régime de charge de la bague intérieure. */
export interface LoadRegime {
  id: string;
  name: string;
  /** Pourquoi ce régime impose ce qu'il impose. */
  explanation: string;
}

export interface DiameterRange {
  from: number;
  to: number | null;
}

/** Une ligne du tableau : une classe, et les échelons où elle s'applique. */
export interface MountingRow {
  class: string;
  /** Vrai quand la ligne vaut quel que soit le diamètre. */
  all_diameters: boolean;
  /**
   * Échelon par famille de roulement. Une famille absente signifie que la
   * source ne définit rien pour elle sur cette ligne.
   */
  ranges: Record<string, DiameterRange>;
}

/** Un cas d'emploi : un régime, une condition, et ses lignes. */
export interface MountingCase {
  regime: string;
  condition: string;
  examples: string;
  rows: MountingRow[];
}

/** Ce qu'il faut pour peupler l'écran des roulements avant toute saisie. */
export interface BearingCatalogue {
  families: BearingFamily[];
  regimes: LoadRegime[];
  cases: MountingCase[];
  provenance: Provenance;
  /** La réserve de source, dont celle qui dit que le tableau n'est pas normatif. */
  warnings: string[];
}

/**
 * Une lecture possible d'une désignation de roulement.
 *
 * Plusieurs, parce que la source ne dit pas comment découper une désignation :
 * « 6203 » se lit série 62 + symbole 03, « 623 » se lit série 62 + symbole 3, et
 * rien ne permet de trancher mécaniquement. Le moteur rend donc les lectures
 * possibles au lieu d'en choisir une.
 */
export interface DesignationReading {
  designation: string;
  /** Ce qui précède le symbole d'alésage : série, type, suffixes de tête. */
  series: string;
  bore_code: string;
  bore: Nanometres;
  explanation: string;
}

/** Un cas d'emploi proposé à l'utilisateur, avec ce qu'il donnerait. */
export interface MountingOption {
  regime: string;
  condition: string;
  examples: string;
  /** La classe, ou `null` avec la raison de son absence. */
  class: string | null;
  unavailable: string | null;
}

/**
 * Le conseil de portée d'arbre, écarts compris.
 *
 * Le résultat croise deux natures de source, et l'écran doit les tenir
 * séparées : `class` est une **recommandation de fabricant** — aucune norme ne
 * l'impose — tandis que `shaft`, les écarts qui en découlent, est **normatif**
 * (ISO 286). `conclusion.why` pose les deux étapes côte à côte.
 */
export interface MountingAdvice {
  family: BearingFamily;
  regime: LoadRegime;
  condition: string;
  examples: string;
  bore: Nanometres;
  /** La classe recommandée, par ex. `"k5"`. */
  class: string;
  /** La cote telle qu'elle s'inscrirait sur le plan, par ex. `"Ø50 k5"`. */
  designation: string;
  /** Les écarts réels, calculés par le moteur ISO 286. */
  shaft: FeatureAnalysis;
  /** La tolérance propre de l'alésage du roulement, selon l'ISO 492. */
  bearing_bore: RingTolerance;
  /** L'ajustement qui résulte des deux. */
  fit: BearingFit;
  conclusion: Conclusion;
  provenance: Provenance;
}

/** La bague concernée par une tolérance ISO 492. */
export type Ring = "inner" | "outer";

/** La tolérance normalisée d'une bague de roulement. */
export interface RingTolerance {
  ring: Ring;
  nominal: Nanometres;
  deviations: Deviations;
  /** Le symbole de la caractéristique, par ex. `"Δdmp"`. */
  characteristic: string;
  /** Ce que ce symbole désigne, en toutes lettres. */
  meaning: string;
  /** L'échelon de la table d'où sort cette tolérance. */
  range_label: string;
  /** La classe de tolérance du roulement, par ex. `"Normale"`. */
  tolerance_class: string;
}

/**
 * L'ajustement entre l'arbre et l'alésage du roulement.
 *
 * La grandeur portée est le **jeu** signé, dont le négatif est un serrage —
 * même convention que `Fit`, et pour la même raison : deux conventions opposées
 * dans le même dépôt seraient une source d'erreurs de signe.
 *
 * Le texte à afficher est `summary`, écrit par le moteur : l'écran n'a pas à
 * décider si « −2 µm de jeu » se dit « 2 µm de serrage ».
 */
export interface BearingFit {
  /** `EI − es` : l'alésage au plus petit, l'arbre au plus grand. */
  min_clearance: Nanometres;
  /** `ES − ei` : l'alésage au plus grand, l'arbre au plus petit. */
  max_clearance: Nanometres;
  kind: FitKind;
  /**
   * L'ajustement en toutes lettres, rédigé par le moteur.
   *
   * Cet écran ne recompose pas la phrase à partir des signes : il l'affiche.
   * La rédiger ici aussi reviendrait à tenir la même règle en deux langues.
   */
  summary: string;
}

/* ---------- État d'une source ---------- */

/**
 * L'état d'une source, en toutes lettres.
 *
 * Comme les verdicts, il ne doit jamais être porté par la seule couleur : ces
 * quatre libellés accompagnent toujours la pastille correspondante.
 */
export const VERIFICATION_LABEL: Record<VerificationStatus["state"], string> = {
  verified: "Vérifié sur la norme",
  secondary: "Source secondaire",
  recommended: "Pratique recommandée",
  unverified: "Non vérifié",
};

/**
 * La phrase qui dit *comment* la source a été établie.
 *
 * Elle se compose des champs que le moteur transmet — jamais d'un texte écrit
 * ici : ce sont ses mots, avec sa date.
 */
export function VERIFICATION_DETAIL(status: VerificationStatus): string {
  switch (status.state) {
    case "verified":
      return `Confronté à ${status.against} — le ${status.on}.`;
    case "secondary":
      return `Transcrit de ${status.from}, qui reproduit ${status.reproduces} — le ${status.on}.`;
    case "recommended":
      return `Recommandé par ${status.by} — relevé le ${status.on}. Aucune norme ne l'impose.`;
    case "unverified":
      return `Reste à faire : ${status.pending}`;
  }
}

/* ---------- Catalogue des classes de tolerance ---------- */

/**
 * Une classe que l'interface a le droit de proposer.
 *
 * ⚠ Cette liste vient du moteur, et **ne doit jamais être écrite dans
 * l'interface**. Les lettres disponibles ne sont pas une décision d'affichage :
 * ce sont celles dont MecaTool possède les écarts fondamentaux — dix sur
 * vingt-huit. Une liste rédigée ici proposerait des classes que le moteur
 * refuserait ensuite de calculer, et l'utilisateur ne comprendrait pas pourquoi.
 */
export interface ClassOption {
  /** La désignation telle qu'elle s'écrit sur un plan : `"H7"`, `"g6"`. */
  designation: string;
  /** La lettre, dans la casse de l'élément. */
  letter: string;
  /** Le degré, par ex. `"IT7"`. */
  grade: string;
}

export interface ClassCatalogue {
  hole: ClassOption[];
  shaft: ClassOption[];
  /** Les degrés seuls, du plus fin au plus large. */
  grades: string[];
  provenance: Provenance;
}
