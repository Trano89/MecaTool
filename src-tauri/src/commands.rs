//! Commandes exposees a l'interface.
//!
//! Cette couche ne calcule rien. Elle lit l'entree, appelle le moteur, et rend
//! le resultat tel quel. Toute regle normative, toute conversion d'unite, toute
//! decision de geometrie reste en amont : le frontend recoit des valeurs deja
//! calculees et deja mises en forme.
//!
//! Les champs traversent la frontiere en `snake_case`, exactement comme en
//! Rust. Renommer a la volee ferait diverger silencieusement les deux cotes le
//! jour ou un champ change de nom.

use mecatool_core::{Conclusion, DeviationLetter, Length, Provenance, Unit};
use mecatool_engine::bearing::{BearingEngine, DesignationReading, MountingAdvice, MountingOption};
use mecatool_engine::chain::{
    analyse_chain, contribution_chart, verify_chain, ChainAnalysis, ContributionChart,
};
use mecatool_engine::compare::{compare_fits, FitComparison};
use mecatool_engine::diagram::{fit_diagram, Diagram, DiagramMode, DiagramOptions};
use mecatool_engine::domain::Domain;
use mecatool_engine::fasteners::{FastenerEngine, ThreadReport};
use mecatool_engine::geometric::{GeometricEngine, GroupAnalysis};
use mecatool_engine::iso2768::{ClassComparison, Iso2768Engine};
use mecatool_engine::iso286::{
    classification_conclusion, FeatureAnalysis, FitAnalysis, Iso286Engine,
};
use mecatool_engine::materials::{MaterialsEngine, SteelReading, ThermalFit};
use mecatool_engine::parser::{
    parse, parse_chain, parse_clearance_window, parse_comparison, ParsedInput,
};
use mecatool_engine::requirement::{verify_clearance, ClearanceRequirement, Verification};
use mecatool_engine::search::{find_fits, SearchOptions, SearchResult};
use mecatool_engine::surface::{RoughnessChart, SurfaceAnalysis, SurfaceEngine};
use mecatool_engine::welding::{ProcessReading, WeldReading, WeldRequest, WeldingEngine};
use mecatool_engine::EngineError;
use mecatool_standards::iso2768::MeasureKind;
use mecatool_standards::matieres::{MaterialFamily, StructuralGrade, UseGroup};
use mecatool_standards::roulements::{BearingFamily, LoadRegime, MountingCase};
use mecatool_standards::soudure::{
    ElementarySymbol, Imperfection, ProcessScope, QualityLevel, QualityVariable, SizeLetter,
    SupplementarySymbol, WeldSystem, WeldingProcess,
};
use mecatool_standards::surface::{
    LaySymbol, ProcessRoughness, ProfileParameter, RoughnessGrade, SymbolVariant,
};
use mecatool_standards::visserie::{BoltClass, ClearanceSeries, MetricThread};
use mecatool_standards::{Characteristic, FamilyDefinition, Modifier};
use serde::{Deserialize, Serialize};

/// Une erreur telle que l'interface doit la presenter.
///
/// Le message dit ce qui ne va pas ; la piste, quand elle existe, dit quoi
/// faire. Les deux viennent du moteur, jamais d'une reformulation ici.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppError {
    pub message: String,
    pub hint: Option<String>,
}

impl From<EngineError> for AppError {
    fn from(error: EngineError) -> Self {
        // Les variantes qui portent deja une piste d'action la transmettent
        // separement, pour que l'interface puisse la mettre en valeur.
        let hint = match &error {
            EngineError::Unparsable { hint, .. } => Some(hint.clone()),
            EngineError::Ambiguous { question, .. } => Some(question.clone()),
            _ => None,
        };
        let message = match &error {
            EngineError::Unparsable { input, .. } => format!("Entrée illisible : « {input} »"),
            EngineError::Ambiguous { input, .. } => format!("Entrée ambiguë : « {input} »"),
            other => other.to_string(),
        };
        AppError { message, hint }
    }
}

impl AppError {
    fn simple(message: impl Into<String>) -> Self {
        AppError {
            message: message.into(),
            hint: None,
        }
    }
}

/// Le resultat d'un ajustement, avec son dessin et sa conclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitReport {
    pub analysis: FitAnalysis,
    pub diagram: Diagram,
    /// Enonce du type d'ajustement, pastille comprise.
    pub classification: String,
    /// Ce que produit l'ajustement, sans jugement d'adequation.
    pub conclusion: Conclusion,
    /// Le verdict, present seulement si une exigence a ete fournie.
    pub verification: Option<Verification>,
}

/// Le resultat du tolerancement d'un seul element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureReport {
    pub analysis: FeatureAnalysis,
}

/// Le resultat d'une recherche de solutions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchReport {
    pub result: SearchResult,
    pub requirement: ClearanceRequirement,
    /// Pourquoi aucune solution ne convient entierement, le cas echeant.
    pub diagnosis: Option<String>,
}

/// Ce que le moteur a compris de la demande.
///
/// Les variantes ont des tailles tres differentes, ce que clippy signale. Les
/// mettre derriere un `Box` supprimerait l'avertissement au prix d'une
/// indirection sans contrepartie : un `Report` est construit une fois par appel,
/// deplace une fois, serialise, puis detruit. Quelques centaines d'octets de
/// pile ne pesent rien face a la serialisation qui suit.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Report {
    Fit(FitReport),
    Feature(FeatureReport),
    Search(SearchReport),
}

/// Etat du moteur et de ses donnees, affiche au demarrage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub app_version: String,
    /// Lettres d'ecart fondamental effectivement disponibles.
    pub available_letters: Vec<String>,
    /// Plus grande dimension nominale couverte, en millimetres.
    pub max_nominal_mm: String,
    pub provenance: Provenance,
    /// Avertissements de non-verification, a afficher en permanence.
    pub warnings: Vec<String>,
}

fn engine() -> Result<Iso286Engine, AppError> {
    Iso286Engine::new().map_err(AppError::from)
}

/// Les tolerances generales d'une cote, classe par classe.
///
/// `kind` vaut `linear`, `broken_edge` ou `angular` ; `nominal_mm` est la cote,
/// ou pour une cote angulaire la longueur du cote le plus court de l'angle.
#[tauri::command]
pub fn general_tolerances(kind: String, nominal_mm: String) -> Result<ClassComparison, AppError> {
    let kind = match kind.as_str() {
        "linear" => MeasureKind::Linear,
        "broken_edge" => MeasureKind::BrokenEdge,
        "angular" => MeasureKind::Angular,
        other => {
            return Err(AppError {
                message: format!("Type de cote inconnu : « {other} »."),
                hint: Some("Attendu « linear », « broken_edge » ou « angular ».".to_string()),
            })
        }
    };

    let nominal =
        Length::parse(nominal_mm.trim(), Unit::Millimetre).map_err(|source| AppError {
            message: format!("Dimension illisible : « {nominal_mm} »."),
            hint: Some(source.to_string()),
        })?;

    let engine = Iso2768Engine::new()?;
    Ok(engine.across_classes(kind, nominal))
}

/// Le catalogue des caracteristiques geometriques.
///
/// Il est rendu tel quel, sans mise en forme, parce que le frontend ne doit
/// porter aucune regle normative : les symboles, les familles, l'exigence de
/// reference et la forme des zones viennent toutes du fichier de donnees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometricCatalogue {
    pub families: Vec<FamilyDefinition>,
    pub characteristics: Vec<Characteristic>,
    pub modifiers: Vec<Modifier>,
    pub provenance: Provenance,
    /// La reserve de source, a afficher des l'ouverture de l'ecran.
    ///
    /// Elle ne rejoint pas le bandeau global : la donnee ISO 1101 est
    /// secondaire, mais l'ISO 286 et l'ISO 2768 ne le sont pas. Une reserve
    /// affichee partout finirait par ne plus rien vouloir dire nulle part.
    pub warnings: Vec<String>,
}

/// Le catalogue complet, pour consultation.
#[tauri::command]
pub fn geometric_catalogue() -> Result<GeometricCatalogue, AppError> {
    let engine = GeometricEngine::new()?;
    let table = engine.table();
    let provenance = Provenance::new().with(table.standard().clone());
    Ok(GeometricCatalogue {
        families: table.families().to_vec(),
        characteristics: table.characteristics().to_vec(),
        modifiers: table.modifiers().to_vec(),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

/// Lit et controle une ou plusieurs specifications geometriques.
///
/// Les specifications sont censees porter sur **le meme element** : c'est ce qui
/// donne son sens au controle de recouvrement. Les poser sur des elements
/// differents produirait des recouvrements imaginaires.
#[tauri::command]
pub fn geometric(specs: Vec<String>) -> Result<GroupAnalysis, AppError> {
    let engine = GeometricEngine::new()?;
    let parsed = specs
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| engine.parse(s))
        .collect::<mecatool_engine::Result<Vec<_>>>()?;
    Ok(engine.analyse_group(&parsed)?)
}

/// Une classe de tolerance proposable dans une liste de selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassOption {
    /// La designation telle qu'elle s'ecrit sur un plan : `"H7"`, `"g6"`.
    pub designation: String,
    /// La lettre, dans la casse de l'element.
    pub letter: String,
    /// Le degre, par ex. `"IT7"`.
    pub grade: String,
}

/// Les classes que l'interface peut proposer, alesage et arbre.
///
/// # Pourquoi le moteur, et non une liste ecrite dans l'interface
///
/// Les lettres disponibles ne sont pas une decision d'affichage : ce sont celles
/// dont MecaTool possede les ecarts fondamentaux. Dix sur vingt-huit
/// aujourd'hui. Une liste ecrite dans l'interface proposerait des classes que le
/// moteur refuserait ensuite de calculer, ce qui est la pire facon de refuser.
///
/// Toutes les combinaisons lettre × degre sont licites : hors des degres qu'elle
/// vise, une lettre restreinte donne simplement un ecart fondamental nul. Il n'y
/// a donc rien a filtrer, et c'est une propriete de la norme, pas une
/// simplification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassCatalogue {
    pub hole: Vec<ClassOption>,
    pub shaft: Vec<ClassOption>,
    /// Les classes de tolerance generale de l'ISO 2768-1, du plus fin au plus
    /// grossier : pour qu'une cote sans tolerance se choisisse, elle aussi.
    pub general: Vec<GeneralClassOption>,
    /// Les degres seuls, du plus fin au plus large.
    pub grades: Vec<String>,
    pub provenance: Provenance,
}

/// Une classe de tolerance generale proposable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralClassOption {
    /// `m`.
    pub symbol: String,
    /// `moyen`.
    pub name: String,
    /// `ISO 2768-m`, telle que la chaine de cotes la lit.
    pub designation: String,
}

#[tauri::command]
pub fn tolerance_classes() -> Result<ClassCatalogue, AppError> {
    let engine = engine()?;
    let letters = engine.available_letters();

    let options = |feature: mecatool_core::Feature| -> Vec<ClassOption> {
        letters
            .iter()
            .flat_map(|letter| {
                mecatool_core::Grade::all().map(move |grade| {
                    let class = mecatool_core::ToleranceClass::new(feature, *letter, grade);
                    ClassOption {
                        designation: class.to_string(),
                        letter: letter.as_str_for(feature).to_string(),
                        grade: grade.name().to_string(),
                    }
                })
            })
            .collect()
    };

    Ok(ClassCatalogue {
        hole: options(mecatool_core::Feature::Hole),
        shaft: options(mecatool_core::Feature::Shaft),
        general: mecatool_standards::iso2768::ALL_CLASSES
            .iter()
            .map(|class| GeneralClassOption {
                symbol: class.symbol().to_string(),
                name: class.name_fr().to_string(),
                designation: class.designation(),
            })
            .collect(),
        grades: mecatool_core::Grade::all()
            .map(|grade| grade.name().to_string())
            .collect(),
        provenance: engine.provenance(),
    })
}

/// Le registre des domaines, lu une fois au demarrage.
///
/// La navigation se construit a partir de cette liste, et non d'une suite
/// d'onglets ecrite dans l'interface : ajouter un domaine ne doit toucher ni la
/// barre laterale, ni l'accueil, ni la recherche.
#[tauri::command]
pub fn domains() -> Result<Vec<Domain>, AppError> {
    Ok(mecatool_engine::domain::registry()?)
}

/// Ce qu'il faut pour peupler l'ecran des roulements avant toute saisie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearingCatalogue {
    pub families: Vec<BearingFamily>,
    pub regimes: Vec<LoadRegime>,
    pub cases: Vec<MountingCase>,
    pub provenance: Provenance,
    /// La reserve de source, dont celle qui dit que le tableau n'est pas normatif.
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn bearing_catalogue() -> Result<BearingCatalogue, AppError> {
    let engine = BearingEngine::new()?;
    let provenance = engine.provenance();
    Ok(BearingCatalogue {
        families: engine.families().to_vec(),
        regimes: engine.regimes().to_vec(),
        cases: engine.cases().to_vec(),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

/// Les lectures possibles d'une designation de roulement.
///
/// Plusieurs, parce que la source ne dit pas comment decouper une designation :
/// c'est a l'utilisateur de reconnaitre la sienne.
#[tauri::command]
pub fn bearing_read(designation: String) -> Result<Vec<DesignationReading>, AppError> {
    Ok(BearingEngine::new()?.read_designation(&designation)?)
}

/// Les cas d'emploi d'un regime, avec ce que chacun donnerait.
#[tauri::command]
pub fn bearing_options(
    regime: String,
    family: String,
    bore_mm: String,
) -> Result<Vec<MountingOption>, AppError> {
    let engine = BearingEngine::new()?;
    Ok(engine.options(&regime, &family, parse_bore(&bore_mm)?)?)
}

/// Le conseil complet : la classe, et les ecarts qui en decoulent.
#[tauri::command]
pub fn bearing_advise(
    regime: String,
    condition: String,
    family: String,
    bore_mm: String,
) -> Result<MountingAdvice, AppError> {
    let engine = BearingEngine::new()?;
    Ok(engine.advise(&regime, &condition, &family, parse_bore(&bore_mm)?)?)
}

fn parse_bore(bore_mm: &str) -> Result<Length, AppError> {
    Length::parse(bore_mm.trim(), Unit::Millimetre).map_err(|source| AppError {
        message: format!("Alésage illisible : « {bore_mm} »."),
        hint: Some(source.to_string()),
    })
}

/// Ce qu'il faut pour peupler l'ecran des etats de surface avant toute saisie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceCatalogue {
    pub symbols: Vec<SymbolVariant>,
    pub lays: Vec<LaySymbol>,
    pub parameters: Vec<ProfileParameter>,
    pub grades: Vec<RoughnessGrade>,
    pub processes: Vec<ProcessRoughness>,
    /// Les plages de tous les procedes, sans exigence.
    pub chart: RoughnessChart,
    pub provenance: Provenance,
    /// Les reserves de source, a afficher des l'ouverture de l'ecran.
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn surface_catalogue() -> Result<SurfaceCatalogue, AppError> {
    let engine = SurfaceEngine::new()?;
    let indication = engine.indication_table();
    let provenance = engine.provenance();
    Ok(SurfaceCatalogue {
        symbols: indication.symbols().to_vec(),
        lays: indication.lays().to_vec(),
        parameters: indication.parameters().to_vec(),
        grades: engine.grades().to_vec(),
        processes: engine.processes().to_vec(),
        chart: engine.catalogue_chart(),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

/// Lit une indication d'etat de surface : `Ra 0.8`, `MRR Ra 1.6 ⊥`, `N7`.
#[tauri::command]
pub fn surface_read(input: String) -> Result<SurfaceAnalysis, AppError> {
    Ok(SurfaceEngine::new()?.read(&input)?)
}

/// Ce qu'il faut pour peupler l'ecran de la soudure avant toute saisie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeldingCatalogue {
    pub processes: Vec<WeldingProcess>,
    /// Les systemes A et B, et les regles qui les separent.
    pub systems: Vec<WeldSystem>,
    pub system_rules: Vec<String>,
    pub sizes: Vec<SizeLetter>,
    pub elementary: Vec<ElementarySymbol>,
    pub supplementary: Vec<SupplementarySymbol>,
    pub levels: Vec<QualityLevel>,
    pub variables: Vec<QualityVariable>,
    pub imperfections: Vec<Imperfection>,
    pub process_scope: ProcessScope,
    pub provenance: Provenance,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn welding_catalogue() -> Result<WeldingCatalogue, AppError> {
    let engine = WeldingEngine::new()?;
    let symbols = engine.symbol_table();
    let quality = engine.quality_table();
    let provenance = engine.provenance();
    Ok(WeldingCatalogue {
        processes: engine.process_table().processes().to_vec(),
        systems: symbols.systems().to_vec(),
        system_rules: symbols.system_rules().to_vec(),
        sizes: symbols.sizes().to_vec(),
        elementary: symbols.elementary().to_vec(),
        supplementary: symbols.supplementary().to_vec(),
        levels: quality.levels().to_vec(),
        variables: quality.variables().to_vec(),
        imperfections: quality.imperfections().to_vec(),
        process_scope: quality.process_scope().clone(),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

/// Les lectures d'un numero de procede ou d'un nom d'atelier.
///
/// Plusieurs, parfois : « MAG » designe 135, 136 et 138, et le moteur ne
/// choisit pas a la place de l'utilisateur.
#[tauri::command]
pub fn welding_process(input: String) -> Result<Vec<ProcessReading>, AppError> {
    Ok(WeldingEngine::new()?.read_process(&input)?)
}

/// Lit un symbole de soudure complet.
///
/// Les grandeurs voyagent en texte, en millimetres : c'est le moteur qui les
/// lit.
#[tauri::command]
pub fn welding_read(request: WeldRequest) -> Result<WeldReading, AppError> {
    Ok(WeldingEngine::new()?.read_weld(&request)?)
}

/// Ce qu'il faut pour peupler l'ecran de la visserie avant toute saisie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastenerCatalogue {
    pub threads: Vec<MetricThread>,
    pub clearance_series: Vec<ClearanceSeries>,
    pub bolt_classes: Vec<BoltClass>,
    pub nut_classes: Vec<u32>,
    pub provenance: Provenance,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn fastener_catalogue() -> Result<FastenerCatalogue, AppError> {
    let engine = FastenerEngine::new()?;
    let provenance = engine.provenance();
    Ok(FastenerCatalogue {
        threads: engine.thread_table().threads().to_vec(),
        clearance_series: engine.clearance_table().series().to_vec(),
        bolt_classes: engine.strength_table().bolt_classes().to_vec(),
        nut_classes: engine.strength_table().nut_classes().to_vec(),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

/// Lit une designation de filetage : `M10`, `M12 x 1.5`, `M8 8.8`.
#[tauri::command]
pub fn fastener_read(input: String) -> Result<ThreadReport, AppError> {
    Ok(FastenerEngine::new()?.read(&input)?)
}

/// Ce qu'il faut pour peupler l'ecran des matieres avant toute saisie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialsCatalogue {
    pub use_groups: Vec<UseGroup>,
    /// Les nuances d'aciers de construction embarquees, et leurs qualites.
    pub structural_grades: Vec<StructuralGrade>,
    pub families: Vec<MaterialFamily>,
    pub provenance: Provenance,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn materials_catalogue() -> Result<MaterialsCatalogue, AppError> {
    let engine = MaterialsEngine::new()?;
    let provenance = engine.provenance();
    Ok(MaterialsCatalogue {
        use_groups: engine.rules().use_groups().to_vec(),
        structural_grades: engine.structural_grades().to_vec(),
        families: engine.families().to_vec(),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

/// Decompose une designation d'acier : `S355J2`, `42CrMo4`, `X5CrNi18-10`.
#[tauri::command]
pub fn materials_read(input: String) -> Result<SteelReading, AppError> {
    Ok(MaterialsEngine::new()?.read(&input)?)
}

/// Ce que devient un ajustement quand la temperature change.
///
/// Les classes sont facultatives : sans elles, seule la variation du jeu est
/// rendue. Les grandeurs arrivent en texte, et c'est le moteur qui les lit.
#[tauri::command]
pub fn thermal_fit(
    nominal_mm: String,
    delta_t: String,
    hole_family: String,
    shaft_family: String,
    hole_class: Option<String>,
    shaft_class: Option<String>,
) -> Result<ThermalFit, AppError> {
    let nominal =
        Length::parse(nominal_mm.trim(), Unit::Millimetre).map_err(|source| AppError {
            message: format!("Diamètre illisible : « {nominal_mm} »."),
            hint: Some(source.to_string()),
        })?;
    let delta: i32 = delta_t.trim().parse().map_err(|_| AppError {
        message: format!("Écart de température illisible : « {delta_t} »."),
        hint: Some("Un nombre entier de kelvins, par exemple « 80 » ou « -40 ».".into()),
    })?;
    let present = |text: Option<String>| text.filter(|t| !t.trim().is_empty());
    let classes = match (present(hole_class), present(shaft_class)) {
        (Some(hole), Some(shaft)) => Some((
            mecatool_core::ToleranceClass::parse(&hole).map_err(EngineError::from)?,
            mecatool_core::ToleranceClass::parse(&shaft).map_err(EngineError::from)?,
        )),
        (None, None) => None,
        _ => {
            return Err(AppError::simple(
                "Un ajustement demande les deux classes, alésage et arbre — ou aucune.",
            ))
        }
    };
    Ok(
        MaterialsEngine::new()?.thermal_fit(
            nominal,
            delta,
            &hole_family,
            &shaft_family,
            classes,
        )?,
    )
}

/// Le resultat d'une chaine de cotes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainReport {
    pub analysis: ChainAnalysis,
    pub chart: ContributionChart,
    /// La resultante telle qu'elle s'ecrirait sur un plan.
    pub designation: String,
    /// Le verdict, ou le constat qu'il manque une exigence pour conclure.
    pub conclusion: Conclusion,
}

/// Calcule une chaine de cotes.
///
/// `input` liste les maillons, un par ligne : `"A = 20 ±0.1"`. Un signe moins
/// devant le repere ou le nominal marque un maillon diminuant.
///
/// L'estimation statistique n'est jointe que si elle est demandee : c'est une
/// hypothese sur la fabrication, pas une propriete de la geometrie.
#[tauri::command]
pub fn dimension_chain(
    input: String,
    statistical: Option<bool>,
    minimum_mm: Option<String>,
    maximum_mm: Option<String>,
) -> Result<ChainReport, AppError> {
    let links = parse_chain(&input)?;
    let analysis = analyse_chain(&links, statistical.unwrap_or(false))?;

    let bound = |text: Option<String>, side: &str| -> Result<Option<Length>, AppError> {
        match text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            Some(value) => Length::parse(value, Unit::Millimetre)
                .map(Some)
                .map_err(|source| AppError {
                    message: format!("Limite {side} illisible : « {value} »."),
                    hint: Some(source.to_string()),
                }),
            None => Ok(None),
        }
    };

    let minimum = bound(minimum_mm, "minimale")?;
    let maximum = bound(maximum_mm, "maximale")?;

    Ok(ChainReport {
        chart: contribution_chart(&analysis, 640.0),
        designation: analysis.designation(),
        conclusion: verify_chain(&analysis, minimum, maximum),
        analysis,
    })
}

/// Compare plusieurs ajustements sur une meme dimension.
///
/// `input` s'ecrit `"Ø20 H7/g6, H7/h6, H7/k6, H7/p6"` : la virgule separe les
/// ajustements, la barre separe l'alesage de l'arbre.
#[tauri::command]
pub fn compare(
    input: String,
    clearance: Option<String>,
    true_to_scale: Option<bool>,
) -> Result<FitComparison, AppError> {
    let request = parse_comparison(&input)?;

    let requirement = match clearance
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(text) => {
            let (min, max) = parse_clearance_window(text)?;
            Some(ClearanceRequirement::new(request.nominal, min, max)?)
        }
        None => None,
    };

    let options = DiagramOptions {
        mode: if true_to_scale.unwrap_or(false) {
            DiagramMode::TrueToScale
        } else {
            DiagramMode::Deviations
        },
        ..DiagramOptions::default()
    };

    Ok(compare_fits(
        &engine()?,
        request.nominal,
        &request.pairs,
        requirement.as_ref(),
        &options,
    )?)
}

/// Analyse une entree utilisateur, avec ou sans exigence de jeu.
///
/// Une seule commande couvre les trois cas — ajustement, element seul, recherche
/// — parce que c'est l'entree qui les distingue. L'interface n'a pas a deviner
/// quel appel faire avant que le moteur ait lu la saisie.
#[tauri::command]
pub fn analyse(
    input: String,
    clearance: Option<String>,
    true_to_scale: Option<bool>,
) -> Result<Report, AppError> {
    let engine = engine()?;
    let parsed = parse(&input)?;

    let requirement = match clearance
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        Some(text) => {
            let (min, max) = parse_clearance_window(text)?;
            Some(ClearanceRequirement::new(parsed.nominal(), min, max)?)
        }
        None => None,
    };

    let options = DiagramOptions {
        mode: if true_to_scale.unwrap_or(false) {
            DiagramMode::TrueToScale
        } else {
            DiagramMode::Deviations
        },
        ..DiagramOptions::default()
    };

    match parsed {
        ParsedInput::Fit {
            nominal,
            hole,
            shaft,
            ..
        } => {
            let analysis = engine.fit(nominal, hole, shaft)?;
            let verification = match &requirement {
                Some(requirement) => Some(verify_clearance(&analysis.fit, requirement)?),
                None => None,
            };
            Ok(Report::Fit(FitReport {
                diagram: fit_diagram(&analysis.fit, &options),
                classification: analysis.classification_fr(),
                conclusion: classification_conclusion(&analysis),
                verification,
                analysis,
            }))
        }

        ParsedInput::Feature { nominal, class, .. } => Ok(Report::Feature(FeatureReport {
            analysis: engine.feature(nominal, class)?,
        })),

        ParsedInput::NominalOnly { .. } => {
            let Some(requirement) = requirement else {
                return Err(AppError {
                    message: format!("« {input} » ne donne qu'une dimension nominale."),
                    hint: Some(
                        "Ajoutez une classe de tolérance pour calculer, par exemple \
                         « H7/g6 », ou indiquez le jeu recherché pour que MecaTool \
                         propose des solutions."
                            .to_string(),
                    ),
                });
            };
            let result = find_fits(&engine, &requirement, &SearchOptions::default())?;
            Ok(Report::Search(SearchReport {
                diagnosis: result.diagnosis_fr(&requirement),
                requirement,
                result,
            }))
        }
    }
}

/// Le dessin d'un ajustement deja calcule, dans l'autre mode d'echelle.
///
/// Evite de refaire tout le calcul quand l'utilisateur bascule entre le mode
/// lisible et le mode fidele.
#[tauri::command]
pub fn rescale_diagram(input: String, true_to_scale: bool) -> Result<Diagram, AppError> {
    let engine = engine()?;
    match parse(&input)? {
        ParsedInput::Fit {
            nominal,
            hole,
            shaft,
            ..
        } => {
            let analysis = engine.fit(nominal, hole, shaft)?;
            Ok(fit_diagram(
                &analysis.fit,
                &DiagramOptions {
                    mode: if true_to_scale {
                        DiagramMode::TrueToScale
                    } else {
                        DiagramMode::Deviations
                    },
                    ..DiagramOptions::default()
                },
            ))
        }
        _ => Err(AppError::simple(
            "Un diagramme d'ajustement demande un alésage et un arbre.",
        )),
    }
}

/// Etat du moteur, appele une fois au demarrage.
#[tauri::command]
pub fn engine_info() -> Result<EngineInfo, AppError> {
    let engine = engine()?;
    let provenance = engine.provenance();
    Ok(EngineInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        available_letters: engine
            .available_letters()
            .iter()
            .map(|letter: &DeviationLetter| letter.as_lower().to_string())
            .collect(),
        max_nominal_mm: trim_number(engine.max_nominal().to_decimal_string(Unit::Millimetre, 3)),
        warnings: provenance.warnings_fr(),
        provenance,
    })
}

fn trim_number(rendered: String) -> String {
    if rendered.contains('.') {
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_ajustement_produit_un_rapport_complet() {
        let report = analyse("Ø10 H7/g6".into(), None, None).unwrap();
        let Report::Fit(fit) = report else {
            panic!("un ajustement était attendu");
        };
        assert_eq!(fit.analysis.fit.designation(), "H7/g6");
        assert_eq!(fit.diagram.bands.len(), 2);
        assert!(fit.classification.contains("JEU"));
        assert!(fit.verification.is_none());
    }

    #[test]
    fn une_exigence_ajoute_le_verdict() {
        let report = analyse("Ø20 H7/g6".into(), Some("5..50".into()), None).unwrap();
        let Report::Fit(fit) = report else {
            panic!("un ajustement était attendu");
        };
        let verification = fit.verification.expect("un verdict était attendu");
        assert_eq!(verification.verdict, mecatool_core::Verdict::Compatible);
    }

    #[test]
    fn une_dimension_seule_avec_un_besoin_lance_la_recherche() {
        let report = analyse("Ø20".into(), Some("10..30".into()), None).unwrap();
        let Report::Search(search) = report else {
            panic!("une recherche était attendue");
        };
        assert!(!search.result.solutions.is_empty());
        assert!(search.diagnosis.is_some(), "le diagnostic doit être fourni");
    }

    #[test]
    fn une_dimension_seule_sans_besoin_donne_une_piste_daction() {
        let error = analyse("Ø20".into(), None, None).unwrap_err();
        let hint = error.hint.expect("une piste d'action était attendue");
        assert!(hint.contains("H7/g6"));
    }

    #[test]
    fn un_element_seul_est_reconnu() {
        let report = analyse("20 H7".into(), None, None).unwrap();
        let Report::Feature(feature) = report else {
            panic!("un élément seul était attendu");
        };
        assert_eq!(feature.analysis.tolerance.class.to_string(), "H7");
    }

    #[test]
    fn une_entree_illisible_remonte_message_et_piste() {
        let error = analyse("bonjour".into(), None, None).unwrap_err();
        assert!(error.message.contains("illisible"));
        assert!(error.hint.is_some());
    }

    #[test]
    fn une_entree_ambigue_pose_une_question() {
        let error = analyse("10 H7 g6 k6".into(), None, None).unwrap_err();
        assert!(error.message.contains("ambiguë"));
        assert!(error.hint.unwrap().contains("Laquelle"));
    }

    #[test]
    fn le_mode_dechelle_change_le_dessin() {
        let lisible = rescale_diagram("Ø20 H7/g6".into(), false).unwrap();
        let fidele = rescale_diagram("Ø20 H7/g6".into(), true).unwrap();
        assert_ne!(lisible.pixels_per_micrometre, fidele.pixels_per_micrometre);
        assert!(lisible.scale_note.contains("amplifiés"));
        assert!(fidele.scale_note.contains("fidèle"));
    }

    #[test]
    fn letat_du_moteur_decrit_ses_donnees_a_linterface() {
        let info = engine_info().unwrap();
        assert_eq!(info.available_letters.len(), 10);
        assert_eq!(info.max_nominal_mm, "500");
        assert!(!info.provenance.references.is_empty());

        // L'interface affiche un bandeau si et seulement si une source n'est pas
        // vérifiée : les deux doivent donc rester d'accord.
        assert_eq!(
            info.provenance.is_fully_verified(),
            info.warnings.is_empty(),
            "l'état de vérification et les avertissements divergent"
        );
    }

    #[test]
    fn une_chaine_de_cotes_donne_sa_resultante() {
        let report = dimension_chain(
            "A = 20 ±0.1\nB = 10 ±0.05\n-C = 5 ±0.02".into(),
            None,
            None,
            None,
        )
        .unwrap();

        // 20 + 10 − 5 = 25, tolérance 0,1×2 + 0,05×2 + 0,02×2 = 0,34.
        assert_eq!(report.designation, "25 ± 0.17");
        assert_eq!(report.chart.bars.len(), 3);
        // Sans exigence, le moteur ne se prononce pas.
        assert_eq!(
            report.conclusion.verdict,
            mecatool_core::Verdict::InsufficientData
        );
        // L'estimation statistique n'est pas faite sans qu'on la demande.
        assert!(report.analysis.statistical.is_none());
    }

    #[test]
    fn une_chaine_confrontee_a_ses_limites_rend_un_verdict() {
        let report = dimension_chain(
            "A = 20 ±0.1\nB = 10 ±0.05".into(),
            None,
            Some("29.8".into()),
            Some("30.2".into()),
        )
        .unwrap();
        assert_eq!(
            report.conclusion.verdict,
            mecatool_core::Verdict::Compatible
        );
    }

    #[test]
    fn lestimation_statistique_arrive_avec_ses_hypotheses() {
        let report =
            dimension_chain("A = 20 ±0.1\nB = 10 ±0.1".into(), Some(true), None, None).unwrap();
        let estimate = report.analysis.statistical.expect("estimation demandée");
        assert!(!estimate.assumptions.is_empty());
        assert!(estimate.tolerance < report.analysis.tolerance);
    }

    #[test]
    fn une_chaine_illisible_donne_une_piste_daction() {
        let error = dimension_chain("A = 20".into(), None, None, None).unwrap_err();
        assert!(error.hint.unwrap().contains("±0.1"));
    }

    #[test]
    fn un_comparatif_rend_les_ajustements_dans_lordre_de_saisie() {
        let comparison = compare("Ø20 H7/g6, H7/h6, H7/k6, H7/p6".into(), None, None).unwrap();
        assert_eq!(
            comparison
                .entries
                .iter()
                .map(|e| e.designation.as_str())
                .collect::<Vec<_>>(),
            vec!["H7/g6", "H7/h6", "H7/k6", "H7/p6"]
        );
        // Deux zones par ajustement, sur une échelle unique.
        assert_eq!(comparison.diagram.bands.len(), 8);
    }

    #[test]
    fn un_comparatif_avec_exigence_porte_un_verdict_par_ligne() {
        let comparison = compare("Ø20 H7/g6, H7/p6".into(), Some("5..50".into()), None).unwrap();
        assert!(comparison.entries.iter().all(|e| e.verification.is_some()));
        assert_eq!(
            comparison.entries[0].verification.as_ref().unwrap().verdict,
            mecatool_core::Verdict::Compatible
        );
    }

    #[test]
    fn un_comparatif_illisible_donne_une_piste_daction() {
        let error = compare("Ø20 H7".into(), None, None).unwrap_err();
        assert!(error.hint.unwrap().contains("deux éléments"));
    }

    #[test]
    fn les_tolerances_generales_rendent_les_quatre_classes() {
        let comparison = general_tolerances("linear".into(), "50".into()).unwrap();
        assert_eq!(comparison.rows.len(), 4);
        assert_eq!(comparison.rows[1].symbol, "m");
        assert_eq!(
            comparison.rows[1].deviation_label.as_deref(),
            Some("± 0.3 mm")
        );
    }

    #[test]
    fn une_cote_angulaire_utilise_la_bonne_table() {
        let comparison = general_tolerances("angular".into(), "30".into()).unwrap();
        assert_eq!(
            comparison.rows[3].deviation_label.as_deref(),
            Some("\u{b1} 2\u{b0}")
        );
    }

    #[test]
    fn un_type_de_cote_inconnu_donne_une_piste_daction() {
        let error = general_tolerances("surface".into(), "50".into()).unwrap_err();
        assert!(error.hint.unwrap().contains("linear"));
    }

    /// Ecrit un echantillon de chaque rapport, que TypeScript relit pour valider
    /// ses propres types. Un champ renomme cote Rust fait echouer `tsc`.
    #[test]
    fn exporte_les_echantillons_pour_typescript() {
        let dir = std::path::Path::new("../src/fixtures");
        std::fs::create_dir_all(dir).expect("création du dossier d'échantillons");

        // La recherche complète rend plusieurs centaines de solutions. Un
        // échantillon n'a besoin que de la *forme*, pas du volume : on n'en
        // garde que trois, ce qui laisse le fichier lisible à la relecture.
        let mut search = analyse("Ø20".into(), Some("10..30".into()), None).unwrap();
        if let Report::Search(report) = &mut search {
            report.result.solutions.truncate(3);
        }

        let cases: [(&str, Report); 3] = [
            (
                "fit-report.json",
                analyse("Ø10 H7/g6".into(), Some("2..40".into()), None).unwrap(),
            ),
            (
                "feature-report.json",
                analyse("20 H7".into(), None, None).unwrap(),
            ),
            ("search-report.json", search),
        ];

        for (name, report) in cases {
            let json = serde_json::to_string_pretty(&report).expect("sérialisation");
            std::fs::write(dir.join(name), json + "\n").expect("écriture de l'échantillon");
        }

        let info = serde_json::to_string_pretty(&engine_info().unwrap()).expect("sérialisation");
        std::fs::write(dir.join("engine-info.json"), info + "\n").expect("écriture");

        // Une cote de 2 mm : la classe v n'y est pas définie, ce qui met dans
        // l'échantillon le cas d'une classe indisponible avec sa raison.
        let general =
            serde_json::to_string_pretty(&general_tolerances("linear".into(), "2".into()).unwrap())
                .expect("sérialisation");
        std::fs::write(dir.join("general-tolerances.json"), general + "\n").expect("écriture");

        // Les quatre ajustements du cahier des charges, avec une exigence : cet
        // échantillon couvre à la fois le comparatif et les verdicts.
        let comparison = serde_json::to_string_pretty(
            &compare(
                "Ø20 H7/g6, H7/h6, H7/k6, H7/p6".into(),
                Some("5..50".into()),
                None,
            )
            .unwrap(),
        )
        .expect("sérialisation");
        std::fs::write(dir.join("fit-comparison.json"), comparison + "\n").expect("écriture");

        // Une chaîne avec un maillon diminuant, une estimation statistique et
        // des limites fonctionnelles : l'échantillon couvre tout le rapport.
        let chain = serde_json::to_string_pretty(
            &dimension_chain(
                "A = 20 ±0.1\nB = 10 ±0.05\n-C = 5 ±0.02".into(),
                Some(true),
                Some("24.7".into()),
                Some("25.3".into()),
            )
            .unwrap(),
        )
        .expect("sérialisation");
        std::fs::write(dir.join("chain-report.json"), chain + "\n").expect("écriture");

        // Trois spécifications sur un même élément : une correcte, une à qui
        // manque sa référence, et une que la première rend inopérante. Un seul
        // échantillon couvre ainsi les trois sortes de constats.
        let geometry = serde_json::to_string_pretty(
            &geometric(vec!["// 0.02 A".into(), "⏥ 0.05".into(), "⟂ 0.03".into()]).unwrap(),
        )
        .expect("sérialisation");
        std::fs::write(dir.join("geometric-group.json"), geometry + "\n").expect("écriture");

        let catalogue =
            serde_json::to_string_pretty(&geometric_catalogue().unwrap()).expect("sérialisation");
        std::fs::write(dir.join("geometric-catalogue.json"), catalogue + "\n").expect("écriture");

        // Le registre : c'est lui qui construit la navigation, il doit donc
        // traverser la frontière sous une forme que l'interface sait typer.
        // Le catalogue de classes est volumineux : dix lettres par vingt degrés,
        // deux fois. L'échantillon n'a besoin que de la *forme*, mais tronquer
        // en tête ne garderait qu'une seule lettre — un échantillon qui ne
        // montrerait pas la variété qu'il est censé représenter. On retient donc
        // deux degrés, ce qui traverse toutes les lettres.
        let mut classes = tolerance_classes().unwrap();
        let sample = |list: &mut Vec<ClassOption>| {
            list.retain(|option| option.grade == "IT6" || option.grade == "IT7");
        };
        sample(&mut classes.hole);
        sample(&mut classes.shaft);
        let classes = serde_json::to_string_pretty(&classes).expect("sérialisation");
        std::fs::write(
            dir.join("tolerance-classes.json"),
            classes
                + "
",
        )
        .expect("écriture");

        let registry = serde_json::to_string_pretty(&domains().unwrap()).expect("sérialisation");
        std::fs::write(dir.join("domains.json"), registry + "\n").expect("écriture");

        // Un 6210 monté sous charge normale : l'échantillon porte la classe
        // recommandée ET les écarts qui en découlent, donc la composition
        // entière.
        let advice = serde_json::to_string_pretty(
            &bearing_advise(
                "rotating_inner".into(),
                "Charges normales et grandes".into(),
                "ball_radial".into(),
                "50".into(),
            )
            .unwrap(),
        )
        .expect("sérialisation");
        std::fs::write(dir.join("bearing-advice.json"), advice + "\n").expect("écriture");

        let bearings =
            serde_json::to_string_pretty(&bearing_catalogue().unwrap()).expect("sérialisation");
        std::fs::write(dir.join("bearing-catalogue.json"), bearings + "\n").expect("écriture");
        // Les trois domaines ouverts en 0.2. Chaque echantillon couvre a la fois
        // une lecture reussie et des constats : une indication qui cite un
        // procede, un symbole complet avec niveau de qualite, un filetage avec
        // sa classe de qualite.
        let write = |name: &str, json: String| {
            std::fs::write(dir.join(name), json + "\n").expect("écriture");
        };
        write(
            "surface-catalogue.json",
            serde_json::to_string_pretty(&surface_catalogue().unwrap()).expect("sérialisation"),
        );
        write(
            "surface-analysis.json",
            serde_json::to_string_pretty(&surface_read("MRR Ra 0.8 ⊥ fraisé".into()).unwrap())
                .expect("sérialisation"),
        );
        write(
            "welding-catalogue.json",
            serde_json::to_string_pretty(&welding_catalogue().unwrap()).expect("sérialisation"),
        );
        write(
            "welding-process.json",
            serde_json::to_string_pretty(&welding_process("MAG".into()).unwrap())
                .expect("sérialisation"),
        );
        write(
            "weld-reading.json",
            serde_json::to_string_pretty(&welding_read(sample_weld()).unwrap())
                .expect("sérialisation"),
        );
        write(
            "fastener-catalogue.json",
            serde_json::to_string_pretty(&fastener_catalogue().unwrap()).expect("sérialisation"),
        );
        write(
            "materials-catalogue.json",
            serde_json::to_string_pretty(&materials_catalogue().unwrap()).expect("sérialisation"),
        );
        write(
            "steel-reading.json",
            serde_json::to_string_pretty(&materials_read("S355J2".into()).unwrap())
                .expect("sérialisation"),
        );
        // Un logement aluminium sur un arbre acier, Ø50 H7/p6 a +80 K : le
        // serrage disparait. L'echantillon porte le cas qui fait conclure.
        write(
            "thermal-fit.json",
            serde_json::to_string_pretty(
                &thermal_fit(
                    "50".into(),
                    "80".into(),
                    "aluminium".into(),
                    "steel".into(),
                    Some("H7".into()),
                    Some("p6".into()),
                )
                .unwrap(),
            )
            .expect("sérialisation"),
        );
        write(
            "thread-report.json",
            serde_json::to_string_pretty(&fastener_read("M10 8.8".into()).unwrap())
                .expect("sérialisation"),
        );
    }

    /// Un cordon d'angle a5 discontinu, MAG, niveau C : l'echantillon couvre
    /// la cote equivalente, la discontinuite, le procede et les limites.
    fn sample_weld() -> WeldRequest {
        WeldRequest {
            symbol: "fillet".into(),
            size_letter: Some("a".into()),
            size_mm: Some("5".into()),
            count: Some("3".into()),
            length_mm: Some("100".into()),
            spacing_mm: Some("50".into()),
            supplementary: vec!["convex".into()],
            process: Some("135".into()),
            level: Some("C".into()),
            thickness_mm: Some("10".into()),
            width_mm: Some("10".into()),
            ..WeldRequest::default()
        }
    }

    #[test]
    fn une_indication_de_surface_traverse_la_frontiere() {
        let analysis = surface_read("Ra 0.8".into()).unwrap();
        assert_eq!(analysis.designation, "Ra 0.8");
        assert!(!analysis.processes.is_empty());
        let catalogue = surface_catalogue().unwrap();
        assert_eq!(catalogue.warnings.len(), 3);
        assert_eq!(catalogue.chart.rows.len(), catalogue.processes.len());
    }

    #[test]
    fn une_indication_illisible_rend_une_piste_daction() {
        let err = surface_read("bidule".into()).unwrap_err();
        assert!(err.message.contains("illisible"));
        assert!(err.hint.is_some());
    }

    #[test]
    fn un_symbole_de_soudure_traverse_la_frontiere() {
        let reading = welding_read(sample_weld()).unwrap();
        assert!(reading.designation.contains("a5"));
        assert!(reading.quality.is_some());
        let readings = welding_process("MAG".into()).unwrap();
        assert_eq!(readings.len(), 3);
    }

    #[test]
    fn un_symbole_inconnu_dit_lesquels_existent() {
        let err = welding_read(WeldRequest {
            symbol: "bidule".into(),
            ..WeldRequest::default()
        })
        .unwrap_err();
        assert!(err.hint.unwrap().contains("fillet"));
    }

    #[test]
    fn un_filetage_traverse_la_frontiere_avec_ses_trous() {
        let report = fastener_read("M10".into()).unwrap();
        assert_eq!(report.normalised, "M10");
        assert_eq!(report.clearance_holes.len(), 3);
        let catalogue = fastener_catalogue().unwrap();
        assert_eq!(catalogue.warnings.len(), 4);
    }

    #[test]
    fn une_nuance_et_un_ajustement_a_chaud_traversent_la_frontiere() {
        let reading = materials_read("42CrMo4".into()).unwrap();
        assert_eq!(reading.elements.len(), 2);
        let thermal = thermal_fit(
            "50".into(),
            "80".into(),
            "aluminium".into(),
            "steel".into(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(thermal.clearance_shift_label, "+44 µm");
        assert_eq!(materials_catalogue().unwrap().warnings.len(), 3);
    }

    #[test]
    fn une_seule_classe_est_refusee() {
        let err = thermal_fit(
            "50".into(),
            "80".into(),
            "aluminium".into(),
            "steel".into(),
            Some("H7".into()),
            None,
        )
        .unwrap_err();
        assert!(err.message.contains("les deux classes"));
    }

    #[test]
    fn les_nouveaux_domaines_ne_chargent_pas_le_bandeau_global() {
        // Leurs reserves s'affichent sur leur ecran. Le bandeau general reste
        // celui des donnees de l'ajustement, confrontees a leur source.
        assert!(engine_info().unwrap().warnings.is_empty());
    }

    #[test]
    fn les_classes_proposables_viennent_du_moteur() {
        let catalogue = tolerance_classes().unwrap();

        // Dix lettres sur vingt-huit sont embarquees : la liste proposee doit
        // s'y tenir. En proposer d'autres ferait offrir a l'utilisateur des
        // classes que le moteur refuserait ensuite de calculer.
        let letters: std::collections::BTreeSet<&str> =
            catalogue.hole.iter().map(|c| c.letter.as_str()).collect();
        assert_eq!(letters.len(), engine().unwrap().available_letters().len());
        assert!(letters.contains("H"));
        assert!(!letters.contains("A"));

        // La casse porte l'element : majuscule pour l'alesage, minuscule pour
        // l'arbre. C'est ce qui distingue H7 de h7.
        assert!(catalogue.hole.iter().any(|c| c.designation == "H7"));
        assert!(catalogue.shaft.iter().any(|c| c.designation == "g6"));
        assert!(!catalogue.shaft.iter().any(|c| c.designation == "H7"));

        // Toute classe proposee doit se relire : une liste de selection qui
        // proposerait une designation illisible par le parseur serait un piege.
        for option in catalogue.hole.iter().chain(&catalogue.shaft) {
            mecatool_core::ToleranceClass::parse(&option.designation)
                .unwrap_or_else(|e| panic!("{} : {e}", option.designation));
        }

        assert_eq!(catalogue.grades.len(), 20);
        assert_eq!(catalogue.grades[0], "IT01");
    }

    #[test]
    fn le_registre_traverse_la_frontiere() {
        let registry = domains().unwrap();
        assert!(registry.len() >= 10);

        // Un domaine bloqué doit arriver avec sa raison : la navigation
        // l'affichera grisé, mais elle doit pouvoir dire pourquoi. Tous les
        // domaines du registre sont ouverts aujourd'hui ; la règle tient pour
        // le prochain.
        let bloques: Vec<_> = registry.iter().filter(|d| !d.is_available()).collect();
        for domain in bloques {
            assert!(domain.unavailable.is_some(), "{}", domain.id);
        }
    }

    #[test]
    fn une_designation_de_roulement_se_lit() {
        let readings = bearing_read("6210".into()).unwrap();
        assert_eq!(readings[0].bore_code, "10");
    }

    #[test]
    fn le_conseil_de_montage_porte_les_deux_natures_de_source() {
        let advice = bearing_advise(
            "rotating_inner".into(),
            "Charges normales et grandes".into(),
            "ball_radial".into(),
            "50".into(),
        )
        .unwrap();
        assert_eq!(advice.class, "k5");
        // La recommandation doit se voir, sans que les écarts ISO 286 héritent
        // d'un doute qu'ils ne méritent pas.
        assert!(advice
            .conclusion
            .warnings
            .join(" ")
            .contains("sans caractère normatif"));
    }

    #[test]
    fn un_alesage_illisible_rend_une_piste_daction() {
        let err = bearing_options("rotating_inner".into(), "ball_radial".into(), "abc".into())
            .unwrap_err();
        assert!(err.message.contains("Alésage illisible"));
        assert!(err.hint.is_some());
    }

    #[test]
    fn le_catalogue_traverse_la_frontiere_avec_sa_reserve() {
        let catalogue = geometric_catalogue().unwrap();
        assert_eq!(catalogue.families.len(), 4);
        assert!(!catalogue.characteristics.is_empty());
        assert!(!catalogue.modifiers.is_empty());

        // La donnee est secondaire : la reserve doit arriver a l'ecran des son
        // ouverture, avant meme que l'utilisateur ait saisi quoi que ce soit.
        assert_eq!(catalogue.warnings.len(), 1);
        assert!(catalogue.warnings[0].contains("recueil"));
        assert!(!catalogue.provenance.is_fully_verified());
    }

    #[test]
    fn le_bandeau_global_ne_se_charge_pas_de_la_reserve_geometrique() {
        // L'ISO 286 et l'ISO 2768 sont verifiees contre leur source primaire.
        // Faire remonter la reserve ISO 1101 dans le bandeau general reviendrait
        // a jeter un doute sur des donnees qui n'en meritent pas, et a diluer
        // celle qui en merite un.
        let info = engine_info().unwrap();
        assert!(info.warnings.is_empty());
        assert!(info.provenance.is_fully_verified());
    }

    #[test]
    fn une_specification_illisible_rend_une_piste_daction() {
        let err = geometric(vec!["bidule 0.1".into()]).unwrap_err();
        assert!(err.message.contains("illisible"));
        assert!(err.hint.is_some(), "la piste d'action manque");
    }

    #[test]
    fn les_lignes_vides_sont_ignorees() {
        // L'interface propose plusieurs champs : les laisser vides ne doit pas
        // faire echouer la lecture des autres.
        let group = geometric(vec!["⏥ 0.05".into(), "  ".into(), String::new()]).unwrap();
        assert_eq!(group.specs.len(), 1);
    }

    #[test]
    fn un_groupe_entierement_vide_est_refuse() {
        assert!(geometric(vec![String::new(), "  ".into()]).is_err());
    }
}
