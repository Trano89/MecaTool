//! Erreurs du moteur de calcul.

use mecatool_core::{ClassError, LengthError, ToleranceError};
use mecatool_standards::StandardsError;

/// Ce qui peut empecher le moteur de conclure.
///
/// Aucune variante ne correspond a « on a fait au mieux » : le moteur produit
/// un resultat exact ou une erreur nommee, jamais une approximation muette.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Standards(#[from] StandardsError),

    #[error(transparent)]
    Tolerance(#[from] ToleranceError),

    #[error(transparent)]
    Class(#[from] ClassError),

    #[error(transparent)]
    Length(#[from] LengthError),

    #[error(
        "un ajustement associe un alesage et un arbre : {first} et {second} designent \
         deux fois le meme type d'element"
    )]
    NotAFitPair { first: String, second: String },

    #[error(
        "la regle du delta demande le degre immediatement plus fin que {grade}, \
         qui n'existe pas"
    )]
    NoFinerGrade { grade: String },

    #[error("entree illisible : {input:?}. {hint}")]
    Unparsable { input: String, hint: String },

    #[error("entree ambigue : {input:?}. {question}")]
    Ambiguous { input: String, question: String },

    #[error("exigence fonctionnelle incoherente : {detail}")]
    ContradictoryRequirement { detail: String },
}

pub type Result<T> = core::result::Result<T, EngineError>;
