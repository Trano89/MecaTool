//! Erreurs du chargement et de la consultation des donnees normatives.

use mecatol_core::{ClassError, LengthError};

/// Ce qui peut mal tourner entre un fichier de donnees et une valeur exploitable.
///
/// Chaque variante nomme le jeu de donnees fautif : quand une table normative
/// est erronee, le message doit permettre de retrouver la case a corriger.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StandardsError {
    #[error("donnee normative illisible dans {dataset} : {detail}")]
    Malformed { dataset: String, detail: String },

    #[error(
        "dimension nominale {nominal} hors de la plage couverte par {dataset} \
         (de {min} exclu a {max} inclus)"
    )]
    NominalOutOfRange {
        dataset: String,
        nominal: String,
        min: String,
        max: String,
    },

    #[error("le degre {grade} ne figure pas dans {dataset}")]
    GradeUnavailable { dataset: String, grade: String },

    #[error("la lettre d'ecart fondamental {letter} ne figure pas dans {dataset}")]
    LetterUnavailable { dataset: String, letter: String },

    #[error("classe de tolérance générale inconnue : {given:?} (attendu f, m, c ou v)")]
    UnknownGeneralClass { given: String },

    #[error(
        "la cote nominale {nominal} est inférieure à {minimum} : l'ISO 2768-1 impose \
         d'indiquer les écarts limites directement à côté de la cote nominale"
    )]
    BelowGeneralMinimum { nominal: String, minimum: String },

    #[error("l'ISO 2768-1 ne définit pas la classe {class} pour une {kind} de l'échelon {range}")]
    GeneralNotDefined {
        class: String,
        kind: String,
        range: String,
    },

    #[error("{grade} ne s'applique pas ici : {reason}")]
    GradeNotApplicable { grade: String, reason: String },

    #[error("table incoherente dans {dataset} : {detail}")]
    Inconsistent { dataset: String, detail: String },

    #[error(transparent)]
    Length(#[from] LengthError),

    #[error(transparent)]
    Class(#[from] ClassError),
}

pub type Result<T> = core::result::Result<T, StandardsError>;
