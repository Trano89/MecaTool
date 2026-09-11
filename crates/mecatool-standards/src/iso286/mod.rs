//! Donnees normatives de l'ISO 286 (systeme de codification des tolerances).
//!
//! Organisation :
//!
//! * [`it_grades`] : les degres de tolerance normalises `IT`, qui donnent la
//!   **largeur** d'une zone de tolerance ;
//! * [`deviations`] : les ecarts fondamentaux, qui en donnent la **position**,
//!   ainsi que la regle de derivation des alesages ;
//! * [`formula`] : controle de vraisemblance, sans valeur normative.

pub mod deviations;
pub mod formula;
pub mod it_grades;

pub use deviations::{DeviationSide, ShaftDeviation, ShaftDeviationTable};
pub use it_grades::{ItGradeTable, ItValue};
