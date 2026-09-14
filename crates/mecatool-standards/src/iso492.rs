//! Tolerances dimensionnelles des roulements radiaux — ISO 492.
//!
//! # Ce que ce module ferme
//!
//! MecaTool savait recommander une classe de tolerance pour l'arbre — k5, m6 —
//! et s'arretait la. Il ignorait la tolerance propre de l'alesage du roulement,
//! et ne pouvait donc pas dire quel serrage en resultait.
//!
//! Un alesage de roulement n'est pas `h0`. Il porte un ecart normalise, dont
//! l'ecart superieur vaut **toujours zero** et l'inferieur toujours negatif :
//! le diametre moyen ne depasse jamais le nominal. C'est precisement ce qui rend
//! un montage serre possible avec un arbre en k ou en m.
//!
//! # Ce qu'il ne couvre pas, et le dit
//!
//! Une seule classe de tolerance sur les cinq — la Normale — et un seul
//! caracteristique par bague : l'ecart du diametre moyen, celui qui determine
//! l'ajustement. Les variations de forme et de largeur des memes tableaux
//! decrivent autre chose.

use std::sync::OnceLock;

use mecatool_core::{Deviations, Length, StandardReference};
use serde::{Deserialize, Serialize};

use crate::error::{Result, StandardsError};

/// La bague dont on veut la tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ring {
    /// Bague interieure : l'alesage, qui repose sur l'arbre.
    Inner,
    /// Bague exterieure : le diametre exterieur, qui entre dans le logement.
    Outer,
}

impl Ring {
    pub const fn name_fr(self) -> &'static str {
        match self {
            Ring::Inner => "bague intérieure",
            Ring::Outer => "bague extérieure",
        }
    }

    /// Le symbole normatif de la caracteristique.
    pub const fn symbol(self) -> &'static str {
        match self {
            Ring::Inner => "Δdmp",
            Ring::Outer => "ΔDmp",
        }
    }
}

/// Une ligne telle qu'elle se lit dans le fichier.
///
/// Les bornes d'echelon s'y ecrivent en millimetres decimaux — « 0,6 », « 2,5 »
/// — parce que c'est ainsi que la norme les imprime. Elles sont converties en
/// nanometres entiers des le chargement : **aucun flottant ne survit dans la
/// table chargee**, conformement au principe d'exactitude du projet.
#[derive(Debug, Clone, Copy, Deserialize)]
struct RawRange {
    from: f64,
    to: f64,
    upper: i64,
    lower: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawRingTable {
    characteristic: String,
    meaning: String,
    ranges: Vec<RawRange>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTable {
    dataset: String,
    standard: StandardReference,
    tolerance_class: String,
    bore: RawRingTable,
    outside: RawRingTable,
}

/// Un echelon, une fois converti en grandeurs exactes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Echelon {
    /// Borne basse, exclue.
    from: Length,
    /// Borne haute, incluse.
    to: Length,
    deviations: Deviations,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RingTable {
    characteristic: String,
    meaning: String,
    echelons: Vec<Echelon>,
}

/// Les tolerances de la classe Normale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearingToleranceTable {
    dataset: String,
    standard: StandardReference,
    tolerance_class: String,
    bore: RingTable,
    outside: RingTable,
}

/// Ce que la norme dit d'une bague, a un diametre donne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingTolerance {
    pub ring: Ring,
    /// Le diametre nominal interroge.
    pub nominal: Length,
    /// Les ecarts, par rapport au nominal.
    pub deviations: Deviations,
    /// Le symbole normatif, par ex. `"Δdmp"`.
    pub characteristic: String,
    /// Ce que ce symbole designe, en clair.
    pub meaning: String,
    /// L'echelon employe, tel qu'il s'ecrit.
    pub range_label: String,
    /// La classe de tolerance du roulement.
    pub tolerance_class: String,
}

const EMBEDDED: &str = include_str!("../../../data/roulements/iso492-2014.tolerances.json");

impl BearingToleranceTable {
    pub fn embedded() -> Result<&'static BearingToleranceTable> {
        static CACHE: OnceLock<core::result::Result<BearingToleranceTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| BearingToleranceTable::parse(EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Lit le fichier et convertit toutes ses bornes en grandeurs exactes.
    fn parse(text: &str) -> Result<BearingToleranceTable> {
        let raw: RawTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso492-2014.tolerances".into(),
                detail: source.to_string(),
            })?;

        let table = BearingToleranceTable {
            bore: convert(&raw.dataset, &raw.bore)?,
            outside: convert(&raw.dataset, &raw.outside)?,
            dataset: raw.dataset,
            standard: raw.standard,
            tolerance_class: raw.tolerance_class,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        for (name, table) in [("bore", &self.bore), ("outside", &self.outside)] {
            if table.echelons.is_empty() {
                return Err(bad(format!("{name} : aucun echelon")));
            }
            for (previous, next) in table.echelons.iter().zip(table.echelons.iter().skip(1)) {
                // Les echelons doivent se toucher : un trou ferait refuser un
                // diametre parfaitement normal, sans que rien ne le signale.
                if previous.to != next.from {
                    return Err(bad(format!(
                        "{name} : trou entre {} et {}",
                        previous.to, next.from
                    )));
                }
                // La tolerance s'elargit avec la taille. Une inversion
                // trahirait une ligne recopiee de travers.
                if next.deviations.lower() > previous.deviations.lower() {
                    return Err(bad(format!(
                        "{name} : la tolerance se resserre entre {} et {}",
                        previous.to, next.to
                    )));
                }
            }
            for echelon in &table.echelons {
                // L'ecart superieur nul n'est pas une commodite : c'est ce qui
                // rend un montage serre possible avec un arbre en k ou m. Une
                // valeur positive trahirait une ligne mal lue.
                if echelon.deviations.upper() != Length::ZERO {
                    return Err(bad(format!(
                        "{name} : ecart superieur non nul a l'echelon {} — {}",
                        echelon.from, echelon.to
                    )));
                }
                if echelon.deviations.lower() >= Length::ZERO {
                    return Err(bad(format!(
                        "{name} : ecart inferieur non negatif a l'echelon {} — {}",
                        echelon.from, echelon.to
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn tolerance_class(&self) -> &str {
        &self.tolerance_class
    }

    /// Le plus grand diametre couvert, pour la bague donnee.
    pub fn max_nominal(&self, ring: Ring) -> Length {
        let table = self.table(ring);
        table.echelons[table.echelons.len() - 1].to
    }

    fn table(&self, ring: Ring) -> &RingTable {
        match ring {
            Ring::Inner => &self.bore,
            Ring::Outer => &self.outside,
        }
    }

    /// Les ecarts d'une bague, au diametre nominal donne.
    pub fn tolerance(&self, ring: Ring, nominal: Length) -> Result<RingTolerance> {
        let table = self.table(ring);
        let echelon = table
            .echelons
            .iter()
            .find(|e| nominal > e.from && nominal <= e.to)
            .ok_or_else(|| StandardsError::NominalOutOfRange {
                dataset: self.dataset.clone(),
                nominal: nominal.to_string(),
                min: table.echelons[0].from.to_string(),
                max: table.echelons[table.echelons.len() - 1].to.to_string(),
            })?;

        Ok(RingTolerance {
            ring,
            nominal,
            deviations: echelon.deviations,
            characteristic: table.characteristic.clone(),
            meaning: table.meaning.clone(),
            range_label: format!("au-dessus de {} jusqu'à {}", echelon.from, echelon.to),
            tolerance_class: self.tolerance_class.clone(),
        })
    }
}

/// Convertit une table lue en table exacte.
fn convert(dataset: &str, raw: &RawRingTable) -> Result<RingTable> {
    let mut echelons = Vec::with_capacity(raw.ranges.len());
    for range in &raw.ranges {
        let deviations = Deviations::new(
            Length::from_micrometres(range.lower),
            Length::from_micrometres(range.upper),
        )
        .map_err(|source| StandardsError::Inconsistent {
            dataset: dataset.to_string(),
            detail: source.to_string(),
        })?;
        echelons.push(Echelon {
            from: millimetres(range.from),
            to: millimetres(range.to),
            deviations,
        });
    }
    Ok(RingTable {
        characteristic: raw.characteristic.clone(),
        meaning: raw.meaning.clone(),
        echelons,
    })
}

fn millimetres(value: f64) -> Length {
    // Les bornes du tableau sont des dixiemes au plus fin (0,6 / 2,5), donc
    // exactement representables en nanometres entiers.
    Length::from_nanometres((value * 1_000_000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> &'static BearingToleranceTable {
        BearingToleranceTable::embedded().expect("les tolerances ISO 492 doivent charger")
    }

    fn mm(value: i64) -> Length {
        Length::from_millimetres(value)
    }

    #[test]
    fn la_table_charge_et_se_valide() {
        assert_eq!(table().tolerance_class(), "Normale");
        assert_eq!(table().max_nominal(Ring::Inner), mm(2000));
        assert_eq!(table().max_nominal(Ring::Outer), mm(2500));
    }

    #[test]
    fn la_source_est_la_norme_elle_meme() {
        let verification = &table().standard().verification;
        assert!(verification.is_verified());
        assert!(verification.banner_fr().is_none());
        assert_eq!(table().standard().citation(), "ISO 492:2014");
    }

    #[test]
    fn lalesage_dun_roulement_de_cinquante() {
        // Le cas qui ferme la boucle : un 6210 a un alesage de 50 mm, qui tombe
        // dans l'echelon « au-dessus de 30 jusqu'a 50 ».
        let t = table().tolerance(Ring::Inner, mm(50)).unwrap();
        assert_eq!(t.deviations.upper(), Length::ZERO);
        assert_eq!(t.deviations.lower(), Length::from_micrometres(-12));
        assert_eq!(t.characteristic, "Δdmp");
        assert!(t.range_label.contains("30"));
    }

    #[test]
    fn lecart_superieur_est_toujours_nul() {
        // C'est ce qui rend un montage serre possible avec un arbre en k ou m :
        // l'alesage d'un roulement ne depasse jamais son nominal.
        for ring in [Ring::Inner, Ring::Outer] {
            for d in [1, 20, 50, 120, 500, 1000] {
                let t = table().tolerance(ring, mm(d)).unwrap();
                assert_eq!(t.deviations.upper(), Length::ZERO, "{ring:?} a {d} mm");
                assert!(t.deviations.lower() < Length::ZERO, "{ring:?} a {d} mm");
            }
        }
    }

    #[test]
    fn les_deux_bagues_nont_pas_les_memes_echelons() {
        // L'alesage distingue 2,5 a 10 puis 10 a 18 ; l'exterieur distingue
        // 2,5 a 6 puis 6 a 18. Et l'exterieur coupe a 150 la ou l'interieur va
        // d'un trait de 120 a 180. Fusionner les deux tables donnerait des
        // valeurs fausses pres de ces bornes.
        let inner = table().tolerance(Ring::Inner, mm(140)).unwrap();
        let outer = table().tolerance(Ring::Outer, mm(140)).unwrap();
        assert_eq!(inner.deviations.lower(), Length::from_micrometres(-25));
        assert_eq!(outer.deviations.lower(), Length::from_micrometres(-18));
    }

    #[test]
    fn les_bornes_dechelon_suivent_la_convention() {
        // « au-dessus de 30 jusqu'a 50 » : 50 est dedans, 50,001 est dehors.
        let at = table().tolerance(Ring::Inner, mm(50)).unwrap();
        let above = table()
            .tolerance(Ring::Inner, Length::from_nanometres(50_000_001))
            .unwrap();
        assert_eq!(at.deviations.lower(), Length::from_micrometres(-12));
        assert_eq!(above.deviations.lower(), Length::from_micrometres(-15));
    }

    #[test]
    fn un_diametre_hors_table_est_refuse() {
        // Plutot que d'extrapoler le dernier echelon.
        let err = table().tolerance(Ring::Inner, mm(3000)).unwrap_err();
        assert!(
            matches!(err, StandardsError::NominalOutOfRange { .. }),
            "{err}"
        );
    }

    #[test]
    fn une_table_dont_la_tolerance_se_resserre_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(EMBEDDED).unwrap();
        broken["bore"]["ranges"][10]["lower"] = serde_json::json!(-5);
        let err = BearingToleranceTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. }
                     if detail.contains("se resserre")),
            "{err}"
        );
    }

    #[test]
    fn une_table_a_trou_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(EMBEDDED).unwrap();
        broken["outside"]["ranges"][3]["to"] = serde_json::json!(28);
        let err = BearingToleranceTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. }
                     if detail.contains("trou")),
            "{err}"
        );
    }
}
