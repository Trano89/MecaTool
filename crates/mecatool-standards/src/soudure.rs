//! Soudure : procedes (ISO 4063), symboles (ISO 2553), niveaux de qualite
//! (ISO 5817).
//!
//! # Trois jeux, une meme reserve
//!
//! Les trois jeux sont saisis sans document source ouvert, et portent donc
//! [`VerificationStatus::Unverified`]. Ce sont des **selections** : un numero de
//! procede, un symbole ou une imperfection absents ne sont pas invalides, ils
//! ne sont pas embarques — et le moteur le dit dans ces termes.
//!
//! # Ce que la validation garantit
//!
//! * chaque numero de procede a son parent : `135` suppose `13`, qui suppose
//!   `1` — la hierarchie se lit dans les chiffres, elle ne doit pas se rompre ;
//! * chaque imperfection couvre les epaisseurs de 0,5 mm a l'infini, sans trou
//!   ni recouvrement ;
//! * **un niveau plus exigeant ne tolere jamais davantage** : B ≤ C ≤ D sur
//!   chaque ligne, constante, coefficient et plafond compris. C'est le controle
//!   le plus utile : une valeur saisie dans la mauvaise colonne le fait tomber.
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
/*  Procedes — ISO 4063                                                */
/* ------------------------------------------------------------------ */

/// Un procede, un sous-groupe ou un groupe de la nomenclature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeldingProcess {
    /// Le numero de reference : `135`.
    pub number: String,
    pub name: String,
    /// Abreviations et noms d'atelier, en minuscules. Un meme alias peut
    /// designer plusieurs numeros : `mag` designe 135, 136 et 138.
    pub aliases: Vec<String>,
}

/// La nomenclature embarquee.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WeldingProcessTable {
    dataset: String,
    standard: StandardReference,
    processes: Vec<WeldingProcess>,
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
        }
        // La hierarchie se lit dans les chiffres : chaque numero suppose son
        // parent. Un parent manquant laisserait un procede sans groupe, et la
        // lecture « 135 appartient au sous-groupe 13 » deviendrait fausse.
        for p in &self.processes {
            if p.number.len() > 1 {
                let parent = &p.number[..p.number.len() - 1];
                if !numbers.contains(parent) {
                    return Err(bad(format!("{} sans son parent {parent}", p.number)));
                }
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

    /// Tous les numeros que designe un alias. Plusieurs, parfois.
    pub fn by_alias(&self, alias: &str) -> Vec<&WeldingProcess> {
        let lower = alias.trim().to_lowercase();
        self.processes
            .iter()
            .filter(|p| p.aliases.contains(&lower) || p.name == lower)
            .collect()
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
/*  Symboles — ISO 2553                                                */
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

/// Un symbole elementaire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementarySymbol {
    pub id: String,
    pub name: String,
    /// Le nom de la forme symetrique, quand elle existe : V des deux cotes
    /// donne un X.
    pub both_sides_name: Option<String>,
    pub family: JointFamily,
    /// Les lettres de cote admises.
    pub sizes: Vec<String>,
}

/// Un symbole supplementaire : forme de surface, support envers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplementarySymbol {
    pub id: String,
    pub name: String,
    pub meaning: String,
    /// Les familles sur lesquelles on le rencontre.
    pub families: Vec<JointFamily>,
}

/// Les symboles embarques.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WeldSymbolTable {
    dataset: String,
    standard: StandardReference,
    sizes: Vec<SizeLetter>,
    elementary: Vec<ElementarySymbol>,
    supplementary: Vec<SupplementarySymbol>,
}

const SYMBOLS_EMBEDDED: &str = include_str!("../../../data/soudure/iso2553.symboles.json");

impl WeldSymbolTable {
    pub fn embedded() -> Result<&'static WeldSymbolTable> {
        static CACHE: OnceLock<core::result::Result<WeldSymbolTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| WeldSymbolTable::parse(SYMBOLS_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<WeldSymbolTable> {
        let table: WeldSymbolTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso2553.symboles".into(),
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
            // La regle de lecture la plus utile : une soudure bout a bout se
            // cote en s, une soudure d'angle en a ou z, et jamais l'inverse.
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
            if symbol.family == JointFamily::Other
                && symbol
                    .sizes
                    .iter()
                    .any(|s| matches!(s.as_str(), "a" | "z" | "s"))
            {
                return Err(bad(format!(
                    "{} : a, z et s sont reserves aux soudures bout a bout et d'angle",
                    symbol.id
                )));
            }
        }
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
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
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

/// Les procedes auxquels MecaTool applique la norme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessScope {
    pub note: String,
    /// Prefixes de numeros ISO 4063 vises : soudage par fusion.
    pub fusion: Vec<String>,
    pub excluded: Vec<ScopeExclusion>,
}

/// Ce que la norme dit d'un procede.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScopeVerdict {
    /// Procede de soudage par fusion : la norme s'applique.
    InScope,
    /// La norme ne vise pas ce procede.
    Excluded { reason: String },
    /// MecaTool ne sait pas : ni vise, ni ecarte par les donnees embarquees.
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
                ..
            },
            Limit::Bound {
                constant: c2,
                factor_hundredths: f2,
                of: of2,
                max: m2,
                ..
            },
        ) => {
            let same_basis = of1 == of2 || *f1 == 0 || *f2 == 0;
            let ceiling = match (m1, m2) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(a), Some(b)) => a <= b,
            };
            same_basis && c1 <= c2 && f1 <= f2 && ceiling
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
    fn les_trois_jeux_chargent_et_ne_se_disent_pas_verifies() {
        for reference in [
            processes().standard(),
            symbols().standard(),
            quality().standard(),
        ] {
            assert!(!reference.verification.is_verified(), "{}", reference.id);
            assert!(
                reference.edition.is_empty(),
                "millesime invente : {}",
                reference.id
            );
        }
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
        assert_eq!(processes().by_alias("tig").len(), 2);
    }

    #[test]
    fn un_numero_sans_parent_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(PROCESSES_EMBEDDED).unwrap();
        broken["processes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "number": "667", "name": "x", "aliases": [] }));
        let err = WeldingProcessTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("sans son parent 66"), "{err}");
    }

    #[test]
    fn une_soudure_dangle_se_cote_en_a_ou_z_et_une_bout_a_bout_en_s() {
        assert_eq!(symbols().symbol("fillet").unwrap().sizes, ["a", "z"]);
        assert_eq!(symbols().symbol("single_v").unwrap().sizes, ["s"]);
        let mut broken: serde_json::Value = serde_json::from_str(SYMBOLS_EMBEDDED).unwrap();
        broken["elementary"][1]["sizes"] = serde_json::json!(["a"]);
        let err = WeldSymbolTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("se cote en"), "{err}");
    }

    #[test]
    fn le_v_des_deux_cotes_est_un_x() {
        assert_eq!(
            symbols()
                .symbol("single_v")
                .unwrap()
                .both_sides_name
                .as_deref(),
            Some("soudure en X")
        );
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
        let row = &mut broken["imperfections"][7]["rows"][0];
        let b = row["B"].clone();
        row["B"] = row["D"].clone();
        row["D"] = b;
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
    fn la_norme_vise_le_soudage_par_fusion() {
        let scope = quality().process_scope();
        assert_eq!(scope.verdict("135"), ScopeVerdict::InScope);
        assert_eq!(scope.verdict("311"), ScopeVerdict::InScope);
        assert!(matches!(scope.verdict("21"), ScopeVerdict::Excluded { .. }));
        match scope.verdict("52") {
            ScopeVerdict::Excluded { reason } => assert!(reason.contains("13919")),
            other => panic!("{other:?}"),
        }
        // Les goujons : ni vises, ni ecartes. MecaTool dit qu'il ne sait pas.
        assert_eq!(scope.verdict("783"), ScopeVerdict::Unknown);
    }

    #[test]
    fn le_jeu_dit_quil_ne_choisit_pas_le_niveau() {
        let notes = quality().standard().notes.join(" ");
        assert!(notes.contains("NE CHOISIT PAS LE NIVEAU"));
    }
}
