//! # mecatol-core
//!
//! Types fondamentaux de Mecatol, sans aucune dependance a une norme
//! particuliere ni a une interface.
//!
//! Ce crate repond a trois exigences de l'architecture :
//!
//! 1. **Exactitude.** Toute longueur est un entier de nanometres
//!    ([`Length`]). Le moteur ne manipule jamais de flottant sur le chemin de
//!    calcul dimensionnel.
//! 2. **Structures fortes.** Un ecart, une zone, un ajustement ont chacun leur
//!    type, avec leurs invariants verifies a la construction. Il n'existe pas
//!    d'objet « valeur numerique vague ».
//! 3. **Tracabilite.** Un resultat ne peut pas exister sans sa [`Provenance`],
//!    qui dit de quelle norme, de quelle edition et de quel etat de
//!    verification proviennent ses chiffres.
//!
//! Les valeurs normatives elles-memes vivent dans `mecatol-standards` ; les
//! algorithmes dans `mecatol-engine`.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod conclusion;
pub mod fit;
pub mod iso_class;
pub mod length;
pub mod standard;
pub mod tolerance;

pub use conclusion::{Conclusion, ReasoningStep, Verdict};
pub use fit::{Fit, FitKind};
pub use iso_class::{
    ClassError, DeviationLetter, Feature, Grade, ToleranceClass, ALL_LETTERS,
};
pub use length::{Length, LengthError, Unit, NM_PER_INCH, NM_PER_MM, NM_PER_UM};
pub use standard::{Provenance, StandardReference, VerificationStatus};
pub use tolerance::{
    Deviations, FeatureTolerance, LimitsOfSize, SizeRange, ToleranceError,
};
