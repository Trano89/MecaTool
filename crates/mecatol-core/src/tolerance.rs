//! Ecarts, zones de tolerance et dimensions limites.
//!
//! Vocabulaire retenu, conforme a l'ISO 286-1 :
//!
//! ```text
//!                                 ecart superieur (ES / es)
//!                        +---------------------------+  <- dimension maximale
//!   ligne zero  ---------|///////  zone IT  //////////|----------------------
//!   (nominal)            +---------------------------+  <- dimension minimale
//!                                 ecart inferieur (EI / ei)
//! ```
//!
//! Majuscules pour l'alesage (`EI`, `ES`), minuscules pour l'arbre (`ei`, `es`).

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::iso_class::{Feature, ToleranceClass};
use crate::length::{Length, Unit};

/// Erreurs de construction d'une tolerance.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ToleranceError {
    #[error(
        "ecarts incoherents : l'ecart inferieur ({lower}) depasse l'ecart superieur ({upper})"
    )]
    InvertedDeviations { lower: String, upper: String },
    #[error("dimension nominale non positive : {0}")]
    NonPositiveNominal(String),
}

/// Le couple d'ecarts qui definit une zone de tolerance par rapport au nominal.
///
/// L'invariant `lower <= upper` est verifie a la construction : une zone
/// inversee n'est pas representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deviations {
    lower: Length,
    upper: Length,
}

impl Deviations {
    /// Construit un couple d'ecarts en verifiant leur ordre.
    pub fn new(lower: Length, upper: Length) -> Result<Self, ToleranceError> {
        if lower > upper {
            return Err(ToleranceError::InvertedDeviations {
                lower: lower.to_string(),
                upper: upper.to_string(),
            });
        }
        Ok(Deviations { lower, upper })
    }

    /// Construit depuis l'ecart inferieur et la largeur de la zone.
    pub fn from_lower_and_width(lower: Length, width: Length) -> Result<Self, ToleranceError> {
        Deviations::new(lower, lower + width)
    }

    /// Construit depuis l'ecart superieur et la largeur de la zone.
    pub fn from_upper_and_width(upper: Length, width: Length) -> Result<Self, ToleranceError> {
        Deviations::new(upper - width, upper)
    }

    /// Zone symetrique `+/- half`, comme pour `js`/`JS` ou une cote `20 +/- 0,05`.
    pub fn symmetric(half: Length) -> Self {
        let half = half.abs();
        Deviations {
            lower: -half,
            upper: half,
        }
    }

    /// Ecart inferieur : `EI` pour un alesage, `ei` pour un arbre.
    pub const fn lower(self) -> Length {
        self.lower
    }

    /// Ecart superieur : `ES` pour un alesage, `es` pour un arbre.
    pub const fn upper(self) -> Length {
        self.upper
    }

    /// Largeur de la zone, c'est-a-dire la valeur `IT`.
    pub fn width(self) -> Length {
        self.upper - self.lower
    }

    /// Nom normatif de l'ecart inferieur pour l'element donne.
    pub const fn lower_symbol(feature: Feature) -> &'static str {
        match feature {
            Feature::Hole => "EI",
            Feature::Shaft => "ei",
        }
    }

    /// Nom normatif de l'ecart superieur pour l'element donne.
    pub const fn upper_symbol(feature: Feature) -> &'static str {
        match feature {
            Feature::Hole => "ES",
            Feature::Shaft => "es",
        }
    }
}

/// Les deux dimensions limites entre lesquelles la piece est conforme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitsOfSize {
    min: Length,
    max: Length,
}

impl LimitsOfSize {
    pub fn new(min: Length, max: Length) -> Result<Self, ToleranceError> {
        if min > max {
            return Err(ToleranceError::InvertedDeviations {
                lower: min.to_string(),
                upper: max.to_string(),
            });
        }
        Ok(LimitsOfSize { min, max })
    }

    /// Dimension minimale admissible.
    pub const fn min(self) -> Length {
        self.min
    }

    /// Dimension maximale admissible.
    pub const fn max(self) -> Length {
        self.max
    }

    /// Etendue entre les deux limites.
    pub fn width(self) -> Length {
        self.max - self.min
    }

    /// Vrai si `measured` tombe dans les limites, bornes comprises.
    pub fn contains(self, measured: Length) -> bool {
        measured >= self.min && measured <= self.max
    }
}

/// Un intervalle de dimensions nominales des tables ISO : `above < D <= up_to`.
///
/// Les tables ISO 286 sont donnees par echelons ; les bornes sont exclusives en
/// bas et inclusives en haut. Un diametre de 10 mm appartient donc a l'echelon
/// « au-dessus de 6 jusqu'a 10 inclus », et non a « au-dessus de 10 ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeRange {
    pub above: Length,
    pub up_to: Length,
}

impl SizeRange {
    pub const fn new(above: Length, up_to: Length) -> Self {
        SizeRange { above, up_to }
    }

    pub fn contains(self, nominal: Length) -> bool {
        nominal > self.above && nominal <= self.up_to
    }

    /// Libelle normatif de l'echelon, par ex. `"au-dessus de 6 jusqu'a 10"`.
    pub fn label_fr(&self) -> String {
        format!(
            "au-dessus de {} jusqu'a {}",
            trim_mm(self.above),
            trim_mm(self.up_to)
        )
    }
}

fn trim_mm(value: Length) -> String {
    let rendered = value.to_decimal_string(Unit::Millimetre, 3);
    let trimmed = rendered.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

impl fmt::Display for SizeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "> {} .. {} mm", trim_mm(self.above), trim_mm(self.up_to))
    }
}

/// Le resultat complet du tolerancement d'un element : alesage ou arbre.
///
/// C'est l'objet que l'interface affiche dans un bloc « ALESAGE H7 » ou
/// « ARBRE g6 ». Il porte a la fois l'entree (nominal, classe) et tout ce qui en
/// est deduit, de sorte qu'aucun recalcul cote interface ne soit necessaire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureTolerance {
    /// Alesage ou arbre.
    pub feature: Feature,
    /// La classe demandee, par ex. `H7`.
    pub class: ToleranceClass,
    /// Dimension nominale.
    pub nominal: Length,
    /// Ecarts par rapport au nominal.
    pub deviations: Deviations,
    /// Dimensions limites resultantes.
    pub limits: LimitsOfSize,
    /// Valeur du degre de tolerance `IT` employe.
    pub it: Length,
    /// Echelon de dimensions nominales utilise dans les tables.
    pub size_range: SizeRange,
    /// Ecart fondamental, celui des deux ecarts que la lettre fixe.
    pub fundamental_deviation: Length,
}

impl FeatureTolerance {
    /// Assemble un resultat d'element a partir du nominal et des ecarts.
    pub fn assemble(
        feature: Feature,
        class: ToleranceClass,
        nominal: Length,
        deviations: Deviations,
        size_range: SizeRange,
        fundamental_deviation: Length,
    ) -> Result<Self, ToleranceError> {
        if !nominal.is_positive() {
            return Err(ToleranceError::NonPositiveNominal(nominal.to_string()));
        }
        let limits = LimitsOfSize::new(nominal + deviations.lower(), nominal + deviations.upper())?;
        Ok(FeatureTolerance {
            feature,
            class,
            nominal,
            deviations,
            limits,
            it: deviations.width(),
            size_range,
            fundamental_deviation,
        })
    }

    /// Designation complete, par ex. `"20 H7"`.
    pub fn designation(&self, unit: Unit) -> String {
        let nominal = self.nominal.to_decimal_string(unit, unit.default_decimals());
        let trimmed = nominal.trim_end_matches('0').trim_end_matches('.');
        format!("{trimmed} {}", self.class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso_class::{DeviationLetter, Grade};

    fn um(v: i64) -> Length {
        Length::from_micrometres(v)
    }

    #[test]
    fn largeur_de_zone_egale_la_difference_des_ecarts() {
        let d = Deviations::new(um(0), um(15)).unwrap();
        assert_eq!(d.width(), um(15));
        assert_eq!(d.lower(), Length::ZERO);
        assert_eq!(d.upper(), um(15));
    }

    #[test]
    fn une_zone_inversee_est_refusee() {
        let err = Deviations::new(um(10), um(-10)).unwrap_err();
        assert!(matches!(err, ToleranceError::InvertedDeviations { .. }));
    }

    #[test]
    fn zone_symetrique() {
        let d = Deviations::symmetric(um(50));
        assert_eq!(d.lower(), um(-50));
        assert_eq!(d.upper(), um(50));
        assert_eq!(d.width(), um(100));
        // Le signe fourni ne change rien : +/- 0,05 est symetrique dans les deux sens.
        assert_eq!(Deviations::symmetric(um(-50)), d);
    }

    #[test]
    fn construction_depuis_une_borne_et_la_largeur() {
        // Alesage H7 a Ou10 : EI = 0, IT7 = 15 um.
        let d = Deviations::from_lower_and_width(Length::ZERO, um(15)).unwrap();
        assert_eq!(d.upper(), um(15));
        // Arbre g6 a Ou10 : es = -5 um, IT6 = 9 um.
        let d = Deviations::from_upper_and_width(um(-5), um(9)).unwrap();
        assert_eq!(d.lower(), um(-14));
    }

    #[test]
    fn les_bornes_dechelon_sont_exclusive_puis_inclusive() {
        let range = SizeRange::new(Length::from_millimetres(6), Length::from_millimetres(10));
        assert!(range.contains(Length::from_millimetres(10)));
        assert!(range.contains(Length::from_millimetres(7)));
        assert!(!range.contains(Length::from_millimetres(6)));
        assert!(!range.contains(Length::from_millimetres(11)));
        assert_eq!(range.label_fr(), "au-dessus de 6 jusqu'a 10");
    }

    #[test]
    fn assemblage_dun_element_tolerance() {
        let class = ToleranceClass::new(
            Feature::Hole,
            DeviationLetter::H,
            Grade::from_it_number(7).unwrap(),
        );
        let ft = FeatureTolerance::assemble(
            Feature::Hole,
            class,
            Length::from_millimetres(10),
            Deviations::new(Length::ZERO, um(15)).unwrap(),
            SizeRange::new(Length::from_millimetres(6), Length::from_millimetres(10)),
            Length::ZERO,
        )
        .unwrap();

        assert_eq!(ft.limits.min(), Length::from_millimetres(10));
        assert_eq!(ft.limits.max(), Length::parse("10.015", Unit::Millimetre).unwrap());
        assert_eq!(ft.it, um(15));
        assert_eq!(ft.designation(Unit::Millimetre), "10 H7");
        assert!(ft.limits.contains(Length::parse("10.007", Unit::Millimetre).unwrap()));
        assert!(!ft.limits.contains(Length::parse("10.016", Unit::Millimetre).unwrap()));
    }

    #[test]
    fn un_nominal_nul_ou_negatif_est_refuse() {
        let class = ToleranceClass::parse("H7").unwrap();
        let err = FeatureTolerance::assemble(
            Feature::Hole,
            class,
            Length::ZERO,
            Deviations::new(Length::ZERO, um(15)).unwrap(),
            SizeRange::new(Length::ZERO, Length::from_millimetres(3)),
            Length::ZERO,
        )
        .unwrap_err();
        assert!(matches!(err, ToleranceError::NonPositiveNominal(_)));
    }

    #[test]
    fn symboles_normatifs_des_ecarts() {
        assert_eq!(Deviations::lower_symbol(Feature::Hole), "EI");
        assert_eq!(Deviations::upper_symbol(Feature::Hole), "ES");
        assert_eq!(Deviations::lower_symbol(Feature::Shaft), "ei");
        assert_eq!(Deviations::upper_symbol(Feature::Shaft), "es");
    }
}
