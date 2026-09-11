//! Angles exacts.
//!
//! Meme parti pris que pour les longueurs : un entier, jamais un flottant.
//!
//! L'unite interne est la **milliseconde d'arc**. Le choix n'est pas arbitraire :
//! les tolerances angulaires se lisent en degres et en minutes, et une minute ne
//! tombe pas juste en degres decimaux. `0°20'` vaut `1/3` de degre — impossible
//! a ecrire exactement en base dix. En secondes d'arc, c'est `1200"`, exact.
//!
//! ```text
//!   1° = 60'          = 3 600"        = 3 600 000 mas
//!   1' =  60"         =    60 000 mas
//!   1" =   1 000 mas
//! ```
//!
//! La milliseconde plutot que la seconde laisse de la marge pour les tolerances
//! geometriques fines, ou l'on rencontre des valeurs en degres decimaux :
//! `0,001°` vaut `3 600 mas`, exact lui aussi.

use core::fmt;
use core::ops::{Add, Neg, Sub};

use serde::{Deserialize, Serialize};

/// Millisecondes d'arc dans une seconde d'arc.
pub const MAS_PER_ARCSECOND: i64 = 1_000;
/// Millisecondes d'arc dans une minute d'arc.
pub const MAS_PER_ARCMINUTE: i64 = 60_000;
/// Millisecondes d'arc dans un degre.
pub const MAS_PER_DEGREE: i64 = 3_600_000;

/// Erreurs de lecture d'un angle.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AngleError {
    #[error("valeur angulaire vide")]
    Empty,
    #[error(
        "angle illisible : {0:?} (attendu par ex. \"1\u{b0}30'\", \"0\u{b0}20'\" ou \"1.5\u{b0}\")"
    )]
    Invalid(String),
    #[error(
        "l'angle {0:?} ne peut pas etre represente exactement : MecaTool travaille \
         a la milliseconde d'arc pres"
    )]
    NotRepresentable(String),
    #[error("depassement de capacite lors du calcul d'un angle")]
    Overflow,
}

/// Un angle signe, exact, stocke en millisecondes d'arc.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Angle {
    mas: i64,
}

impl Angle {
    pub const ZERO: Angle = Angle { mas: 0 };

    pub const fn from_milliarcseconds(mas: i64) -> Self {
        Angle { mas }
    }

    pub const fn from_arcseconds(seconds: i64) -> Self {
        Angle {
            mas: seconds * MAS_PER_ARCSECOND,
        }
    }

    pub const fn from_arcminutes(minutes: i64) -> Self {
        Angle {
            mas: minutes * MAS_PER_ARCMINUTE,
        }
    }

    pub const fn from_degrees(degrees: i64) -> Self {
        Angle {
            mas: degrees * MAS_PER_DEGREE,
        }
    }

    /// Construit depuis la notation sexagesimale, toutes parts de meme signe.
    ///
    /// `from_dms(0, 30, 0)` vaut une demi-heure d'angle ; `from_dms(-1, 30, 0)`
    /// vaut `-1°30'`, et non `-1° + 30'`.
    pub const fn from_dms(degrees: i64, minutes: i64, seconds: i64) -> Self {
        let magnitude = degrees.abs() * MAS_PER_DEGREE
            + minutes.abs() * MAS_PER_ARCMINUTE
            + seconds.abs() * MAS_PER_ARCSECOND;
        let negative = degrees < 0 || minutes < 0 || seconds < 0;
        Angle {
            mas: if negative { -magnitude } else { magnitude },
        }
    }

    pub const fn milliarcseconds(self) -> i64 {
        self.mas
    }

    pub const fn is_zero(self) -> bool {
        self.mas == 0
    }

    pub const fn is_negative(self) -> bool {
        self.mas < 0
    }

    pub const fn abs(self) -> Self {
        Angle {
            mas: if self.mas < 0 { -self.mas } else { self.mas },
        }
    }

    /// Decompose en degres, minutes et secondes, tous de meme signe que l'angle.
    pub const fn to_dms(self) -> (i64, i64, i64) {
        let magnitude = self.abs().mas;
        let degrees = magnitude / MAS_PER_DEGREE;
        let rest = magnitude % MAS_PER_DEGREE;
        let minutes = rest / MAS_PER_ARCMINUTE;
        let seconds = (rest % MAS_PER_ARCMINUTE) / MAS_PER_ARCSECOND;
        if self.mas < 0 {
            (-degrees, -minutes, -seconds)
        } else {
            (degrees, minutes, seconds)
        }
    }

    /// Lit `"1\u{b0}30'"`, `"0\u{b0}20'"`, `"90\u{b0}"`, `"1.5\u{b0}"` ou `"30'"`.
    ///
    /// Le degre decimal est accepte, mais rejete s'il ne tombe pas exactement sur
    /// une milliseconde d'arc — plutot que d'etre arrondi en silence.
    pub fn parse(input: &str) -> Result<Self, AngleError> {
        let trimmed = input.trim().replace(',', ".");
        if trimmed.is_empty() {
            return Err(AngleError::Empty);
        }

        let (negative, body) = match trimmed.as_bytes()[0] {
            b'-' => (true, trimmed[1..].trim_start()),
            b'+' => (false, trimmed[1..].trim_start()),
            _ => (false, trimmed.as_str()),
        };

        let mut total: i64 = 0;
        let mut rest = body;
        let mut matched = false;

        for (marker, scale) in [
            ('\u{b0}', MAS_PER_DEGREE),
            ('\'', MAS_PER_ARCMINUTE),
            ('"', MAS_PER_ARCSECOND),
        ] {
            let Some(position) = rest.find(marker) else {
                continue;
            };
            let (head, tail) = rest.split_at(position);
            let value = head.trim();
            if !value.is_empty() {
                total = total
                    .checked_add(scaled(value, scale, input)?)
                    .ok_or(AngleError::Overflow)?;
                matched = true;
            }
            rest = &tail[marker.len_utf8()..];
        }

        if !matched || !rest.trim().is_empty() {
            return Err(AngleError::Invalid(input.to_string()));
        }
        Ok(Angle::from_milliarcseconds(if negative {
            -total
        } else {
            total
        }))
    }

    /// Ecriture sexagesimale compacte : `"1\u{b0}"`, `"0\u{b0}30'"`, `"0\u{b0}0'30\""`.
    ///
    /// Les parts nulles de tete sont conservees quand une part plus fine suit,
    /// pour que la lecture reste sans ambiguite.
    pub fn to_dms_string(self) -> String {
        let (degrees, minutes, seconds) = self.abs().to_dms();
        let sign = if self.mas < 0 { "-" } else { "" };
        let body = match (degrees, minutes, seconds) {
            (0, 0, 0) => "0\u{b0}".to_string(),
            (d, 0, 0) => format!("{d}\u{b0}"),
            (d, m, 0) => format!("{d}\u{b0}{m}'"),
            (d, m, s) => format!("{d}\u{b0}{m}'{s}\""),
        };
        format!("{sign}{body}")
    }

    /// Ecriture signee explicite d'un ecart : `"\u{b1} 0\u{b0}30'"`.
    pub fn to_symmetric_string(self) -> String {
        format!("\u{b1} {}", self.abs().to_dms_string())
    }
}

/// Convertit une part decimale en millisecondes d'arc, sans perte.
fn scaled(value: &str, scale: i64, original: &str) -> Result<i64, AngleError> {
    let (whole, fraction) = match value.split_once('.') {
        Some((w, f)) => (w, f),
        None => (value, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return Err(AngleError::Invalid(original.to_string()));
    }
    for part in [whole, fraction] {
        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(AngleError::Invalid(original.to_string()));
        }
    }
    if fraction.len() > 12 {
        return Err(AngleError::NotRepresentable(original.to_string()));
    }

    let whole_value: i128 = if whole.is_empty() {
        0
    } else {
        whole.parse().unwrap_or(0)
    };
    let mut total = whole_value * i128::from(scale);

    if !fraction.is_empty() {
        let divisor = 10i128.pow(fraction.len() as u32);
        let numerator = fraction.parse::<i128>().unwrap_or(0) * i128::from(scale);
        if numerator % divisor != 0 {
            return Err(AngleError::NotRepresentable(original.to_string()));
        }
        total += numerator / divisor;
    }
    i64::try_from(total).map_err(|_| AngleError::Overflow)
}

impl Add for Angle {
    type Output = Angle;
    fn add(self, rhs: Angle) -> Angle {
        Angle::from_milliarcseconds(self.mas + rhs.mas)
    }
}

impl Sub for Angle {
    type Output = Angle;
    fn sub(self, rhs: Angle) -> Angle {
        Angle::from_milliarcseconds(self.mas - rhs.mas)
    }
}

impl Neg for Angle {
    type Output = Angle;
    fn neg(self) -> Angle {
        Angle::from_milliarcseconds(-self.mas)
    }
}

impl core::ops::Mul<i64> for Angle {
    type Output = Angle;
    fn mul(self, factor: i64) -> Angle {
        Angle::from_milliarcseconds(self.mas * factor)
    }
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_dms_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_valeurs_de_liso_2768_sont_exactes() {
        // 0°20' vaut un tiers de degre : inexprimable en degres decimaux, exact ici.
        let twenty_minutes = Angle::from_dms(0, 20, 0);
        assert_eq!(twenty_minutes.milliarcseconds(), 1_200_000);
        assert_eq!(twenty_minutes * 3, Angle::from_degrees(1));
    }

    #[test]
    fn conversions_de_base() {
        assert_eq!(Angle::from_degrees(1), Angle::from_arcminutes(60));
        assert_eq!(Angle::from_arcminutes(1), Angle::from_arcseconds(60));
        assert_eq!(Angle::from_arcseconds(1).milliarcseconds(), 1_000);
        assert_eq!(Angle::from_dms(1, 30, 0), Angle::from_arcminutes(90));
    }

    #[test]
    fn decomposition_sexagesimale() {
        assert_eq!(Angle::from_dms(1, 30, 0).to_dms(), (1, 30, 0));
        assert_eq!(Angle::from_dms(0, 20, 0).to_dms(), (0, 20, 0));
        assert_eq!(Angle::from_degrees(3).to_dms(), (3, 0, 0));
        // Le signe porte sur l'ensemble, pas sur chaque part.
        assert_eq!(Angle::from_dms(-1, 30, 0).to_dms(), (-1, -30, 0));
    }

    #[test]
    fn ecriture_des_valeurs_normatives() {
        assert_eq!(Angle::from_degrees(1).to_dms_string(), "1\u{b0}");
        assert_eq!(Angle::from_dms(0, 30, 0).to_dms_string(), "0\u{b0}30'");
        assert_eq!(Angle::from_dms(0, 5, 0).to_dms_string(), "0\u{b0}5'");
        assert_eq!(Angle::from_dms(1, 30, 0).to_dms_string(), "1\u{b0}30'");
        assert_eq!(Angle::ZERO.to_dms_string(), "0\u{b0}");
        assert_eq!(
            Angle::from_dms(0, 30, 0).to_symmetric_string(),
            "\u{b1} 0\u{b0}30'"
        );
    }

    #[test]
    fn lecture_des_ecritures_usuelles() {
        assert_eq!(
            Angle::parse("1\u{b0}30'").unwrap(),
            Angle::from_dms(1, 30, 0)
        );
        assert_eq!(
            Angle::parse("0\u{b0}20'").unwrap(),
            Angle::from_dms(0, 20, 0)
        );
        assert_eq!(Angle::parse("90\u{b0}").unwrap(), Angle::from_degrees(90));
        assert_eq!(Angle::parse("30'").unwrap(), Angle::from_arcminutes(30));
        assert_eq!(Angle::parse("45\"").unwrap(), Angle::from_arcseconds(45));
        // Degre decimal, virgule francophone comprise.
        assert_eq!(
            Angle::parse("1.5\u{b0}").unwrap(),
            Angle::from_dms(1, 30, 0)
        );
        assert_eq!(
            Angle::parse("1,5\u{b0}").unwrap(),
            Angle::from_dms(1, 30, 0)
        );
        assert_eq!(
            Angle::parse("-0\u{b0}30'").unwrap(),
            -Angle::from_dms(0, 30, 0)
        );
    }

    #[test]
    fn aller_retour_texte() {
        for text in [
            "1\u{b0}",
            "0\u{b0}30'",
            "0\u{b0}20'",
            "1\u{b0}30'",
            "3\u{b0}",
        ] {
            assert_eq!(Angle::parse(text).unwrap().to_dms_string(), text);
        }
    }

    #[test]
    fn une_precision_impossible_est_refusee() {
        // 0,0000001° vaut 0,00036 mas : sous la resolution interne.
        let err = Angle::parse("0.0000001\u{b0}").unwrap_err();
        assert!(matches!(err, AngleError::NotRepresentable(_)));
    }

    #[test]
    fn ecritures_illisibles() {
        assert!(matches!(Angle::parse(""), Err(AngleError::Empty)));
        assert!(matches!(Angle::parse("abc"), Err(AngleError::Invalid(_))));
        // Un nombre sans unite est ambigu : degres ? minutes ? On refuse.
        assert!(matches!(Angle::parse("30"), Err(AngleError::Invalid(_))));
        assert!(matches!(
            Angle::parse("1\u{b0}30"),
            Err(AngleError::Invalid(_))
        ));
    }

    #[test]
    fn arithmetique_exacte() {
        let third = Angle::from_dms(0, 20, 0);
        assert_eq!(third + third + third, Angle::from_degrees(1));
        assert_eq!(Angle::from_degrees(1) - third, Angle::from_dms(0, 40, 0));
        // Un cumul de 1000 termes ne derive pas.
        let total: Angle = (0..1000).fold(Angle::ZERO, |acc, _| acc + Angle::from_arcseconds(1));
        assert_eq!(total, Angle::from_arcseconds(1000));
    }
}
