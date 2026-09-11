//! Tracabilite normative.
//!
//! Regle absolue de MecaTool : aucune valeur presentee comme normative ne peut
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
///
/// # Pourquoi trois etats et non deux
///
/// Entre « confronte a la norme » et « pas encore verifie » il existe un cas
/// intermediaire courant : la donnee vient d'un recueil technique qui reproduit
/// la norme, consulte avec soin, mais qui n'est pas la norme. Le confondre avec
/// `Verified` reviendrait a citer une norme qu'on n'a pas lue ; le confondre
/// avec `Unverified` reviendrait a dire qu'on n'a rien verifie. `Secondary`
/// nomme exactement ce qui a ete fait, et compte comme non verifie pour
/// l'affichage : l'utilisateur doit savoir sur quoi il s'appuie.
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
    /// Valeurs transcrites d'un recueil reproduisant la norme, pas de la norme.
    Secondary {
        /// Le recueil consulte, avec la page, par ex. `"VSM 2022, page 175"`.
        from: String,
        /// Ce que ce recueil declare reproduire, par ex. `"SN EN ISO 1101"`.
        reproduces: String,
        /// Date de la transcription, au format `AAAA-MM-JJ`.
        on: String,
    },
    /// Valeurs saisies mais pas encore confrontees a la source.
    Unverified {
        /// Ce qu'il reste a faire pour lever le doute.
        pending: String,
    },
}

impl VerificationStatus {
    /// Vrai seulement si la donnee a ete confrontee a la norme elle-meme.
    pub const fn is_verified(&self) -> bool {
        matches!(self, VerificationStatus::Verified { .. })
    }

    /// Vrai si la donnee vient d'un recueil reproduisant la norme.
    pub const fn is_secondary(&self) -> bool {
        matches!(self, VerificationStatus::Secondary { .. })
    }

    /// Message court destine a l'interface.
    pub fn banner_fr(&self) -> Option<String> {
        match self {
            VerificationStatus::Verified { .. } => None,
            VerificationStatus::Secondary {
                from, reproduces, ..
            } => Some(format!(
                "Donnée transcrite d'un recueil technique, non confrontée à la \
                 norme elle-même. Source : {from}, qui reproduit {reproduces}."
            )),
            VerificationStatus::Unverified { pending } => {
                Some(format!("Donnée normative non vérifiée. {pending}"))
            }
        }
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Verified { against, on } => {
                write!(f, "vérifiée le {on} contre {against}")
            }
            VerificationStatus::Secondary { from, on, .. } => {
                write!(f, "transcrite le {on} depuis {from}")
            }
            VerificationStatus::Unverified { .. } => f.write_str("non vérifiée"),
        }
    }
}

/// Reference a une norme, telle que citee dans un resultat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandardReference {
    /// Designation, par ex. `"ISO 286-1"`.
    pub id: String,
    /// Edition, par ex. `"2010"`.
    ///
    /// Vide quand le millesime n'est pas connu : une source secondaire cite
    /// parfois une norme sans son annee. Inventer ce millesime serait inventer
    /// une reference normative, donc il reste vide et la citation l'omet.
    pub edition: String,
    /// Titre officiel.
    pub title: String,
    /// Ce que ce jeu de donnees couvre precisement dans la norme.
    ///
    /// Une meme norme fournit plusieurs tables ; sans cette precision, deux
    /// references citees cote a cote seraient indiscernables a l'affichage.
    #[serde(default)]
    pub scope: Option<String>,
    /// Origine des valeurs saisies dans MecaTool.
    pub source: String,
    /// Etat de verification du jeu de donnees.
    pub verification: VerificationStatus,
    /// Remarques utiles a la relecture (plages non couvertes, cas particuliers).
    #[serde(default)]
    pub notes: Vec<String>,
}

impl StandardReference {
    /// Citation courte : `"ISO 286-1:2010"`, ou `"ISO 1101"` sans millesime.
    pub fn citation(&self) -> String {
        if self.edition.is_empty() {
            self.id.clone()
        } else {
            format!("{}:{}", self.id, self.edition)
        }
    }

    /// Citation suivie du perimetre, quand il est connu.
    ///
    /// `"ISO 286-1:2010 (degres de tolerance normalises IT)"`.
    pub fn labelled_citation(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{} ({scope})", self.citation()),
            None => self.citation(),
        }
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
                    .map(|banner| format!("{} : {banner}", r.labelled_citation()))
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
            scope: None,
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
            scope: None,
            source: "source".into(),
            verification: VerificationStatus::Unverified {
                pending: "a confronter au scan".into(),
            },
            notes: vec![],
        }
    }

    fn secondary(id: &str) -> StandardReference {
        StandardReference {
            id: id.to_string(),
            edition: String::new(),
            title: "titre".into(),
            scope: None,
            source: "source".into(),
            verification: VerificationStatus::Secondary {
                from: "VSM 2022, page 175".into(),
                reproduces: "SN EN ISO 1101".into(),
                on: "2026-09-11".into(),
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
        let p = Provenance::new()
            .with(verified("ISO 286-1"))
            .with(unverified("ISO 286-2"));
        assert!(!p.is_fully_verified());
        assert_eq!(p.warnings_fr().len(), 1);
        assert!(p.warnings_fr()[0].contains("ISO 286-2:2010"));
    }

    #[test]
    fn toutes_verifiees_donne_un_resultat_verifie_sans_avertissement() {
        let p = Provenance::new()
            .with(verified("ISO 286-1"))
            .with(verified("ISO 286-2"));
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

    #[test]
    fn une_source_secondaire_ne_compte_pas_comme_verifiee() {
        let p = Provenance::new().with(secondary("ISO 1101"));
        assert!(!p.is_fully_verified());
    }

    #[test]
    fn une_source_secondaire_dit_de_quel_recueil_elle_vient() {
        let p = Provenance::new().with(secondary("ISO 1101"));
        let warnings = p.warnings_fr();
        assert_eq!(warnings.len(), 1);

        // Le message doit nommer le recueil ET la norme reproduite : citer l'une
        // sans l'autre laisserait croire soit qu'on a lu la norme, soit qu'on ne
        // sait pas de quoi on parle.
        assert!(warnings[0].contains("VSM 2022, page 175"));
        assert!(warnings[0].contains("SN EN ISO 1101"));

        // Et il ne doit pas se confondre avec le message des donnees non saisies.
        assert!(!warnings[0].contains("non vérifiée"));
    }

    #[test]
    fn une_citation_sans_millesime_nen_invente_pas() {
        // Le deux-points pendant de « ISO 1101: » se lirait comme un millesime
        // manquant par erreur plutot que comme un millesime inconnu.
        assert_eq!(secondary("ISO 1101").citation(), "ISO 1101");
    }
}
