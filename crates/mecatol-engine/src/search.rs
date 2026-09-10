//! Recherche des ajustements normalises repondant a un besoin fonctionnel.
//!
//! C'est le renversement qui fait l'interet de Mecatol : au lieu de partir d'une
//! designation pour obtenir un jeu, on part du jeu voulu pour obtenir les
//! designations possibles.
//!
//! # Etendue de la recherche
//!
//! L'espace complet des combinaisons est immense et largement depourvu de sens
//! pratique. La recherche se limite donc aux deux systemes usuels :
//!
//! * **systeme de l'alesage normal** : l'alesage est un `H`, l'arbre porte la
//!   lettre qui donne le jeu voulu. C'est le systeme le plus repandu, parce
//!   qu'un alesage se realise avec un outil de dimension fixe ;
//! * **systeme de l'arbre normal** : l'arbre est un `h`, l'alesage varie. Utile
//!   quand l'arbre est un profile du commerce.
//!
//! Les degres explores et l'ecart maximal entre les deux degres sont
//! parametrables. Toute restriction appliquee est rendue dans le resultat : une
//! recherche infructueuse doit se lire « aucune solution dans ce perimetre », et
//! jamais « aucune solution n'existe ».

use serde::{Deserialize, Serialize};

use mecatol_core::{DeviationLetter, Fit, Grade, Length, Provenance, ToleranceClass, Verdict};

use crate::error::Result;
use crate::iso286::Iso286Engine;
use crate::requirement::{verify_clearance, ClearanceRequirement, Verification};

/// Lequel des deux elements sert de reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Basis {
    /// Systeme de l'alesage normal : l'alesage est un `H`.
    Hole,
    /// Systeme de l'arbre normal : l'arbre est un `h`.
    Shaft,
}

impl Basis {
    pub const fn label_fr(self) -> &'static str {
        match self {
            Basis::Hole => "système de l'alésage normal",
            Basis::Shaft => "système de l'arbre normal",
        }
    }
}

/// Perimetre de la recherche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOptions {
    pub bases: Vec<Basis>,
    /// Degre le plus fin explore, par son numero usuel.
    pub finest_grade: u8,
    /// Degre le plus grossier explore.
    pub coarsest_grade: u8,
    /// Ecart maximal admis entre le degre de l'alesage et celui de l'arbre.
    pub max_grade_difference: u8,
    /// Conserver les solutions qui ne conviennent que partiellement.
    pub include_caution: bool,
}

impl Default for SearchOptions {
    /// Perimetre usuel de la construction mecanique courante.
    ///
    /// Les degres IT5 a IT11 couvrent de l'ajustement de precision au montage
    /// libre. En dessous, on entre dans la metrologie ; au-dessus, la notion
    /// d'ajustement perd son sens. L'ecart d'un degre entre alesage et arbre
    /// est la pratique courante (`H7/g6`), l'egalite restant admise.
    fn default() -> Self {
        SearchOptions {
            bases: vec![Basis::Hole, Basis::Shaft],
            finest_grade: 5,
            coarsest_grade: 11,
            max_grade_difference: 1,
            include_caution: true,
        }
    }
}

impl SearchOptions {
    /// Enonce des restrictions appliquees, a afficher avec le resultat.
    pub fn notes_fr(&self, letters: &[DeviationLetter]) -> Vec<String> {
        let systems: Vec<&str> = self.bases.iter().map(|b| b.label_fr()).collect();
        let letter_list: Vec<&str> = letters.iter().map(|l| l.as_lower()).collect();
        vec![
            format!("Systèmes explorés : {}.", systems.join(", ")),
            format!(
                "Degrés explorés : IT{} à IT{}, avec au plus {} degré(s) d'écart entre \
                 l'alésage et l'arbre.",
                self.finest_grade, self.coarsest_grade, self.max_grade_difference
            ),
            format!(
                "Lettres disponibles dans les données : {}. Les autres lettres de \
                 l'ISO 286 ne sont pas encore saisies et n'ont donc pas été examinées.",
                letter_list.join(", ")
            ),
        ]
    }
}

/// Une solution retenue par la recherche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Solution {
    pub fit: Fit,
    pub verification: Verification,
    /// `IT(alesage) + IT(arbre)` : la dispersion totale a tenir en fabrication.
    pub total_tolerance: Length,
    pub basis: Basis,
}

impl Solution {
    pub fn designation(&self) -> String {
        self.fit.designation()
    }

    /// Ecart entre le milieu de la plage calculee et le milieu de la fenetre demandee.
    ///
    /// Plus il est faible, mieux la solution est centree sur le besoin.
    pub fn offset_from_centre(&self, requirement: &ClearanceRequirement) -> Option<Length> {
        let (min, max) = (requirement.min_clearance?, requirement.max_clearance?);
        let wanted = midpoint(min, max);
        let obtained = midpoint(self.fit.min_clearance, self.fit.max_clearance);
        Some((obtained - wanted).abs())
    }

    /// Somme des depassements de l'exigence, en valeur absolue.
    ///
    /// Nulle pour une solution pleinement compatible. C'est le critere qui
    /// ordonne les solutions partielles : celle qui s'approche le plus du besoin
    /// vient en premier.
    pub fn violation(&self) -> Length {
        let overshoot = |margin: Option<Length>| match margin {
            Some(m) if m.is_negative() => -m,
            _ => Length::ZERO,
        };
        overshoot(self.verification.margins.lower) + overshoot(self.verification.margins.upper)
    }
}

fn midpoint(a: Length, b: Length) -> Length {
    Length::from_nanometres((a.nanometres() + b.nanometres()) / 2)
}

/// Le resultat d'une recherche, avec son perimetre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    /// Solutions retenues, classees de la plus recommandable a la moins.
    pub solutions: Vec<Solution>,
    /// Nombre de combinaisons effectivement calculees.
    pub examined: usize,
    /// La plus faible dispersion rencontree, toutes combinaisons confondues.
    ///
    /// Sert a expliquer une recherche infructueuse : si meme le plus fin des
    /// ajustements disperse plus que la fenetre demandee, le probleme est la
    /// largeur du besoin, pas le choix de la lettre.
    pub narrowest_span: Option<Length>,
    pub provenance: Provenance,
    /// Restrictions appliquees, a afficher pour que l'absence de resultat se lise
    /// correctement.
    pub notes: Vec<String>,
}

impl SearchResult {
    pub fn is_empty(&self) -> bool {
        self.solutions.is_empty()
    }

    /// Explique pourquoi aucune solution ne satisfait entierement l'exigence.
    ///
    /// Rend `None` des qu'une solution compatible existe : il n'y a alors rien
    /// a expliquer.
    pub fn diagnosis_fr(&self, requirement: &ClearanceRequirement) -> Option<String> {
        if self.fully_compatible().next().is_some() {
            return None;
        }

        let (Some(min), Some(max)) = (requirement.min_clearance, requirement.max_clearance) else {
            return Some(
                "Aucune solution du périmètre ne satisfait entièrement l'exigence.".to_string(),
            );
        };
        let window = max - min;
        let Some(narrowest) = self.narrowest_span else {
            return Some(
                "Aucune combinaison n'a pu être calculée pour cette dimension nominale."
                    .to_string(),
            );
        };

        if narrowest > window {
            // Problème de largeur : géométriquement impossible.
            Some(format!(
                "La fenêtre demandée mesure {}, alors que le plus fin ajustement du périmètre \
                 disperse déjà de {}. Aucun ajustement normalisé ne peut tenir dans une fenêtre \
                 aussi étroite : il faudrait l'élargir d'au moins {}, ou explorer des degrés \
                 plus fins que ceux du périmètre.",
                crate::format::um(window),
                crate::format::um(narrowest),
                crate::format::um(narrowest - window)
            ))
        } else {
            // Problème de position : la largeur suffirait, mais aucune lettre ne
            // place la plage au bon endroit.
            let closest = match self.solutions.first() {
                Some(s) => format!(
                    "La solution la plus proche est {}, à {} de dépassement cumulé.",
                    s.designation(),
                    crate::format::um(s.violation())
                ),
                None => String::new(),
            };
            Some(format!(
                "La fenêtre demandée mesure {}, et le plus fin ajustement du périmètre disperse \
                 de {} : une solution serait géométriquement possible. Mais aucune des lettres \
                 disponibles ne positionne sa plage à l'intérieur de la fenêtre. {closest}",
                crate::format::um(window),
                crate::format::um(narrowest),
            ))
        }
    }

    /// La solution recommandee : la premiere du classement.
    pub fn recommended(&self) -> Option<&Solution> {
        self.solutions.first()
    }

    /// Les solutions qui satisfont entierement l'exigence.
    pub fn fully_compatible(&self) -> impl Iterator<Item = &Solution> {
        self.solutions
            .iter()
            .filter(|s| s.verification.verdict == Verdict::Compatible)
    }

    /// La solution la plus serree, c'est-a-dire de plus faible tolerance totale.
    ///
    /// Volontairement decrite comme « plus serree » et non « plus chere » : sans
    /// donnees de fabrication, Mecatol ne se prononce pas sur les couts.
    pub fn tightest(&self) -> Option<&Solution> {
        self.fully_compatible()
            .min_by_key(|s| s.total_tolerance.nanometres())
    }

    /// La solution la plus large, donc potentiellement la plus facile a obtenir.
    pub fn widest(&self) -> Option<&Solution> {
        self.fully_compatible()
            .max_by_key(|s| s.total_tolerance.nanometres())
    }
}

/// Cherche les ajustements normalises repondant a l'exigence.
pub fn find_fits(
    engine: &Iso286Engine,
    requirement: &ClearanceRequirement,
    options: &SearchOptions,
) -> Result<SearchResult> {
    let letters = engine.available_letters();
    let mut solutions: Vec<Solution> = Vec::new();
    let mut examined = 0usize;
    let mut narrowest_span: Option<Length> = None;

    for basis in &options.bases {
        for letter in &letters {
            for (hole_grade, shaft_grade) in grade_pairs(options) {
                let (hole_class, shaft_class) = match basis {
                    Basis::Hole => (
                        ToleranceClass::new(
                            mecatol_core::Feature::Hole,
                            DeviationLetter::H,
                            hole_grade,
                        ),
                        ToleranceClass::new(mecatol_core::Feature::Shaft, *letter, shaft_grade),
                    ),
                    Basis::Shaft => (
                        ToleranceClass::new(mecatol_core::Feature::Hole, *letter, hole_grade),
                        ToleranceClass::new(
                            mecatol_core::Feature::Shaft,
                            DeviationLetter::H,
                            shaft_grade,
                        ),
                    ),
                };

                // Une combinaison hors donnees ou hors plage n'est pas une
                // solution : on l'ignore sans bruit, l'erreur n'a de sens que
                // pour une demande explicite de l'utilisateur.
                let Ok(analysis) = engine.fit(requirement.nominal, hole_class, shaft_class) else {
                    continue;
                };
                examined += 1;

                let span = analysis.fit.clearance_span();
                narrowest_span = Some(match narrowest_span {
                    Some(current) => current.min(span),
                    None => span,
                });

                let verification = verify_clearance(&analysis.fit, requirement)?;
                let keep = match verification.verdict {
                    Verdict::Compatible => true,
                    Verdict::Caution => options.include_caution,
                    _ => false,
                };
                if !keep {
                    continue;
                }

                let designation = analysis.fit.designation();
                if solutions.iter().any(|s| s.designation() == designation) {
                    continue;
                }

                let total_tolerance = analysis.fit.clearance_span();
                solutions.push(Solution {
                    fit: analysis.fit,
                    verification,
                    total_tolerance,
                    basis: *basis,
                });
            }
        }
    }

    rank(&mut solutions, requirement);

    Ok(SearchResult {
        solutions,
        examined,
        narrowest_span,
        provenance: engine.provenance(),
        notes: options.notes_fr(&letters),
    })
}

/// Les couples de degres a explorer, dans l'ordre du plus fin au plus grossier.
fn grade_pairs(options: &SearchOptions) -> Vec<(Grade, Grade)> {
    let mut pairs = Vec::new();
    for hole in options.finest_grade..=options.coarsest_grade {
        for shaft in options.finest_grade..=options.coarsest_grade {
            if hole.abs_diff(shaft) > options.max_grade_difference {
                continue;
            }
            if let (Some(h), Some(s)) = (Grade::from_it_number(hole), Grade::from_it_number(shaft))
            {
                pairs.push((h, s));
            }
        }
    }
    pairs
}

/// Classe les solutions de la plus recommandable a la moins.
///
/// Ordre retenu :
///
/// 1. celles qui satisfont entierement l'exigence avant celles qui ne la
///    satisfont qu'en partie ;
/// 2. le plus faible depassement de l'exigence. Nul pour toutes les solutions
///    compatibles, ce critere n'ordonne en pratique que les partielles, en
///    mettant en tete celle qui s'approche le plus du besoin ;
/// 3. a depassement egal, la tolerance totale la plus large : c'est la
///    dispersion la plus facile a tenir en fabrication ;
/// 4. puis la mieux centree sur la fenetre demandee ;
/// 5. enfin la designation, pour que le classement soit reproductible.
///
/// L'ordre des criteres 2 et 3 compte. Les intervertir reviendrait a
/// recommander, parmi les solutions partielles, celle qui rate le plus la
/// cible : la dispersion la plus large est une qualite seulement lorsque
/// l'exigence est deja satisfaite.
fn rank(solutions: &mut [Solution], requirement: &ClearanceRequirement) {
    solutions.sort_by(|a, b| {
        verdict_rank(a.verification.verdict)
            .cmp(&verdict_rank(b.verification.verdict))
            .then(a.violation().cmp(&b.violation()))
            .then(b.total_tolerance.cmp(&a.total_tolerance))
            .then(
                a.offset_from_centre(requirement)
                    .cmp(&b.offset_from_centre(requirement)),
            )
            .then(a.designation().cmp(&b.designation()))
    });
}

fn verdict_rank(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Compatible => 0,
        Verdict::Caution => 1,
        Verdict::Incompatible => 2,
        Verdict::InsufficientData => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mecatol_core::Unit;

    fn engine() -> Iso286Engine {
        Iso286Engine::new().unwrap()
    }

    fn requirement(nominal: &str, min: i64, max: i64) -> ClearanceRequirement {
        ClearanceRequirement::new(
            Length::parse(nominal, Unit::Millimetre).unwrap(),
            Some(Length::from_micrometres(min)),
            Some(Length::from_micrometres(max)),
        )
        .unwrap()
    }

    /// Le scenario du cahier des charges : Ø20, jeu voulu entre 10 et 30 µm.
    #[test]
    fn le_scenario_du_cahier_des_charges() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        assert!(!result.is_empty(), "aucune solution trouvee");
        assert!(result.examined > 100, "recherche trop etroite");

        // Chaque solution pleinement compatible doit reellement tenir la fenetre.
        for solution in result.fully_compatible() {
            assert!(
                solution.fit.min_clearance >= Length::from_micrometres(10),
                "{} : jeu minimal {} sous la borne",
                solution.designation(),
                solution.fit.min_clearance
            );
            assert!(
                solution.fit.max_clearance <= Length::from_micrometres(30),
                "{} : jeu maximal {} au-dessus de la borne",
                solution.designation(),
                solution.fit.max_clearance
            );
        }
    }

    #[test]
    fn les_solutions_compatibles_precedent_les_partielles() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        let mut seen_caution = false;
        for solution in &result.solutions {
            match solution.verification.verdict {
                Verdict::Caution => seen_caution = true,
                Verdict::Compatible => assert!(
                    !seen_caution,
                    "{} : une solution compatible apparait apres une solution partielle",
                    solution.designation()
                ),
                other => panic!("verdict inattendu dans les resultats : {other:?}"),
            }
        }
    }

    #[test]
    fn la_plus_serree_et_la_plus_large_encadrent_les_autres() {
        let e = engine();
        let req = requirement("20", 5, 60);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        let tightest = result.tightest().expect("une solution la plus serree");
        let widest = result.widest().expect("une solution la plus large");
        assert!(tightest.total_tolerance <= widest.total_tolerance);

        for solution in result.fully_compatible() {
            assert!(solution.total_tolerance >= tightest.total_tolerance);
            assert!(solution.total_tolerance <= widest.total_tolerance);
        }
    }

    #[test]
    fn une_fenetre_impossible_ne_rend_aucune_solution() {
        let e = engine();
        // Un jeu de 5 mm a Ø20 n'est atteignable par aucune classe ISO 286.
        let req = ClearanceRequirement::new(
            Length::from_millimetres(20),
            Some(Length::from_millimetres(5)),
            Some(Length::from_millimetres(6)),
        )
        .unwrap();
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();
        assert!(result.is_empty());
        // ... mais le perimetre doit etre rendu, pour que l'absence se lise bien.
        assert_eq!(result.notes.len(), 3);
        assert!(result.notes[2].contains("ne sont pas encore saisies"));
    }

    #[test]
    fn exclure_les_solutions_partielles_ne_garde_que_les_compatibles() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let strict = SearchOptions {
            include_caution: false,
            ..SearchOptions::default()
        };
        let result = find_fits(&e, &req, &strict).unwrap();
        assert!(result
            .solutions
            .iter()
            .all(|s| s.verification.verdict == Verdict::Compatible));
    }

    #[test]
    fn le_systeme_de_lalesage_normal_donne_bien_des_alesages_h() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let options = SearchOptions {
            bases: vec![Basis::Hole],
            ..SearchOptions::default()
        };
        let result = find_fits(&e, &req, &options).unwrap();
        assert!(!result.is_empty());
        for solution in &result.solutions {
            assert_eq!(solution.fit.hole.class.letter, DeviationLetter::H);
        }
    }

    #[test]
    fn le_systeme_de_larbre_normal_donne_bien_des_arbres_h() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let options = SearchOptions {
            bases: vec![Basis::Shaft],
            ..SearchOptions::default()
        };
        let result = find_fits(&e, &req, &options).unwrap();
        assert!(!result.is_empty());
        for solution in &result.solutions {
            assert_eq!(solution.fit.shaft.class.letter, DeviationLetter::H);
        }
    }

    #[test]
    fn aucune_solution_nest_proposee_deux_fois() {
        let e = engine();
        let req = requirement("20", 0, 100);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();
        let mut designations: Vec<String> =
            result.solutions.iter().map(|s| s.designation()).collect();
        let before = designations.len();
        designations.sort();
        designations.dedup();
        assert_eq!(before, designations.len(), "doublons dans les solutions");
    }

    #[test]
    fn le_classement_est_reproductible() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let first = find_fits(&e, &req, &SearchOptions::default()).unwrap();
        let second = find_fits(&e, &req, &SearchOptions::default()).unwrap();
        assert_eq!(first.solutions, second.solutions);
    }

    #[test]
    fn lecart_de_degres_est_respecte() {
        let e = engine();
        let req = requirement("20", 0, 200);
        let options = SearchOptions {
            max_grade_difference: 0,
            ..SearchOptions::default()
        };
        let result = find_fits(&e, &req, &options).unwrap();
        assert!(!result.is_empty());
        for solution in &result.solutions {
            assert_eq!(
                solution.fit.hole.class.grade,
                solution.fit.shaft.class.grade,
                "{} melange deux degres",
                solution.designation()
            );
        }
    }

    /// Sans ce classement, la recherche recommandait la pire des solutions
    /// partielles : la plus large dispersion est une qualite seulement une fois
    /// l'exigence satisfaite.
    #[test]
    fn parmi_les_solutions_partielles_la_plus_proche_vient_en_tete() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        assert_eq!(
            result.fully_compatible().count(),
            0,
            "ce scenario doit rester sans solution exacte"
        );

        let violations: Vec<Length> = result.solutions.iter().map(|s| s.violation()).collect();
        for pair in violations.windows(2) {
            assert!(
                pair[0] <= pair[1],
                "les depassements ne sont pas croissants : {} puis {}",
                pair[0],
                pair[1]
            );
        }

        let best = result.recommended().unwrap();
        assert!(
            best.violation() < Length::from_micrometres(20),
            "{} depasse de {}, ce n'est pas la plus proche",
            best.designation(),
            best.violation()
        );
    }

    #[test]
    fn un_besoin_trop_etroit_est_diagnostique_comme_tel() {
        let e = engine();
        // Fenetre de 5 µm a Ø20 : plus etroite que la dispersion la plus fine.
        let req = requirement("20", 10, 15);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        let diagnosis = result.diagnosis_fr(&req).expect("un diagnostic attendu");
        assert!(
            diagnosis.contains("aussi étroite"),
            "diagnostic inattendu : {diagnosis}"
        );
        assert!(diagnosis.contains("élargir"), "diagnostic : {diagnosis}");
    }

    #[test]
    fn un_besoin_assez_large_mais_mal_place_est_diagnostique_comme_tel() {
        let e = engine();
        // Fenetre de 20 µm a Ø20 : la dispersion la plus fine vaut 18 µm, donc
        // une solution serait geometriquement possible.
        let req = requirement("20", 10, 30);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        let diagnosis = result.diagnosis_fr(&req).expect("un diagnostic attendu");
        assert!(
            diagnosis.contains("géométriquement possible"),
            "diagnostic inattendu : {diagnosis}"
        );
        assert!(diagnosis.contains("positionne"), "diagnostic : {diagnosis}");
    }

    #[test]
    fn aucun_diagnostic_lorsquune_solution_convient() {
        let e = engine();
        let req = requirement("20", 0, 60);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();
        assert!(result.fully_compatible().next().is_some());
        assert_eq!(result.diagnosis_fr(&req), None);
    }

    #[test]
    fn la_dispersion_la_plus_fine_est_relevee() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();
        // A Ø20, IT5 vaut 9 µm : deux elements en IT5 dispersent de 18 µm.
        assert_eq!(result.narrowest_span, Some(Length::from_micrometres(18)));
    }

    #[test]
    fn les_resultats_portent_la_provenance_de_leurs_donnees() {
        let e = engine();
        let req = requirement("20", 10, 30);
        let result = find_fits(&e, &req, &SearchOptions::default()).unwrap();

        assert!(
            !result.provenance.references.is_empty(),
            "recherche sans provenance"
        );
        // Les avertissements apparaissent si et seulement si une source n'est
        // pas verifiee.
        assert_eq!(
            result.provenance.is_fully_verified(),
            result.provenance.warnings_fr().is_empty()
        );
    }
}
