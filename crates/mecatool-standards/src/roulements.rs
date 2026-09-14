//! Roulements : symbole d'alesage, et classes de tolerance de montage.
//!
//! Ce module porte deux jeux de donnees de **natures differentes**, et la
//! difference est le point important.
//!
//! Le **symbole d'alesage** vient de l'ISO 15, relayee par un recueil. C'est une
//! regle normative, et c'est une *regle* et non une table : la source l'enonce
//! en une phrase, MecaTool l'applique au lieu de recopier quatre-vingt-seize
//! correspondances. Seuls les quatre codes speciaux s'enumerent, parce qu'ils
//! echappent justement a la regle.
//!
//! Les **classes de tolerance de montage** ne viennent d'aucune norme. Le
//! tableau qui les donne porte la mention « Dimensions du fabricant » : ce sont
//! des pratiques recommandees. Elles portent donc
//! [`VerificationStatus::Recommended`], dont le message dit que s'en ecarter
//! reste legitime.
//!
//! [`VerificationStatus::Recommended`]: mecatool_core::VerificationStatus::Recommended

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use mecatool_core::{Length, StandardReference};
use serde::{Deserialize, Serialize};

use crate::error::{Result, StandardsError};

/* ------------------------------------------------------------------ */
/*  Symbole d'alesage — ISO 15                                         */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct DirectRule {
    max_bore_mm: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SpecialCode {
    code: String,
    bore_mm: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MultipliedRule {
    min_code: i64,
    max_code: i64,
    factor: i64,
}

/// La regle de correspondance entre symbole d'alesage et diametre.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BoreDesignation {
    dataset: String,
    standard: StandardReference,
    direct: DirectRule,
    special_codes: Vec<SpecialCode>,
    multiplied: MultipliedRule,
}

const BORE_EMBEDDED: &str = include_str!("../../../data/roulements/iso15.bore-designation.json");

impl BoreDesignation {
    pub fn embedded() -> Result<&'static BoreDesignation> {
        static CACHE: OnceLock<core::result::Result<BoreDesignation, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| BoreDesignation::parse(BORE_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<BoreDesignation> {
        let rule: BoreDesignation =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso15.bore-designation".into(),
                detail: source.to_string(),
            })?;
        rule.validate()?;
        Ok(rule)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        if self.special_codes.is_empty() {
            return Err(bad("aucun code special".into()));
        }
        // Les codes speciaux doivent etre croissants et sans doublon : deux
        // codes rendant le meme diametre, ou l'inverse, rendrait la lecture
        // d'une designation ambigue sans que rien ne le signale.
        let mut previous = 0;
        let mut seen = BTreeSet::new();
        for special in &self.special_codes {
            if !seen.insert(special.code.as_str()) {
                return Err(bad(format!("code special en double : {}", special.code)));
            }
            if special.bore_mm <= previous {
                return Err(bad(format!(
                    "les codes speciaux ne croissent pas : {} apres {previous}",
                    special.bore_mm
                )));
            }
            previous = special.bore_mm;
        }

        // La regle multipliee doit prendre le relais au-dessus des codes
        // speciaux, sans trou ni recouvrement.
        if self.multiplied.factor <= 0 {
            return Err(bad("facteur nul ou negatif".into()));
        }
        if self.multiplied.min_code <= 0 || self.multiplied.max_code < self.multiplied.min_code {
            return Err(bad("plage de codes multiplies incoherente".into()));
        }
        let first_multiplied = self.multiplied.min_code * self.multiplied.factor;
        if first_multiplied <= previous {
            return Err(bad(format!(
                "la regle multipliee commence a {first_multiplied} mm, \
                 qui n'est pas au-dessus du dernier code special ({previous} mm)"
            )));
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    /// Le plus grand diametre que la regle couvre.
    pub fn max_bore(&self) -> Length {
        Length::from_millimetres(self.multiplied.max_code * self.multiplied.factor)
    }

    /// Le diametre d'alesage que designe un symbole.
    ///
    /// Trois branches, exactement celles que la source enonce : le symbole vaut
    /// le diametre jusqu'a 9 mm, quatre codes a deux chiffres sont speciaux, et
    /// au-dela le symbole vaut le cinquieme du diametre.
    pub fn bore_diameter(&self, code: &str) -> Result<Length> {
        let code = code.trim();

        let unknown = |reason: &str| StandardsError::Malformed {
            dataset: self.dataset.clone(),
            detail: format!("symbole d'alésage « {code} » : {reason}"),
        };

        if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(unknown("un symbole d'alésage ne porte que des chiffres"));
        }

        // Les codes speciaux se reconnaissent a leur ecriture exacte, deux
        // chiffres compris. « 0 » n'est pas « 00 » : le premier designerait un
        // alesage nul, le second un alesage de 10 mm.
        if let Some(special) = self.special_codes.iter().find(|s| s.code == code) {
            return Ok(Length::from_millimetres(special.bore_mm));
        }

        let value: i64 = code.parse().map_err(|_| unknown("nombre illisible"))?;

        if code.len() == 1 {
            if value == 0 {
                return Err(unknown("un alésage nul n'existe pas"));
            }
            if value <= self.direct.max_bore_mm {
                return Ok(Length::from_millimetres(value));
            }
            return Err(unknown("hors de la plage des symboles à un chiffre"));
        }

        if value < self.multiplied.min_code {
            return Err(unknown(
                "code à deux chiffres inférieur au premier code multiplié, \
                 et qui ne figure pas parmi les codes spéciaux",
            ));
        }
        if value > self.multiplied.max_code {
            return Err(StandardsError::Malformed {
                dataset: self.dataset.clone(),
                detail: format!(
                    "symbole d'alésage « {code} » : au-delà du symbole {}, la source \
                     ne définit rien. MecaTool refuse de prolonger la règle.",
                    self.multiplied.max_code
                ),
            });
        }
        Ok(Length::from_millimetres(value * self.multiplied.factor))
    }

    /// Le symbole qui designe un diametre, quand il en existe un.
    ///
    /// L'operation inverse n'est pas totale : tous les diametres ne s'ecrivent
    /// pas. 11 mm n'a pas de symbole, 25 mm en a un.
    pub fn code_for(&self, bore: Length) -> Option<String> {
        let mm = bore.to_millimetres_exact()?;
        if let Some(special) = self.special_codes.iter().find(|s| s.bore_mm == mm) {
            return Some(special.code.clone());
        }
        if (1..=self.direct.max_bore_mm).contains(&mm) {
            return Some(mm.to_string());
        }
        if mm % self.multiplied.factor == 0 {
            let code = mm / self.multiplied.factor;
            if (self.multiplied.min_code..=self.multiplied.max_code).contains(&code) {
                return Some(format!("{code:02}"));
            }
        }
        None
    }
}

/* ------------------------------------------------------------------ */
/*  Classes de tolerance de montage — recommandation fabricant         */
/* ------------------------------------------------------------------ */

/// Un echelon de diametre, borne basse exclue et borne haute incluse.
///
/// Meme convention que les echelons de l'ISO 286 : « au-dessus de 18 jusqu'a
/// 100 ». Une borne haute absente signifie un echelon ouvert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiameterRange {
    pub from: i64,
    #[serde(default)]
    pub to: Option<i64>,
}

impl DiameterRange {
    pub fn contains(&self, diameter: Length) -> bool {
        let Some(mm) = diameter.to_millimetres_exact() else {
            // Un diametre non entier tombe dans l'echelon dont il respecte les
            // bornes : on compare alors en nanometres.
            let from = Length::from_millimetres(self.from);
            let above = diameter > from;
            return match self.to {
                Some(to) => above && diameter <= Length::from_millimetres(to),
                None => above,
            };
        };
        let above = mm > self.from;
        match self.to {
            Some(to) => above && mm <= to,
            None => above,
        }
    }

    pub fn label_fr(&self) -> String {
        match self.to {
            Some(to) if self.from == 0 => format!("jusqu'à {to} mm"),
            Some(to) => format!("au-dessus de {} jusqu'à {to} mm", self.from),
            None => format!("au-dessus de {} mm", self.from),
        }
    }
}

/// Une famille de roulements, au sens de la colonne du tableau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BearingFamily {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Le regime de charge de la bague interieure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRegime {
    pub id: String,
    pub name: String,
    /// Pourquoi ce regime impose ce qu'il impose.
    pub explanation: String,
}

/// Une ligne du tableau : une classe, et les echelons ou elle s'applique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountingRow {
    pub class: String,
    /// Vrai quand la ligne vaut quel que soit le diametre.
    #[serde(default)]
    pub all_diameters: bool,
    /// Echelon par famille de roulement. Une famille absente signifie que la
    /// source ne definit rien pour elle sur cette ligne.
    #[serde(default)]
    pub ranges: BTreeMap<String, DiameterRange>,
}

/// Un cas d'emploi : un regime, une condition, et ses lignes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountingCase {
    pub regime: String,
    pub condition: String,
    pub examples: String,
    pub rows: Vec<MountingRow>,
}

/// Le tableau des classes de tolerance de portee d'arbre.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ShaftMountingTable {
    dataset: String,
    standard: StandardReference,
    families: Vec<BearingFamily>,
    load_regimes: Vec<LoadRegime>,
    cases: Vec<MountingCase>,
}

const MOUNTING_EMBEDDED: &str = include_str!("../../../data/roulements/montage.classes-arbre.json");

impl ShaftMountingTable {
    pub fn embedded() -> Result<&'static ShaftMountingTable> {
        static CACHE: OnceLock<core::result::Result<ShaftMountingTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| ShaftMountingTable::parse(MOUNTING_EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<ShaftMountingTable> {
        let table: ShaftMountingTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "roulements.montage-arbre".into(),
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

        let families: BTreeSet<&str> = self.families.iter().map(|f| f.id.as_str()).collect();
        let regimes: BTreeSet<&str> = self.load_regimes.iter().map(|r| r.id.as_str()).collect();
        if families.len() != self.families.len() {
            return Err(bad("une famille est decrite deux fois".into()));
        }
        if self.cases.is_empty() {
            return Err(bad("aucun cas d'emploi".into()));
        }

        for case in &self.cases {
            if !regimes.contains(case.regime.as_str()) {
                return Err(bad(format!("regime inconnu : {}", case.regime)));
            }
            if case.rows.is_empty() {
                return Err(bad(format!(
                    "le cas « {} » n'a aucune ligne",
                    case.condition
                )));
            }

            // Deux echelons d'une meme famille ne peuvent pas se recouvrir a
            // l'interieur d'un cas : un diametre rendrait alors deux classes,
            // et le moteur en choisirait une au hasard.
            let mut seen: BTreeMap<&str, Vec<DiameterRange>> = BTreeMap::new();
            for row in &case.rows {
                if row.all_diameters && !row.ranges.is_empty() {
                    return Err(bad(format!(
                        "la ligne {} vaut « tous diametres » ET porte des echelons",
                        row.class
                    )));
                }
                if !row.all_diameters && row.ranges.is_empty() {
                    return Err(bad(format!(
                        "la ligne {} ne couvre aucun diametre",
                        row.class
                    )));
                }
                for (family, range) in &row.ranges {
                    if !families.contains(family.as_str()) {
                        return Err(bad(format!("famille inconnue : {family}")));
                    }
                    if let Some(to) = range.to {
                        if to <= range.from {
                            return Err(bad(format!(
                                "echelon vide pour {family} sur la ligne {}",
                                row.class
                            )));
                        }
                    }
                    let entry = seen.entry(family.as_str()).or_default();
                    if entry.iter().any(|other| overlaps(other, range)) {
                        return Err(bad(format!(
                            "echelons qui se recouvrent pour {family} dans le cas « {} »",
                            case.condition
                        )));
                    }
                    entry.push(*range);
                }
            }
        }
        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn families(&self) -> &[BearingFamily] {
        &self.families
    }

    pub fn regimes(&self) -> &[LoadRegime] {
        &self.load_regimes
    }

    pub fn cases(&self) -> &[MountingCase] {
        &self.cases
    }

    pub fn family(&self, id: &str) -> Option<&BearingFamily> {
        self.families.iter().find(|f| f.id == id)
    }

    pub fn regime(&self, id: &str) -> Option<&LoadRegime> {
        self.load_regimes.iter().find(|r| r.id == id)
    }

    /// Les cas d'emploi d'un regime donne, dans l'ordre du tableau.
    pub fn cases_for(&self, regime: &str) -> Vec<&MountingCase> {
        self.cases.iter().filter(|c| c.regime == regime).collect()
    }

    /// La classe recommandee pour un cas, une famille et un diametre.
    ///
    /// Rend `None` quand la source ne definit rien : c'est un refus explicite,
    /// pas une absence de reponse. Le tableau laisse volontairement des cases
    /// vides, et les combler serait inventer une recommandation.
    pub fn class_for<'a>(
        &self,
        case: &'a MountingCase,
        family: &str,
        diameter: Length,
    ) -> Option<&'a MountingRow> {
        case.rows.iter().find(|row| {
            if row.all_diameters {
                return true;
            }
            row.ranges
                .get(family)
                .is_some_and(|range| range.contains(diameter))
        })
    }
}

fn overlaps(a: &DiameterRange, b: &DiameterRange) -> bool {
    let a_to = a.to.unwrap_or(i64::MAX);
    let b_to = b.to.unwrap_or(i64::MAX);
    a.from < b_to && b.from < a_to
}

/// Les millimetres entiers d'une longueur, quand elle en est un compte exact.
trait ExactMillimetres {
    fn to_millimetres_exact(self) -> Option<i64>;
}

impl ExactMillimetres for Length {
    fn to_millimetres_exact(self) -> Option<i64> {
        let per_mm = Length::from_millimetres(1).nanometres();
        let nm = self.nanometres();
        (nm % per_mm == 0).then_some(nm / per_mm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mecatool_core::VerificationStatus;

    fn bore() -> &'static BoreDesignation {
        BoreDesignation::embedded().expect("la regle d'alesage doit charger")
    }

    fn mounting() -> &'static ShaftMountingTable {
        ShaftMountingTable::embedded().expect("le tableau de montage doit charger")
    }

    fn mm(value: i64) -> Length {
        Length::from_millimetres(value)
    }

    #[test]
    fn les_quatre_codes_speciaux() {
        // Ce sont les seuls qui echappent a la regle, d'ou leur enumeration.
        assert_eq!(bore().bore_diameter("00").unwrap(), mm(10));
        assert_eq!(bore().bore_diameter("01").unwrap(), mm(12));
        assert_eq!(bore().bore_diameter("02").unwrap(), mm(15));
        assert_eq!(bore().bore_diameter("03").unwrap(), mm(17));
    }

    #[test]
    fn le_symbole_vaut_le_diametre_jusqua_neuf() {
        assert_eq!(bore().bore_diameter("1").unwrap(), mm(1));
        assert_eq!(bore().bore_diameter("9").unwrap(), mm(9));
    }

    #[test]
    fn au_dela_le_symbole_vaut_le_cinquieme_du_diametre() {
        assert_eq!(bore().bore_diameter("04").unwrap(), mm(20));
        assert_eq!(bore().bore_diameter("10").unwrap(), mm(50));
        assert_eq!(bore().bore_diameter("96").unwrap(), mm(480));
    }

    #[test]
    fn zero_seul_nest_pas_double_zero() {
        // « 00 » vaut 10 mm, « 0 » ne vaut rien : un alesage nul n'existe pas.
        // Confondre les deux ferait rendre 10 mm pour une saisie fautive.
        assert_eq!(bore().bore_diameter("00").unwrap(), mm(10));
        assert!(bore().bore_diameter("0").is_err());
    }

    #[test]
    fn la_regle_nest_pas_prolongee_au_dela_de_la_source() {
        let err = bore().bore_diameter("97").unwrap_err();
        assert!(err.to_string().contains("refuse de prolonger"), "{err}");
        assert_eq!(bore().max_bore(), mm(480));
    }

    #[test]
    fn un_symbole_illisible_est_refuse() {
        assert!(bore().bore_diameter("6A").is_err());
        assert!(bore().bore_diameter("").is_err());
    }

    #[test]
    fn le_symbole_inverse_nexiste_pas_toujours() {
        assert_eq!(bore().code_for(mm(17)).as_deref(), Some("03"));
        assert_eq!(bore().code_for(mm(50)).as_deref(), Some("10"));
        assert_eq!(bore().code_for(mm(9)).as_deref(), Some("9"));
        // 11 mm ne s'ecrit pas : ni code special, ni multiple de cinq.
        assert_eq!(bore().code_for(mm(11)), None);
        assert_eq!(bore().code_for(mm(500)), None);
    }

    #[test]
    fn la_regle_dalesage_vient_dun_recueil() {
        let verification = &bore().standard().verification;
        assert!(verification.is_secondary());
        assert!(!verification.is_verified());
    }

    #[test]
    fn la_regle_dalesage_ne_sattribue_a_aucune_norme() {
        // Elle a porte l'identifiant « ISO 15 », parce que le recueil l'enonce
        // sous un titre citant cette norme. L'ISO 15:2011, lue depuis, ne
        // contient aucun symbole d'alesage : ses tableaux donnent le diametre
        // directement. L'attribution etait donc fausse.
        //
        // Ce test empeche qu'elle revienne. Citer une norme qui ne dit pas ce
        // qu'on lui fait dire est pire que ne citer personne.
        let standard = bore().standard();
        assert!(
            !standard.id.contains("ISO 15"),
            "la regle de designation ne vient pas de l'ISO 15 : {}",
            standard.id
        );
        match &standard.verification {
            VerificationStatus::Secondary { reproduces, .. } => {
                assert!(
                    !reproduces.starts_with("ISO 15"),
                    "le champ `reproduces` ne doit pas nommer l'ISO 15 comme \
                     source de la regle : {reproduces}"
                );
            }
            other => panic!("statut inattendu : {other:?}"),
        }
    }

    #[test]
    fn le_tableau_de_montage_est_une_recommandation_pas_une_norme() {
        // Le coeur de ce jeu de donnees : il ne vient d'aucune norme, et le
        // confondre avec une exigence tromperait celui qui doit decider.
        let verification = &mounting().standard().verification;
        assert!(verification.is_recommended());
        assert!(!verification.is_verified());
        assert!(!verification.is_secondary());

        let banner = verification.banner_fr().unwrap();
        assert!(banner.contains("sans caractère normatif"));
        assert!(banner.contains("fabricants de roulements"));
    }

    #[test]
    fn le_tableau_charge_et_se_valide() {
        assert_eq!(mounting().families().len(), 4);
        assert_eq!(mounting().regimes().len(), 2);
        assert_eq!(mounting().cases().len(), 5);
    }

    #[test]
    fn une_charge_fixe_vaut_pour_tous_les_diametres() {
        let cases = mounting().cases_for("stationary_inner");
        assert_eq!(cases.len(), 2);
        for case in cases {
            let row = mounting()
                .class_for(case, "ball_radial", mm(500))
                .expect("« tous diametres » ne connait pas de borne");
            assert!(row.all_diameters);
        }
        // Roue folle : la bague doit coulisser, donc jeu.
        let libre = mounting().cases_for("stationary_inner")[0];
        assert_eq!(
            mounting()
                .class_for(libre, "ball_radial", mm(50))
                .unwrap()
                .class,
            "g6"
        );
    }

    #[test]
    fn la_classe_se_resserre_quand_la_charge_grandit() {
        let t = mounting();
        let normales = t
            .cases_for("rotating_inner")
            .into_iter()
            .find(|c| c.condition.contains("normales"))
            .unwrap();

        // Un roulement a billes de 50 mm sous charge normale : k5.
        assert_eq!(
            t.class_for(normales, "ball_radial", mm(50)).unwrap().class,
            "k5"
        );
        // Le meme plus gros se resserre encore.
        assert_eq!(
            t.class_for(normales, "ball_radial", mm(120)).unwrap().class,
            "m5"
        );
        assert_eq!(
            t.class_for(normales, "ball_radial", mm(160)).unwrap().class,
            "m6"
        );
        // Et le plus petit se relache.
        assert_eq!(
            t.class_for(normales, "ball_radial", mm(10)).unwrap().class,
            "js5"
        );
    }

    #[test]
    fn les_bornes_dechelon_suivent_la_convention_iso() {
        let t = mounting();
        let normales = t
            .cases_for("rotating_inner")
            .into_iter()
            .find(|c| c.condition.contains("normales"))
            .unwrap();

        // « jusqu'a 18 » inclut 18 ; « au-dessus de 18 » l'exclut.
        assert_eq!(
            t.class_for(normales, "ball_radial", mm(18)).unwrap().class,
            "js5"
        );
        assert_eq!(
            t.class_for(normales, "ball_radial", mm(19)).unwrap().class,
            "k5"
        );
    }

    #[test]
    fn une_case_vide_du_tableau_ne_se_comble_pas() {
        let t = mounting();
        let faibles = t
            .cases_for("rotating_inner")
            .into_iter()
            .find(|c| c.condition.contains("faibles"))
            .unwrap();

        // Sous charges faibles, la source ne definit rien pour les roulements a
        // rotule sur rouleaux. Rendre None est un refus explicite ; rendre une
        // classe voisine serait inventer une recommandation.
        assert!(t.class_for(faibles, "spherical_roller", mm(50)).is_none());

        // Ni pour un roulement a billes en dessous de 18 mm dans ce cas.
        assert!(t.class_for(faibles, "ball_radial", mm(10)).is_none());
    }

    #[test]
    fn un_echelon_ouvert_na_pas_de_borne_haute() {
        let t = mounting();
        let normales = t
            .cases_for("rotating_inner")
            .into_iter()
            .find(|c| c.condition.contains("normales"))
            .unwrap();
        // « au-dessus de 150 » pour les aiguilles : pas de borne haute.
        assert_eq!(
            t.class_for(normales, "needle_with_inner_ring", mm(400))
                .unwrap()
                .class,
            "m6"
        );
    }

    #[test]
    fn chaque_regime_explique_ce_quil_impose() {
        for regime in mounting().regimes() {
            assert!(!regime.explanation.is_empty(), "{}", regime.id);
        }
        // Le regime tournant doit dire POURQUOI il faut serrer : c'est ce qui
        // permet a l'utilisateur de juger s'il peut s'en ecarter.
        let tournant = mounting().regime("rotating_inner").unwrap();
        assert!(tournant.explanation.contains("ramper"));
    }

    #[test]
    fn la_famille_aiguilles_porte_la_reserve_de_la_source() {
        let famille = mounting().family("needle_with_inner_ring").unwrap();
        let note = famille.note.as_deref().unwrap();
        assert!(note.contains("h5"));
        assert!(note.contains("sans bague intérieure"));
    }

    #[test]
    fn un_tableau_aux_echelons_recouvrants_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(MOUNTING_EMBEDDED).unwrap();
        // On fait deborder le premier echelon sur le suivant.
        broken["cases"][3]["rows"][0]["ranges"]["ball_radial"]["to"] = serde_json::json!(200);
        let err = ShaftMountingTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. }
                     if detail.contains("recouvrent")),
            "{err}"
        );
    }

    #[test]
    fn une_ligne_sans_diametre_est_refusee() {
        let mut broken: serde_json::Value = serde_json::from_str(MOUNTING_EMBEDDED).unwrap();
        broken["cases"][2]["rows"][0]["ranges"] = serde_json::json!({});
        let err = ShaftMountingTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. }
                     if detail.contains("aucun diametre")),
            "{err}"
        );
    }

    #[test]
    fn le_statut_recommande_se_relit_depuis_le_json() {
        match &mounting().standard().verification {
            VerificationStatus::Recommended { by, .. } => {
                assert!(by.contains("fabricants"));
                assert!(by.contains("237/1"));
            }
            other => panic!("statut inattendu : {other:?}"),
        }
    }
}
