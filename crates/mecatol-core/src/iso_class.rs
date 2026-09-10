//! Classes de tolerance ISO : lettre d'ecart fondamental + degre de tolerance.
//!
//! Une classe comme `H7` ou `g6` porte trois informations :
//!
//! * l'**element** : majuscule = alesage, minuscule = arbre ;
//! * la **lettre d'ecart fondamental**, qui positionne la zone par rapport a la
//!   ligne zero (`H` = zone qui part du nominal vers le haut, `h` = vers le bas) ;
//! * le **degre de tolerance** `IT`, qui en fixe la largeur.
//!
//! Ce module ne contient aucune valeur numerique : uniquement la grammaire des
//! designations. Les valeurs viennent de `mecatol-standards`.

use core::fmt;

use serde::{Deserialize, Serialize};

/// L'element tolerance : contenant ou contenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// Alesage : element interieur (le trou).
    Hole,
    /// Arbre : element exterieur.
    Shaft,
}

impl Feature {
    pub const fn label_fr(self) -> &'static str {
        match self {
            Feature::Hole => "alésage",
            Feature::Shaft => "arbre",
        }
    }

    /// Vrai si la designation `s` est ecrite dans la casse de cet element.
    pub const fn is_hole(self) -> bool {
        matches!(self, Feature::Hole)
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label_fr())
    }
}

/// Erreurs de lecture d'une designation de classe de tolerance.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClassError {
    #[error("designation de classe de tolerance vide")]
    Empty,
    #[error("lettre d'ecart fondamental inconnue : {0:?}")]
    UnknownLetter(String),
    #[error("degre de tolerance inconnu : {0:?} (attendu IT01, IT0, puis IT1 a IT18)")]
    UnknownGrade(String),
    #[error(
        "casse ambigue dans {0:?} : une classe s'ecrit soit entierement en majuscules \
         (alesage, par ex. H7) soit entierement en minuscules (arbre, par ex. g6)"
    )]
    AmbiguousCase(String),
    #[error("designation incomplete : {0:?} (attendu une lettre suivie d'un degre, par ex. H7)")]
    Incomplete(String),
}

/// Les 28 lettres d'ecart fondamental definies par l'ISO 286-1.
///
/// L'ordre de declaration est celui de la norme, de l'ecart le plus negatif
/// (`A`, grand jeu) au plus positif (`ZC`, fort serrage), ce qui rend `Ord`
/// directement exploitable pour classer des solutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviationLetter {
    A,
    B,
    C,
    Cd,
    D,
    E,
    Ef,
    F,
    Fg,
    G,
    H,
    Js,
    J,
    K,
    M,
    N,
    P,
    R,
    S,
    T,
    U,
    V,
    X,
    Y,
    Z,
    Za,
    Zb,
    Zc,
}

/// Toutes les lettres, dans l'ordre normatif.
pub const ALL_LETTERS: [DeviationLetter; 28] = {
    use DeviationLetter::*;
    [
        A, B, C, Cd, D, E, Ef, F, Fg, G, H, Js, J, K, M, N, P, R, S, T, U, V, X, Y, Z, Za, Zb, Zc,
    ]
};

impl DeviationLetter {
    /// Forme majuscule canonique (`"H"`, `"JS"`, `"ZC"`).
    pub const fn as_upper(self) -> &'static str {
        use DeviationLetter::*;
        match self {
            A => "A",
            B => "B",
            C => "C",
            Cd => "CD",
            D => "D",
            E => "E",
            Ef => "EF",
            F => "F",
            Fg => "FG",
            G => "G",
            H => "H",
            Js => "JS",
            J => "J",
            K => "K",
            M => "M",
            N => "N",
            P => "P",
            R => "R",
            S => "S",
            T => "T",
            U => "U",
            V => "V",
            X => "X",
            Y => "Y",
            Z => "Z",
            Za => "ZA",
            Zb => "ZB",
            Zc => "ZC",
        }
    }

    /// Forme minuscule canonique (`"h"`, `"js"`, `"zc"`).
    pub const fn as_lower(self) -> &'static str {
        use DeviationLetter::*;
        match self {
            A => "a",
            B => "b",
            C => "c",
            Cd => "cd",
            D => "d",
            E => "e",
            Ef => "ef",
            F => "f",
            Fg => "fg",
            G => "g",
            H => "h",
            Js => "js",
            J => "j",
            K => "k",
            M => "m",
            N => "n",
            P => "p",
            R => "r",
            S => "s",
            T => "t",
            U => "u",
            V => "v",
            X => "x",
            Y => "y",
            Z => "z",
            Za => "za",
            Zb => "zb",
            Zc => "zc",
        }
    }

    /// Ecriture dans la casse correspondant a `feature`.
    pub const fn as_str_for(self, feature: Feature) -> &'static str {
        match feature {
            Feature::Hole => self.as_upper(),
            Feature::Shaft => self.as_lower(),
        }
    }

    /// `JS`/`js` designe une zone symetrique par rapport a la ligne zero.
    pub const fn is_symmetric(self) -> bool {
        matches!(self, DeviationLetter::Js)
    }

    /// Lit une lettre depuis sa forme majuscule ou minuscule.
    pub fn parse(s: &str) -> Result<Self, ClassError> {
        let upper = s.trim().to_ascii_uppercase();
        ALL_LETTERS
            .into_iter()
            .find(|letter| letter.as_upper() == upper)
            .ok_or_else(|| ClassError::UnknownLetter(s.to_string()))
    }
}

impl fmt::Display for DeviationLetter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_upper())
    }
}

/// Noms des degres de tolerance normalises, dans l'ordre croissant de largeur.
const GRADE_NAMES: [&str; 20] = [
    "IT01", "IT0", "IT1", "IT2", "IT3", "IT4", "IT5", "IT6", "IT7", "IT8", "IT9", "IT10", "IT11",
    "IT12", "IT13", "IT14", "IT15", "IT16", "IT17", "IT18",
];

/// Un degre de tolerance normalise, de `IT01` a `IT18`.
///
/// La representation interne est un indice dans [`GRADE_NAMES`], ce qui donne
/// gratuitement l'ordre (`IT6 < IT7`) et le degre precedent, necessaire au
/// calcul de l'ecart `delta` des alesages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Grade(u8);

impl Grade {
    pub const IT01: Grade = Grade(0);
    pub const IT0: Grade = Grade(1);

    /// Construit depuis le numero usuel : `1` -> `IT1`, `18` -> `IT18`.
    ///
    /// `IT01` et `IT0` n'ont pas de numero et doivent passer par les constantes.
    pub const fn from_it_number(n: u8) -> Option<Grade> {
        if n >= 1 && n <= 18 {
            Some(Grade(n + 1))
        } else {
            None
        }
    }

    /// Le numero usuel, ou `None` pour `IT01` et `IT0`.
    pub const fn it_number(self) -> Option<u8> {
        if self.0 >= 2 {
            Some(self.0 - 1)
        } else {
            None
        }
    }

    /// Le nom normalise, par ex. `"IT7"`.
    pub fn name(self) -> &'static str {
        GRADE_NAMES[self.0 as usize]
    }

    /// Le degre immediatement plus fin, ou `None` pour `IT01`.
    ///
    /// Sert a la regle `delta = IT(n) - IT(n-1)` des ecarts d'alesage.
    pub const fn previous(self) -> Option<Grade> {
        if self.0 == 0 {
            None
        } else {
            Some(Grade(self.0 - 1))
        }
    }

    /// Indice interne, exploite par les tables normatives.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Lit `"IT7"`, `"it7"`, `"7"`, `"IT01"` ou `"01"`.
    pub fn parse(s: &str) -> Result<Self, ClassError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ClassError::Empty);
        }
        let digits = trimmed
            .strip_prefix("IT")
            .or_else(|| trimmed.strip_prefix("it"))
            .or_else(|| trimmed.strip_prefix("It"))
            .unwrap_or(trimmed);
        let canonical = format!("IT{digits}");
        GRADE_NAMES
            .iter()
            .position(|name| *name == canonical)
            .map(|i| Grade(i as u8))
            .ok_or_else(|| ClassError::UnknownGrade(s.to_string()))
    }

    /// Tous les degres, du plus fin au plus large.
    pub fn all() -> impl Iterator<Item = Grade> {
        (0..GRADE_NAMES.len() as u8).map(Grade)
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Grade {
    type Error = ClassError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Grade::parse(&value)
    }
}

impl From<Grade> for String {
    fn from(value: Grade) -> String {
        value.name().to_string()
    }
}

/// Une classe de tolerance complete, par ex. `H7` (alesage) ou `g6` (arbre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToleranceClass {
    pub feature: Feature,
    pub letter: DeviationLetter,
    pub grade: Grade,
}

impl ToleranceClass {
    pub const fn new(feature: Feature, letter: DeviationLetter, grade: Grade) -> Self {
        ToleranceClass {
            feature,
            letter,
            grade,
        }
    }

    /// Lit une designation comme `"H7"`, `"g6"`, `"js9"`, `"ZC10"`.
    ///
    /// La casse determine l'element : une designation mixte comme `"Hg7"` ou
    /// `"Js6"` est refusee plutot que devinee.
    ///
    /// ```
    /// # use mecatol_core::{ToleranceClass, Feature, DeviationLetter, Grade};
    /// let c = ToleranceClass::parse("H7").unwrap();
    /// assert_eq!(c.feature, Feature::Hole);
    /// assert_eq!(c.letter, DeviationLetter::H);
    /// assert_eq!(c.grade, Grade::from_it_number(7).unwrap());
    /// assert_eq!(c.to_string(), "H7");
    /// ```
    pub fn parse(input: &str) -> Result<Self, ClassError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ClassError::Empty);
        }

        let split = trimmed
            .find(|c: char| c.is_ascii_digit())
            .ok_or_else(|| ClassError::Incomplete(input.to_string()))?;
        let (letters, digits) = trimmed.split_at(split);
        if letters.is_empty() {
            return Err(ClassError::Incomplete(input.to_string()));
        }

        // "IT7" seul est un degre, pas une classe : le signaler explicitement.
        if letters.eq_ignore_ascii_case("IT") {
            return Err(ClassError::UnknownLetter(letters.to_string()));
        }

        let all_upper = letters.chars().all(|c| c.is_ascii_uppercase());
        let all_lower = letters.chars().all(|c| c.is_ascii_lowercase());
        let feature = match (all_upper, all_lower) {
            (true, false) => Feature::Hole,
            (false, true) => Feature::Shaft,
            _ => return Err(ClassError::AmbiguousCase(input.to_string())),
        };

        Ok(ToleranceClass {
            feature,
            letter: DeviationLetter::parse(letters)?,
            grade: Grade::parse(digits)?,
        })
    }
}

impl fmt::Display for ToleranceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let number = match self.grade.it_number() {
            Some(n) => n.to_string(),
            None => self.grade.name().trim_start_matches("IT").to_string(),
        };
        write!(f, "{}{}", self.letter.as_str_for(self.feature), number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_des_classes_usuelles() {
        let hole = ToleranceClass::parse("H7").unwrap();
        assert_eq!(hole.feature, Feature::Hole);
        assert_eq!(hole.letter, DeviationLetter::H);
        assert_eq!(hole.grade.it_number(), Some(7));

        let shaft = ToleranceClass::parse("g6").unwrap();
        assert_eq!(shaft.feature, Feature::Shaft);
        assert_eq!(shaft.letter, DeviationLetter::G);
        assert_eq!(shaft.grade.it_number(), Some(6));
    }

    #[test]
    fn lettres_a_deux_caracteres() {
        for (text, letter, feature) in [
            ("js9", DeviationLetter::Js, Feature::Shaft),
            ("JS9", DeviationLetter::Js, Feature::Hole),
            ("zc10", DeviationLetter::Zc, Feature::Shaft),
            ("CD6", DeviationLetter::Cd, Feature::Hole),
            ("ef7", DeviationLetter::Ef, Feature::Shaft),
            ("fg5", DeviationLetter::Fg, Feature::Shaft),
        ] {
            let class = ToleranceClass::parse(text).unwrap();
            assert_eq!(class.letter, letter, "lettre de {text}");
            assert_eq!(class.feature, feature, "element de {text}");
            assert_eq!(class.to_string(), text);
        }
    }

    #[test]
    fn aller_retour_texte() {
        for text in ["H7", "g6", "h11", "P9", "js6", "ZC10", "M6", "k6"] {
            assert_eq!(ToleranceClass::parse(text).unwrap().to_string(), text);
        }
    }

    #[test]
    fn la_casse_mixte_est_refusee_et_non_devinee() {
        assert!(matches!(
            ToleranceClass::parse("Js6"),
            Err(ClassError::AmbiguousCase(_))
        ));
        assert!(matches!(
            ToleranceClass::parse("hG6"),
            Err(ClassError::AmbiguousCase(_))
        ));
    }

    #[test]
    fn designations_invalides() {
        assert!(matches!(
            ToleranceClass::parse("Q7"),
            Err(ClassError::UnknownLetter(_))
        ));
        assert!(matches!(
            ToleranceClass::parse("H99"),
            Err(ClassError::UnknownGrade(_))
        ));
        assert!(matches!(
            ToleranceClass::parse("H"),
            Err(ClassError::Incomplete(_))
        ));
        assert!(matches!(
            ToleranceClass::parse("IT7"),
            Err(ClassError::UnknownLetter(_))
        ));
        assert!(matches!(ToleranceClass::parse(""), Err(ClassError::Empty)));
    }

    #[test]
    fn degres_ordonnes_et_predecesseurs() {
        let it6 = Grade::parse("IT6").unwrap();
        let it7 = Grade::parse("7").unwrap();
        assert!(it6 < it7);
        assert_eq!(it7.previous(), Some(it6));
        assert_eq!(Grade::IT0.previous(), Some(Grade::IT01));
        assert_eq!(Grade::IT01.previous(), None);
        assert_eq!(Grade::parse("IT01").unwrap(), Grade::IT01);
        assert_eq!(Grade::parse("0").unwrap(), Grade::IT0);
        assert_eq!(Grade::parse("IT18").unwrap().it_number(), Some(18));
        assert!(Grade::parse("IT19").is_err());
    }

    #[test]
    fn les_lettres_sont_ordonnees_du_jeu_vers_le_serrage() {
        assert!(DeviationLetter::A < DeviationLetter::H);
        assert!(DeviationLetter::H < DeviationLetter::P);
        assert!(DeviationLetter::P < DeviationLetter::Zc);
        assert_eq!(ALL_LETTERS.len(), 28);
    }
}
