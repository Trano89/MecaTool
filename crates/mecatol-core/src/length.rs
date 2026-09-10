//! Longueurs exactes.
//!
//! Toute grandeur dimensionnelle de Mecatol est stockee comme un entier signe de
//! **nanometres**. Ce choix n'est pas cosmetique : il garantit que le moteur ne
//! produit jamais de resultat du type `0.1 + 0.2 = 0.30000000000000004`.
//!
//! Il est suffisant pour representer exactement :
//!
//! * toutes les valeurs des tables ISO 286 (donnees au micrometre, avec au plus
//!   une decimale : `IT1 = 0,8 um` -> `800 nm`) ;
//! * le pouce, defini exactement comme `25,4 mm` -> `25 400 000 nm`.
//!
//! La plage utile de `i64` en nanometres depasse le milliard de kilometres.

use core::fmt;
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// Nanometres par micrometre.
pub const NM_PER_UM: i64 = 1_000;
/// Nanometres par millimetre.
pub const NM_PER_MM: i64 = 1_000_000;
/// Nanometres par pouce (1 in = 25,4 mm exactement).
pub const NM_PER_INCH: i64 = 25_400_000;

/// Nombre maximal de chiffres acceptes de part et d'autre du separateur decimal.
const MAX_DIGITS: usize = 15;

/// Unite de longueur exposee a l'utilisateur.
///
/// L'unite est une propriete de *presentation*. Le moteur calcule toujours en
/// nanometres et ne consulte l'unite qu'au moment d'analyser ou d'afficher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Millimetre,
    Micrometre,
    Inch,
}

impl Unit {
    /// Nombre de nanometres dans une unite.
    pub const fn nanometres(self) -> i64 {
        match self {
            Unit::Millimetre => NM_PER_MM,
            Unit::Micrometre => NM_PER_UM,
            Unit::Inch => NM_PER_INCH,
        }
    }

    /// Symbole affiche.
    pub const fn symbol(self) -> &'static str {
        match self {
            Unit::Millimetre => "mm",
            Unit::Micrometre => "\u{b5}m",
            Unit::Inch => "in",
        }
    }

    /// Nombre de decimales utilise par defaut a l'affichage dans cette unite.
    pub const fn default_decimals(self) -> u32 {
        match self {
            Unit::Millimetre => 4,
            Unit::Micrometre => 1,
            Unit::Inch => 6,
        }
    }

    /// Toutes les unites supportees.
    pub const ALL: [Unit; 3] = [Unit::Millimetre, Unit::Micrometre, Unit::Inch];
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

impl core::str::FromStr for Unit {
    type Err = LengthError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mm" | "millimetre" | "millimeter" | "millimetres" | "millimeters" => {
                Ok(Unit::Millimetre)
            }
            "um" | "\u{b5}m" | "\u{3bc}m" | "micron" | "microns" | "micrometre" | "micrometer" => {
                Ok(Unit::Micrometre)
            }
            "in" | "inch" | "inches" | "\"" => Ok(Unit::Inch),
            _ => Err(LengthError::UnknownUnit(s.to_string())),
        }
    }
}

/// Erreurs liees a l'analyse ou a la conversion d'une longueur.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LengthError {
    #[error("valeur numerique vide")]
    Empty,
    #[error("caractere invalide dans la valeur numerique : {0:?}")]
    InvalidNumber(String),
    #[error("trop de chiffres dans la valeur numerique (maximum {MAX_DIGITS})")]
    TooManyDigits,
    #[error(
        "la valeur {value} {unit} ne peut pas etre representee exactement : \
         Mecatol travaille au nanometre pres"
    )]
    NotRepresentable { value: String, unit: Unit },
    #[error("depassement de capacite lors du calcul d'une longueur")]
    Overflow,
    #[error("unite inconnue : {0:?}")]
    UnknownUnit(String),
}

/// Une longueur signee, exacte, stockee en nanometres.
///
/// `Length` est volontairement `Ord` : comparer deux cotes est une operation
/// exacte sur des entiers, jamais une comparaison de flottants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Length {
    nm: i64,
}

impl Length {
    /// La longueur nulle.
    pub const ZERO: Length = Length { nm: 0 };

    /// Construit une longueur a partir d'un nombre entier de nanometres.
    pub const fn from_nanometres(nm: i64) -> Self {
        Length { nm }
    }

    /// Construit une longueur a partir d'un nombre entier de micrometres.
    pub const fn from_micrometres(um: i64) -> Self {
        Length { nm: um * NM_PER_UM }
    }

    /// Construit une longueur a partir d'un nombre entier de millimetres.
    pub const fn from_millimetres(mm: i64) -> Self {
        Length { nm: mm * NM_PER_MM }
    }

    /// La valeur en nanometres. C'est la seule representation interne.
    pub const fn nanometres(self) -> i64 {
        self.nm
    }

    pub const fn is_zero(self) -> bool {
        self.nm == 0
    }

    pub const fn is_negative(self) -> bool {
        self.nm < 0
    }

    pub const fn is_positive(self) -> bool {
        self.nm > 0
    }

    pub const fn abs(self) -> Self {
        Length {
            nm: if self.nm < 0 { -self.nm } else { self.nm },
        }
    }

    pub const fn signum(self) -> i64 {
        if self.nm > 0 {
            1
        } else if self.nm < 0 {
            -1
        } else {
            0
        }
    }

    pub fn checked_add(self, other: Self) -> Result<Self, LengthError> {
        self.nm
            .checked_add(other.nm)
            .map(Length::from_nanometres)
            .ok_or(LengthError::Overflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, LengthError> {
        self.nm
            .checked_sub(other.nm)
            .map(Length::from_nanometres)
            .ok_or(LengthError::Overflow)
    }

    pub const fn min(self, other: Self) -> Self {
        if self.nm <= other.nm {
            self
        } else {
            other
        }
    }

    pub const fn max(self, other: Self) -> Self {
        if self.nm >= other.nm {
            self
        } else {
            other
        }
    }

    /// Analyse une valeur decimale ecrite dans `unit`, sans perte.
    ///
    /// Accepte le point et la virgule comme separateur decimal, ainsi qu'un
    /// signe de tete. Rejette toute valeur qui ne tomberait pas exactement sur
    /// un nanometre plutot que de l'arrondir silencieusement.
    ///
    /// ```
    /// # use mecatol_core::{Length, Unit};
    /// let a = Length::parse("20.015", Unit::Millimetre).unwrap();
    /// assert_eq!(a.nanometres(), 20_015_000);
    /// // La virgule decimale francophone est acceptee.
    /// assert_eq!(Length::parse("20,015", Unit::Millimetre).unwrap(), a);
    /// ```
    pub fn parse(input: &str, unit: Unit) -> Result<Self, LengthError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(LengthError::Empty);
        }

        let (negative, body) = match trimmed.as_bytes()[0] {
            b'+' => (false, &trimmed[1..]),
            b'-' => (true, &trimmed[1..]),
            _ => (false, trimmed),
        };

        // La virgule est le separateur decimal usuel en francais et en allemand ;
        // les deux notations doivent donner exactement le meme resultat.
        let normalised = body.replace(',', ".");
        let mut parts = normalised.splitn(2, '.');
        let int_digits = parts.next().unwrap_or("");
        let frac_digits = parts.next().unwrap_or("");

        if int_digits.is_empty() && frac_digits.is_empty() {
            return Err(LengthError::Empty);
        }
        for part in [int_digits, frac_digits] {
            if !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(LengthError::InvalidNumber(input.to_string()));
            }
        }
        if int_digits.len() > MAX_DIGITS || frac_digits.len() > MAX_DIGITS {
            return Err(LengthError::TooManyDigits);
        }

        let scale = i128::from(unit.nanometres());

        let int_value: i128 = if int_digits.is_empty() {
            0
        } else {
            int_digits
                .parse::<i128>()
                .map_err(|_| LengthError::TooManyDigits)?
        };
        let mut total = int_value.checked_mul(scale).ok_or(LengthError::Overflow)?;

        if !frac_digits.is_empty() {
            let frac_value: i128 = frac_digits
                .parse::<i128>()
                .map_err(|_| LengthError::TooManyDigits)?;
            let divisor = pow10_i128(frac_digits.len() as u32)?;
            let numerator = frac_value.checked_mul(scale).ok_or(LengthError::Overflow)?;
            if numerator % divisor != 0 {
                // Refuser plutot qu'arrondir en silence : principe 1 (exactitude).
                return Err(LengthError::NotRepresentable {
                    value: trimmed.to_string(),
                    unit,
                });
            }
            total = total
                .checked_add(numerator / divisor)
                .ok_or(LengthError::Overflow)?;
        }

        if negative {
            total = -total;
        }
        let nm = i64::try_from(total).map_err(|_| LengthError::Overflow)?;
        Ok(Length::from_nanometres(nm))
    }

    /// Indique si la longueur s'ecrit exactement avec `decimals` decimales dans `unit`.
    pub fn is_exact_in(self, unit: Unit, decimals: u32) -> bool {
        let scale = i128::from(unit.nanometres());
        match pow10_i128(decimals) {
            Ok(factor) => (i128::from(self.nm) * factor) % scale == 0,
            Err(_) => false,
        }
    }

    /// Rend la longueur dans `unit` avec exactement `decimals` decimales.
    ///
    /// L'arrondi est "au plus proche, moitie a l'oppose de zero", convention
    /// usuelle en cotation. Utiliser [`Length::is_exact_in`] pour savoir si
    /// l'ecriture obtenue est exacte ou arrondie.
    pub fn to_decimal_string(self, unit: Unit, decimals: u32) -> String {
        let scale = i128::from(unit.nanometres());
        let factor = pow10_i128(decimals).unwrap_or(1);
        let magnitude = i128::from(self.nm).abs();

        let numerator = magnitude * factor;
        let mut quotient = numerator / scale;
        let remainder = numerator % scale;
        if remainder * 2 >= scale {
            quotient += 1;
        }

        let sign = if self.nm < 0 && quotient != 0 { "-" } else { "" };
        if decimals == 0 {
            return format!("{sign}{quotient}");
        }
        let whole = quotient / factor;
        let frac = quotient % factor;
        format!("{sign}{whole}.{frac:0width$}", width = decimals as usize)
    }

    /// Comme [`Length::to_decimal_string`], avec le nombre de decimales par defaut de l'unite.
    pub fn to_string_in(self, unit: Unit) -> String {
        self.to_decimal_string(unit, unit.default_decimals())
    }

    /// Rend la longueur suivie de son symbole d'unite, par ex. `"20.0150 mm"`.
    pub fn to_labelled_string(self, unit: Unit) -> String {
        format!("{} {}", self.to_string_in(unit), unit.symbol())
    }

    /// Ecriture signee explicite, utilisee pour les ecarts : `"+0.015"`, `"-0.014"`, `"0"`.
    pub fn to_deviation_string(self, unit: Unit, decimals: u32) -> String {
        if self.nm == 0 {
            return "0".to_string();
        }
        let rendered = self.to_decimal_string(unit, decimals);
        if self.nm > 0 {
            format!("+{rendered}")
        } else {
            rendered
        }
    }
}

fn pow10_i128(exp: u32) -> Result<i128, LengthError> {
    if exp > 30 {
        return Err(LengthError::TooManyDigits);
    }
    Ok(10i128.pow(exp))
}

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Length) -> Length {
        Length::from_nanometres(self.nm + rhs.nm)
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Length) -> Length {
        Length::from_nanometres(self.nm - rhs.nm)
    }
}

impl Neg for Length {
    type Output = Length;
    fn neg(self) -> Length {
        Length::from_nanometres(-self.nm)
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, rhs: Length) {
        self.nm += rhs.nm;
    }
}

impl SubAssign for Length {
    fn sub_assign(&mut self, rhs: Length) {
        self.nm -= rhs.nm;
    }
}

impl core::iter::Sum for Length {
    fn sum<I: Iterator<Item = Length>>(iter: I) -> Length {
        Length::from_nanometres(iter.map(|l| l.nm).sum())
    }
}

impl fmt::Display for Length {
    /// Affichage par defaut en millimetres, sans zeros terminaux superflus.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = self.to_decimal_string(Unit::Millimetre, 6);
        let trimmed = rendered.trim_end_matches('0').trim_end_matches('.');
        write!(f, "{} mm", if trimmed.is_empty() { "0" } else { trimmed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_probleme_du_flottant_nexiste_pas() {
        // 0,1 + 0,2 doit valoir exactement 0,3.
        let a = Length::parse("0.1", Unit::Millimetre).unwrap();
        let b = Length::parse("0.2", Unit::Millimetre).unwrap();
        let c = Length::parse("0.3", Unit::Millimetre).unwrap();
        assert_eq!(a + b, c);
        assert_eq!((a + b).to_decimal_string(Unit::Millimetre, 4), "0.3000");
    }

    #[test]
    fn cumul_de_mille_centiemes() {
        // Une chaine de cotes ne doit pas deriver, meme sur 1000 termes.
        let step = Length::parse("0.01", Unit::Millimetre).unwrap();
        let total: Length = (0..1000).map(|_| step).sum();
        assert_eq!(total, Length::from_millimetres(10));
    }

    #[test]
    fn parse_millimetres() {
        assert_eq!(
            Length::parse("20.015", Unit::Millimetre)
                .unwrap()
                .nanometres(),
            20_015_000
        );
        assert_eq!(
            Length::parse("-0.014", Unit::Millimetre)
                .unwrap()
                .nanometres(),
            -14_000
        );
        assert_eq!(Length::parse("0", Unit::Millimetre).unwrap(), Length::ZERO);
        assert_eq!(
            Length::parse(".5", Unit::Millimetre).unwrap().nanometres(),
            500_000
        );
    }

    #[test]
    fn parse_micrometres_avec_une_decimale() {
        // IT1 pour la plage 0 a 3 mm vaut 0,8 um.
        assert_eq!(
            Length::parse("0.8", Unit::Micrometre).unwrap().nanometres(),
            800
        );
    }

    #[test]
    fn virgule_et_point_sont_equivalents() {
        assert_eq!(
            Length::parse("12,5", Unit::Micrometre).unwrap(),
            Length::parse("12.5", Unit::Micrometre).unwrap()
        );
    }

    #[test]
    fn le_pouce_est_exact() {
        let one_inch = Length::parse("1", Unit::Inch).unwrap();
        assert_eq!(one_inch.nanometres(), 25_400_000);
        assert_eq!(
            one_inch,
            Length::from_millimetres(25) + Length::parse("0.4", Unit::Millimetre).unwrap()
        );
        assert_eq!(one_inch.to_decimal_string(Unit::Inch, 4), "1.0000");
    }

    #[test]
    fn refuse_une_precision_sub_nanometrique() {
        // 0,0000001 mm = 0,1 nm : non representable, donc rejete.
        let err = Length::parse("0.0000001", Unit::Millimetre).unwrap_err();
        assert!(matches!(err, LengthError::NotRepresentable { .. }));
    }

    #[test]
    fn refuse_les_entrees_malformees() {
        assert!(matches!(
            Length::parse("12.3.4", Unit::Millimetre),
            Err(LengthError::InvalidNumber(_))
        ));
        assert!(matches!(
            Length::parse("abc", Unit::Millimetre),
            Err(LengthError::InvalidNumber(_))
        ));
        assert!(matches!(
            Length::parse("   ", Unit::Millimetre),
            Err(LengthError::Empty)
        ));
    }

    #[test]
    fn conversion_entre_unites_sans_perte() {
        let d = Length::parse("25.4", Unit::Millimetre).unwrap();
        assert!(d.is_exact_in(Unit::Inch, 1));
        assert_eq!(d.to_decimal_string(Unit::Inch, 3), "1.000");
        assert_eq!(d.to_decimal_string(Unit::Micrometre, 0), "25400");
    }

    #[test]
    fn arrondi_a_la_moitie_a_l_oppose_de_zero() {
        // 0,0005 mm affiche avec 3 decimales -> 0,001 (et non 0,000).
        let v = Length::parse("0.0005", Unit::Millimetre).unwrap();
        assert_eq!(v.to_decimal_string(Unit::Millimetre, 3), "0.001");
        assert_eq!((-v).to_decimal_string(Unit::Millimetre, 3), "-0.001");
        // ... et l'affichage arrondi est signale comme inexact.
        assert!(!v.is_exact_in(Unit::Millimetre, 3));
        assert!(v.is_exact_in(Unit::Millimetre, 4));
    }

    #[test]
    fn ecriture_des_ecarts() {
        let es = Length::parse("-0.014", Unit::Millimetre).unwrap();
        let ei = Length::parse("0.015", Unit::Millimetre).unwrap();
        assert_eq!(es.to_deviation_string(Unit::Millimetre, 3), "-0.014");
        assert_eq!(ei.to_deviation_string(Unit::Millimetre, 3), "+0.015");
        assert_eq!(Length::ZERO.to_deviation_string(Unit::Millimetre, 3), "0");
    }

    #[test]
    fn unites_analysees_depuis_le_texte() {
        use core::str::FromStr;
        assert_eq!(Unit::from_str("mm").unwrap(), Unit::Millimetre);
        assert_eq!(Unit::from_str("um").unwrap(), Unit::Micrometre);
        assert_eq!(Unit::from_str("\u{b5}m").unwrap(), Unit::Micrometre);
        assert_eq!(Unit::from_str("inch").unwrap(), Unit::Inch);
        assert!(Unit::from_str("furlong").is_err());
    }
}
