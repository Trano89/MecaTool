//! # mecatool-standards
//!
//! Les donnees normatives de MecaTool, et rien d'autre.
//!
//! ## Principes
//!
//! **Les donnees sont separees du code.** Elles vivent dans `/data`, en JSON
//! versionne par norme et par edition. Le code de ce crate sait les lire, les
//! valider et les consulter ; il n'en contient aucune valeur en dur.
//!
//! **Les donnees sont embarquees a la compilation.** Un calcul ne depend ni du
//! reseau ni de la presence d'un fichier a cote du binaire.
//!
//! **Une table fausse doit etre detectable.** Chaque jeu de donnees est valide
//! au chargement : echelons contigus, lignes completes, monotonie. Les tests
//! confrontent en plus les valeurs a la formule normative pour reperer les
//! aberrations.
//!
//! **Rien n'est presente comme verifie sans l'etre.** Chaque jeu porte un
//! [`mecatool_core::VerificationStatus`] ; tant qu'il vaut `Unverified`, tout
//! resultat qui en decoule doit etre affiche avec la mention correspondante.
//!
//! ## Editions
//!
//! Un nouveau millesime de norme donne un nouveau fichier, jamais une
//! modification en place : un calcul archive doit rester reproductible avec les
//! donnees qui l'ont produit.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod error;
pub mod iso1101;
pub mod iso2768;
pub mod iso286;
pub mod roulements;
pub mod value;

pub use error::{Result, StandardsError};
pub use iso1101::{
    Characteristic, CharacteristicTable, DatumRule, FamilyDefinition, Modifier, ToleranceFamily,
    ZoneDefinition, ZoneGeometry,
};
pub use iso2768::{
    GeneralClass, GeneralDeviation, GeneralLookup, GeneralRange, GeneralToleranceTable, MeasureKind,
};
pub use iso286::{DeviationSide, ItGradeTable, ItValue, ShaftDeviation, ShaftDeviationTable};
pub use roulements::{
    BearingFamily, BoreDesignation, DiameterRange, LoadRegime, MountingCase, MountingRow,
    ShaftMountingTable,
};
