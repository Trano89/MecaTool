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
 * Trois états, pas deux.
 *
 * Entre « confronté à la norme » et « pas encore vérifié » il existe un cas
 * intermédiaire : la donnée vient d'un recueil technique qui reproduit la norme,
 * lu avec soin, mais qui n'est pas la norme. `secondary` le nomme, et compte
 * comme non vérifié à l'affichage.
 */
export type VerificationStatus =
  | { state: "verified"; against: string; on: string }
  | { state: "secondary"; from: string; reproduces: string; on: string }
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
