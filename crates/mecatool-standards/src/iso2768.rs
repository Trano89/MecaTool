//! Tolerances generales de l'ISO 2768-1.
//!
//! Les tolerances generales couvrent les cotes qui ne portent pas de tolerance
//! individuelle. Elles ne se calculent pas : elles se lisent dans trois tables,
//! selon la classe retenue et la dimension nominale.
//!
//! ```text
//!   Tableau 1  dimensions lineaires, sauf aretes abattues
//!   Tableau 2  aretes abattues : rayons de courbure et hauteurs de chanfrein
//!   Tableau 3  dimensions angulaires
//! ```
//!
//! # Trois particularites qui distinguent cette norme de l'ISO 286
//!
//! **La borne basse du premier echelon est incluse.** L'ISO 286 ecrit
//! « au-dessus de 6 jusqu'a 10 » ; l'ISO 2768 ecrit « de 0,5 a 3 ». Une cote de
//! 0,5 mm appartient donc bien au premier echelon.
//!
//! **Certains echelons sont ouverts.** « au-dela de 6 » pour les aretes
//! abattues, « au-dela de 400 » pour les angles : sans borne haute.
//!
//! **Les tolerances angulaires se resserrent quand la piece grandit.** Elles
//! dependent de la longueur du cote le plus court de l'angle, et non de l'angle
//! lui-meme : a ecart lineaire egal, un cote plus long donne un angle plus
//! petit. C'est l'inverse des tolerances lineaires, et la validation des tables
//! en tient compte.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mecatool_core::{Angle, Length, StandardReference, Unit};
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Result, StandardsError};
use crate::value::length_from_json;

/// Les quatre classes de tolerance generale.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum GeneralClass {
    Fine,
    Medium,
    Coarse,
    VeryCoarse,
}

/// Les classes dans l'ordre du plus fin au plus grossier.
pub const ALL_CLASSES: [GeneralClass; 4] = [
    GeneralClass::Fine,
    GeneralClass::Medium,
    GeneralClass::Coarse,
    GeneralClass::VeryCoarse,
];

impl GeneralClass {
    /// Le symbole porte sur le dessin : `f`, `m`, `c`, `v`.
    pub const fn symbol(self) -> &'static str {
        match self {
            GeneralClass::Fine => "f",
            GeneralClass::Medium => "m",
            GeneralClass::Coarse => "c",
            GeneralClass::VeryCoarse => "v",
        }
    }

    pub const fn name_fr(self) -> &'static str {
        match self {
            GeneralClass::Fine => "fin",
            GeneralClass::Medium => "moyen",
            GeneralClass::Coarse => "grossier",
            GeneralClass::VeryCoarse => "très grossier",
        }
    }

    /// La designation telle qu'elle s'inscrit dans le cartouche.
    pub fn designation(self) -> String {
        format!("ISO 2768-{}", self.symbol())
    }

    /// Lit `"f"`, `"m"`, `"c"`, `"v"`, ou la designation complete `"ISO 2768-m"`.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        let symbol = trimmed
            .rsplit_once('-')
            .map(|(_, tail)| tail)
            .unwrap_or(trimmed)
            .trim()
            .to_ascii_lowercase();
        ALL_CLASSES
            .into_iter()
            .find(|class| class.symbol() == symbol)
            .ok_or_else(|| StandardsError::UnknownGeneralClass {
                given: input.to_string(),
            })
    }
}

impl core::fmt::Display for GeneralClass {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ({})", self.symbol(), self.name_fr())
    }
}

/// Ce que l'on mesure, et donc la table qui s'applique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureKind {
    /// Dimension lineaire ordinaire.
    Linear,
    /// Arete abattue : rayon de courbure ou hauteur de chanfrein.
    BrokenEdge,
    /// Dimension angulaire, reperee par la longueur du cote le plus court.
    Angular,
}

impl MeasureKind {
    pub const fn label_fr(self) -> &'static str {
        match self {
            MeasureKind::Linear => "dimension linéaire",
            MeasureKind::BrokenEdge => "arête abattue",
            MeasureKind::Angular => "dimension angulaire",
        }
    }
}

/// Un echelon de dimensions nominales de l'ISO 2768.
///
/// Se distingue de celui de l'ISO 286 sur deux points : la borne basse peut
/// etre incluse, et la borne haute peut manquer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GeneralRange {
    pub from: Length,
    #[serde(default)]
    pub from_inclusive: bool,
    /// Absente pour un echelon ouvert (« au-dela de 6 »).
    #[serde(default)]
    pub to: Option<Length>,
}

impl GeneralRange {
    pub fn contains(&self, nominal: Length) -> bool {
        let above = if self.from_inclusive {
            nominal >= self.from
        } else {
            nominal > self.from
        };
        above && self.to.is_none_or(|top| nominal <= top)
    }

    pub fn label_fr(&self) -> String {
        let from = trim(self.from.to_decimal_string(Unit::Millimetre, 3));
        match self.to {
            Some(to) => {
                let to = trim(to.to_decimal_string(Unit::Millimetre, 3));
                if self.from_inclusive {
                    format!("de {from} à {to}")
                } else {
                    format!("au-dessus de {from} jusqu'à {to}")
                }
            }
            None => format!("au-delà de {from}"),
        }
    }
}

fn trim(rendered: String) -> String {
    if rendered.contains('.') {
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        rendered
    }
}

/// Un ecart limite general, symetrique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GeneralDeviation {
    Linear { magnitude: Length },
    Angular { magnitude: Angle },
}

impl GeneralDeviation {
    /// Ecriture symetrique, telle qu'elle figure dans les tables.
    pub fn to_symmetric_string(self) -> String {
        match self {
            GeneralDeviation::Linear { magnitude } => format!(
                "\u{b1} {} mm",
                trim(magnitude.to_decimal_string(Unit::Millimetre, 3))
            ),
            GeneralDeviation::Angular { magnitude } => magnitude.to_symmetric_string(),
        }
    }
}

/// Le resultat d'une consultation des tables.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GeneralLookup {
    pub class: GeneralClass,
    pub kind: MeasureKind,
    pub range: GeneralRange,
    pub range_index: usize,
    pub deviation: GeneralDeviation,
    /// Le tableau de la norme d'ou vient la valeur.
    pub table: String,
}

// --- Chargement ---------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ClassEntry {
    symbol: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct SubTableFile {
    table: String,
    value_unit: String,
    size_unit: Unit,
    size_ranges: Vec<GeneralRangeFile>,
    deviations: BTreeMap<String, Vec<Option<Value>>>,
}

#[derive(Debug, Deserialize)]
struct GeneralRangeFile {
    from: Value,
    #[serde(default)]
    from_inclusive: bool,
    #[serde(default)]
    to: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct GeneralTolerancesFile {
    dataset: String,
    standard: StandardReference,
    classes: Vec<ClassEntry>,
    linear: SubTableFile,
    broken_edges: SubTableFile,
    angular: SubTableFile,
}

/// Une des trois tables, validee.
#[derive(Debug, Clone)]
struct SubTable {
    table: String,
    ranges: Vec<GeneralRange>,
    /// `None` la ou la norme ne definit rien.
    rows: BTreeMap<GeneralClass, Vec<Option<GeneralDeviation>>>,
}

/// Les tolerances generales de l'ISO 2768-1, validees.
#[derive(Debug, Clone)]
pub struct GeneralToleranceTable {
    dataset: String,
    standard: StandardReference,
    linear: SubTable,
    broken_edges: SubTable,
    angular: SubTable,
}

const EMBEDDED: &str = include_str!("../../../data/iso2768/iso2768-1-1989.general-tolerances.json");

/// Sous cette dimension nominale, la norme renvoie a une cotation individuelle.
const MINIMUM_NOMINAL: Length = Length::from_nanometres(500_000);

impl GeneralToleranceTable {
    pub fn embedded() -> Result<&'static GeneralToleranceTable> {
        static CACHE: OnceLock<core::result::Result<GeneralToleranceTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| GeneralToleranceTable::from_json(EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    pub fn from_json(text: &str) -> Result<GeneralToleranceTable> {
        let file: GeneralTolerancesFile =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso2768-1 general-tolerances".to_string(),
                detail: source.to_string(),
            })?;
        let dataset = file.dataset.clone();

        // Les classes declarees doivent etre exactement celles de la norme.
        let declared: Vec<String> = file.classes.iter().map(|c| c.symbol.clone()).collect();
        let expected: Vec<String> = ALL_CLASSES.iter().map(|c| c.symbol().to_string()).collect();
        if declared != expected {
            return Err(StandardsError::Inconsistent {
                dataset,
                detail: format!("classes declarees {declared:?}, attendu {expected:?}"),
            });
        }

        let table = GeneralToleranceTable {
            linear: SubTable::load(&dataset, &file.linear)?,
            broken_edges: SubTable::load(&dataset, &file.broken_edges)?,
            angular: SubTable::load(&dataset, &file.angular)?,
            dataset,
            standard: file.standard,
        };
        table.validate()?;
        Ok(table)
    }

    fn sub_table(&self, kind: MeasureKind) -> &SubTable {
        match kind {
            MeasureKind::Linear => &self.linear,
            MeasureKind::BrokenEdge => &self.broken_edges,
            MeasureKind::Angular => &self.angular,
        }
    }

    /// L'ecart limite general pour une dimension et une classe.
    ///
    /// Pour une dimension angulaire, `nominal` est la **longueur du cote le plus
    /// court** de l'angle, pas l'angle lui-meme.
    pub fn lookup(
        &self,
        kind: MeasureKind,
        nominal: Length,
        class: GeneralClass,
    ) -> Result<GeneralLookup> {
        if !nominal.is_positive() {
            return Err(StandardsError::NominalOutOfRange {
                dataset: self.dataset.clone(),
                nominal: nominal.to_string(),
                min: MINIMUM_NOMINAL.to_string(),
                max: "4 mm".to_string(),
            });
        }

        // La norme le dit elle-meme : sous 0,5 mm, les ecarts limites doivent
        // etre portes a cote de la cote. Le moteur n'a rien a en dire.
        if kind != MeasureKind::Angular && nominal < MINIMUM_NOMINAL {
            return Err(StandardsError::BelowGeneralMinimum {
                nominal: nominal.to_string(),
                minimum: MINIMUM_NOMINAL.to_string(),
            });
        }

        let sub = self.sub_table(kind);
        let range_index = sub
            .ranges
            .iter()
            .position(|range| range.contains(nominal))
            .ok_or_else(|| StandardsError::NominalOutOfRange {
                dataset: self.dataset.clone(),
                nominal: nominal.to_string(),
                min: sub.ranges[0].from.to_string(),
                max: sub.ranges[sub.ranges.len() - 1]
                    .to
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "sans limite".to_string()),
            })?;

        let row = sub
            .rows
            .get(&class)
            .ok_or_else(|| StandardsError::UnknownGeneralClass {
                given: class.symbol().to_string(),
            })?;

        let deviation = row[range_index].ok_or_else(|| StandardsError::GeneralNotDefined {
            class: class.symbol().to_string(),
            kind: kind.label_fr().to_string(),
            range: sub.ranges[range_index].label_fr(),
        })?;

        Ok(GeneralLookup {
            class,
            kind,
            range: sub.ranges[range_index],
            range_index,
            deviation,
            table: sub.table.clone(),
        })
    }

    fn validate(&self) -> Result<()> {
        for (kind, sub) in [
            (MeasureKind::Linear, &self.linear),
            (MeasureKind::BrokenEdge, &self.broken_edges),
            (MeasureKind::Angular, &self.angular),
        ] {
            sub.validate(&self.dataset, kind)?;
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn dataset(&self) -> &str {
        &self.dataset
    }

    pub fn ranges(&self, kind: MeasureKind) -> &[GeneralRange] {
        &self.sub_table(kind).ranges
    }

    pub fn table_name(&self, kind: MeasureKind) -> &str {
        &self.sub_table(kind).table
    }
}

impl SubTable {
    fn load(dataset: &str, file: &SubTableFile) -> Result<SubTable> {
        let mut ranges = Vec::with_capacity(file.size_ranges.len());
        for entry in &file.size_ranges {
            ranges.push(GeneralRange {
                from: length_from_json(dataset, &entry.from, file.size_unit)?,
                from_inclusive: entry.from_inclusive,
                to: match &entry.to {
                    Some(value) => Some(length_from_json(dataset, value, file.size_unit)?),
                    None => None,
                },
            });
        }

        let mut rows = BTreeMap::new();
        for (symbol, values) in &file.deviations {
            let class = GeneralClass::parse(symbol)?;
            let mut row = Vec::with_capacity(values.len());
            for value in values {
                row.push(match value {
                    None => None,
                    Some(v) => Some(parse_deviation(dataset, v, &file.value_unit)?),
                });
            }
            rows.insert(class, row);
        }

        Ok(SubTable {
            table: file.table.clone(),
            ranges,
            rows,
        })
    }

    fn validate(&self, dataset: &str, kind: MeasureKind) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: dataset.to_string(),
            detail: format!("{} : {detail}", kind.label_fr()),
        };

        if self.ranges.is_empty() {
            return Err(bad("aucun echelon".into()));
        }
        // Echelons croissants et contigus : une cote ne peut pas tomber entre
        // deux lignes de la table.
        for (i, range) in self.ranges.iter().enumerate() {
            if let Some(to) = range.to {
                if range.from >= to {
                    return Err(bad(format!("echelon {i} : bornes inversees")));
                }
            } else if i + 1 != self.ranges.len() {
                return Err(bad(format!(
                    "echelon {i} ouvert alors qu'il n'est pas le dernier"
                )));
            }
            if i > 0 && self.ranges[i - 1].to != Some(range.from) {
                return Err(bad(format!("discontinuite avant l'echelon {i}")));
            }
        }

        let width = self.ranges.len();
        for (class, row) in &self.rows {
            if row.len() != width {
                return Err(bad(format!(
                    "classe {} : {} valeurs pour {width} echelons",
                    class.symbol(),
                    row.len()
                )));
            }
            for (i, value) in row.iter().enumerate() {
                if let Some(deviation) = value {
                    if magnitude_nm(*deviation) <= 0 {
                        return Err(bad(format!(
                            "classe {} echelon {i} : ecart non strictement positif",
                            class.symbol()
                        )));
                    }
                }
            }

            // Un ecart lineaire s'elargit quand la piece grandit ; un ecart
            // angulaire se resserre, puisqu'il decoule d'un ecart lineaire
            // rapporte a un bras de levier plus long.
            let mut previous: Option<i64> = None;
            for value in row.iter().flatten() {
                let current = magnitude_nm(*value);
                if let Some(before) = previous {
                    let wrong = match kind {
                        MeasureKind::Angular => current > before,
                        _ => current < before,
                    };
                    if wrong {
                        return Err(bad(format!(
                            "classe {} : progression incoherente ({before} puis {current})",
                            class.symbol()
                        )));
                    }
                }
                previous = Some(current);
            }
        }

        // A dimension egale, une classe plus grossiere ne peut pas etre plus
        // serree qu'une classe plus fine.
        for pair in ALL_CLASSES.windows(2) {
            let (finer, coarser) = (pair[0], pair[1]);
            let (Some(a), Some(b)) = (self.rows.get(&finer), self.rows.get(&coarser)) else {
                continue;
            };
            for i in 0..width {
                if let (Some(fine), Some(coarse)) = (a[i], b[i]) {
                    if magnitude_nm(coarse) < magnitude_nm(fine) {
                        return Err(bad(format!(
                            "echelon {i} : classe {} plus serree que classe {}",
                            coarser.symbol(),
                            finer.symbol()
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Grandeur comparable d'un ecart, quelle que soit sa nature.
fn magnitude_nm(deviation: GeneralDeviation) -> i64 {
    match deviation {
        GeneralDeviation::Linear { magnitude } => magnitude.nanometres(),
        GeneralDeviation::Angular { magnitude } => magnitude.milliarcseconds(),
    }
}

fn parse_deviation(dataset: &str, value: &Value, unit: &str) -> Result<GeneralDeviation> {
    match unit {
        "sexagesimal" => {
            let text = value.as_str().ok_or_else(|| StandardsError::Malformed {
                dataset: dataset.to_string(),
                detail: format!("angle attendu sous forme de chaine, trouve {value}"),
            })?;
            let magnitude = Angle::parse(text).map_err(|source| StandardsError::Malformed {
                dataset: dataset.to_string(),
                detail: format!("angle {text:?} illisible : {source}"),
            })?;
            Ok(GeneralDeviation::Angular { magnitude })
        }
        "millimetre" => Ok(GeneralDeviation::Linear {
            magnitude: length_from_json(dataset, value, Unit::Millimetre)?,
        }),
        other => Err(StandardsError::Malformed {
            dataset: dataset.to_string(),
            detail: format!("unite de valeur inconnue : {other:?}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> &'static GeneralToleranceTable {
        GeneralToleranceTable::embedded().expect("la table embarquee doit se charger")
    }

    fn mm(value: &str) -> Length {
        Length::parse(value, Unit::Millimetre).unwrap()
    }

    fn linear(nominal: &str, class: GeneralClass) -> Length {
        match table()
            .lookup(MeasureKind::Linear, mm(nominal), class)
            .unwrap()
            .deviation
        {
            GeneralDeviation::Linear { magnitude } => magnitude,
            other => panic!("ecart lineaire attendu, obtenu {other:?}"),
        }
    }

    fn angular(nominal: &str, class: GeneralClass) -> Angle {
        match table()
            .lookup(MeasureKind::Angular, mm(nominal), class)
            .unwrap()
            .deviation
        {
            GeneralDeviation::Angular { magnitude } => magnitude,
            other => panic!("ecart angulaire attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn la_table_se_charge_et_passe_ses_invariants() {
        let t = table();
        assert_eq!(t.ranges(MeasureKind::Linear).len(), 8);
        assert_eq!(t.ranges(MeasureKind::BrokenEdge).len(), 3);
        assert_eq!(t.ranges(MeasureKind::Angular).len(), 5);
        assert!(t.standard().verification.is_verified());
    }

    #[test]
    fn valeurs_lineaires_de_reference() {
        // Tableau 1, quelques cases reparties sur la table.
        assert_eq!(linear("1", GeneralClass::Fine), mm("0.05"));
        assert_eq!(linear("50", GeneralClass::Medium), mm("0.3"));
        assert_eq!(linear("100", GeneralClass::Coarse), mm("0.8"));
        assert_eq!(linear("300", GeneralClass::VeryCoarse), mm("2.5"));
        assert_eq!(linear("4000", GeneralClass::Medium), mm("2"));
    }

    #[test]
    fn valeurs_dareetes_abattues() {
        assert_eq!(linear_edge("2", GeneralClass::Fine), mm("0.2"));
        assert_eq!(linear_edge("5", GeneralClass::Coarse), mm("1"));
        // L'echelon « au-dela de 6 » est ouvert : il accepte n'importe quelle taille.
        assert_eq!(linear_edge("10", GeneralClass::Medium), mm("1"));
        assert_eq!(linear_edge("5000", GeneralClass::VeryCoarse), mm("2"));
    }

    fn linear_edge(nominal: &str, class: GeneralClass) -> Length {
        match table()
            .lookup(MeasureKind::BrokenEdge, mm(nominal), class)
            .unwrap()
            .deviation
        {
            GeneralDeviation::Linear { magnitude } => magnitude,
            other => panic!("ecart lineaire attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn valeurs_angulaires_de_reference() {
        assert_eq!(angular("8", GeneralClass::Medium), Angle::from_degrees(1));
        assert_eq!(
            angular("30", GeneralClass::Medium),
            Angle::from_dms(0, 30, 0)
        );
        assert_eq!(
            angular("100", GeneralClass::Fine),
            Angle::from_dms(0, 20, 0)
        );
        assert_eq!(
            angular("200", GeneralClass::Coarse),
            Angle::from_dms(0, 15, 0)
        );
        assert_eq!(
            angular("1000", GeneralClass::VeryCoarse),
            Angle::from_dms(0, 20, 0)
        );
    }

    #[test]
    fn la_borne_basse_du_premier_echelon_est_incluse() {
        // « de 0,5 a 3 » : 0,5 appartient a l'echelon, a la difference de l'ISO 286.
        assert_eq!(linear("0.5", GeneralClass::Medium), mm("0.1"));
        assert_eq!(
            table()
                .lookup(MeasureKind::Linear, mm("0.5"), GeneralClass::Medium)
                .unwrap()
                .range_index,
            0
        );
    }

    #[test]
    fn sous_le_demi_millimetre_la_norme_renvoie_a_une_cotation_individuelle() {
        let err = table()
            .lookup(MeasureKind::Linear, mm("0.3"), GeneralClass::Medium)
            .unwrap_err();
        assert!(matches!(err, StandardsError::BelowGeneralMinimum { .. }));
        // Le message doit dire quoi faire, pas seulement que c'est refuse.
        assert!(err.to_string().contains("cote"));
    }

    #[test]
    fn les_cases_vides_de_la_norme_sont_refusees_et_non_prises_pour_zero() {
        // La classe f n'est pas definie au-dela de 2000 mm.
        let err = table()
            .lookup(MeasureKind::Linear, mm("3000"), GeneralClass::Fine)
            .unwrap_err();
        assert!(matches!(err, StandardsError::GeneralNotDefined { .. }));

        // La classe v n'est pas definie en dessous de 3 mm.
        let err = table()
            .lookup(MeasureKind::Linear, mm("2"), GeneralClass::VeryCoarse)
            .unwrap_err();
        assert!(matches!(err, StandardsError::GeneralNotDefined { .. }));
    }

    #[test]
    fn hors_plage_haute_le_moteur_refuse() {
        let err = table()
            .lookup(MeasureKind::Linear, mm("5000"), GeneralClass::Medium)
            .unwrap_err();
        assert!(matches!(err, StandardsError::NominalOutOfRange { .. }));
    }

    #[test]
    fn les_bornes_dechelon_sont_incluses_en_haut() {
        assert_eq!(linear("3", GeneralClass::Coarse), mm("0.2"));
        assert_eq!(linear("3.001", GeneralClass::Coarse), mm("0.3"));
        assert_eq!(linear("30", GeneralClass::Coarse), mm("0.5"));
        assert_eq!(linear("30.001", GeneralClass::Coarse), mm("0.8"));
    }

    #[test]
    fn les_tolerances_angulaires_se_resserrent_quand_la_piece_grandit() {
        // Propriete physique : le meme ecart lineaire, rapporte a un bras plus
        // long, donne un angle plus petit.
        let mut previous: Option<Angle> = None;
        for length in ["5", "30", "100", "300", "1000"] {
            let current = angular(length, GeneralClass::Coarse);
            if let Some(before) = previous {
                assert!(current <= before, "{length} mm : {current} > {before}");
            }
            previous = Some(current);
        }
    }

    #[test]
    fn les_classes_se_lisent_du_symbole_ou_de_la_designation() {
        assert_eq!(GeneralClass::parse("m").unwrap(), GeneralClass::Medium);
        assert_eq!(GeneralClass::parse("V").unwrap(), GeneralClass::VeryCoarse);
        assert_eq!(
            GeneralClass::parse("ISO 2768-m").unwrap(),
            GeneralClass::Medium
        );
        assert_eq!(GeneralClass::Medium.designation(), "ISO 2768-m");
        assert!(GeneralClass::parse("z").is_err());
    }

    #[test]
    fn le_resultat_cite_le_tableau_dou_il_vient() {
        let found = table()
            .lookup(MeasureKind::Linear, mm("50"), GeneralClass::Medium)
            .unwrap();
        assert!(found.table.contains("Tableau 1"));
        assert_eq!(found.range.label_fr(), "au-dessus de 30 jusqu'à 120");
        assert_eq!(found.deviation.to_symmetric_string(), "\u{b1} 0.3 mm");
    }

    #[test]
    fn une_classe_plus_grossiere_qui_serait_plus_serree_est_rejetee() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "classes": [
                { "symbol": "f", "name": "fin" }, { "symbol": "m", "name": "moyen" },
                { "symbol": "c", "name": "grossier" }, { "symbol": "v", "name": "très grossier" }
            ],
            "linear": {
                "table": "T1", "value_unit": "millimetre", "size_unit": "millimetre",
                "size_ranges": [{ "from": 0.5, "from_inclusive": true, "to": 3 }],
                "deviations": { "f": [0.5], "m": [0.1], "c": [0.2], "v": [0.3] }
            },
            "broken_edges": {
                "table": "T2", "value_unit": "millimetre", "size_unit": "millimetre",
                "size_ranges": [{ "from": 0.5, "from_inclusive": true, "to": 3 }],
                "deviations": { "f": [0.2], "m": [0.2], "c": [0.4], "v": [0.4] }
            },
            "angular": {
                "table": "T3", "value_unit": "sexagesimal", "size_unit": "millimetre",
                "size_ranges": [{ "from": 0, "from_inclusive": true, "to": 10 }],
                "deviations": { "f": ["1°"], "m": ["1°"], "c": ["1°30'"], "v": ["3°"] }
            }
        }"#;
        let err = GeneralToleranceTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("plus serree")),
            "obtenu {err}"
        );
    }

    #[test]
    fn un_echelon_ouvert_ailleurs_qua_la_fin_est_rejete() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "classes": [
                { "symbol": "f", "name": "fin" }, { "symbol": "m", "name": "moyen" },
                { "symbol": "c", "name": "grossier" }, { "symbol": "v", "name": "très grossier" }
            ],
            "linear": {
                "table": "T1", "value_unit": "millimetre", "size_unit": "millimetre",
                "size_ranges": [{ "from": 0.5, "from_inclusive": true }, { "from": 3, "to": 6 }],
                "deviations": { "f": [0.05, 0.05], "m": [0.1, 0.1], "c": [0.2, 0.3], "v": [0.4, 0.5] }
            },
            "broken_edges": {
                "table": "T2", "value_unit": "millimetre", "size_unit": "millimetre",
                "size_ranges": [{ "from": 0.5, "from_inclusive": true, "to": 3 }],
                "deviations": { "f": [0.2], "m": [0.2], "c": [0.4], "v": [0.4] }
            },
            "angular": {
                "table": "T3", "value_unit": "sexagesimal", "size_unit": "millimetre",
                "size_ranges": [{ "from": 0, "from_inclusive": true, "to": 10 }],
                "deviations": { "f": ["1°"], "m": ["1°"], "c": ["1°30'"], "v": ["3°"] }
            }
        }"#;
        let err = GeneralToleranceTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("ouvert")),
            "obtenu {err}"
        );
    }
}
