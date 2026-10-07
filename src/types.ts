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
  /** La norme d'où viennent les écarts, quand ils n'ont pas été saisis. */
  source: LinkSource | null;
}

/** L'origine normative des écarts d'un maillon : `h7`, `ISO 2768-m`. */
export interface LinkSource {
  designation: string;
  provenance: Provenance;
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
  /** Les normes d'où viennent les écarts des maillons qui en citent une. */
  provenance: Provenance;
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
  conclusion: Conclusion;
  provenance: Provenance;
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

/** Une classe de tolérance générale ISO 2768-1 : `m`, « moyen », `ISO 2768-m`. */
export interface GeneralClassOption {
  symbol: string;
  name: string;
  designation: string;
}

export interface ClassCatalogue {
  hole: ClassOption[];
  shaft: ClassOption[];
  general: GeneralClassOption[];
  /** Les degrés seuls, du plus fin au plus large. */
  grades: string[];
  provenance: Provenance;
}

/* ---------- États de surface ---------- */

/** La variante du symbole : tout procédé, enlèvement exigé, enlèvement interdit. */
export type ProcessRequirement = "any" | "removal_required" | "removal_prohibited";

export interface SymbolVariant {
  id: ProcessRequirement;
  /** L'écriture en texte, faute de glyphe : `APA`, `MRR`, `NMR`. */
  code: string;
  name: string;
  meaning: string;
}

export interface LaySymbol {
  symbol: string;
  aliases: string[];
  name: string;
  meaning: string;
}

export interface ProfileParameter {
  symbol: string;
  name: string;
  definition: string;
  note: string | null;
}

/** Une classe N de l'ISO 1302:1992, retirée — lue, jamais écrite. */
export interface RoughnessGrade {
  grade: string;
  ra: Nanometres;
}

export interface RaRange {
  finest: Nanometres;
  coarsest: Nanometres;
}

/** Ordres de grandeur d'atelier, sans caractère normatif. */
export interface ProcessRoughness {
  id: string;
  name: string;
  aliases: string[];
  /** Vrai pour un usinage, faux pour une mise en forme. */
  removal: boolean;
  usual: RaRange;
  possible: RaRange;
}

/**
 * Où se situe un procédé par rapport à la valeur demandée.
 *
 * `finer` n'est pas un succès sans nuance : le procédé fait mieux sans effort,
 * donc probablement plus cher que nécessaire.
 */
export type Reach = "usual" | "finer" | "possible" | "out_of_reach";

export interface ProcessFit {
  process: ProcessRoughness;
  reach: Reach;
  reach_label: string;
  /** Pourquoi le symbole écarte ce procédé, le cas échéant. */
  excluded: string | null;
}

export interface ChartSpan {
  x: number;
  width: number;
}

export interface ChartColumn {
  grade: string;
  ra_label: string;
  x: number;
  width: number;
}

export interface ChartRow {
  id: string;
  name: string;
  removal: boolean;
  y: number;
  height: number;
  usual: ChartSpan;
  possible: ChartSpan;
  reach: Reach | null;
  excluded: boolean;
}

/** Les plages par procédé, géométrie calculée par le moteur. */
export interface RoughnessChart {
  width: number;
  height: number;
  origin: number;
  columns: ChartColumn[];
  rows: ChartRow[];
  marker: { x: number; label: string } | null;
  caption: string;
}

export type LimitKind = "upper" | "lower";

export interface SurfaceIndication {
  requirement: ProcessRequirement | null;
  parameter: string;
  value: Nanometres;
  limit: LimitKind;
  max_rule: boolean;
  lay: string | null;
  process: string | null;
  grade: string | null;
  input: string;
}

export interface SurfaceAnalysis {
  indication: SurfaceIndication;
  /** L'indication reconstituée, normalisée : `MRR Ra 0.8 ⊥`. */
  designation: string;
  requirement: SymbolVariant | null;
  parameter: ProfileParameter;
  lay: LaySymbol | null;
  grade: RoughnessGrade | null;
  /** Vide quand la confrontation n'a pas de sens — `process_note` dit pourquoi. */
  processes: ProcessFit[];
  process_note: string | null;
  stated_process: ProcessFit | null;
  findings: Finding[];
  chart: RoughnessChart;
  conclusion: Conclusion;
  provenance: Provenance;
}

export interface SurfaceCatalogue {
  symbols: SymbolVariant[];
  lays: LaySymbol[];
  parameters: ProfileParameter[];
  grades: RoughnessGrade[];
  processes: ProcessRoughness[];
  chart: RoughnessChart;
  provenance: Provenance;
  warnings: string[];
}

/* ---------- Soudure ---------- */

export interface WeldingProcess {
  number: string;
  name: string;
  aliases: string[];
}

export type JointFamily = "butt" | "fillet" | "other";

export interface SizeLetter {
  letter: string;
  name: string;
  meaning: string;
}

/**
 * Un symbole élémentaire.
 *
 * Deux natures de source dans un même objet : `number`, `name`,
 * `full_penetration` et `multi_part` viennent de l'ISO 2553:2013 lue dans la
 * norme ; `family`, `sizes` et `both_sides_name` d'une surcouche de cotation
 * non vérifiée.
 */
export interface ElementarySymbol {
  id: string;
  number: number;
  name: string;
  full_penetration: boolean;
  multi_part: boolean;
  note: string | null;
  both_sides_name: string | null;
  family: JointFamily;
  sizes: string[];
}

export interface SupplementarySymbol {
  id: string;
  number: number;
  name: string;
  meaning: string | null;
  families: JointFamily[];
}

/** Système A (double trait de référence) ou B (trait unique). */
export interface WeldSystem {
  id: string;
  name: string;
  reference_line: string;
  note: string | null;
}

export interface QualityLevel {
  id: string;
  name: string;
  meaning: string;
}

export interface QualityVariable {
  symbol: string;
  meaning: string;
}

/** La grandeur à laquelle un coefficient s'applique. `weld` vaut s ou a selon le joint. */
export type QualityBasis = "t" | "a" | "s" | "b" | "weld";

export type Limit =
  | { kind: "not_permitted" }
  | { kind: "permitted"; condition: string | null }
  | {
      kind: "bound";
      measure: string;
      constant: Nanometres;
      factor_hundredths: number;
      of: QualityBasis | null;
      max: Nanometres | null;
      short: boolean;
    }
  | { kind: "min_angle"; degrees: number };

export interface ThicknessRange {
  from: Nanometres | null;
  above: Nanometres | null;
  to: Nanometres | null;
}

export interface LimitRow {
  thickness: ThicknessRange;
  limits: { level: string; limit: Limit }[];
}

export interface Imperfection {
  reference: string;
  iso6520: string;
  name: string;
  applies_to: "butt" | "fillet" | "both";
  remark: string | null;
  rows: LimitRow[];
}

export interface ProcessScope {
  note: string;
  fusion: string[];
  excluded: { prefix: string; reason: string }[];
}

export type ScopeVerdict =
  | { kind: "in_scope" }
  | { kind: "excluded"; reason: string }
  | { kind: "unknown" };

export interface WeldingCatalogue {
  processes: WeldingProcess[];
  systems: WeldSystem[];
  /** Les règles de la norme : les deux systèmes ne se mélangent pas. */
  system_rules: string[];
  sizes: SizeLetter[];
  elementary: ElementarySymbol[];
  supplementary: SupplementarySymbol[];
  levels: QualityLevel[];
  variables: QualityVariable[];
  imperfections: Imperfection[];
  process_scope: ProcessScope;
  provenance: Provenance;
  warnings: string[];
}

export interface ProcessReading {
  process: WeldingProcess;
  /** Du groupe au procédé lui-même. */
  lineage: WeldingProcess[];
  is_group: boolean;
  children: WeldingProcess[];
  quality_scope: ScopeVerdict;
  explanation: string;
}

export type Side = "arrow" | "other" | "both";

/**
 * Une demande de lecture de symbole.
 *
 * Les grandeurs voyagent en **texte**, en millimètres : c'est le moteur qui les
 * lit. L'interface ne convertit rien.
 */
export interface WeldRequest {
  symbol: string;
  side: Side;
  size_letter: string | null;
  size_mm: string | null;
  count: string | null;
  length_mm: string | null;
  spacing_mm: string | null;
  staggered: boolean;
  supplementary: string[];
  all_around: boolean;
  field_weld: boolean;
  process: string | null;
  level: string | null;
  thickness_mm: string | null;
  width_mm: string | null;
}

export interface SizeReading {
  letter: string;
  name: string;
  value: Nanometres;
  label: string;
  /** Vrai pour une cote déduite par √2 : arrondie, et annoncée comme telle. */
  rounded: boolean;
}

export interface IntermittentReading {
  count: number;
  length: Nanometres;
  spacing: Nanometres;
  welded_length: Nanometres;
  notation: string;
}

export type LimitStatus = "not_permitted" | "permitted" | "bounded" | "min_angle" | "needs_input";

export interface ImperfectionLimit {
  reference: string;
  iso6520: string;
  name: string;
  remark: string | null;
  thickness_label: string;
  status: LimitStatus;
  /** La limite telle que la norme l'écrit. */
  formula: string;
  value: Nanometres | null;
  value_label: string | null;
  /** La grandeur qui manque pour chiffrer. */
  missing: string | null;
  rounded: boolean;
  short: boolean;
}

export interface QualityAssessment {
  level: QualityLevel;
  joint: JointFamily;
  thickness: Nanometres | null;
  weld_size: Nanometres | null;
  width: Nanometres | null;
  limits: ImperfectionLimit[];
  notes: string[];
}

export interface WeldReading {
  designation: string;
  symbol: ElementarySymbol;
  side: Side;
  /** La lecture en clair, une phrase par élément du symbole. */
  sentences: string[];
  size: SizeReading | null;
  equivalent: SizeReading | null;
  intermittent: IntermittentReading | null;
  supplementary: SupplementarySymbol[];
  process: ProcessReading | null;
  quality: QualityAssessment | null;
  findings: Finding[];
  conclusion: Conclusion;
  provenance: Provenance;
}

export const LIMIT_STATUS_LABEL: Record<LimitStatus, string> = {
  not_permitted: "Non admis",
  permitted: "Admis",
  bounded: "Borné",
  min_angle: "Angle minimal",
  needs_input: "À chiffrer",
};

/* ---------- Visserie ---------- */

export interface MetricThread {
  d: Nanometres;
  choice: number;
  coarse: Nanometres;
  fine: Nanometres[];
}

export interface ClearanceSeries {
  id: string;
  name: string;
  class: string;
}

export interface BoltClass {
  class: string;
  first: number;
  second: number;
  tensile_mpa: number;
  yield_mpa: number;
  max_d: number | null;
}

export interface FastenerCatalogue {
  threads: MetricThread[];
  clearance_series: ClearanceSeries[];
  bolt_classes: BoltClass[];
  nut_classes: number[];
  provenance: Provenance;
  warnings: string[];
}

export type PitchKind = "coarse" | "fine" | "not_listed";

export interface ThreadDesignation {
  d: Nanometres;
  pitch: Nanometres;
  pitch_kind: PitchKind;
  explicit_pitch: boolean;
  tolerance_class: string | null;
  strength_class: string | null;
  input: string;
}

export interface ThreadDimension {
  symbol: string;
  name: string;
  value: Nanometres;
  label: string;
  formula: string;
}

/**
 * Un trou de passage. Le diamètre et la classe viennent de l'ISO 273 ; les
 * écarts, du moteur ISO 286 — deux natures de source dans une même ligne.
 */
export interface ClearanceHole {
  series: string;
  series_name: string;
  diameter: Nanometres;
  class: string;
  designation: string;
  tolerance: FeatureAnalysis | null;
  unavailable: string | null;
}

export interface StrengthReading {
  class: BoltClass;
  nut_class: number | null;
  explanation: string;
}

export interface ThreadReport {
  designation: ThreadDesignation;
  normalised: string;
  thread: MetricThread;
  h: ThreadDimension;
  dimensions: ThreadDimension[];
  stress_area_hundredths_mm2: number;
  stress_area_label: string;
  tap_drill: Nanometres;
  tap_drill_label: string;
  clearance_holes: ClearanceHole[];
  strength: StrengthReading | null;
  findings: Finding[];
  conclusion: Conclusion;
  provenance: Provenance;
}

/* ---------- Matières ---------- */

export type GroupNumber = "yield" | "tensile" | "hardness";

export interface UseGroup {
  letter: string;
  name: string;
  number: GroupNumber;
  note: string;
}

export interface ThicknessBand {
  above: Nanometres;
  to: Nanometres;
}

export interface GradeRow {
  band: ThicknessBand;
  yield_mpa: number;
  tensile_min_mpa: number;
  tensile_max_mpa: number;
}

export interface StructuralGrade {
  grade: string;
  qualities: string[];
  rows: GradeRow[];
}

/** Ordres de grandeur d'une famille, pas les valeurs d'une nuance. */
export interface MaterialFamily {
  id: string;
  name: string;
  e_gpa: number;
  /** En millièmes : 300 pour 0,30. */
  poisson_milli: number;
  /** En kg/m³. */
  density: number;
  /** En dixièmes de µm/(m·K) : 120 pour 12. */
  alpha_tenths: number;
}

export interface MaterialsCatalogue {
  use_groups: UseGroup[];
  structural_grades: StructuralGrade[];
  families: MaterialFamily[];
  provenance: Provenance;
  warnings: string[];
}

export type SteelKind =
  | "use_group"
  | "non_alloy"
  | "low_alloy"
  | "high_alloy"
  | "high_speed"
  | "numeric";

export interface DesignationPart {
  text: string;
  meaning: string;
}

export interface ElementContent {
  element: string;
  /** En millièmes de pour cent. Nul quand la désignation ne donne pas la teneur. */
  thousandths_percent: number | null;
  label: string;
}

export interface ImpactReading {
  code: string;
  joules: number;
  celsius: number;
  label: string;
}

export interface Suffix {
  code: string;
  meaning: string;
}

export interface SteelReading {
  input: string;
  kind: SteelKind;
  kind_label: string;
  cast: boolean;
  parts: DesignationPart[];
  group: UseGroup | null;
  group_value: number | null;
  group_value_label: string | null;
  impact: ImpactReading | null;
  suffixes: Suffix[];
  carbon: ElementContent | null;
  elements: ElementContent[];
  structural: StructuralGrade | null;
  family: MaterialFamily | null;
  family_reason: string;
  findings: Finding[];
  conclusion: Conclusion;
  provenance: Provenance;
}

export interface ThermalFitLimits {
  designation: string;
  cold_min: Nanometres;
  cold_max: Nanometres;
  cold_kind: FitKind;
  hot_min: Nanometres;
  hot_max: Nanometres;
  hot_kind: FitKind;
}

export interface ThermalFit {
  nominal: Nanometres;
  delta_t: number;
  hole_family: MaterialFamily;
  shaft_family: MaterialFamily;
  hole_growth: Nanometres;
  shaft_growth: Nanometres;
  clearance_shift: Nanometres;
  clearance_shift_label: string;
  fit: ThermalFitLimits | null;
  findings: Finding[];
  conclusion: Conclusion;
  provenance: Provenance;
}
