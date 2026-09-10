//! Degres de tolerance normalises `IT` de l'ISO 286-1.
//!
//! La table est la source de verite. Les valeurs publiees resultent d'un
//! arrondi vers des nombres normalises et ne sont pas reproductibles exactement
//! par la formule du facteur de tolerance : voir [`super::formula`], qui ne sert
//! qu'a signaler une case aberrante, jamais a produire une valeur.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mecatol_core::{Grade, Length, SizeRange, StandardReference, Unit};
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Result, StandardsError};
use crate::value::{length_from_json, lengths_from_json};

/// Contenu brut du fichier de donnees, avant validation.
#[derive(Debug, Deserialize)]
struct ItGradesFile {
    dataset: String,
    standard: StandardReference,
    value_unit: Unit,
    size_unit: Unit,
    size_ranges: Vec<Vec<Value>>,
    grades: BTreeMap<String, Vec<Value>>,
}

/// Une valeur `IT` lue dans la table, avec l'echelon qui l'a fournie.
///
/// L'echelon fait partie du resultat : le mode expert doit pouvoir montrer que
/// `IT7` a 10 mm vient bien de la ligne « au-dessus de 6 jusqu'à 10 ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItValue {
    pub value: Length,
    pub range: SizeRange,
    pub range_index: usize,
    pub grade: Grade,
}

/// La table des degres de tolerance, validee.
#[derive(Debug, Clone)]
pub struct ItGradeTable {
    dataset: String,
    standard: StandardReference,
    ranges: Vec<SizeRange>,
    rows: BTreeMap<Grade, Vec<Length>>,
}

/// Au-dela de cette dimension nominale, les degres grossiers redeviennent applicables.
///
/// ISO 286-1 exclut les degres IT14 a IT18 pour les dimensions nominales
/// inferieures ou egales a 1 mm.
const COARSE_GRADE_MIN_NOMINAL: Length = Length::from_millimetres(1);

/// Premier degre concerne par cette restriction.
fn first_coarse_grade() -> Grade {
    Grade::from_it_number(14).expect("IT14 est un degre valide")
}

const EMBEDDED: &str = include_str!("../../../../data/iso286/iso286-1-2010.it-grades.json");

impl ItGradeTable {
    /// La table embarquee dans le binaire.
    ///
    /// Les donnees vivent dans un fichier separe du code (`/data`) mais sont
    /// incluses a la compilation : Mecatol calcule sans acces disque ni reseau,
    /// et ne peut pas demarrer avec une table manquante.
    pub fn embedded() -> Result<&'static ItGradeTable> {
        static CACHE: OnceLock<core::result::Result<ItGradeTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| ItGradeTable::from_json(EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Charge et valide une table depuis son ecriture JSON.
    pub fn from_json(text: &str) -> Result<ItGradeTable> {
        let file: ItGradesFile =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso286-1 it-grades".to_string(),
                detail: source.to_string(),
            })?;

        let dataset = file.dataset.clone();

        let mut ranges = Vec::with_capacity(file.size_ranges.len());
        for bounds in &file.size_ranges {
            let [above, up_to] = bounds.as_slice() else {
                return Err(StandardsError::Malformed {
                    dataset,
                    detail: format!(
                        "un echelon doit avoir exactement 2 bornes, trouve {}",
                        bounds.len()
                    ),
                });
            };
            ranges.push(SizeRange::new(
                length_from_json(&dataset, above, file.size_unit)?,
                length_from_json(&dataset, up_to, file.size_unit)?,
            ));
        }

        let mut rows = BTreeMap::new();
        for (name, values) in &file.grades {
            let grade = Grade::parse(name)?;
            rows.insert(grade, lengths_from_json(&dataset, values, file.value_unit)?);
        }

        let table = ItGradeTable {
            dataset,
            standard: file.standard,
            ranges,
            rows,
        };
        table.validate()?;
        Ok(table)
    }

    /// Verifie les invariants structurels de la table.
    ///
    /// Une erreur de saisie dans une table normative doit etre detectable
    /// automatiquement : c'est le role de ces controles, qui tournent au
    /// chargement et donc a chaque demarrage et a chaque test.
    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        if self.ranges.is_empty() {
            return Err(bad("aucun echelon de dimensions nominales".into()));
        }
        if self.rows.is_empty() {
            return Err(bad("aucun degre de tolerance".into()));
        }

        // Les echelons doivent etre croissants et contigus : sans trou, une
        // dimension nominale ne peut pas tomber entre deux lignes de la table.
        for (i, range) in self.ranges.iter().enumerate() {
            if range.above >= range.up_to {
                return Err(bad(format!(
                    "echelon {i} : borne basse {} superieure ou egale a la borne haute {}",
                    range.above, range.up_to
                )));
            }
            if i > 0 && self.ranges[i - 1].up_to != range.above {
                return Err(bad(format!(
                    "discontinuite entre les echelons {} et {i} : {} puis {}",
                    i - 1,
                    self.ranges[i - 1].up_to,
                    range.above
                )));
            }
        }

        let width = self.ranges.len();
        for (grade, row) in &self.rows {
            if row.len() != width {
                return Err(bad(format!(
                    "{} comporte {} valeurs pour {width} echelons",
                    grade.name(),
                    row.len()
                )));
            }
            for (i, value) in row.iter().enumerate() {
                if !value.is_positive() {
                    return Err(bad(format!(
                        "{} echelon {i} : valeur non strictement positive ({value})",
                        grade.name()
                    )));
                }
            }
            // A degre constant, la tolerance ne peut pas diminuer quand la piece grossit.
            for i in 1..row.len() {
                if row[i] < row[i - 1] {
                    return Err(bad(format!(
                        "{} decroit de l'echelon {} a l'echelon {i} : {} puis {}",
                        grade.name(),
                        i - 1,
                        row[i - 1],
                        row[i]
                    )));
                }
            }
        }

        // A dimension constante, un degre plus grossier doit etre strictement plus large.
        let grades: Vec<Grade> = self.rows.keys().copied().collect();
        for pair in grades.windows(2) {
            let (finer, coarser) = (pair[0], pair[1]);
            for i in 0..width {
                let a = self.rows[&finer][i];
                let b = self.rows[&coarser][i];
                if b <= a {
                    return Err(bad(format!(
                        "echelon {i} : {} ({a}) n'est pas strictement plus fin que {} ({b})",
                        finer.name(),
                        coarser.name()
                    )));
                }
            }
        }

        Ok(())
    }

    /// La valeur `IT` pour une dimension nominale et un degre.
    pub fn value(&self, nominal: Length, grade: Grade) -> Result<ItValue> {
        let range_index = self.range_index(nominal)?;
        self.check_applicability(nominal, grade)?;

        let row = self
            .rows
            .get(&grade)
            .ok_or_else(|| StandardsError::GradeUnavailable {
                dataset: self.dataset.clone(),
                grade: grade.name().to_string(),
            })?;

        Ok(ItValue {
            value: row[range_index],
            range: self.ranges[range_index],
            range_index,
            grade,
        })
    }

    /// L'indice de l'echelon couvrant `nominal`, ou une erreur explicite.
    ///
    /// Hors plage, on refuse : extrapoler une table normative reviendrait a
    /// inventer une valeur ISO.
    pub fn range_index(&self, nominal: Length) -> Result<usize> {
        self.ranges
            .iter()
            .position(|r| r.contains(nominal))
            .ok_or_else(|| StandardsError::NominalOutOfRange {
                dataset: self.dataset.clone(),
                nominal: nominal.to_string(),
                min: self.ranges[0].above.to_string(),
                max: self.ranges[self.ranges.len() - 1].up_to.to_string(),
            })
    }

    /// Applique les restrictions d'emploi enoncees par la norme.
    fn check_applicability(&self, nominal: Length, grade: Grade) -> Result<()> {
        if grade >= first_coarse_grade() && nominal <= COARSE_GRADE_MIN_NOMINAL {
            return Err(StandardsError::GradeNotApplicable {
                grade: grade.name().to_string(),
                reason: format!(
                    "l'ISO 286-1 exclut les degres IT14 a IT18 pour les dimensions \
                     nominales inferieures ou egales a {COARSE_GRADE_MIN_NOMINAL}"
                ),
            });
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn dataset(&self) -> &str {
        &self.dataset
    }

    pub fn ranges(&self) -> &[SizeRange] {
        &self.ranges
    }

    /// Les degres disponibles, du plus fin au plus grossier.
    pub fn grades(&self) -> impl Iterator<Item = Grade> + '_ {
        self.rows.keys().copied()
    }

    /// La plus grande dimension nominale couverte.
    pub fn max_nominal(&self) -> Length {
        self.ranges[self.ranges.len() - 1].up_to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> &'static ItGradeTable {
        ItGradeTable::embedded().expect("la table embarquee doit se charger")
    }

    fn it(nominal_mm: &str, grade: u8) -> Length {
        table()
            .value(
                Length::parse(nominal_mm, Unit::Millimetre).unwrap(),
                Grade::from_it_number(grade).unwrap(),
            )
            .unwrap()
            .value
    }

    #[test]
    fn la_table_embarquee_se_charge_et_passe_ses_invariants() {
        let t = table();
        assert_eq!(t.ranges().len(), 13);
        assert_eq!(t.grades().count(), 20);
        assert_eq!(t.max_nominal(), Length::from_millimetres(500));
    }

    /// Une table declaree verifiee doit citer precisement contre quoi.
    ///
    /// « Verifie » sans source nommee ne vaut pas mieux que « non verifie » :
    /// personne ne pourrait refaire le controle.
    #[test]
    fn la_table_cite_la_source_qui_la_verifie() {
        let standard = table().standard();
        match &standard.verification {
            mecatol_core::VerificationStatus::Verified { against, on } => {
                assert!(
                    against.contains("ISO 286-2"),
                    "source imprecise : {against}"
                );
                assert!(
                    against.contains("tableau 1"),
                    "tableau non cite : {against}"
                );
                assert!(on.starts_with("20"), "date de controle absente : {on}");
            }
            mecatol_core::VerificationStatus::Unverified { pending } => {
                // Etat legitime, mais il doit dire ce qu'il reste a faire.
                assert!(!pending.is_empty(), "non verifie sans marche a suivre");
            }
        }
    }

    #[test]
    fn valeurs_it_de_reference() {
        assert_eq!(it("10", 7), Length::from_micrometres(15));
        assert_eq!(it("10", 6), Length::from_micrometres(9));
        assert_eq!(it("20", 7), Length::from_micrometres(21));
        assert_eq!(it("20", 6), Length::from_micrometres(13));
        assert_eq!(it("25", 7), Length::from_micrometres(21));
        assert_eq!(it("50", 7), Length::from_micrometres(25));
        assert_eq!(it("500", 7), Length::from_micrometres(63));
    }

    #[test]
    fn les_bornes_dechelon_sont_incluses_en_haut() {
        // 10 mm appartient a l'echelon 6..10, pas a 10..18.
        assert_eq!(it("10", 7), Length::from_micrometres(15));
        assert_eq!(it("10.001", 7), Length::from_micrometres(18));
        // 18 mm appartient a 10..18.
        assert_eq!(it("18", 7), Length::from_micrometres(18));
        assert_eq!(it("18.001", 7), Length::from_micrometres(21));
    }

    #[test]
    fn lechelon_utilise_est_rendu_avec_la_valeur() {
        let found = table()
            .value(
                Length::from_millimetres(10),
                Grade::from_it_number(7).unwrap(),
            )
            .unwrap();
        assert_eq!(found.range_index, 2);
        assert_eq!(found.range.label_fr(), "au-dessus de 6 jusqu'à 10");
    }

    #[test]
    fn hors_plage_le_moteur_refuse_au_lieu_dextrapoler() {
        let err = table()
            .value(
                Length::from_millimetres(600),
                Grade::from_it_number(7).unwrap(),
            )
            .unwrap_err();
        assert!(matches!(err, StandardsError::NominalOutOfRange { .. }));

        let err = table()
            .value(Length::ZERO, Grade::from_it_number(7).unwrap())
            .unwrap_err();
        assert!(matches!(err, StandardsError::NominalOutOfRange { .. }));
    }

    #[test]
    fn les_degres_grossiers_sont_refuses_sous_1_mm() {
        let err = table()
            .value(
                Length::parse("0.5", Unit::Millimetre).unwrap(),
                Grade::from_it_number(14).unwrap(),
            )
            .unwrap_err();
        assert!(matches!(err, StandardsError::GradeNotApplicable { .. }));

        // ... et acceptes au-dessus.
        assert!(table()
            .value(
                Length::parse("1.5", Unit::Millimetre).unwrap(),
                Grade::from_it_number(14).unwrap()
            )
            .is_ok());
        // IT13 reste applicable sous 1 mm.
        assert!(table()
            .value(
                Length::parse("0.5", Unit::Millimetre).unwrap(),
                Grade::from_it_number(13).unwrap()
            )
            .is_ok());
    }

    #[test]
    fn une_table_discontinue_est_rejetee() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3], [6, 10]],
            "grades": { "IT7": [10, 15] }
        }"#;
        let err = ItGradeTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("discontinuite")),
            "attendu une discontinuite, obtenu {err}"
        );
    }

    #[test]
    fn une_ligne_de_mauvaise_longueur_est_rejetee() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3], [3, 6]],
            "grades": { "IT7": [10] }
        }"#;
        let err = ItGradeTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { .. }),
            "obtenu {err}"
        );
    }

    #[test]
    fn un_degre_plus_grossier_mais_plus_etroit_est_rejete() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3]],
            "grades": { "IT6": [10], "IT7": [8] }
        }"#;
        let err = ItGradeTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("plus fin")),
            "obtenu {err}"
        );
    }

    #[test]
    fn une_tolerance_qui_diminue_quand_la_piece_grossit_est_rejetee() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3], [3, 6]],
            "grades": { "IT7": [12, 10] }
        }"#;
        let err = ItGradeTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("decroit")),
            "obtenu {err}"
        );
    }
}
