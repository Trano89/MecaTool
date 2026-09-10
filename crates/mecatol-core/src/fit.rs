//! Ajustements : ce qui se passe quand un arbre entre dans un alesage.
//!
//! Les deux grandeurs qui caracterisent un ajustement sont, par convention
//! ISO 286-1, exprimees comme des **jeux** signes :
//!
//! ```text
//!   jeu minimal = EI - es      (alesage au plus petit, arbre au plus grand)
//!   jeu maximal = ES - ei      (alesage au plus grand, arbre au plus petit)
//! ```
//!
//! Un jeu negatif est un serrage. Garder une seule grandeur signee, plutot que
//! deux notions concurrentes, evite les erreurs de signe : le type d'ajustement
//! se lit alors directement sur les signes des deux bornes.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::length::Length;
use crate::tolerance::FeatureTolerance;

/// Nature d'un ajustement, deduite des signes du jeu minimal et du jeu maximal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitKind {
    /// Jeu garanti : les deux bornes sont positives ou nulles.
    Clearance,
    /// Incertain : selon les pieces reelles, on obtient du jeu ou du serrage.
    Transition,
    /// Serrage garanti : les deux bornes sont negatives ou nulles.
    Interference,
}

impl FitKind {
    /// Deduit le type a partir des deux bornes signees.
    ///
    /// Les cas limites sont traites explicitement : un jeu minimal nul reste un
    /// ajustement avec jeu (cas `H/h`), un jeu maximal nul reste un ajustement
    /// avec serrage.
    pub fn classify(min_clearance: Length, max_clearance: Length) -> Self {
        if min_clearance.nanometres() >= 0 {
            FitKind::Clearance
        } else if max_clearance.nanometres() <= 0 {
            FitKind::Interference
        } else {
            FitKind::Transition
        }
    }

    pub const fn label_fr(self) -> &'static str {
        match self {
            FitKind::Clearance => "ajustement avec jeu",
            FitKind::Transition => "ajustement incertain",
            FitKind::Interference => "ajustement avec serrage",
        }
    }

    /// Pastille de statut. Jamais utilisee seule : toujours accompagnee du texte,
    /// pour rester lisible sans distinction des couleurs.
    pub const fn badge(self) -> &'static str {
        match self {
            FitKind::Clearance => "\u{1f7e2}",
            FitKind::Transition => "\u{1f7e0}",
            FitKind::Interference => "\u{1f534}",
        }
    }
}

impl fmt::Display for FitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label_fr())
    }
}

/// Un ajustement complet entre un alesage et un arbre de meme dimension nominale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fit {
    pub hole: FeatureTolerance,
    pub shaft: FeatureTolerance,
    /// `EI - es`. Negatif si l'ajustement peut serrer.
    pub min_clearance: Length,
    /// `ES - ei`. Negatif si l'ajustement serre toujours.
    pub max_clearance: Length,
    pub kind: FitKind,
}

impl Fit {
    /// Assemble un ajustement a partir des deux elements deja tolerances.
    ///
    /// Ne verifie pas que les nominaux coincident : cette verification appartient
    /// au moteur, qui sait produire un diagnostic utilisateur exploitable.
    pub fn assemble(hole: FeatureTolerance, shaft: FeatureTolerance) -> Self {
        let min_clearance = hole.limits.min() - shaft.limits.max();
        let max_clearance = hole.limits.max() - shaft.limits.min();
        let kind = FitKind::classify(min_clearance, max_clearance);
        Fit {
            hole,
            shaft,
            min_clearance,
            max_clearance,
            kind,
        }
    }

    /// Serrage maximal, positif, ou `None` si l'ajustement ne serre jamais.
    pub fn max_interference(&self) -> Option<Length> {
        if self.min_clearance.is_negative() {
            Some(-self.min_clearance)
        } else {
            None
        }
    }

    /// Serrage minimal, positif, ou `None` si l'ajustement peut ne pas serrer.
    pub fn min_interference(&self) -> Option<Length> {
        if self.max_clearance.is_negative() {
            Some(-self.max_clearance)
        } else {
            None
        }
    }

    /// Amplitude de variation du jeu, egale a `IT(alesage) + IT(arbre)`.
    ///
    /// C'est la dispersion totale que la fabrication devra accepter.
    pub fn clearance_span(&self) -> Length {
        self.max_clearance - self.min_clearance
    }

    /// Designation usuelle de l'ajustement, par ex. `"H7/g6"`.
    pub fn designation(&self) -> String {
        format!("{}/{}", self.hole.class, self.shaft.class)
    }

    /// Vrai si les deux elements portent bien la meme dimension nominale.
    pub fn nominals_match(&self) -> bool {
        self.hole.nominal == self.shaft.nominal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso_class::{Feature, ToleranceClass};
    use crate::length::{Length, Unit};
    use crate::tolerance::{Deviations, SizeRange};

    fn um(v: i64) -> Length {
        Length::from_micrometres(v)
    }

    /// Construit un element tolerance directement depuis ses ecarts, sans passer
    /// par les tables : ces tests portent sur l'algebre des ajustements seule.
    fn feature(class: &str, nominal_mm: i64, lower_um: i64, upper_um: i64) -> FeatureTolerance {
        let class = ToleranceClass::parse(class).unwrap();
        FeatureTolerance::assemble(
            class.feature,
            class,
            Length::from_millimetres(nominal_mm),
            Deviations::new(um(lower_um), um(upper_um)).unwrap(),
            SizeRange::new(Length::from_millimetres(6), Length::from_millimetres(10)),
            if class.feature == Feature::Hole {
                um(lower_um)
            } else {
                um(upper_um)
            },
        )
        .unwrap()
    }

    #[test]
    fn h7_g6_a_diametre_10_est_un_ajustement_avec_jeu() {
        // H7 : EI = 0, ES = +15 um. g6 : ei = -14, es = -5 um.
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("g6", 10, -14, -5));
        assert_eq!(fit.min_clearance, um(5));
        assert_eq!(fit.max_clearance, um(29));
        assert_eq!(fit.kind, FitKind::Clearance);
        assert_eq!(fit.max_interference(), None);
        assert_eq!(fit.clearance_span(), um(24));
        assert_eq!(fit.designation(), "H7/g6");
        assert!(fit.nominals_match());
    }

    #[test]
    fn h7_h6_a_un_jeu_minimal_nul_et_reste_un_ajustement_avec_jeu() {
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("h6", 10, -9, 0));
        assert_eq!(fit.min_clearance, Length::ZERO);
        assert_eq!(fit.max_clearance, um(24));
        assert_eq!(fit.kind, FitKind::Clearance);
    }

    #[test]
    fn un_ajustement_incertain_a_des_bornes_de_signes_opposes() {
        // k6 a Ou10 : ei = +1, es = +10 um.
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("k6", 10, 1, 10));
        assert_eq!(fit.min_clearance, um(-10));
        assert_eq!(fit.max_clearance, um(14));
        assert_eq!(fit.kind, FitKind::Transition);
        assert_eq!(fit.max_interference(), Some(um(10)));
        assert_eq!(fit.min_interference(), None);
    }

    #[test]
    fn un_ajustement_avec_serrage_a_ses_deux_bornes_negatives() {
        // Arbre volontairement plus gros que l'alesage sur toute la plage.
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("u6", 10, 20, 29));
        assert_eq!(fit.min_clearance, um(-29));
        assert_eq!(fit.max_clearance, um(-5));
        assert_eq!(fit.kind, FitKind::Interference);
        assert_eq!(fit.max_interference(), Some(um(29)));
        assert_eq!(fit.min_interference(), Some(um(5)));
    }

    #[test]
    fn lamplitude_du_jeu_vaut_la_somme_des_deux_it() {
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("g6", 10, -14, -5));
        assert_eq!(fit.clearance_span(), fit.hole.it + fit.shaft.it);
    }

    #[test]
    fn nominaux_differents_detectes() {
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("g6", 20, -20, -7));
        assert!(!fit.nominals_match());
    }

    #[test]
    fn les_limites_sont_exactes_en_millimetres() {
        let fit = Fit::assemble(feature("H7", 10, 0, 15), feature("g6", 10, -14, -5));
        assert_eq!(fit.hole.limits.max().to_decimal_string(Unit::Millimetre, 3), "10.015");
        assert_eq!(fit.shaft.limits.min().to_decimal_string(Unit::Millimetre, 3), "9.986");
        assert_eq!(fit.shaft.limits.max().to_decimal_string(Unit::Millimetre, 3), "9.995");
    }
}
