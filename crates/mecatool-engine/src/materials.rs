//! Matieres : lecture d'une designation d'acier, proprietes, et effet de la
//! temperature sur un ajustement.
//!
//! # Ce que ce module fait
//!
//! * Il **decompose** une designation symbolique EN 10027-1 — `S355J2`, `C45E`,
//!   `42CrMo4`, `X5CrNi18-10`, `HS6-5-2`, `DX51D+Z`, `M400-50A` — morceau par
//!   morceau, en appliquant les regles de la norme plutot qu'en cherchant dans
//!   une liste. Les symboles additionnels se lisent avec les listes du groupe
//!   d'emploi (tableaux 1 a 15), les symboles apres `+` avec les tableaux 16 a
//!   18.
//! * Pour un acier de construction embarque, il rend la **limite d'elasticite
//!   par epaisseur** : S355 ne vaut 355 MPa que jusqu'a 16 mm.
//! * Il rattache la nuance a une **famille**, et donne les proprietes physiques
//!   representatives de celle-ci.
//! * Il calcule ce que devient un **ajustement a chaud** quand l'alesage et
//!   l'arbre ne sont pas de la meme matiere : c'est le domaine qui alimente
//!   celui des ajustements, comme les roulements avant lui.
//!
//! # Ce que ce module refuse
//!
//! * Affirmer qu'une nuance existe : une designation bien formee se lit, mais
//!   rien ne dit qu'une norme de produit la definit.
//! * Donner la composition garantie : les teneurs codees sont des moyennes
//!   arrondies.
//! * Lire la designation numerique (`1.4301`) au-dela de sa structure : la
//!   table des groupes d'aciers n'est pas embarquee.
//! * Taire l'edition : les regles ont ete lues dans l'EN 10027-1:2005, que
//!   l'edition 2016 remplace sans avoir ete confrontee. Chaque lecture le
//!   rappelle.

use mecatool_core::{
    Conclusion, FitKind, Length, Provenance, ReasoningStep, ToleranceClass, Unit, Verdict,
};
use mecatool_standards::matieres::{
    CompositionRules, DesignationRules, GroupForm, GroupNumber, MaterialFamily,
    MaterialFamilyTable, ProductSymbol, StructuralGrade, StructuralSteelTable, Suffix,
    SuffixDigits, SymbolSet, UseGroup,
};
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::format;
use crate::geometric::{Finding, Severity};
use crate::iso286::Iso286Engine;

/// La categorie de designation reconnue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SteelKind {
    /// Designee par son emploi : `S355J2`, `DC04`, `M400-50A` (tableaux 1 a 11).
    UseGroup,
    /// Non alliee, manganese moyen < 1 % : `C45` (tableau 12).
    NonAlloy,
    /// Chaque element d'alliage sous 5 % : `42CrMo4`, `28Mn6` (tableau 13).
    LowAlloy,
    /// Un element au moins a 5 % : `X5CrNi18-10` (tableau 14).
    HighAlloy,
    /// Acier rapide : `HS6-5-2` (tableau 15).
    HighSpeed,
    /// Designation numerique : `1.4301`.
    Numeric,
}

impl SteelKind {
    pub const fn label_fr(self) -> &'static str {
        match self {
            SteelKind::UseGroup => "désignée par son emploi et ses caractéristiques",
            SteelKind::NonAlloy => {
                "acier non allié (manganèse moyen < 1 %), désigné par sa teneur en carbone"
            }
            SteelKind::LowAlloy => {
                "acier désigné par sa composition, chaque élément d'alliage sous 5 % \
                 (allié, non allié à manganèse ≥ 1 % ou de décolletage)"
            }
            SteelKind::HighAlloy => {
                "acier allié dont un élément au moins atteint 5 %, désigné par sa composition"
            }
            SteelKind::HighSpeed => "acier rapide",
            SteelKind::Numeric => "désignation numérique",
        }
    }
}

/// Un morceau de la designation, et ce qu'il dit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignationPart {
    pub text: String,
    pub meaning: String,
}

/// Une teneur lue dans la designation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementContent {
    pub element: String,
    /// En milliemes de pour cent : 1500 pour 1,5 %. Absente quand la
    /// designation cite l'element sans teneur.
    pub thousandths_percent: Option<i64>,
    pub label: String,
}

/// La resilience lue : `J2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactReading {
    pub code: String,
    pub joules: u32,
    pub celsius: i32,
    pub label: String,
}

/// Un symbole additionnel pour l'acier, lu avec les listes de son tableau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdditionalSymbol {
    /// Tel qu'ecrit, chiffres compris : `L1`, `Cu3`, `-N5`.
    pub code: String,
    /// Groupe 1 ou groupe 2 (EN 10027-1, 7.2).
    pub group: u8,
    pub meaning: String,
}

/// Un symbole pour les produits en acier, apres `+` (tableaux 16 a 18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductSymbolReading {
    /// Tel qu'ecrit, avec son `+`.
    pub code: String,
    /// Le ou les tableaux qui le definissent ; vide pour un symbole inconnu.
    pub tables: Vec<u8>,
    pub meaning: String,
}

/// Une designation decomposee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteelReading {
    pub input: String,
    pub kind: SteelKind,
    pub kind_label: String,
    /// Vrai pour un acier moule (prefixe `G`).
    pub cast: bool,
    /// Vrai pour un acier elabore par metallurgie des poudres (prefixe `PM`).
    pub powder_metallurgy: bool,
    pub parts: Vec<DesignationPart>,
    pub group: Option<UseGroup>,
    /// Le nombre du groupe d'emploi, et ce qu'il porte.
    pub group_value: Option<u32>,
    pub group_value_label: Option<String>,
    pub impact: Option<ImpactReading>,
    /// Les symboles additionnels pour l'acier.
    pub suffixes: Vec<AdditionalSymbol>,
    /// Les symboles pour les produits en acier.
    pub product_symbols: Vec<ProductSymbolReading>,
    pub carbon: Option<ElementContent>,
    pub elements: Vec<ElementContent>,
    /// La nuance de l'EN 10025-2, quand elle est embarquee.
    pub structural: Option<StructuralGrade>,
    /// La famille proposee, et pourquoi.
    pub family: Option<MaterialFamily>,
    pub family_reason: String,
    pub findings: Vec<Finding>,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// L'effet de la temperature sur un ajustement entre deux matieres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalFit {
    pub nominal: Length,
    /// Ecart de temperature par rapport a l'etat de reference, en kelvins.
    pub delta_t: i32,
    pub hole_family: MaterialFamily,
    pub shaft_family: MaterialFamily,
    /// Dilatation du diametre de l'alesage.
    pub hole_growth: Length,
    pub shaft_growth: Length,
    /// Variation du jeu : dilatation de l'alesage moins celle de l'arbre.
    pub clearance_shift: Length,
    pub clearance_shift_label: String,
    /// L'ajustement a froid puis a chaud, quand des classes sont donnees.
    pub fit: Option<ThermalFitLimits>,
    pub findings: Vec<Finding>,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Le jeu a froid et a chaud.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalFitLimits {
    pub designation: String,
    pub cold_min: Length,
    pub cold_max: Length,
    pub cold_kind: FitKind,
    pub hot_min: Length,
    pub hot_max: Length,
    pub hot_kind: FitKind,
}

/// Ce qu'un symbole additionnel a reconnu a une position.
enum SymbolMatch<'a> {
    Named(&'a Suffix),
    Chemical(&'a str),
    AnyLetter(&'a str, char),
}

impl SymbolMatch<'_> {
    fn len(&self) -> usize {
        match self {
            SymbolMatch::Named(s) => s.code.len(),
            SymbolMatch::Chemical(e) => e.len(),
            SymbolMatch::AnyLetter(..) => 1,
        }
    }
}

/// Le moteur du domaine matieres.
#[derive(Debug)]
pub struct MaterialsEngine {
    rules: &'static DesignationRules,
    structural: &'static StructuralSteelTable,
    families: &'static MaterialFamilyTable,
    iso286: Iso286Engine,
}

impl MaterialsEngine {
    pub fn new() -> Result<Self> {
        Ok(MaterialsEngine {
            rules: DesignationRules::embedded()?,
            structural: StructuralSteelTable::embedded()?,
            families: MaterialFamilyTable::embedded()?,
            iso286: Iso286Engine::new()?,
        })
    }

    pub fn rules(&self) -> &'static DesignationRules {
        self.rules
    }

    pub fn structural_grades(&self) -> &'static [StructuralGrade] {
        self.structural.grades()
    }

    pub fn families(&self) -> &'static [MaterialFamily] {
        self.families.families()
    }

    pub fn provenance(&self) -> Provenance {
        Provenance::new()
            .with(self.rules.standard().clone())
            .with(self.structural.standard().clone())
            .with(self.families.standard().clone())
    }

    /// Decompose une designation d'acier.
    pub fn read(&self, input: &str) -> Result<SteelReading> {
        let trimmed = input.trim();
        let unparsable = |hint: String| EngineError::Unparsable {
            input: input.to_string(),
            hint,
        };
        if trimmed.is_empty() {
            return Err(unparsable(
                "Indiquez une désignation d'acier, par exemple « S355J2 », « C45 », \
                 « 42CrMo4 » ou « X5CrNi18-10 »."
                    .into(),
            ));
        }

        // Les symboles pour les produits suivent chacun un « + » (7.2).
        let mut pieces = trimmed.split('+');
        let body = pieces.next().unwrap_or_default().trim();
        let products: Vec<&str> = pieces.map(str::trim).collect();
        if body.is_empty() {
            return Err(unparsable(
                "La désignation de l'acier précède les symboles « + » : « S235JR+AR ».".into(),
            ));
        }
        if products.iter().any(|p| p.is_empty()) {
            return Err(unparsable(
                "Chaque « + » est suivi d'un symbole des tableaux 16 à 18 : « DX51D+Z ».".into(),
            ));
        }

        let mut reading = if is_numeric(body) {
            self.numeric(body)
        } else {
            // 7.1 : G pour une piece moulee (tableaux 1 a 15), PM pour la
            // metallurgie des poudres (tableaux 14 et 15).
            let powder = body
                .strip_prefix("PM")
                .filter(|rest| rest.starts_with('X') || rest.starts_with("HS"));
            let cast = body
                .strip_prefix('G')
                .filter(|rest| rest.starts_with(|c: char| c.is_ascii_alphanumeric()));
            let (prefix, core) = match (powder, cast) {
                (Some(rest), _) => (Some(("PM", "métallurgie des poudres")), rest),
                (None, Some(rest)) => (Some(("G", "acier moulé")), rest),
                (None, None) => (None, body),
            };
            let mut reading = self.symbolic(core, &unparsable)?;
            if let Some((text, meaning)) = prefix {
                reading.cast = text == "G";
                reading.powder_metallurgy = text == "PM";
                reading.parts.insert(
                    0,
                    DesignationPart {
                        text: text.into(),
                        meaning: meaning.into(),
                    },
                );
            }
            reading
        };
        reading.input = trimmed.to_string();

        let (tables, origin) = self.product_tables(&reading);
        for product in products {
            self.product_symbol(&mut reading, product, &tables, &origin);
        }

        self.finish(reading)
    }

    /// Les tableaux de symboles pour les produits que la categorie admet, et
    /// d'ou vient cette liste.
    fn product_tables(&self, reading: &SteelReading) -> (Vec<u8>, String) {
        let composition = |rules: &CompositionRules| {
            (
                rules.product_tables.clone(),
                format!("le tableau {}", rules.table),
            )
        };
        match reading.kind {
            SteelKind::UseGroup => reading
                .group
                .as_ref()
                .map(|g| (g.product_tables.clone(), format!("le tableau {}", g.table)))
                .unwrap_or_default(),
            SteelKind::NonAlloy => composition(self.rules.non_alloy()),
            SteelKind::LowAlloy => composition(self.rules.low_alloy()),
            SteelKind::HighAlloy => composition(self.rules.high_alloy()),
            SteelKind::HighSpeed => composition(self.rules.high_speed()),
            SteelKind::Numeric => (
                self.rules.numeric_product_tables().to_vec(),
                "la note de 7.2".into(),
            ),
        }
    }

    /// Lit un symbole pour les produits : `Z`, `QT`, `C700`, `Z25`.
    fn product_symbol(&self, reading: &mut SteelReading, text: &str, tables: &[u8], origin: &str) {
        let symbols = self.rules.product_symbols();
        let exact: Vec<&ProductSymbol> = symbols
            .iter()
            .filter(|s| !s.value && s.code == text)
            .collect();
        let (matches, value) = if exact.is_empty() {
            let letters: String = text.chars().take_while(char::is_ascii_alphabetic).collect();
            let digits = &text[letters.len()..];
            if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                let found: Vec<&ProductSymbol> = symbols
                    .iter()
                    .filter(|s| s.value && s.code == letters)
                    .collect();
                (found, Some(digits))
            } else {
                (Vec::new(), None)
            }
        } else {
            (exact, None)
        };

        if matches.is_empty() {
            reading.findings.push(Finding::new(
                "product_symbol_unknown",
                Severity::Caution,
                format!(
                    "« +{text} » ne figure pas aux tableaux 16 à 18 de l'EN 10027-1 : sa \
                     signification, s'il en a une, est dans la norme de produit."
                ),
            ));
            let meaning = "symbole absent des tableaux 16 à 18".to_string();
            reading.parts.push(DesignationPart {
                text: format!("+{text}"),
                meaning: meaning.clone(),
            });
            reading.product_symbols.push(ProductSymbolReading {
                code: format!("+{text}"),
                tables: Vec::new(),
                meaning,
            });
            return;
        }

        let applicable: Vec<&ProductSymbol> = matches
            .iter()
            .copied()
            .filter(|s| tables.contains(&s.table))
            .collect();
        let chosen = if applicable.is_empty() {
            let listed = if tables.is_empty() {
                "aucun".to_string()
            } else {
                join_tables(tables)
            };
            reading.findings.push(Finding::new(
                "product_symbol_table",
                Severity::Caution,
                format!(
                    "« +{text} » vient du tableau {}, auquel {origin} ne renvoie pas \
                     (tableaux prévus : {listed}).",
                    join_tables(&matches.iter().map(|s| s.table).collect::<Vec<_>>()),
                ),
            ));
            matches
        } else {
            applicable
        };
        let meaning = chosen
            .iter()
            .map(|s| match value {
                Some(n) => s.meaning.replace("{n}", n),
                None => s.meaning.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ou ");
        let found_in: Vec<u8> = chosen.iter().map(|s| s.table).collect();
        reading.parts.push(DesignationPart {
            text: format!("+{text}"),
            meaning: format!(
                "{meaning} ({} {})",
                if found_in.len() > 1 {
                    "tableaux"
                } else {
                    "tableau"
                },
                join_tables(&found_in)
            ),
        });
        reading.product_symbols.push(ProductSymbolReading {
            code: format!("+{text}"),
            tables: found_in,
            meaning,
        });
    }

    fn numeric(&self, body: &str) -> SteelReading {
        let (group, order) = body[2..].split_at(2);
        let mut reading = blank(SteelKind::Numeric);
        reading.parts = vec![
            DesignationPart {
                text: "1".into(),
                meaning: "groupe de matériaux : acier".into(),
            },
            DesignationPart {
                text: group.into(),
                meaning: "numéro de groupe d'aciers".into(),
            },
            DesignationPart {
                text: order.into(),
                meaning: "numéro d'ordre dans le groupe".into(),
            },
        ];
        reading.findings.push(Finding::new(
            "numeric_table_absent",
            Severity::Caution,
            format!(
                "La structure de « {body} » se lit, pas ce qu'elle désigne : la table des \
                 groupes d'aciers de l'EN 10027-2 n'est pas embarquée."
            ),
        ));
        reading.family = self.families.family("steel").cloned();
        reading.family_reason = "Une désignation numérique en 1. désigne un acier.".into();
        reading
    }

    fn symbolic(
        &self,
        core: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<SteelReading> {
        if let Some(rest) = core.strip_prefix("HS") {
            return self.high_speed(rest, unparsable);
        }
        if let Some(rest) = core.strip_prefix('X') {
            return self.composition(rest, true, unparsable);
        }
        if core.starts_with(|c: char| c.is_ascii_digit()) {
            return self.composition(core, false, unparsable);
        }
        if let Some(rest) = core.strip_prefix('C') {
            if rest.starts_with(|c: char| c.is_ascii_digit()) {
                return self.non_alloy(rest, unparsable);
            }
        }
        let first = &core[..core.chars().next().map_or(0, char::len_utf8)];
        if let Some(group) = self.rules.use_group(&first.to_ascii_uppercase()) {
            return self.use_group(group, &core[first.len()..], unparsable);
        }
        Err(unparsable(format!(
            "« {core} » ne commence ni par une lettre de groupe d'emploi ({}), ni par C, X ou \
             HS, ni par une teneur en carbone.",
            self.rules
                .use_groups()
                .iter()
                .map(|g| g.letter.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }

    fn use_group(
        &self,
        group: &UseGroup,
        rest: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<SteelReading> {
        // La forme au plus long prefixe : `HXT` avant `HX`.
        let form = group
            .forms
            .iter()
            .filter(|f| rest.starts_with(f.prefix.as_str()))
            .max_by_key(|f| f.prefix.len())
            .ok_or_else(|| {
                unparsable(format!(
                    "« {} » ({}) se poursuit par {} : {}",
                    group.letter,
                    group.name,
                    group
                        .forms
                        .iter()
                        .map(|f| format!("{}{}", group.letter, f.prefix))
                        .collect::<Vec<_>>()
                        .join(", "),
                    lowercase_first(&group.forms[0].note)
                ))
            })?;
        let after = &rest[form.prefix.len()..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return Err(unparsable(format!(
                "« {}{} » attend un nombre : {}",
                group.letter,
                form.prefix,
                lowercase_first(&form.note)
            )));
        }
        let value: u32 = digits
            .parse()
            .map_err(|_| unparsable("Nombre illisible.".into()))?;
        let tail = &after[digits.len()..];

        let mut reading = blank(SteelKind::UseGroup);
        reading.parts.push(DesignationPart {
            text: group.letter.clone(),
            meaning: format!("{} (tableau {})", group.name, group.table),
        });
        if !form.prefix.is_empty() {
            reading.parts.push(DesignationPart {
                text: form.prefix.clone(),
                meaning: form.meaning.clone(),
            });
        }
        if let Some(expected) = form.digits {
            if digits.len() != usize::from(expected) {
                reading.findings.push(Finding::new(
                    "digit_count",
                    Severity::Caution,
                    format!(
                        "« {digits} » : le tableau {} prévoit {expected} chiffres. {}",
                        group.table, form.note
                    ),
                ));
            }
        }

        match form.number {
            GroupNumber::Losses => {
                self.electrical(&mut reading, group, form, value, &digits, tail, unparsable)?;
            }
            GroupNumber::Code => {
                reading.parts.push(DesignationPart {
                    text: digits.clone(),
                    meaning: format!(
                        "{} : {digits}, attribués par l'organisme responsable",
                        form.label
                    ),
                });
                reading.group_value_label = Some(format!("{digits} — {}", form.label));
            }
            GroupNumber::Yield | GroupNumber::Tensile | GroupNumber::Hardness => {
                let unit = if form.number == GroupNumber::Hardness {
                    "HBW"
                } else {
                    "MPa"
                };
                reading.parts.push(DesignationPart {
                    text: digits.clone(),
                    meaning: format!("{} : {value} {unit}", form.label),
                });
                reading.group_value = Some(value);
                reading.group_value_label = Some(format!("{value} {unit} — {}", form.label));
            }
        }
        if form.number != GroupNumber::Losses {
            self.additional(
                &mut reading,
                tail,
                &group.group1,
                &group.group2,
                group.table,
                unparsable,
            )?;
        }

        reading.group = Some(group.clone());
        if group.letter == "S" {
            let grade = format!("S{value}");
            if let Some(structural) = self.structural.grade(&grade) {
                if let Some(impact) = &reading.impact {
                    if !structural.qualities.contains(&impact.code) {
                        let listed = if structural.qualities.is_empty() {
                            "aucune qualité".to_string()
                        } else {
                            structural.qualities.join(", ")
                        };
                        reading.findings.push(Finding::new(
                            "quality_not_listed",
                            Severity::Caution,
                            format!(
                                "La qualité {} n'est pas listée pour {grade} dans la table \
                                 embarquée ({listed}). La désignation reste bien formée.",
                                impact.code,
                            ),
                        ));
                    }
                }
                if let Some(restriction) = &structural.restriction {
                    reading.findings.push(Finding::new(
                        "grade_restricted",
                        Severity::Caution,
                        format!("{grade} : {restriction}"),
                    ));
                }
                reading.structural = Some(structural.clone());
            }
        }
        reading.family = self.families.family("steel").cloned();
        reading.family_reason = format!(
            "Rattachement indicatif : un {} est un acier non inoxydable.",
            group.name
        );
        Ok(reading)
    }

    /// Tableau 11 : `M400-50A`, pertes, epaisseur, type de produit.
    #[allow(clippy::too_many_arguments)]
    fn electrical(
        &self,
        reading: &mut SteelReading,
        group: &UseGroup,
        form: &GroupForm,
        value: u32,
        digits: &str,
        tail: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<()> {
        let hint = || {
            unparsable(format!(
                "Un acier électrique s'écrit « M400-50A » : {}",
                lowercase_first(&form.note)
            ))
        };
        let rest = tail.strip_prefix('-').ok_or_else(hint)?;
        let thickness: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let kind = &rest[thickness.len()..];
        let product = group
            .types
            .iter()
            .find(|t| t.code == kind)
            .ok_or_else(hint)?;
        if thickness.is_empty() {
            return Err(hint());
        }
        let thickness_value: i64 = thickness.parse().map_err(|_| hint())?;
        let losses = format!("{} W/kg", percent(i64::from(value) * 10));
        reading.parts.push(DesignationPart {
            text: digits.to_string(),
            meaning: format!("{} : {losses} ({value} / 100)", form.label),
        });
        reading.parts.push(DesignationPart {
            text: format!("-{thickness}"),
            meaning: format!(
                "épaisseur nominale : {} mm ({thickness} / 100)",
                percent(thickness_value * 10)
            ),
        });
        reading.parts.push(DesignationPart {
            text: product.code.clone(),
            meaning: product.meaning.clone(),
        });
        reading.group_value = Some(value);
        reading.group_value_label = Some(format!("{losses} — {}", form.label));
        Ok(())
    }

    /// Lit les symboles additionnels pour l'acier : groupe 1, puis groupe 2,
    /// qui ne s'emploie qu'apres lui (7.2).
    fn additional(
        &self,
        reading: &mut SteelReading,
        text: &str,
        group1: &SymbolSet,
        group2: &SymbolSet,
        table: u8,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<()> {
        let mut rest = text;
        // Tableaux 1 et 4 : la resilience ouvre le groupe 1.
        if group1.impact {
            if let Some(impact) = self.impact_at(rest) {
                reading.parts.push(DesignationPart {
                    text: impact.code.clone(),
                    meaning: format!("résilience : {}", impact.label),
                });
                reading.impact = Some(impact);
                rest = &rest[2..];
            }
        }
        let mut phase = 1;
        while !rest.is_empty() {
            let first = if phase == 1 {
                self.symbol_at(rest, group1).map(|m| (1, m))
            } else {
                None
            };
            let found = first.or_else(|| self.symbol_at(rest, group2).map(|m| (2, m)));
            let Some((group, found)) = found else {
                let listed = |set: &SymbolSet| {
                    let mut codes: Vec<String> = Vec::new();
                    if set.impact {
                        codes.push("un code de résilience en tête (J2, K2…)".into());
                    }
                    codes.extend(set.symbols.iter().map(|s| s.code.clone()));
                    if let Some(any) = &set.any_letter {
                        codes.push(format!("une lettre de {any}"));
                    }
                    if set.chemical.is_some() {
                        codes.push("un symbole chimique".into());
                    }
                    if codes.is_empty() {
                        "aucun".into()
                    } else {
                        codes.join(", ")
                    }
                };
                return Err(unparsable(format!(
                    "« {rest} » n'est pas un symbole additionnel du tableau {table} \
                     (groupe 1 : {} ; groupe 2 : {}).",
                    listed(group1),
                    listed(group2)
                )));
            };
            phase = group;
            let symbol_text = &rest[..found.len()];
            rest = &rest[found.len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            rest = &rest[digits.len()..];
            let set = if group == 1 { group1 } else { group2 };
            let code = format!("{symbol_text}{digits}");
            let meaning = match found {
                SymbolMatch::Named(suffix) => {
                    let rule = suffix
                        .digits
                        .or(set.digits.then_some(SuffixDigits::Quality));
                    self.digits_meaning(reading, &suffix.meaning, &digits, rule, &code, table)
                }
                SymbolMatch::AnyLetter(meaning, letter) => {
                    let rule = set.digits.then_some(SuffixDigits::Quality);
                    let meaning = format!("{meaning} {letter}");
                    self.digits_meaning(reading, &meaning, &digits, rule, &code, table)
                }
                SymbolMatch::Chemical(element) => {
                    let what = set.chemical.as_deref().unwrap_or("élément");
                    self.chemical_symbol(reading, element, &digits, what, &code, table)
                }
            };
            reading.parts.push(DesignationPart {
                text: code.clone(),
                meaning: format!("groupe {group} : {meaning}"),
            });
            reading.suffixes.push(AdditionalSymbol {
                code,
                group,
                meaning,
            });
        }
        Ok(())
    }

    /// Un code de resilience au debut du texte : `J2`, `KR`.
    fn impact_at(&self, text: &str) -> Option<ImpactReading> {
        let mut chars = text.chars();
        let (energy, code) = (chars.next()?, chars.next()?);
        let impact = self.rules.impact();
        let e = impact
            .energies
            .iter()
            .find(|e| e.letter.chars().eq([energy]))?;
        let t = impact
            .temperatures
            .iter()
            .find(|t| t.code.chars().eq([code]))?;
        let label = format!("{} J à {} °C", e.joules, t.celsius);
        Some(ImpactReading {
            code: format!("{energy}{code}"),
            joules: e.joules,
            celsius: t.celsius,
            label,
        })
    }

    /// Le symbole le plus long qu'un groupe reconnait au debut du texte. A
    /// longueur egale, le symbole nomme passe avant le symbole chimique : `S`
    /// d'un acier de construction est « construction navale », pas le soufre.
    fn symbol_at<'a>(&self, text: &str, set: &'a SymbolSet) -> Option<SymbolMatch<'a>> {
        let named = set
            .symbols
            .iter()
            .filter(|s| text.starts_with(s.code.as_str()))
            .max_by_key(|s| s.code.len())
            .map(SymbolMatch::Named);
        let chemical = set.chemical.as_ref().and_then(|_| {
            self.rules
                .elements()
                .into_iter()
                .find(|e| text.starts_with(e))
                .map(SymbolMatch::Chemical)
        });
        let best = match (named, chemical) {
            (Some(n), Some(c)) if c.len() > n.len() => Some(c),
            (Some(n), _) => Some(n),
            (None, c) => c,
        };
        best.or_else(|| {
            let any = set.any_letter.as_deref()?;
            let letter = text.chars().next().filter(char::is_ascii_uppercase)?;
            Some(SymbolMatch::AnyLetter(any, letter))
        })
    }

    /// Le sens d'un symbole, avec les chiffres qui le suivent.
    fn digits_meaning(
        &self,
        reading: &mut SteelReading,
        meaning: &str,
        digits: &str,
        rule: Option<SuffixDigits>,
        code: &str,
        table: u8,
    ) -> String {
        if digits.is_empty() {
            return meaning.to_string();
        }
        match rule {
            Some(SuffixDigits::SulphurHundredths) => {
                let hundredths: i64 = digits.parse().unwrap_or(0);
                format!(
                    "{meaning} ; {digits} : soufre {} %",
                    percent(hundredths * 10)
                )
            }
            Some(SuffixDigits::Quality) => {
                if digits.len() > 2 {
                    reading.findings.push(Finding::new(
                        "unexpected_digits",
                        Severity::Caution,
                        format!(
                            "« {code} » : le tableau {table} prévoit un ou deux chiffres de \
                             qualité, pas {}.",
                            digits.len()
                        ),
                    ));
                }
                format!("{meaning} ; {digits} : qualité, selon la norme de produit")
            }
            None => {
                reading.findings.push(Finding::new(
                    "unexpected_digits",
                    Severity::Caution,
                    format!("« {code} » : le tableau {table} ne prévoit pas de chiffre ici."),
                ));
                format!("{meaning} ; {digits} : chiffres non prévus par le tableau {table}")
            }
        }
    }

    /// Un symbole chimique additionnel, suivi au plus d'un chiffre valant 10 ×
    /// sa teneur moyenne (tableaux 1, 7, 8 et 12).
    fn chemical_symbol(
        &self,
        reading: &mut SteelReading,
        element: &str,
        digits: &str,
        what: &str,
        code: &str,
        table: u8,
    ) -> String {
        let content = if digits.is_empty() {
            None
        } else {
            if digits.len() > 1 {
                reading.findings.push(Finding::new(
                    "unexpected_digits",
                    Severity::Caution,
                    format!(
                        "« {code} » : le tableau {table} prévoit un seul chiffre après un \
                         symbole chimique, 10 × sa teneur moyenne."
                    ),
                ));
            }
            digits.parse::<i64>().ok().map(|n| n * 100)
        };
        let label = match content {
            Some(t) => format!("{} %", percent(t)),
            None => "présent, teneur non indiquée".into(),
        };
        reading.elements.push(ElementContent {
            element: element.to_string(),
            thousandths_percent: content,
            label: label.clone(),
        });
        match content {
            Some(_) => format!("{what} : {element}, {label} ({digits} / 10)"),
            None => format!("{what} : {element}"),
        }
    }

    fn non_alloy(
        &self,
        rest: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<SteelReading> {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let carbon: i64 = digits
            .parse()
            .map_err(|_| unparsable("Teneur en carbone illisible.".into()))?;
        let mut reading = blank(SteelKind::NonAlloy);
        let carbon = carbon_content(carbon);
        reading.parts.push(DesignationPart {
            text: "C".into(),
            meaning: "carbone : acier non allié (tableau 12)".into(),
        });
        reading.parts.push(DesignationPart {
            text: digits.clone(),
            meaning: format!("carbone : {}", carbon.label),
        });
        reading.carbon = Some(carbon);
        let rules = self.rules.non_alloy();
        self.additional(
            &mut reading,
            &rest[digits.len()..],
            &rules.group1,
            &rules.group2,
            rules.table,
            unparsable,
        )?;
        reading.family = self.families.family("steel").cloned();
        reading.family_reason = "Un acier non allié.".into();
        Ok(reading)
    }

    fn composition(
        &self,
        rest: &str,
        high: bool,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<SteelReading> {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return Err(unparsable(
                "La teneur en carbone, multipliée par 100, doit suivre : « X5CrNi18-10 ».".into(),
            ));
        }
        let carbon = carbon_content(digits.parse().unwrap_or(0));
        let mut tail = &rest[digits.len()..];

        // Les symboles chimiques, du plus long au plus court a chaque position.
        let known = self.rules.elements();
        let mut symbols: Vec<String> = Vec::new();
        while let Some(found) = known.iter().find(|e| tail.starts_with(**e)) {
            symbols.push((*found).to_string());
            tail = &tail[found.len()..];
        }
        if symbols.is_empty() {
            return Err(unparsable(format!(
                "Aucun symbole chimique connu après la teneur en carbone dans « {rest} »."
            )));
        }
        // Les nombres, puis (tableau 14) les symboles additionnels « -N5 ».
        let mut numbers: Vec<i64> = Vec::new();
        let mut extras: Vec<&str> = Vec::new();
        if !tail.trim().is_empty() {
            for segment in tail.split('-').map(str::trim) {
                if !extras.is_empty() || segment.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    extras.push(segment);
                } else {
                    numbers.push(
                        segment
                            .parse::<i64>()
                            .map_err(|_| unparsable(format!("Teneurs illisibles : « {tail} ».")))?,
                    );
                }
            }
        }
        if numbers.len() > symbols.len() {
            return Err(unparsable(format!(
                "{} teneurs pour {} éléments : chaque nombre se rapporte à un élément, dans \
                 l'ordre.",
                numbers.len(),
                symbols.len()
            )));
        }
        if !high && !extras.is_empty() {
            return Err(unparsable(format!(
                "Teneurs illisibles : « {tail} ». Le tableau 13 ne prévoit pas de symbole \
                 additionnel pour l'acier."
            )));
        }

        let mut reading = blank(if high {
            SteelKind::HighAlloy
        } else {
            SteelKind::LowAlloy
        });
        if high {
            reading.parts.push(DesignationPart {
                text: "X".into(),
                meaning: "un élément d'alliage au moins a une teneur moyenne ≥ 5 % (tableau 14)"
                    .into(),
            });
        }
        reading.parts.push(DesignationPart {
            text: digits.clone(),
            meaning: format!("carbone : {}", carbon.label),
        });
        reading.carbon = Some(carbon);

        for (index, symbol) in symbols.iter().enumerate() {
            let number = numbers.get(index).copied();
            let (thousandths, how) = match (number, high) {
                (Some(n), true) => (Some(n * 1000), format!("{n} = teneur en %")),
                (Some(n), false) => {
                    let factor = i64::from(self.rules.factor(symbol).unwrap_or(1));
                    (Some(n * 1000 / factor), format!("{n} / {factor}"))
                }
                (None, _) => (None, "sans teneur indiquée".into()),
            };
            let label = match thousandths {
                Some(t) => format!("{} %", percent(t)),
                None => "présent, teneur non indiquée".into(),
            };
            reading.parts.push(DesignationPart {
                text: match number {
                    Some(n) => format!("{symbol} {n}"),
                    None => symbol.clone(),
                },
                meaning: format!("{symbol} : {label} ({how})"),
            });
            reading.elements.push(ElementContent {
                element: symbol.clone(),
                thousandths_percent: thousandths,
                label,
            });
        }
        let main_known = numbers.len() == symbols.len();

        for extra in extras {
            self.high_alloy_extra(&mut reading, extra, unparsable)?;
        }

        // Un acier du tableau 13 dont un element atteint 5 % aurait du s'ecrire
        // en X : la designation se contredit.
        if !high {
            if let Some(over) = reading
                .elements
                .iter()
                .find(|e| e.thousandths_percent.is_some_and(|t| t >= 5000))
            {
                reading.findings.push(Finding::new(
                    "low_alloy_over_five",
                    Severity::Error,
                    format!(
                        "{} atteint {} : un acier dont un élément atteint 5 % se désigne avec \
                         le préfixe X, et ses teneurs se lisent alors sans facteur.",
                        over.element, over.label
                    ),
                ));
            }
        } else if main_known
            && reading
                .elements
                .iter()
                .all(|e| e.thousandths_percent.is_some_and(|t| t < 5000))
        {
            reading.findings.push(Finding::new(
                "high_alloy_under_five",
                Severity::Caution,
                "Toutes les teneurs sont données et aucune n'atteint 5 % : le préfixe X est \
                 réservé aux aciers dont un élément d'alliage atteint 5 % (tableau 14). \
                 Vérifiez la saisie.",
            ));
        }

        let content = |element: &str| {
            reading
                .elements
                .iter()
                .find(|e| e.element == element)
                .and_then(|e| e.thousandths_percent)
                .unwrap_or(0)
        };
        let (family, reason) = if high && content("Cr") >= 10_500 {
            if content("Ni") >= 6_000 {
                (
                    "stainless_austenitic",
                    "Rattachement indicatif : Cr ≥ 10,5 % et Ni ≥ 6 %, d'ordinaire un \
                     inoxydable austénitique."
                        .to_string(),
                )
            } else {
                (
                    "stainless_ferritic",
                    "Rattachement indicatif : Cr ≥ 10,5 % sans nickel notable, d'ordinaire un \
                     inoxydable ferritique ou martensitique."
                        .to_string(),
                )
            }
        } else {
            (
                "steel",
                "Moins de 10,5 % de chrome : un acier non inoxydable.".to_string(),
            )
        };
        reading.family = self.families.family(family).cloned();
        reading.family_reason = reason;
        Ok(reading)
    }

    /// Tableau 14, groupe 1 : `-N5`, un element entre 0,20 et 1,0 %, suivi de
    /// 10 × sa teneur moyenne.
    fn high_alloy_extra(
        &self,
        reading: &mut SteelReading,
        extra: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<()> {
        let element = self
            .rules
            .elements()
            .into_iter()
            .find(|e| extra.starts_with(e))
            .ok_or_else(|| {
                unparsable(format!(
                    "« -{extra} » : le tableau 14 attend un symbole chimique suivi de 10 × sa \
                     teneur moyenne, par exemple « -N5 »."
                ))
            })?;
        let digits = &extra[element.len()..];
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(unparsable(format!(
                "« -{extra} » : seul un nombre suit le symbole chimique."
            )));
        }
        let content = digits.parse::<i64>().ok().map(|n| n * 100);
        if content.is_some_and(|t| !(200..=1000).contains(&t)) {
            reading.findings.push(Finding::new(
                "additional_out_of_range",
                Severity::Caution,
                format!(
                    "« -{extra} » : le tableau 14 réserve ce symbole à un élément dont la teneur \
                     est comprise entre 0,20 % et 1,0 %."
                ),
            ));
        }
        let label = match content {
            Some(t) => format!("{} %", percent(t)),
            None => "présent, teneur non indiquée".into(),
        };
        match reading.elements.iter_mut().find(|e| e.element == element) {
            Some(existing) if existing.thousandths_percent.is_none() => {
                existing.thousandths_percent = content;
                existing.label = label.clone();
            }
            Some(_) => {}
            None => reading.elements.push(ElementContent {
                element: element.to_string(),
                thousandths_percent: content,
                label: label.clone(),
            }),
        }
        let meaning = format!(
            "{element} : {label}, élément entre 0,20 et 1,0 % (nombre = 10 × teneur moyenne)"
        );
        reading.parts.push(DesignationPart {
            text: format!("-{extra}"),
            meaning: format!("groupe 1 : {meaning}"),
        });
        reading.suffixes.push(AdditionalSymbol {
            code: format!("-{extra}"),
            group: 1,
            meaning,
        });
        Ok(())
    }

    fn high_speed(
        &self,
        rest: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<SteelReading> {
        let order = self.rules.high_speed_order();
        let hint = || {
            unparsable(format!(
                "MecaTool lit de 3 à {} teneurs, dans l'ordre {} (tableau 15) : « HS6-5-2 ».",
                order.len(),
                order.join(", ")
            ))
        };
        // Le dernier nombre peut porter les symboles du groupe 1 : `HS6-5-2C`.
        let numbers_end = rest
            .rfind(|c: char| c.is_ascii_digit())
            .map_or(0, |i| i + 1);
        let (numbers_text, mut symbols_text) = rest.split_at(numbers_end);
        let numbers: Vec<i64> = numbers_text
            .split('-')
            .map(|n| n.trim().parse::<i64>())
            .collect::<core::result::Result<_, _>>()
            .map_err(|_| hint())?;
        if numbers.len() < 3 || numbers.len() > order.len() {
            return Err(hint());
        }
        let mut reading = blank(SteelKind::HighSpeed);
        reading.parts.push(DesignationPart {
            text: "HS".into(),
            meaning: "acier rapide (tableau 15)".into(),
        });
        for (symbol, n) in order.iter().zip(&numbers) {
            let label = format!("{} %", percent(n * 1000));
            reading.parts.push(DesignationPart {
                text: n.to_string(),
                meaning: format!("{symbol} : {label}"),
            });
            reading.elements.push(ElementContent {
                element: symbol.clone(),
                thousandths_percent: Some(n * 1000),
                label,
            });
        }
        // Groupe 1 : l'element, ou les elements, a teneur plus elevee.
        let known = self.rules.elements();
        while !symbols_text.is_empty() {
            let element = known
                .iter()
                .copied()
                .chain(["C"])
                .find(|e| symbols_text.starts_with(e))
                .ok_or_else(|| {
                    unparsable(format!(
                        "« {symbols_text} » : après les teneurs, le tableau 15 n'attend que des \
                         symboles chimiques (« HS6-5-2C »)."
                    ))
                })?;
            symbols_text = &symbols_text[element.len()..];
            let meaning = format!("{element} : teneur plus élevée, pour une même nuance");
            reading.parts.push(DesignationPart {
                text: element.to_string(),
                meaning: format!("groupe 1 : {meaning}"),
            });
            reading.suffixes.push(AdditionalSymbol {
                code: element.to_string(),
                group: 1,
                meaning,
            });
        }
        reading.family = self.families.family("steel").cloned();
        reading.family_reason = "Un acier à outils.".into();
        Ok(reading)
    }

    fn finish(&self, mut reading: SteelReading) -> Result<SteelReading> {
        let provenance = self.provenance();
        if let Some(by) = self.rules.superseded_by() {
            reading.findings.push(Finding::new(
                "edition_superseded",
                Severity::Note,
                format!(
                    "Règles lues dans l'{}, remplacée par l'{by}, qui n'a pas été confrontée : \
                     une règle ou un symbole a pu y changer.",
                    self.rules.standard().citation()
                ),
            ));
        }
        reading.findings.push(Finding::new(
            "existence_not_claimed",
            Severity::Note,
            "Une désignation bien formée se lit ; rien ne dit qu'une norme de produit définit \
             cette nuance.",
        ));
        let summary = reading
            .parts
            .iter()
            .map(|p| format!("{} : {}", p.text, p.meaning))
            .collect::<Vec<_>>()
            .join(" ; ");
        let mut conclusion = match reading
            .findings
            .iter()
            .find(|f| f.severity == Severity::Error)
        {
            Some(error) => Conclusion::new(Verdict::Incompatible, error.message.clone()),
            None => Conclusion::new(
                Verdict::Caution,
                format!("{} — {}.", reading.input, reading.kind.label_fr()),
            ),
        };
        conclusion.why = vec![
            ReasoningStep::new("Catégorie")
                .with_expression(reading.kind.label_fr())
                .with_value(summary),
            ReasoningStep::new("Famille")
                .with_expression(
                    reading
                        .family
                        .as_ref()
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| "non rattachée".into()),
                )
                .with_value(reading.family_reason.clone()),
        ];
        conclusion.warnings = provenance.warnings_fr();
        reading.kind_label = reading.kind.label_fr().to_string();
        reading.conclusion = conclusion;
        reading.provenance = provenance;
        Ok(reading)
    }

    /// Ce que devient un ajustement quand la temperature change.
    ///
    /// Les deux pieces sont supposees a la meme temperature, uniforme. La
    /// dilatation se calcule sur le diametre nominal : la difference due aux
    /// dimensions reelles est du second ordre, et elle est dite.
    pub fn thermal_fit(
        &self,
        nominal: Length,
        delta_t: i32,
        hole_family: &str,
        shaft_family: &str,
        classes: Option<(ToleranceClass, ToleranceClass)>,
    ) -> Result<ThermalFit> {
        let family = |id: &str| {
            self.families
                .family(id)
                .cloned()
                .ok_or_else(|| EngineError::Unparsable {
                    input: id.to_string(),
                    hint: format!(
                        "Famille inconnue. Familles embarquées : {}.",
                        self.families
                            .families()
                            .iter()
                            .map(|f| f.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                })
        };
        let hole_family = family(hole_family)?;
        let shaft_family = family(shaft_family)?;
        if !nominal.is_positive() {
            return Err(EngineError::Unparsable {
                input: format::mm_trimmed(nominal),
                hint: "Le diamètre est strictement positif.".into(),
            });
        }

        let (hole_growth, hole_rounded) = growth(nominal, &hole_family, delta_t);
        let (shaft_growth, shaft_rounded) = growth(nominal, &shaft_family, delta_t);
        let clearance_shift = hole_growth - shaft_growth;
        let shift_label = format!("{} µm", signed_um(clearance_shift));

        let mut provenance = Provenance::new().with(self.families.standard().clone());
        let mut findings = Vec::new();
        let mut steps = vec![
            ReasoningStep::new("Dilatation de l'alésage")
                .with_expression(format!(
                    "Ø × α × ΔT = {} mm × {} µm/(m·K) × {} K",
                    format::mm_trimmed(nominal),
                    alpha_label(&hole_family),
                    delta_t
                ))
                .with_value(format!(
                    "{} µm ({})",
                    signed_um(hole_growth),
                    hole_family.name
                )),
            ReasoningStep::new("Dilatation de l'arbre")
                .with_expression(format!(
                    "Ø × α × ΔT = {} mm × {} µm/(m·K) × {} K",
                    format::mm_trimmed(nominal),
                    alpha_label(&shaft_family),
                    delta_t
                ))
                .with_value(format!(
                    "{} µm ({})",
                    signed_um(shaft_growth),
                    shaft_family.name
                )),
            ReasoningStep::new("Variation du jeu")
                .with_expression("dilatation de l'alésage − dilatation de l'arbre")
                .with_value(shift_label.clone()),
        ];
        if hole_rounded || shaft_rounded {
            steps.push(
                ReasoningStep::new("Arrondi")
                    .with_expression("au nanomètre le plus proche")
                    .with_value("le produit ne tombe pas sur un nanomètre entier"),
            );
        }

        let fit = match classes {
            Some((hole, shaft)) => {
                let analysis = self.iso286.fit(nominal, hole, shaft)?;
                provenance.merge(&analysis.provenance);
                let cold = &analysis.fit;
                let hot_min = cold.min_clearance + clearance_shift;
                let hot_max = cold.max_clearance + clearance_shift;
                let hot_kind = kind_of(hot_min, hot_max);
                if hot_kind != cold.kind {
                    findings.push(Finding::new(
                        "fit_kind_changes",
                        Severity::Caution,
                        format!(
                            "L'ajustement change de nature : {} à froid, {} à ΔT = {delta_t} K.",
                            kind_label(cold.kind),
                            kind_label(hot_kind)
                        ),
                    ));
                }
                if cold.min_clearance >= Length::ZERO && hot_min < Length::ZERO {
                    findings.push(Finding::new(
                        "clearance_lost",
                        Severity::Error,
                        "Le jeu minimal devient négatif : l'arbre peut serrer, voire gripper, à \
                         cette température.",
                    ));
                }
                if cold.max_clearance <= Length::ZERO && hot_max > Length::ZERO {
                    findings.push(Finding::new(
                        "interference_lost",
                        Severity::Error,
                        "Le serrage peut disparaître : la pièce frettée peut tourner ou glisser à \
                         cette température.",
                    ));
                }
                steps.push(
                    ReasoningStep::new("Jeu à chaud")
                        .with_expression("jeu à froid + variation")
                        .with_value(format!(
                            "{} µm à {} µm",
                            signed_um(hot_min),
                            signed_um(hot_max)
                        )),
                );
                Some(ThermalFitLimits {
                    designation: format!("Ø{} {}/{}", format::mm_trimmed(nominal), hole, shaft),
                    cold_min: cold.min_clearance,
                    cold_max: cold.max_clearance,
                    cold_kind: cold.kind,
                    hot_min,
                    hot_max,
                    hot_kind,
                })
            }
            None => None,
        };

        if hole_family.id == shaft_family.id {
            findings.push(Finding::new(
                "same_family",
                Severity::Note,
                "Même famille pour les deux pièces : à température uniforme, le jeu ne varie \
                 qu'au second ordre.",
            ));
        }
        findings.push(Finding::new(
            "nominal_basis",
            Severity::Note,
            "Calculé sur le diamètre nominal, pièces à température uniforme. L'écart dû aux \
             dimensions réelles est du second ordre.",
        ));

        let mut conclusion = match findings.iter().find(|f| f.severity == Severity::Error) {
            Some(error) => Conclusion::new(Verdict::Incompatible, error.message.clone()),
            None => Conclusion::new(
                Verdict::Caution,
                format!(
                    "À ΔT = {delta_t} K, le jeu varie de {shift_label} : {} contre {}.",
                    hole_family.name, shaft_family.name
                ),
            ),
        };
        conclusion.why = steps;
        conclusion.warnings = provenance.warnings_fr();

        Ok(ThermalFit {
            nominal,
            delta_t,
            hole_family,
            shaft_family,
            hole_growth,
            shaft_growth,
            clearance_shift,
            clearance_shift_label: shift_label,
            fit,
            findings,
            conclusion,
            provenance,
        })
    }
}

fn blank(kind: SteelKind) -> SteelReading {
    SteelReading {
        input: String::new(),
        kind,
        kind_label: String::new(),
        cast: false,
        powder_metallurgy: false,
        parts: Vec::new(),
        group: None,
        group_value: None,
        group_value_label: None,
        impact: None,
        suffixes: Vec::new(),
        product_symbols: Vec::new(),
        carbon: None,
        elements: Vec::new(),
        structural: None,
        family: None,
        family_reason: String::new(),
        findings: Vec::new(),
        conclusion: Conclusion::new(Verdict::InsufficientData, String::new()),
        provenance: Provenance::new(),
    }
}

/// `1.4301` ou `1.430101` : la designation numerique.
fn is_numeric(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("1.") else {
        return false;
    };
    (rest.len() == 4 || rest.len() == 6) && rest.chars().all(|c| c.is_ascii_digit())
}

/// `17`, `17 et 18`, `16, 17 et 18`.
fn join_tables(tables: &[u8]) -> String {
    let text: Vec<String> = tables.iter().map(u8::to_string).collect();
    match text.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} et {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    }
}

/// La teneur en carbone : le nombre vaut cent fois le pourcentage.
fn carbon_content(hundredths: i64) -> ElementContent {
    let thousandths = hundredths * 10;
    ElementContent {
        element: "C".into(),
        thousandths_percent: Some(thousandths),
        label: format!("{} %", percent(thousandths)),
    }
}

/// Des milliemes de pour cent, sans zero superflu : 1250 donne `1.25`.
fn percent(thousandths: i64) -> String {
    let whole = thousandths / 1000;
    let fraction = thousandths % 1000;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:03}")
            .trim_end_matches('0')
            .to_string()
    }
}

/// `Ø × α × ΔT`, arrondi au nanometre le plus proche, et si l'arrondi a joue.
fn growth(nominal: Length, family: &MaterialFamily, delta_t: i32) -> (Length, bool) {
    // α en dixiemes de µm/(m·K) : nm × α × ΔT / (10 × 10^6).
    let numerator =
        i128::from(nominal.nanometres()) * i128::from(family.alpha_tenths) * i128::from(delta_t);
    let denominator: i128 = 10_000_000;
    let rounded = numerator % denominator != 0;
    let half = denominator / 2;
    let value = if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    };
    (Length::from_nanometres(value as i64), rounded)
}

fn alpha_label(family: &MaterialFamily) -> String {
    let whole = family.alpha_tenths / 10;
    let tenth = family.alpha_tenths % 10;
    if tenth == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{tenth}")
    }
}

fn signed_um(value: Length) -> String {
    let body = value.to_decimal_string(Unit::Micrometre, 1);
    let body = if body.contains('.') {
        body.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        body
    };
    if value.is_positive() {
        format!("+{body}")
    } else {
        body
    }
}

fn kind_of(min: Length, max: Length) -> FitKind {
    if min >= Length::ZERO {
        FitKind::Clearance
    } else if max <= Length::ZERO {
        FitKind::Interference
    } else {
        FitKind::Transition
    }
}

fn kind_label(kind: FitKind) -> &'static str {
    match kind {
        FitKind::Clearance => "avec jeu",
        FitKind::Transition => "incertain",
        FitKind::Interference => "serré",
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> MaterialsEngine {
        MaterialsEngine::new().expect("les donnees matieres doivent charger")
    }

    fn content(reading: &SteelReading, element: &str) -> Option<i64> {
        reading
            .elements
            .iter()
            .find(|e| e.element == element)
            .and_then(|e| e.thousandths_percent)
    }

    #[test]
    fn lit_un_acier_de_construction() {
        let reading = engine().read("S355J2").unwrap();
        assert_eq!(reading.kind, SteelKind::UseGroup);
        assert_eq!(reading.group_value, Some(355));
        let impact = reading.impact.unwrap();
        assert_eq!((impact.joules, impact.celsius), (27, -20));
        let structural = reading.structural.unwrap();
        assert_eq!(structural.rows[0].yield_mpa, 355);
        assert_eq!(reading.family.unwrap().id, "steel");
    }

    #[test]
    fn les_codes_de_resilience_se_lisent_par_regle() {
        assert_eq!(engine().read("S275JR").unwrap().impact.unwrap().celsius, 20);
        assert_eq!(engine().read("S235J0").unwrap().impact.unwrap().celsius, 0);
        let k2 = engine().read("S355K2").unwrap().impact.unwrap();
        assert_eq!((k2.joules, k2.celsius), (40, -20));
    }

    #[test]
    fn les_symboles_additionnels_se_lisent() {
        let reading = engine().read("S355NL").unwrap();
        let codes: Vec<&str> = reading.suffixes.iter().map(|s| s.code.as_str()).collect();
        assert_eq!(codes, ["N", "L"]);
        assert!(reading.impact.is_none());
    }

    #[test]
    fn une_qualite_non_listee_est_signalee_sans_refus() {
        let reading = engine().read("S235K2").unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "quality_not_listed"));
        // S185 n'a aucune qualite : le message le dit sans liste vide.
        let reading = engine().read("S185JR").unwrap();
        let finding = reading
            .findings
            .iter()
            .find(|f| f.code == "quality_not_listed")
            .unwrap();
        assert!(
            finding.message.contains("aucune qualité"),
            "{}",
            finding.message
        );
        // Sans qualite, la nuance se lit et retrouve sa table, sans constat.
        let reading = engine().read("S185").unwrap();
        assert_eq!(reading.structural.as_ref().unwrap().grade, "S185");
        assert!(!reading
            .findings
            .iter()
            .any(|f| f.code == "quality_not_listed"));
    }

    #[test]
    fn s460_rappelle_quil_ne_vaut_que_pour_les_produits_longs() {
        // EN 10025-2:2019, article 1 et tableau 6, note b.
        let reading = engine().read("S460J2").unwrap();
        let structural = reading.structural.as_ref().unwrap();
        assert_eq!(structural.rows.len(), 6);
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "grade_restricted" && f.message.contains("longs")));
        // Une nuance sans restriction ne la signale pas.
        let reading = engine().read("S355J2").unwrap();
        assert!(!reading
            .findings
            .iter()
            .any(|f| f.code == "grade_restricted"));
        assert_eq!(reading.structural.unwrap().rows.len(), 9);
    }

    #[test]
    fn lit_un_acier_non_allie() {
        let reading = engine().read("C45E").unwrap();
        assert_eq!(reading.kind, SteelKind::NonAlloy);
        assert_eq!(reading.carbon.unwrap().thousandths_percent, Some(450));
        assert_eq!(reading.suffixes[0].code, "E");
    }

    #[test]
    fn lit_un_acier_faiblement_allie_avec_ses_facteurs() {
        let reading = engine().read("42CrMo4").unwrap();
        assert_eq!(reading.kind, SteelKind::LowAlloy);
        assert_eq!(reading.carbon.as_ref().unwrap().label, "0.42 %");
        // Cr : 4 / 4 = 1 %. Mo : present, sans teneur.
        assert_eq!(content(&reading, "Cr"), Some(1000));
        assert_eq!(content(&reading, "Mo"), None);
        let reading = engine().read("16MnCr5").unwrap();
        assert_eq!(content(&reading, "Mn"), Some(1250));
        let reading = engine().read("17NiCrMo6-4").unwrap();
        assert_eq!(content(&reading, "Ni"), Some(1500));
        assert_eq!(content(&reading, "Cr"), Some(1000));
        let reading = engine().read("100Cr6").unwrap();
        assert_eq!(reading.carbon.unwrap().label, "1 %");
    }

    #[test]
    fn lit_un_acier_fortement_allie_sans_facteur() {
        let reading = engine().read("X5CrNi18-10").unwrap();
        assert_eq!(reading.kind, SteelKind::HighAlloy);
        assert_eq!(reading.carbon.as_ref().unwrap().label, "0.05 %");
        assert_eq!(content(&reading, "Cr"), Some(18_000));
        assert_eq!(content(&reading, "Ni"), Some(10_000));
        assert_eq!(reading.family.unwrap().id, "stainless_austenitic");
        let ferritic = engine().read("X6Cr17").unwrap();
        assert_eq!(ferritic.family.unwrap().id, "stainless_ferritic");
    }

    #[test]
    fn un_faiblement_allie_a_plus_de_cinq_pour_cent_se_contredit() {
        // 36NiCrMo16 : Ni = 16 / 4 = 4 %, licite. 30Cr24 : Cr = 6 %, il aurait
        // fallu ecrire X.
        assert!(engine()
            .read("36NiCrMo16")
            .unwrap()
            .findings
            .iter()
            .all(|f| f.severity != Severity::Error));
        let reading = engine().read("30Cr24").unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Incompatible);
    }

    #[test]
    fn lit_un_acier_rapide() {
        let reading = engine().read("HS6-5-2-5").unwrap();
        assert_eq!(content(&reading, "W"), Some(6000));
        assert_eq!(content(&reading, "Co"), Some(5000));
    }

    #[test]
    fn lit_un_acier_moule_et_un_symbole_de_produit() {
        let reading = engine().read("GX5CrNi19-10").unwrap();
        assert!(reading.cast);
        assert!(!reading.powder_metallurgy);
        // EN 10027-1:2005, tableau 18 : +AR est defini par la norme, il ne
        // renvoie pas a la norme de produit.
        let reading = engine().read("S235JR+AR").unwrap();
        let product = &reading.product_symbols[0];
        assert_eq!(product.code, "+AR");
        assert_eq!(product.tables, [18]);
        assert!(
            product.meaning.starts_with("brut de laminage"),
            "{}",
            product.meaning
        );
        assert!(!reading
            .findings
            .iter()
            .any(|f| f.code.starts_with("product_symbol")));
    }

    #[test]
    fn le_groupe_dit_quels_tableaux_de_produit_sappliquent() {
        // Tableau 8 -> tableaux 17 et 18 : +Z est la galvanisation.
        let reading = engine().read("DX51D+Z").unwrap();
        assert_eq!(reading.product_symbols[0].tables, [17]);
        assert!(reading.product_symbols[0].meaning.contains("galvanisation"));
        // Tableau 12 -> tableau 18 seul : +A est un recuit, pas un revetement.
        let reading = engine().read("C45+A").unwrap();
        assert_eq!(reading.product_symbols[0].tables, [18]);
        assert_eq!(reading.product_symbols[0].meaning, "recuit d'adoucissement");
        // Tableau 1 -> 16, 17 et 18 : +A a deux sens, les deux sont donnes.
        let reading = engine().read("S235JR+A").unwrap();
        assert_eq!(reading.product_symbols[0].tables, [17, 18]);
        assert!(reading.product_symbols[0].meaning.contains(" ou "));
        // Une valeur : +C700.
        let reading = engine().read("S355J2+C700").unwrap();
        assert!(reading.product_symbols[0].meaning.contains("700 MPa"));
        // Plusieurs symboles, chacun apres son « + ».
        let reading = engine().read("S355J2+Z25+N").unwrap();
        let codes: Vec<&str> = reading
            .product_symbols
            .iter()
            .map(|p| p.code.as_str())
            .collect();
        assert_eq!(codes, ["+Z25", "+N"]);
        assert_eq!(reading.product_symbols[0].tables, [16]);
        // Tableau 4 -> tableau 18 seul : un revetement est signale.
        let reading = engine().read("E295+Z").unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "product_symbol_table"));
        // Un symbole absent des tableaux 16 a 18 est signale, pas refuse.
        let reading = engine().read("S235JR+XYZ").unwrap();
        assert!(reading.product_symbols[0].tables.is_empty());
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "product_symbol_unknown"));
        assert!(engine().read("S235JR+").is_err());
    }

    #[test]
    fn les_exemples_de_la_norme_se_lisent_sans_reserve() {
        // EN 10027-1:2005, exemples des tableaux 1 a 15.
        for input in [
            "S235JR",
            "S355JR",
            "S355J0",
            "S355J2",
            "S355K2",
            "S450J0",
            "S355N",
            "S355NL",
            "S355M",
            "S355ML",
            "S235J0W",
            "S235J2W",
            "S355J0WP",
            "S355J2WP",
            "S355J0W",
            "S355J2W",
            "S355K2W",
            "S460Q",
            "S460QL",
            "S460QL1",
            "S355MC",
            "S355NC",
            "S355J2H",
            "S355GP",
            "S350GD",
            "S350GD+Z",
            "P265GH",
            "P355NH",
            "P355M",
            "P355ML1",
            "P355Q",
            "P355QH",
            "P355QL1",
            "P265NB",
            "P265S",
            "GP240GR",
            "GP240GH",
            "L360GA",
            "L360NB",
            "L360QB",
            "L360MB",
            "E295",
            "E295GC",
            "E335",
            "E360",
            "GE240",
            "E355K2",
            "B500A",
            "Y1770C",
            "Y1770S7",
            "Y1230H",
            "R320Cr",
            "DD14",
            "DC04",
            "DC03+ZE",
            "DC04EK",
            "DX51D+Z",
            "HC400LA",
            "HXT450X",
            "TH550",
            "TS550",
            "M400-50A",
            "M140-30S",
            "M660-50D",
            "M390-50E",
            "C20D",
            "C2D1",
            "C20D2",
            "C35E",
            "C35R",
            "C35",
            "C85S",
            "C8C",
            "13CrMo4-5",
            "13MnNi6-3",
            "28Mn6",
            "27MnCrB5-2",
            "11SMnPb30",
            "X100CrMoV 5",
            "X38CrMoNb16",
            "X10CrNi18-8",
            "X6CrMoNb17-1",
            "X5CrNiCuNb16-4",
            "X30NiCrN15-1-N5",
            "HS2-9-1-8",
            "HS6-5-2",
            "HS6-5-2C",
        ] {
            let reading = engine()
                .read(input)
                .unwrap_or_else(|e| panic!("{input} : {e}"));
            let reserved: Vec<&str> = reading
                .findings
                .iter()
                .filter(|f| f.severity == Severity::Error || f.code.contains("digit"))
                .chain(
                    reading
                        .findings
                        .iter()
                        .filter(|f| f.code.starts_with("product_symbol")),
                )
                .map(|f| f.code.as_str())
                .collect();
            assert!(reserved.is_empty(), "{input} : {reserved:?}");
        }
    }

    #[test]
    fn les_symboles_additionnels_dependent_du_groupe() {
        // Tableau 2 : H est la temperature elevee, pas le profil creux du
        // tableau 1.
        let reading = engine().read("P265GH").unwrap();
        let groups: Vec<(&str, u8)> = reading
            .suffixes
            .iter()
            .map(|s| (s.code.as_str(), s.group))
            .collect();
        assert_eq!(groups, [("G", 1), ("H", 2)]);
        assert_eq!(reading.suffixes[1].meaning, "température élevée");
        let reading = engine().read("S355J2H").unwrap();
        assert_eq!(reading.suffixes[0].meaning, "profil creux");
        // Tableau 3 : toute lettre du groupe 2 est une classe d'exigence.
        let reading = engine().read("L360NB").unwrap();
        assert_eq!(reading.suffixes[1].meaning, "classe d'exigence B");
        // Tableau 1, groupe 2 : un symbole chimique et 10 × sa teneur.
        let reading = engine().read("S355J2WCu3").unwrap();
        assert_eq!(content(&reading, "Cu"), Some(300));
        // Tableau 12, note d : E2 vaut 0,02 % de soufre au plus.
        let reading = engine().read("C35E2").unwrap();
        assert!(reading.suffixes[0].meaning.contains("soufre 0.02 %"));
        // Tableau 7 : Cr est « allie au chrome », groupe 1 ; HT, groupe 2.
        let reading = engine().read("R350HT").unwrap();
        assert_eq!(
            (reading.suffixes[0].code.as_str(), reading.suffixes[0].group),
            ("HT", 2)
        );
        assert_eq!(engine().read("R320Cr").unwrap().group_value, Some(320));
    }

    #[test]
    fn les_groupes_des_tableaux_8_a_11() {
        // Tableau 8 : le nombre n'est pas une caracteristique.
        let reading = engine().read("DC04").unwrap();
        assert_eq!(reading.group_value, None);
        assert_eq!(reading.group.as_ref().unwrap().table, 8);
        // Tableau 9 : HXT + resistance a la traction.
        let reading = engine().read("HXT450X").unwrap();
        assert_eq!(reading.group_value, Some(450));
        assert!(reading
            .group_value_label
            .as_deref()
            .unwrap()
            .contains("résistance à la traction"));
        assert_eq!(reading.suffixes[0].meaning, "biphasé");
        // Tableau 10 : valeur nominale.
        let reading = engine().read("TS550").unwrap();
        assert!(reading.group_value_label.unwrap().contains("nominale"));
        // Tableau 11 : pertes, epaisseur, type.
        let reading = engine().read("M400-50A").unwrap();
        assert_eq!(reading.group_value, Some(400));
        assert!(reading.group_value_label.unwrap().starts_with("4 W/kg"));
        assert!(reading.parts.iter().any(|p| p.meaning.contains("0.5 mm")));
        assert!(reading
            .parts
            .iter()
            .any(|p| p.meaning.starts_with("grains non orientés")));
        assert!(engine().read("M400-50Z").is_err());
        assert!(engine().read("M400").is_err());
        // Tableau 6 : quatre chiffres, le premier nul sous 1000 MPa.
        let reading = engine().read("Y960C").unwrap();
        assert!(reading.findings.iter().any(|f| f.code == "digit_count"));
        assert!(!engine()
            .read("Y0960C")
            .unwrap()
            .findings
            .iter()
            .any(|f| f.code == "digit_count"));
    }

    #[test]
    fn les_symboles_additionnels_des_tableaux_14_et_15() {
        // Tableau 14 : -N5 = azote entre 0,20 et 1,0 %, 10 × la teneur.
        let reading = engine().read("X30NiCrN15-1-N5").unwrap();
        assert_eq!(content(&reading, "Ni"), Some(15_000));
        assert_eq!(content(&reading, "N"), Some(500));
        assert_eq!(reading.suffixes[0].code, "-N5");
        // Tableau 15 : HS6-5-2C, variante a carbone plus eleve.
        let reading = engine().read("HS6-5-2C").unwrap();
        assert_eq!(content(&reading, "V"), Some(2000));
        assert_eq!(reading.suffixes[0].code, "C");
        // 7.1 : PM, metallurgie des poudres, devant X ou HS.
        let reading = engine().read("PMHS6-5-3-8").unwrap();
        assert!(reading.powder_metallurgy);
        assert_eq!(content(&reading, "Co"), Some(8000));
        // Le tableau 13 n'a pas de symbole additionnel.
        assert!(engine().read("42CrMo4-N5").is_err());
        // X sans element a 5 %, toutes teneurs donnees : signale.
        let reading = engine().read("X5Cr4").unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "high_alloy_under_five"));
    }

    #[test]
    fn la_designation_numerique_se_lit_sans_etre_devinee() {
        let reading = engine().read("1.4301").unwrap();
        assert_eq!(reading.kind, SteelKind::Numeric);
        assert_eq!(reading.parts[1].text, "43");
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "numeric_table_absent"));
    }

    #[test]
    fn les_saisies_fautives_disent_quoi_faire() {
        for (input, expected) in [
            ("", "S355J2"),
            ("S", "attend un nombre"),
            ("Z300", "ne commence"),
            ("42Zz4", "Aucun symbole chimique"),
            ("42Cr4-4", "teneurs pour"),
            ("S355J2Q7Z", "symbole"),
            ("HS6", "de 3 à"),
        ] {
            let err = engine()
                .read(input)
                .err()
                .unwrap_or_else(|| panic!("{input:?} aurait dû échouer"))
                .to_string();
            assert!(err.contains(expected), "{input:?} : {err}");
        }
    }

    #[test]
    fn lexistence_dune_nuance_nest_jamais_affirmee() {
        let reading = engine().read("S355J2").unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "existence_not_claimed"));
        assert_eq!(reading.conclusion.verdict, Verdict::Caution);
        // EN 10025-2 et EN 10027-1 sont verifiees : seules les familles
        // portent encore une reserve.
        assert_eq!(reading.conclusion.warnings.len(), 1);
        // L'edition lue (2005) est remplacee, et chaque lecture le dit.
        let superseded = reading
            .findings
            .iter()
            .find(|f| f.code == "edition_superseded")
            .unwrap();
        assert!(superseded.message.contains("EN 10027-1:2005"));
        assert!(superseded.message.contains("EN 10027-1:2016"));
    }

    #[test]
    fn la_dilatation_est_exacte_au_nanometre() {
        // Acier, Ø100, ΔT = 50 K : 100 mm × 12 µm/(m·K) × 50 = 60 µm.
        let thermal = engine()
            .thermal_fit(Length::from_millimetres(100), 50, "steel", "steel", None)
            .unwrap();
        assert_eq!(thermal.hole_growth, Length::from_micrometres(60));
        assert_eq!(thermal.clearance_shift, Length::ZERO);
    }

    #[test]
    fn un_alesage_en_aluminium_sur_un_arbre_en_acier_prend_du_jeu_a_chaud() {
        // Ø50, ΔT = +80 K : (23 − 12) × 50 × 80 / 1000 = 44 µm de jeu en plus.
        let thermal = engine()
            .thermal_fit(Length::from_millimetres(50), 80, "aluminium", "steel", None)
            .unwrap();
        assert_eq!(thermal.clearance_shift, Length::from_micrometres(44));
        assert_eq!(thermal.clearance_shift_label, "+44 µm");
    }

    #[test]
    fn un_serrage_peut_se_perdre_a_chaud() {
        // Ø50 H7/p6 : serrage de 1 à 42 µm environ. Un logement aluminium a
        // +80 K gagne 44 µm : le serrage disparait.
        let classes = (
            ToleranceClass::parse("H7").unwrap(),
            ToleranceClass::parse("p6").unwrap(),
        );
        let thermal = engine()
            .thermal_fit(
                Length::from_millimetres(50),
                80,
                "aluminium",
                "steel",
                Some(classes),
            )
            .unwrap();
        let fit = thermal.fit.unwrap();
        assert_eq!(fit.cold_kind, FitKind::Interference);
        assert_eq!(fit.hot_kind, FitKind::Clearance);
        assert_eq!(thermal.conclusion.verdict, Verdict::Incompatible);
        assert!(thermal
            .findings
            .iter()
            .any(|f| f.code == "interference_lost"));
        // La provenance croise la famille (non verifiee) et l'ISO 286 (verifiee).
        assert!(thermal
            .provenance
            .references
            .iter()
            .any(|r| r.verification.is_verified()));
    }

    #[test]
    fn un_jeu_peut_disparaitre_a_froid() {
        // Le meme logement aluminium a −40 K perd du jeu : un H7/g6 peut serrer.
        let classes = (
            ToleranceClass::parse("H7").unwrap(),
            ToleranceClass::parse("g6").unwrap(),
        );
        let thermal = engine()
            .thermal_fit(
                Length::from_millimetres(50),
                -40,
                "aluminium",
                "steel",
                Some(classes),
            )
            .unwrap();
        assert!(thermal.findings.iter().any(|f| f.code == "clearance_lost"));
    }

    #[test]
    fn une_famille_inconnue_dit_lesquelles_existent() {
        let err = engine()
            .thermal_fit(Length::from_millimetres(50), 10, "bidule", "steel", None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("aluminium"), "{err}");
    }

    #[test]
    fn les_pourcentages_saffichent_sans_zero_superflu() {
        assert_eq!(percent(1250), "1.25");
        assert_eq!(percent(1000), "1");
        assert_eq!(percent(50), "0.05");
        assert_eq!(percent(420), "0.42");
    }
}
