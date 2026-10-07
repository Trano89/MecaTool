//! Etats de surface : lecture d'une indication, et procedes qui l'atteignent.
//!
//! # Ce que ce module fait
//!
//! Il lit une exigence d'etat de surface ecrite en clair — `Ra 0.8`,
//! `MRR Ra 1.6 ⊥`, `N7`, `Rz 6.3 max` — et repond a deux questions :
//!
//! 1. **que dit cette indication ?** le symbole, le parametre, la limite, le
//!    sens des stries, et la classe N equivalente quand il y en a une ;
//! 2. **quel procede l'obtient ?** chaque procede du tableau est situe par
//!    rapport a la valeur demandee : atteinte d'ordinaire, seulement avec des
//!    soins particuliers, hors d'atteinte — ou atteinte sans effort, ce qui
//!    signale un procede probablement plus fin (et plus cher) que necessaire.
//!
//! # Ce que ce module refuse
//!
//! * **Convertir Rz en Ra**, ou l'inverse. Aucune relation fixe ne les lie : le
//!   rapport depend du procede. Une exigence en Rz se lit, mais ne se confronte
//!   pas au tableau des procedes, qui est en Ra.
//! * **Deviner une regle d'acceptation par defaut**, une longueur de base ou un
//!   filtre. Ils different entre les deux generations de normes (ISO 4288 et
//!   ISO 21920-3), et ce module n'en porte qu'une partie commune.
//! * **Ecrire une classe N.** Il les lit, parce qu'elles abondent sur les plans
//!   existants, et rappelle a chaque fois qu'elles ont ete retirees.
//!
//! # La convention de saisie
//!
//! Faute de glyphe pour les trois variantes du symbole, on les ecrit comme
//! l'ISO 1302 le faisait en texte : `APA` (tout procede), `MRR` (enlevement de
//! matiere exige), `NMR` (enlevement interdit). Le `√` seul vaut `APA`. Les
//! valeurs sont en micrometres, la virgule decimale est acceptee.

use mecatool_core::{Conclusion, Length, Provenance, ReasoningStep, Unit, Verdict};
use mecatool_standards::surface::{
    GradeTable, IndicationTable, LaySymbol, ProcessRequirement, ProcessRoughness, ProcessTable,
    ProfileParameter, RoughnessGrade, SymbolVariant,
};
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::geometric::{Finding, Severity};

/// Le sens de la limite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    /// Limite superieure : la surface ne doit pas etre plus rugueuse.
    Upper,
    /// Limite inferieure : la surface ne doit pas etre plus lisse.
    Lower,
}

/// Une exigence d'etat de surface, telle qu'elle a ete lue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceIndication {
    /// La variante du symbole, quand elle est indiquee.
    pub requirement: Option<ProcessRequirement>,
    /// Le symbole du parametre, avec sa casse : `Ra`.
    pub parameter: String,
    pub value: Length,
    pub limit: LimitKind,
    /// Vrai quand l'indication porte `max` : regle du maximum.
    pub max_rule: bool,
    /// Le symbole du sens des stries, quand il est indique.
    pub lay: Option<String>,
    /// L'identifiant du procede cite, quand il y en a un.
    pub process: Option<String>,
    /// La classe N lue, quand l'exigence a ete saisie ainsi.
    pub grade: Option<String>,
    pub input: String,
}

/// Ou se situe un procede par rapport a la valeur demandee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// La valeur tombe dans sa plage usuelle.
    Usual,
    /// La valeur est au-dessus de sa plage usuelle : le procede fait mieux sans
    /// effort, et il est probablement plus fin — donc plus cher — que necessaire.
    Finer,
    /// La valeur demande des soins particuliers.
    Possible,
    /// Le procede ne l'atteint pas, meme avec des soins particuliers.
    OutOfReach,
}

impl Reach {
    pub const fn label_fr(self) -> &'static str {
        match self {
            Reach::Usual => "atteinte d'ordinaire",
            Reach::Finer => "atteinte sans effort — procédé plus fin que nécessaire",
            Reach::Possible => "atteinte avec des soins particuliers",
            Reach::OutOfReach => "hors d'atteinte",
        }
    }
}

/// Un procede, situe par rapport a une exigence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessFit {
    pub process: ProcessRoughness,
    pub reach: Reach,
    pub reach_label: String,
    /// Pourquoi le symbole ecarte ce procede, le cas echeant.
    pub excluded: Option<String>,
}

/// Une plage horizontale du graphique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSpan {
    pub x: f64,
    pub width: f64,
}

/// Une colonne du graphique : une classe N et sa valeur Ra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartColumn {
    pub grade: String,
    pub ra_label: String,
    pub x: f64,
    pub width: f64,
}

/// Une ligne du graphique : un procede.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartRow {
    pub id: String,
    pub name: String,
    pub removal: bool,
    pub y: f64,
    pub height: f64,
    pub usual: ChartSpan,
    pub possible: ChartSpan,
    /// Present quand le graphique accompagne une exigence.
    pub reach: Option<Reach>,
    pub excluded: bool,
}

/// La valeur demandee, reportee sur l'echelle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartMarker {
    pub x: f64,
    pub label: String,
}

/// Les plages de rugosite par procede, pretes a tracer.
///
/// Comme pour les zones de tolerance, la geometrie est calculee ici. Les
/// colonnes sont les classes N : l'echelle est donc logarithmique par
/// construction, chaque colonne doublant la precedente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoughnessChart {
    pub width: f64,
    pub height: f64,
    /// Abscisse ou commencent les colonnes, apres celle des noms.
    pub origin: f64,
    pub columns: Vec<ChartColumn>,
    pub rows: Vec<ChartRow>,
    pub marker: Option<ChartMarker>,
    pub caption: String,
}

/// Le resultat de la lecture d'une indication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceAnalysis {
    pub indication: SurfaceIndication,
    /// L'indication reconstituee, normalisee : `MRR Ra 0.8 ⊥`.
    pub designation: String,
    pub requirement: Option<SymbolVariant>,
    pub parameter: ProfileParameter,
    pub lay: Option<LaySymbol>,
    /// La classe N dont la valeur Ra est exactement celle demandee.
    pub grade: Option<RoughnessGrade>,
    /// Chaque procede du tableau, situe par rapport a l'exigence. Vide quand la
    /// confrontation n'a pas de sens, et `process_note` dit pourquoi.
    pub processes: Vec<ProcessFit>,
    pub process_note: Option<String>,
    /// Le procede cite dans l'indication, situe lui aussi.
    pub stated_process: Option<ProcessFit>,
    pub findings: Vec<Finding>,
    pub chart: RoughnessChart,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Largeur reservee aux noms de procedes.
const NAME_GUTTER: f64 = 190.0;
/// Hauteur d'une ligne et de son interligne.
const ROW_HEIGHT: f64 = 22.0;
/// Largeur du graphique, en unites de dessin.
const CHART_WIDTH: f64 = 720.0;

/// Le moteur du domaine etats de surface.
#[derive(Debug)]
pub struct SurfaceEngine {
    indication: &'static IndicationTable,
    grades: &'static GradeTable,
    processes: &'static ProcessTable,
}

impl SurfaceEngine {
    pub fn new() -> Result<Self> {
        Ok(SurfaceEngine {
            indication: IndicationTable::embedded()?,
            grades: GradeTable::embedded()?,
            processes: ProcessTable::embedded()?,
        })
    }

    pub fn indication_table(&self) -> &'static IndicationTable {
        self.indication
    }

    pub fn grades(&self) -> &'static [RoughnessGrade] {
        self.grades.grades()
    }

    pub fn processes(&self) -> &'static [ProcessRoughness] {
        self.processes.processes()
    }

    /// Les trois sources, dans l'ordre ou l'ecran les presente.
    pub fn provenance(&self) -> Provenance {
        Provenance::new()
            .with(self.indication.standard().clone())
            .with(self.grades.standard().clone())
            .with(self.processes.standard().clone())
    }

    /// Le graphique de tous les procedes, sans exigence.
    pub fn catalogue_chart(&self) -> RoughnessChart {
        self.chart(None, &[])
    }

    /// Lit puis analyse une indication.
    pub fn read(&self, input: &str) -> Result<SurfaceAnalysis> {
        let indication = self.parse(input)?;
        self.analyse(&indication)
    }

    /// Lit une indication ecrite en clair.
    pub fn parse(&self, input: &str) -> Result<SurfaceIndication> {
        let unparsable = |hint: String| EngineError::Unparsable {
            input: input.to_string(),
            hint,
        };

        // La virgule decimale est d'usage en France : elle n'a ici aucun autre
        // role, on la lit donc comme un point. Le « √ » est detache, pour etre
        // lu comme un jeton quel que soit ce qui le suit.
        let normalised = input.replace(',', ".").replace('√', " √ ");
        let tokens = split_tokens(&normalised);
        if tokens.is_empty() {
            return Err(unparsable(
                "Indiquez un paramètre et sa valeur en micromètres, par exemple « Ra 0.8 », \
                 ou une classe, par exemple « N7 »."
                    .into(),
            ));
        }

        let mut requirement: Option<ProcessRequirement> = None;
        let mut limit: Option<LimitKind> = None;
        let mut parameter: Option<&ProfileParameter> = None;
        let mut value: Option<Length> = None;
        let mut grade: Option<&RoughnessGrade> = None;
        let mut max_rule = false;
        let mut lay: Option<&LaySymbol> = None;
        let mut process: Option<&ProcessRoughness> = None;

        let mut index = 0;
        while index < tokens.len() {
            let token = tokens[index].as_str();
            let lower = token.to_lowercase();

            if token == "√" || self.indication.symbol_by_code(token).is_some() {
                let id = if token == "√" {
                    ProcessRequirement::Any
                } else {
                    self.indication.symbol_by_code(token).map(|s| s.id).unwrap()
                };
                // « √ MRR » est une redite, pas une contradiction : le symbole
                // de base est le tronc commun des trois variantes.
                requirement = match (requirement, id) {
                    (None | Some(ProcessRequirement::Any), id) => Some(id),
                    (Some(current), ProcessRequirement::Any) => Some(current),
                    (Some(current), id) if current == id => Some(id),
                    _ => {
                        return Err(unparsable(
                            "L'indication porte deux variantes de symbole contradictoires : \
                             MRR exige l'enlèvement de matière, NMR l'interdit."
                                .into(),
                        ))
                    }
                };
            } else if lower == "u" || lower == "l" {
                limit = Some(if lower == "u" {
                    LimitKind::Upper
                } else {
                    LimitKind::Lower
                });
            } else if let Some(found) = self.indication.parameter(token) {
                if parameter.is_some() || grade.is_some() {
                    return Err(unparsable(one_requirement_hint()));
                }
                parameter = Some(found);
            } else if let Some(found) = self.grade_token(token) {
                if parameter.is_some() || grade.is_some() {
                    return Err(unparsable(one_requirement_hint()));
                }
                grade = Some(found);
            } else if looks_numeric(token) {
                if parameter.is_none() {
                    return Err(unparsable(format!(
                        "« {token} » est une valeur, mais aucun paramètre ne la précède. \
                         Écrivez par exemple « Ra {token} »."
                    )));
                }
                if value.is_some() {
                    return Err(unparsable(one_requirement_hint()));
                }
                let parsed = Length::parse(token, Unit::Micrometre).map_err(|source| {
                    unparsable(format!("Valeur « {token} » inexploitable : {source}."))
                })?;
                if !parsed.is_positive() {
                    return Err(unparsable(
                        "Une exigence de rugosité est une valeur strictement positive.".into(),
                    ));
                }
                value = Some(parsed);
            } else if lower == "µm" || lower == "um" {
                // L'unite est celle de toute valeur de rugosite : rien a lire.
            } else if lower == "max" {
                max_rule = true;
            } else if let Some(found) = self.indication.lay(token) {
                lay = Some(found);
            } else if let Some((found, used)) = self.process_at(&tokens, index) {
                process = Some(found);
                index += used;
                continue;
            } else if is_uncovered_parameter(token) {
                return Err(unparsable(format!(
                    "« {token} » n'est pas couvert. MecaTool ne connaît que les paramètres \
                     d'amplitude du profil de rugosité : {}. Les paramètres d'espacement et \
                     les profils W et P ne sont pas embarqués.",
                    self.parameter_list()
                )));
            } else {
                return Err(unparsable(format!(
                    "« {token} » n'est ni un paramètre ({}), ni une classe N, ni un sens des \
                     stries ({}), ni un procédé du tableau.",
                    self.parameter_list(),
                    self.indication
                        .lays()
                        .iter()
                        .map(|l| l.symbol.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                )));
            }
            index += 1;
        }

        let (parameter, value) = match (parameter, value, grade) {
            (Some(parameter), Some(value), None) => (parameter, value),
            (None, None, Some(grade)) => (
                self.indication
                    .parameter("Ra")
                    .expect("Ra est controle au chargement"),
                grade.ra,
            ),
            (Some(parameter), None, None) => {
                return Err(unparsable(format!(
                    "{} attend une valeur en micromètres, par exemple « {} 0.8 ».",
                    parameter.symbol, parameter.symbol
                )))
            }
            _ => {
                return Err(unparsable(
                    "Indiquez un paramètre et sa valeur en micromètres, par exemple \
                     « Ra 0.8 », ou une classe, par exemple « N7 »."
                        .into(),
                ))
            }
        };

        Ok(SurfaceIndication {
            requirement,
            parameter: parameter.symbol.clone(),
            value,
            limit: limit.unwrap_or(LimitKind::Upper),
            max_rule,
            lay: lay.map(|l| l.symbol.clone()),
            process: process.map(|p| p.id.clone()),
            grade: grade.map(|g| g.grade.clone()),
            input: input.trim().to_string(),
        })
    }

    /// Analyse une indication deja lue.
    pub fn analyse(&self, indication: &SurfaceIndication) -> Result<SurfaceAnalysis> {
        let parameter = self
            .indication
            .parameter(&indication.parameter)
            .ok_or_else(|| EngineError::Unparsable {
                input: indication.parameter.clone(),
                hint: format!("Paramètres connus : {}.", self.parameter_list()),
            })?
            .clone();
        let requirement = indication
            .requirement
            .and_then(|id| self.indication.symbol(id))
            .cloned();
        let lay = indication
            .lay
            .as_deref()
            .and_then(|symbol| self.indication.lay(symbol))
            .cloned();
        let is_ra = parameter.symbol == "Ra";
        let grade = if is_ra {
            self.grades.by_ra(indication.value).cloned()
        } else {
            None
        };

        // La confrontation au tableau n'a de sens que pour une limite haute en
        // Ra. Dans les autres cas, on le dit plutot que de rendre une liste vide
        // qu'on lirait comme « aucun procede ne convient ».
        let process_note = if !is_ra {
            Some(format!(
                "Le tableau des procédés est exprimé en Ra. MecaTool ne convertit pas {} en \
                 Ra : aucune relation fixe ne lie les deux paramètres, le rapport dépend du \
                 procédé.",
                parameter.symbol
            ))
        } else if indication.limit == LimitKind::Lower {
            Some(
                "Une limite inférieure fixe une rugosité minimale. Le tableau dit ce qu'un \
                 procédé atteint au plus fin, pas ce qu'il garantit au plus grossier : \
                 MecaTool ne le retourne pas pour répondre à cette question."
                    .to_string(),
            )
        } else {
            None
        };

        let processes: Vec<ProcessFit> = if process_note.is_none() {
            self.processes
                .processes()
                .iter()
                .map(|p| self.fit(p, indication.value, indication.requirement))
                .collect()
        } else {
            Vec::new()
        };

        let stated = indication
            .process
            .as_deref()
            .and_then(|id| self.processes.by_id(id));
        let stated_process = match (stated, process_note.is_none()) {
            (Some(p), true) => Some(self.fit(p, indication.value, indication.requirement)),
            _ => None,
        };

        let designation = self.designation(indication, stated);
        let findings = self.findings(indication, &parameter, stated, stated_process.as_ref());

        let provenance = self.provenance();
        let mut conclusion = self.conclusion(
            &designation,
            &findings,
            &processes,
            process_note.as_deref(),
            stated_process.as_ref(),
        );
        conclusion.why = self.reasoning(indication, &requirement, &parameter, &grade, &processes);
        conclusion.warnings = provenance.warnings_fr();

        let marker = (is_ra && indication.limit == LimitKind::Upper).then_some(indication.value);
        let chart = self.chart(marker, &processes);

        Ok(SurfaceAnalysis {
            indication: indication.clone(),
            designation,
            requirement,
            parameter,
            lay,
            grade,
            processes,
            process_note,
            stated_process,
            findings,
            chart,
            conclusion,
            provenance,
        })
    }

    /// Situe un procede par rapport a une limite haute en Ra.
    fn fit(
        &self,
        process: &ProcessRoughness,
        value: Length,
        requirement: Option<ProcessRequirement>,
    ) -> ProcessFit {
        let reach = if value > process.usual.coarsest {
            Reach::Finer
        } else if value >= process.usual.finest {
            Reach::Usual
        } else if value >= process.possible.finest {
            Reach::Possible
        } else {
            Reach::OutOfReach
        };
        let excluded = match requirement {
            Some(ProcessRequirement::RemovalRequired) if !process.removal => Some(
                "Le symbole exige un enlèvement de matière : un procédé de mise en forme \
                 n'y répond pas."
                    .to_string(),
            ),
            Some(ProcessRequirement::RemovalProhibited) if process.removal => Some(
                "Le symbole interdit l'enlèvement de matière : un usinage n'y répond pas."
                    .to_string(),
            ),
            _ => None,
        };
        ProcessFit {
            process: process.clone(),
            reach,
            reach_label: reach.label_fr().to_string(),
            excluded,
        }
    }

    fn findings(
        &self,
        indication: &SurfaceIndication,
        parameter: &ProfileParameter,
        stated: Option<&ProcessRoughness>,
        stated_fit: Option<&ProcessFit>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        if let Some(grade) = &indication.grade {
            findings.push(Finding::new(
                "n_grade_withdrawn",
                Severity::Caution,
                format!(
                    "{grade} est une classe de l'ISO 1302:1992, retirée depuis : sur un plan \
                     neuf, inscrivez la valeur elle-même, « Ra {} ».",
                    um_number(indication.value)
                ),
            ));
        }

        if let Some(process) = stated {
            match indication.requirement {
                Some(ProcessRequirement::RemovalRequired) if !process.removal => {
                    findings.push(Finding::new(
                        "requirement_contradicts_process",
                        Severity::Error,
                        format!(
                            "Le symbole exige un enlèvement de matière (MRR), mais le procédé \
                             indiqué — {} — est une mise en forme.",
                            process.name
                        ),
                    ))
                }
                Some(ProcessRequirement::RemovalProhibited) if process.removal => {
                    findings.push(Finding::new(
                        "requirement_contradicts_process",
                        Severity::Error,
                        format!(
                            "Le symbole interdit l'enlèvement de matière (NMR), mais le procédé \
                             indiqué — {} — est un usinage.",
                            process.name
                        ),
                    ))
                }
                _ => {}
            }
        }

        if let (Some(process), Some(fit)) = (stated, stated_fit) {
            let value = um_number(indication.value);
            match fit.reach {
                Reach::OutOfReach => findings.push(Finding::new(
                    "process_out_of_reach",
                    Severity::Caution,
                    format!(
                        "Selon le tableau, le {} n'atteint pas Ra {value}, même avec des soins \
                         particuliers : au plus fin, Ra {}.",
                        process.name,
                        um_number(process.possible.finest)
                    ),
                )),
                Reach::Possible => findings.push(Finding::new(
                    "process_needs_care",
                    Severity::Caution,
                    format!(
                        "Le {} n'atteint Ra {value} qu'avec des soins particuliers : sa plage \
                         usuelle s'arrête à Ra {}.",
                        process.name,
                        um_number(process.usual.finest)
                    ),
                )),
                Reach::Finer => findings.push(Finding::new(
                    "process_finer_than_needed",
                    Severity::Note,
                    format!(
                        "Le {} fait mieux que Ra {value} sans effort : un procédé moins fin, \
                         et moins cher, suffirait peut-être.",
                        process.name
                    ),
                )),
                Reach::Usual => {}
            }
        }

        if indication.max_rule {
            findings.push(Finding::new(
                "max_rule",
                Severity::Note,
                "« max » demande la règle du maximum : aucune des valeurs mesurées ne doit \
                 franchir la limite.",
            ));
        }

        if parameter.symbol == "Ra"
            && indication.grade.is_none()
            && self.grades.by_ra(indication.value).is_none()
        {
            findings.push(Finding::new(
                "off_series",
                Severity::Note,
                format!(
                    "Ra {} ne correspond à aucune classe N : la valeur reste licite, mais elle \
                     sort de la série usuelle (… 0.8 · 1.6 · 3.2 …).",
                    um_number(indication.value)
                ),
            ));
        }

        if let Some(note) = &parameter.note {
            findings.push(Finding::new("parameter_note", Severity::Note, note.clone()));
        }

        findings
    }

    fn conclusion(
        &self,
        designation: &str,
        findings: &[Finding],
        processes: &[ProcessFit],
        process_note: Option<&str>,
        stated: Option<&ProcessFit>,
    ) -> Conclusion {
        if let Some(error) = findings.iter().find(|f| f.severity == Severity::Error) {
            return Conclusion::new(Verdict::Incompatible, error.message.clone());
        }
        if let Some(note) = process_note {
            return Conclusion::new(
                Verdict::InsufficientData,
                format!("{designation} se lit, mais ne se confronte pas aux procédés. {note}"),
            );
        }

        let names = |reach: &[Reach]| -> Vec<String> {
            processes
                .iter()
                .filter(|p| p.excluded.is_none() && reach.contains(&p.reach))
                .map(|p| p.process.name.clone())
                .collect()
        };
        let usual = names(&[Reach::Usual]);
        let possible = names(&[Reach::Possible]);

        let detail = if let Some(fit) = stated {
            format!(
                "{designation} — le procédé indiqué ({}) : {}.",
                fit.process.name, fit.reach_label
            )
        } else if !usual.is_empty() {
            format!(
                "{designation} — atteinte d'ordinaire par : {}.",
                usual.join(", ")
            )
        } else if !possible.is_empty() {
            format!(
                "{designation} — aucun procédé du tableau ne l'atteint d'ordinaire ; avec des \
                 soins particuliers : {}.",
                possible.join(", ")
            )
        } else {
            return Conclusion::new(
                Verdict::InsufficientData,
                format!(
                    "{designation} — aucun procédé du tableau ne l'atteint, même avec des soins \
                     particuliers. Le tableau n'est pas exhaustif : ce n'est pas une \
                     impossibilité, c'est une absence de donnée."
                ),
            );
        };
        // Toujours prudent : le tableau est un ordre de grandeur, pas une
        // garantie, et il n'est pas encore confronte a sa source.
        Conclusion::new(Verdict::Caution, detail)
    }

    fn reasoning(
        &self,
        indication: &SurfaceIndication,
        requirement: &Option<SymbolVariant>,
        parameter: &ProfileParameter,
        grade: &Option<RoughnessGrade>,
        processes: &[ProcessFit],
    ) -> Vec<ReasoningStep> {
        let mut steps = Vec::new();
        steps.push(match requirement {
            Some(symbol) => ReasoningStep::new("Symbole")
                .with_expression(format!("{} — {}", symbol.code, symbol.name))
                .with_value(symbol.meaning.clone()),
            None => ReasoningStep::new("Symbole")
                .with_expression("non précisé")
                .with_value("Sans APA, MRR ni NMR, tout procédé est envisagé."),
        });
        steps.push(
            ReasoningStep::new("Paramètre")
                .with_expression(format!("{} — {}", parameter.symbol, parameter.name))
                .with_value(parameter.definition.clone()),
        );
        steps.push(
            ReasoningStep::new("Limite")
                .with_expression(match indication.limit {
                    LimitKind::Upper => "supérieure : la surface ne doit pas être plus rugueuse",
                    LimitKind::Lower => "inférieure : la surface ne doit pas être plus lisse",
                })
                .with_value(format!(
                    "{} {} µm",
                    parameter.symbol,
                    um_number(indication.value)
                )),
        );
        if let Some(grade) = grade {
            steps.push(
                ReasoningStep::new("Classe N équivalente")
                    .with_expression("ISO 1302:1992, retirée")
                    .with_value(format!("{} = Ra {} µm", grade.grade, um_number(grade.ra))),
            );
        }
        steps.push(
            ReasoningStep::new("Règle d'acceptation")
                .with_expression(if indication.max_rule {
                    "règle du maximum (« max »)"
                } else {
                    "non indiquée"
                })
                .with_value(if indication.max_rule {
                    "Aucune valeur mesurée ne doit franchir la limite."
                } else {
                    "La règle par défaut relève de la norme de spécification appliquée. \
                     MecaTool ne l'embarque pas et ne la tranche pas."
                }),
        );
        if !processes.is_empty() {
            let count = |reach: Reach| {
                processes
                    .iter()
                    .filter(|p| p.excluded.is_none() && p.reach == reach)
                    .count()
            };
            let excluded = processes.iter().filter(|p| p.excluded.is_some()).count();
            steps.push(
                ReasoningStep::new("Procédés du tableau")
                    .with_expression(format!("{} procédés examinés", processes.len()))
                    .with_value(format!(
                        "{} d'ordinaire, {} avec des soins, {} plus fins que nécessaire, \
                         {} hors d'atteinte, {} écartés par le symbole",
                        count(Reach::Usual),
                        count(Reach::Possible),
                        count(Reach::Finer),
                        count(Reach::OutOfReach),
                        excluded
                    )),
            );
        }
        steps.push(
            ReasoningStep::new("Ce que MecaTool ne fournit pas")
                .with_expression("longueur de base, filtre, règle par défaut")
                .with_value(
                    "Ils diffèrent entre l'ISO 4288 et l'ISO 21920-3 : le module ne porte que \
                     ce que les deux générations ont en commun.",
                ),
        );
        steps
    }

    fn designation(
        &self,
        indication: &SurfaceIndication,
        process: Option<&ProcessRoughness>,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(symbol) = indication
            .requirement
            .and_then(|id| self.indication.symbol(id))
        {
            parts.push(symbol.code.clone());
        }
        if indication.limit == LimitKind::Lower {
            parts.push("L".into());
        }
        parts.push(indication.parameter.clone());
        if indication.max_rule {
            parts.push("max".into());
        }
        parts.push(um_number(indication.value));
        if let Some(lay) = &indication.lay {
            parts.push(lay.clone());
        }
        if let Some(process) = process {
            parts.push(format!("({})", process.name));
        }
        parts.join(" ")
    }

    /// Construit le graphique des plages, avec ou sans exigence.
    fn chart(&self, marker: Option<Length>, fits: &[ProcessFit]) -> RoughnessChart {
        let grades = self.grades.grades();
        let usable = CHART_WIDTH - NAME_GUTTER;
        let column = usable / grades.len() as f64;
        let span = |finest: Length, coarsest: Length| -> ChartSpan {
            // Les bornes tombent sur la serie : c'est controle au chargement.
            let from = self.grades.index_of(finest).unwrap_or(0) as f64;
            let to = self.grades.index_of(coarsest).unwrap_or(grades.len() - 1) as f64;
            ChartSpan {
                x: NAME_GUTTER + from * column,
                width: (to - from + 1.0) * column,
            }
        };

        let rows = self
            .processes
            .processes()
            .iter()
            .enumerate()
            .map(|(index, process)| {
                let fit = fits.iter().find(|f| f.process.id == process.id);
                ChartRow {
                    id: process.id.clone(),
                    name: process.name.clone(),
                    removal: process.removal,
                    y: index as f64 * ROW_HEIGHT + 4.0,
                    height: ROW_HEIGHT - 8.0,
                    usual: span(process.usual.finest, process.usual.coarsest),
                    possible: span(process.possible.finest, process.possible.coarsest),
                    reach: fit.map(|f| f.reach),
                    excluded: fit.is_some_and(|f| f.excluded.is_some()),
                }
            })
            .collect::<Vec<_>>();

        let columns = grades
            .iter()
            .enumerate()
            .map(|(index, grade)| ChartColumn {
                grade: grade.grade.clone(),
                ra_label: um_number(grade.ra),
                x: NAME_GUTTER + index as f64 * column,
                width: column,
            })
            .collect();

        let marker = marker.and_then(|value| {
            self.marker_position(value).map(|position| ChartMarker {
                x: NAME_GUTTER + position * column,
                label: format!("Ra {}", um_number(value)),
            })
        });

        RoughnessChart {
            width: CHART_WIDTH,
            height: rows.len() as f64 * ROW_HEIGHT + 8.0,
            origin: NAME_GUTTER,
            columns,
            rows,
            marker,
            caption: "Plage usuelle en trait plein, plage atteignable avec des soins \
                      particuliers en trait clair. Chaque colonne est une classe N : l'échelle \
                      double d'une colonne à la suivante. Ordres de grandeur, non normatifs."
                .to_string(),
        }
    }

    /// La position d'une valeur sur l'echelle, en colonnes depuis le bord gauche.
    ///
    /// Une valeur de la serie tombe au milieu de sa colonne. Une valeur hors
    /// serie s'interpole en logarithme entre ses deux voisines — c'est un
    /// dessin, pas une valeur : le flottant y est a sa place. Hors de l'echelle,
    /// pas de repere plutot qu'un repere colle au bord, qui mentirait.
    fn marker_position(&self, value: Length) -> Option<f64> {
        let grades = self.grades.grades();
        if let Some(index) = self.grades.index_of(value) {
            return Some(index as f64 + 0.5);
        }
        let upper = grades.iter().position(|g| g.ra > value)?;
        if upper == 0 {
            return None;
        }
        let low = grades[upper - 1].ra.nanometres() as f64;
        let high = grades[upper].ra.nanometres() as f64;
        let fraction = (value.nanometres() as f64 / low).ln() / (high / low).ln();
        Some(upper as f64 - 0.5 + fraction)
    }

    /// Une classe N ecrite seule : `N7`.
    fn grade_token(&self, token: &str) -> Option<&'static RoughnessGrade> {
        let mut chars = token.chars();
        let first = chars.next()?;
        if !(first == 'N' || first == 'n') || !chars.clone().all(|c| c.is_ascii_digit()) {
            return None;
        }
        // Au moins un chiffre apres le N : « N » seul n'est pas une classe.
        chars.next()?;
        self.grades.by_name(token)
    }

    /// Un procede ecrit en un a trois mots, a partir de la position donnee.
    fn process_at(
        &self,
        tokens: &[String],
        start: usize,
    ) -> Option<(&'static ProcessRoughness, usize)> {
        for length in (1..=3).rev() {
            if start + length > tokens.len() {
                continue;
            }
            let phrase = tokens[start..start + length].join(" ");
            if let Some(process) = self.processes.lookup(&phrase) {
                return Some((process, length));
            }
        }
        None
    }

    fn parameter_list(&self) -> String {
        self.indication
            .parameters()
            .iter()
            .map(|p| p.symbol.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn one_requirement_hint() -> String {
    "Une seule exigence à la fois : un paramètre et sa valeur, ou une classe N.".to_string()
}

/// Separe les jetons, et detache la valeur collee a son parametre (`Ra0.8`).
fn split_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in text.split_whitespace() {
        let split = word
            .char_indices()
            .find(|(_, c)| c.is_ascii_digit() || *c == '.')
            .map(|(i, _)| i);
        match split {
            // « Ra0.8 » : deux lettres puis un nombre. « N7 » reste entier : une
            // classe N n'est pas un parametre suivi d'une valeur.
            Some(i)
                if i >= 2
                    && word[..i].chars().all(|c| c.is_ascii_alphabetic())
                    && looks_numeric(&word[i..]) =>
            {
                tokens.push(word[..i].to_string());
                tokens.push(word[i..].to_string());
            }
            _ => tokens.push(word.to_string()),
        }
    }
    tokens
}

fn looks_numeric(token: &str) -> bool {
    !token.is_empty()
        && token.chars().any(|c| c.is_ascii_digit())
        && token
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
}

/// Un parametre de rugosite que MecaTool ne couvre pas : `Rsm`, `Rmr`, `Wa`...
fn is_uncovered_parameter(token: &str) -> bool {
    let mut chars = token.chars();
    matches!(chars.next(), Some('R' | 'W' | 'P'))
        && (2..=4).contains(&token.chars().count())
        && token.chars().skip(1).all(|c| c.is_ascii_alphabetic())
        && token.chars().nth(1).is_some_and(|c| c.is_ascii_lowercase())
}

/// Une valeur en micrometres, sans unite ni zero superflu : `0.8`, `12.5`.
fn um_number(value: Length) -> String {
    let rendered = value.to_decimal_string(Unit::Micrometre, 3);
    if rendered.contains('.') {
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> SurfaceEngine {
        SurfaceEngine::new().expect("les donnees d'etat de surface doivent charger")
    }

    fn um(text: &str) -> Length {
        Length::parse(text, Unit::Micrometre).unwrap()
    }

    fn reach_of(analysis: &SurfaceAnalysis, id: &str) -> Reach {
        analysis
            .processes
            .iter()
            .find(|p| p.process.id == id)
            .unwrap_or_else(|| panic!("{id} absent"))
            .reach
    }

    #[test]
    fn lit_une_exigence_simple() {
        let indication = engine().parse("Ra 0.8").unwrap();
        assert_eq!(indication.parameter, "Ra");
        assert_eq!(indication.value, um("0.8"));
        assert_eq!(indication.limit, LimitKind::Upper);
        assert!(indication.requirement.is_none());
    }

    #[test]
    fn accepte_la_virgule_et_la_valeur_collee() {
        let a = engine().parse("Ra0,8").unwrap();
        let b = engine().parse("ra 0.8 µm").unwrap();
        assert_eq!(a.value, um("0.8"));
        assert_eq!(b.parameter, "Ra");
        assert_eq!(a.value, b.value);
    }

    #[test]
    fn lit_le_symbole_les_stries_et_le_procede() {
        let indication = engine().parse("MRR Ra 1.6 ⊥ rectifié").unwrap();
        assert_eq!(
            indication.requirement,
            Some(ProcessRequirement::RemovalRequired)
        );
        assert_eq!(indication.lay.as_deref(), Some("⊥"));
        assert_eq!(indication.process.as_deref(), Some("grinding"));
    }

    #[test]
    fn lit_un_procede_en_plusieurs_mots() {
        let indication = engine().parse("NMR Ra 25 moulage en sable").unwrap();
        assert_eq!(indication.process.as_deref(), Some("sand_casting"));
    }

    #[test]
    fn une_classe_n_se_lit_en_ra_et_rappelle_quelle_est_retiree() {
        let analysis = engine().read("N7").unwrap();
        assert_eq!(analysis.indication.value, um("1.6"));
        assert_eq!(analysis.parameter.symbol, "Ra");
        assert_eq!(analysis.grade.as_ref().unwrap().grade, "N7");
        assert!(analysis
            .findings
            .iter()
            .any(|f| f.code == "n_grade_withdrawn" && f.message.contains("Ra 1.6")));
    }

    #[test]
    fn situe_chaque_procede_par_rapport_a_la_valeur() {
        let analysis = engine().read("Ra 0.8").unwrap();
        // Rectification : 0,1 a 1,6 d'ordinaire.
        assert_eq!(reach_of(&analysis, "grinding"), Reach::Usual);
        // Fraisage : 0,8 a 6,3 d'ordinaire — la borne est incluse.
        assert_eq!(reach_of(&analysis, "milling"), Reach::Usual);
        // Percage : 1,6 d'ordinaire au mieux, 0,8 avec des soins.
        assert_eq!(reach_of(&analysis, "drilling"), Reach::Possible);
        // Moulage en sable : hors d'atteinte.
        assert_eq!(reach_of(&analysis, "sand_casting"), Reach::OutOfReach);
        // Superfinition : bien plus fine que necessaire.
        assert_eq!(reach_of(&analysis, "superfinishing"), Reach::Finer);
    }

    #[test]
    fn le_symbole_ecarte_les_procedes_de_lautre_famille() {
        let analysis = engine().read("MRR Ra 3.2").unwrap();
        let forging = analysis
            .processes
            .iter()
            .find(|p| p.process.id == "forging")
            .unwrap();
        assert!(forging.excluded.is_some());
        let milling = analysis
            .processes
            .iter()
            .find(|p| p.process.id == "milling")
            .unwrap();
        assert!(milling.excluded.is_none());
        // La conclusion ne cite pas un procede ecarte.
        assert!(!analysis.conclusion.detail.contains("forgeage"));
    }

    #[test]
    fn une_contradiction_entre_symbole_et_procede_est_une_faute() {
        let analysis = engine().read("MRR Ra 3.2 forgé").unwrap();
        assert_eq!(analysis.conclusion.verdict, Verdict::Incompatible);
        assert!(analysis
            .findings
            .iter()
            .any(|f| f.code == "requirement_contradicts_process"));
    }

    #[test]
    fn rz_se_lit_mais_ne_se_convertit_pas() {
        // Le refus central du module : aucune relation fixe ne lie Rz et Ra.
        let analysis = engine().read("Rz 6.3").unwrap();
        assert_eq!(analysis.parameter.symbol, "Rz");
        assert!(analysis.processes.is_empty());
        assert!(analysis
            .process_note
            .as_deref()
            .unwrap()
            .contains("ne convertit pas"));
        assert_eq!(analysis.conclusion.verdict, Verdict::InsufficientData);
        assert!(analysis.chart.marker.is_none());
        // Et la mise en garde historique sur Rz suit.
        assert!(analysis.findings.iter().any(|f| f.code == "parameter_note"));
    }

    #[test]
    fn une_limite_inferieure_ne_se_confronte_pas_au_tableau() {
        let analysis = engine().read("L Ra 0.4").unwrap();
        assert_eq!(analysis.indication.limit, LimitKind::Lower);
        assert!(analysis.processes.is_empty());
        assert!(analysis.designation.starts_with("L Ra"));
    }

    #[test]
    fn le_procede_cite_trop_grossier_est_signale() {
        let analysis = engine().read("Ra 0.2 fraisé").unwrap();
        assert!(analysis
            .findings
            .iter()
            .any(|f| f.code == "process_needs_care"));
        let analysis = engine().read("Ra 0.1 fraisé").unwrap();
        assert!(analysis
            .findings
            .iter()
            .any(|f| f.code == "process_out_of_reach"));
    }

    #[test]
    fn la_regle_du_maximum_est_expliquee_et_le_defaut_nest_pas_devine() {
        let with_max = engine().read("Ra max 0.8").unwrap();
        assert!(with_max.indication.max_rule);
        assert!(with_max.findings.iter().any(|f| f.code == "max_rule"));

        let without = engine().read("Ra 0.8").unwrap();
        let rule = without
            .conclusion
            .why
            .iter()
            .find(|s| s.label == "Règle d'acceptation")
            .unwrap();
        assert!(rule.value.as_deref().unwrap().contains("ne la tranche pas"));
    }

    #[test]
    fn une_valeur_hors_serie_est_licite_mais_signalee() {
        let analysis = engine().read("Ra 1").unwrap();
        assert!(analysis.grade.is_none());
        assert!(analysis.findings.iter().any(|f| f.code == "off_series"));
        // Le repere s'interpole entre N6 (0,8) et N7 (1,6).
        let marker = analysis.chart.marker.unwrap();
        let n6 = &analysis.chart.columns[5];
        let n7 = &analysis.chart.columns[6];
        assert!(marker.x > n6.x + n6.width / 2.0 && marker.x < n7.x + n7.width / 2.0);
    }

    #[test]
    fn rien_dans_le_tableau_nest_pas_une_impossibilite() {
        let analysis = engine().read("Ra 0.025 fraisé").unwrap();
        assert_eq!(reach_of(&analysis, "milling"), Reach::OutOfReach);
        let analysis = engine().read("Ra 0.012").unwrap();
        assert_eq!(analysis.conclusion.verdict, Verdict::InsufficientData);
        assert!(analysis.conclusion.detail.contains("absence de donnée"));
        // Hors de l'echelle : pas de repere colle au bord.
        assert!(analysis.chart.marker.is_none());
    }

    #[test]
    fn le_verdict_reste_prudent_et_porte_la_reserve() {
        let analysis = engine().read("Ra 1.6").unwrap();
        assert_eq!(analysis.conclusion.verdict, Verdict::Caution);
        assert!(!analysis.provenance.is_fully_verified());
        assert_eq!(analysis.conclusion.warnings.len(), 3);
        assert!(analysis.conclusion.warnings[0].contains("non vérifiée"));
    }

    #[test]
    fn la_designation_se_reconstitue() {
        let analysis = engine().read("mrr ra 0,8 perp").unwrap();
        assert_eq!(analysis.designation, "MRR Ra 0.8 ⊥");
        let analysis = engine().read("√ Ra 3.2 tourné").unwrap();
        assert_eq!(
            analysis.designation,
            "APA Ra 3.2 (tournage, alésage à l'outil)"
        );
    }

    #[test]
    fn les_saisies_fautives_disent_quoi_faire() {
        let e = engine();
        for (input, expected) in [
            ("", "Ra 0.8"),
            ("Ra", "attend une valeur"),
            ("0.8", "aucun paramètre"),
            ("Ra 0.8 Rz 3.2", "Une seule exigence"),
            ("Ra 0", "strictement positive"),
            ("Rsm 0.1", "pas couvert"),
            ("Ra 0.8 bidule", "ni un paramètre"),
            ("MRR NMR Ra 0.8", "contradictoires"),
        ] {
            let err = e.parse(input).unwrap_err().to_string();
            assert!(err.contains(expected), "{input:?} : {err}");
        }
    }

    #[test]
    fn le_graphique_place_chaque_plage_sur_ses_colonnes() {
        let chart = engine().catalogue_chart();
        assert_eq!(chart.columns.len(), 12);
        assert!(chart.marker.is_none());
        let grinding = chart.rows.iter().find(|r| r.id == "grinding").unwrap();
        // Usuelle 0,1 (N3) a 1,6 (N7) : cinq colonnes a partir de la troisieme.
        let column = chart.columns[0].width;
        assert!((grinding.usual.x - chart.columns[2].x).abs() < 1e-9);
        assert!((grinding.usual.width - 5.0 * column).abs() < 1e-9);
        // La plage atteignable englobe la plage usuelle.
        assert!(grinding.possible.x <= grinding.usual.x);
        assert!(
            grinding.possible.x + grinding.possible.width
                >= grinding.usual.x + grinding.usual.width
        );
    }

    #[test]
    fn le_repere_tombe_au_milieu_de_sa_colonne() {
        let analysis = engine().read("Ra 0.8").unwrap();
        let marker = analysis.chart.marker.unwrap();
        let n6 = &analysis.chart.columns[5];
        assert!((marker.x - (n6.x + n6.width / 2.0)).abs() < 1e-9);
        assert_eq!(marker.label, "Ra 0.8");
    }
}
