//! Lecture des designations ecrites a la main.
//!
//! Accepte les formes usuelles d'atelier et de bureau d'etudes :
//!
//! ```text
//!   Ou10 H7/g6      Ou 10 H7 g6      10H7/g6
//!   20 H7           Ou25,4 h6        1 in H7/g6
//! ```
//!
//! Regle de conduite : en cas d'ambiguite, poser une question plutot que
//! deviner. Un parser silencieusement indulgent sur une cote est un parser qui
//! fabrique des pieces fausses.

use core::str::FromStr;

use mecatol_core::{Feature, Length, ToleranceClass, Unit};

use crate::error::{EngineError, Result};

/// Ce qu'une entree utilisateur peut designer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedInput {
    /// Un ajustement : un alesage et un arbre.
    Fit {
        nominal: Length,
        unit: Unit,
        hole: ToleranceClass,
        shaft: ToleranceClass,
    },
    /// Un seul element tolerance.
    Feature {
        nominal: Length,
        unit: Unit,
        class: ToleranceClass,
    },
    /// Une dimension nominale seule, sans classe.
    ///
    /// Insuffisant pour calculer, mais suffisant pour lancer une recherche de
    /// solutions a partir d'un besoin fonctionnel. C'est a l'appelant de dire si
    /// cela lui suffit ; le parser se contente de rapporter ce qu'il a lu.
    NominalOnly { nominal: Length, unit: Unit },
}

impl ParsedInput {
    pub fn nominal(&self) -> Length {
        match self {
            ParsedInput::Fit { nominal, .. }
            | ParsedInput::Feature { nominal, .. }
            | ParsedInput::NominalOnly { nominal, .. } => *nominal,
        }
    }

    pub fn unit(&self) -> Unit {
        match self {
            ParsedInput::Fit { unit, .. }
            | ParsedInput::Feature { unit, .. }
            | ParsedInput::NominalOnly { unit, .. } => *unit,
        }
    }
}

/// Lit une fenetre de jeu ecrite `"MIN..MAX"`.
///
/// Les valeurs sont en micrometres par defaut ; une unite peut suivre et
/// s'applique aux deux bornes. Une borne vide signifie « non exigee ».
///
/// ```
/// # use mecatol_engine::parser::parse_clearance_window;
/// # use mecatol_core::Length;
/// let (min, max) = parse_clearance_window("10..30").unwrap();
/// assert_eq!(min, Some(Length::from_micrometres(10)));
/// assert_eq!(max, Some(Length::from_micrometres(30)));
///
/// // Une seule borne suffit.
/// let (min, max) = parse_clearance_window("10..").unwrap();
/// assert_eq!(max, None);
/// assert_eq!(min, Some(Length::from_micrometres(10)));
/// ```
pub fn parse_clearance_window(input: &str) -> Result<(Option<Length>, Option<Length>)> {
    let trimmed = input.trim();
    let (body, unit) = split_trailing_unit(trimmed);

    let Some((low, high)) = body.split_once("..") else {
        return Err(EngineError::Unparsable {
            input: input.to_string(),
            hint: "Une fenêtre de jeu s'écrit \"MIN..MAX\", par ex. \"10..30\" (en \u{b5}m) \
                   ou \"0.01..0.03 mm\"."
                .to_string(),
        });
    };

    let bound = |text: &str| -> Result<Option<Length>> {
        let text = text.trim();
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Length::parse(text, unit)?))
        }
    };

    let (min, max) = (bound(low)?, bound(high)?);
    if min.is_none() && max.is_none() {
        return Err(EngineError::Unparsable {
            input: input.to_string(),
            hint: "Indiquez au moins une des deux bornes.".to_string(),
        });
    }
    Ok((min, max))
}

/// Detache une unite ecrite en fin de chaine ; le micrometre par defaut.
fn split_trailing_unit(text: &str) -> (&str, Unit) {
    for (suffix, unit) in [
        ("\u{b5}m", Unit::Micrometre),
        ("um", Unit::Micrometre),
        ("mm", Unit::Millimetre),
        ("inch", Unit::Inch),
        ("in", Unit::Inch),
    ] {
        if let Some(head) = text.strip_suffix(suffix) {
            return (head.trim_end(), unit);
        }
    }
    (text, Unit::Micrometre)
}

/// Caracteres qui marquent un diametre ou separent deux classes.
fn is_separator(c: char) -> bool {
    matches!(
        c,
        '\u{d8}'      // Ø
        | '\u{f8}'    // ø
        | '\u{2300}'  // ⌀
        | '/'
        | ':'
        | '\u{d7}' // ×
    )
}

/// Lit une designation complete.
///
/// ```
/// # use mecatol_engine::parser::{parse, ParsedInput};
/// # use mecatol_core::{Length, Unit};
/// let parsed = parse("\u{d8}10 H7/g6").unwrap();
/// match parsed {
///     ParsedInput::Fit { nominal, hole, shaft, .. } => {
///         assert_eq!(nominal, Length::from_millimetres(10));
///         assert_eq!(hole.to_string(), "H7");
///         assert_eq!(shaft.to_string(), "g6");
///     }
///     _ => panic!("un ajustement etait attendu"),
/// }
/// ```
pub fn parse(input: &str) -> Result<ParsedInput> {
    let cleaned: String = input
        .chars()
        .map(|c| if is_separator(c) { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();

    if trimmed.is_empty() {
        return Err(EngineError::Unparsable {
            input: input.to_string(),
            hint: "Indiquez une dimension nominale et au moins une classe, par ex. \"20 H7/g6\"."
                .to_string(),
        });
    }

    let (number, rest) = split_leading_number(trimmed).ok_or_else(|| EngineError::Unparsable {
        input: input.to_string(),
        hint: "La désignation doit commencer par la dimension nominale, par ex. \"20 H7/g6\"."
            .to_string(),
    })?;

    let mut tokens = rest.split_whitespace().peekable();

    // Une unite peut suivre immediatement le nombre ; sinon on reste en millimetres.
    let unit = match tokens.peek().and_then(|t| Unit::from_str(t).ok()) {
        Some(u) => {
            tokens.next();
            u
        }
        None => Unit::Millimetre,
    };

    let nominal = Length::parse(number, unit)?;
    if !nominal.is_positive() {
        return Err(EngineError::Unparsable {
            input: input.to_string(),
            hint: "La dimension nominale doit être strictement positive.".to_string(),
        });
    }

    let classes: Vec<&str> = tokens.collect();
    match classes.as_slice() {
        // Une dimension seule n'est pas une erreur : c'est le point de depart
        // d'une recherche de solutions. L'appelant tranche.
        [] => Ok(ParsedInput::NominalOnly { nominal, unit }),

        [single] => Ok(ParsedInput::Feature {
            nominal,
            unit,
            class: ToleranceClass::parse(single)?,
        }),

        [first, second] => {
            let a = ToleranceClass::parse(first)?;
            let b = ToleranceClass::parse(second)?;
            match (a.feature, b.feature) {
                (Feature::Hole, Feature::Shaft) => Ok(ParsedInput::Fit {
                    nominal,
                    unit,
                    hole: a,
                    shaft: b,
                }),
                // L'ordre usuel est alesage/arbre, mais la casse leve toute
                // ambiguite : on accepte l'ordre inverse sans le deviner.
                (Feature::Shaft, Feature::Hole) => Ok(ParsedInput::Fit {
                    nominal,
                    unit,
                    hole: b,
                    shaft: a,
                }),
                (Feature::Hole, Feature::Hole) => Err(EngineError::NotAFitPair {
                    first: a.to_string(),
                    second: b.to_string(),
                }),
                (Feature::Shaft, Feature::Shaft) => Err(EngineError::NotAFitPair {
                    first: a.to_string(),
                    second: b.to_string(),
                }),
            }
        }

        more => Err(EngineError::Ambiguous {
            input: input.to_string(),
            question: format!(
                "{} classes de tolérance ont été lues ({}). Un ajustement en associe exactement \
                 deux, un élément une seule. Laquelle faut-il retenir ?",
                more.len(),
                more.join(", ")
            ),
        }),
    }
}

/// Detache le nombre de tete du reste de la chaine.
fn split_leading_number(text: &str) -> Option<(&str, &str)> {
    let mut end = 0;
    for (i, c) in text.char_indices() {
        let acceptable = c.is_ascii_digit() || c == '.' || c == ',' || (i == 0 && c == '+');
        if acceptable {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 || !text[..end].chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((&text[..end], &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fit_of(input: &str) -> (Length, Unit, String, String) {
        match parse(input).unwrap() {
            ParsedInput::Fit {
                nominal,
                unit,
                hole,
                shaft,
            } => (nominal, unit, hole.to_string(), shaft.to_string()),
            other => panic!("un ajustement etait attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn les_ecritures_usuelles_dun_ajustement_sont_equivalentes() {
        let reference = fit_of("\u{d8}10 H7/g6");
        for variant in [
            "\u{d8}10 H7/g6",
            "\u{d8} 10 H7 g6",
            "10 H7/g6",
            "10H7/g6",
            "  10   H7  /  g6  ",
            "\u{2300}10H7/g6",
            "10 mm H7/g6",
        ] {
            assert_eq!(fit_of(variant), reference, "variante {variant:?}");
        }
        assert_eq!(reference.0, Length::from_millimetres(10));
        assert_eq!(reference.1, Unit::Millimetre);
        assert_eq!(reference.2, "H7");
        assert_eq!(reference.3, "g6");
    }

    #[test]
    fn la_virgule_decimale_est_acceptee() {
        let (nominal, ..) = fit_of("\u{d8}25,4 H7/g6");
        assert_eq!(nominal, Length::parse("25.4", Unit::Millimetre).unwrap());
    }

    #[test]
    fn une_unite_explicite_est_respectee() {
        let (nominal, unit, ..) = fit_of("1 in H7/g6");
        assert_eq!(unit, Unit::Inch);
        assert_eq!(nominal, Length::from_nanometres(25_400_000));
    }

    #[test]
    fn un_element_seul_est_reconnu() {
        match parse("20 H7").unwrap() {
            ParsedInput::Feature { nominal, class, .. } => {
                assert_eq!(nominal, Length::from_millimetres(20));
                assert_eq!(class.to_string(), "H7");
            }
            other => panic!("un element seul etait attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn lordre_inverse_est_accepte_car_la_casse_leve_lambiguite() {
        // g6/H7 designe le meme ajustement que H7/g6 : la casse dit qui est qui.
        assert_eq!(fit_of("10 g6/H7"), fit_of("10 H7/g6"));
    }

    #[test]
    fn deux_elements_de_meme_nature_sont_refuses() {
        assert!(matches!(
            parse("10 H7/G6"),
            Err(EngineError::NotAFitPair { .. })
        ));
        assert!(matches!(
            parse("10 h7/g6"),
            Err(EngineError::NotAFitPair { .. })
        ));
    }

    #[test]
    fn une_entree_ambigue_pose_une_question_au_lieu_de_deviner() {
        let err = parse("10 H7 g6 k6").unwrap_err();
        match err {
            EngineError::Ambiguous { question, .. } => {
                assert!(question.contains("Laquelle"), "question : {question}");
            }
            other => panic!("une question etait attendue, obtenu {other}"),
        }
    }

    #[test]
    fn entrees_illisibles() {
        assert!(matches!(parse(""), Err(EngineError::Unparsable { .. })));
        assert!(matches!(
            parse("H7/g6"),
            Err(EngineError::Unparsable { .. })
        ));
        assert!(matches!(parse("   "), Err(EngineError::Unparsable { .. })));
    }

    #[test]
    fn une_dimension_seule_est_un_point_de_depart_valide() {
        for text in ["10", "\u{d8}10", "\u{d8} 10 mm"] {
            match parse(text).unwrap() {
                ParsedInput::NominalOnly { nominal, .. } => {
                    assert_eq!(nominal, Length::from_millimetres(10), "entree {text:?}");
                }
                other => panic!("{text:?} : attendu une dimension seule, obtenu {other:?}"),
            }
        }
    }

    #[test]
    fn fenetres_de_jeu() {
        let um = Length::from_micrometres;
        assert_eq!(
            parse_clearance_window("10..30").unwrap(),
            (Some(um(10)), Some(um(30)))
        );
        assert_eq!(
            parse_clearance_window("10..").unwrap(),
            (Some(um(10)), None)
        );
        assert_eq!(
            parse_clearance_window("..30").unwrap(),
            (None, Some(um(30)))
        );
        // L'unite s'applique aux deux bornes.
        assert_eq!(
            parse_clearance_window("0.01..0.03 mm").unwrap(),
            (Some(um(10)), Some(um(30)))
        );
        assert_eq!(
            parse_clearance_window("10..30 \u{b5}m").unwrap(),
            (Some(um(10)), Some(um(30)))
        );
        // Un serrage s'exprime comme un jeu negatif.
        assert_eq!(
            parse_clearance_window("-20..-5").unwrap(),
            (Some(um(-20)), Some(um(-5)))
        );
    }

    #[test]
    fn fenetres_de_jeu_malformees() {
        assert!(matches!(
            parse_clearance_window("10"),
            Err(EngineError::Unparsable { .. })
        ));
        assert!(matches!(
            parse_clearance_window(".."),
            Err(EngineError::Unparsable { .. })
        ));
        assert!(matches!(
            parse_clearance_window("dix..trente"),
            Err(EngineError::Length(_))
        ));
    }

    #[test]
    fn une_classe_inexistante_remonte_lerreur_de_classe() {
        assert!(matches!(parse("10 Q7/g6"), Err(EngineError::Class(_))));
        assert!(matches!(parse("10 H99/g6"), Err(EngineError::Class(_))));
        // La casse mixte reste refusee jusqu'ici.
        assert!(matches!(parse("10 Js7"), Err(EngineError::Class(_))));
    }

    #[test]
    fn un_nominal_nul_est_refuse() {
        assert!(matches!(
            parse("0 H7/g6"),
            Err(EngineError::Unparsable { .. })
        ));
    }
}
