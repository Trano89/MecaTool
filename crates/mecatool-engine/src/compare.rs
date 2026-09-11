//! Comparaison de plusieurs ajustements sur une meme dimension.
//!
//! Comparer, ce n'est pas calculer quatre fois : c'est mettre les resultats sur
//! la **meme echelle**, dans le **meme ordre**, avec les **memes colonnes**.
//! Ce module ne fait rien d'autre que garantir ces trois choses.
//!
//! L'ordre est celui de la saisie. Reclasser sans qu'on l'ait demande ferait
//! perdre l'intention : quand on ecrit « H7/g6, H7/k6, H7/p6 », on veut voir la
//! progression du jeu vers le serrage, pas un classement par largeur.

use serde::{Deserialize, Serialize};

use mecatool_core::{Feature, Fit, Length, Provenance, ToleranceClass, Unit};

use crate::diagram::{comparison_diagram, Diagram, DiagramOptions};
use crate::error::{EngineError, Result};
use crate::iso286::Iso286Engine;
use crate::requirement::{verify_clearance, ClearanceRequirement, Verification};

/// Un ajustement, tel qu'il figure dans le comparatif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparedFit {
    pub fit: Fit,
    pub designation: String,
    /// Enonce du type d'ajustement, pastille comprise.
    pub classification: String,
    /// `IT(alesage) + IT(arbre)` : la dispersion a tenir en fabrication.
    pub span: Length,
    /// Le verdict, present seulement si une exigence a ete fournie.
    pub verification: Option<Verification>,
}

/// Le comparatif complet.
///
/// `Eq` est absent volontairement : le diagramme porte des coordonnees
/// flottantes, qui n'admettent pas d'egalite totale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitComparison {
    pub nominal: Length,
    /// Dans l'ordre de la saisie.
    pub entries: Vec<ComparedFit>,
    /// Toutes les zones sur une echelle unique.
    pub diagram: Diagram,
    pub requirement: Option<ClearanceRequirement>,
    pub provenance: Provenance,
    /// Ce que le comparatif fait ressortir, en une phrase.
    pub summary: String,
}

impl FitComparison {
    /// L'ajustement de plus faible dispersion : le plus exigeant a fabriquer.
    pub fn tightest(&self) -> Option<&ComparedFit> {
        self.entries
            .iter()
            .min_by_key(|entry| entry.span.nanometres())
    }

    /// L'ajustement de plus forte dispersion.
    pub fn widest(&self) -> Option<&ComparedFit> {
        self.entries
            .iter()
            .max_by_key(|entry| entry.span.nanometres())
    }
}

/// Nombre maximal d'ajustements comparables d'un coup.
///
/// Au-dela, le dessin devient illisible : chaque zone n'aurait plus que quelques
/// pixels de large, et le comparatif perdrait ce qui en fait l'interet.
pub const MAX_COMPARED: usize = 6;

/// Compare plusieurs ajustements sur une meme dimension nominale.
pub fn compare_fits(
    engine: &Iso286Engine,
    nominal: Length,
    pairs: &[(ToleranceClass, ToleranceClass)],
    requirement: Option<&ClearanceRequirement>,
    options: &DiagramOptions,
) -> Result<FitComparison> {
    if pairs.is_empty() {
        return Err(EngineError::Unparsable {
            input: String::new(),
            hint: "Indiquez au moins deux ajustements à comparer, par exemple \
                   « Ø20 H7/g6, H7/k6 »."
                .to_string(),
        });
    }
    if pairs.len() > MAX_COMPARED {
        return Err(EngineError::Ambiguous {
            input: format!("{} ajustements", pairs.len()),
            question: format!(
                "MecaTool compare au plus {MAX_COMPARED} ajustements à la fois : au-delà, \
                 les zones deviennent trop étroites pour être lues. Lesquels retenir ?"
            ),
        });
    }

    let mut entries = Vec::with_capacity(pairs.len());
    let mut fits = Vec::with_capacity(pairs.len());

    for (hole, shaft) in pairs {
        if hole.feature != Feature::Hole || shaft.feature != Feature::Shaft {
            return Err(EngineError::NotAFitPair {
                first: hole.to_string(),
                second: shaft.to_string(),
            });
        }
        let analysis = engine.fit(nominal, *hole, *shaft)?;
        let verification = match requirement {
            Some(requirement) => Some(verify_clearance(&analysis.fit, requirement)?),
            None => None,
        };

        entries.push(ComparedFit {
            designation: analysis.fit.designation(),
            classification: analysis.classification_fr(),
            span: analysis.fit.clearance_span(),
            verification,
            fit: analysis.fit.clone(),
        });
        fits.push(analysis.fit);
    }

    let summary = summarise(&entries);
    Ok(FitComparison {
        nominal,
        diagram: comparison_diagram(&fits, options),
        requirement: requirement.copied(),
        provenance: engine.provenance(),
        entries,
        summary,
    })
}

/// Ce que le comparatif fait ressortir.
///
/// Une phrase, pas un paragraphe : elle doit se lire avant le tableau, pas a la
/// place. Elle ne recommande rien — sans exigence fonctionnelle, il n'y a rien a
/// recommander.
fn summarise(entries: &[ComparedFit]) -> String {
    use mecatool_core::FitKind::*;

    let count =
        |kind: mecatool_core::FitKind| entries.iter().filter(|e| e.fit.kind == kind).count();
    let (clearance, transition, interference) =
        (count(Clearance), count(Transition), count(Interference));

    let mut parts = Vec::new();
    if clearance > 0 {
        parts.push(format!("{clearance} avec jeu"));
    }
    if transition > 0 {
        parts.push(format!("{transition} incertain{}", plural(transition)));
    }
    if interference > 0 {
        parts.push(format!("{interference} avec serrage"));
    }

    let spans: Vec<i64> = entries.iter().map(|e| e.span.nanometres()).collect();
    let narrowest = spans.iter().min().copied().unwrap_or(0);
    let widest = spans.iter().max().copied().unwrap_or(0);

    let dispersion = if narrowest == widest {
        format!(
            "Tous demandent la même dispersion de {}.",
            crate::format::um(Length::from_nanometres(widest))
        )
    } else {
        format!(
            "La dispersion à tenir va de {} à {}.",
            crate::format::um(Length::from_nanometres(narrowest)),
            crate::format::um(Length::from_nanometres(widest))
        )
    };

    format!(
        "{} ajustements : {}. {dispersion}",
        entries.len(),
        parts.join(", ")
    )
}

fn plural(count: usize) -> &'static str {
    if count > 1 {
        "s"
    } else {
        ""
    }
}

/// Enonce court d'une dimension nominale, pour les titres.
pub fn nominal_label(nominal: Length) -> String {
    let rendered = nominal.to_decimal_string(Unit::Millimetre, 4);
    let trimmed = rendered.trim_end_matches('0').trim_end_matches('.');
    format!("Ø{}", if trimmed.is_empty() { "0" } else { trimmed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mecatool_core::{FitKind, Verdict};

    fn engine() -> Iso286Engine {
        Iso286Engine::new().unwrap()
    }

    fn mm(value: &str) -> Length {
        Length::parse(value, Unit::Millimetre).unwrap()
    }

    fn pair(hole: &str, shaft: &str) -> (ToleranceClass, ToleranceClass) {
        (
            ToleranceClass::parse(hole).unwrap(),
            ToleranceClass::parse(shaft).unwrap(),
        )
    }

    fn classic() -> Vec<(ToleranceClass, ToleranceClass)> {
        vec![
            pair("H7", "g6"),
            pair("H7", "h6"),
            pair("H7", "k6"),
            pair("H7", "p6"),
        ]
    }

    fn compare(pairs: &[(ToleranceClass, ToleranceClass)]) -> FitComparison {
        compare_fits(&engine(), mm("20"), pairs, None, &DiagramOptions::default()).unwrap()
    }

    /// Le scénario du cahier des charges.
    #[test]
    fn les_quatre_ajustements_classiques_se_comparent() {
        let comparison = compare(&classic());

        assert_eq!(comparison.entries.len(), 4);
        assert_eq!(
            comparison
                .entries
                .iter()
                .map(|e| e.designation.as_str())
                .collect::<Vec<_>>(),
            vec!["H7/g6", "H7/h6", "H7/k6", "H7/p6"]
        );

        // La progression du jeu vers le serrage doit être visible.
        let kinds: Vec<FitKind> = comparison.entries.iter().map(|e| e.fit.kind).collect();
        assert_eq!(kinds[0], FitKind::Clearance);
        assert_eq!(kinds[1], FitKind::Clearance);
        assert_eq!(kinds[2], FitKind::Transition);
        assert_eq!(kinds[3], FitKind::Interference);
    }

    #[test]
    fn lordre_de_saisie_est_conserve() {
        // Reclasser ferait perdre l'intention : « g6, k6, p6 » se lit comme une
        // progression, pas comme un classement par largeur.
        let reversed = vec![pair("H7", "p6"), pair("H7", "g6")];
        let comparison = compare(&reversed);
        assert_eq!(comparison.entries[0].designation, "H7/p6");
        assert_eq!(comparison.entries[1].designation, "H7/g6");
    }

    #[test]
    fn le_dessin_couvre_tous_les_ajustements_sur_une_echelle() {
        let comparison = compare(&classic());
        assert_eq!(comparison.diagram.bands.len(), 8);
        // Une seule échelle, donc un seul facteur de conversion.
        assert!(comparison.diagram.pixels_per_micrometre > 0.0);
    }

    #[test]
    fn les_extremes_de_dispersion_sont_identifies() {
        let comparison = compare(&classic());
        let tightest = comparison.tightest().unwrap();
        let widest = comparison.widest().unwrap();
        assert!(tightest.span <= widest.span);
        for entry in &comparison.entries {
            assert!(entry.span >= tightest.span);
            assert!(entry.span <= widest.span);
        }
    }

    #[test]
    fn le_resume_denombre_les_types_sans_recommander() {
        let comparison = compare(&classic());
        assert!(comparison.summary.contains("2 avec jeu"));
        assert!(comparison.summary.contains("1 incertain"));
        assert!(comparison.summary.contains("1 avec serrage"));
        // Sans exigence fonctionnelle, il n'y a rien à recommander.
        assert!(!comparison.summary.to_lowercase().contains("recommand"));
    }

    #[test]
    fn une_exigence_ajoute_un_verdict_a_chaque_ligne() {
        let requirement = ClearanceRequirement::new(
            mm("20"),
            Some(Length::from_micrometres(5)),
            Some(Length::from_micrometres(50)),
        )
        .unwrap();
        let comparison = compare_fits(
            &engine(),
            mm("20"),
            &classic(),
            Some(&requirement),
            &DiagramOptions::default(),
        )
        .unwrap();

        assert!(comparison.entries.iter().all(|e| e.verification.is_some()));
        // H7/g6 donne 7..41 µm : il tient dans 5..50.
        assert_eq!(
            comparison.entries[0].verification.as_ref().unwrap().verdict,
            Verdict::Compatible
        );
        // H7/p6 serre : il ne peut pas satisfaire une exigence de jeu.
        assert_eq!(
            comparison.entries[3].verification.as_ref().unwrap().verdict,
            Verdict::Incompatible
        );
        assert_eq!(comparison.requirement, Some(requirement));
    }

    #[test]
    fn comparer_moins_de_deux_ajustements_demande_une_liste() {
        let error =
            compare_fits(&engine(), mm("20"), &[], None, &DiagramOptions::default()).unwrap_err();
        assert!(matches!(error, EngineError::Unparsable { .. }));
    }

    #[test]
    fn au_dela_de_six_le_dessin_deviendrait_illisible() {
        let many: Vec<_> = ["g6", "h6", "k6", "m6", "n6", "p6", "f7"]
            .iter()
            .map(|shaft| pair("H7", shaft))
            .collect();
        let error =
            compare_fits(&engine(), mm("20"), &many, None, &DiagramOptions::default()).unwrap_err();
        match error {
            EngineError::Ambiguous { question, .. } => {
                assert!(question.contains("trop étroites"), "question : {question}");
            }
            other => panic!("une question était attendue, obtenu {other}"),
        }
    }

    #[test]
    fn deux_elements_de_meme_nature_sont_refuses() {
        let error = compare_fits(
            &engine(),
            mm("20"),
            &[(
                ToleranceClass::parse("H7").unwrap(),
                ToleranceClass::parse("G6").unwrap(),
            )],
            None,
            &DiagramOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::NotAFitPair { .. }));
    }

    #[test]
    fn le_libelle_de_dimension_est_compact() {
        assert_eq!(nominal_label(mm("20")), "Ø20");
        assert_eq!(nominal_label(mm("25.4")), "Ø25.4");
    }
}
