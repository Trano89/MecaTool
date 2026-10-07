//! # mecatool-engine
//!
//! Le moteur de calcul de MecaTool : deterministe, sans interface, sans reseau.
//!
//! ## Ce que le moteur garantit
//!
//! * **Il n'invente rien.** Toutes les valeurs normatives viennent de
//!   `mecatool-standards`. Le moteur applique des regles, il ne les complete pas.
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

pub mod bearing;
pub mod chain;
pub mod compare;
pub mod diagram;
pub mod domain;
pub mod error;
pub mod fasteners;
pub mod format;
pub mod geometric;
pub mod iso2768;
pub mod iso286;
pub mod parser;
pub mod requirement;
pub mod search;
pub mod surface;
pub mod welding;

pub use bearing::{BearingEngine, DesignationReading, MountingAdvice, MountingOption};
pub use chain::{analyse_chain, verify_chain, ChainAnalysis, Link, LinkDirection};
pub use compare::{compare_fits, ComparedFit, FitComparison};
pub use diagram::{comparison_diagram, fit_diagram, to_svg, Diagram, DiagramMode, DiagramOptions};
pub use domain::{registry, Domain, DomainGroup, DomainStatus};
pub use error::{EngineError, Result};
pub use fasteners::{FastenerEngine, ThreadReport};
pub use geometric::{
    Finding, GeometricEngine, GeometricSpec, GroupAnalysis, Overlap, Severity, SpecAnalysis,
};
pub use iso2768::{ClassComparison, ClassRow, GeneralAnalysis, Iso2768Engine};
pub use iso286::{FeatureAnalysis, FitAnalysis, Iso286Engine};
pub use parser::{parse, ParsedInput};
pub use requirement::{verify_clearance, ClearanceRequirement, Margins, Verification};
pub use search::{find_fits, Basis, SearchOptions, SearchResult, Solution};
pub use surface::{ProcessFit, Reach, RoughnessChart, SurfaceAnalysis, SurfaceEngine};
pub use welding::{QualityAssessment, WeldReading, WeldRequest, WeldingEngine};
