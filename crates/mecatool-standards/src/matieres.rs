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
//! La table EN 10025-2 a ete relue dans la norme (EN 10025-2:2019, tableaux 6
//! et 7), les regles de designation dans l'EN 10027-1:2005 (articles 4 a 7,
//! tableaux 1 a 18) : elles portent [`VerificationStatus::Verified`]. L'edition
//! 2005 est remplacee par l'EN 10027-1:2016, qui n'a pas ete confrontee : le
//! jeu le dit (`superseded_by`), et le moteur le rappelle. Les proprietes
//! physiques sont saisies sans document ouvert : elles portent
//! [`VerificationStatus::Unverified`].
//!
//! Les symboles additionnels dependent du groupe d'emploi (`H` : « profil
//! creux » pour un acier `S`, « temperature elevee » pour un acier `P`) :
//! chaque groupe porte donc ses propres listes, groupe 1 et groupe 2.
//!
//! [`VerificationStatus::Verified`]: mecatool_core::VerificationStatus::Verified
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
    /// Limite d'elasticite, en MPa (minimale, ou nominale pour `T`).
    Yield,
    /// Resistance a la traction, en MPa.
    Tensile,
    /// Durete Brinell minimale, HBW.
    Hardness,
    /// Pertes totales, en W/kg × 100, suivies de l'epaisseur et du type
    /// (Tableau 11).
    Losses,
    /// Deux symboles attribues par l'organisme responsable : pas une
    /// caracteristique (Tableau 8).
    Code,
}

/// Une forme des symboles principaux d'un groupe : `HC` + limite
/// d'elasticite, `HCT` + resistance a la traction...
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupForm {
    /// Les lettres qui suivent celle du groupe, vides le plus souvent.
    #[serde(default)]
    pub prefix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub meaning: String,
    pub number: GroupNumber,
    /// Ce que porte le nombre, en quelques mots.
    pub label: String,
    /// Le nombre de chiffres prevu par la norme, quand elle le fixe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<u8>,
    pub note: String,
}

/// Ce que disent les chiffres qui suivent un symbole additionnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuffixDigits {
    /// Un ou deux chiffres qui distinguent des qualites (norme de produit).
    Quality,
    /// 100 × la teneur en soufre, arrondie a 0,01 % (Tableau 12, note d).
    SulphurHundredths,
}

/// Un symbole additionnel et son sens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suffix {
    pub code: String,
    pub meaning: String,
    /// Les chiffres que ce symbole peut porter, quand la regle lui est propre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<SuffixDigits>,
}

/// Les symboles additionnels d'un groupe (groupe 1 ou groupe 2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSet {
    /// Les codes de resilience du Tableau 1 ouvrent le groupe 1.
    #[serde(default)]
    pub impact: bool,
    #[serde(default)]
    pub symbols: Vec<Suffix>,
    /// « a = classe ... » : toute lettre, avec ce sens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub any_letter: Option<String>,
    /// « an = symbole chimique ... » : le sens d'un symbole chimique, suivi
    /// le cas echeant d'un chiffre valant 10 × sa teneur moyenne.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chemical: Option<String>,
    /// Les symboles de ce groupe peuvent porter un ou deux chiffres de
    /// qualite.
    #[serde(default)]
    pub digits: bool,
}

impl SymbolSet {
    pub fn is_empty(&self) -> bool {
        !self.impact
            && self.symbols.is_empty()
            && self.any_letter.is_none()
            && self.chemical.is_none()
    }
}

/// Un groupe d'aciers designes par leur emploi : `S`, `E`, `P`... (Tableaux
/// 1 a 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseGroup {
    pub letter: String,
    /// Le tableau de l'EN 10027-1 qui definit le groupe.
    pub table: u8,
    pub name: String,
    pub forms: Vec<GroupForm>,
    #[serde(default)]
    pub group1: SymbolSet,
    #[serde(default)]
    pub group2: SymbolSet,
    /// Les types de produit d'un acier electrique (Tableau 11).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<Suffix>,
    /// Les tableaux de symboles pour les produits (16, 17, 18) auxquels le
    /// groupe renvoie.
    #[serde(default)]
    pub product_tables: Vec<u8>,
}

/// Les regles d'une categorie designee par sa composition (Tableaux 12 a 15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionRules {
    pub table: u8,
    #[serde(default)]
    pub group1: SymbolSet,
    #[serde(default)]
    pub group2: SymbolSet,
    #[serde(default)]
    pub product_tables: Vec<u8>,
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

/// Les elements qui partagent un facteur de teneur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlloyFactor {
    pub factor: u32,
    pub elements: Vec<String>,
}

/// Un symbole pour les produits en acier, apres un `+` (Tableaux 16 a 18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductSymbol {
    pub code: String,
    pub table: u8,
    /// Vrai quand le code est suivi d'une valeur en MPa (`+C700`) ; le sens
    /// porte alors `{n}` a sa place.
    #[serde(default)]
    pub value: bool,
    pub meaning: String,
}

/// Les regles de la designation symbolique.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DesignationRules {
    dataset: String,
    standard: StandardReference,
    /// L'edition qui remplace celle qui a ete lue, quand on le sait.
    #[serde(default)]
    superseded_by: Option<String>,
    use_groups: Vec<UseGroup>,
    impact: ImpactCodes,
    non_alloy: CompositionRules,
    low_alloy: CompositionRules,
    high_alloy: CompositionRules,
    high_speed: CompositionRules,
    numeric_product_tables: Vec<u8>,
    alloy_factors: Vec<AlloyFactor>,
    high_speed_order: Vec<String>,
    product_symbols: Vec<ProductSymbol>,
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
        let product_table = |table: &u8| (16..=18).contains(table);

        let mut letters = BTreeSet::new();
        for group in &self.use_groups {
            if group.letter.chars().count() != 1
                || !group.letter.chars().all(|c| c.is_ascii_uppercase())
                || !letters.insert(group.letter.as_str())
            {
                return Err(bad(format!("lettre de groupe invalide : {}", group.letter)));
            }
            // Deux formes au meme prefixe se liraient de deux facons.
            let mut prefixes = BTreeSet::new();
            for form in &group.forms {
                if !form.prefix.chars().all(|c| c.is_ascii_uppercase())
                    || !prefixes.insert(form.prefix.as_str())
                {
                    return Err(bad(format!(
                        "{} : prefixe invalide ou en double : {:?}",
                        group.letter, form.prefix
                    )));
                }
            }
            if group.forms.is_empty() {
                return Err(bad(format!("{} : aucune forme", group.letter)));
            }
            let losses = group.forms.iter().any(|f| f.number == GroupNumber::Losses);
            if losses == group.types.is_empty() {
                return Err(bad(format!(
                    "{} : les types de produit vont avec les pertes, et seulement avec elles",
                    group.letter
                )));
            }
            for set in [&group.group1, &group.group2] {
                check_set(set).map_err(|detail| bad(format!("{} : {detail}", group.letter)))?;
            }
            if group.group1.is_empty() && !group.group2.is_empty() {
                return Err(bad(format!(
                    "{} : un groupe 2 ne s'emploie qu'avec un groupe 1",
                    group.letter
                )));
            }
            if group.group2.impact {
                return Err(bad(format!(
                    "{} : la resilience ouvre le groupe 1",
                    group.letter
                )));
            }
            if !group.product_tables.iter().all(product_table) {
                return Err(bad(format!(
                    "{} : tableau de produit inconnu",
                    group.letter
                )));
            }
        }
        for rules in [
            &self.non_alloy,
            &self.low_alloy,
            &self.high_alloy,
            &self.high_speed,
        ] {
            for set in [&rules.group1, &rules.group2] {
                check_set(set)
                    .map_err(|detail| bad(format!("tableau {} : {detail}", rules.table)))?;
                if set.impact {
                    return Err(bad(format!("tableau {} : pas de resilience", rules.table)));
                }
            }
            if !rules.product_tables.iter().all(product_table) {
                return Err(bad(format!(
                    "tableau {} : tableau de produit inconnu",
                    rules.table
                )));
            }
        }
        if !self.numeric_product_tables.iter().all(product_table) {
            return Err(bad(
                "designation numerique : tableau de produit inconnu".into()
            ));
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
            if t.code.chars().count() != 1 || !codes.insert(t.code.as_str()) {
                return Err(bad(format!("code de temperature invalide : {}", t.code)));
            }
        }
        if self
            .impact
            .energies
            .windows(2)
            .any(|w| w[1].joules <= w[0].joules)
            || self
                .impact
                .energies
                .iter()
                .any(|e| e.letter.chars().count() != 1)
        {
            return Err(bad(
                "energies de rupture invalides ou non croissantes".into()
            ));
        }
        // Un meme code peut figurer dans deux tableaux (+A), pas deux fois
        // dans le meme.
        let mut symbols = BTreeSet::new();
        for symbol in &self.product_symbols {
            let well_formed = !symbol.code.is_empty()
                && symbol
                    .code
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            if !well_formed
                || !product_table(&symbol.table)
                || symbol.value != symbol.meaning.contains("{n}")
                || !symbols.insert((symbol.table, symbol.code.as_str(), symbol.value))
            {
                return Err(bad(format!(
                    "symbole de produit invalide ou en double : +{} (tableau {})",
                    symbol.code, symbol.table
                )));
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    /// L'edition qui remplace celle qui a ete lue : `EN 10027-1:2016`.
    pub fn superseded_by(&self) -> Option<&str> {
        self.superseded_by.as_deref()
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

    /// Aciers non allies a manganese moyen < 1 % : `C45E` (Tableau 12).
    pub fn non_alloy(&self) -> &CompositionRules {
        &self.non_alloy
    }

    /// Chaque element d'alliage sous 5 % : `42CrMo4` (Tableau 13).
    pub fn low_alloy(&self) -> &CompositionRules {
        &self.low_alloy
    }

    /// Un element au moins a 5 % : `X5CrNi18-10` (Tableau 14).
    pub fn high_alloy(&self) -> &CompositionRules {
        &self.high_alloy
    }

    /// Aciers rapides : `HS6-5-2` (Tableau 15).
    pub fn high_speed(&self) -> &CompositionRules {
        &self.high_speed
    }

    /// Les tableaux de produit qui peuvent suivre un numero d'acier (7.2).
    pub fn numeric_product_tables(&self) -> &[u8] {
        &self.numeric_product_tables
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

    pub fn product_symbols(&self) -> &[ProductSymbol] {
        &self.product_symbols
    }
}

/// Les codes d'un groupe sont uniques et bien formes : `N`, `LA`, `Cr`.
fn check_set(set: &SymbolSet) -> core::result::Result<(), String> {
    let mut codes = BTreeSet::new();
    for symbol in &set.symbols {
        let mut chars = symbol.code.chars();
        let well_formed = matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
            && chars.all(|c| c.is_ascii_alphabetic());
        if !well_formed || !codes.insert(symbol.code.as_str()) {
            return Err(format!(
                "symbole additionnel invalide ou en double : {}",
                symbol.code
            ));
        }
    }
    Ok(())
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
    /// `JR`, `J0`, `J2`, `K2` ; vide pour une nuance sans qualite (`S185`).
    pub qualities: Vec<String>,
    /// Une restriction d'emploi que la norme attache a la nuance : `S460`
    /// ne vaut que pour les produits longs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restriction: Option<String>,
    /// Les echelons tabules, du plus mince au plus epais. Une nuance peut
    /// s'arreter avant le dernier echelon de la table : au-dela, la norme ne
    /// donne rien pour elle, et rien n'est extrapole.
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
    #[serde(default)]
    restriction: Option<String>,
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
            // Une nuance couvre les premiers echelons, sans trou : elle peut
            // s'arreter avant le dernier (S460 a 150 mm), pas en sauter un.
            if g.yield_mpa.is_empty()
                || g.yield_mpa.len() > bands.len()
                || g.tensile_mpa.len() != g.yield_mpa.len()
            {
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
                restriction: g.restriction.clone(),
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
        // Une nuance plus resistante l'est sur tous les echelons qu'elles
        // partagent.
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
    fn la_designation_et_la_table_en10025_sont_verifiees() {
        // EN 10025-2:2019 et EN 10027-1:2005 ont ete relues ; les familles
        // restent saisies sans document ouvert.
        let reference = structural().standard();
        assert!(reference.verification.is_verified());
        assert_eq!(reference.citation(), "EN 10025-2:2019");
        let reference = rules().standard();
        assert!(reference.verification.is_verified());
        assert_eq!(reference.citation(), "EN 10027-1:2005");
        assert!(!families().standard().verification.is_verified());
    }

    #[test]
    fn ledition_lue_est_dite_remplacee() {
        // La vérification porte sur 2005 ; 2016 n'a pas été confrontée.
        assert_eq!(rules().superseded_by(), Some("EN 10027-1:2016"));
        assert!(rules()
            .standard()
            .notes
            .iter()
            .any(|n| n.contains("2016") && n.contains("pas été confrontée")));
    }

    #[test]
    fn chaque_groupe_porte_ses_propres_symboles() {
        // EN 10027-1:2005, tableaux 1 et 2 : H n'a pas le meme sens.
        let s = rules().use_group("S").unwrap();
        let p = rules().use_group("P").unwrap();
        let h = |set: &SymbolSet| {
            set.symbols
                .iter()
                .find(|x| x.code == "H")
                .map(|x| x.meaning.clone())
        };
        assert_eq!(h(&s.group2).as_deref(), Some("profil creux"));
        assert_eq!(h(&p.group2).as_deref(), Some("température élevée"));
        assert_eq!((s.table, p.table), (1, 2));
        // Les tableaux 1 a 11 sont tous la.
        let letters: Vec<&str> = rules()
            .use_groups()
            .iter()
            .map(|g| g.letter.as_str())
            .collect();
        assert_eq!(
            letters,
            ["S", "P", "L", "E", "B", "Y", "R", "D", "H", "T", "M"]
        );
        // Seuls S et E ouvrent leur groupe 1 par la resilience.
        let impact: Vec<&str> = rules()
            .use_groups()
            .iter()
            .filter(|g| g.group1.impact)
            .map(|g| g.letter.as_str())
            .collect();
        assert_eq!(impact, ["S", "E"]);
    }

    #[test]
    fn les_codes_de_resilience_du_tableau_1() {
        let impact = rules().impact();
        let joules: Vec<u32> = impact.energies.iter().map(|e| e.joules).collect();
        assert_eq!(joules, [27, 40, 60]);
        let celsius: Vec<i32> = impact.temperatures.iter().map(|t| t.celsius).collect();
        assert_eq!(celsius, [20, 0, -20, -30, -40, -50, -60]);
    }

    #[test]
    fn les_symboles_pour_les_produits() {
        // Tableaux 16 a 18 : +A est un revetement ou un recuit selon le tableau.
        let a: Vec<u8> = rules()
            .product_symbols()
            .iter()
            .filter(|s| s.code == "A" && !s.value)
            .map(|s| s.table)
            .collect();
        assert_eq!(a, [17, 18]);
        let count = |table: u8| {
            rules()
                .product_symbols()
                .iter()
                .filter(|s| s.table == table)
                .count()
        };
        assert_eq!((count(16), count(17), count(18)), (5, 16, 29));
    }

    #[test]
    fn un_symbole_de_produit_en_double_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(DESIGNATION_EMBEDDED).unwrap();
        broken["product_symbols"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "code": "Z", "table": 17, "meaning": "doublon" }));
        let err = DesignationRules::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("en double"), "{err}");
    }

    #[test]
    fn un_groupe_2_sans_groupe_1_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(DESIGNATION_EMBEDDED).unwrap();
        broken["use_groups"][0]["group1"] = serde_json::json!({});
        let err = DesignationRules::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("groupe 1"), "{err}");
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
    fn les_valeurs_du_tableau_6_aux_extremites() {
        // EN 10025-2:2019, tableau 6 : le dernier echelon de S235, et la
        // resistance qui baisse au-dela de 100 mm.
        let s235 = structural().grade("S235").unwrap();
        let last = s235.rows.last().unwrap();
        assert_eq!(last.band.to, Length::from_millimetres(400));
        assert_eq!(
            (last.yield_mpa, last.tensile_min_mpa, last.tensile_max_mpa),
            (165, 330, 480)
        );
        let s355 = structural().grade("S355").unwrap();
        assert_eq!(s355.rows.len(), 9);
        assert_eq!(
            (s355.rows[5].tensile_min_mpa, s355.rows[5].tensile_max_mpa),
            (450, 600)
        );
        assert!(s355.restriction.is_none());
    }

    #[test]
    fn s460_ne_vaut_que_pour_les_produits_longs_jusqua_150_mm() {
        let s460 = structural().grade("S460").unwrap();
        assert_eq!(s460.rows.len(), 6);
        assert_eq!(
            s460.rows.last().unwrap().band.to,
            Length::from_millimetres(150)
        );
        assert_eq!(s460.rows.last().unwrap().yield_mpa, 390);
        assert!(s460.restriction.as_deref().unwrap().contains("longs"));
        assert_eq!(structural().grade("S500").unwrap().qualities, ["J0"]);
        // S185 n'a pas de qualite, et s'arrete a 250 mm.
        let s185 = structural().grade("S185").unwrap();
        assert!(s185.qualities.is_empty());
        assert_eq!(
            s185.rows.last().unwrap().band.to,
            Length::from_millimetres(250)
        );
    }

    #[test]
    fn une_ligne_plus_longue_que_la_table_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(STRUCTURAL_EMBEDDED).unwrap();
        broken["grades"][1]["yield_mpa"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!(160));
        broken["grades"][1]["tensile_mpa"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!([320, 470]));
        let err = StructuralSteelTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("ligne incomplete"), "{err}");
    }

    #[test]
    fn une_resistance_sans_limite_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(STRUCTURAL_EMBEDDED).unwrap();
        broken["grades"][4]["tensile_mpa"]
            .as_array_mut()
            .unwrap()
            .pop();
        let err = StructuralSteelTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("ligne incomplete"), "{err}");
    }

    #[test]
    fn une_limite_qui_remonte_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(STRUCTURAL_EMBEDDED).unwrap();
        broken["grades"][3]["yield_mpa"][3] = serde_json::json!(350);
        let err = StructuralSteelTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("remonte"), "{err}");
    }

    #[test]
    fn la_premiere_limite_est_celle_de_la_designation() {
        let mut broken: serde_json::Value = serde_json::from_str(STRUCTURAL_EMBEDDED).unwrap();
        broken["grades"][1]["yield_mpa"][0] = serde_json::json!(240);
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
