//! Mise en forme des grandeurs pour les explications.
//!
//! Ces fonctions ne servent qu'a l'affichage. Elles arrivent en bout de chaine,
//! une fois tous les calculs faits en entiers exacts.

use mecatool_core::{Length, Unit};

/// Retire les zeros decimaux inutiles : `"15.0"` devient `"15"`, `"10.5"` reste.
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

/// Une longueur en micrometres, par ex. `"15 um"` ou `"10,5 um"`.
pub fn um(value: Length) -> String {
    format!(
        "{} {}",
        trim(value.to_decimal_string(Unit::Micrometre, 1)),
        Unit::Micrometre.symbol()
    )
}

/// Une longueur en micrometres avec signe explicite, par ex. `"+15 um"`, `"-5 um"`, `"0"`.
///
/// Le zero s'ecrit sans signe ni unite : un ecart nul n'est ni positif ni negatif.
pub fn um_signed(value: Length) -> String {
    if value.is_zero() {
        return "0".to_string();
    }
    let body = um(value);
    if value.is_positive() {
        format!("+{body}")
    } else {
        body
    }
}

/// Une longueur en millimetres avec 3 decimales, comme sur un plan.
pub fn mm(value: Length) -> String {
    value.to_decimal_string(Unit::Millimetre, 3)
}

/// Une longueur en millimetres sans zero decimal superflu : `"0.05"`, `"0.2"`.
///
/// Une cote se pade — `10.015` se lit d'un coup d'oeil sur un plan. Une largeur
/// de zone de tolerance, non : les tables normatives ecrivent `0,08` et non
/// `0,080`, et l'ajout de zeros donnerait a croire a une precision de saisie qui
/// n'est pas celle de la source.
pub fn mm_trimmed(value: Length) -> String {
    trim(value.to_decimal_string(Unit::Millimetre, 3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micrometres_sans_decimale_superflue() {
        assert_eq!(um(Length::from_micrometres(15)), "15 \u{b5}m");
        assert_eq!(um(Length::from_nanometres(10_500)), "10.5 \u{b5}m");
        assert_eq!(um(Length::ZERO), "0 \u{b5}m");
    }

    #[test]
    fn ecarts_signes() {
        assert_eq!(um_signed(Length::from_micrometres(15)), "+15 \u{b5}m");
        assert_eq!(um_signed(Length::from_micrometres(-5)), "-5 \u{b5}m");
        // Un ecart nul n'a ni signe ni unite.
        assert_eq!(um_signed(Length::ZERO), "0");
    }

    #[test]
    fn millimetres_sans_zero_superflu() {
        // Une largeur de zone s'ecrit comme la source l'ecrit.
        assert_eq!(mm_trimmed(Length::from_micrometres(80)), "0.08");
        assert_eq!(mm_trimmed(Length::from_micrometres(200)), "0.2");
        assert_eq!(mm_trimmed(Length::from_millimetres(2)), "2");
        assert_eq!(mm_trimmed(Length::ZERO), "0");
        // Alors qu'une cote garde ses decimales.
        assert_eq!(mm(Length::from_micrometres(80)), "0.080");
    }

    #[test]
    fn millimetres_a_trois_decimales() {
        assert_eq!(
            mm(Length::parse("10.015", Unit::Millimetre).unwrap()),
            "10.015"
        );
        assert_eq!(mm(Length::from_millimetres(10)), "10.000");
    }
}
