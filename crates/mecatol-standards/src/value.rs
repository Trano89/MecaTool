//! Lecture exacte des valeurs numeriques des fichiers de donnees.
//!
//! Un nombre JSON transite par un `f64`, ce qui suffirait a introduire dans les
//! tables normatives exactement le genre d'imprecision que Mecatol s'interdit.
//! On ne convertit donc jamais le flottant : on reprend son ecriture decimale
//! (`serde_json` restitue la representation la plus courte qui reboucle) et on
//! la relit avec l'analyseur exact de `mecatol-core`.
//!
//! Consequence utile : une table contenant `0.30000000000000004` est **rejetee**
//! au chargement au lieu d'etre silencieusement arrondie.

use mecatol_core::{Length, Unit};
use serde_json::Value;

use crate::error::{Result, StandardsError};

/// Convertit une valeur JSON (nombre ou chaine) en longueur exacte.
pub fn length_from_json(dataset: &str, value: &Value, unit: Unit) -> Result<Length> {
    let text = match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => {
            return Err(StandardsError::Malformed {
                dataset: dataset.to_string(),
                detail: format!("valeur numerique attendue, trouve {other}"),
            })
        }
    };
    Length::parse(&text, unit).map_err(|source| StandardsError::Malformed {
        dataset: dataset.to_string(),
        detail: format!("valeur {text:?} inexploitable en {unit} : {source}"),
    })
}

/// Convertit une suite de valeurs JSON.
pub fn lengths_from_json(dataset: &str, values: &[Value], unit: Unit) -> Result<Vec<Length>> {
    values
        .iter()
        .map(|v| length_from_json(dataset, v, unit))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn un_nombre_a_une_decimale_est_lu_exactement() {
        // 0,8 um vaut exactement 800 nm, et non 799,999...
        let v = length_from_json("test", &json!(0.8), Unit::Micrometre).unwrap();
        assert_eq!(v.nanometres(), 800);
    }

    #[test]
    fn un_entier_est_lu_exactement() {
        let v = length_from_json("test", &json!(63), Unit::Micrometre).unwrap();
        assert_eq!(v.nanometres(), 63_000);
    }

    #[test]
    fn une_chaine_est_acceptee() {
        let v = length_from_json("test", &json!("-0.014"), Unit::Millimetre).unwrap();
        assert_eq!(v.nanometres(), -14_000);
    }

    #[test]
    fn une_valeur_polluee_par_le_flottant_est_rejetee() {
        let err =
            length_from_json("test", &json!(0.30000000000000004), Unit::Micrometre).unwrap_err();
        assert!(matches!(err, StandardsError::Malformed { .. }));
    }

    #[test]
    fn un_type_inattendu_est_rejete() {
        let err = length_from_json("test", &json!(null), Unit::Micrometre).unwrap_err();
        assert!(matches!(err, StandardsError::Malformed { .. }));
        let err = length_from_json("test", &json!(true), Unit::Micrometre).unwrap_err();
        assert!(matches!(err, StandardsError::Malformed { .. }));
    }

    #[test]
    fn une_ligne_complete_est_convertie() {
        let row = vec![json!(0.3), json!(0.4), json!(1), json!(2.5)];
        let lengths = lengths_from_json("test", &row, Unit::Micrometre).unwrap();
        assert_eq!(
            lengths.iter().map(|l| l.nanometres()).collect::<Vec<_>>(),
            vec![300, 400, 1_000, 2_500]
        );
    }
}
