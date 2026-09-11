//! Exigences fonctionnelles et verdict d'adequation.
//!
//! Un ajustement calcule dit ce qui *va se passer*. Une exigence dit ce qu'on
//! *veut*. Ce module confronte les deux et rend un verdict motive.
//!
//! # La semantique retenue
//!
//! Un ajustement ne produit pas un jeu, mais une **plage** de jeux : selon les
//! pieces reellement fabriquees, l'assemblage tombera quelque part entre le jeu
//! minimal et le jeu maximal. L'exigence definit elle aussi une plage.
//!
//! ```text
//!   exigence   [------------------------]
//!   calcul        [--------------]              tout assemblage convient  -> compatible
//!
//!   exigence   [------------------------]
//!   calcul                        [-----------] certains assemblages sortent -> attention
//!
//!   exigence   [------------]
//!   calcul                        [-----------] aucun assemblage ne convient -> non compatible
//! ```
//!
//! Le verdict porte donc sur **tous** les assemblages possibles, pas sur un cas
//! moyen. C'est la seule lecture qui ait un sens en fabrication.

use serde::{Deserialize, Serialize};

use mecatool_core::{Conclusion, Fit, Length, ReasoningStep, Verdict};

use crate::error::{EngineError, Result};
use crate::format::um;

/// Une exigence portant sur le jeu d'un ajustement.
///
/// Les deux bornes sont facultatives : « au moins 10 µm de jeu » est une
/// exigence recevable, et le moteur ne doit pas exiger une borne haute qui
/// n'existe pas dans le besoin reel.
///
/// Un serrage s'exprime comme un jeu negatif, conformement a la convention du
/// moteur : « serrage d'au moins 5 µm » se note `max_clearance = -5 µm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearanceRequirement {
    pub nominal: Length,
    pub min_clearance: Option<Length>,
    pub max_clearance: Option<Length>,
}

impl ClearanceRequirement {
    /// Construit une exigence en verifiant sa coherence interne.
    pub fn new(
        nominal: Length,
        min_clearance: Option<Length>,
        max_clearance: Option<Length>,
    ) -> Result<Self> {
        if let (Some(min), Some(max)) = (min_clearance, max_clearance) {
            if min > max {
                return Err(EngineError::ContradictoryRequirement {
                    detail: format!(
                        "le jeu minimal demandé ({}) dépasse le jeu maximal demandé ({})",
                        um(min),
                        um(max)
                    ),
                });
            }
        }
        if !nominal.is_positive() {
            return Err(EngineError::ContradictoryRequirement {
                detail: "la dimension nominale doit être strictement positive".to_string(),
            });
        }
        Ok(ClearanceRequirement {
            nominal,
            min_clearance,
            max_clearance,
        })
    }

    /// Vrai si aucune borne n'a ete donnee : le moteur ne peut alors rien conclure.
    pub fn is_empty(&self) -> bool {
        self.min_clearance.is_none() && self.max_clearance.is_none()
    }

    /// Enonce de la fenetre demandee, pour l'affichage.
    pub fn window_fr(&self) -> String {
        match (self.min_clearance, self.max_clearance) {
            (Some(min), Some(max)) => format!("{} à {}", um(min), um(max)),
            (Some(min), None) => format!("au moins {}", um(min)),
            (None, Some(max)) => format!("au plus {}", um(max)),
            (None, None) => "non précisée".to_string(),
        }
    }
}

/// De combien la solution respecte, ou viole, chaque borne de l'exigence.
///
/// Positif = marge disponible. Negatif = depassement. `None` = borne non exigee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Margins {
    /// `jeu minimal calcule - jeu minimal demande`.
    pub lower: Option<Length>,
    /// `jeu maximal demande - jeu maximal calcule`.
    pub upper: Option<Length>,
}

impl Margins {
    /// La plus petite marge, celle qui limite la solution.
    pub fn tightest(&self) -> Option<Length> {
        match (self.lower, self.upper) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    pub fn respects_all(&self) -> bool {
        self.lower.is_none_or(|m| !m.is_negative()) && self.upper.is_none_or(|m| !m.is_negative())
    }
}

/// Le resultat complet d'une verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    pub verdict: Verdict,
    pub margins: Margins,
    pub conclusion: Conclusion,
}

/// Confronte un ajustement calcule a une exigence fonctionnelle.
///
/// Refuse si les dimensions nominales ne coincident pas : comparer le jeu d'un
/// Ø20 a une exigence formulee pour un Ø25 n'aurait aucun sens.
pub fn verify_clearance(fit: &Fit, requirement: &ClearanceRequirement) -> Result<Verification> {
    if fit.hole.nominal != requirement.nominal {
        return Err(EngineError::ContradictoryRequirement {
            detail: format!(
                "l'ajustement porte sur {} et l'exigence sur {}",
                fit.hole.nominal, requirement.nominal
            ),
        });
    }

    let margins = Margins {
        lower: requirement.min_clearance.map(|m| fit.min_clearance - m),
        upper: requirement.max_clearance.map(|m| m - fit.max_clearance),
    };

    let calculated = format!("{} à {}", um(fit.min_clearance), um(fit.max_clearance));
    let required = requirement.window_fr();

    let mut steps = vec![
        ReasoningStep::new("Fenêtre fonctionnelle demandée").with_value(required.clone()),
        ReasoningStep::new("Plage de jeu calculée")
            .with_expression(format!("ajustement {}", fit.designation()))
            .with_value(calculated.clone()),
    ];

    if let (Some(required_min), Some(margin)) = (requirement.min_clearance, margins.lower) {
        steps.push(
            ReasoningStep::new("Marge sur le jeu minimal")
                .with_expression("jeu minimal calculé - jeu minimal demandé")
                .with_value(format!(
                    "{} - {} = {}",
                    um(fit.min_clearance),
                    um(required_min),
                    signed(margin)
                )),
        );
    }
    if let (Some(required_max), Some(margin)) = (requirement.max_clearance, margins.upper) {
        steps.push(
            ReasoningStep::new("Marge sur le jeu maximal")
                .with_expression("jeu maximal demandé - jeu maximal calculé")
                .with_value(format!(
                    "{} - {} = {}",
                    um(required_max),
                    um(fit.max_clearance),
                    signed(margin)
                )),
        );
    }

    // Sans borne, il n'y a rien a verifier : le dire plutot que de conclure.
    if requirement.is_empty() {
        let conclusion = Conclusion::insufficient_data(
            "Aucune borne de jeu n'a été indiquée. MecaTool ne peut pas se prononcer sur \
             l'adéquation de cet ajustement sans savoir quel jeu vous recherchez.",
        )
        .with_steps(steps);
        return Ok(Verification {
            verdict: Verdict::InsufficientData,
            margins,
            conclusion,
        });
    }

    let respects_all = margins.respects_all();
    // Les deux plages se rencontrent-elles, ne serait-ce qu'en partie ?
    let overlaps = requirement
        .min_clearance
        .is_none_or(|m| fit.max_clearance >= m)
        && requirement
            .max_clearance
            .is_none_or(|m| fit.min_clearance <= m);

    let (verdict, detail) = if respects_all {
        (
            Verdict::Compatible,
            format!(
                "La plage de jeu calculée ({calculated}) reste entièrement dans votre fenêtre \
                 fonctionnelle ({required}). Toute pièce conforme au plan conviendra."
            ),
        )
    } else if overlaps {
        (Verdict::Caution, partial_detail(fit, requirement, &margins))
    } else {
        (
            Verdict::Incompatible,
            format!(
                "La plage de jeu calculée ({calculated}) ne rencontre jamais votre fenêtre \
                 fonctionnelle ({required}). Aucune pièce conforme au plan ne conviendra."
            ),
        )
    };

    Ok(Verification {
        verdict,
        margins,
        conclusion: Conclusion::new(verdict, detail).with_steps(steps),
    })
}

/// Enonce precis de ce qui coince quand la solution ne convient qu'en partie.
fn partial_detail(fit: &Fit, requirement: &ClearanceRequirement, margins: &Margins) -> String {
    let below = margins.lower.is_some_and(|m| m.is_negative());
    let above = margins.upper.is_some_and(|m| m.is_negative());

    match (below, above) {
        (true, true) => format!(
            "La plage calculée déborde des deux côtés : le jeu peut descendre à {} \
             (minimum demande {}) et monter à {} (maximum demande {}).",
            um(fit.min_clearance),
            um(requirement.min_clearance.unwrap_or(Length::ZERO)),
            um(fit.max_clearance),
            um(requirement.max_clearance.unwrap_or(Length::ZERO)),
        ),
        (true, false) => format!(
            "Le jeu maximal est respecté, mais le jeu peut descendre à {}, soit {} \
             sous le minimum demandé de {}.",
            um(fit.min_clearance),
            um(margins.lower.unwrap_or(Length::ZERO).abs()),
            um(requirement.min_clearance.unwrap_or(Length::ZERO)),
        ),
        (false, true) => format!(
            "Le jeu minimal est respecté, mais le jeu peut monter à {}, soit {} \
             au-delà du maximum demandé de {}.",
            um(fit.max_clearance),
            um(margins.upper.unwrap_or(Length::ZERO).abs()),
            um(requirement.max_clearance.unwrap_or(Length::ZERO)),
        ),
        (false, false) => "La solution respecte les bornes demandées.".to_string(),
    }
}

fn signed(value: Length) -> String {
    if value.is_negative() {
        format!("-{}", um(value.abs()))
    } else {
        format!("+{}", um(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso286::Iso286Engine;
    use mecatool_core::{ToleranceClass, Unit};

    fn mm(v: &str) -> Length {
        Length::parse(v, Unit::Millimetre).unwrap()
    }

    fn um_of(v: i64) -> Length {
        Length::from_micrometres(v)
    }

    fn fit_of(nominal: &str, hole: &str, shaft: &str) -> Fit {
        Iso286Engine::new()
            .unwrap()
            .fit(
                mm(nominal),
                ToleranceClass::parse(hole).unwrap(),
                ToleranceClass::parse(shaft).unwrap(),
            )
            .unwrap()
            .fit
    }

    fn requirement(nominal: &str, min: Option<i64>, max: Option<i64>) -> ClearanceRequirement {
        ClearanceRequirement::new(mm(nominal), min.map(um_of), max.map(um_of)).unwrap()
    }

    #[test]
    fn une_solution_entierement_dans_la_fenetre_est_compatible() {
        // H7/g6 a Ø20 donne 7..41 µm. Fenetre demandee 5..50 µm.
        let fit = fit_of("20", "H7", "g6");
        let v = verify_clearance(&fit, &requirement("20", Some(5), Some(50))).unwrap();
        assert_eq!(v.verdict, Verdict::Compatible);
        assert_eq!(v.margins.lower, Some(um_of(2)));
        assert_eq!(v.margins.upper, Some(um_of(9)));
        assert!(v.margins.respects_all());
        assert!(v.conclusion.detail.contains("entièrement"));
    }

    #[test]
    fn un_depassement_par_le_haut_donne_attention() {
        // H7/g6 a Ø20 : 7..41 µm. Fenetre 5..30 µm : le maximum est depasse.
        let fit = fit_of("20", "H7", "g6");
        let v = verify_clearance(&fit, &requirement("20", Some(5), Some(30))).unwrap();
        assert_eq!(v.verdict, Verdict::Caution);
        assert_eq!(v.margins.lower, Some(um_of(2)));
        assert_eq!(v.margins.upper, Some(um_of(-11)));
        assert!(!v.margins.respects_all());
        assert!(
            v.conclusion.detail.contains("jeu minimal est respecté"),
            "detail : {}",
            v.conclusion.detail
        );
    }

    #[test]
    fn un_depassement_par_le_bas_donne_attention() {
        // H7/h6 a Ø20 : 0..34 µm. Fenetre 10..50 µm : le minimum n'est pas tenu.
        let fit = fit_of("20", "H7", "h6");
        let v = verify_clearance(&fit, &requirement("20", Some(10), Some(50))).unwrap();
        assert_eq!(v.verdict, Verdict::Caution);
        assert_eq!(v.margins.lower, Some(um_of(-10)));
        assert!(v.conclusion.detail.contains("jeu maximal est respecté"));
    }

    #[test]
    fn deux_plages_disjointes_donnent_non_compatible() {
        // H7/h6 a Ø20 : 0..34 µm. Fenetre 50..80 µm : aucune rencontre.
        let fit = fit_of("20", "H7", "h6");
        let v = verify_clearance(&fit, &requirement("20", Some(50), Some(80))).unwrap();
        assert_eq!(v.verdict, Verdict::Incompatible);
        assert!(v.conclusion.detail.contains("ne rencontre jamais"));
    }

    #[test]
    fn un_ajustement_serre_face_a_une_exigence_de_jeu_est_non_compatible() {
        // H7/p6 a Ø20 : jeu -22..+9... verifions plutot un cas franchement serre.
        let fit = fit_of("20", "H7", "p6");
        let v = verify_clearance(&fit, &requirement("20", Some(20), Some(40))).unwrap();
        assert_eq!(v.verdict, Verdict::Incompatible);
    }

    #[test]
    fn une_borne_seule_suffit_a_conclure() {
        let fit = fit_of("20", "H7", "g6");
        // Uniquement un jeu minimal : 7 >= 5, donc compatible.
        let v = verify_clearance(&fit, &requirement("20", Some(5), None)).unwrap();
        assert_eq!(v.verdict, Verdict::Compatible);
        assert_eq!(v.margins.upper, None);

        // Uniquement un jeu maximal : 41 > 30, donc insuffisant.
        let v = verify_clearance(&fit, &requirement("20", None, Some(30))).unwrap();
        assert_eq!(v.verdict, Verdict::Caution);
        assert_eq!(v.margins.lower, None);
    }

    #[test]
    fn sans_aucune_borne_le_moteur_refuse_de_conclure() {
        let fit = fit_of("20", "H7", "g6");
        let v = verify_clearance(&fit, &requirement("20", None, None)).unwrap();
        assert_eq!(v.verdict, Verdict::InsufficientData);
        assert!(v.conclusion.detail.contains("ne peut pas se prononcer"));
    }

    #[test]
    fn une_exigence_contradictoire_est_refusee() {
        let err =
            ClearanceRequirement::new(mm("20"), Some(um_of(30)), Some(um_of(10))).unwrap_err();
        assert!(matches!(err, EngineError::ContradictoryRequirement { .. }));
    }

    #[test]
    fn une_exigence_portant_sur_un_autre_nominal_est_refusee() {
        let fit = fit_of("20", "H7", "g6");
        let err = verify_clearance(&fit, &requirement("25", Some(5), Some(50))).unwrap_err();
        assert!(matches!(err, EngineError::ContradictoryRequirement { .. }));
    }

    #[test]
    fn le_cas_du_cahier_des_charges() {
        // Besoin 5..20 µm, solution 3..25 µm -> non compatible par les deux bouts.
        // On le reproduit avec H8/f7 a Ø10 pour rester sur des valeurs reelles.
        let fit = fit_of("20", "H7", "g6");
        let v = verify_clearance(&fit, &requirement("20", Some(10), Some(30))).unwrap();
        assert_eq!(v.verdict, Verdict::Caution);
        // Le « pourquoi » doit contenir les deux grandeurs comparees.
        let rendered: Vec<String> = v
            .conclusion
            .why
            .iter()
            .filter_map(|s| s.value.clone())
            .collect();
        assert!(rendered.iter().any(|v| v.contains("10")));
        assert!(rendered.iter().any(|v| v.contains("41")));
    }

    #[test]
    fn la_marge_la_plus_serree_est_celle_qui_limite() {
        let margins = Margins {
            lower: Some(um_of(2)),
            upper: Some(um_of(9)),
        };
        assert_eq!(margins.tightest(), Some(um_of(2)));

        let margins = Margins {
            lower: None,
            upper: Some(um_of(9)),
        };
        assert_eq!(margins.tightest(), Some(um_of(9)));

        assert_eq!(Margins::default().tightest(), None);
    }
}
