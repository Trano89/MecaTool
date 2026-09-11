//! Ecarts fondamentaux de l'ISO 286-1.
//!
//! L'ecart fondamental est celui des deux ecarts que la **lettre** fixe ; le
//! degre `IT` fournit ensuite la largeur, et le second ecart s'en deduit.
//!
//! ```text
//!   lettres a..h   la lettre donne es (<= 0)   ->   ei = es - IT
//!   lettres j..zc  la lettre donne ei (>= 0)   ->   es = ei + IT
//!   js             zone symetrique             ->   es = +IT/2, ei = -IT/2
//! ```
//!
//! Les valeurs tabulees ici sont celles des **arbres**. Celles des alesages ne
//! sont pas une seconde table : la norme les derive des precedentes par la
//! regle exposee dans [`hole_rule`].

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use mecatool_core::{DeviationLetter, Grade, Length, SizeRange, StandardReference, Unit};
use serde::Deserialize;
use serde_json::Value;

use crate::error::{Result, StandardsError};
use crate::value::{length_from_json, lengths_from_json};

/// Lequel des deux ecarts la lettre fixe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviationSide {
    /// La lettre donne l'ecart superieur `es` / `ES` (lettres a..h).
    Upper,
    /// La lettre donne l'ecart inferieur `ei` / `EI` (lettres j..zc).
    Lower,
    /// Zone symetrique `+/- IT/2` (lettre js).
    SymmetricHalfIt,
}

impl DeviationSide {
    pub const fn symbol_fr(self) -> &'static str {
        match self {
            DeviationSide::Upper => "es",
            DeviationSide::Lower => "ei",
            DeviationSide::SymmetricHalfIt => "+/- IT/2",
        }
    }
}

/// Un ecart fondamental lu dans la table, avec sa provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaftDeviation {
    /// La valeur tabulee. Sans objet lorsque `side` vaut `SymmetricHalfIt`.
    pub value: Length,
    pub side: DeviationSide,
    pub letter: DeviationLetter,
    pub range: SizeRange,
    pub range_index: usize,
}

#[derive(Debug, Deserialize)]
struct LetterEntry {
    side: DeviationSide,
    #[serde(default)]
    values: Option<Vec<Value>>,
    /// Degres pour lesquels `values` s'applique ; ailleurs l'ecart est nul.
    #[serde(default)]
    applies_to_grades: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DeviationsFile {
    dataset: String,
    standard: StandardReference,
    value_unit: Unit,
    size_unit: Unit,
    size_ranges: Vec<Vec<Value>>,
    letters: BTreeMap<String, LetterEntry>,
}

#[derive(Debug, Clone)]
struct LetterRule {
    side: DeviationSide,
    values: Vec<Length>,
    restricted_to: Option<BTreeSet<Grade>>,
}

/// La table des ecarts fondamentaux des arbres, validee.
#[derive(Debug, Clone)]
pub struct ShaftDeviationTable {
    dataset: String,
    standard: StandardReference,
    ranges: Vec<SizeRange>,
    rules: BTreeMap<DeviationLetter, LetterRule>,
}

const EMBEDDED: &str = include_str!("../../../../data/iso286/iso286-1-2010.shaft-deviations.json");

impl ShaftDeviationTable {
    /// La table embarquee dans le binaire.
    pub fn embedded() -> Result<&'static ShaftDeviationTable> {
        static CACHE: OnceLock<core::result::Result<ShaftDeviationTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| ShaftDeviationTable::from_json(EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    pub fn from_json(text: &str) -> Result<ShaftDeviationTable> {
        let file: DeviationsFile =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso286-1 shaft-deviations".to_string(),
                detail: source.to_string(),
            })?;
        let dataset = file.dataset.clone();

        let mut ranges = Vec::with_capacity(file.size_ranges.len());
        for bounds in &file.size_ranges {
            let [above, up_to] = bounds.as_slice() else {
                return Err(StandardsError::Malformed {
                    dataset,
                    detail: format!("un echelon doit avoir 2 bornes, trouve {}", bounds.len()),
                });
            };
            ranges.push(SizeRange::new(
                length_from_json(&dataset, above, file.size_unit)?,
                length_from_json(&dataset, up_to, file.size_unit)?,
            ));
        }

        let mut rules = BTreeMap::new();
        for (name, entry) in &file.letters {
            let letter = DeviationLetter::parse(name)?;
            let values = match (&entry.values, entry.side) {
                (Some(v), _) => lengths_from_json(&dataset, v, file.value_unit)?,
                (None, DeviationSide::SymmetricHalfIt) => Vec::new(),
                (None, _) => {
                    return Err(StandardsError::Malformed {
                        dataset,
                        detail: format!("la lettre {name} n'a ni valeurs ni regle symetrique"),
                    })
                }
            };
            let restricted_to = entry
                .applies_to_grades
                .as_ref()
                .map(|names| {
                    names
                        .iter()
                        .map(|n| Grade::parse(n))
                        .collect::<core::result::Result<BTreeSet<_>, _>>()
                })
                .transpose()?;
            rules.insert(
                letter,
                LetterRule {
                    side: entry.side,
                    values,
                    restricted_to,
                },
            );
        }

        let table = ShaftDeviationTable {
            dataset,
            standard: file.standard,
            ranges,
            rules,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        if self.ranges.is_empty() {
            return Err(bad("aucun echelon de dimensions nominales".into()));
        }
        for (i, range) in self.ranges.iter().enumerate() {
            if range.above >= range.up_to {
                return Err(bad(format!("echelon {i} : bornes inversees")));
            }
            if i > 0 && self.ranges[i - 1].up_to != range.above {
                return Err(bad(format!("discontinuite avant l'echelon {i}")));
            }
        }

        let width = self.ranges.len();
        for (letter, rule) in &self.rules {
            if rule.side == DeviationSide::SymmetricHalfIt {
                if !rule.values.is_empty() {
                    return Err(bad(format!(
                        "{letter} est symetrique et ne doit pas porter de valeurs tabulees"
                    )));
                }
                continue;
            }
            if rule.values.len() != width {
                return Err(bad(format!(
                    "{letter} comporte {} valeurs pour {width} echelons",
                    rule.values.len()
                )));
            }
            // Le signe est impose par le cote : es <= 0 pour a..h, ei >= 0 pour j..zc.
            for (i, value) in rule.values.iter().enumerate() {
                let wrong_sign = match rule.side {
                    DeviationSide::Upper => value.is_positive(),
                    DeviationSide::Lower => value.is_negative(),
                    DeviationSide::SymmetricHalfIt => false,
                };
                if wrong_sign {
                    return Err(bad(format!(
                        "{letter} echelon {i} : {value} a un signe incompatible avec un ecart {}",
                        rule.side.symbol_fr()
                    )));
                }
            }
            // L'ecart s'eloigne de la ligne zero quand la piece grossit.
            for i in 1..width {
                let (prev, cur) = (rule.values[i - 1], rule.values[i]);
                let regressed = match rule.side {
                    DeviationSide::Upper => cur > prev,
                    DeviationSide::Lower => cur < prev,
                    DeviationSide::SymmetricHalfIt => false,
                };
                if regressed {
                    return Err(bad(format!(
                        "{letter} se rapproche de la ligne zero de l'echelon {} a {i} : {prev} puis {cur}",
                        i - 1
                    )));
                }
            }
        }

        // A dimension egale, les lettres se succedent du plus grand jeu au plus
        // grand serrage : leurs ecarts fondamentaux ne peuvent pas se croiser.
        let ordered: Vec<(&DeviationLetter, &LetterRule)> = self
            .rules
            .iter()
            .filter(|(_, r)| r.side != DeviationSide::SymmetricHalfIt)
            .collect();
        for pair in ordered.windows(2) {
            let ((left, lrule), (right, rrule)) = (pair[0], pair[1]);
            if lrule.side != rrule.side {
                continue;
            }
            for i in 0..width {
                if lrule.values[i] > rrule.values[i] {
                    return Err(bad(format!(
                        "echelon {i} : {left} ({}) devrait rester sous {right} ({})",
                        lrule.values[i], rrule.values[i]
                    )));
                }
            }
        }

        Ok(())
    }

    /// L'ecart fondamental d'un arbre pour une dimension, une lettre et un degre.
    pub fn fundamental(
        &self,
        nominal: Length,
        letter: DeviationLetter,
        grade: Grade,
    ) -> Result<ShaftDeviation> {
        let range_index = self.range_index(nominal)?;
        let rule = self
            .rules
            .get(&letter)
            .ok_or_else(|| StandardsError::LetterUnavailable {
                dataset: self.dataset.clone(),
                letter: letter.as_lower().to_string(),
            })?;

        let value = match rule.side {
            DeviationSide::SymmetricHalfIt => Length::ZERO,
            _ => match &rule.restricted_to {
                // Hors des degres vises, l'ecart fondamental est nul (cas de k).
                Some(grades) if !grades.contains(&grade) => Length::ZERO,
                _ => rule.values[range_index],
            },
        };

        Ok(ShaftDeviation {
            value,
            side: rule.side,
            letter,
            range: self.ranges[range_index],
            range_index,
        })
    }

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

    /// Les lettres effectivement couvertes par ce jeu de donnees.
    pub fn letters(&self) -> impl Iterator<Item = DeviationLetter> + '_ {
        self.rules.keys().copied()
    }

    pub fn has_letter(&self, letter: DeviationLetter) -> bool {
        self.rules.contains_key(&letter)
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
}

/// La regle de derivation des ecarts d'alesage a partir de ceux des arbres.
///
/// L'ISO 286-1 ne tabule pas deux fois les ecarts fondamentaux : ceux des
/// alesages se deduisent de ceux des arbres de meme lettre.
///
/// **Regle generale**
///
/// ```text
///   lettres A..H    EI = -es(arbre correspondant)
///   lettres J..ZC   ES = -ei(arbre correspondant)
/// ```
///
/// **Regle speciale, dite regle du delta**
///
/// Pour K, M, N jusqu'au degre IT8, et pour P a ZC jusqu'au degre IT7 :
///
/// ```text
///   ES = -ei + delta        avec  delta = IT(n) - IT(n-1)
/// ```
///
/// Sans ce correctif, un alesage et un arbre de meme lettre mais de degres
/// differents ne donneraient pas le meme serrage, ce que la norme cherche
/// precisement a garantir.
pub mod hole_rule {
    use mecatool_core::{DeviationLetter, Grade};

    /// Vrai si la lettre fixe l'ecart inferieur de l'arbre (lettres j..zc).
    pub fn uses_lower_deviation(letter: DeviationLetter) -> bool {
        letter >= DeviationLetter::Js
    }

    /// Vrai si le correctif `delta` s'applique a cette combinaison.
    pub fn applies_delta(letter: DeviationLetter, grade: Grade) -> bool {
        use DeviationLetter::*;
        let Some(number) = grade.it_number() else {
            return false;
        };
        match letter {
            K | M | N => number <= 8,
            P | R | S | T | U | V | X | Y | Z | Za | Zb | Zc => number <= 7,
            _ => false,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn le_delta_sapplique_a_k_m_n_jusqua_it8() {
            let it8 = Grade::from_it_number(8).unwrap();
            let it9 = Grade::from_it_number(9).unwrap();
            for letter in [DeviationLetter::K, DeviationLetter::M, DeviationLetter::N] {
                assert!(applies_delta(letter, it8), "{letter} en IT8");
                assert!(!applies_delta(letter, it9), "{letter} en IT9");
            }
        }

        #[test]
        fn le_delta_sapplique_a_p_et_au_dela_jusqua_it7() {
            let it7 = Grade::from_it_number(7).unwrap();
            let it8 = Grade::from_it_number(8).unwrap();
            assert!(applies_delta(DeviationLetter::P, it7));
            assert!(!applies_delta(DeviationLetter::P, it8));
            assert!(applies_delta(DeviationLetter::Zc, it7));
        }

        #[test]
        fn le_delta_ne_sapplique_jamais_aux_lettres_de_jeu() {
            let it6 = Grade::from_it_number(6).unwrap();
            for letter in [
                DeviationLetter::A,
                DeviationLetter::G,
                DeviationLetter::H,
                DeviationLetter::Js,
                DeviationLetter::J,
            ] {
                assert!(!applies_delta(letter, it6), "{letter}");
            }
        }

        #[test]
        fn la_frontiere_entre_les_deux_regles_generales_est_js() {
            assert!(!uses_lower_deviation(DeviationLetter::H));
            assert!(uses_lower_deviation(DeviationLetter::Js));
            assert!(uses_lower_deviation(DeviationLetter::P));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso286::ItGradeTable;

    fn table() -> &'static ShaftDeviationTable {
        ShaftDeviationTable::embedded().expect("la table embarquee doit se charger")
    }

    fn fd(nominal_mm: &str, letter: &str, grade: u8) -> Length {
        table()
            .fundamental(
                Length::parse(nominal_mm, Unit::Millimetre).unwrap(),
                DeviationLetter::parse(letter).unwrap(),
                Grade::from_it_number(grade).unwrap(),
            )
            .unwrap()
            .value
    }

    #[test]
    fn la_table_se_charge_et_passe_ses_invariants() {
        let t = table();
        assert_eq!(t.ranges().len(), 13);
        assert_eq!(t.letters().count(), 10);
        assert!(
            t.standard().verification.is_verified(),
            "les valeurs ont ete confrontees a l'ISO 286-2 : voir scripts/verify-iso286-tables.py"
        );
    }

    #[test]
    fn ecarts_de_reference_des_arbres() {
        // g6 a Ou10 : es = -5 um.
        assert_eq!(fd("10", "g", 6), Length::from_micrometres(-5));
        // g6 a Ou20 : es = -7 um.
        assert_eq!(fd("20", "g", 6), Length::from_micrometres(-7));
        // h : ecart superieur nul par definition.
        assert_eq!(fd("10", "h", 6), Length::ZERO);
        assert_eq!(fd("250", "h", 11), Length::ZERO);
        // f7 a Ou20 : es = -20 um.
        assert_eq!(fd("20", "f", 7), Length::from_micrometres(-20));
        // p6 a Ou20 : ei = +22 um.
        assert_eq!(fd("20", "p", 6), Length::from_micrometres(22));
        // n6 a Ou20 : ei = +15 um.
        assert_eq!(fd("20", "n", 6), Length::from_micrometres(15));
    }

    #[test]
    fn la_lettre_k_ne_vaut_que_pour_les_degres_it4_a_it7() {
        // k6 a Ou20 : ei = +2 um.
        assert_eq!(fd("20", "k", 6), Length::from_micrometres(2));
        assert_eq!(fd("20", "k", 4), Length::from_micrometres(2));
        assert_eq!(fd("20", "k", 7), Length::from_micrometres(2));
        // Hors de cette fenetre, l'ecart fondamental de k est nul.
        assert_eq!(fd("20", "k", 3), Length::ZERO);
        assert_eq!(fd("20", "k", 8), Length::ZERO);
        assert_eq!(fd("20", "k", 11), Length::ZERO);
    }

    #[test]
    fn js_est_symetrique_et_ne_porte_pas_de_valeur_tabulee() {
        let found = table()
            .fundamental(
                Length::from_millimetres(20),
                DeviationLetter::Js,
                Grade::from_it_number(7).unwrap(),
            )
            .unwrap();
        assert_eq!(found.side, DeviationSide::SymmetricHalfIt);
        assert_eq!(found.value, Length::ZERO);
    }

    #[test]
    fn une_lettre_non_couverte_est_refusee_et_non_approximee() {
        let err = table()
            .fundamental(
                Length::from_millimetres(20),
                DeviationLetter::S,
                Grade::from_it_number(6).unwrap(),
            )
            .unwrap_err();
        assert!(matches!(err, StandardsError::LetterUnavailable { .. }));
        assert!(!table().has_letter(DeviationLetter::A));
        assert!(!table().has_letter(DeviationLetter::Zc));
    }

    #[test]
    fn hors_plage_le_moteur_refuse() {
        let err = table()
            .fundamental(
                Length::from_millimetres(600),
                DeviationLetter::G,
                Grade::from_it_number(6).unwrap(),
            )
            .unwrap_err();
        assert!(matches!(err, StandardsError::NominalOutOfRange { .. }));
    }

    /// Recoupement entre deux tables saisies independamment.
    ///
    /// L'ecart fondamental de `m` vaut `IT7 - IT6` sur tous les echelons sauf le
    /// premier, ou la norme s'ecarte de la formule. Si l'une des deux tables
    /// comportait une faute de frappe, cette egalite tomberait.
    #[test]
    fn lecart_de_m_recoupe_la_difference_it7_moins_it6() {
        let deviations = table();
        let grades = ItGradeTable::embedded().unwrap();
        let it6 = Grade::from_it_number(6).unwrap();
        let it7 = Grade::from_it_number(7).unwrap();

        for (index, range) in deviations.ranges().iter().enumerate().skip(1) {
            let probe = range.above + Length::from_nanometres(1);
            let m = deviations
                .fundamental(probe, DeviationLetter::M, it6)
                .unwrap()
                .value;
            let expected =
                grades.value(probe, it7).unwrap().value - grades.value(probe, it6).unwrap().value;
            assert_eq!(
                m, expected,
                "echelon {index} ({range}) : m = {m}, IT7 - IT6 = {expected}"
            );
        }
    }

    /// Second recoupement : l'ecart de `n` vaut le double de celui de `g`, au
    /// signe pres, les deux derivant de la meme puissance de D.
    #[test]
    fn lecart_de_n_vaut_le_double_de_celui_de_g() {
        let t = table();
        let it6 = Grade::from_it_number(6).unwrap();
        for range in t.ranges().iter().skip(1) {
            let probe = range.above + Length::from_nanometres(1);
            let g = t.fundamental(probe, DeviationLetter::G, it6).unwrap().value;
            let n = t.fundamental(probe, DeviationLetter::N, it6).unwrap().value;
            let doubled = -g - g;
            let gap = (n - doubled).abs();
            assert!(
                gap <= Length::from_micrometres(1),
                "echelon {range} : n = {n}, 2 x |g| = {doubled}"
            );
        }
    }

    #[test]
    fn un_signe_incompatible_avec_le_cote_est_rejete() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3]],
            "letters": { "G": { "side": "upper", "values": [5] } }
        }"#;
        let err = ShaftDeviationTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("signe")),
            "obtenu {err}"
        );
    }

    #[test]
    fn des_lettres_qui_se_croisent_sont_rejetees() {
        // f doit rester sous g : ici f = -2 et g = -6, donc croisement.
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3]],
            "letters": {
                "F": { "side": "upper", "values": [-2] },
                "G": { "side": "upper", "values": [-6] }
            }
        }"#;
        let err = ShaftDeviationTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("rester sous")),
            "obtenu {err}"
        );
    }

    #[test]
    fn un_ecart_qui_se_rapproche_de_zero_quand_la_piece_grossit_est_rejete() {
        let broken = r#"{
            "dataset": "test",
            "standard": {
                "id": "X", "edition": "1", "title": "t", "source": "s",
                "verification": { "state": "unverified", "pending": "p" }
            },
            "value_unit": "micrometre",
            "size_unit": "millimetre",
            "size_ranges": [[0, 3], [3, 6]],
            "letters": { "G": { "side": "upper", "values": [-6, -2] } }
        }"#;
        let err = ShaftDeviationTable::from_json(broken).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. } if detail.contains("rapproche")),
            "obtenu {err}"
        );
    }
}
