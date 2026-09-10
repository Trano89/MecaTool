//! Calcul des tolerances et des ajustements selon l'ISO 286.
//!
//! Le moteur ne contient aucune valeur normative : il assemble celles que
//! `mecatol-standards` lui fournit, en appliquant les regles de composition de
//! la norme et en gardant la trace de chaque etape.

use serde::{Deserialize, Serialize};

use mecatol_core::{
    Conclusion, DeviationLetter, Deviations, Feature, FeatureTolerance, Fit, Grade, Length,
    Provenance, ReasoningStep, SizeRange, ToleranceClass, Unit, Verdict,
};
use mecatol_standards::iso286::deviations::{hole_rule, DeviationSide};
use mecatol_standards::{ItGradeTable, ShaftDeviationTable};

use crate::error::{EngineError, Result};
use crate::format::{um, um_signed};

/// Le resultat du tolerancement d'un element, avec sa justification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureAnalysis {
    pub tolerance: FeatureTolerance,
    /// Le detail du calcul, destine au mode expert.
    pub steps: Vec<ReasoningStep>,
    pub provenance: Provenance,
}

/// Le resultat d'un ajustement, avec sa justification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FitAnalysis {
    pub fit: Fit,
    pub hole_steps: Vec<ReasoningStep>,
    pub shaft_steps: Vec<ReasoningStep>,
    /// Le calcul du jeu proprement dit.
    pub fit_steps: Vec<ReasoningStep>,
    pub provenance: Provenance,
}

impl FitAnalysis {
    /// Enonce de la classification, texte et pastille.
    ///
    /// La pastille ne porte jamais seule l'information : elle accompagne un
    /// libelle lisible sans distinction des couleurs.
    pub fn classification_fr(&self) -> String {
        format!(
            "{} {}",
            self.fit.kind.badge(),
            self.fit.kind.label_fr().to_uppercase()
        )
    }

    /// Avertissements a afficher, notamment sur les donnees non verifiees.
    pub fn warnings(&self) -> Vec<String> {
        self.provenance.warnings_fr()
    }
}

/// Le moteur ISO 286, adosse aux tables normatives.
#[derive(Debug, Clone, Copy)]
pub struct Iso286Engine {
    grades: &'static ItGradeTable,
    shaft_deviations: &'static ShaftDeviationTable,
}

impl Iso286Engine {
    /// Ouvre le moteur sur les tables embarquees.
    pub fn new() -> Result<Self> {
        Ok(Iso286Engine {
            grades: ItGradeTable::embedded()?,
            shaft_deviations: ShaftDeviationTable::embedded()?,
        })
    }

    /// Les lettres d'ecart fondamental effectivement disponibles.
    pub fn available_letters(&self) -> Vec<DeviationLetter> {
        self.shaft_deviations.letters().collect()
    }

    /// La plus grande dimension nominale couverte par les tables.
    pub fn max_nominal(&self) -> Length {
        self.grades.max_nominal()
    }

    /// Provenance des donnees mobilisees par le moteur.
    pub fn provenance(&self) -> Provenance {
        Provenance::new()
            .with(self.grades.standard().clone())
            .with(self.shaft_deviations.standard().clone())
    }

    /// Calcule les ecarts et dimensions limites d'un element.
    ///
    /// # Regles appliquees
    ///
    /// La largeur de la zone vient du degre `IT`. Sa position vient de la
    /// lettre :
    ///
    /// * arbre, lettres a..h : la table donne `es`, donc `ei = es - IT` ;
    /// * arbre, lettres j..zc : la table donne `ei`, donc `es = ei + IT` ;
    /// * alesage, lettres A..H : `EI = -es(arbre)`, puis `ES = EI + IT` ;
    /// * alesage, lettres J..ZC : `ES = -ei(arbre) + delta`, puis `EI = ES - IT` ;
    /// * `js` / `JS` : zone symetrique `+/- IT/2`.
    pub fn feature(&self, nominal: Length, class: ToleranceClass) -> Result<FeatureAnalysis> {
        let it = self.grades.value(nominal, class.grade)?;
        let shaft_fd = self
            .shaft_deviations
            .fundamental(nominal, class.letter, class.grade)?;

        let mut steps = vec![ReasoningStep::new(format!("Degré {}", class.grade.name()))
            .with_expression(format!("table ISO 286-1, échelon {}", it.range.label_fr()))
            .with_value(um(it.value))];

        let (deviations, fundamental) = match (class.feature, shaft_fd.side) {
            // Zone symetrique : la lettre ne tabule rien, elle partage IT.
            (_, DeviationSide::SymmetricHalfIt) => {
                let half = half(it.value);
                steps.push(
                    ReasoningStep::new("Zone symétrique")
                        .with_expression("+/- IT/2")
                        .with_value(format!("+/- {}", um(half))),
                );
                (Deviations::symmetric(half), half)
            }

            // Arbre : la table donne directement l'ecart, l'autre s'en deduit.
            (Feature::Shaft, DeviationSide::Upper) => {
                steps.push(
                    ReasoningStep::new(format!(
                        "Écart fondamental es ({})",
                        class.letter.as_lower()
                    ))
                    .with_expression(format!(
                        "table des écarts, échelon {}",
                        shaft_fd.range.label_fr()
                    ))
                    .with_value(um_signed(shaft_fd.value)),
                );
                steps.push(
                    ReasoningStep::new("Écart inférieur ei")
                        .with_expression("ei = es - IT")
                        .with_value(format!(
                            "{} - {} = {}",
                            um_signed(shaft_fd.value),
                            um(it.value),
                            um_signed(shaft_fd.value - it.value)
                        )),
                );
                (
                    Deviations::from_upper_and_width(shaft_fd.value, it.value)?,
                    shaft_fd.value,
                )
            }
            (Feature::Shaft, DeviationSide::Lower) => {
                steps.push(
                    ReasoningStep::new(format!(
                        "Écart fondamental ei ({})",
                        class.letter.as_lower()
                    ))
                    .with_expression(format!(
                        "table des écarts, échelon {}",
                        shaft_fd.range.label_fr()
                    ))
                    .with_value(um_signed(shaft_fd.value)),
                );
                steps.push(
                    ReasoningStep::new("Écart supérieur es")
                        .with_expression("es = ei + IT")
                        .with_value(format!(
                            "{} + {} = {}",
                            um_signed(shaft_fd.value),
                            um(it.value),
                            um_signed(shaft_fd.value + it.value)
                        )),
                );
                (
                    Deviations::from_lower_and_width(shaft_fd.value, it.value)?,
                    shaft_fd.value,
                )
            }

            // Alesage, lettres A..H : symetrique de l'arbre par rapport a la ligne zero.
            (Feature::Hole, DeviationSide::Upper) => {
                let ei = -shaft_fd.value;
                steps.push(
                    ReasoningStep::new(format!(
                        "Écart fondamental EI ({})",
                        class.letter.as_upper()
                    ))
                    .with_expression(format!(
                        "EI = -es({}) : règle générale de dérivation des alésages",
                        class.letter.as_lower()
                    ))
                    .with_value(format!(
                        "-({}) = {}",
                        um_signed(shaft_fd.value),
                        um_signed(ei)
                    )),
                );
                steps.push(
                    ReasoningStep::new("Écart supérieur ES")
                        .with_expression("ES = EI + IT")
                        .with_value(format!(
                            "{} + {} = {}",
                            um_signed(ei),
                            um(it.value),
                            um_signed(ei + it.value)
                        )),
                );
                (Deviations::from_lower_and_width(ei, it.value)?, ei)
            }

            // Alesage, lettres J..ZC : regle generale, corrigee du delta.
            (Feature::Hole, DeviationSide::Lower) => {
                let base = -shaft_fd.value;
                let delta = if hole_rule::applies_delta(class.letter, class.grade) {
                    Some(self.delta(nominal, class.grade)?)
                } else {
                    None
                };
                let es = base + delta.map(|d| d.value).unwrap_or(Length::ZERO);

                steps.push(
                    ReasoningStep::new(format!(
                        "Écart fondamental ES ({})",
                        class.letter.as_upper()
                    ))
                    .with_expression(format!(
                        "ES = -ei({}) : règle générale de dérivation des alésages",
                        class.letter.as_lower()
                    ))
                    .with_value(format!(
                        "-({}) = {}",
                        um_signed(shaft_fd.value),
                        um_signed(base)
                    )),
                );
                if let Some(d) = delta {
                    steps.push(
                        ReasoningStep::new("Correctif delta")
                            .with_expression(format!(
                                "delta = {} - {} (règle spéciale pour {} jusqu'à {})",
                                class.grade.name(),
                                d.finer.name(),
                                class.letter.as_upper(),
                                if matches!(
                                    class.letter,
                                    DeviationLetter::K | DeviationLetter::M | DeviationLetter::N
                                ) {
                                    "IT8"
                                } else {
                                    "IT7"
                                }
                            ))
                            .with_value(format!(
                                "{} - {} = {}",
                                um(d.coarse),
                                um(d.fine),
                                um(d.value)
                            )),
                    );
                    steps.push(
                        ReasoningStep::new("Écart supérieur ES corrigé")
                            .with_expression("ES = -ei + delta")
                            .with_value(format!(
                                "{} + {} = {}",
                                um_signed(base),
                                um(d.value),
                                um_signed(es)
                            )),
                    );
                }
                steps.push(
                    ReasoningStep::new("Écart inférieur EI")
                        .with_expression("EI = ES - IT")
                        .with_value(format!(
                            "{} - {} = {}",
                            um_signed(es),
                            um(it.value),
                            um_signed(es - it.value)
                        )),
                );
                (Deviations::from_upper_and_width(es, it.value)?, es)
            }
        };

        let tolerance = FeatureTolerance::assemble(
            class.feature,
            class,
            nominal,
            deviations,
            it.range,
            fundamental,
        )?;

        steps.push(
            ReasoningStep::new("Dimensions limites")
                .with_expression("nominal + écarts")
                .with_value(format!(
                    "{} .. {} mm",
                    tolerance
                        .limits
                        .min()
                        .to_decimal_string(Unit::Millimetre, 3),
                    tolerance
                        .limits
                        .max()
                        .to_decimal_string(Unit::Millimetre, 3)
                )),
        );

        Ok(FeatureAnalysis {
            tolerance,
            steps,
            provenance: self.provenance(),
        })
    }

    /// Le correctif `delta = IT(n) - IT(n-1)` employe par la regle des alesages.
    fn delta(&self, nominal: Length, grade: Grade) -> Result<DeltaTerm> {
        let finer = grade.previous().ok_or_else(|| EngineError::NoFinerGrade {
            grade: grade.name().to_string(),
        })?;
        let coarse = self.grades.value(nominal, grade)?.value;
        let fine = self.grades.value(nominal, finer)?.value;
        Ok(DeltaTerm {
            value: coarse - fine,
            coarse,
            fine,
            finer,
        })
    }

    /// Calcule un ajustement complet.
    pub fn fit(
        &self,
        nominal: Length,
        hole: ToleranceClass,
        shaft: ToleranceClass,
    ) -> Result<FitAnalysis> {
        if hole.feature != Feature::Hole || shaft.feature != Feature::Shaft {
            return Err(EngineError::NotAFitPair {
                first: hole.to_string(),
                second: shaft.to_string(),
            });
        }

        let hole_analysis = self.feature(nominal, hole)?;
        let shaft_analysis = self.feature(nominal, shaft)?;
        let fit = Fit::assemble(hole_analysis.tolerance, shaft_analysis.tolerance);

        let fit_steps = vec![
            ReasoningStep::new("Jeu minimal")
                .with_expression("EI - es : alésage au plus petit, arbre au plus grand")
                .with_value(format!(
                    "{} - ({}) = {}",
                    um_signed(fit.hole.deviations.lower()),
                    um_signed(fit.shaft.deviations.upper()),
                    um_signed(fit.min_clearance)
                )),
            ReasoningStep::new("Jeu maximal")
                .with_expression("ES - ei : alésage au plus grand, arbre au plus petit")
                .with_value(format!(
                    "{} - ({}) = {}",
                    um_signed(fit.hole.deviations.upper()),
                    um_signed(fit.shaft.deviations.lower()),
                    um_signed(fit.max_clearance)
                )),
            ReasoningStep::new("Amplitude du jeu")
                .with_expression("IT(alésage) + IT(arbre)")
                .with_value(format!(
                    "{} + {} = {}",
                    um(fit.hole.it),
                    um(fit.shaft.it),
                    um(fit.clearance_span())
                )),
            ReasoningStep::new("Classification")
                .with_expression("signe des deux bornes du jeu")
                .with_value(fit.kind.label_fr().to_string()),
        ];

        Ok(FitAnalysis {
            fit,
            hole_steps: hole_analysis.steps,
            shaft_steps: shaft_analysis.steps,
            fit_steps,
            provenance: self.provenance(),
        })
    }
}

/// Le terme `delta` et les valeurs qui le composent, pour le mode expert.
#[derive(Debug, Clone, Copy)]
struct DeltaTerm {
    value: Length,
    coarse: Length,
    fine: Length,
    finer: Grade,
}

/// Moitie exacte d'une longueur.
///
/// Un `IT` impair en micrometres donne un demi-micrometre, represente sans
/// perte grace au stockage en nanometres.
fn half(value: Length) -> Length {
    Length::from_nanometres(value.nanometres() / 2)
}

/// Enonce la classification d'un ajustement sous forme de conclusion.
///
/// Utilise lorsqu'aucune exigence fonctionnelle n'a ete fournie : le moteur dit
/// ce que l'ajustement produit, sans se prononcer sur son adequation.
pub fn classification_conclusion(analysis: &FitAnalysis) -> Conclusion {
    let fit = &analysis.fit;
    let detail = match fit.kind {
        mecatol_core::FitKind::Clearance => format!(
            "L'arbre restera toujours plus petit que l'alésage : le jeu varie de {} à {}.",
            um(fit.min_clearance),
            um(fit.max_clearance)
        ),
        mecatol_core::FitKind::Transition => format!(
            "Selon les pièces réelles, l'assemblage présentera soit un serrage jusqu'à {}, \
             soit un jeu jusqu'à {}.",
            um(-fit.min_clearance),
            um(fit.max_clearance)
        ),
        mecatol_core::FitKind::Interference => format!(
            "L'arbre sera toujours plus grand que l'alésage : le serrage varie de {} à {}.",
            um(-fit.max_clearance),
            um(-fit.min_clearance)
        ),
    };

    // Une classification n'est pas un jugement d'adequation : sans exigence
    // fonctionnelle, le moteur ne peut pas dire si l'ajustement convient.
    Conclusion::new(Verdict::InsufficientData, detail)
        .with_steps(analysis.fit_steps.clone())
        .with_warnings(analysis.warnings())
}

/// Echelon de dimensions utilise, expose pour l'interface.
pub fn size_range_of(analysis: &FeatureAnalysis) -> SizeRange {
    analysis.tolerance.size_range
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Iso286Engine {
        Iso286Engine::new().expect("les tables doivent se charger")
    }

    fn mm(v: &str) -> Length {
        Length::parse(v, Unit::Millimetre).unwrap()
    }

    fn class(s: &str) -> ToleranceClass {
        ToleranceClass::parse(s).unwrap()
    }

    fn limits(analysis: &FeatureAnalysis) -> (String, String) {
        (
            analysis
                .tolerance
                .limits
                .min()
                .to_decimal_string(Unit::Millimetre, 3),
            analysis
                .tolerance
                .limits
                .max()
                .to_decimal_string(Unit::Millimetre, 3),
        )
    }

    #[test]
    fn alesage_h7_a_diametre_10() {
        let a = engine().feature(mm("10"), class("H7")).unwrap();
        assert_eq!(a.tolerance.deviations.lower(), Length::ZERO);
        assert_eq!(a.tolerance.deviations.upper(), Length::from_micrometres(15));
        assert_eq!(limits(&a), ("10.000".into(), "10.015".into()));
        assert_eq!(a.tolerance.it, Length::from_micrometres(15));
    }

    #[test]
    fn arbre_g6_a_diametre_10() {
        let a = engine().feature(mm("10"), class("g6")).unwrap();
        assert_eq!(a.tolerance.deviations.upper(), Length::from_micrometres(-5));
        assert_eq!(
            a.tolerance.deviations.lower(),
            Length::from_micrometres(-14)
        );
        assert_eq!(limits(&a), ("9.986".into(), "9.995".into()));
        assert_eq!(a.tolerance.it, Length::from_micrometres(9));
    }

    /// Le cas de reference du cahier des charges.
    #[test]
    fn ajustement_h7_g6_a_diametre_10() {
        let f = engine().fit(mm("10"), class("H7"), class("g6")).unwrap();
        assert_eq!(f.fit.min_clearance, Length::from_micrometres(5));
        assert_eq!(f.fit.max_clearance, Length::from_micrometres(29));
        assert_eq!(f.fit.kind, mecatol_core::FitKind::Clearance);
        assert_eq!(f.fit.designation(), "H7/g6");
        assert!(f.classification_fr().contains("AJUSTEMENT AVEC JEU"));
    }

    #[test]
    fn ajustement_h7_g6_a_diametre_20() {
        let f = engine().fit(mm("20"), class("H7"), class("g6")).unwrap();
        // H7 : 0 / +21 um. g6 : -7 / -20 um.
        assert_eq!(f.fit.hole.deviations.upper(), Length::from_micrometres(21));
        assert_eq!(f.fit.shaft.deviations.upper(), Length::from_micrometres(-7));
        assert_eq!(
            f.fit.shaft.deviations.lower(),
            Length::from_micrometres(-20)
        );
        assert_eq!(f.fit.min_clearance, Length::from_micrometres(7));
        assert_eq!(f.fit.max_clearance, Length::from_micrometres(41));
    }

    #[test]
    fn ajustement_h7_h6_a_diametre_25_a_un_jeu_minimal_nul() {
        let f = engine().fit(mm("25"), class("H7"), class("h6")).unwrap();
        assert_eq!(f.fit.min_clearance, Length::ZERO);
        assert_eq!(f.fit.max_clearance, Length::from_micrometres(34));
        assert_eq!(f.fit.kind, mecatol_core::FitKind::Clearance);
    }

    #[test]
    fn ajustement_h7_k6_a_diametre_50_est_incertain() {
        let f = engine().fit(mm("50"), class("H7"), class("k6")).unwrap();
        // IT7 = 25, IT6 = 16, ei(k) = +2 -> k6 : +2 / +18.
        assert_eq!(f.fit.shaft.deviations.lower(), Length::from_micrometres(2));
        assert_eq!(f.fit.shaft.deviations.upper(), Length::from_micrometres(18));
        assert_eq!(f.fit.min_clearance, Length::from_micrometres(-18));
        assert_eq!(f.fit.max_clearance, Length::from_micrometres(23));
        assert_eq!(f.fit.kind, mecatol_core::FitKind::Transition);
    }

    /// Verifie la regle du delta sur des alesages dont les valeurs sont connues.
    #[test]
    fn la_regle_du_delta_donne_les_ecarts_dalesage_attendus() {
        let e = engine();
        for (class_name, nominal, expected_es_um, expected_ei_um) in [
            // M6 a Ou20 : delta = IT6 - IT5 = 13 - 9 = 4 ; ES = -8 + 4 = -4.
            ("M6", "20", -4, -17),
            // N7 a Ou20 : delta = IT7 - IT6 = 21 - 13 = 8 ; ES = -15 + 8 = -7.
            ("N7", "20", -7, -28),
            // P7 a Ou20 : delta = 8 ; ES = -22 + 8 = -14.
            ("P7", "20", -14, -35),
            // K7 a Ou20 : delta = 8 ; ES = -2 + 8 = +6.
            ("K7", "20", 6, -15),
        ] {
            let a = e.feature(mm(nominal), class(class_name)).unwrap();
            assert_eq!(
                a.tolerance.deviations.upper(),
                Length::from_micrometres(expected_es_um),
                "{class_name} a {nominal} : ES"
            );
            assert_eq!(
                a.tolerance.deviations.lower(),
                Length::from_micrometres(expected_ei_um),
                "{class_name} a {nominal} : EI"
            );
        }
    }

    #[test]
    fn alesage_g7_derive_de_larbre_g() {
        // EI = -es(g) = +5 ; ES = 5 + 15 = +20.
        let a = engine().feature(mm("10"), class("G7")).unwrap();
        assert_eq!(a.tolerance.deviations.lower(), Length::from_micrometres(5));
        assert_eq!(a.tolerance.deviations.upper(), Length::from_micrometres(20));
    }

    #[test]
    fn js_partage_it_de_part_et_dautre_du_nominal() {
        // IT7 a Ou20 vaut 21 um : la demi-valeur tombe sur 10,5 um et doit rester exacte.
        let a = engine().feature(mm("20"), class("js7")).unwrap();
        assert_eq!(
            a.tolerance.deviations.upper(),
            Length::from_nanometres(10_500)
        );
        assert_eq!(
            a.tolerance.deviations.lower(),
            Length::from_nanometres(-10_500)
        );
        assert_eq!(a.tolerance.it, Length::from_micrometres(21));
    }

    #[test]
    fn deux_alesages_ou_deux_arbres_ne_font_pas_un_ajustement() {
        let e = engine();
        assert!(matches!(
            e.fit(mm("10"), class("H7"), class("G6")),
            Err(EngineError::NotAFitPair { .. })
        ));
        assert!(matches!(
            e.fit(mm("10"), class("h7"), class("g6")),
            Err(EngineError::NotAFitPair { .. })
        ));
    }

    #[test]
    fn une_lettre_non_couverte_remonte_une_erreur_explicite() {
        let err = engine().feature(mm("20"), class("s6")).unwrap_err();
        let message = err.to_string();
        assert!(message.contains('s'), "message peu clair : {message}");
    }

    #[test]
    fn le_resultat_porte_ses_avertissements_de_non_verification() {
        let f = engine().fit(mm("10"), class("H7"), class("g6")).unwrap();
        // Tant que les tables ne sont pas verifiees, le resultat doit le dire.
        assert!(!f.provenance.is_fully_verified());
        let warnings = f.warnings();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().all(|w| w.contains("non vérifiée")));
    }

    #[test]
    fn le_raisonnement_est_complet_et_exploitable() {
        let f = engine().fit(mm("10"), class("H7"), class("g6")).unwrap();
        assert!(f.hole_steps.len() >= 3);
        assert!(f.shaft_steps.len() >= 3);
        assert_eq!(f.fit_steps.len(), 4);
        let min_step = &f.fit_steps[0];
        assert_eq!(min_step.label, "Jeu minimal");
        assert!(min_step.value.as_ref().unwrap().contains("5"));
    }

    #[test]
    fn sans_exigence_le_moteur_ne_se_prononce_pas_sur_ladequation() {
        let f = engine().fit(mm("10"), class("H7"), class("g6")).unwrap();
        let c = classification_conclusion(&f);
        assert_eq!(c.verdict, Verdict::InsufficientData);
        assert!(c.detail.contains("jeu varie"));
    }
}
