//! Lecture et controle d'une specification geometrique.
//!
//! # Ce que ce module fait
//!
//! Il lit un cadre de tolerance ecrit en clair — `⟂ 0.05 A`, `⌖ ø0.2 (M) A B C` —
//! et repond a trois questions :
//!
//! 1. la specification est-elle **complete** ? une reference manque-t-elle, ou
//!    y en a-t-il une la ou la caracteristique n'en prend pas ?
//! 2. la valeur est-elle **cotee comme il faut** ? une zone cylindrique se cote
//!    en diametre, une zone entre deux plans en largeur ;
//! 3. plusieurs specifications posees sur le meme element **se recouvrent**-elles,
//!    au point que l'une rende l'autre inoperante ?
//!
//! # Ce que ce module ne fait pas
//!
//! Il ne propose **aucune valeur**. L'ISO 1101 n'en donne pas : elle definit un
//! vocabulaire, et la valeur releve du concepteur. Un module qui suggererait
//! « 0,05 conviendrait ici » inventerait une regle normative.
//!
//! Il ne juge pas non plus l'applicabilite d'un modificateur a une
//! caracteristique : la source cataloguee ne l'enonce pas, et Mecatol ne comble
//! pas ce qu'elle ne dit pas.
//!
//! # La convention de saisie
//!
//! Sur un dessin, un modificateur est entoure. En texte brut, `A` serait donc
//! ambigu : c'est a la fois la lettre de reference la plus courante et le
//! symbole de l'element derive. Mecatol tranche par la forme :
//!
//! - une lettre **seule et capitale** est une reference specifiee (`A`, `B`) ;
//! - un modificateur s'ecrit **entoure** (`Ⓜ`) ou **entre parentheses** (`(M)`) ;
//! - les modificateurs de deux lettres ou plus (`CZ`, `ACS`, `UF`) n'ont pas
//!   besoin de marque : aucune reference ne s'ecrit ainsi.

use mecatol_core::{Conclusion, Length, Provenance, ReasoningStep, Unit, Verdict};
use mecatol_standards::{Characteristic, CharacteristicTable, DatumRule, ZoneGeometry};
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::format;

/// Une specification geometrique, telle qu'elle se lit dans un cadre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeometricSpec {
    /// Identifiant de la caracteristique dans le catalogue.
    pub characteristic: String,
    /// La valeur de tolerance. Jamais negative.
    pub value: Length,
    /// Vrai si la valeur porte un `ø` : la zone se cote alors en diametre.
    pub diametral: bool,
    /// Les references specifiees, dans l'ordre de lecture du cadre.
    pub datums: Vec<String>,
    /// Les modificateurs releves, sans jugement d'applicabilite.
    pub modifiers: Vec<String>,
    /// La specification telle qu'elle a ete saisie.
    pub input: String,
}

/// Gravite d'un constat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// La specification est fautive au regard de la source.
    Error,
    /// Mecatol ne peut pas trancher, et dit pourquoi.
    Caution,
    /// Rien a corriger : une precision utile a la lecture.
    Note,
}

impl Severity {
    pub const fn verdict(self) -> Verdict {
        match self {
            Severity::Error => Verdict::Incompatible,
            Severity::Caution => Verdict::Caution,
            Severity::Note => Verdict::Compatible,
        }
    }
}

/// Un constat sur une specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Identifiant stable, pour les tests et le style de l'interface.
    pub code: String,
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    fn new(code: &str, severity: Severity, message: impl Into<String>) -> Self {
        Finding {
            code: code.to_string(),
            severity,
            message: message.into(),
        }
    }
}

/// Le resultat de la lecture d'une specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecAnalysis {
    pub spec: GeometricSpec,
    /// La caracteristique reconnue, telle que le catalogue la decrit.
    pub characteristic: Characteristic,
    /// Le cadre reconstitue, normalise : `⟂ 0,05 A`.
    pub designation: String,
    pub findings: Vec<Finding>,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Deux specifications posees sur le meme element, et leur recouvrement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlap {
    /// La specification la plus large, celle qui borne l'autre.
    pub wider: String,
    /// La specification bornee.
    pub narrower: String,
    pub finding: Finding,
}

/// Plusieurs specifications posees sur un meme element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupAnalysis {
    pub specs: Vec<SpecAnalysis>,
    pub overlaps: Vec<Overlap>,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Nombre maximal de specifications comparees d'un coup.
///
/// Au-dela, le tableau croise devient illisible et cesse d'aider a decider.
pub const MAX_SPECS: usize = 8;

/// Le moteur de lecture des specifications geometriques.
#[derive(Debug)]
pub struct GeometricEngine {
    table: &'static CharacteristicTable,
}

impl GeometricEngine {
    pub fn new() -> Result<Self> {
        Ok(GeometricEngine {
            table: CharacteristicTable::embedded()?,
        })
    }

    pub fn table(&self) -> &'static CharacteristicTable {
        self.table
    }

    fn provenance(&self) -> Provenance {
        Provenance::new().with(self.table.standard().clone())
    }

    /// Lit une specification ecrite en clair.
    pub fn parse(&self, input: &str) -> Result<GeometricSpec> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(EngineError::Unparsable {
                input: input.to_string(),
                hint: "Indiquez une caractéristique, une valeur, et les références \
                       éventuelles : par exemple « ⟂ 0.05 A »."
                    .into(),
            });
        }

        let tokens: Vec<&str> = trimmed
            .split(|c: char| c.is_whitespace() || c == '|')
            .filter(|t| !t.is_empty())
            .collect();

        // La caracteristique peut s'ecrire en plusieurs mots (« profil de ligne »).
        // On essaie donc le prefixe le plus long d'abord : « profil de ligne »
        // avant « profil », sans quoi la suite serait lue comme une valeur.
        let (characteristic, rest) = self.take_characteristic(&tokens, trimmed)?;

        let mut value: Option<(Length, bool)> = None;
        let mut datums = Vec::new();
        let mut modifiers = Vec::new();

        for token in rest {
            if let Some(modifier) = read_modifier(token) {
                modifiers.push(modifier);
            } else if value.is_none() {
                value = Some(read_value(token, trimmed)?);
            } else if is_datum(token) {
                datums.push(token.to_string());
            } else {
                return Err(EngineError::Unparsable {
                    input: input.to_string(),
                    hint: format!(
                        "« {token} » n'est ni une référence spécifiée ni un modificateur. \
                         Une référence s'écrit en capitales (A, B, A-B) ; un modificateur \
                         s'écrit entouré ou entre parenthèses, par exemple (M)."
                    ),
                });
            }
        }

        let (value, diametral) = value.ok_or_else(|| EngineError::Unparsable {
            input: input.to_string(),
            hint: format!(
                "La valeur de tolérance manque après « {} ».",
                characteristic.name
            ),
        })?;

        Ok(GeometricSpec {
            characteristic: characteristic.id.clone(),
            value,
            diametral,
            datums,
            modifiers,
            input: trimmed.to_string(),
        })
    }

    /// Reconnait la caracteristique en tete de saisie.
    ///
    /// Un symbole peut designer plusieurs caracteristiques : `⌒` vaut profil
    /// d'une ligne en forme, en orientation et en position. La presence d'une
    /// reference specifiee suffit a departager, puisque c'est exactement ce qui
    /// distingue ces trois emplois ; Mecatol tranche donc apres avoir vu la
    /// suite, au lieu de refuser une saisie parfaitement lisible.
    fn take_characteristic<'a>(
        &self,
        tokens: &'a [&'a str],
        input: &str,
    ) -> Result<(&'static Characteristic, &'a [&'a str])> {
        let max = tokens.len().min(4);
        for take in (1..=max).rev() {
            let needle = tokens[..take].join(" ");
            let found = self.table.lookup(&needle);
            if found.is_empty() {
                continue;
            }
            let rest = &tokens[take..];
            let chosen = if found.len() == 1 {
                found[0]
            } else {
                let has_datum = rest.iter().any(|t| is_datum(t));
                // Sans reference, seule la lecture « forme » est possible ; avec
                // reference, on retient la premiere lecture qui en accepte une.
                let wanted = found.iter().copied().find(|c| {
                    if has_datum {
                        c.datum != DatumRule::None
                    } else {
                        c.datum == DatumRule::None
                    }
                });
                match wanted {
                    Some(c) => c,
                    None => {
                        return Err(EngineError::Ambiguous {
                            input: input.to_string(),
                            question: format!(
                                "Le symbole « {} » désigne plusieurs caractéristiques : {}. \
                                 Précisez laquelle.",
                                needle,
                                found
                                    .iter()
                                    .map(|c| c.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        })
                    }
                }
            };
            return Ok((chosen, rest));
        }

        Err(EngineError::Unparsable {
            input: input.to_string(),
            hint: format!(
                "« {} » n'est pas une caractéristique géométrique connue. \
                 Mecatol en reconnaît {} : consultez le catalogue.",
                tokens[0],
                self.table.characteristics().len()
            ),
        })
    }

    /// Controle une specification et explique ce qu'elle signifie.
    pub fn analyse(&self, spec: &GeometricSpec) -> Result<SpecAnalysis> {
        let characteristic =
            self.table
                .by_id(&spec.characteristic)
                .ok_or_else(|| EngineError::Unparsable {
                    input: spec.characteristic.clone(),
                    hint: "Caractéristique inconnue du catalogue.".into(),
                })?;

        let mut findings = Vec::new();
        self.check_datums(spec, characteristic, &mut findings);
        self.check_diameter(spec, characteristic, &mut findings);
        self.check_value(spec, &mut findings);
        self.check_ted(characteristic, &mut findings);
        self.check_modifiers(spec, &mut findings);

        let verdict = findings.iter().fold(Verdict::Compatible, |acc, f| {
            acc.worst(f.severity.verdict())
        });

        let designation = designation(characteristic, spec);
        let detail = match verdict {
            Verdict::Compatible => format!(
                "{designation} se lit : {}",
                self.zone_sentence(characteristic, spec)
            ),
            _ => format!(
                "{designation} appelle {} remarque{}.",
                findings.len(),
                if findings.len() > 1 { "s" } else { "" }
            ),
        };

        let mut conclusion = Conclusion::new(verdict, detail);
        conclusion.why = self.reasoning(characteristic, spec);
        conclusion.warnings = self.provenance().warnings_fr();

        Ok(SpecAnalysis {
            spec: spec.clone(),
            characteristic: characteristic.clone(),
            designation,
            findings,
            conclusion,
            provenance: self.provenance(),
        })
    }

    fn check_datums(
        &self,
        spec: &GeometricSpec,
        characteristic: &Characteristic,
        findings: &mut Vec<Finding>,
    ) {
        match (characteristic.datum, spec.datums.is_empty()) {
            (DatumRule::Required, true) => findings.push(Finding::new(
                "datum_missing",
                Severity::Error,
                format!(
                    "{} est une tolérance de {} : elle exige au moins une référence \
                     spécifiée. Sans référence, il n'y a rien par rapport à quoi \
                     mesurer l'écart.",
                    capitalise(&characteristic.name),
                    characteristic.family.name_fr()
                ),
            )),
            (DatumRule::None, false) => findings.push(Finding::new(
                "datum_unexpected",
                Severity::Error,
                format!(
                    "{} est une tolérance de forme : elle ne prend pas de référence \
                     spécifiée. Elle limite l'écart de l'élément par rapport à sa \
                     propre forme géométrique, pas par rapport à un autre élément.",
                    capitalise(&characteristic.name)
                ),
            )),
            (DatumRule::Optional, true) => findings.push(Finding::new(
                "datum_optional_absent",
                Severity::Note,
                format!(
                    "{} admet l'absence de référence spécifiée. Sans référence, la \
                     zone se place par rapport aux dimensions théoriques exactes \
                     seules, et non par rapport à un système de références.",
                    capitalise(&characteristic.name)
                ),
            )),
            _ => {}
        }
    }

    fn check_diameter(
        &self,
        spec: &GeometricSpec,
        characteristic: &Characteristic,
        findings: &mut Vec<Finding>,
    ) {
        if spec.diametral && !characteristic.allows_diametral() {
            let zone = characteristic.zones[0].geometry;
            findings.push(Finding::new(
                "diameter_forbidden",
                Severity::Error,
                format!(
                    "Le ø n'a pas de sens ici : la zone de {} est {}, elle se cote \
                     donc en largeur et non en diamètre.",
                    characteristic.name,
                    zone.name_fr()
                ),
            ));
        } else if !spec.diametral && characteristic.always_diametral() {
            findings.push(Finding::new(
                "diameter_missing",
                Severity::Error,
                format!(
                    "La zone de {} est toujours cylindrique : la valeur se cote en \
                     diamètre, et doit donc porter un ø.",
                    characteristic.name
                ),
            ));
        } else if characteristic.allows_diametral() && !characteristic.always_diametral() {
            // Cas le plus interessant : les deux notations sont correctes et ne
            // veulent pas dire la meme chose. Mecatol ne voit pas le dessin, donc
            // il n'arbitre pas — il explique ce que la saisie engage.
            let (with, without) = diametral_pair(characteristic);
            findings.push(Finding::new(
                "diameter_changes_meaning",
                Severity::Note,
                format!(
                    "Avec un ø, la zone est {} et s'applique à {}. Sans ø, elle est \
                     {} et s'applique à {}. Votre saisie retient la {}.",
                    ZoneGeometry::Cylinder.name_fr(),
                    with,
                    non_diametral_geometry(characteristic).name_fr(),
                    without,
                    if spec.diametral {
                        "première"
                    } else {
                        "seconde"
                    }
                ),
            ));
        }
    }

    fn check_value(&self, spec: &GeometricSpec, findings: &mut Vec<Finding>) {
        if spec.value > Length::ZERO {
            return;
        }
        // Une tolerance nulle existe : elle se combine a l'exigence du maximum
        // ou du minimum de matiere, et la source en donne l'exemple. Hors de ce
        // cas, une zone d'epaisseur nulle est irrealisable.
        let material = spec
            .modifiers
            .iter()
            .any(|m| m == "M" || m == "L" || m == "R");
        if !material {
            findings.push(Finding::new(
                "zero_without_material_condition",
                Severity::Error,
                "Une tolérance nulle n'a de sens qu'associée à l'exigence du maximum \
                 ou du minimum de matière, qui rend la zone fonction de la dimension \
                 réelle. Seule, elle exigerait une géométrie parfaite."
                    .to_string(),
            ));
        }
    }

    fn check_ted(&self, characteristic: &Characteristic, findings: &mut Vec<Finding>) {
        if !characteristic.requires_ted {
            return;
        }
        findings.push(Finding::new(
            "ted_required",
            Severity::Caution,
            format!(
                "{} ne s'emploie qu'avec des dimensions théoriques exactes (TED) \
                 explicites, qui placent la zone. Mecatol ne voit pas le dessin : \
                 vérifiez qu'elles y figurent, encadrées.",
                capitalise(&characteristic.name)
            ),
        ));
    }

    fn check_modifiers(&self, spec: &GeometricSpec, findings: &mut Vec<Finding>) {
        if spec.modifiers.is_empty() {
            return;
        }
        let known: Vec<String> = spec
            .modifiers
            .iter()
            .map(
                |symbol| match self.table.modifiers().iter().find(|m| &m.symbol == symbol) {
                    Some(m) => format!("{} ({}, {})", m.symbol, m.name, m.defined_by),
                    None => format!("{symbol} (non catalogué)"),
                },
            )
            .collect();

        findings.push(Finding::new(
            "modifiers_not_checked",
            Severity::Caution,
            format!(
                "Modificateurs relevés : {}. Mecatol les restitue mais ne vérifie pas \
                 qu'ils s'appliquent à cette caractéristique : la source consultée les \
                 catalogue sans énoncer de règle d'applicabilité complète.",
                known.join(" ; ")
            ),
        ));
    }

    fn zone_sentence(&self, characteristic: &Characteristic, spec: &GeometricSpec) -> String {
        let zone = self.chosen_zone(characteristic, spec);
        let value = format::mm_trimmed(spec.value);
        let prefix = if spec.diametral { "ø" } else { "" };
        format!(
            "{} — zone {}, {prefix}{value} mm.",
            zone.definition,
            zone.geometry.name_fr()
        )
    }

    /// La zone que la saisie designe, parmi celles que la source decrit.
    fn chosen_zone<'a>(
        &self,
        characteristic: &'a Characteristic,
        spec: &GeometricSpec,
    ) -> &'a mecatol_standards::ZoneDefinition {
        characteristic
            .zones
            .iter()
            .find(|z| z.geometry.is_diametral() == spec.diametral)
            .unwrap_or(&characteristic.zones[0])
    }

    fn reasoning(
        &self,
        characteristic: &Characteristic,
        spec: &GeometricSpec,
    ) -> Vec<ReasoningStep> {
        let mut steps = vec![
            ReasoningStep::new("Caractéristique")
                .with_expression(characteristic.symbol.clone())
                .with_value(format!(
                    "{} — tolérance de {}",
                    characteristic.name,
                    characteristic.family.name_fr()
                )),
            ReasoningStep::new("Référence spécifiée")
                .with_expression(match characteristic.datum {
                    DatumRule::None => "aucune".to_string(),
                    DatumRule::Required => "requise".to_string(),
                    DatumRule::Optional => "au choix".to_string(),
                })
                .with_value(if spec.datums.is_empty() {
                    "aucune indiquée".to_string()
                } else {
                    spec.datums.join(", ")
                }),
            ReasoningStep::new("Zone de tolérance")
                .with_expression(self.chosen_zone(characteristic, spec).geometry.name_fr())
                .with_value(format!(
                    "{}{} mm",
                    if spec.diametral { "ø" } else { "" },
                    format::mm_trimmed(spec.value)
                )),
            ReasoningStep::new("Source")
                .with_expression(self.table.standard().citation())
                .with_value(format!("page {}", characteristic.page)),
        ];

        if let Some(family) = self.table.family(characteristic.family) {
            if !family.limits.is_empty() {
                steps.push(
                    ReasoningStep::new("Ce que cette tolérance borne aussi")
                        .with_expression(
                            family
                                .limits
                                .iter()
                                .map(|f| f.name_fr())
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                        .with_value(family.note.clone()),
                );
            }
        }

        steps
    }

    /// Lit plusieurs specifications posees sur un meme element.
    pub fn analyse_group(&self, specs: &[GeometricSpec]) -> Result<GroupAnalysis> {
        if specs.is_empty() {
            return Err(EngineError::Unparsable {
                input: String::new(),
                hint: "Indiquez au moins une spécification géométrique.".into(),
            });
        }
        if specs.len() > MAX_SPECS {
            return Err(EngineError::Ambiguous {
                input: format!("{} spécifications", specs.len()),
                question: format!(
                    "Mecatol en compare {MAX_SPECS} au plus : au-delà, le recoupement \
                     cesse d'aider à décider. Retirez-en {}.",
                    specs.len() - MAX_SPECS
                ),
            });
        }

        let analyses: Vec<SpecAnalysis> = specs
            .iter()
            .map(|s| self.analyse(s))
            .collect::<Result<_>>()?;

        let overlaps = self.overlaps(&analyses);

        let verdict = analyses
            .iter()
            .map(|a| a.conclusion.verdict)
            .chain(overlaps.iter().map(|o| o.finding.severity.verdict()))
            .fold(Verdict::Compatible, Verdict::worst);

        let detail = if overlaps.is_empty() {
            format!(
                "{} spécification{} sur cet élément, sans recouvrement signalé.",
                analyses.len(),
                if analyses.len() > 1 { "s" } else { "" }
            )
        } else {
            format!(
                "{} recouvrement{} entre ces spécifications.",
                overlaps.len(),
                if overlaps.len() > 1 { "s" } else { "" }
            )
        };

        let mut conclusion = Conclusion::new(verdict, detail);
        conclusion.warnings = self.provenance().warnings_fr();

        Ok(GroupAnalysis {
            specs: analyses,
            overlaps,
            conclusion,
            provenance: self.provenance(),
        })
    }

    /// Cherche les specifications qu'une autre rend inoperantes.
    ///
    /// La source enonce un emboitement : une tolerance d'orientation borne aussi
    /// l'ecart de forme, une tolerance de position borne forme et orientation.
    /// Il suit qu'une specification plus large que celle qui la borne deja
    /// n'ajoute rien au dessin.
    ///
    /// La comparaison n'a cependant de sens que **entre zones de meme nature**.
    /// Une zone cylindrique ø0,1 et une zone de 0,1 entre deux plans ne sont pas
    /// la meme chose : les confronter par leurs seuls nombres conclurait de
    /// travers. Quand les natures different, Mecatol le dit au lieu de trancher.
    fn overlaps(&self, analyses: &[SpecAnalysis]) -> Vec<Overlap> {
        let mut overlaps = Vec::new();
        for wider in analyses {
            for narrower in analyses {
                if std::ptr::eq(wider, narrower) {
                    continue;
                }
                if !self
                    .table
                    .limits(wider.characteristic.family, narrower.characteristic.family)
                {
                    continue;
                }

                if wider.spec.diametral != narrower.spec.diametral {
                    overlaps.push(Overlap {
                        wider: wider.designation.clone(),
                        narrower: narrower.designation.clone(),
                        finding: Finding::new(
                            "overlap_not_comparable",
                            Severity::Caution,
                            format!(
                                "{} borne aussi la {}, mais les deux zones ne sont pas \
                                 de même nature : l'une se cote en diamètre, l'autre en \
                                 largeur. Mecatol ne les compare pas.",
                                wider.designation,
                                narrower.characteristic.family.name_fr()
                            ),
                        ),
                    });
                    continue;
                }

                if wider.spec.value <= narrower.spec.value {
                    overlaps.push(Overlap {
                        wider: wider.designation.clone(),
                        narrower: narrower.designation.clone(),
                        finding: Finding::new(
                            "overlap_inoperative",
                            Severity::Caution,
                            format!(
                                "{} n'ajoute rien : {} borne déjà la {} de cet élément, \
                                 et le fait plus serré ({} contre {}).",
                                narrower.designation,
                                wider.designation,
                                narrower.characteristic.family.name_fr(),
                                format::mm_trimmed(wider.spec.value),
                                format::mm_trimmed(narrower.spec.value),
                            ),
                        ),
                    });
                }
            }
        }
        overlaps
    }
}

/// Le cadre reconstitue sous sa forme normalisee.
fn designation(characteristic: &Characteristic, spec: &GeometricSpec) -> String {
    let mut out = format!(
        "{} {}{}",
        characteristic.symbol,
        if spec.diametral { "ø" } else { "" },
        format::mm_trimmed(spec.value)
    );
    for modifier in &spec.modifiers {
        out.push_str(&format!(" ({modifier})"));
    }
    for datum in &spec.datums {
        out.push(' ');
        out.push_str(datum);
    }
    out
}

/// Les elements auxquels s'appliquent la zone cylindrique et l'autre.
fn diametral_pair(characteristic: &Characteristic) -> (String, String) {
    let with = characteristic
        .zones
        .iter()
        .find(|z| z.geometry.is_diametral())
        .map(|z| z.feature.clone())
        .unwrap_or_else(|| "un axe".into());
    let without = characteristic
        .zones
        .iter()
        .find(|z| !z.geometry.is_diametral())
        .map(|z| z.feature.clone())
        .unwrap_or_else(|| "une surface".into());
    (with, without)
}

fn non_diametral_geometry(characteristic: &Characteristic) -> ZoneGeometry {
    characteristic
        .zones
        .iter()
        .find(|z| !z.geometry.is_diametral())
        .map(|z| z.geometry)
        .unwrap_or(ZoneGeometry::TwoParallelPlanes)
}

/// Lit une valeur, avec son eventuel prefixe de diametre.
fn read_value(token: &str, input: &str) -> Result<(Length, bool)> {
    let (body, diametral) = match token
        .strip_prefix('ø')
        .or_else(|| token.strip_prefix('Ø'))
        .or_else(|| token.strip_prefix('\u{2300}'))
        .or_else(|| token.strip_prefix("dia"))
    {
        Some(rest) => (rest, true),
        None => (token, false),
    };

    let value =
        Length::parse(body, Unit::Millimetre).map_err(|source| EngineError::Unparsable {
            input: input.to_string(),
            hint: format!("« {token} » ne se lit pas comme une valeur en millimètres : {source}"),
        })?;

    if value < Length::ZERO {
        return Err(EngineError::Unparsable {
            input: input.to_string(),
            hint: "Une tolérance géométrique est une largeur de zone : elle ne peut \
                   pas être négative."
                .into(),
        });
    }

    Ok((value, diametral))
}

/// Vrai si le jeton se lit comme une reference specifiee.
///
/// Une reference est une lettre capitale, eventuellement composee (`A-B` pour
/// une reference commune) ou indicee (`A1` pour une reference partielle).
fn is_datum(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    token
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

/// Lit un modificateur, quelle que soit sa notation.
fn read_modifier(token: &str) -> Option<String> {
    // Forme entre parentheses : (M), (CZ).
    if let Some(inner) = token.strip_prefix('(').and_then(|t| t.strip_suffix(')')) {
        return (!inner.is_empty()).then(|| inner.to_ascii_uppercase());
    }

    // Forme entouree : les capitales cerclees Unicode, Ⓐ a Ⓩ.
    let letters: Option<String> = token
        .chars()
        .map(|c| {
            let code = c as u32;
            (0x24B6..=0x24CF)
                .contains(&code)
                .then(|| char::from_u32(code - 0x24B6 + u32::from(b'A')))
                .flatten()
        })
        .collect();
    if let Some(letters) = letters {
        if !letters.is_empty() {
            return Some(letters);
        }
    }

    // Modificateurs de deux lettres ou plus : aucune reference ne s'ecrit ainsi,
    // la marque est donc superflue.
    const MULTI: [&str; 9] = ["CZ", "SZ", "UZ", "OZ", "VA", "ACS", "UF", "LD", "MD"];
    MULTI
        .contains(&token.to_ascii_uppercase().as_str())
        .then(|| token.to_ascii_uppercase())
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> GeometricEngine {
        GeometricEngine::new().expect("le catalogue doit charger")
    }

    fn spec(input: &str) -> GeometricSpec {
        engine()
            .parse(input)
            .unwrap_or_else(|e| panic!("{input} : {e}"))
    }

    fn analyse(input: &str) -> SpecAnalysis {
        let e = engine();
        e.analyse(&spec(input)).unwrap()
    }

    fn codes(analysis: &SpecAnalysis) -> Vec<&str> {
        analysis.findings.iter().map(|f| f.code.as_str()).collect()
    }

    #[test]
    fn lit_un_cadre_simple() {
        let s = spec("⟂ 0.05 A");
        assert_eq!(s.characteristic, "perpendicularity");
        assert_eq!(s.value, Length::from_micrometres(50));
        assert!(!s.diametral);
        assert_eq!(s.datums, ["A"]);
    }

    #[test]
    fn accepte_les_alias_en_clair() {
        assert_eq!(spec("perp 0.05 A").characteristic, "perpendicularity");
        assert_eq!(spec("// 0.1 B").characteristic, "parallelism");
        assert_eq!(spec("plan 0.02").characteristic, "flatness");
    }

    #[test]
    fn lit_le_prefixe_de_diametre() {
        assert!(spec("⌖ ø0.2 A B C").diametral);
        assert!(spec("⌖ Ø0.2 A B C").diametral);
        assert!(!spec("⌖ 0.2 A B C").diametral);
    }

    #[test]
    fn distingue_une_reference_dun_modificateur() {
        // Le coeur de la convention de saisie : « A » seul est une reference,
        // « (M) » est un modificateur. Sans cette regle, l'element derive Ⓐ et
        // la reference A seraient indiscernables en texte brut.
        let s = spec("⌖ ø0.2 (M) A B C");
        assert_eq!(s.datums, ["A", "B", "C"]);
        assert_eq!(s.modifiers, ["M"]);
    }

    #[test]
    fn accepte_les_capitales_cerclees() {
        let s = spec("⌖ ø0.2 \u{24C2} A");
        assert_eq!(s.modifiers, ["M"]);
        assert_eq!(s.datums, ["A"]);
    }

    #[test]
    fn accepte_les_modificateurs_de_plusieurs_lettres_sans_marque() {
        let s = spec("⌓ 0.2 CZ A B");
        assert_eq!(s.modifiers, ["CZ"]);
        assert_eq!(s.datums, ["A", "B"]);
    }

    #[test]
    fn accepte_une_reference_commune_et_une_reference_partielle() {
        assert_eq!(spec("◎ ø0.08 A-B").datums, ["A-B"]);
        assert_eq!(spec("⟂ 0.05 A1").datums, ["A1"]);
    }

    #[test]
    fn refuse_une_valeur_negative() {
        let err = engine().parse("⟂ -0.05 A").unwrap_err();
        assert!(err.to_string().contains("négative"), "{err}");
    }

    #[test]
    fn refuse_une_caracteristique_inconnue() {
        let err = engine().parse("bidule 0.1").unwrap_err();
        assert!(matches!(err, EngineError::Unparsable { .. }), "{err}");
    }

    #[test]
    fn signale_une_reference_manquante() {
        let a = analyse("⟂ 0.05");
        assert!(codes(&a).contains(&"datum_missing"));
        assert_eq!(a.conclusion.verdict, Verdict::Incompatible);
    }

    #[test]
    fn signale_une_reference_de_trop() {
        // La planeite ne se compare a rien d'autre : une reference ici trahit
        // une confusion avec le parallelisme.
        let a = analyse("⏥ 0.05 A");
        assert!(codes(&a).contains(&"datum_unexpected"));
        assert_eq!(a.conclusion.verdict, Verdict::Incompatible);
    }

    #[test]
    fn signale_un_diametre_impossible() {
        let a = analyse("⏥ ø0.05");
        assert!(codes(&a).contains(&"diameter_forbidden"));
    }

    #[test]
    fn signale_un_diametre_manquant() {
        // La coaxialite n'a qu'une zone, cylindrique : la valeur est un diametre.
        let a = analyse("◎ 0.08 A-B");
        assert!(codes(&a).contains(&"diameter_missing"));
    }

    #[test]
    fn explique_le_diametre_quand_les_deux_notations_valent() {
        // La perpendicularite accepte les deux, et elles ne disent pas la meme
        // chose. Mecatol ne voit pas le dessin : il explique au lieu d'arbitrer.
        let a = analyse("⟂ 0.05 A");
        assert!(codes(&a).contains(&"diameter_changes_meaning"));
        assert_eq!(a.conclusion.verdict, Verdict::Compatible);

        let message = &a
            .findings
            .iter()
            .find(|f| f.code == "diameter_changes_meaning")
            .unwrap()
            .message;
        assert!(message.contains("cylindrique"));
        assert!(message.contains("seconde"));
    }

    #[test]
    fn reclame_les_ted_quand_la_caracteristique_les_exige() {
        let a = analyse("⌖ ø0.2 A B C");
        assert!(codes(&a).contains(&"ted_required"));
        assert_eq!(a.conclusion.verdict, Verdict::Caution);
    }

    #[test]
    fn une_tolerance_nulle_exige_une_condition_de_matiere() {
        let sans = analyse("⟂ 0 A");
        assert!(codes(&sans).contains(&"zero_without_material_condition"));

        // Avec le maximum de matiere, la source elle-meme en donne l'exemple.
        let avec = analyse("⟂ ø0 (M) A");
        assert!(!codes(&avec).contains(&"zero_without_material_condition"));
    }

    #[test]
    fn restitue_les_modificateurs_sans_les_juger() {
        let a = analyse("⌖ ø0.2 (M) A B C");
        let finding = a
            .findings
            .iter()
            .find(|f| f.code == "modifiers_not_checked")
            .unwrap();
        // Il doit nommer la norme qui definit le symbole, qui n'est pas l'ISO 1101.
        assert!(finding.message.contains("ISO 2692"));
        // Et dire explicitement qu'il ne verifie pas l'applicabilite.
        assert!(finding.message.contains("ne vérifie pas"));
    }

    #[test]
    fn le_symbole_partage_se_tranche_par_la_reference() {
        // « ⌒ » vaut profil d'une ligne en forme, en orientation et en position.
        // Sans reference, seule la lecture « forme » tient.
        assert_eq!(spec("⌒ 0.04").characteristic, "line_profile_form");
        // Avec reference, c'est l'une des deux autres.
        let avec = spec("⌒ 0.04 A");
        assert_ne!(avec.characteristic, "line_profile_form");
    }

    #[test]
    fn la_conclusion_porte_toujours_la_reserve_de_source() {
        // La donnee vient d'un recueil : la reserve doit suivre le resultat
        // jusqu'au bout, pas rester dans le fichier de donnees.
        let a = analyse("⟂ 0.05 A");
        assert!(!a.conclusion.warnings.is_empty());
        assert!(a.conclusion.warnings[0].contains("recueil"));
        assert!(!a.provenance.is_fully_verified());
    }

    #[test]
    fn le_raisonnement_cite_la_page() {
        let a = analyse("⟂ 0.05 A");
        let source = a
            .conclusion
            .why
            .iter()
            .find(|s| s.label == "Source")
            .unwrap();
        assert_eq!(source.expression.as_deref(), Some("ISO 1101"));
        assert_eq!(source.value.as_deref(), Some("page 186"));
    }

    #[test]
    fn une_orientation_plus_serree_rend_la_forme_inoperante() {
        let e = engine();
        // Le parallelisme borne aussi la planeite. A 0,02 contre 0,05, la
        // planeite n'ajoute rien au dessin.
        let group = e
            .analyse_group(&[spec("// 0.02 A"), spec("⏥ 0.05")])
            .unwrap();
        assert_eq!(group.overlaps.len(), 1);
        assert_eq!(group.overlaps[0].finding.code, "overlap_inoperative");
        assert!(group.overlaps[0].narrower.starts_with('⏥'));
    }

    #[test]
    fn une_forme_plus_serree_que_lorientation_garde_son_utilite() {
        let e = engine();
        let group = e
            .analyse_group(&[spec("// 0.05 A"), spec("⏥ 0.02")])
            .unwrap();
        assert!(group.overlaps.is_empty());
    }

    #[test]
    fn deux_zones_de_natures_differentes_ne_se_comparent_pas() {
        // Une zone cylindrique ø0,02 et une zone de 0,05 entre deux plans ne
        // sont pas commensurables : comparer les nombres conclurait de travers.
        let e = engine();
        let group = e
            .analyse_group(&[spec("⟂ ø0.02 A"), spec("⏤ 0.05")])
            .unwrap();
        assert_eq!(group.overlaps.len(), 1);
        assert_eq!(group.overlaps[0].finding.code, "overlap_not_comparable");
        assert!(group.overlaps[0]
            .finding
            .message
            .contains("ne les compare pas"));
    }

    #[test]
    fn le_sens_de_lemboitement_nest_pas_symetrique() {
        // La forme ne borne pas l'orientation : l'inverse du test precedent ne
        // doit produire aucun recouvrement, meme a valeur plus serree.
        let e = engine();
        let group = e
            .analyse_group(&[spec("⏥ 0.01"), spec("// 0.05 A")])
            .unwrap();
        assert!(group.overlaps.is_empty());
    }

    #[test]
    fn refuse_un_groupe_vide_ou_trop_grand() {
        let e = engine();
        assert!(e.analyse_group(&[]).is_err());
        let many: Vec<_> = (0..MAX_SPECS + 1).map(|_| spec("⏥ 0.05")).collect();
        assert!(e.analyse_group(&many).is_err());
    }

    #[test]
    fn la_designation_se_relit() {
        assert_eq!(analyse("perp 0.05 A").designation, "⟂ 0.05 A");
        assert_eq!(analyse("⌖ ø0.2 (M) A B C").designation, "⌖ ø0.2 (M) A B C");
    }

    #[test]
    fn aucune_valeur_nest_jamais_suggeree() {
        // Garde-fou : le moteur explique une valeur saisie, il n'en propose
        // aucune. Un message qui recommanderait un chiffre serait une regle
        // normative inventee.
        let a = analyse("⟂ 0.05");
        for finding in &a.findings {
            assert!(
                !finding.message.contains("il conviendrait")
                    && !finding.message.contains("recommandé"),
                "{}",
                finding.message
            );
        }
    }
}
