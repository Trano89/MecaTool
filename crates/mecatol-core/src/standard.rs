//! Tracabilite normative.
//!
//! Regle absolue de Mecatol : aucune valeur presentee comme normative ne peut
//! l'etre sans reference verifiable. Ce module porte cette regle dans le systeme
//! de types, de sorte qu'un resultat ne peut pas etre construit sans dire d'ou
//! viennent ses chiffres.

use core::fmt;

use serde::{Deserialize, Serialize};

/// Etat de verification d'un jeu de donnees normatives.
///
/// Un resultat calcule a partir de donnees `Unverified` doit etre presente a
/// l'utilisateur avec un avertissement visible : c'est la traduction directe de
/// l'interdiction de faire passer une approximation pour une valeur ISO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Chaque valeur a ete confrontee a la source citee.
    Verified {
        /// Document ayant servi au controle, par ex. `"ISO 286-1:2010, tableau 1"`.
        against: String,
        /// Date du controle, au format `AAAA-MM-JJ`.
        on: String,
    },
    /// Valeurs saisies mais pas encore confrontees a la source.
    Unverified {
        /// Ce qu'il reste a faire pour lever le doute.
        pending: String,
    },
}

impl VerificationStatus {
    pub const fn is_verified(&self) -> bool {
        matches!(self, VerificationStatus::Verified { .. })
    }

    /// Message court destine a l'interface.
    pub fn banner_fr(&self) -> Option<String> {
        match self {
            VerificationStatus::Verified { .. } => None,
            VerificationStatus::Unverified { pending } => Some(format!(
                "Donnee normative non verifiee. {pending}"
            )),
        }
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Verified { against, on } => {
                write!(f, "verifiee le {on} contre {against}")
            }
            VerificationStatus::Unverified { .. } => f.write_str("non verifiee"),
        }
    }
}

/// Reference a une norme, telle que citee dans un resultat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandardReference {
    /// Designation, par ex. `"ISO 286-1"`.
    pub id: String,
    /// Edition, par ex. `"2010"`.
    pub edition: String,
    /// Titre officiel.
    pub title: String,
    /// Origine des valeurs saisies dans Mecatol.
    pub source: String,
    /// Etat de verification du jeu de donnees.
    pub verification: VerificationStatus,
    /// Remarques utiles a la relecture (plages non couvertes, cas particuliers).
    #[serde(default)]
    pub notes: Vec<String>,
}

impl StandardReference {
    /// Citation courte : `"ISO 286-1:2010"`.
    pub fn citation(&self) -> String {
        format!("{}:{}", self.id, self.edition)
    }
}

impl fmt::Display for StandardReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} \u{2014} {}", self.citation(), self.title)
    }
}

/// Ensemble des references mobilisees par un calcul.
///
/// Un calcul peut croiser plusieurs jeux de donnees (les degres IT et les ecarts
/// fondamentaux, par exemple). Le resultat est considere comme verifie seulement
/// si toutes ses sources le sont.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub references: Vec<StandardReference>,
}

impl Provenance {
    pub fn new() -> Self {
        Provenance::default()
    }

    pub fn with(mut self, reference: StandardReference) -> Self {
        self.push(reference);
        self
    }

    pub fn push(&mut self, reference: StandardReference) {
        if !self.references.contains(&reference) {
            self.references.push(reference);
        }
    }

    pub fn merge(&mut self, other: &Provenance) {
        for reference in &other.references {
            self.push(reference.clone());
        }
    }

    /// Vrai seulement si toutes les sources sont verifiees.
    pub fn is_fully_verified(&self) -> bool {
        !self.references.is_empty() && self.references.iter().all(|r| r.verification.is_verified())
    }

    /// Avertissements a afficher, un par source non verifiee.
    pub fn warnings_fr(&self) -> Vec<String> {
        self.references
            .iter()
            .filter_map(|r| {
                r.verification
                    .banner_fr()
                    .map(|banner| format!("{} : {banner}", r.citation()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verified(id: &str) -> StandardReference {
        StandardReference {
            id: id.to_string(),
            edition: "2010".into(),
            title: "titre".into(),
            source: "source".into(),
            verification: VerificationStatus::Verified {
                against: "scan".into(),
                on: "2026-09-10".into(),
            },
            notes: vec![],
        }
    }

    fn unverified(id: &str) -> StandardReference {
        StandardReference {
            id: id.to_string(),
            edition: "2010".into(),
            title: "titre".into(),
            source: "source".into(),
            verification: VerificationStatus::Unverified {
                pending: "a confronter au scan".into(),
            },
            notes: vec![],
        }
    }

    #[test]
    fn une_provenance_vide_nest_pas_verifiee() {
        assert!(!Provenance::new().is_fully_verified());
    }

    #[test]
    fn une_seule_source_non_verifiee_contamine_le_resultat() {
        let p = Provenance::new().with(verified("ISO 286-1")).with(unverified("ISO 286-2"));
        assert!(!p.is_fully_verified());
        assert_eq!(p.warnings_fr().len(), 1);
        assert!(p.warnings_fr()[0].contains("ISO 286-2:2010"));
    }

    #[test]
    fn toutes_verifiees_donne_un_resultat_verifie_sans_avertissement() {
        let p = Provenance::new().with(verified("ISO 286-1")).with(verified("ISO 286-2"));
        assert!(p.is_fully_verified());
        assert!(p.warnings_fr().is_empty());
    }

    #[test]
    fn les_doublons_sont_ecartes() {
        let mut p = Provenance::new();
        p.push(verified("ISO 286-1"));
        p.push(verified("ISO 286-1"));
        assert_eq!(p.references.len(), 1);
    }

    #[test]
    fn citation_lisible() {
        assert_eq!(verified("ISO 286-1").citation(), "ISO 286-1:2010");
    }
}
