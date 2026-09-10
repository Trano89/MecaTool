//! # mecatol-engine
//!
//! Le moteur de calcul de Mecatol : deterministe, sans interface, sans reseau.
//!
//! ## Ce que le moteur garantit
//!
//! * **Il n'invente rien.** Toutes les valeurs normatives viennent de
//!   `mecatol-standards`. Le moteur applique des regles, il ne les complete pas.
//! * **Il refuse plutot que d'approximer.** Hors plage, lettre absente,
//!   combinaison invalide : une erreur nommee, jamais un resultat plausible.
//! * **Il montre son travail.** Chaque resultat porte la suite des etapes qui
//!   l'ont produit, pour le mode expert et pour l'explication du « pourquoi ».
//! * **Il dit d'ou viennent ses chiffres.** Chaque resultat porte sa
//!   `Provenance`, avertissements de non-verification compris.
//!
//! ## Organisation
//!
//! * [`iso286`] : tolerances d'un element et ajustements ;
//! * [`error`] : les manieres nommees d'echouer ;
//! * [`format`] : mise en forme, en bout de chaine uniquement.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod error;
pub mod format;
pub mod iso286;
pub mod parser;
pub mod requirement;
pub mod search;

pub use error::{EngineError, Result};
pub use iso286::{FeatureAnalysis, FitAnalysis, Iso286Engine};
pub use parser::{parse, ParsedInput};
pub use requirement::{verify_clearance, ClearanceRequirement, Margins, Verification};
pub use search::{find_fits, Basis, SearchOptions, SearchResult, Solution};
