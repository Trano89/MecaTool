//! Soudure : procedes (ISO 4063), symboles (ISO 2553), niveaux de qualite
//! (ISO 5817).
//!
//! # Ce qui a ete lu dans la norme
//!
//! La nomenclature ISO 4063 est complete et lue dans la norme (ISO 4063:2009,
//! version corrigee 2010) : un numero absent n'appartient pas a la norme.
//! L'etat de verification de chaque autre jeu est porte par son fichier.
//!
//! # Ce que la validation garantit
//!
//! * chaque numero de procede releve d'un groupe principal embarque : `135`
//!   suppose `1` — la hierarchie se lit dans les chiffres ;
//! * un numero de l'Annexe A (remplace ou depasse) n'est jamais aussi en
//!   usage ;
//! * chaque imperfection couvre les epaisseurs de 0,5 mm a l'infini, sans trou
//!   ni recouvrement ;
//! * **un niveau plus exigeant ne tolere jamais davantage** : B ≤ C ≤ D sur
//!   chaque ligne, constante, coefficient et plafond compris. C'est le controle
//!   le plus utile : une valeur saisie dans la mauvaise colonne le fait tomber.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use mecatool_core::{Length, StandardReference, Unit};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Result, StandardsError};
use crate::value::length_from_json;

/* ------------------------------------------------------------------ */
/*  Procedes — ISO 4063                                                */
/* ------------------------------------------------------------------ */

/// Un procede, un groupe ou un groupe principal de la nomenclature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeldingProcess {
    /// Le numero de reference : `135`.
    pub number: String,
    /// Le terme prefere de la norme.
    pub name: String,
    /// Les synonymes que la norme donne a la suite du terme prefere.
    #[serde(default)]
    pub synonyms: Vec<String>,
    /// Abreviations et noms d'atelier, en minuscules. Un meme alias peut
    /// designer plusieurs numeros : `mag` designe 135, 136 et 138. Ce sont
    /// des cles de recherche, pas des termes de la norme.
    pub aliases: Vec<String>,
    /// Les designations americaines que l'Annexe B donne pour exactement
    /// equivalentes : `SMAW` pour 111.
    #[serde(default)]
    pub us_designations: Vec<String>,
}

/// Une lettre de variante : mode de transfert (Tableau 1) ou element
/// additionnel (Tableau 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantLetter {
    pub letter: String,
    pub name: String,
}

/// Un numero que l'Annexe A donne pour remplace ou depasse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplacedProcess {
    pub number: String,
    pub name: String,
}

/// La nomenclature embarquee.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WeldingProcessTable {
    dataset: String,
    standard: StandardReference,
    transfer_modes: Vec<VariantLetter>,
    additional_items: Vec<VariantLetter>,
    processes: Vec<WeldingProcess>,
    replaced: Vec<ReplacedProcess>,
}

const PROCESSES_EMBEDDED: &str = include_str!("../../../data/soudure/iso4063.procedes.json");

impl WeldingProcessTable {
    pub fn embedded() -> Result<&'static WeldingProcessTable> {
        static CACHE: OnceLock<core::result::Result<WeldingProcessTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| WeldingProcessTable::parse(PROCESSES_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<WeldingProcessTable> {
        let table: WeldingProcessTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso4063.procedes".into(),
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
        let mut numbers = BTreeSet::new();
        for p in &self.processes {
            if p.number.is_empty()
                || p.number.len() > 3
                || !p.number.chars().all(|c| c.is_ascii_digit())
            {
                return Err(bad(format!("numero illisible : {:?}", p.number)));
            }
            if !numbers.insert(p.number.as_str()) {
                return Err(bad(format!("numero en double : {}", p.number)));
            }
            for alias in &p.aliases {
                if alias != &alias.to_lowercase() {
                    return Err(bad(format!("alias non normalise : {alias}")));
                }
            }
            for us in &p.us_designations {
                if us.is_empty()
                    || !us
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
                {
                    return Err(bad(format!("designation US illisible : {us:?}")));
                }
            }
        }
        // La hierarchie se lit dans les chiffres : chaque numero releve d'un
        // groupe principal embarque. La norme ne donne pas toujours le niveau
        // intermediaire — 185 n'a pas de groupe 18 —, mais jamais un numero
        // sans groupe principal.
        for p in &self.processes {
            let root = &p.number[..1];
            if !numbers.contains(root) {
                return Err(bad(format!(
                    "{} sans son groupe principal {root}",
                    p.number
                )));
            }
        }
        // Un numero remplace qui serait aussi dans la liste principale
        // rendrait la lecture ambigue : la liste principale prevaut, et
        // l'entree de l'Annexe A ne doit pas etre embarquee.
        for r in &self.replaced {
            if numbers.contains(r.number.as_str()) {
                return Err(bad(format!(
                    "{} est a la fois en usage et remplace",
                    r.number
                )));
            }
        }
        // Une lettre de variante ne designe qu'une chose : `C` ne peut etre a
        // la fois un mode de transfert et un element additionnel.
        let mut letters = BTreeSet::new();
        for v in self.transfer_modes.iter().chain(&self.additional_items) {
            if v.letter.len() != 1 || !v.letter.chars().all(|c| c.is_ascii_uppercase()) {
                return Err(bad(format!(
                    "lettre de variante illisible : {:?}",
                    v.letter
                )));
            }
            if !letters.insert(v.letter.as_str()) {
                return Err(bad(format!("lettre de variante en double : {}", v.letter)));
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn processes(&self) -> &[WeldingProcess] {
        &self.processes
    }

    pub fn by_number(&self, number: &str) -> Option<&WeldingProcess> {
        self.processes.iter().find(|p| p.number == number.trim())
    }

    /// Tous les numeros que designe un alias, un terme de la norme ou une
    /// designation US. Plusieurs, parfois.
    pub fn by_alias(&self, alias: &str) -> Vec<&WeldingProcess> {
        let lower = alias.trim().to_lowercase();
        self.processes
            .iter()
            .filter(|p| {
                p.aliases.contains(&lower)
                    || p.name.to_lowercase() == lower
                    || p.synonyms.iter().any(|s| s.to_lowercase() == lower)
                    || p.us_designations.iter().any(|u| u.to_lowercase() == lower)
            })
            .collect()
    }

    /// Un numero de l'Annexe A : remplace ou depasse, encore lisible dans des
    /// documents anciens.
    pub fn replaced(&self, number: &str) -> Option<&ReplacedProcess> {
        self.replaced.iter().find(|r| r.number == number.trim())
    }

    pub fn replaced_processes(&self) -> &[ReplacedProcess] {
        &self.replaced
    }

    /// Les modes de transfert du Tableau 1.
    pub fn transfer_modes(&self) -> &[VariantLetter] {
        &self.transfer_modes
    }

    /// Les elements additionnels du Tableau 2.
    pub fn additional_items(&self) -> &[VariantLetter] {
        &self.additional_items
    }

    /// Le groupe, puis le sous-groupe, puis le procede lui-meme.
    pub fn lineage(&self, number: &str) -> Vec<&WeldingProcess> {
        (1..=number.len())
            .filter_map(|end| self.by_number(&number[..end]))
            .collect()
    }

    /// Vrai si d'autres numeros embarques descendent de celui-ci.
    pub fn has_children(&self, number: &str) -> bool {
        self.processes
            .iter()
            .any(|p| p.number.len() > number.len() && p.number.starts_with(number))
    }

    /// Les numeros embarques qui descendent directement de celui-ci.
    pub fn children(&self, number: &str) -> Vec<&WeldingProcess> {
        self.processes
            .iter()
            .filter(|p| p.number.len() == number.len() + 1 && p.number.starts_with(number))
            .collect()
    }
}

/* ------------------------------------------------------------------ */
/*  Symboles — ISO 2553:2013, et sa cotation                           */
/* ------------------------------------------------------------------ */

/// La famille d'un symbole elementaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointFamily {
    /// Soudure bout a bout : se cote en `s`.
    Butt,
    /// Soudure d'angle : se cote en `a` ou en `z`.
    Fillet,
    /// Tout le reste : points, bouchons, rechargement...
    Other,
}

/// Une lettre de cote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeLetter {
    pub letter: String,
    pub name: String,
    pub meaning: String,
}

/// Un systeme de representation : A ou B.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeldSystem {
    pub id: String,
    pub name: String,
    pub reference_line: String,
    pub note: Option<String>,
}

/// Un symbole elementaire : le tableau 1, complete de sa cotation (article 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementarySymbol {
    /// Identifiant stable, celui de la surcouche : `fillet`.
    pub id: String,
    /// Le numero du tableau 1 de l'ISO 2553:2013.
    pub number: u32,
    /// La designation de la norme, mot pour mot.
    pub name: String,
    /// Vrai quand la norme le declare a pleine penetration par defaut.
    pub full_penetration: bool,
    /// Vrai quand il peut servir pour plus de deux parties.
    pub multi_part: bool,
    pub note: Option<String>,
    /// Cotation : le nom de la forme symetrique, quand le tableau 2 lui en
    /// donne un (double V, K, double U). `None` ne veut pas dire que la forme
    /// double n'existe pas : la norme ne la nomme pas.
    pub both_sides_name: Option<String>,
    /// Cotation : la famille de joint.
    pub family: JointFamily,
    /// Cotation : les lettres de cote principale admises (5.2 a 5.12).
    pub sizes: Vec<String>,
    /// Cotation : vrai quand la norme exige une cote (soudures evasees, 5.4.4).
    #[serde(default)]
    pub size_required: bool,
    /// Cotation : le nom d'une lettre propre a ce symbole, quand il differe du
    /// nom general (`s` = epaisseur du rechargement sur une soudure de
    /// rechargement).
    #[serde(default)]
    pub size_names: std::collections::BTreeMap<String, String>,
}

impl ElementarySymbol {
    /// Le nom de la cote `letter` sur ce symbole, s'il lui est propre.
    pub fn size_name(&self, letter: &str) -> Option<&str> {
        self.size_names.get(letter).map(String::as_str)
    }
}

/// Un symbole supplementaire : le tableau 3, complete de son emploi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplementarySymbol {
    pub id: String,
    /// Le numero du tableau 3 de l'ISO 2553:2013.
    pub number: u32,
    /// La designation de la norme, mot pour mot.
    pub name: String,
    /// La note de la norme, quand elle en porte une.
    pub meaning: Option<String>,
    /// Cotation : les familles sur lesquelles la norme le montre ou le
    /// prescrit. Ce n'est pas une interdiction ailleurs.
    pub families: Vec<JointFamily>,
    /// Cotation : les symboles elementaires, hors de ces familles, avec
    /// lesquels la norme l'emploie aussi.
    #[serde(default)]
    pub symbols: Vec<u32>,
}

impl SupplementarySymbol {
    /// Vrai si la norme montre ce symbole supplementaire sur `symbol`.
    pub fn shown_on(&self, symbol: &ElementarySymbol) -> bool {
        self.families.contains(&symbol.family) || self.symbols.contains(&symbol.number)
    }
}

#[derive(Debug, Deserialize)]
struct VerifiedElementary {
    number: u32,
    designation: String,
    full_penetration: bool,
    multi_part: bool,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifiedSupplementary {
    number: u32,
    designation: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifiedFootnotes {
    full_penetration: String,
}

#[derive(Debug, Deserialize)]
struct VerifiedSymbols {
    dataset: String,
    standard: StandardReference,
    systems: Vec<WeldSystem>,
    system_rules: Vec<String>,
    footnotes: VerifiedFootnotes,
    elementary: Vec<VerifiedElementary>,
    supplementary: Vec<VerifiedSupplementary>,
}

#[derive(Debug, Deserialize)]
struct OverlayElementary {
    number: u32,
    id: String,
    family: JointFamily,
    sizes: Vec<String>,
    #[serde(default)]
    size_required: bool,
    #[serde(default)]
    size_names: std::collections::BTreeMap<String, String>,
    both_sides_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OverlaySupplementary {
    number: u32,
    id: String,
    families: Vec<JointFamily>,
    #[serde(default)]
    symbols: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct Overlay {
    dataset: String,
    standard: StandardReference,
    sizes: Vec<SizeLetter>,
    elementary: Vec<OverlayElementary>,
    supplementary: Vec<OverlaySupplementary>,
}

/// Les symboles embarques.
///
/// Deux jeux de donnees, tous deux lus dans l'ISO 2553:2013 (`verified`) : le
/// tableau des symboles (tableaux 1 et 3, 4.2 a 4.4), et sa cotation — famille,
/// cotes admises, forme double — lue a l'article 5 et aux tableaux 2 et 5. La
/// cotation est une surcouche : elle ne remplace aucun mot du tableau, elle s'y
/// accroche par le numero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeldSymbolTable {
    standard: StandardReference,
    cotation: StandardReference,
    systems: Vec<WeldSystem>,
    system_rules: Vec<String>,
    full_penetration_rule: String,
    sizes: Vec<SizeLetter>,
    elementary: Vec<ElementarySymbol>,
    supplementary: Vec<SupplementarySymbol>,
}

const SYMBOLS_EMBEDDED: &str = include_str!("../../../data/soudure/iso2553-2013.symbols.json");
const COTATION_EMBEDDED: &str = include_str!("../../../data/soudure/iso2553-2013.cotation.json");

impl WeldSymbolTable {
    pub fn embedded() -> Result<&'static WeldSymbolTable> {
        static CACHE: OnceLock<core::result::Result<WeldSymbolTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| WeldSymbolTable::parse(SYMBOLS_EMBEDDED, COTATION_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(symbols: &str, cotation: &str) -> Result<WeldSymbolTable> {
        let verified: VerifiedSymbols =
            serde_json::from_str(symbols).map_err(|source| StandardsError::Malformed {
                dataset: "iso2553-2013.symbols".into(),
                detail: source.to_string(),
            })?;
        let overlay: Overlay =
            serde_json::from_str(cotation).map_err(|source| StandardsError::Malformed {
                dataset: "iso2553-2013.cotation".into(),
                detail: source.to_string(),
            })?;
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: overlay.dataset.clone(),
            detail,
        };

        // La surcouche couvre exactement le tableau verifie : ni symbole oublie,
        // ni symbole invente.
        let verified_numbers: Vec<u32> = verified.elementary.iter().map(|e| e.number).collect();
        let overlay_numbers: Vec<u32> = overlay.elementary.iter().map(|e| e.number).collect();
        if verified_numbers != overlay_numbers {
            return Err(bad(format!(
                "la surcouche ne couvre pas exactement le tableau verifie ({} : {overlay_numbers:?}, {} : {verified_numbers:?})",
                overlay.dataset, verified.dataset
            )));
        }
        let verified_supplementary: Vec<u32> =
            verified.supplementary.iter().map(|e| e.number).collect();
        let overlay_supplementary: Vec<u32> =
            overlay.supplementary.iter().map(|e| e.number).collect();
        if verified_supplementary != overlay_supplementary {
            return Err(bad(
                "la surcouche ne couvre pas exactement les symboles supplementaires".into(),
            ));
        }

        let elementary = verified
            .elementary
            .into_iter()
            .zip(overlay.elementary)
            .map(|(v, o)| ElementarySymbol {
                id: o.id,
                number: v.number,
                name: v.designation,
                full_penetration: v.full_penetration,
                multi_part: v.multi_part,
                note: v.note,
                both_sides_name: o.both_sides_name,
                family: o.family,
                sizes: o.sizes,
                size_required: o.size_required,
                size_names: o.size_names,
            })
            .collect();
        let supplementary = verified
            .supplementary
            .into_iter()
            .zip(overlay.supplementary)
            .map(|(v, o)| SupplementarySymbol {
                id: o.id,
                number: v.number,
                name: v.designation,
                meaning: v.note,
                families: o.families,
                symbols: o.symbols,
            })
            .collect();

        let table = WeldSymbolTable {
            standard: verified.standard,
            cotation: overlay.standard,
            systems: verified.systems,
            system_rules: verified.system_rules,
            full_penetration_rule: verified.footnotes.full_penetration,
            sizes: overlay.sizes,
            elementary,
            supplementary,
        };
        table.validate(&overlay.dataset)?;
        Ok(table)
    }

    fn validate(&self, dataset: &str) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: dataset.to_string(),
            detail,
        };
        let letters: BTreeSet<&str> = self.sizes.iter().map(|s| s.letter.as_str()).collect();
        let mut ids = BTreeSet::new();
        for symbol in &self.elementary {
            if !ids.insert(symbol.id.as_str()) {
                return Err(bad(format!("symbole en double : {}", symbol.id)));
            }
            for size in &symbol.sizes {
                if !letters.contains(size.as_str()) {
                    return Err(bad(format!("{} : cote inconnue {size}", symbol.id)));
                }
            }
            // Le recoupement entre les deux jeux. Ce que la norme declare a
            // pleine penetration par defaut (note b du tableau 1) est une
            // soudure bout a bout ; une soudure bout a bout qui ne l'est pas
            // doit toujours etre cotee (5.4.4). Une famille mal rangee fait
            // tomber le chargement.
            if symbol.full_penetration && symbol.family != JointFamily::Butt {
                return Err(bad(format!(
                    "{} (n° {}) : la norme la declare a pleine penetration, la surcouche ne \
                     la range pas parmi les soudures bout a bout",
                    symbol.id, symbol.number
                )));
            }
            if symbol.family == JointFamily::Butt && !symbol.full_penetration {
                if !symbol.size_required {
                    return Err(bad(format!(
                        "{} (n° {}) : soudure bout a bout que la norme ne declare pas a pleine \
                         penetration, elle doit toujours etre cotee (5.4.4)",
                        symbol.id, symbol.number
                    )));
                }
            } else if symbol.size_required {
                return Err(bad(format!(
                    "{} (n° {}) : seules les soudures bout a bout sans pleine penetration par \
                     defaut doivent toujours etre cotees",
                    symbol.id, symbol.number
                )));
            }
            // Une soudure bout a bout se cote en s, une soudure d'angle en a ou
            // z, et jamais l'inverse.
            let expected: &[&str] = match symbol.family {
                JointFamily::Butt => &["s"],
                JointFamily::Fillet => &["a", "z"],
                JointFamily::Other => &[],
            };
            if !expected.is_empty() && symbol.sizes != expected {
                return Err(bad(format!(
                    "{} : une famille {:?} se cote en {expected:?}",
                    symbol.id, symbol.family
                )));
            }
            // s se retrouve hors des deux familles (soudure sur chant,
            // rechargement : 5.10, 5.12) ; a et z, jamais.
            if symbol.family == JointFamily::Other
                && symbol.sizes.iter().any(|s| matches!(s.as_str(), "a" | "z"))
            {
                return Err(bad(format!(
                    "{} : a et z sont reserves aux soudures d'angle",
                    symbol.id
                )));
            }
            for letter in symbol.size_names.keys() {
                if !symbol.sizes.contains(letter) {
                    return Err(bad(format!(
                        "{} : nom propre a la cote {letter}, qui n'est pas admise",
                        symbol.id
                    )));
                }
            }
        }
        let numbers: BTreeSet<u32> = self.elementary.iter().map(|s| s.number).collect();
        let mut supplementary = BTreeSet::new();
        for symbol in &self.supplementary {
            if !supplementary.insert(symbol.id.as_str()) {
                return Err(bad(format!(
                    "symbole supplementaire en double : {}",
                    symbol.id
                )));
            }
            if symbol.families.is_empty() {
                return Err(bad(format!("{} ne s'applique a aucune famille", symbol.id)));
            }
            if let Some(unknown) = symbol.symbols.iter().find(|n| !numbers.contains(n)) {
                return Err(bad(format!(
                    "{} : symbole elementaire n° {unknown} inconnu",
                    symbol.id
                )));
            }
        }
        Ok(())
    }

    /// Le tableau des symboles, verifie sur la norme.
    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    /// La surcouche de cotation, lue a l'article 5 de la norme.
    pub fn cotation_standard(&self) -> &StandardReference {
        &self.cotation
    }

    pub fn systems(&self) -> &[WeldSystem] {
        &self.systems
    }

    pub fn system_rules(&self) -> &[String] {
        &self.system_rules
    }

    /// La regle de pleine penetration, telle que la norme l'enonce.
    pub fn full_penetration_rule(&self) -> &str {
        &self.full_penetration_rule
    }

    pub fn sizes(&self) -> &[SizeLetter] {
        &self.sizes
    }

    pub fn elementary(&self) -> &[ElementarySymbol] {
        &self.elementary
    }

    pub fn supplementary(&self) -> &[SupplementarySymbol] {
        &self.supplementary
    }

    pub fn symbol(&self, id: &str) -> Option<&ElementarySymbol> {
        self.elementary.iter().find(|s| s.id == id)
    }

    pub fn size(&self, letter: &str) -> Option<&SizeLetter> {
        self.sizes.iter().find(|s| s.letter == letter)
    }

    pub fn supplementary_symbol(&self, id: &str) -> Option<&SupplementarySymbol> {
        self.supplementary.iter().find(|s| s.id == id)
    }
}

/* ------------------------------------------------------------------ */
/*  Niveaux de qualite — ISO 5817                                      */
/* ------------------------------------------------------------------ */

/// Un niveau de qualite : B, C ou D.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityLevel {
    pub id: String,
    pub name: String,
    pub meaning: String,
}

/// Une grandeur dont une limite depend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityVariable {
    pub symbol: String,
    pub meaning: String,
}

/// La grandeur a laquelle un coefficient s'applique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// Epaisseur de la piece.
    T,
    /// Epaisseur de gorge d'une soudure d'angle.
    A,
    /// Epaisseur nominale d'une soudure bout a bout.
    S,
    /// Largeur de la surepaisseur ou de la penetration.
    B,
    /// `s` pour une soudure bout a bout, `a` pour une soudure d'angle.
    Weld,
}

impl Basis {
    pub const fn symbol(self) -> &'static str {
        match self {
            Basis::T => "t",
            Basis::A => "a",
            Basis::S => "s",
            Basis::B => "b",
            Basis::Weld => "s ou a",
        }
    }
}

/// Le type de soudure auquel une imperfection se rapporte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliesTo {
    Butt,
    Fillet,
    Both,
}

impl AppliesTo {
    pub fn includes(self, family: JointFamily) -> bool {
        match self {
            AppliesTo::Both => matches!(family, JointFamily::Butt | JointFamily::Fillet),
            AppliesTo::Butt => family == JointFamily::Butt,
            AppliesTo::Fillet => family == JointFamily::Fillet,
        }
    }
}

/// La limite d'une imperfection pour un niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Limit {
    NotPermitted,
    Permitted {
        /// La condition qui accompagne l'admission, le cas echeant.
        condition: Option<String>,
    },
    /// `h ≤ constante + coefficient × grandeur`, plafonne le cas echeant.
    Bound {
        /// La grandeur bornee : `h` (hauteur) ou `d` (diametre).
        measure: String,
        constant: Length,
        /// Le coefficient, en centiemes : 15 pour 0,15. Entier, donc exact.
        factor_hundredths: i64,
        of: Option<Basis>,
        max: Option<Length>,
        /// Vrai pour les « defauts courts ».
        short: bool,
    },
    /// L'angle de raccordement doit valoir au moins tant de degres.
    MinAngle {
        degrees: u32,
    },
}

impl Limit {
    /// Rang de permissivite, pour comparer deux niveaux.
    fn rank(&self) -> u8 {
        match self {
            Limit::NotPermitted => 0,
            Limit::Bound { .. } | Limit::MinAngle { .. } => 1,
            Limit::Permitted { .. } => 2,
        }
    }
}

/// Une plage d'epaisseur. `from` est inclus, `above` exclu, `to` inclus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThicknessRange {
    pub from: Option<Length>,
    pub above: Option<Length>,
    pub to: Option<Length>,
}

impl ThicknessRange {
    pub fn contains(&self, t: Length) -> bool {
        let low = match (self.from, self.above) {
            (Some(from), _) => t >= from,
            (None, Some(above)) => t > above,
            (None, None) => true,
        };
        low && self.to.is_none_or(|to| t <= to)
    }
}

/// Une limite pour un niveau donne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelLimit {
    pub level: String,
    pub limit: Limit,
}

/// Une ligne du tableau : une plage d'epaisseur et une limite par niveau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitRow {
    pub thickness: ThicknessRange,
    /// Dans l'ordre B, C, D.
    pub limits: Vec<LevelLimit>,
}

impl LimitRow {
    pub fn limit(&self, level: &str) -> Option<&Limit> {
        self.limits
            .iter()
            .find(|l| l.level == level)
            .map(|l| &l.limit)
    }
}

/// Une imperfection et ses limites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imperfection {
    /// Le numero de ligne du tableau : `1.9`.
    pub reference: String,
    /// Le numero de l'ISO 6520-1 : `502`.
    pub iso6520: String,
    pub name: String,
    pub applies_to: AppliesTo,
    pub remark: Option<String>,
    pub rows: Vec<LimitRow>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RawLimit {
    NotPermitted,
    Permitted {
        #[serde(default)]
        condition: Option<String>,
    },
    Bound {
        #[serde(default)]
        measure: Option<String>,
        #[serde(default)]
        constant_mm: Option<Value>,
        #[serde(default)]
        factor: Option<Value>,
        #[serde(default)]
        of: Option<Basis>,
        #[serde(default)]
        max_mm: Option<Value>,
        #[serde(default)]
        short: bool,
    },
    MinAngle {
        degrees: u32,
    },
}

#[derive(Debug, Deserialize)]
struct RawThickness {
    #[serde(default)]
    from: Option<Value>,
    #[serde(default)]
    above: Option<Value>,
    #[serde(default)]
    to: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RawRow {
    thickness: RawThickness,
    #[serde(rename = "B")]
    b: RawLimit,
    #[serde(rename = "C")]
    c: RawLimit,
    #[serde(rename = "D")]
    d: RawLimit,
}

#[derive(Debug, Deserialize)]
struct RawImperfection {
    #[serde(rename = "ref")]
    reference: String,
    iso6520: String,
    name: String,
    applies_to: AppliesTo,
    remark: Option<String>,
    rows: Vec<RawRow>,
}

/// Un groupe de procedes que la norme ne vise pas, et pourquoi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeExclusion {
    pub prefix: String,
    pub reason: String,
}

/// Une reserve que la norme attache a un procede vise : `31` pour l'acier
/// uniquement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeCondition {
    pub prefix: String,
    pub condition: String,
}

/// Les procedes auxquels la norme s'applique (ISO 5817, article 1 g).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessScope {
    pub note: String,
    /// Prefixes de numeros ISO 4063 que la norme cite : 11, 12, 13, 14, 15
    /// et 31, avec leurs sous-categories.
    pub fusion: Vec<String>,
    /// Les reserves attachees a certains procedes cites.
    #[serde(default)]
    pub conditions: Vec<ScopeCondition>,
    pub excluded: Vec<ScopeExclusion>,
    /// Ce que la norme dit d'un procede qu'elle ne cite ni n'exclut.
    pub not_listed: String,
}

/// Ce que la norme dit d'un procede.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScopeVerdict {
    /// Procede cite par l'article 1 g) : la norme s'applique, sous la reserve
    /// eventuelle que donne [`ProcessScope::condition`].
    InScope,
    /// La norme ne vise pas ce procede.
    Excluded { reason: String },
    /// Ni cite, ni exclu : l'Annexe B admet l'application a d'autres procedes
    /// de soudage par fusion, le cas echeant. Voir [`ProcessScope::not_listed`].
    Unknown,
}

impl ProcessScope {
    pub fn verdict(&self, number: &str) -> ScopeVerdict {
        if let Some(exclusion) = self
            .excluded
            .iter()
            .find(|e| number.starts_with(e.prefix.as_str()))
        {
            return ScopeVerdict::Excluded {
                reason: exclusion.reason.clone(),
            };
        }
        if self.fusion.iter().any(|p| number.starts_with(p.as_str())) {
            ScopeVerdict::InScope
        } else {
            ScopeVerdict::Unknown
        }
    }

    /// La reserve attachee a un procede vise : « pour l'acier uniquement ».
    pub fn condition(&self, number: &str) -> Option<&str> {
        self.conditions
            .iter()
            .find(|c| number.starts_with(c.prefix.as_str()))
            .map(|c| c.condition.as_str())
    }

    /// Les procedes cites que contient un groupe : `1` contient 11 a 15. Vide
    /// pour un numero qui n'est pas un groupe englobant un procede cite.
    pub fn covered_within(&self, number: &str) -> Vec<&str> {
        self.fusion
            .iter()
            .filter(|p| p.len() > number.len() && p.starts_with(number))
            .map(String::as_str)
            .collect()
    }

    fn validate(&self) -> core::result::Result<(), String> {
        let readable =
            |p: &str| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit());
        let all: Vec<&str> = self
            .fusion
            .iter()
            .map(String::as_str)
            .chain(self.excluded.iter().map(|e| e.prefix.as_str()))
            .collect();
        if self.fusion.is_empty() {
            return Err("aucun procede vise".into());
        }
        if let Some(bad) = all.iter().find(|p| !readable(p)) {
            return Err(format!("prefixe de procede illisible : {bad:?}"));
        }
        // Un procede ne peut etre a la fois vise et exclu : aucun prefixe n'en
        // englobe un autre.
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                if a.starts_with(b) || b.starts_with(a) {
                    return Err(format!("les prefixes {a} et {b} se recouvrent"));
                }
            }
        }
        for condition in &self.conditions {
            if !self
                .fusion
                .iter()
                .any(|p| condition.prefix.starts_with(p.as_str()))
            {
                return Err(format!(
                    "reserve sur {}, qui n'est pas un procede vise",
                    condition.prefix
                ));
            }
        }
        if self.not_listed.trim().is_empty() {
            return Err("rien n'est dit des procedes non cites".into());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct RawQualityTable {
    dataset: String,
    standard: StandardReference,
    process_scope: ProcessScope,
    levels: Vec<QualityLevel>,
    variables: Vec<QualityVariable>,
    imperfections: Vec<RawImperfection>,
}

/// Les niveaux de qualite et les limites embarquees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityTable {
    dataset: String,
    standard: StandardReference,
    process_scope: ProcessScope,
    levels: Vec<QualityLevel>,
    variables: Vec<QualityVariable>,
    imperfections: Vec<Imperfection>,
}

const QUALITY_EMBEDDED: &str = include_str!("../../../data/soudure/iso5817.niveaux-qualite.json");

/// L'epaisseur a partir de laquelle la norme s'applique.
const MIN_THICKNESS_UM: i64 = 500;

impl QualityTable {
    pub fn embedded() -> Result<&'static QualityTable> {
        static CACHE: OnceLock<core::result::Result<QualityTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| QualityTable::parse(QUALITY_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<QualityTable> {
        let raw: RawQualityTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso5817.niveaux-qualite".into(),
                detail: source.to_string(),
            })?;
        let dataset = raw.dataset.clone();
        let mm = |value: &Value| length_from_json(&dataset, value, Unit::Millimetre);
        let optional_mm = |value: &Option<Value>| value.as_ref().map(mm).transpose();

        let limit = |raw: &RawLimit| -> Result<Limit> {
            Ok(match raw {
                RawLimit::NotPermitted => Limit::NotPermitted,
                RawLimit::Permitted { condition } => Limit::Permitted {
                    condition: condition.clone(),
                },
                RawLimit::MinAngle { degrees } => Limit::MinAngle { degrees: *degrees },
                RawLimit::Bound {
                    measure,
                    constant_mm,
                    factor,
                    of,
                    max_mm,
                    short,
                } => Limit::Bound {
                    measure: measure.clone().unwrap_or_else(|| "h".to_string()),
                    constant: optional_mm(constant_mm)?.unwrap_or(Length::ZERO),
                    factor_hundredths: match factor {
                        Some(value) => hundredths(&dataset, value)?,
                        None => 0,
                    },
                    of: *of,
                    max: optional_mm(max_mm)?,
                    short: *short,
                },
            })
        };

        let imperfections = raw
            .imperfections
            .iter()
            .map(|i| {
                let rows = i
                    .rows
                    .iter()
                    .map(|row| {
                        Ok(LimitRow {
                            thickness: ThicknessRange {
                                from: optional_mm(&row.thickness.from)?,
                                above: optional_mm(&row.thickness.above)?,
                                to: optional_mm(&row.thickness.to)?,
                            },
                            limits: vec![
                                LevelLimit {
                                    level: "B".into(),
                                    limit: limit(&row.b)?,
                                },
                                LevelLimit {
                                    level: "C".into(),
                                    limit: limit(&row.c)?,
                                },
                                LevelLimit {
                                    level: "D".into(),
                                    limit: limit(&row.d)?,
                                },
                            ],
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Imperfection {
                    reference: i.reference.clone(),
                    iso6520: i.iso6520.clone(),
                    name: i.name.clone(),
                    applies_to: i.applies_to,
                    remark: i.remark.clone(),
                    rows,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let table = QualityTable {
            dataset: raw.dataset,
            standard: raw.standard,
            process_scope: raw.process_scope,
            levels: raw.levels,
            variables: raw.variables,
            imperfections,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        let levels: Vec<&str> = self.levels.iter().map(|l| l.id.as_str()).collect();
        if levels != ["B", "C", "D"] {
            return Err(bad(format!("niveaux attendus B, C, D ; trouve {levels:?}")));
        }
        if self.imperfections.is_empty() {
            return Err(bad("aucune imperfection".into()));
        }
        self.process_scope
            .validate()
            .map_err(|detail| bad(format!("domaine des procedes : {detail}")))?;
        let minimum = Length::from_micrometres(MIN_THICKNESS_UM);

        for imperfection in &self.imperfections {
            let label = format!("{} ({})", imperfection.reference, imperfection.name);
            if imperfection.rows.is_empty() {
                return Err(bad(format!("{label} : aucune ligne")));
            }

            // Les plages d'epaisseur se suivent de 0,5 mm a l'infini, sans trou
            // ni recouvrement.
            let first = imperfection.rows[0].thickness;
            if first.from != Some(minimum) || first.above.is_some() {
                return Err(bad(format!(
                    "{label} : la premiere plage doit partir de 0,5 mm"
                )));
            }
            for pair in imperfection.rows.windows(2) {
                let (previous, next) = (pair[0].thickness, pair[1].thickness);
                match (previous.to, next.above, next.from) {
                    (Some(to), Some(above), None) if to == above => {}
                    _ => {
                        return Err(bad(format!(
                            "{label} : les plages d'epaisseur ne se suivent pas"
                        )))
                    }
                }
            }
            if imperfection.rows.last().unwrap().thickness.to.is_some() {
                return Err(bad(format!(
                    "{label} : la derniere plage doit etre ouverte"
                )));
            }

            for row in &imperfection.rows {
                for level_limit in &row.limits {
                    if let Limit::Bound {
                        factor_hundredths,
                        of,
                        constant,
                        max,
                        ..
                    } = &level_limit.limit
                    {
                        if *factor_hundredths != 0 && of.is_none() {
                            return Err(bad(format!(
                                "{label} : un coefficient sans grandeur de reference"
                            )));
                        }
                        if *of == Some(Basis::Weld) && imperfection.applies_to != AppliesTo::Both {
                            return Err(bad(format!(
                                "{label} : « s ou a » suppose une imperfection commune aux deux \
                                 types de soudure"
                            )));
                        }
                        if *factor_hundredths == 0 && constant.is_zero() {
                            return Err(bad(format!("{label} : une borne nulle")));
                        }
                        if max.is_some_and(|m| !m.is_positive()) {
                            return Err(bad(format!("{label} : un plafond nul")));
                        }
                    }
                }
                // Un niveau plus exigeant ne tolere jamais davantage. C'est le
                // controle qui fait tomber une valeur saisie dans la mauvaise
                // colonne.
                for pair in row.limits.windows(2) {
                    if !not_looser(&pair[0].limit, &pair[1].limit) {
                        return Err(bad(format!(
                            "{label} : le niveau {} tolere davantage que le niveau {}",
                            pair[0].level, pair[1].level
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn process_scope(&self) -> &ProcessScope {
        &self.process_scope
    }

    pub fn levels(&self) -> &[QualityLevel] {
        &self.levels
    }

    pub fn variables(&self) -> &[QualityVariable] {
        &self.variables
    }

    pub fn imperfections(&self) -> &[Imperfection] {
        &self.imperfections
    }

    pub fn level(&self, id: &str) -> Option<&QualityLevel> {
        self.levels
            .iter()
            .find(|l| l.id.eq_ignore_ascii_case(id.trim()))
    }

    /// L'epaisseur en dessous de laquelle la norme ne s'applique pas.
    pub fn minimum_thickness(&self) -> Length {
        Length::from_micrometres(MIN_THICKNESS_UM)
    }
}

/// Vrai si `strict` ne tolere pas davantage que `loose`.
fn not_looser(strict: &Limit, loose: &Limit) -> bool {
    if strict.rank() != loose.rank() {
        return strict.rank() < loose.rank();
    }
    match (strict, loose) {
        (
            Limit::Bound {
                constant: c1,
                factor_hundredths: f1,
                of: of1,
                max: m1,
                short: s1,
                ..
            },
            Limit::Bound {
                constant: c2,
                factor_hundredths: f2,
                of: of2,
                max: m2,
                short: s2,
                ..
            },
        ) => {
            let same_basis = of1 == of2 || *f1 == 0 || *f2 == 0;
            let ceiling = match (m1, m2) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(a), Some(b)) => a <= b,
            };
            // Une borne reservee aux defauts courts tolere moins qu'une borne
            // valable sur toute la longueur : le niveau exigeant ne peut pas
            // etre le seul a l'admettre sur toute la longueur. (1.17, t ≤ 3 :
            // D sur toute la longueur, C en defauts courts — et non l'inverse.)
            let length = *s1 || !*s2;
            same_basis && c1 <= c2 && f1 <= f2 && ceiling && length
        }
        // Un angle plus grand est plus exigeant : le raccordement est plus doux.
        (Limit::MinAngle { degrees: a }, Limit::MinAngle { degrees: b }) => a >= b,
        (Limit::Bound { .. }, Limit::MinAngle { .. })
        | (Limit::MinAngle { .. }, Limit::Bound { .. }) => false,
        _ => true,
    }
}

/// Un coefficient decimal, lu en centiemes exacts : `0.15` donne 15.
///
/// Un coefficient a trois decimales n'existe pas dans la norme : il est refuse
/// plutot qu'arrondi.
fn hundredths(dataset: &str, value: &Value) -> Result<i64> {
    let text = match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => {
            return Err(StandardsError::Malformed {
                dataset: dataset.to_string(),
                detail: format!("coefficient numerique attendu, trouve {other}"),
            })
        }
    };
    let malformed = || StandardsError::Malformed {
        dataset: dataset.to_string(),
        detail: format!("coefficient {text:?} : au plus deux decimales"),
    };
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    if fraction.len() > 2 || whole.is_empty() {
        return Err(malformed());
    }
    let whole: i64 = whole.parse().map_err(|_| malformed())?;
    let fraction: i64 = if fraction.is_empty() {
        0
    } else {
        format!("{fraction:0<2}").parse().map_err(|_| malformed())?
    };
    Ok(whole * 100 + fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processes() -> &'static WeldingProcessTable {
        WeldingProcessTable::embedded().expect("la nomenclature doit charger")
    }

    fn symbols() -> &'static WeldSymbolTable {
        WeldSymbolTable::embedded().expect("les symboles doivent charger")
    }

    fn quality() -> &'static QualityTable {
        QualityTable::embedded().expect("les niveaux de qualite doivent charger")
    }

    fn mm(text: &str) -> Length {
        Length::parse(text, Unit::Millimetre).unwrap()
    }

    #[test]
    fn la_cotation_est_lue_a_larticle_5() {
        // Les niveaux de qualite (ISO 5817:2014) sont desormais lus dans la
        // norme : voir `les_niveaux_de_qualite_sont_lus_dans_la_norme`. La
        // cotation des symboles l'est aussi, a l'article 5 de l'ISO 2553:2013.
        let reference = symbols().cotation_standard();
        assert!(reference.verification.is_verified(), "{}", reference.id);
        assert_eq!(reference.citation(), "ISO 2553:2013");
        // Meme norme que le tableau des symboles : seul le perimetre les
        // distingue a l'affichage.
        assert_ne!(reference.scope, symbols().standard().scope);
        assert!(reference.source.contains("article 5"));
    }

    #[test]
    fn la_nomenclature_est_lue_dans_la_norme() {
        let reference = processes().standard();
        assert!(reference.verification.is_verified());
        assert_eq!(reference.edition, "2009");
        // Les numeros que la version corrigee a deplaces ou ajoutes.
        assert!(processes().by_number("977").is_some());
        assert!(processes().by_number("973").is_some());
        assert!(processes().by_number("983").is_none());
        assert!(processes().by_number("987").is_none());
        assert_eq!(
            processes().by_number("81").unwrap().name,
            "coupage à la flamme"
        );
    }

    #[test]
    fn un_numero_remplace_se_lit_a_lannexe_a() {
        assert!(processes().by_number("181").is_none());
        assert_eq!(
            processes().replaced("181").unwrap().name,
            "soudage à l'arc avec électrode de carbone"
        );
        // 43 a change de sens : la liste principale prevaut.
        assert!(processes().replaced("43").is_none());
        assert_eq!(
            processes().by_number("43").unwrap().name,
            "soudage par friction-malaxage"
        );
    }

    #[test]
    fn les_designations_us_de_lannexe_b_se_lisent() {
        let numbers = |text: &str| -> Vec<String> {
            processes()
                .by_alias(text)
                .iter()
                .map(|p| p.number.clone())
                .collect()
        };
        assert_eq!(numbers("SMAW"), ["111"]);
        assert_eq!(numbers("fcaw"), ["114", "136"]);
        assert_eq!(numbers("SW"), ["783", "785", "786"]);
    }

    #[test]
    fn un_numero_remplace_ne_double_pas_un_numero_en_usage() {
        let mut broken: serde_json::Value = serde_json::from_str(PROCESSES_EMBEDDED).unwrap();
        broken["replaced"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "number": "43", "name": "soudage par forgeage" }));
        let err = WeldingProcessTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("en usage et remplace"), "{err}");
    }

    #[test]
    fn la_hierarchie_se_lit_dans_les_chiffres() {
        let lineage: Vec<&str> = processes()
            .lineage("135")
            .iter()
            .map(|p| p.number.as_str())
            .collect();
        assert_eq!(lineage, ["1", "13", "135"]);
        assert!(processes().has_children("13"));
        assert!(!processes().has_children("135"));
    }

    #[test]
    fn un_alias_datelier_designe_plusieurs_numeros() {
        let mag: Vec<&str> = processes()
            .by_alias("MAG")
            .iter()
            .map(|p| p.number.as_str())
            .collect();
        assert_eq!(mag, ["135", "136", "138"]);
        assert_eq!(processes().by_alias("tig").len(), 5);
    }

    #[test]
    fn un_numero_sans_groupe_principal_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(PROCESSES_EMBEDDED).unwrap();
        broken["processes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "number": "667", "name": "x", "aliases": [] }));
        let err = WeldingProcessTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            err.to_string().contains("sans son groupe principal 6"),
            "{err}"
        );
    }

    #[test]
    fn une_soudure_dangle_se_cote_en_a_ou_z_et_une_bout_a_bout_en_s() {
        assert_eq!(symbols().symbol("fillet").unwrap().sizes, ["a", "z"]);
        assert_eq!(symbols().symbol("single_v").unwrap().sizes, ["s"]);
        let mut broken: serde_json::Value = serde_json::from_str(COTATION_EMBEDDED).unwrap();
        broken["elementary"][1]["sizes"] = serde_json::json!(["a"]);
        let err = WeldSymbolTable::parse(SYMBOLS_EMBEDDED, &broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("se cote en"), "{err}");
    }

    #[test]
    fn les_designations_sont_celles_de_ledition_2013() {
        // Le recueil reproduit l'edition 1992 (« soudure en I ») ; MecaTool suit
        // la norme lue elle-meme.
        let square = symbols().symbol("square_butt").unwrap();
        assert_eq!(square.number, 1);
        assert_eq!(square.name, "soudure bout à bout à bords droits");
        assert_eq!(symbols().elementary().len(), 22);
        assert_eq!(symbols().supplementary().len(), 6);
        assert_eq!(symbols().systems().len(), 2);
    }

    #[test]
    fn deux_jeux_une_meme_norme() {
        assert!(symbols().standard().verification.is_verified());
        assert_eq!(symbols().standard().citation(), "ISO 2553:2013");
        assert!(symbols().cotation_standard().verification.is_verified());
    }

    #[test]
    fn la_pleine_penetration_de_la_norme_reste_bout_a_bout() {
        // On sort la soudure en V des soudures bout a bout : la norme la
        // declare a pleine penetration, le recoupement doit tomber.
        let mut broken: serde_json::Value = serde_json::from_str(COTATION_EMBEDDED).unwrap();
        broken["elementary"][1]["family"] = serde_json::json!("other");
        broken["elementary"][1]["sizes"] = serde_json::json!([]);
        let err = WeldSymbolTable::parse(SYMBOLS_EMBEDDED, &broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("pleine penetration"), "{err}");
    }

    #[test]
    fn une_bout_a_bout_sans_pleine_penetration_doit_etre_cotee() {
        // Les soudures evasees sont bout a bout (3.20, 3.21) et toujours
        // cotees (5.4.4).
        for id in ["flare_v", "flare_bevel"] {
            let flare = symbols().symbol(id).unwrap();
            assert_eq!(flare.family, JointFamily::Butt);
            assert!(!flare.full_penetration);
            assert!(flare.size_required);
            assert_eq!(flare.sizes, ["s"]);
        }
        // On range la soudure d'angle parmi les bout a bout, sans l'obligation
        // de cote : le recoupement doit tomber.
        let mut broken: serde_json::Value = serde_json::from_str(COTATION_EMBEDDED).unwrap();
        broken["elementary"][9]["family"] = serde_json::json!("butt");
        broken["elementary"][9]["sizes"] = serde_json::json!(["s"]);
        let err = WeldSymbolTable::parse(SYMBOLS_EMBEDDED, &broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("toujours etre cotee"), "{err}");
    }

    #[test]
    fn s_cote_aussi_la_soudure_sur_chant_et_le_rechargement() {
        let edge = symbols().symbol("edge").unwrap();
        assert_eq!(edge.sizes, ["s"]);
        assert_eq!(edge.size_name("s"), Some("épaisseur de métal fondu"));
        let surfacing = symbols().symbol("surfacing").unwrap();
        assert_eq!(surfacing.size_name("s"), Some("épaisseur du rechargement"));
        // Les bords releves n'exigent pas de cotation (5.4.3), la soudure par
        // transparence n'en recoit pas a l'article 5 de l'edition 2013.
        assert!(symbols().symbol("flanged").unwrap().sizes.is_empty());
        assert!(symbols().symbol("transparency").unwrap().sizes.is_empty());
        // a et z restent reserves aux soudures d'angle.
        let mut broken: serde_json::Value = serde_json::from_str(COTATION_EMBEDDED).unwrap();
        broken["elementary"][18]["sizes"] = serde_json::json!(["a"]);
        broken["elementary"][18]["size_names"] = serde_json::json!({});
        let err = WeldSymbolTable::parse(SYMBOLS_EMBEDDED, &broken.to_string()).unwrap_err();
        assert!(
            err.to_string().contains("reserves aux soudures d'angle"),
            "{err}"
        );
    }

    #[test]
    fn la_surepaisseur_a_la_racine_se_montre_aussi_sur_chant() {
        let root = symbols()
            .supplementary_symbol("root_reinforcement")
            .unwrap();
        assert!(root.shown_on(symbols().symbol("single_v").unwrap()));
        assert!(root.shown_on(symbols().symbol("edge").unwrap()));
        assert!(!root.shown_on(symbols().symbol("fillet").unwrap()));
        let mut broken: serde_json::Value = serde_json::from_str(COTATION_EMBEDDED).unwrap();
        broken["supplementary"][5]["symbols"] = serde_json::json!([99]);
        let err = WeldSymbolTable::parse(SYMBOLS_EMBEDDED, &broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("n° 99 inconnu"), "{err}");
    }

    #[test]
    fn la_surcouche_couvre_exactement_le_tableau_verifie() {
        let mut broken: serde_json::Value = serde_json::from_str(COTATION_EMBEDDED).unwrap();
        broken["elementary"].as_array_mut().unwrap().pop();
        let err = WeldSymbolTable::parse(SYMBOLS_EMBEDDED, &broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("exactement"), "{err}");
    }

    #[test]
    fn les_formes_doubles_sont_celles_du_tableau_2() {
        let double = |id: &str| symbols().symbol(id).unwrap().both_sides_name.clone();
        assert_eq!(
            double("single_v").as_deref(),
            Some("soudure bout à bout en double V")
        );
        assert_eq!(
            double("single_bevel").as_deref(),
            Some("soudure bout à bout en K")
        );
        assert_eq!(
            double("single_u").as_deref(),
            Some("soudure bout à bout en double U")
        );
        // La norme ne nomme pas les autres : la surcouche n'invente rien.
        let named = symbols()
            .elementary()
            .iter()
            .filter(|s| s.both_sides_name.is_some())
            .count();
        assert_eq!(named, 3);
    }

    #[test]
    fn les_niveaux_sont_b_c_d() {
        assert_eq!(quality().levels().len(), 3);
        assert_eq!(quality().level("c").unwrap().name, "intermédiaire");
    }

    #[test]
    fn une_fissure_nest_admise_a_aucun_niveau() {
        let crack = &quality().imperfections()[0];
        assert_eq!(crack.iso6520, "100");
        for level in ["B", "C", "D"] {
            assert_eq!(crack.rows[0].limit(level), Some(&Limit::NotPermitted));
        }
    }

    #[test]
    fn les_coefficients_sont_lus_en_centiemes_exacts() {
        let excess = quality()
            .imperfections()
            .iter()
            .find(|i| i.iso6520 == "502")
            .unwrap();
        match excess.rows[0].limit("C").unwrap() {
            Limit::Bound {
                constant,
                factor_hundredths,
                of,
                max,
                ..
            } => {
                assert_eq!(*constant, mm("1"));
                assert_eq!(*factor_hundredths, 15);
                assert_eq!(*of, Some(Basis::B));
                assert_eq!(*max, Some(mm("7")));
            }
            other => panic!("borne attendue : {other:?}"),
        }
        assert!(hundredths("t", &serde_json::json!(0.125)).is_err());
        assert_eq!(hundredths("t", &serde_json::json!(1.0)).unwrap(), 100);
        assert_eq!(hundredths("t", &serde_json::json!(0.05)).unwrap(), 5);
    }

    #[test]
    fn les_plages_depaisseur_se_suivent() {
        let undercut = quality()
            .imperfections()
            .iter()
            .find(|i| i.reference == "1.7")
            .unwrap();
        assert!(undercut.rows[0].thickness.contains(mm("3")));
        assert!(!undercut.rows[1].thickness.contains(mm("3")));
        assert!(undercut.rows[1].thickness.contains(mm("3.001")));
        assert!(!undercut.rows[0].thickness.contains(mm("0.4")));
    }

    #[test]
    fn un_niveau_plus_exigeant_ne_tolere_jamais_davantage() {
        // On inverse B et D sur la surepaisseur : B tolererait 10 mm, D 5 mm.
        let mut broken: serde_json::Value = serde_json::from_str(QUALITY_EMBEDDED).unwrap();
        let row = &mut broken["imperfections"][quality_index("502")]["rows"][0];
        let b = row["B"].clone();
        row["B"] = row["D"].clone();
        row["D"] = b;
        let err = QualityTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("tolere davantage"), "{err}");
    }

    /// La position d'une imperfection dans le fichier embarque.
    fn quality_index(iso6520: &str) -> usize {
        quality()
            .imperfections()
            .iter()
            .position(|i| i.iso6520 == iso6520)
            .unwrap_or_else(|| panic!("{iso6520} absent"))
    }

    #[test]
    fn les_niveaux_de_qualite_sont_lus_dans_la_norme() {
        let reference = quality().standard();
        assert!(reference.verification.is_verified());
        assert_eq!(reference.citation(), "ISO 5817:2014");
        let line = |iso6520: &str| &quality().imperfections()[quality_index(iso6520)];

        // Fissure de cratere : non autorisee, niveau D compris (Tableau 1, 1.2).
        assert_eq!(line("104").rows[0].limit("D"), Some(&Limit::NotPermitted));
        // Le defaut d'alignement se lit en 5071 et 5072, et non 507.
        assert!(quality().imperfections().iter().all(|i| i.iso6520 != "507"));
        assert_eq!(line("5072").rows[0].limit("B").unwrap().rank(), 1);
        // Retassure a la racine, t ≤ 3 : D vaut sur toute la longueur, C
        // seulement en defauts courts.
        match (
            line("515").rows[0].limit("D"),
            line("515").rows[0].limit("C"),
        ) {
            (Some(Limit::Bound { short: d, .. }), Some(Limit::Bound { short: c, .. })) => {
                assert!(!d && *c);
            }
            other => panic!("{other:?}"),
        }
        // Les noms des niveaux ne disent que leur rang : la norme ne les nomme
        // pas.
        assert!(quality()
            .level("B")
            .unwrap()
            .meaning
            .contains("plus élevée"));
    }

    #[test]
    fn un_niveau_exigeant_nadmet_pas_sur_toute_la_longueur_ce_que_d_reserve_aux_defauts_courts() {
        // Caniveau, t ≤ 3 : D et C en defauts courts. On retire la reserve a
        // C, qui tolererait alors sur toute la longueur ce que D limite.
        let mut broken: serde_json::Value = serde_json::from_str(QUALITY_EMBEDDED).unwrap();
        broken["imperfections"][quality_index("5011, 5012")]["rows"][0]["C"]["short"] =
            serde_json::json!(false);
        let err = QualityTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("tolere davantage"), "{err}");
    }

    #[test]
    fn une_plage_depaisseur_trouee_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(QUALITY_EMBEDDED).unwrap();
        broken["imperfections"][2]["rows"][1]["thickness"] = serde_json::json!({ "above": 4 });
        let err = QualityTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("ne se suivent pas"), "{err}");
    }

    #[test]
    fn la_norme_vise_les_procedes_de_son_article_1() {
        let scope = quality().process_scope();
        assert_eq!(scope.verdict("135"), ScopeVerdict::InScope);
        assert_eq!(scope.condition("135"), None);
        // 31 : pour l'acier uniquement.
        assert_eq!(scope.verdict("311"), ScopeVerdict::InScope);
        assert!(scope.condition("311").unwrap().contains("acier"));
        match scope.verdict("52") {
            ScopeVerdict::Excluded { reason } => assert!(reason.contains("13919")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(scope.verdict("91"), ScopeVerdict::Excluded { .. }));
        // Le soudage sous laitier, les goujons, la resistance : ni cites, ni
        // exclus. L'Annexe B laisse la porte ouverte, MecaTool aussi.
        for number in ["72", "783", "21"] {
            assert_eq!(scope.verdict(number), ScopeVerdict::Unknown, "{number}");
        }
        assert!(scope.not_listed.contains("Annexe B"));
        // Le groupe 1 n'est vise que par ses groupes 11 a 15.
        assert_eq!(scope.verdict("1"), ScopeVerdict::Unknown);
        assert_eq!(scope.covered_within("1"), ["11", "12", "13", "14", "15"]);
        assert!(scope.covered_within("135").is_empty());
    }

    #[test]
    fn les_procedes_du_domaine_existent_dans_la_nomenclature() {
        let scope = quality().process_scope();
        for prefix in scope
            .fusion
            .iter()
            .chain(scope.excluded.iter().map(|e| &e.prefix))
            .chain(scope.conditions.iter().map(|c| &c.prefix))
        {
            assert!(processes().by_number(prefix).is_some(), "{prefix}");
        }
    }

    #[test]
    fn un_procede_ne_peut_etre_a_la_fois_vise_et_exclu() {
        let mut broken: serde_json::Value = serde_json::from_str(QUALITY_EMBEDDED).unwrap();
        broken["process_scope"]["excluded"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "prefix": "1", "reason": "x" }));
        let err = QualityTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("se recouvrent"), "{err}");

        let mut broken: serde_json::Value = serde_json::from_str(QUALITY_EMBEDDED).unwrap();
        broken["process_scope"]["conditions"][0]["prefix"] = serde_json::json!("72");
        let err = QualityTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("pas un procede vise"), "{err}");
    }

    #[test]
    fn le_jeu_dit_quil_ne_choisit_pas_le_niveau() {
        let notes = quality().standard().notes.join(" ");
        assert!(notes.contains("NE CHOISIT PAS LE NIVEAU"));
    }
}
