//! Caracteristiques geometriques de l'ISO 1101.
//!
//! # Ce que ce module contient, et ce qu'il ne contient pas
//!
//! L'ISO 1101 ne donne **aucune valeur de tolerance**. Elle definit un
//! vocabulaire : quatorze caracteristiques, leurs symboles, la forme de la zone
//! que chacune delimite, et si une reference specifiee est requise. La valeur,
//! elle, est choisie par le concepteur en fonction de la piece.
//!
//! Ce module ne propose donc jamais de valeur geometrique. Il repond a des
//! questions de structure : « la perpendicularite exige-t-elle une reference ? »,
//! « la zone est-elle cylindrique, donc la valeur se prefixe-t-elle d'un ø ? »,
//! « cette specification en rend-elle une autre inoperante ? »
//!
//! # Pourquoi une provenance secondaire
//!
//! Les donnees viennent d'un recueil technique qui reproduit la norme, pas de la
//! norme. Le jeu porte donc [`VerificationStatus::Secondary`], ce qui suffit a
//! faire remonter un avertissement jusqu'a l'interface : la regle absolue veut
//! qu'une donnee insuffisamment verifiee se voie.
//!
//! [`VerificationStatus::Secondary`]: mecatool_core::VerificationStatus::Secondary

use std::collections::BTreeSet;
use std::sync::OnceLock;

use mecatool_core::StandardReference;
use serde::{Deserialize, Serialize};

use crate::error::{Result, StandardsError};

/// Les quatre familles de tolerances geometriques.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceFamily {
    Form,
    Orientation,
    Location,
    Runout,
}

impl ToleranceFamily {
    pub const fn name_fr(self) -> &'static str {
        match self {
            ToleranceFamily::Form => "forme",
            ToleranceFamily::Orientation => "orientation",
            ToleranceFamily::Location => "position",
            ToleranceFamily::Runout => "battement",
        }
    }
}

/// Ce que la caracteristique attend en matiere de reference specifiee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatumRule {
    /// Aucune reference : la caracteristique ne se compare qu'a elle-meme.
    None,
    /// Au moins une reference specifiee est necessaire.
    Required,
    /// La reference est admise mais pas imposee.
    Optional,
}

/// Geometrie de la zone de tolerance.
///
/// C'est elle qui decide si la valeur se prefixe d'un `ø` : une zone cylindrique
/// se cote en diametre, une zone entre deux plans se cote en largeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneGeometry {
    TwoParallelStraightLines,
    TwoParallelPlanes,
    TwoConcentricCircles,
    TwoCoaxialCylinders,
    TwoEquidistantLines,
    TwoEquidistantSurfaces,
    Cylinder,
    /// Le battement circulaire ne delimite pas un volume : il borne un ecart
    /// releve pendant une revolution, plan de mesurage par plan de mesurage.
    MeasuredPerSection,
}

impl ZoneGeometry {
    /// Vrai si la zone se cote en diametre, donc si la valeur prend un `ø`.
    pub const fn is_diametral(self) -> bool {
        matches!(self, ZoneGeometry::Cylinder)
    }

    pub const fn name_fr(self) -> &'static str {
        match self {
            ZoneGeometry::TwoParallelStraightLines => "entre deux droites parallèles",
            ZoneGeometry::TwoParallelPlanes => "entre deux plans parallèles",
            ZoneGeometry::TwoConcentricCircles => "entre deux cercles concentriques",
            ZoneGeometry::TwoCoaxialCylinders => "entre deux cylindres coaxiaux",
            ZoneGeometry::TwoEquidistantLines => "entre deux lignes équidistantes",
            ZoneGeometry::TwoEquidistantSurfaces => "entre deux surfaces équidistantes",
            ZoneGeometry::Cylinder => "cylindrique",
            ZoneGeometry::MeasuredPerSection => "relevée par plan de mesurage",
        }
    }
}

/// Une zone, telle que la norme la decrit pour un type d'element donne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneDefinition {
    /// L'element tolerance concerne, par ex. `"un axe"`.
    pub feature: String,
    pub geometry: ZoneGeometry,
    /// La phrase d'interpretation, reprise au plus pres de la source.
    pub definition: String,
}

/// Une caracteristique geometrique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Characteristic {
    pub id: String,
    pub name: String,
    /// Le symbole normalise, par ex. `"⟂"`.
    pub symbol: String,
    /// Saisies acceptees en plus du symbole, toutes en minuscules.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(rename = "class")]
    pub family: ToleranceFamily,
    #[serde(rename = "datum")]
    pub datum: DatumRule,
    pub requires_ted: bool,
    pub zones: Vec<ZoneDefinition>,
    /// Page de la source ou la caracteristique est definie.
    pub page: u32,
}

impl Characteristic {
    /// Vrai si au moins une des zones decrites se cote en diametre.
    ///
    /// Plusieurs caracteristiques admettent les deux : la rectitude d'une ligne
    /// se cote en largeur, celle d'un axe en diametre. Le `ø` n'est donc ni
    /// impose ni interdit pour elles — seulement pour celles dont toutes les
    /// zones sont d'un seul type.
    pub fn allows_diametral(&self) -> bool {
        self.zones.iter().any(|z| z.geometry.is_diametral())
    }

    pub fn always_diametral(&self) -> bool {
        !self.zones.is_empty() && self.zones.iter().all(|z| z.geometry.is_diametral())
    }
}

/// Une famille, et ce qu'elle limite en plus d'elle-meme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyDefinition {
    pub id: ToleranceFamily,
    pub name: String,
    pub heading: String,
    /// Les familles que celle-ci borne aussi, d'apres la source.
    pub limits: Vec<ToleranceFamily>,
    /// La phrase de la source qui enonce cet emboitement.
    pub note: String,
}

/// Un modificateur, avec la norme qui le definit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    pub symbol: String,
    pub name: String,
    pub family: String,
    /// La norme qui definit ce symbole : toutes ne viennent pas de l'ISO 1101.
    pub defined_by: String,
    pub page: u32,
}

/// Le catalogue complet.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CharacteristicTable {
    dataset: String,
    standard: StandardReference,
    classes: Vec<FamilyDefinition>,
    characteristics: Vec<Characteristic>,
    modifiers: Vec<Modifier>,
}

const EMBEDDED: &str = include_str!("../../../data/iso1101/iso1101.geometric-characteristics.json");

impl CharacteristicTable {
    /// Le catalogue embarque, charge et valide une seule fois.
    pub fn embedded() -> Result<&'static CharacteristicTable> {
        static CACHE: OnceLock<core::result::Result<CharacteristicTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| CharacteristicTable::parse(EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    fn parse(text: &str) -> Result<CharacteristicTable> {
        let table: CharacteristicTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso1101.geometric-characteristics".into(),
                detail: source.to_string(),
            })?;
        table.validate()?;
        Ok(table)
    }

    /// Controles de coherence interne.
    ///
    /// Un catalogue faux doit etre detectable au chargement, pas a l'usage :
    /// un symbole en double ou une famille inconnue rendrait la recherche
    /// silencieusement ambigue.
    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        if self.characteristics.is_empty() {
            return Err(bad("aucune caracteristique".into()));
        }

        let families: BTreeSet<ToleranceFamily> = self.classes.iter().map(|c| c.id).collect();
        if families.len() != self.classes.len() {
            return Err(bad("une famille est decrite deux fois".into()));
        }

        let mut ids = BTreeSet::new();
        for c in &self.characteristics {
            if !ids.insert(c.id.as_str()) {
                return Err(bad(format!("identifiant en double : {}", c.id)));
            }
            if !families.contains(&c.family) {
                return Err(bad(format!(
                    "{} appartient a une famille non decrite : {:?}",
                    c.id, c.family
                )));
            }
            if c.zones.is_empty() {
                return Err(bad(format!("{} n'a aucune zone definie", c.id)));
            }
            // Une famille sans reference ne peut pas porter de caracteristique
            // qui en exige une : ce serait contredire le classement lui-meme.
            if c.family == ToleranceFamily::Form && c.datum != DatumRule::None {
                return Err(bad(format!(
                    "{} est classee en forme mais attend une reference",
                    c.id
                )));
            }
            if c.family != ToleranceFamily::Form && c.datum == DatumRule::None {
                return Err(bad(format!(
                    "{} n'est pas une tolerance de forme mais n'attend aucune reference",
                    c.id
                )));
            }
        }

        // Un alias doit designer une seule caracteristique, sinon une saisie
        // devient ambigue sans que rien ne le signale.
        let mut aliases = BTreeSet::new();
        for c in &self.characteristics {
            for alias in &c.aliases {
                if alias != &alias.to_lowercase() {
                    return Err(bad(format!("alias non normalise : {alias}")));
                }
                if !aliases.insert(alias.as_str()) {
                    return Err(bad(format!("alias en double : {alias}")));
                }
            }
        }

        // L'emboitement doit etre acyclique et ne citer que des familles connues.
        for class in &self.classes {
            for limited in &class.limits {
                if !families.contains(limited) {
                    return Err(bad(format!(
                        "{:?} limite une famille non decrite : {limited:?}",
                        class.id
                    )));
                }
                if *limited == class.id {
                    return Err(bad(format!("{:?} se limiterait elle-meme", class.id)));
                }
            }
        }

        Ok(())
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    pub fn characteristics(&self) -> &[Characteristic] {
        &self.characteristics
    }

    pub fn families(&self) -> &[FamilyDefinition] {
        &self.classes
    }

    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }

    pub fn family(&self, id: ToleranceFamily) -> Option<&FamilyDefinition> {
        self.classes.iter().find(|c| c.id == id)
    }

    pub fn by_id(&self, id: &str) -> Option<&Characteristic> {
        self.characteristics.iter().find(|c| c.id == id)
    }

    /// Retrouve une caracteristique par son symbole ou par un alias.
    ///
    /// Plusieurs caracteristiques partagent un symbole : `⌒` designe le profil
    /// d'une ligne, qu'il soit classe en forme, en orientation ou en position.
    /// La distinction se fait a la reference specifiee, pas au symbole ; cette
    /// recherche rend donc toutes les candidates, et c'est a l'appelant de
    /// trancher avec ce qu'il sait de la specification.
    pub fn lookup(&self, needle: &str) -> Vec<&Characteristic> {
        let needle = needle.trim();
        let lower = needle.to_lowercase();
        self.characteristics
            .iter()
            .filter(|c| {
                c.symbol == needle
                    || c.id == lower
                    || c.name.to_lowercase() == lower
                    || c.aliases.iter().any(|a| a == &lower)
            })
            .collect()
    }

    /// Vrai si la famille `wider` borne aussi la famille `narrower`.
    pub fn limits(&self, wider: ToleranceFamily, narrower: ToleranceFamily) -> bool {
        self.family(wider)
            .is_some_and(|f| f.limits.contains(&narrower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mecatool_core::VerificationStatus;

    fn table() -> &'static CharacteristicTable {
        CharacteristicTable::embedded().expect("le catalogue embarque doit charger")
    }

    #[test]
    fn le_catalogue_charge_et_se_valide() {
        assert!(!table().characteristics().is_empty());
        assert_eq!(table().families().len(), 4);
    }

    #[test]
    fn la_source_est_declaree_secondaire() {
        // Le point entier de ce jeu de donnees : il vient d'un recueil, pas de
        // la norme, et le systeme de types doit le porter jusqu'a l'interface.
        let verification = &table().standard().verification;
        assert!(verification.is_secondary());
        assert!(!verification.is_verified());
        assert!(verification.banner_fr().is_some());
    }

    #[test]
    fn le_millesime_nest_pas_invente() {
        // Le recueil cite « SN EN ISO 1101 » sans annee. Rien ne doit la combler.
        assert!(table().standard().edition.is_empty());
        assert_eq!(table().standard().citation(), "ISO 1101");
    }

    #[test]
    fn aucune_valeur_de_tolerance_ne_figure_dans_le_catalogue() {
        // Garde-fou contre une derive : l'ISO 1101 ne donne pas de valeurs, et
        // MecaTool ne doit pas se mettre a en suggerer par ce chemin.
        let json = EMBEDDED;
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        let characteristics = parsed["characteristics"].as_array().unwrap();
        for c in characteristics {
            for (key, value) in c.as_object().unwrap() {
                if key == "page" {
                    continue;
                }
                assert!(
                    !value.is_number(),
                    "{} porte une valeur numerique sous la cle {key}",
                    c["id"]
                );
            }
        }
    }

    #[test]
    fn les_tolerances_de_forme_nont_pas_de_reference() {
        for c in table().characteristics() {
            if c.family == ToleranceFamily::Form {
                assert_eq!(c.datum, DatumRule::None, "{}", c.id);
            } else {
                assert_ne!(c.datum, DatumRule::None, "{}", c.id);
            }
        }
    }

    #[test]
    fn la_perpendicularite_dun_axe_a_une_zone_cylindrique() {
        let perp = table().by_id("perpendicularity").unwrap();
        assert_eq!(perp.family, ToleranceFamily::Orientation);
        assert_eq!(perp.datum, DatumRule::Required);
        assert!(perp.allows_diametral());
        // Mais pas toujours : une surface perpendiculaire se cote en largeur.
        assert!(!perp.always_diametral());
    }

    #[test]
    fn la_coaxialite_est_toujours_diametrale() {
        let coax = table().by_id("coaxiality").unwrap();
        assert!(coax.always_diametral());
    }

    #[test]
    fn la_planeite_nest_jamais_diametrale() {
        assert!(!table().by_id("flatness").unwrap().allows_diametral());
    }

    #[test]
    fn lemboitement_suit_la_source() {
        let t = table();
        assert!(t.limits(ToleranceFamily::Orientation, ToleranceFamily::Form));
        assert!(t.limits(ToleranceFamily::Location, ToleranceFamily::Form));
        assert!(t.limits(ToleranceFamily::Location, ToleranceFamily::Orientation));
        assert!(t.limits(ToleranceFamily::Runout, ToleranceFamily::Form));

        // L'inverse est faux : une tolerance de forme ne borne pas l'orientation.
        assert!(!t.limits(ToleranceFamily::Form, ToleranceFamily::Orientation));
    }

    #[test]
    fn chaque_famille_cite_la_phrase_qui_la_fonde() {
        for family in table().families() {
            assert!(!family.note.is_empty(), "{:?}", family.id);
        }
    }

    #[test]
    fn la_recherche_accepte_le_symbole_et_les_alias() {
        let t = table();
        assert_eq!(t.lookup("⟂").len(), 1);
        assert_eq!(t.lookup("perp").len(), 1);
        assert_eq!(t.lookup("Perpendicularité").len(), 1);
        assert_eq!(t.lookup("//")[0].id, "parallelism");
        assert!(t.lookup("inconnu").is_empty());
    }

    #[test]
    fn un_symbole_partage_rend_toutes_les_candidates() {
        // Le profil d'une ligne existe en forme, en orientation et en position :
        // le symbole seul ne suffit pas a trancher, et la recherche ne doit pas
        // faire croire le contraire en n'en rendant qu'une.
        let found = table().lookup("⌒");
        assert!(found.len() > 1);
        let families: BTreeSet<_> = found.iter().map(|c| c.family).collect();
        assert!(families.contains(&ToleranceFamily::Form));
        assert!(families.contains(&ToleranceFamily::Orientation));
    }

    #[test]
    fn les_modificateurs_disent_quelle_norme_les_definit() {
        let modifiers = table().modifiers();
        let maximum = modifiers.iter().find(|m| m.symbol == "M").unwrap();
        // Le maximum de matiere n'est pas defini par l'ISO 1101 : le dire
        // evite de porter au credit d'une norme ce qui vient d'une autre.
        assert_eq!(maximum.defined_by, "ISO 2692");

        let enveloppe = modifiers.iter().find(|m| m.symbol == "E").unwrap();
        assert_eq!(enveloppe.defined_by, "ISO 14405-1");
    }

    #[test]
    fn un_catalogue_dont_une_forme_exige_une_reference_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(EMBEDDED).unwrap();
        broken["characteristics"][0]["datum"] = serde_json::json!("required");
        let err = CharacteristicTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. }
                     if detail.contains("classee en forme")),
            "{err}"
        );
    }

    #[test]
    fn un_alias_en_double_est_refuse() {
        let mut broken: serde_json::Value = serde_json::from_str(EMBEDDED).unwrap();
        let alias = broken["characteristics"][0]["aliases"][0].clone();
        broken["characteristics"][1]["aliases"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        let err = CharacteristicTable::parse(&broken.to_string()).unwrap_err();
        assert!(
            matches!(&err, StandardsError::Inconsistent { detail, .. }
                     if detail.contains("alias en double")),
            "{err}"
        );
    }

    #[test]
    fn le_statut_secondaire_se_relit_depuis_le_json() {
        // Le fichier ecrit `"state": "secondary"` ; si la deserialisation le
        // rangeait ailleurs, la reserve disparaitrait sans bruit.
        match &table().standard().verification {
            VerificationStatus::Secondary {
                reproduces, from, ..
            } => {
                assert!(reproduces.contains("ISO 1101"));
                assert!(from.contains("VSM"));
            }
            other => panic!("statut inattendu : {other:?}"),
        }
    }
}
