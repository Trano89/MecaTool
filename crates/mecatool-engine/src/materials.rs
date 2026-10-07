//! Matieres : lecture d'une designation d'acier, proprietes, et effet de la
//! temperature sur un ajustement.
//!
//! # Ce que ce module fait
//!
//! * Il **decompose** une designation symbolique EN 10027-1 — `S355J2`, `C45E`,
//!   `42CrMo4`, `X5CrNi18-10`, `HS6-5-2` — morceau par morceau, en appliquant
//!   les regles de la norme plutot qu'en cherchant dans une liste.
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

use mecatool_core::{
    Conclusion, FitKind, Length, Provenance, ReasoningStep, ToleranceClass, Unit, Verdict,
};
use mecatool_standards::matieres::{
    DesignationRules, GroupNumber, MaterialFamily, MaterialFamilyTable, StructuralGrade,
    StructuralSteelTable, Suffix, UseGroup,
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
    /// Designee par son emploi : `S355J2`.
    UseGroup,
    /// Non alliee : `C45`.
    NonAlloy,
    /// Faiblement alliee : `42CrMo4`.
    LowAlloy,
    /// Fortement alliee : `X5CrNi18-10`.
    HighAlloy,
    /// Acier rapide : `HS6-5-2`.
    HighSpeed,
    /// Designation numerique : `1.4301`.
    Numeric,
}

impl SteelKind {
    pub const fn label_fr(self) -> &'static str {
        match self {
            SteelKind::UseGroup => "désignée par son emploi et ses caractéristiques",
            SteelKind::NonAlloy => "acier non allié, désigné par sa teneur en carbone",
            SteelKind::LowAlloy => "acier faiblement allié, désigné par sa composition",
            SteelKind::HighAlloy => "acier fortement allié, désigné par sa composition",
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

/// Une designation decomposee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteelReading {
    pub input: String,
    pub kind: SteelKind,
    pub kind_label: String,
    /// Vrai pour un acier moule (prefixe `G`).
    pub cast: bool,
    pub parts: Vec<DesignationPart>,
    pub group: Option<UseGroup>,
    /// Le nombre du groupe d'emploi, et ce qu'il porte.
    pub group_value: Option<u32>,
    pub group_value_label: Option<String>,
    pub impact: Option<ImpactReading>,
    pub suffixes: Vec<Suffix>,
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

        // L'etat de livraison suit un « + » : il ne change pas la nuance.
        let (body, delivery) = match trimmed.split_once('+') {
            Some((body, delivery)) => (body.trim(), Some(delivery.trim().to_string())),
            None => (trimmed, None),
        };

        let mut reading = if is_numeric(body) {
            self.numeric(body)
        } else {
            let (cast, core) = match body.strip_prefix('G') {
                Some(rest) if rest.starts_with(|c: char| c.is_ascii_alphanumeric()) => (true, rest),
                _ => (false, body),
            };
            let mut reading = self.symbolic(core, &unparsable)?;
            if cast {
                reading.cast = true;
                reading.parts.insert(
                    0,
                    DesignationPart {
                        text: "G".into(),
                        meaning: "acier moulé".into(),
                    },
                );
            }
            reading
        };
        reading.input = trimmed.to_string();

        if let Some(delivery) = delivery {
            reading.parts.push(DesignationPart {
                text: format!("+{delivery}"),
                meaning: "état de livraison, lu sans être interprété".into(),
            });
            reading.findings.push(Finding::new(
                "delivery_condition",
                Severity::Note,
                format!(
                    "« +{delivery} » désigne un état de livraison. Il ne change pas la nuance ; \
                     sa signification est dans la norme de produit."
                ),
            ));
        }

        self.finish(reading)
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
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return Err(unparsable(format!(
                "« {} » attend un nombre : {}",
                group.letter,
                lowercase_first(&group.note)
            )));
        }
        let value: u32 = digits
            .parse()
            .map_err(|_| unparsable("Nombre illisible.".into()))?;
        let mut tail = &rest[digits.len()..];

        let mut reading = blank(SteelKind::UseGroup);
        let (unit, label) = match group.number {
            GroupNumber::Yield => ("MPa", "limite d'élasticité minimale"),
            GroupNumber::Tensile => ("MPa", "résistance à la traction nominale"),
            GroupNumber::Hardness => ("HBW", "dureté Brinell minimale"),
        };
        reading.parts.push(DesignationPart {
            text: group.letter.clone(),
            meaning: group.name.clone(),
        });
        reading.parts.push(DesignationPart {
            text: digits.clone(),
            meaning: format!("{label} : {value} {unit}"),
        });
        reading.group_value = Some(value);
        reading.group_value_label = Some(format!("{value} {unit} — {label}"));

        // La resilience : une lettre d'energie puis un code de temperature.
        let mut chars = tail.chars();
        if let (Some(energy), Some(code)) = (chars.next(), chars.next()) {
            let energy_def = self
                .rules
                .impact()
                .energies
                .iter()
                .find(|e| e.letter.starts_with(energy));
            let temperature = self
                .rules
                .impact()
                .temperatures
                .iter()
                .find(|t| t.code.starts_with(code));
            if let (Some(e), Some(t)) = (energy_def, temperature) {
                let text = format!("{energy}{code}");
                let label = format!("{} J à {} °C", e.joules, t.celsius);
                reading.parts.push(DesignationPart {
                    text: text.clone(),
                    meaning: format!("résilience : {label}"),
                });
                reading.impact = Some(ImpactReading {
                    code: text,
                    joules: e.joules,
                    celsius: t.celsius,
                    label,
                });
                tail = &tail[2..];
            }
        }

        for symbol in split_suffixes(tail) {
            let code = &symbol[..1];
            let suffix = self.rules.suffix(code).ok_or_else(|| {
                unparsable(format!(
                    "« {symbol} » n'est ni un code de résilience (J2, K2…) ni un symbole \
                     additionnel connu ({}).",
                    self.rules
                        .suffixes()
                        .iter()
                        .map(|s| s.code.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;
            reading.parts.push(DesignationPart {
                text: symbol.clone(),
                meaning: suffix.meaning.clone(),
            });
            reading.suffixes.push(suffix.clone());
        }

        reading.group = Some(group.clone());
        if group.letter == "S" {
            let grade = format!("S{value}");
            if let Some(structural) = self.structural.grade(&grade) {
                if let Some(impact) = &reading.impact {
                    if !structural.qualities.contains(&impact.code) {
                        reading.findings.push(Finding::new(
                            "quality_not_listed",
                            Severity::Caution,
                            format!(
                                "La qualité {} n'est pas listée pour {grade} dans la table \
                                 embarquée ({}). La désignation reste bien formée.",
                                impact.code,
                                structural.qualities.join(", ")
                            ),
                        ));
                    }
                }
                reading.structural = Some(structural.clone());
            }
        }
        reading.family = self.families.family("steel").cloned();
        reading.family_reason = format!("Un {} est un acier non inoxydable.", group.name);
        Ok(reading)
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
            meaning: "acier non allié".into(),
        });
        reading.parts.push(DesignationPart {
            text: digits.clone(),
            meaning: format!("carbone : {}", carbon.label),
        });
        reading.carbon = Some(carbon);
        for symbol in rest[digits.len()..].chars() {
            let code = symbol.to_string();
            let suffix = self.rules.non_alloy_suffix(&code).ok_or_else(|| {
                unparsable(format!(
                    "« {code} » n'est pas un symbole additionnel connu d'un acier non allié."
                ))
            })?;
            reading.parts.push(DesignationPart {
                text: code,
                meaning: suffix.meaning.clone(),
            });
            reading.suffixes.push(suffix.clone());
        }
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
        let numbers: Vec<i64> = if tail.is_empty() {
            Vec::new()
        } else {
            tail.split('-')
                .map(|n| n.trim().parse::<i64>())
                .collect::<core::result::Result<_, _>>()
                .map_err(|_| unparsable(format!("Teneurs illisibles : « {tail} ».")))?
        };
        if numbers.len() > symbols.len() {
            return Err(unparsable(format!(
                "{} teneurs pour {} éléments : chaque nombre se rapporte à un élément, dans \
                 l'ordre.",
                numbers.len(),
                symbols.len()
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
                meaning: "acier fortement allié : un élément au moins atteint 5 %".into(),
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

        // Un faiblement allie dont un element atteint 5 % aurait du s'ecrire en
        // X : la designation se contredit.
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

    fn high_speed(
        &self,
        rest: &str,
        unparsable: &dyn Fn(String) -> EngineError,
    ) -> Result<SteelReading> {
        let numbers: Vec<i64> = rest
            .split('-')
            .map(|n| n.trim().parse::<i64>())
            .collect::<core::result::Result<_, _>>()
            .map_err(|_| unparsable("Un acier rapide s'écrit « HS6-5-2 ».".into()))?;
        let order = self.rules.high_speed_order();
        if numbers.len() < 3 || numbers.len() > order.len() {
            return Err(unparsable(format!(
                "Un acier rapide porte de 3 à {} teneurs, dans l'ordre {}.",
                order.len(),
                order.join(", ")
            )));
        }
        let mut reading = blank(SteelKind::HighSpeed);
        reading.parts.push(DesignationPart {
            text: "HS".into(),
            meaning: "acier rapide".into(),
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
        reading.family = self.families.family("steel").cloned();
        reading.family_reason = "Un acier à outils.".into();
        Ok(reading)
    }

    fn finish(&self, mut reading: SteelReading) -> Result<SteelReading> {
        let provenance = self.provenance();
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
        parts: Vec::new(),
        group: None,
        group_value: None,
        group_value_label: None,
        impact: None,
        suffixes: Vec::new(),
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

/// Les symboles additionnels : une lettre, suivie le cas echeant de chiffres.
fn split_suffixes(text: &str) -> Vec<String> {
    let mut symbols: Vec<String> = Vec::new();
    for c in text.chars() {
        match symbols.last_mut() {
            Some(last) if c.is_ascii_digit() => last.push(c),
            _ => symbols.push(c.to_string()),
        }
    }
    symbols
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
    fn lit_un_acier_moule_et_un_etat_de_livraison() {
        let reading = engine().read("GX5CrNi19-10").unwrap();
        assert!(reading.cast);
        let reading = engine().read("S235JR+AR").unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "delivery_condition"));
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
        assert_eq!(reading.conclusion.warnings.len(), 3);
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
