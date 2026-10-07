//! Visserie : filetages metriques (ISO 261), profil de base (ISO 68-1), trous
//! de passage (ISO 273), classes de qualite (ISO 898).
//!
//! # Regles et tables
//!
//! Comme pour le symbole d'alesage des roulements, ce qui s'enonce en une
//! phrase reste une phrase : le profil de base est une **regle** (chaque
//! diametre se deduit du nominal en retranchant une fraction de `H`), le symbole
//! de classe de qualite aussi (`8.8` se lit, il ne se recopie pas). Seules les
//! correspondances sans regle — pas par diametre, trous de passage — sont des
//! tables.
//!
//! # Ce que la validation garantit
//!
//! * les fractions de `H` ordonnent les diametres : `d3 < d1 < d2 < d` ;
//! * pour chaque diametre, le pas gros est le plus gros, et les pas fins sont
//!   distincts et decroissants ;
//! * chaque filetage embarque a ses trous de passage, et la serie fine est plus
//!   serree que la moyenne, elle-meme plus serree que la large ;
//! * chaque classe de qualite se lit selon la regle, et la resistance croit
//!   d'une classe a la suivante.
//!
//! Les quatre jeux sont saisis sans document ouvert : ils portent
//! [`VerificationStatus::Unverified`].
//!
//! [`VerificationStatus::Unverified`]: mecatool_core::VerificationStatus::Unverified

use std::collections::BTreeSet;
use std::sync::OnceLock;

use mecatool_core::{Length, StandardReference, Unit};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Result, StandardsError};
use crate::value::{length_from_json, lengths_from_json};

/* ------------------------------------------------------------------ */
/*  Filetages — ISO 261                                                */
/* ------------------------------------------------------------------ */

/// Un diametre nominal et ses pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricThread {
    pub d: Length,
    /// Premier ou deuxieme choix.
    pub choice: u8,
    pub coarse: Length,
    /// Du plus gros au plus fin.
    pub fine: Vec<Length>,
}

#[derive(Debug, Deserialize)]
struct RawThread {
    d: Value,
    choice: u8,
    coarse: Value,
    fine: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct RawThreadTable {
    dataset: String,
    standard: StandardReference,
    threads: Vec<RawThread>,
}

/// Les filetages metriques embarques.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadTable {
    dataset: String,
    standard: StandardReference,
    threads: Vec<MetricThread>,
}

const THREADS_EMBEDDED: &str =
    include_str!("../../../data/visserie/iso261.filetages-metriques.json");

impl ThreadTable {
    pub fn embedded() -> Result<&'static ThreadTable> {
        static CACHE: OnceLock<core::result::Result<ThreadTable, StandardsError>> = OnceLock::new();
        CACHE
            .get_or_init(|| ThreadTable::parse(THREADS_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<ThreadTable> {
        let raw: RawThreadTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso261.filetages-metriques".into(),
                detail: source.to_string(),
            })?;
        let threads = raw
            .threads
            .iter()
            .map(|t| {
                Ok(MetricThread {
                    d: length_from_json(&raw.dataset, &t.d, Unit::Millimetre)?,
                    choice: t.choice,
                    coarse: length_from_json(&raw.dataset, &t.coarse, Unit::Millimetre)?,
                    fine: lengths_from_json(&raw.dataset, &t.fine, Unit::Millimetre)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let table = ThreadTable {
            dataset: raw.dataset,
            standard: raw.standard,
            threads,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        if self.threads.is_empty() {
            return Err(bad("aucun filetage".into()));
        }
        for pair in self.threads.windows(2) {
            if pair[1].d <= pair[0].d {
                return Err(bad(format!(
                    "diametres non croissants : {} apres {}",
                    pair[1].d.nanometres(),
                    pair[0].d.nanometres()
                )));
            }
            // Le pas gros ne decroit jamais quand le diametre grandit.
            if pair[1].coarse < pair[0].coarse {
                return Err(bad(format!(
                    "le pas gros de M{} est plus fin que celui de M{}",
                    pair[1].d, pair[0].d
                )));
            }
        }
        for thread in &self.threads {
            if !(1..=2).contains(&thread.choice) {
                return Err(bad(format!(
                    "M{} : choix {} inconnu",
                    thread.d, thread.choice
                )));
            }
            if !thread.coarse.is_positive() || thread.coarse >= thread.d {
                return Err(bad(format!("M{} : pas gros incoherent", thread.d)));
            }
            let mut previous = thread.coarse;
            for pitch in &thread.fine {
                if *pitch >= previous || !pitch.is_positive() {
                    return Err(bad(format!(
                        "M{} : les pas fins doivent etre plus fins que le pas gros, et \
                         decroissants",
                        thread.d
                    )));
                }
                previous = *pitch;
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn threads(&self) -> &[MetricThread] {
        &self.threads
    }

    pub fn by_diameter(&self, d: Length) -> Option<&MetricThread> {
        self.threads.iter().find(|t| t.d == d)
    }
}

/* ------------------------------------------------------------------ */
/*  Profil de base — ISO 68-1                                          */
/* ------------------------------------------------------------------ */

/// Un diametre de base, exprime comme `d − (n/m)·H`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicDiameter {
    pub symbol: String,
    pub name: String,
    /// La fraction de `H` retranchee du diametre nominal : `[3, 4]`.
    pub h_fraction: [i64; 2],
}

/// Le profil de base.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BasicProfile {
    dataset: String,
    standard: StandardReference,
    triangle: String,
    diameters: Vec<BasicDiameter>,
}

const PROFILE_EMBEDDED: &str = include_str!("../../../data/visserie/iso68-1.profil.json");

impl BasicProfile {
    pub fn embedded() -> Result<&'static BasicProfile> {
        static CACHE: OnceLock<core::result::Result<BasicProfile, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| BasicProfile::parse(PROFILE_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<BasicProfile> {
        let profile: BasicProfile =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso68-1.profil".into(),
                detail: source.to_string(),
            })?;
        profile.validate()?;
        Ok(profile)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        for symbol in ["d2", "d1", "d3"] {
            if self.diameter(symbol).is_none() {
                return Err(bad(format!("{symbol} manque")));
            }
        }
        // Les fractions ordonnent les diametres : chacun retranche davantage
        // que le precedent. Une fraction saisie a l'envers inverserait d1 et d2
        // sans que rien d'autre ne casse.
        let mut previous = (0, 1);
        for diameter in &self.diameters {
            let [num, den] = diameter.h_fraction;
            if num <= 0 || den <= 0 {
                return Err(bad(format!("{} : fraction non positive", diameter.symbol)));
            }
            if num * previous.1 <= previous.0 * den {
                return Err(bad(format!(
                    "{} ne retranche pas davantage que le diametre precedent",
                    diameter.symbol
                )));
            }
            previous = (num, den);
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    /// La relation du triangle generateur, en clair.
    pub fn triangle(&self) -> &str {
        &self.triangle
    }

    pub fn diameters(&self) -> &[BasicDiameter] {
        &self.diameters
    }

    pub fn diameter(&self, symbol: &str) -> Option<&BasicDiameter> {
        self.diameters.iter().find(|d| d.symbol == symbol)
    }
}

/* ------------------------------------------------------------------ */
/*  Trous de passage — ISO 273                                         */
/* ------------------------------------------------------------------ */

/// Une serie de trous de passage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearanceSeries {
    pub id: String,
    pub name: String,
    /// La classe de tolerance d'alesage du trou : `H13`.
    pub class: String,
}

/// Les trous de passage d'un diametre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearanceRow {
    pub d: Length,
    pub fine: Length,
    pub medium: Length,
    pub coarse: Length,
}

impl ClearanceRow {
    pub fn by_series(&self, series: &str) -> Option<Length> {
        match series {
            "fine" => Some(self.fine),
            "medium" => Some(self.medium),
            "coarse" => Some(self.coarse),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawClearanceRow {
    d: Value,
    fine: Value,
    medium: Value,
    coarse: Value,
}

#[derive(Debug, Deserialize)]
struct RawClearanceTable {
    dataset: String,
    standard: StandardReference,
    series: Vec<ClearanceSeries>,
    holes: Vec<RawClearanceRow>,
}

/// Les trous de passage embarques.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearanceTable {
    dataset: String,
    standard: StandardReference,
    series: Vec<ClearanceSeries>,
    holes: Vec<ClearanceRow>,
}

const CLEARANCE_EMBEDDED: &str =
    include_str!("../../../data/visserie/iso273.trous-de-passage.json");

impl ClearanceTable {
    pub fn embedded() -> Result<&'static ClearanceTable> {
        static CACHE: OnceLock<core::result::Result<ClearanceTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| ClearanceTable::parse(CLEARANCE_EMBEDDED, ThreadTable::embedded()?))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str, threads: &ThreadTable) -> Result<ClearanceTable> {
        let raw: RawClearanceTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso273.trous-de-passage".into(),
                detail: source.to_string(),
            })?;
        let mm = |v: &Value| length_from_json(&raw.dataset, v, Unit::Millimetre);
        let holes = raw
            .holes
            .iter()
            .map(|h| {
                Ok(ClearanceRow {
                    d: mm(&h.d)?,
                    fine: mm(&h.fine)?,
                    medium: mm(&h.medium)?,
                    coarse: mm(&h.coarse)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let table = ClearanceTable {
            dataset: raw.dataset,
            standard: raw.standard,
            series: raw.series,
            holes,
        };
        table.validate(threads)?;
        Ok(table)
    }

    fn validate(&self, threads: &ThreadTable) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        let ids: Vec<&str> = self.series.iter().map(|s| s.id.as_str()).collect();
        if ids != ["fine", "medium", "coarse"] {
            return Err(bad(format!(
                "series attendues fine, medium, coarse : {ids:?}"
            )));
        }
        for row in &self.holes {
            // Un trou de passage laisse passer la vis, et la serie fine est la
            // plus serree : d < fine < moyenne < large.
            if !(row.d < row.fine && row.fine < row.medium && row.medium < row.coarse) {
                return Err(bad(format!("M{} : series dans le desordre", row.d)));
            }
        }
        for pair in self.holes.windows(2) {
            if pair[1].d <= pair[0].d || pair[1].fine <= pair[0].fine {
                return Err(bad(format!("trous non croissants apres M{}", pair[0].d)));
            }
        }
        // Recoupement : chaque filetage embarque a ses trous de passage, et
        // aucun trou ne vise un filetage inconnu.
        let holes: BTreeSet<i64> = self.holes.iter().map(|h| h.d.nanometres()).collect();
        let diameters: BTreeSet<i64> = threads.threads().iter().map(|t| t.d.nanometres()).collect();
        if holes != diameters {
            return Err(bad(
                "les trous de passage et les filetages ne couvrent pas les memes diametres".into(),
            ));
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn series(&self) -> &[ClearanceSeries] {
        &self.series
    }

    pub fn holes(&self) -> &[ClearanceRow] {
        &self.holes
    }

    pub fn by_diameter(&self, d: Length) -> Option<&ClearanceRow> {
        self.holes.iter().find(|h| h.d == d)
    }
}

/* ------------------------------------------------------------------ */
/*  Classes de qualite — ISO 898                                       */
/* ------------------------------------------------------------------ */

/// Une classe de qualite de vis, lue selon la regle du symbole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoltClass {
    /// `8.8`.
    pub class: String,
    /// Le premier nombre : centieme de la resistance nominale.
    pub first: u32,
    /// Le second nombre : dix fois le rapport Re / Rm.
    pub second: u32,
    /// Resistance a la traction nominale, en MPa.
    pub tensile_mpa: u32,
    /// Limite d'elasticite nominale, en MPa.
    pub yield_mpa: u32,
    /// Le plus grand diametre pour lequel la classe est definie, en mm.
    pub max_d: Option<u32>,
}

impl BoltClass {
    /// Applique la regle du symbole : `8.8` donne 800 et 640 MPa.
    pub fn read(class: &str) -> Option<(u32, u32)> {
        let (first, second) = class.trim().split_once('.')?;
        if first.is_empty() || second.len() != 1 {
            return None;
        }
        Some((first.parse().ok()?, second.parse().ok()?))
    }
}

#[derive(Debug, Deserialize)]
struct RawBoltClass {
    class: String,
    #[serde(default)]
    max_d: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawStrengthTable {
    dataset: String,
    standard: StandardReference,
    bolt_classes: Vec<RawBoltClass>,
    nut_classes: Vec<String>,
}

/// Les classes de qualite embarquees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrengthTable {
    dataset: String,
    standard: StandardReference,
    bolt_classes: Vec<BoltClass>,
    nut_classes: Vec<u32>,
}

const STRENGTH_EMBEDDED: &str = include_str!("../../../data/visserie/iso898.classes-qualite.json");

impl StrengthTable {
    pub fn embedded() -> Result<&'static StrengthTable> {
        static CACHE: OnceLock<core::result::Result<StrengthTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| StrengthTable::parse(STRENGTH_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<StrengthTable> {
        let raw: RawStrengthTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso898.classes-qualite".into(),
                detail: source.to_string(),
            })?;
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: raw.dataset.clone(),
            detail,
        };
        let bolt_classes = raw
            .bolt_classes
            .iter()
            .map(|c| {
                let (first, second) = BoltClass::read(&c.class)
                    .ok_or_else(|| bad(format!("classe illisible : {}", c.class)))?;
                Ok(BoltClass {
                    class: c.class.clone(),
                    first,
                    second,
                    tensile_mpa: first * 100,
                    yield_mpa: first * second * 10,
                    max_d: c.max_d,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let nut_classes = raw
            .nut_classes
            .iter()
            .map(|n| {
                n.parse::<u32>()
                    .map_err(|_| bad(format!("classe d'ecrou illisible : {n}")))
            })
            .collect::<Result<Vec<_>>>()?;
        let table = StrengthTable {
            dataset: raw.dataset,
            standard: raw.standard,
            bolt_classes,
            nut_classes,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };
        for class in &self.bolt_classes {
            // Le rapport Re / Rm est strictement compris entre 0 et 1 : une
            // limite d'elasticite egale ou superieure a la rupture n'a pas de
            // sens.
            if !(1..=9).contains(&class.second) {
                return Err(bad(format!("{} : second nombre hors de 1..9", class.class)));
            }
        }
        for pair in self.bolt_classes.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if (b.tensile_mpa, b.yield_mpa) <= (a.tensile_mpa, a.yield_mpa) {
                return Err(bad(format!("{} ne suit pas {}", b.class, a.class)));
            }
        }
        if self.nut_classes.windows(2).any(|w| w[1] <= w[0]) {
            return Err(bad("classes d'ecrous non croissantes".into()));
        }
        // Toute vis doit trouver son ecrou.
        for class in &self.bolt_classes {
            if self.nut_for(class).is_none() {
                return Err(bad(format!("aucun ecrou pour la classe {}", class.class)));
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn bolt_classes(&self) -> &[BoltClass] {
        &self.bolt_classes
    }

    pub fn nut_classes(&self) -> &[u32] {
        &self.nut_classes
    }

    pub fn bolt_class(&self, class: &str) -> Option<&BoltClass> {
        self.bolt_classes.iter().find(|c| c.class == class.trim())
    }

    /// La plus petite classe d'ecrou au moins egale au premier nombre.
    pub fn nut_for(&self, bolt: &BoltClass) -> Option<u32> {
        self.nut_classes.iter().copied().find(|n| *n >= bolt.first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(text: &str) -> Length {
        Length::parse(text, Unit::Millimetre).unwrap()
    }

    fn threads() -> &'static ThreadTable {
        ThreadTable::embedded().expect("les filetages doivent charger")
    }

    fn holes() -> &'static ClearanceTable {
        ClearanceTable::embedded().expect("les trous de passage doivent charger")
    }

    fn strength() -> &'static StrengthTable {
        StrengthTable::embedded().expect("les classes de qualite doivent charger")
    }

    #[test]
    fn les_quatre_jeux_chargent_sans_se_dire_verifies() {
        for reference in [
            threads().standard(),
            BasicProfile::embedded().unwrap().standard(),
            holes().standard(),
            strength().standard(),
        ] {
            assert!(!reference.verification.is_verified(), "{}", reference.id);
        }
    }

    #[test]
    fn les_pas_gros_dusage() {
        let pitch = |d: &str| threads().by_diameter(mm(d)).unwrap().coarse;
        assert_eq!(pitch("6"), mm("1"));
        assert_eq!(pitch("8"), mm("1.25"));
        assert_eq!(pitch("10"), mm("1.5"));
        assert_eq!(pitch("12"), mm("1.75"));
        assert_eq!(pitch("24"), mm("3"));
        assert!(threads().by_diameter(mm("11")).is_none());
    }

    #[test]
    fn les_pas_fins_sont_plus_fins_que_le_pas_gros() {
        let mut broken: serde_json::Value = serde_json::from_str(THREADS_EMBEDDED).unwrap();
        broken["threads"][11]["fine"] = serde_json::json!([1.75]);
        let err = ThreadTable::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("pas fins"), "{err}");
    }

    #[test]
    fn le_profil_ordonne_ses_diametres() {
        let profile = BasicProfile::embedded().unwrap();
        assert_eq!(profile.diameter("d2").unwrap().h_fraction, [3, 4]);
        let mut broken: serde_json::Value = serde_json::from_str(PROFILE_EMBEDDED).unwrap();
        broken["diameters"][1]["h_fraction"] = serde_json::json!([1, 2]);
        let err = BasicProfile::parse(&broken.to_string()).unwrap_err();
        assert!(err.to_string().contains("davantage"), "{err}");
    }

    #[test]
    fn les_trous_de_passage_dusage() {
        let m10 = holes().by_diameter(mm("10")).unwrap();
        assert_eq!(
            (m10.fine, m10.medium, m10.coarse),
            (mm("10.5"), mm("11"), mm("12"))
        );
        let classes: Vec<&str> = holes().series().iter().map(|s| s.class.as_str()).collect();
        assert_eq!(classes, ["H12", "H13", "H14"]);
    }

    #[test]
    fn chaque_filetage_a_ses_trous() {
        let mut broken: serde_json::Value = serde_json::from_str(CLEARANCE_EMBEDDED).unwrap();
        broken["holes"].as_array_mut().unwrap().pop();
        let err = ClearanceTable::parse(&broken.to_string(), threads()).unwrap_err();
        assert!(err.to_string().contains("memes diametres"), "{err}");
    }

    #[test]
    fn des_series_dans_le_desordre_sont_refusees() {
        let mut broken: serde_json::Value = serde_json::from_str(CLEARANCE_EMBEDDED).unwrap();
        broken["holes"][9]["medium"] = serde_json::json!(6.3);
        let err = ClearanceTable::parse(&broken.to_string(), threads()).unwrap_err();
        assert!(err.to_string().contains("desordre"), "{err}");
    }

    #[test]
    fn la_classe_se_lit_selon_la_regle() {
        let c = strength().bolt_class("8.8").unwrap();
        assert_eq!((c.tensile_mpa, c.yield_mpa), (800, 640));
        let c = strength().bolt_class("10.9").unwrap();
        assert_eq!((c.tensile_mpa, c.yield_mpa), (1000, 900));
        let c = strength().bolt_class("4.6").unwrap();
        assert_eq!((c.tensile_mpa, c.yield_mpa), (400, 240));
        assert!(strength().bolt_class("7.7").is_none());
    }

    #[test]
    fn chaque_vis_trouve_son_ecrou() {
        let nut = |c: &str| strength().nut_for(strength().bolt_class(c).unwrap());
        assert_eq!(nut("4.6"), Some(5));
        assert_eq!(nut("8.8"), Some(8));
        assert_eq!(nut("9.8"), Some(10));
        assert_eq!(nut("12.9"), Some(12));
    }

    #[test]
    fn la_classe_9_8_sarrete_a_m16() {
        assert_eq!(strength().bolt_class("9.8").unwrap().max_d, Some(16));
    }
}
