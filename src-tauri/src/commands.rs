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

use mecatol_core::{Conclusion, DeviationLetter, Provenance, Unit};
use mecatol_engine::diagram::{fit_diagram, Diagram, DiagramMode, DiagramOptions};
use mecatol_engine::iso286::{
    classification_conclusion, FeatureAnalysis, FitAnalysis, Iso286Engine,
};
use mecatol_engine::parser::{parse, parse_clearance_window, ParsedInput};
use mecatol_engine::requirement::{verify_clearance, ClearanceRequirement, Verification};
use mecatol_engine::search::{find_fits, SearchOptions, SearchResult};
use mecatol_engine::EngineError;
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
    fn letat_du_moteur_annonce_les_donnees_non_verifiees() {
        let info = engine_info().unwrap();
        assert_eq!(info.available_letters.len(), 10);
        assert_eq!(info.max_nominal_mm, "500");
        // Tant que les tables ne sont pas vérifiées, l'interface doit le savoir.
        assert!(!info.warnings.is_empty());
        assert!(!info.provenance.is_fully_verified());
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
    }
}
