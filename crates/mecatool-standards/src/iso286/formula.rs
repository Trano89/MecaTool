//! Controle de vraisemblance des degres `IT` par la formule.
//!
//! # Ce module ne produit jamais de valeur normative
//!
//! L'ISO 286-1 definit un facteur de tolerance normalise
//!
//! ```text
//!   i = 0,45 x racine_cubique(D) + 0,001 x D        (D en mm, i en um)
//! ```
//!
//! puis, pour les degres IT5 a IT18, `IT(n) = k(n) x i`. Mais **les valeurs
//! publiees sont arrondies vers des nombres normalises** et ne coincident pas
//! avec le calcul brut. Quelques ecarts constates :
//!
//! | Cas          | Formule | Table |
//! |--------------|---------|-------|
//! | IT7, 0..3    | 8,67 um | 10 um |
//! | IT7, 6..10   | 14,4 um | 15 um |
//! | IT7, 30..50  | 25,0 um | 25 um |
//!
//! La formule ne peut donc pas regenerer la table. Elle sert uniquement a
//! reperer une case aberrante : une erreur de frappe deplace la valeur d'un
//! facteur 10, la que l'arrondi normatif ne depasse jamais quelques pour cent.
//!
//! C'est aussi la seule partie du crate qui manipule des flottants, precisement
//! parce qu'elle est un diagnostic et non un chemin de calcul.

use mecatool_core::{Grade, Length, SizeRange, Unit};

use super::it_grades::ItGradeTable;

/// Ecart relatif maximal tolere entre table et formule avant signalement.
///
/// Calibre au-dessus du plus grand ecart d'arrondi normatif observe (environ
/// 13 % dans le premier echelon) et tres en dessous de ce que produirait une
/// faute de frappe.
pub const MAX_RELATIVE_DEVIATION: f64 = 0.20;

/// Facteur de tolerance normalise `i`, en micrometres, pour `d_mm` en millimetres.
///
/// Valable pour les dimensions nominales jusqu'a 500 mm.
pub fn tolerance_factor_um(d_mm: f64) -> f64 {
    0.45 * d_mm.cbrt() + 0.001 * d_mm
}

/// Multiplicateur `k` tel que `IT(n) = k x i`, pour les degres IT5 a IT18.
///
/// `None` pour IT01 a IT4, qui relevent d'autres expressions.
pub fn grade_multiplier(grade: Grade) -> Option<f64> {
    match grade.it_number()? {
        5 => Some(7.0),
        6 => Some(10.0),
        7 => Some(16.0),
        8 => Some(25.0),
        9 => Some(40.0),
        10 => Some(64.0),
        11 => Some(100.0),
        12 => Some(160.0),
        13 => Some(250.0),
        14 => Some(400.0),
        15 => Some(640.0),
        16 => Some(1000.0),
        17 => Some(1600.0),
        18 => Some(2500.0),
        _ => None,
    }
}

/// Dimension representative d'un echelon : la moyenne geometrique de ses bornes.
///
/// Le premier echelon part de 0, valeur inutilisable dans une moyenne
/// geometrique ; la convention usuelle lui substitue 1 mm.
pub fn representative_size_mm(range: SizeRange) -> f64 {
    let to_mm = |l: Length| l.nanometres() as f64 / mecatool_core::NM_PER_MM as f64;
    let above = to_mm(range.above);
    let up_to = to_mm(range.up_to);
    let low = if above <= 0.0 { 1.0 } else { above };
    (low * up_to).sqrt()
}

/// Une case de la table confrontee a la formule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossCheck {
    pub grade: Grade,
    pub range: SizeRange,
    /// Valeur lue dans la table, en micrometres.
    pub table_um: f64,
    /// Valeur calculee par la formule, en micrometres.
    pub formula_um: f64,
}

impl CrossCheck {
    /// Ecart relatif signe entre table et formule.
    pub fn relative_deviation(&self) -> f64 {
        (self.table_um - self.formula_um) / self.formula_um
    }

    /// Vrai si l'ecart depasse ce que l'arrondi normatif peut expliquer.
    pub fn is_suspicious(&self) -> bool {
        self.relative_deviation().abs() > MAX_RELATIVE_DEVIATION
    }
}

/// Confronte toutes les cases exploitables de la table a la formule.
///
/// Les degres IT01 a IT4 sont ignores : ils ne relevent pas de `k x i`.
pub fn cross_check(table: &ItGradeTable) -> Vec<CrossCheck> {
    let mut report = Vec::new();
    for grade in table.grades() {
        let Some(k) = grade_multiplier(grade) else {
            continue;
        };
        for (index, range) in table.ranges().iter().enumerate() {
            let midpoint = range.above + Length::from_nanometres(1);
            let Ok(found) = table.value(midpoint, grade) else {
                continue;
            };
            debug_assert_eq!(found.range_index, index);
            let d = representative_size_mm(*range);
            report.push(CrossCheck {
                grade,
                range: *range,
                table_um: found
                    .value
                    .to_decimal_string(Unit::Micrometre, 1)
                    .parse()
                    .unwrap_or(f64::NAN),
                formula_um: k * tolerance_factor_um(d),
            });
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_facteur_de_tolerance_suit_la_formule() {
        // Echelon 18..30 : D = racine(18 x 30) = 23,238 mm.
        let i = tolerance_factor_um(23.2379);
        assert!((i - 1.3076).abs() < 0.001, "i = {i}");
        // IT7 = 16 i, soit 20,9 um, arrondi a 21 um dans la table.
        assert!((16.0 * i - 20.92).abs() < 0.02);
    }

    #[test]
    fn dimension_representative_du_premier_echelon() {
        let first = SizeRange::new(Length::ZERO, Length::from_millimetres(3));
        // La borne basse nulle est remplacee par 1 mm : D = racine(1 x 3).
        assert!((representative_size_mm(first) - 3f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn aucune_case_de_la_table_nest_aberrante() {
        let table = ItGradeTable::embedded().unwrap();
        let report = cross_check(table);
        assert!(!report.is_empty());

        let suspicious: Vec<String> = report
            .iter()
            .filter(|c| c.is_suspicious())
            .map(|c| {
                format!(
                    "{} sur {} : table {} um, formule {:.2} um, ecart {:+.1} %",
                    c.grade.name(),
                    c.range,
                    c.table_um,
                    c.formula_um,
                    c.relative_deviation() * 100.0
                )
            })
            .collect();

        assert!(
            suspicious.is_empty(),
            "cases a verifier contre la source primaire :\n{}",
            suspicious.join("\n")
        );
    }

    #[test]
    fn lecart_darrondi_reste_modere_mais_non_nul() {
        // On documente le fait que la formule ne reproduit pas la table :
        // si un jour ce test echoue parce que tout coincide, c'est que
        // quelqu'un a regenere la table depuis la formule. Ce serait un bug.
        let table = ItGradeTable::embedded().unwrap();
        let report = cross_check(table);
        let exact = report
            .iter()
            .filter(|c| (c.table_um - c.formula_um).abs() < 1e-9)
            .count();
        assert!(
            exact < report.len() / 2,
            "la table semble avoir ete generee par la formule et non transcrite"
        );
    }

    #[test]
    fn les_degres_fins_nont_pas_de_multiplicateur() {
        assert_eq!(grade_multiplier(Grade::IT01), None);
        assert_eq!(grade_multiplier(Grade::IT0), None);
        assert_eq!(grade_multiplier(Grade::from_it_number(4).unwrap()), None);
        assert_eq!(
            grade_multiplier(Grade::from_it_number(7).unwrap()),
            Some(16.0)
        );
    }
}
