//! Etats de surface : indication, classes N, rugosite par procede.
//!
//! # Une seule generation
//!
//! Le domaine est reste bloque tant que la seule source disponible existait en
//! deux editions incompatibles : l'une suit l'ISO 21920, l'autre l'ISO 4287 et
//! l'ISO 1302. Melanger deux generations de parametres produirait un module faux
//! d'une facon difficile a reperer.
//!
//! Le blocage se leve en **retrecissant** le perimetre plutot qu'en tranchant :
//! ce module ne porte que ce que les deux generations ont en commun — les trois
//! variantes du symbole, les sept sens des stries, la signification des
//! parametres d'amplitude. Ce qui les distingue (longueurs de base par defaut,
//! regle d'acceptation par defaut, bandes de transmission) n'est pas embarque,
//! et le moteur le dit au lieu de le deviner.
//!
//! # Trois jeux, trois natures
//!
//! * l'**indication** (symboles, stries, parametres) releve de l'ISO 21920-1 ;
//! * les **classes N** viennent de l'ISO 1302:1992, retiree — on les lit parce
//!   qu'elles abondent sur les plans existants, jamais pour en ecrire ;
//! * la **rugosite par procede** ne vient d'aucune norme : ce sont des ordres de
//!   grandeur d'atelier.
//!
//! Les trois sont saisis sans document source ouvert, et portent donc
//! [`VerificationStatus::Unverified`] : l'interface le signale avant toute
//! saisie.
//!
//! [`VerificationStatus::Unverified`]: mecatool_core::VerificationStatus::Unverified

use std::collections::BTreeSet;
use std::sync::OnceLock;

use mecatool_core::{Length, StandardReference, Unit};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Result, StandardsError};
use crate::value::length_from_json;

/* ------------------------------------------------------------------ */
/*  Indication — ISO 21920-1                                           */
/* ------------------------------------------------------------------ */

/// L'exigence portee par la variante du symbole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessRequirement {
    /// Tout procede admis.
    Any,
    /// Enlevement de matiere exige.
    RemovalRequired,
    /// Enlevement de matiere interdit.
    RemovalProhibited,
}

/// Une variante du symbole graphique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolVariant {
    pub id: ProcessRequirement,
    /// L'ecriture en texte, faute de glyphe : `APA`, `MRR`, `NMR`.
    pub code: String,
    pub name: String,
    pub meaning: String,
}

/// Un symbole de sens des stries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaySymbol {
    pub symbol: String,
    /// Ecritures de remplacement, en minuscules.
    pub aliases: Vec<String>,
    pub name: String,
    pub meaning: String,
}

/// Un parametre du profil de rugosite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileParameter {
    /// `Ra`, `Rz`... avec sa casse : c'est ainsi qu'il s'ecrit sur un plan.
    pub symbol: String,
    pub name: String,
    pub definition: String,
    /// Une mise en garde propre a ce parametre, le cas echeant.
    #[serde(default)]
    pub note: Option<String>,
}

/// Le vocabulaire de l'indication des etats de surface.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct IndicationTable {
    dataset: String,
    standard: StandardReference,
    symbols: Vec<SymbolVariant>,
    lays: Vec<LaySymbol>,
    parameters: Vec<ProfileParameter>,
}

const INDICATION_EMBEDDED: &str = include_str!("../../../data/surface/iso21920.indication.json");

impl IndicationTable {
    pub fn embedded() -> Result<&'static IndicationTable> {
        static CACHE: OnceLock<core::result::Result<IndicationTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| IndicationTable::parse(INDICATION_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<IndicationTable> {
        let table: IndicationTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso21920.indication".into(),
                detail: source.to_string(),
            })?;
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        // Les trois variantes, ni plus ni moins : une quatrieme serait une
        // invention, une manquante rendrait une exigence illisible.
        let ids: BTreeSet<_> = self.symbols.iter().map(|s| s.id as u8).collect();
        if ids.len() != 3 || self.symbols.len() != 3 {
            return Err(bad("il faut exactement trois variantes du symbole".into()));
        }

        // Un alias doit designer un seul sens des stries, et ne pas se
        // confondre avec un parametre : « R » lu comme une strie radiale ne
        // doit pas pouvoir etre le debut de « Ra ».
        let mut seen = BTreeSet::new();
        for lay in &self.lays {
            for spelling in std::iter::once(lay.symbol.to_lowercase()).chain(lay.aliases.clone()) {
                if spelling != spelling.to_lowercase() {
                    return Err(bad(format!("alias non normalise : {spelling}")));
                }
                if !seen.insert(spelling.clone()) {
                    return Err(bad(format!("ecriture de strie en double : {spelling}")));
                }
            }
        }

        let mut symbols = BTreeSet::new();
        for parameter in &self.parameters {
            if !parameter.symbol.starts_with('R') {
                return Err(bad(format!(
                    "{} n'est pas un parametre du profil de rugosite",
                    parameter.symbol
                )));
            }
            if !symbols.insert(parameter.symbol.to_lowercase()) {
                return Err(bad(format!("parametre en double : {}", parameter.symbol)));
            }
            if seen.contains(&parameter.symbol.to_lowercase()) {
                return Err(bad(format!(
                    "{} s'ecrit comme un sens des stries",
                    parameter.symbol
                )));
            }
        }
        if !symbols.contains("ra") {
            return Err(bad(
                "Ra manque : c'est le parametre du tableau des procedes".into(),
            ));
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn symbols(&self) -> &[SymbolVariant] {
        &self.symbols
    }

    pub fn lays(&self) -> &[LaySymbol] {
        &self.lays
    }

    pub fn parameters(&self) -> &[ProfileParameter] {
        &self.parameters
    }

    pub fn symbol(&self, id: ProcessRequirement) -> Option<&SymbolVariant> {
        self.symbols.iter().find(|s| s.id == id)
    }

    /// Le symbole designe par son ecriture en texte, quelle que soit la casse.
    pub fn symbol_by_code(&self, code: &str) -> Option<&SymbolVariant> {
        self.symbols
            .iter()
            .find(|s| s.code.eq_ignore_ascii_case(code.trim()))
    }

    /// Le sens des stries designe par son symbole ou un alias.
    pub fn lay(&self, needle: &str) -> Option<&LaySymbol> {
        let lower = needle.trim().to_lowercase();
        self.lays
            .iter()
            .find(|l| l.symbol.to_lowercase() == lower || l.aliases.contains(&lower))
    }

    /// Le parametre designe par son symbole, quelle que soit la casse.
    pub fn parameter(&self, needle: &str) -> Option<&ProfileParameter> {
        self.parameters
            .iter()
            .find(|p| p.symbol.eq_ignore_ascii_case(needle.trim()))
    }
}

/* ------------------------------------------------------------------ */
/*  Classes N — ISO 1302:1992                                          */
/* ------------------------------------------------------------------ */

/// Une classe de rugosite et sa valeur Ra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoughnessGrade {
    /// `N7`.
    pub grade: String,
    /// La valeur Ra maximale que la classe designe.
    pub ra: Length,
}

#[derive(Debug, Deserialize)]
struct RawGrade {
    grade: String,
    ra_um: Value,
}

#[derive(Debug, Deserialize)]
struct RawGradeTable {
    dataset: String,
    standard: StandardReference,
    grades: Vec<RawGrade>,
}

/// Les classes N1 a N12.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradeTable {
    dataset: String,
    standard: StandardReference,
    grades: Vec<RoughnessGrade>,
}

const GRADES_EMBEDDED: &str = include_str!("../../../data/surface/iso1302-1992.classes-n.json");

impl GradeTable {
    pub fn embedded() -> Result<&'static GradeTable> {
        static CACHE: OnceLock<core::result::Result<GradeTable, StandardsError>> = OnceLock::new();
        CACHE
            .get_or_init(|| GradeTable::parse(GRADES_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<GradeTable> {
        let raw: RawGradeTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso1302-1992.classes-n".into(),
                detail: source.to_string(),
            })?;
        let grades = raw
            .grades
            .iter()
            .map(|g| {
                Ok(RoughnessGrade {
                    grade: g.grade.clone(),
                    ra: length_from_json(&raw.dataset, &g.ra_um, Unit::Micrometre)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let table = GradeTable {
            dataset: raw.dataset,
            standard: raw.standard,
            grades,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        if self.grades.is_empty() {
            return Err(bad("aucune classe".into()));
        }
        for (index, grade) in self.grades.iter().enumerate() {
            // Les classes se suivent sans trou : N1, N2... Une classe sautee
            // decalerait toutes les suivantes d'un cran sans rien casser
            // d'autre, d'ou ce controle explicite.
            if grade.grade != format!("N{}", index + 1) {
                return Err(bad(format!(
                    "classe {} au rang {} : N{} attendue",
                    grade.grade,
                    index + 1,
                    index + 1
                )));
            }
            if !grade.ra.is_positive() {
                return Err(bad(format!("{} sans valeur positive", grade.grade)));
            }
        }
        // Chaque classe double la precedente, a l'arrondi pres (3,2 puis 6,3).
        // Une faute de frappe deplace une valeur d'un facteur dix : elle sort
        // largement de cette fourchette, la ou l'arrondi reste a quelques pour
        // cent.
        for pair in self.grades.windows(2) {
            let (previous, next) = (pair[0].ra.nanometres(), pair[1].ra.nanometres());
            if !(19 * previous <= 10 * next && 10 * next <= 21 * previous) {
                return Err(bad(format!(
                    "{} ne double pas {} : {} contre {}",
                    pair[1].grade, pair[0].grade, next, previous
                )));
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn grades(&self) -> &[RoughnessGrade] {
        &self.grades
    }

    /// La classe designee par son nom, `N7` ou `n7`.
    pub fn by_name(&self, name: &str) -> Option<&RoughnessGrade> {
        self.grades
            .iter()
            .find(|g| g.grade.eq_ignore_ascii_case(name.trim()))
    }

    /// La classe dont la valeur Ra est exactement celle-ci.
    pub fn by_ra(&self, ra: Length) -> Option<&RoughnessGrade> {
        self.grades.iter().find(|g| g.ra == ra)
    }

    /// Le rang d'une valeur dans la serie, quand elle y figure.
    pub fn index_of(&self, ra: Length) -> Option<usize> {
        self.grades.iter().position(|g| g.ra == ra)
    }
}

/* ------------------------------------------------------------------ */
/*  Rugosite par procede — ordres de grandeur d'atelier                */
/* ------------------------------------------------------------------ */

/// Une plage de Ra, bornes incluses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaRange {
    /// La valeur la plus fine.
    pub finest: Length,
    /// La valeur la plus grossiere.
    pub coarsest: Length,
}

impl RaRange {
    pub fn contains(&self, ra: Length) -> bool {
        self.finest <= ra && ra <= self.coarsest
    }
}

/// Ce qu'un procede donne d'ordinaire, et ce qu'il peut donner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRoughness {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    /// Vrai pour un usinage, faux pour une mise en forme.
    pub removal: bool,
    /// Fabrication courante.
    pub usual: RaRange,
    /// Avec des soins particuliers.
    pub possible: RaRange,
}

#[derive(Debug, Deserialize)]
struct RawProcess {
    id: String,
    name: String,
    aliases: Vec<String>,
    removal: bool,
    usual_um: [Value; 2],
    possible_um: [Value; 2],
}

#[derive(Debug, Deserialize)]
struct RawProcessTable {
    dataset: String,
    standard: StandardReference,
    processes: Vec<RawProcess>,
}

/// Le tableau des procedes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTable {
    dataset: String,
    standard: StandardReference,
    processes: Vec<ProcessRoughness>,
}

const PROCESSES_EMBEDDED: &str = include_str!("../../../data/surface/procedes.rugosite.json");

impl ProcessTable {
    pub fn embedded() -> Result<&'static ProcessTable> {
        static CACHE: OnceLock<core::result::Result<ProcessTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| ProcessTable::parse(PROCESSES_EMBEDDED, GradeTable::embedded()?))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str, grades: &GradeTable) -> Result<ProcessTable> {
        let raw: RawProcessTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "procedes.rugosite".into(),
                detail: source.to_string(),
            })?;
        let range = |pair: &[Value; 2]| -> Result<RaRange> {
            Ok(RaRange {
                finest: length_from_json(&raw.dataset, &pair[0], Unit::Micrometre)?,
                coarsest: length_from_json(&raw.dataset, &pair[1], Unit::Micrometre)?,
            })
        };
        let processes = raw
            .processes
            .iter()
            .map(|p| {
                Ok(ProcessRoughness {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    aliases: p.aliases.clone(),
                    removal: p.removal,
                    usual: range(&p.usual_um)?,
                    possible: range(&p.possible_um)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let table = ProcessTable {
            dataset: raw.dataset,
            standard: raw.standard,
            processes,
        };
        table.validate(grades)?;
        Ok(table)
    }

    /// Controles, dont un recoupement avec la table des classes N.
    fn validate(&self, grades: &GradeTable) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        if self.processes.is_empty() {
            return Err(bad("aucun procede".into()));
        }
        let mut ids = BTreeSet::new();
        let mut aliases = BTreeSet::new();
        for p in &self.processes {
            if !ids.insert(p.id.as_str()) {
                return Err(bad(format!("procede en double : {}", p.id)));
            }
            for alias in &p.aliases {
                if alias != &alias.to_lowercase() {
                    return Err(bad(format!("alias non normalise : {alias}")));
                }
                // Un alias partage rendrait « rectifie » ambigu sans que rien
                // ne le signale.
                if !aliases.insert(alias.as_str()) {
                    return Err(bad(format!("alias en double : {alias}")));
                }
            }
            for (label, range) in [("usuelle", p.usual), ("atteignable", p.possible)] {
                if range.finest >= range.coarsest {
                    return Err(bad(format!("{} : plage {label} vide ou inversee", p.id)));
                }
                // Toute borne tombe sur la serie des classes N : une borne hors
                // serie serait une faute, ou une precision que la donnee n'a pas.
                for bound in [range.finest, range.coarsest] {
                    if grades.by_ra(bound).is_none() {
                        return Err(bad(format!(
                            "{} : la borne {} nm n'est pas une valeur de la serie N",
                            p.id,
                            bound.nanometres()
                        )));
                    }
                }
            }
            // Ce qu'un procede donne d'ordinaire, il peut le donner : la plage
            // usuelle est contenue dans la plage atteignable.
            if !(p.possible.contains(p.usual.finest) && p.possible.contains(p.usual.coarsest)) {
                return Err(bad(format!(
                    "{} : la plage usuelle deborde de la plage atteignable",
                    p.id
                )));
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn processes(&self) -> &[ProcessRoughness] {
        &self.processes
    }

    pub fn by_id(&self, id: &str) -> Option<&ProcessRoughness> {
        self.processes.iter().find(|p| p.id == id)
    }

    /// Le procede designe par un nom ou un alias, quelle que soit la casse.
    pub fn lookup(&self, needle: &str) -> Option<&ProcessRoughness> {
        let lower = needle.trim().to_lowercase();
        self.processes
            .iter()
            .find(|p| p.id == lower || p.name == lower || p.aliases.contains(&lower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn um(text: &str) -> Length {
        Length::parse(text, Unit::Micrometre).unwrap()
    }

    fn indication() -> &'static IndicationTable {
        IndicationTable::embedded().expect("l'indication doit charger")
    }

    fn grades() -> &'static GradeTable {
        GradeTable::embedded().expect("les classes N doivent charger")
    }

    fn processes() -> &'static ProcessTable {
        ProcessTable::embedded().expect("le tableau des procedes doit charger")
    }

    #[test]
    fn les_trois_jeux_chargent() {
        assert_eq!(indication().symbols().len(), 3);
        assert_eq!(indication().lays().len(), 7);
        assert_eq!(grades().grades().len(), 12);
        assert!(processes().processes().len() >= 15);
    }

    #[test]
    fn aucun_jeu_ne_se_dit_verifie() {
        // Saisis sans document ouvert : les presenter autrement serait citer
        // une source qu'on n'a pas lue.
        for reference in [
            indication().standard(),
            grades().standard(),
            processes().standard(),
        ] {
            assert!(!reference.verification.is_verified(), "{}", reference.id);
            let banner = reference.verification.banner_fr().unwrap();
            assert!(banner.contains("non vérifiée"), "{banner}");
        }
    }

    #[test]
    fn le_jeu_dindication_dit_quil_ne_mele_pas_deux_generations() {
        let notes = indication().standard().notes.join(" ");
        assert!(notes.contains("21920"));
        assert!(notes.contains("4287"));
        // Et le millesime n'est pas invente.
        assert_eq!(indication().standard().citation(), "ISO 21920-1");
    }

    #[test]
    fn les_classes_n_rendent_les_valeurs_dusage() {
        assert_eq!(grades().by_name("N7").unwrap().ra, um("1.6"));
        assert_eq!(grades().by_name("n1").unwrap().ra, um("0.025"));
        assert_eq!(grades().by_name("N12").unwrap().ra, um("50"));
        assert_eq!(grades().by_ra(um("6.3")).unwrap().grade, "N9");
        // 1 µm ne correspond a aucune classe.
        assert!(grades().by_ra(um("1")).is_none());
        assert!(grades().by_name("N13").is_none());
    }

    #[test]
    fn la_norme_des_classes_n_est_datee_et_dite_retiree() {
        assert_eq!(grades().standard().citation(), "ISO 1302:1992");
        assert!(grades().standard().notes[0].contains("RETIRÉE"));
    }

    #[test]
    fn une_classe_n_sautee_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(GRADES_EMBEDDED).unwrap();
        broken["grades"].as_array_mut().unwrap().remove(4);
        let err = GradeTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("N5 attendue"), "{err}");
    }

    #[test]
    fn une_faute_de_frappe_dans_les_classes_est_reperee() {
        // 1,6 saisi 16 : la valeur ne double plus la precedente.
        let mut broken: serde_json::Value = serde_json::from_str(GRADES_EMBEDDED).unwrap();
        broken["grades"][6]["ra_um"] = serde_json::json!(16);
        let err = GradeTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("ne double pas"), "{err}");
    }

    #[test]
    fn les_bornes_des_procedes_tombent_sur_la_serie() {
        let mut broken: serde_json::Value = serde_json::from_str(PROCESSES_EMBEDDED).unwrap();
        broken["processes"][0]["usual_um"][0] = serde_json::json!(1.5);
        let err = ProcessTable::parse(&broken.to_string(), grades()).unwrap_err();
        assert!(err.to_string().contains("serie N"), "{err}");
    }

    #[test]
    fn la_plage_usuelle_reste_dans_la_plage_atteignable() {
        let mut broken: serde_json::Value = serde_json::from_str(PROCESSES_EMBEDDED).unwrap();
        broken["processes"][0]["possible_um"] = serde_json::json!([3.2, 50]);
        let err = ProcessTable::parse(&broken.to_string(), grades()).unwrap_err();
        assert!(err.to_string().contains("deborde"), "{err}");
    }

    #[test]
    fn un_alias_partage_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(PROCESSES_EMBEDDED).unwrap();
        broken["processes"][1]["aliases"] = serde_json::json!(["sciage"]);
        let err = ProcessTable::parse(&broken.to_string(), grades()).unwrap_err();
        assert!(err.to_string().contains("alias en double"), "{err}");
    }

    #[test]
    fn la_rectification_est_plus_fine_que_le_fraisage() {
        // Un controle de vraisemblance, pas une valeur : un tableau ou le
        // fraisage battrait la rectification serait faux dans son principe.
        let grinding = processes().lookup("rectifié").unwrap();
        let milling = processes().lookup("fraisage").unwrap();
        assert!(grinding.usual.finest < milling.usual.finest);
        assert!(grinding.removal && milling.removal);
        assert!(!processes().lookup("forgé").unwrap().removal);
    }

    #[test]
    fn les_ecritures_se_retrouvent() {
        assert_eq!(
            indication().symbol_by_code("mrr").unwrap().id,
            ProcessRequirement::RemovalRequired
        );
        assert_eq!(indication().lay("perp").unwrap().symbol, "⊥");
        assert_eq!(indication().lay("x").unwrap().name, "croisées");
        assert_eq!(indication().parameter("rz").unwrap().symbol, "Rz");
        assert!(indication().parameter("Rsm").is_none());
    }

    #[test]
    fn rz_porte_sa_mise_en_garde_historique() {
        let rz = indication().parameter("Rz").unwrap();
        assert!(rz.note.as_deref().unwrap().contains("1984"));
    }
}
