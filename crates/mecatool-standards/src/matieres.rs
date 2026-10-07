//! Matieres : designation des aciers (EN 10027-1), aciers de construction
//! (EN 10025-2), proprietes physiques par famille.
//!
//! # Regles, tables, ordres de grandeur
//!
//! Trois natures de donnees, comme ailleurs dans MecaTool :
//!
//! * la **designation** des aciers est une *regle* : `S355J2` se decompose, il
//!   ne se cherche pas dans une liste. Le fichier porte les lettres, les codes
//!   et les facteurs ; le moteur les applique ;
//! * les **caracteristiques mecaniques** des aciers de construction sont une
//!   *table*, par nuance et par epaisseur ;
//! * les **proprietes physiques** sont des *ordres de grandeur* par famille,
//!   sans caractere normatif.
//!
//! Les trois sont saisis sans document ouvert : ils portent
//! [`VerificationStatus::Unverified`].
//!
//! [`VerificationStatus::Unverified`]: mecatool_core::VerificationStatus::Unverified

use std::collections::BTreeSet;
use std::sync::OnceLock;

use mecatool_core::{Length, StandardReference};
use serde::{Deserialize, Serialize};

use crate::error::{Result, StandardsError};

/* ------------------------------------------------------------------ */
/*  Designation — EN 10027-1                                           */
/* ------------------------------------------------------------------ */

/// Ce que porte le nombre d'un groupe d'emploi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupNumber {
    /// Limite d'elasticite minimale, en MPa.
    Yield,
    /// Resistance a la traction nominale, en MPa.
    Tensile,
    /// Durete Brinell minimale.
    Hardness,
}

/// Un groupe d'aciers designes par leur emploi : `S`, `E`, `P`...
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseGroup {
    pub letter: String,
    pub name: String,
    pub number: GroupNumber,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactEnergy {
    pub letter: String,
    pub joules: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactTemperature {
    pub code: String,
    pub celsius: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactCodes {
    pub energies: Vec<ImpactEnergy>,
    pub temperatures: Vec<ImpactTemperature>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suffix {
    pub code: String,
    pub meaning: String,
}

/// Les elements qui partagent un facteur de teneur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlloyFactor {
    pub factor: u32,
    pub elements: Vec<String>,
}

/// Les regles de la designation symbolique.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DesignationRules {
    dataset: String,
    standard: StandardReference,
    use_groups: Vec<UseGroup>,
    impact: ImpactCodes,
    suffixes: Vec<Suffix>,
    non_alloy_suffixes: Vec<Suffix>,
    alloy_factors: Vec<AlloyFactor>,
    high_speed_order: Vec<String>,
}

const DESIGNATION_EMBEDDED: &str =
    include_str!("../../../data/matieres/en10027-1.designation.json");

impl DesignationRules {
    pub fn embedded() -> Result<&'static DesignationRules> {
        static CACHE: OnceLock<core::result::Result<DesignationRules, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| DesignationRules::parse(DESIGNATION_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<DesignationRules> {
        let rules: DesignationRules =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "en10027-1.designation".into(),
                detail: source.to_string(),
            })?;
        rules.validate()?;
        Ok(rules)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        let mut letters = BTreeSet::new();
        for group in &self.use_groups {
            if group.letter.chars().count() != 1 || !letters.insert(group.letter.as_str()) {
                return Err(bad(format!("lettre de groupe invalide : {}", group.letter)));
            }
        }
        // Un element ne porte qu'un seul facteur : sinon « Cr4 » se lirait de
        // deux facons.
        let mut elements = BTreeSet::new();
        let mut previous = 0;
        for factor in &self.alloy_factors {
            if factor.factor <= previous {
                return Err(bad("facteurs non croissants".into()));
            }
            previous = factor.factor;
            for element in &factor.elements {
                if !is_symbol(element) || !elements.insert(element.as_str()) {
                    return Err(bad(format!("element invalide ou en double : {element}")));
                }
            }
        }
        for element in &self.high_speed_order {
            if !elements.contains(element.as_str()) {
                return Err(bad(format!("element d'acier rapide inconnu : {element}")));
            }
        }
        // Les codes de temperature sont uniques, et les energies croissent.
        let mut codes = BTreeSet::new();
        for t in &self.impact.temperatures {
            if !codes.insert(t.code.as_str()) {
                return Err(bad(format!("code de temperature en double : {}", t.code)));
            }
        }
        if self
            .impact
            .energies
            .windows(2)
            .any(|w| w[1].joules <= w[0].joules)
        {
            return Err(bad("energies de rupture non croissantes".into()));
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn use_groups(&self) -> &[UseGroup] {
        &self.use_groups
    }

    pub fn use_group(&self, letter: &str) -> Option<&UseGroup> {
        self.use_groups.iter().find(|g| g.letter == letter)
    }

    pub fn impact(&self) -> &ImpactCodes {
        &self.impact
    }

    pub fn suffixes(&self) -> &[Suffix] {
        &self.suffixes
    }

    pub fn suffix(&self, code: &str) -> Option<&Suffix> {
        self.suffixes.iter().find(|s| s.code == code)
    }

    /// Les symboles additionnels des aciers non allies : `C45E`.
    pub fn non_alloy_suffix(&self, code: &str) -> Option<&Suffix> {
        self.non_alloy_suffixes.iter().find(|s| s.code == code)
    }

    /// Le facteur de teneur d'un element, s'il en a un.
    pub fn factor(&self, element: &str) -> Option<u32> {
        self.alloy_factors
            .iter()
            .find(|f| f.elements.iter().any(|e| e == element))
            .map(|f| f.factor)
    }

    /// Tous les symboles d'elements connus, du plus long au plus court : c'est
    /// l'ordre de reconnaissance, pour que « Cr » ne soit pas lu « C » + « r ».
    pub fn elements(&self) -> Vec<&str> {
        let mut all: Vec<&str> = self
            .alloy_factors
            .iter()
            .flat_map(|f| f.elements.iter().map(String::as_str))
            .collect();
        all.sort_by_key(|e| std::cmp::Reverse(e.len()));
        all
    }

    pub fn high_speed_order(&self) -> &[String] {
        &self.high_speed_order
    }
}

fn is_symbol(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
        && chars.clone().count() <= 1
        && chars.all(|c| c.is_ascii_lowercase())
}

/* ------------------------------------------------------------------ */
/*  Aciers de construction — EN 10025-2                                */
/* ------------------------------------------------------------------ */

/// Un echelon d'epaisseur : borne basse exclue, borne haute incluse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThicknessBand {
    pub above: Length,
    pub to: Length,
}

impl ThicknessBand {
    pub fn contains(&self, t: Length) -> bool {
        t > self.above && t <= self.to
    }
}

/// Les caracteristiques d'une nuance sur un echelon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradeRow {
    pub band: ThicknessBand,
    pub yield_mpa: u32,
    pub tensile_min_mpa: u32,
    pub tensile_max_mpa: u32,
}

/// Une nuance d'acier de construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralGrade {
    /// `S355`.
    pub grade: String,
    /// `JR`, `J0`, `J2`, `K2`.
    pub qualities: Vec<String>,
    pub rows: Vec<GradeRow>,
}

#[derive(Debug, Deserialize)]
struct RawBand {
    above: i64,
    to: i64,
}

#[derive(Debug, Deserialize)]
struct RawGrade {
    grade: String,
    qualities: Vec<String>,
    yield_mpa: Vec<u32>,
    tensile_mpa: Vec<[u32; 2]>,
}

#[derive(Debug, Deserialize)]
struct RawStructural {
    dataset: String,
    standard: StandardReference,
    tensile_from_mm: i64,
    thickness_ranges: Vec<RawBand>,
    grades: Vec<RawGrade>,
}

/// La table des aciers de construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralSteelTable {
    dataset: String,
    standard: StandardReference,
    tensile_from: Length,
    grades: Vec<StructuralGrade>,
}

const STRUCTURAL_EMBEDDED: &str =
    include_str!("../../../data/matieres/en10025-2.aciers-construction.json");

impl StructuralSteelTable {
    pub fn embedded() -> Result<&'static StructuralSteelTable> {
        static CACHE: OnceLock<core::result::Result<StructuralSteelTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| StructuralSteelTable::parse(STRUCTURAL_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<StructuralSteelTable> {
        let raw: RawStructural =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "en10025-2.aciers-construction".into(),
                detail: source.to_string(),
            })?;
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: raw.dataset.clone(),
            detail,
        };
        let bands: Vec<ThicknessBand> = raw
            .thickness_ranges
            .iter()
            .map(|b| ThicknessBand {
                above: Length::from_millimetres(b.above),
                to: Length::from_millimetres(b.to),
            })
            .collect();
        // Echelons contigus et croissants, comme dans les tables ISO.
        if bands.first().map(|b| b.above) != Some(Length::ZERO) {
            return Err(bad("le premier echelon doit partir de 0".into()));
        }
        for pair in bands.windows(2) {
            if pair[1].above != pair[0].to || pair[1].to <= pair[1].above {
                return Err(bad("echelons d'epaisseur non contigus".into()));
            }
        }

        let mut grades = Vec::new();
        for g in &raw.grades {
            if g.yield_mpa.len() != bands.len() || g.tensile_mpa.len() != bands.len() {
                return Err(bad(format!("{} : ligne incomplete", g.grade)));
            }
            // Le nombre de la designation est la limite de la plus petite
            // epaisseur : S355 donne 355 MPa sur le premier echelon.
            let designated: u32 = g
                .grade
                .trim_start_matches('S')
                .parse()
                .map_err(|_| bad(format!("nuance illisible : {}", g.grade)))?;
            if g.yield_mpa[0] != designated {
                return Err(bad(format!(
                    "{} : la premiere limite d'elasticite ({}) contredit la designation",
                    g.grade, g.yield_mpa[0]
                )));
            }
            // Elle decroit quand l'epaisseur croit, sans jamais remonter.
            if g.yield_mpa.windows(2).any(|w| w[1] > w[0]) {
                return Err(bad(format!(
                    "{} : la limite d'elasticite remonte avec l'epaisseur",
                    g.grade
                )));
            }
            for [min, max] in &g.tensile_mpa {
                if min >= max {
                    return Err(bad(format!("{} : plage de resistance vide", g.grade)));
                }
            }
            grades.push(StructuralGrade {
                grade: g.grade.clone(),
                qualities: g.qualities.clone(),
                rows: bands
                    .iter()
                    .zip(&g.yield_mpa)
                    .zip(&g.tensile_mpa)
                    .map(|((band, y), [min, max])| GradeRow {
                        band: *band,
                        yield_mpa: *y,
                        tensile_min_mpa: *min,
                        tensile_max_mpa: *max,
                    })
                    .collect(),
            });
        }
        // Une nuance plus resistante l'est sur tous les echelons.
        for pair in grades.windows(2) {
            for (a, b) in pair[0].rows.iter().zip(&pair[1].rows) {
                if b.yield_mpa <= a.yield_mpa {
                    return Err(bad(format!(
                        "{} n'est pas plus resistante que {}",
                        pair[1].grade, pair[0].grade
                    )));
                }
            }
        }
        Ok(StructuralSteelTable {
            dataset: raw.dataset,
            standard: raw.standard,
            tensile_from: Length::from_millimetres(raw.tensile_from_mm),
            grades,
        })
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn grades(&self) -> &[StructuralGrade] {
        &self.grades
    }

    pub fn grade(&self, grade: &str) -> Option<&StructuralGrade> {
        self.grades.iter().find(|g| g.grade == grade)
    }

    /// L'epaisseur en dessous de laquelle la resistance tabulee ne vaut pas.
    pub fn tensile_from(&self) -> Length {
        self.tensile_from
    }
}

/* ------------------------------------------------------------------ */
/*  Proprietes physiques — ordres de grandeur                          */
/* ------------------------------------------------------------------ */

/// Une famille de materiaux et ses proprietes representatives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialFamily {
    pub id: String,
    pub name: String,
    /// Module d'elasticite, en GPa.
    pub e_gpa: u32,
    /// Coefficient de Poisson, en milliemes.
    pub poisson_milli: u32,
    /// Masse volumique, en kg/m³.
    pub density: u32,
    /// Coefficient de dilatation, en dixiemes de µm/(m·K) : 120 pour 12.
    pub alpha_tenths: u32,
}

#[derive(Debug, Deserialize)]
struct RawFamilies {
    dataset: String,
    standard: StandardReference,
    families: Vec<MaterialFamily>,
}

/// Les familles embarquees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialFamilyTable {
    standard: StandardReference,
    families: Vec<MaterialFamily>,
}

const FAMILIES_EMBEDDED: &str = include_str!("../../../data/matieres/proprietes-physiques.json");

impl MaterialFamilyTable {
    pub fn embedded() -> Result<&'static MaterialFamilyTable> {
        static CACHE: OnceLock<core::result::Result<MaterialFamilyTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| MaterialFamilyTable::parse(FAMILIES_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<MaterialFamilyTable> {
        let raw: RawFamilies =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "proprietes-physiques".into(),
                detail: source.to_string(),
            })?;
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: raw.dataset.clone(),
            detail,
        };
        let mut ids = BTreeSet::new();
        for family in &raw.families {
            if !ids.insert(family.id.as_str()) {
                return Err(bad(format!("famille en double : {}", family.id)));
            }
            // Bornes physiques larges : elles ne valident pas une valeur, elles
            // attrapent une unite oubliee (210 000 au lieu de 210 GPa).
            if !(1..=500).contains(&family.e_gpa)
                || !(100..500).contains(&family.poisson_milli)
                || !(500..=25_000).contains(&family.density)
                || !(1..=3000).contains(&family.alpha_tenths)
            {
                return Err(bad(format!(
                    "{} : valeur hors des bornes physiques",
                    family.id
                )));
            }
        }
        Ok(MaterialFamilyTable {
            standard: raw.standard,
            families: raw.families,
        })
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn families(&self) -> &[MaterialFamily] {
        &self.families
    }

    pub fn family(&self, id: &str) -> Option<&MaterialFamily> {
        self.families.iter().find(|f| f.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> &'static DesignationRules {
        DesignationRules::embedded().expect("les regles de designation doivent charger")
    }

    fn structural() -> &'static StructuralSteelTable {
        StructuralSteelTable::embedded().expect("la table EN 10025-2 doit charger")
    }

    fn families() -> &'static MaterialFamilyTable {
        MaterialFamilyTable::embedded().expect("les familles doivent charger")
    }

    #[test]
    fn les_trois_jeux_chargent_sans_se_dire_verifies() {
        for reference in [
            rules().standard(),
            structural().standard(),
            families().standard(),
        ] {
            assert!(!reference.verification.is_verified(), "{}", reference.id);
        }
    }

    #[test]
    fn les_facteurs_de_teneur() {
        assert_eq!(rules().factor("Cr"), Some(4));
        assert_eq!(rules().factor("Mo"), Some(10));
        assert_eq!(rules().factor("S"), Some(100));
        assert_eq!(rules().factor("B"), Some(1000));
        assert_eq!(rules().factor("Fe"), None);
        // Les symboles longs passent d'abord : « Cr » avant « C ».
        assert_eq!(rules().elements()[0].len(), 2);
    }

    #[test]
    fn un_element_a_deux_facteurs_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(DESIGNATION_EMBEDDED).unwrap();
        broken["alloy_factors"][1]["elements"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("Cr"));
        let err = DesignationRules::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("en double"), "{err}");
    }

    #[test]
    fn la_limite_delasticite_decroit_avec_lepaisseur() {
        let s355 = structural().grade("S355").unwrap();
        assert_eq!(s355.rows[0].yield_mpa, 355);
        assert_eq!(s355.rows[1].yield_mpa, 345);
        assert!(s355.rows[1].band.contains(Length::from_millimetres(20)));
        assert!(!s355.rows[0].band.contains(Length::from_millimetres(20)));
    }

    #[test]
    fn une_limite_qui_remonte_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(STRUCTURAL_EMBEDDED).unwrap();
        broken["grades"][2]["yield_mpa"][3] = serde_json::json!(350);
        let err = StructuralSteelTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("remonte"), "{err}");
    }

    #[test]
    fn la_premiere_limite_est_celle_de_la_designation() {
        let mut broken: serde_json::Value = serde_json::from_str(STRUCTURAL_EMBEDDED).unwrap();
        broken["grades"][0]["yield_mpa"][0] = serde_json::json!(240);
        let err = StructuralSteelTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            err.to_string().contains("contredit la designation"),
            "{err}"
        );
    }

    #[test]
    fn une_unite_oubliee_est_attrapee() {
        let mut broken: serde_json::Value = serde_json::from_str(FAMILIES_EMBEDDED).unwrap();
        broken["families"][0]["e_gpa"] = serde_json::json!(210000);
        let err = MaterialFamilyTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("bornes physiques"), "{err}");
    }

    #[test]
    fn laluminium_se_dilate_deux_fois_plus_que_lacier() {
        // Un controle de vraisemblance, pas une valeur.
        let steel = families().family("steel").unwrap();
        let aluminium = families().family("aluminium").unwrap();
        assert!(aluminium.alpha_tenths > steel.alpha_tenths * 3 / 2);
        assert!(aluminium.density < steel.density / 2);
    }
}
