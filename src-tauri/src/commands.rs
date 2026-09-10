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

use mecatol_core::{Conclusion, DeviationLetter, Length, Provenance, Unit};
use mecatol_engine::chain::{
    analyse_chain, contribution_chart, verify_chain, ChainAnalysis, ContributionChart,
};
use mecatol_engine::compare::{compare_fits, FitComparison};
use mecatol_engine::diagram::{fit_diagram, Diagram, DiagramMode, DiagramOptions};
use mecatol_engine::iso2768::{ClassComparison, Iso2768Engine};
use mecatol_engine::iso286::{
    classification_conclusion, FeatureAnalysis, FitAnalysis, Iso286Engine,
};
use mecatol_engine::parser::{
    parse, parse_chain, parse_clearance_window, parse_comparison, ParsedInput,
};
use mecatol_engine::requirement::{verify_clearance, ClearanceRequirement, Verification};
use mecatol_engine::search::{find_fits, SearchOptions, SearchResult};
use mecatol_engine::EngineError;
use mecatol_standards::iso2768::MeasureKind;
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
                         « H7/g6 », ou indiquez le jeu recherché pour que Mecatol \
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
        assert_eq!(verification.verdict, mecatol_core::Verdict::Compatible);
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
            mecatol_core::Verdict::InsufficientData
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
        assert_eq!(report.conclusion.verdict, mecatol_core::Verdict::Compatible);
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
            mecatol_core::Verdict::Compatible
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
    }
}
