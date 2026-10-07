//! Soudure : lecture d'un symbole, d'un procede, et de ce que tolere un niveau
//! de qualite.
//!
//! # Ce que ce module fait
//!
//! * Il lit un **numero de procede** (`135`) ou un nom d'atelier (`MAG`) et
//!   rend sa place dans la nomenclature. Un nom d'atelier designe souvent
//!   plusieurs numeros : le moteur les rend tous, il n'en choisit pas un.
//! * Il lit un **symbole de soudure complet** — symbole elementaire, cote, cote
//!   de discontinuite, symboles supplementaires, procede, niveau de qualite — et
//!   le restitue en clair, phrase par phrase, avec ce qui cloche.
//! * Il **chiffre les limites** d'un niveau de qualite pour une geometrie
//!   donnee : `h ≤ 1 mm + 0,15 b, max. 7 mm` devient `h ≤ 2,5 mm` pour
//!   `b = 10 mm`.
//!
//! # Ce que ce module ne fait pas
//!
//! Il ne **recommande pas de niveau de qualite**. L'ISO 5817 dit ce qu'un niveau
//! tolere, pas lequel exiger : c'est l'affaire de la norme d'application ou du
//! concepteur. Il ne **dimensionne pas** non plus un cordon : la relation
//! `z = a·√2` est de la geometrie, pas un calcul de resistance.
//!
//! # Exactitude
//!
//! Les limites se calculent en nanometres entiers, coefficients lus en
//! centiemes exacts. `√2` n'est pas exact : la cote equivalente d'un cordon
//! d'angle est donc **annoncee** arrondie au centieme de millimetre, jamais
//! presentee comme exacte.

use mecatool_core::{Conclusion, Length, Provenance, ReasoningStep, Unit, Verdict};
use mecatool_standards::soudure::{
    ElementarySymbol, Imperfection, JointFamily, Limit, QualityLevel, QualityTable, ScopeVerdict,
    SupplementarySymbol, VariantLetter, WeldSymbolTable, WeldingProcess, WeldingProcessTable,
};
use mecatool_standards::Basis;
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::format;
use crate::geometric::{Finding, Severity};

/// Le cote ou se trouve la soudure, lu sur la ligne de reference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Cote de la fleche.
    #[default]
    Arrow,
    /// Cote oppose a la fleche.
    Other,
    /// Des deux cotes : soudure symetrique.
    Both,
}

impl Side {
    pub const fn label_fr(self) -> &'static str {
        match self {
            Side::Arrow => "côté de la flèche",
            Side::Other => "côté opposé à la flèche",
            Side::Both => "des deux côtés",
        }
    }
}

/// Une demande de lecture de symbole, telle que l'ecran la construit.
///
/// Les grandeurs arrivent en texte, en millimetres : c'est le moteur qui les
/// lit, pas l'interface, pour qu'aucune conversion ne vive de l'autre cote de
/// la frontiere.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeldRequest {
    /// Identifiant du symbole elementaire : `fillet`, `single_v`...
    pub symbol: String,
    pub side: Side,
    /// `a`, `z`, `s`, `d` ou `c`.
    pub size_letter: Option<String>,
    pub size_mm: Option<String>,
    /// Soudure discontinue : nombre d'elements.
    pub count: Option<String>,
    /// Soudure discontinue : longueur d'un element.
    pub length_mm: Option<String>,
    /// Soudure discontinue : intervalle entre deux elements.
    pub spacing_mm: Option<String>,
    /// Soudure discontinue alternee, des deux cotes.
    pub staggered: bool,
    /// Identifiants des symboles supplementaires.
    pub supplementary: Vec<String>,
    pub all_around: bool,
    pub field_weld: bool,
    /// Numero ISO 4063 porte dans la queue du symbole.
    pub process: Option<String>,
    /// Niveau de qualite ISO 5817 : `B`, `C` ou `D`.
    pub level: Option<String>,
    /// Epaisseur `t` de la piece la plus mince.
    pub thickness_mm: Option<String>,
    /// Largeur `b` de la surepaisseur, pour chiffrer les limites qui en
    /// dependent.
    pub width_mm: Option<String>,
}

/// Une cote lue, ou deduite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeReading {
    pub letter: String,
    pub name: String,
    pub value: Length,
    /// `a = 5 mm`, ou `z ≈ 7.07 mm` pour une valeur deduite et arrondie.
    pub label: String,
    /// Vrai quand la valeur est deduite par une relation irrationnelle, donc
    /// arrondie.
    pub rounded: bool,
}

/// Une soudure discontinue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntermittentReading {
    pub count: u32,
    pub length: Length,
    pub spacing: Length,
    /// `n × l`, la longueur effectivement soudee.
    pub welded_length: Length,
    /// `n × l (e)`, tel qu'il s'ecrit sur le symbole.
    pub notation: String,
}

/// Une variante de procede (ISO 4063, 2.2) : mode de transfert, nombre
/// d'electrodes, element additionnel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessVariant {
    pub transfer: Option<VariantLetter>,
    /// Le nombre d'electrodes, quand il y en a plus d'une.
    pub electrodes: Option<u32>,
    pub additional: Option<VariantLetter>,
}

impl ProcessVariant {
    fn is_empty(&self) -> bool {
        self.transfer.is_none() && self.electrodes.is_none() && self.additional.is_none()
    }

    /// Les suffixes, dans l'ordre ou ils ont ete lus : `-D-2`.
    fn suffix(&self, order: &[char]) -> String {
        order
            .iter()
            .filter_map(|kind| match kind {
                'T' => self.transfer.as_ref().map(|v| v.letter.clone()),
                'E' => self.electrodes.map(|n| n.to_string()),
                'A' => self.additional.as_ref().map(|v| v.letter.clone()),
                _ => None,
            })
            .map(|part| format!("-{part}"))
            .collect()
    }
}

/// Un procede, et sa place dans la nomenclature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReading {
    pub process: WeldingProcess,
    /// Le numero avec ses variantes : `131-D`.
    pub code: String,
    /// La designation complete selon 2.1 : `ISO 4063 - 131-D`.
    pub designation: String,
    pub variant: ProcessVariant,
    /// Pour un procede hybride (2.3), la designation entiere : `522+15`.
    /// Chaque procede du melange a alors sa propre lecture.
    pub hybrid: Option<String>,
    /// Du groupe au procede lui-meme.
    pub lineage: Vec<WeldingProcess>,
    /// Vrai quand le numero designe un groupe dont MecaTool connait des
    /// subdivisions : il reste a preciser.
    pub is_group: bool,
    pub children: Vec<WeldingProcess>,
    /// Ce que l'ISO 5817 dit de ce procede.
    pub quality_scope: ScopeVerdict,
    pub explanation: String,
}

/// Ce qu'une limite devient, pour une geometrie donnee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitStatus {
    NotPermitted,
    Permitted,
    /// Une borne chiffree.
    Bounded,
    /// Un angle minimal.
    MinAngle,
    /// La borne depend d'une grandeur qui n'a pas ete fournie.
    NeedsInput,
}

/// Une ligne du tableau des limites, chiffree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImperfectionLimit {
    pub reference: String,
    pub iso6520: String,
    pub name: String,
    pub remark: Option<String>,
    /// La plage d'epaisseur retenue, en clair.
    pub thickness_label: String,
    pub status: LimitStatus,
    /// La limite telle que la norme l'ecrit : `h ≤ 1 mm + 0.15 b, max. 7 mm`.
    pub formula: String,
    /// La borne chiffree, quand elle a pu l'etre.
    pub value: Option<Length>,
    /// `h ≤ 2.5 mm`.
    pub value_label: Option<String>,
    /// La grandeur qui manque pour chiffrer.
    pub missing: Option<String>,
    /// Vrai si la borne a du etre arrondie, par defaut, au nanometre.
    pub rounded: bool,
    pub short: bool,
}

/// Ce que tolere un niveau de qualite, pour une soudure donnee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityAssessment {
    pub level: QualityLevel,
    pub joint: JointFamily,
    pub thickness: Option<Length>,
    /// `s` pour une soudure bout a bout, `a` pour une soudure d'angle.
    pub weld_size: Option<Length>,
    pub width: Option<Length>,
    pub limits: Vec<ImperfectionLimit>,
    /// D'ou viennent les grandeurs employees.
    pub notes: Vec<String>,
}

/// Le symbole lu en entier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeldReading {
    /// Le symbole recompose en une ligne.
    pub designation: String,
    pub symbol: ElementarySymbol,
    pub side: Side,
    /// La lecture en clair, une phrase par element du symbole.
    pub sentences: Vec<String>,
    pub size: Option<SizeReading>,
    /// L'autre cote d'un cordon d'angle : `z` pour `a`, `a` pour `z`.
    pub equivalent: Option<SizeReading>,
    pub intermittent: Option<IntermittentReading>,
    pub supplementary: Vec<SupplementarySymbol>,
    pub process: Option<ProcessReading>,
    pub quality: Option<QualityAssessment>,
    pub findings: Vec<Finding>,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Le moteur du domaine soudure.
#[derive(Debug)]
pub struct WeldingEngine {
    processes: &'static WeldingProcessTable,
    symbols: &'static WeldSymbolTable,
    quality: &'static QualityTable,
}

impl WeldingEngine {
    pub fn new() -> Result<Self> {
        Ok(WeldingEngine {
            processes: WeldingProcessTable::embedded()?,
            symbols: WeldSymbolTable::embedded()?,
            quality: QualityTable::embedded()?,
        })
    }

    pub fn process_table(&self) -> &'static WeldingProcessTable {
        self.processes
    }

    pub fn symbol_table(&self) -> &'static WeldSymbolTable {
        self.symbols
    }

    pub fn quality_table(&self) -> &'static QualityTable {
        self.quality
    }

    pub fn provenance(&self) -> Provenance {
        Provenance::new()
            .with(self.symbols.standard().clone())
            .with(self.symbols.cotation_standard().clone())
            .with(self.processes.standard().clone())
            .with(self.quality.standard().clone())
    }

    /// Les lectures d'un numero de procede ou d'un nom d'atelier.
    pub fn read_process(&self, input: &str) -> Result<Vec<ProcessReading>> {
        let cleaned = strip_prefix_ci(input.trim(), "iso 4063")
            .trim_start_matches(['-', ':', ' '])
            .trim();
        let unparsable = |hint: String| EngineError::Unparsable {
            input: input.to_string(),
            hint,
        };
        if cleaned.is_empty() {
            return Err(unparsable(
                "Indiquez un numéro de procédé, par exemple « 135 », ou un nom d'atelier, \
                 par exemple « MAG »."
                    .into(),
            ));
        }

        // Un procede hybride s'ecrit numero + numero (2.3) : chaque procede du
        // melange se lit pour lui-meme.
        if cleaned.contains('+') {
            let parts: Vec<&str> = cleaned.split('+').map(str::trim).collect();
            if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
                return Err(unparsable(
                    "Un procédé hybride s'écrit avec deux numéros séparés par « + », par \
                     exemple « 522+15 »."
                        .into(),
                ));
            }
            let mut readings = Vec::new();
            for part in &parts {
                if !part.starts_with(|c: char| c.is_ascii_digit()) {
                    return Err(unparsable(format!(
                        "« {part} » : un procédé hybride se désigne par les numéros de ses \
                         procédés, pas par un nom d'atelier."
                    )));
                }
                readings.push(self.read_number(input, part)?);
            }
            let whole = readings
                .iter()
                .map(|r| r.code.as_str())
                .collect::<Vec<_>>()
                .join("+");
            for reading in &mut readings {
                reading.hybrid = Some(whole.clone());
                reading.designation = format!("ISO 4063 - {whole}");
                reading.explanation = format!(
                    "Procédé hybride {whole} (ISO 4063, 2.3) : {}",
                    reading.explanation
                );
            }
            return Ok(readings);
        }

        if cleaned.starts_with(|c: char| c.is_ascii_digit()) {
            return Ok(vec![self.read_number(input, cleaned)?]);
        }

        let found = self.processes.by_alias(cleaned);
        if found.is_empty() {
            return Err(unparsable(
                "Ni un numéro de procédé, ni un nom d'atelier connu. Essayez « 135 », « MAG », \
                 « TIG », « SMAW » ou « électrode enrobée »."
                    .into(),
            ));
        }
        Ok(found
            .into_iter()
            .map(|p| self.process_reading(p, ProcessVariant::default(), &[]))
            .collect())
    }

    /// Un numero, suivi ou non de ses variantes : `131`, `131-D`, `131-2`,
    /// `121-C`.
    fn read_number(&self, input: &str, text: &str) -> Result<ProcessReading> {
        let unparsable = |hint: String| EngineError::Unparsable {
            input: input.to_string(),
            hint,
        };
        let mut pieces = text.split('-').map(str::trim);
        let number = pieces.next().unwrap_or_default();
        if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
            return Err(unparsable(format!(
                "« {text} » ne commence pas par un numéro de procédé."
            )));
        }
        let Some(process) = self.processes.by_number(number) else {
            return Err(unparsable(self.unknown_number(number)));
        };

        let mut variant = ProcessVariant::default();
        let mut order = Vec::new();
        for piece in pieces {
            let upper = piece.to_uppercase();
            if !upper.is_empty() && upper.chars().all(|c| c.is_ascii_digit()) {
                let count: u32 = upper.parse().unwrap_or(0);
                if count < 2 || variant.electrodes.is_some() {
                    return Err(unparsable(format!(
                        "« {piece} » : le nombre d'électrodes ne s'indique que s'il y en a \
                         plus d'une, et une seule fois (2.2.3)."
                    )));
                }
                variant.electrodes = Some(count);
                order.push('E');
            } else if let Some(mode) = self
                .processes
                .transfer_modes()
                .iter()
                .find(|m| m.letter == upper)
            {
                if variant.transfer.is_some() {
                    return Err(unparsable(
                        "Un seul mode de transfert par désignation (2.2.2).".into(),
                    ));
                }
                variant.transfer = Some(mode.clone());
                order.push('T');
            } else if let Some(item) = self
                .processes
                .additional_items()
                .iter()
                .find(|m| m.letter == upper)
            {
                if variant.additional.is_some() {
                    return Err(unparsable(
                        "Un seul élément additionnel par désignation (2.2.4).".into(),
                    ));
                }
                variant.additional = Some(item.clone());
                order.push('A');
            } else {
                let letters = |list: &[VariantLetter]| {
                    list.iter()
                        .map(|v| format!("{} ({})", v.letter, v.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                return Err(unparsable(format!(
                    "« {piece} » n'est pas une variante de l'ISO 4063. Modes de transfert : \
                     {}. Éléments additionnels : {}. Nombre d'électrodes : un nombre, à partir \
                     de 2.",
                    letters(self.processes.transfer_modes()),
                    letters(self.processes.additional_items())
                )));
            }
        }
        Ok(self.process_reading(process, variant, &order))
    }

    /// Ce qu'on dit d'un numero que la liste principale ne porte pas.
    fn unknown_number(&self, number: &str) -> String {
        if let Some(replaced) = self.processes.replaced(number) {
            return format!(
                "{number} — {} n'est plus en usage : l'ISO 4063:2009 le range parmi les \
                 procédés remplacés ou dépassés (Annexe A). Il peut encore se lire dans des \
                 documents anciens.",
                replaced.name
            );
        }
        let known = (1..number.len())
            .rev()
            .find_map(|end| self.processes.by_number(&number[..end]));
        match known {
            Some(parent) => format!(
                "Le numéro {number} n'existe pas dans l'ISO 4063:2009. Le groupe le plus proche \
                 est {} — {}.",
                parent.number, parent.name
            ),
            None => format!("Le numéro {number} n'existe pas dans l'ISO 4063:2009."),
        }
    }

    fn process_reading(
        &self,
        process: &WeldingProcess,
        variant: ProcessVariant,
        order: &[char],
    ) -> ProcessReading {
        let lineage: Vec<WeldingProcess> = self
            .processes
            .lineage(&process.number)
            .into_iter()
            .cloned()
            .collect();
        let children: Vec<WeldingProcess> = self
            .processes
            .children(&process.number)
            .into_iter()
            .cloned()
            .collect();
        let is_group = !children.is_empty();
        let ancestry = lineage
            .iter()
            .rev()
            .skip(1)
            .map(|p| format!("{} ({})", p.number, p.name))
            .collect::<Vec<_>>();
        let mut explanation = if ancestry.is_empty() {
            format!(
                "{} — {} : un groupe principal de la nomenclature.",
                process.number, process.name
            )
        } else {
            format!(
                "{} — {}, qui relève de {}.",
                process.number,
                process.name,
                ancestry.join(", puis de ")
            )
        };
        let mut details = Vec::new();
        if let Some(mode) = &variant.transfer {
            details.push(format!("mode de transfert {} : {}", mode.letter, mode.name));
        }
        if let Some(count) = variant.electrodes {
            details.push(format!("{count} électrodes"));
        }
        if let Some(item) = &variant.additional {
            details.push(format!(
                "élément additionnel {} : {}",
                item.letter, item.name
            ));
        }
        if !details.is_empty() {
            explanation.push_str(&format!(" Variante : {}.", details.join(", ")));
        }
        let code = format!("{}{}", process.number, variant.suffix(order));
        ProcessReading {
            process: process.clone(),
            designation: format!("ISO 4063 - {code}"),
            code,
            variant: if variant.is_empty() {
                ProcessVariant::default()
            } else {
                variant
            },
            hybrid: None,
            lineage,
            is_group,
            children,
            quality_scope: self.quality.process_scope().verdict(&process.number),
            explanation,
        }
    }

    /// Lit un symbole de soudure complet.
    pub fn read_weld(&self, request: &WeldRequest) -> Result<WeldReading> {
        let symbol = self
            .symbols
            .symbol(request.symbol.trim())
            .ok_or_else(|| EngineError::Unparsable {
                input: request.symbol.clone(),
                hint: format!(
                    "Symbole élémentaire inconnu. MecaTool en connaît {} : {}.",
                    self.symbols.elementary().len(),
                    self.symbols
                        .elementary()
                        .iter()
                        .map(|s| s.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?
            .clone();

        let mut findings: Vec<Finding> = Vec::new();
        let mut sentences: Vec<String> = Vec::new();

        // Le symbole et le cote.
        let shape = match (request.side, &symbol.both_sides_name) {
            (Side::Both, Some(double)) => double.clone(),
            _ => symbol.name.clone(),
        };
        sentences.push(format!(
            "{} — {}.",
            capitalise(&shape),
            request.side.label_fr()
        ));
        // Les soudures bout a bout et d'angle se font des deux cotes (4.4.3,
        // 5.4.2, 5.5.1), meme quand la norme ne nomme pas la forme double. Pour
        // les autres, elle ne decrit rien de tel.
        if request.side == Side::Both
            && symbol.both_sides_name.is_none()
            && symbol.family == JointFamily::Other
        {
            findings.push(Finding::new(
                "no_double_form",
                Severity::Caution,
                format!(
                    "L'ISO 2553 ne décrit pas de forme double pour la {}. Vérifiez que le \
                     symbole doit bien figurer des deux côtés.",
                    symbol.name
                ),
            ));
        }

        // La cote principale.
        let size = self.size(request, &symbol, &mut findings)?;
        let equivalent = size
            .as_ref()
            .and_then(fillet_equivalent)
            .filter(|_| symbol.family == JointFamily::Fillet)
            .map(|mut equivalent| {
                if let Some(definition) = self.symbols.size(&equivalent.letter) {
                    equivalent.name = definition.name.clone();
                }
                equivalent
            });
        match (&size, symbol.family) {
            (Some(size), _) => {
                let mut sentence = format!("{} {}.", capitalise(&size.name), size.label);
                if let Some(equivalent) = &equivalent {
                    sentence = format!(
                        "{} {} — soit {} ({}), pour un cordon à côtés égaux et à 90°.",
                        capitalise(&size.name),
                        size.label,
                        equivalent.label,
                        equivalent.name
                    );
                }
                sentences.push(sentence);
                // Des deux cotes, chaque cote porte sa cote (5.4.2), meme
                // identique pour une soudure d'angle (5.5.1).
                if request.side == Side::Both && symbol.family != JointFamily::Other {
                    findings.push(Finding::new(
                        "both_sides_sizes",
                        Severity::Note,
                        match symbol.family {
                            JointFamily::Fillet => {
                                "Des deux côtés, la cote de chaque soudure d'angle doit être \
                                 inscrite, même identique (ISO 2553:2013, 5.5.1). MecaTool lit \
                                 la même cote pour les deux."
                            }
                            _ => {
                                "Des deux côtés, les cotes de chaque côté s'indiquent \
                                 séparément (ISO 2553:2013, 5.4.2). MecaTool lit la même cote \
                                 pour les deux."
                            }
                        },
                    ));
                }
            }
            (None, _) if symbol.full_penetration => {
                sentences.push("Sans cote : pénétration complète.".into());
                findings.push(Finding::new(
                    "full_penetration",
                    Severity::Note,
                    format!("Aucune cote s. {}", self.symbols.full_penetration_rule()),
                ));
            }
            (None, _) if symbol.size_required => findings.push(Finding::new(
                "size_required",
                Severity::Error,
                format!(
                    "La {} doit toujours être cotée (ISO 2553:2013, 5.4.4) : sans {}, rien ne \
                     dit la pénétration voulue.",
                    symbol.name,
                    symbol
                        .sizes
                        .iter()
                        .map(|letter| format!("cote {letter}"))
                        .collect::<Vec<_>>()
                        .join(" ni ")
                ),
            )),
            (None, JointFamily::Fillet) => findings.push(Finding::new(
                "fillet_without_size",
                Severity::Caution,
                "Une soudure d'angle se cote : sans épaisseur de gorge a ni côté z, rien ne dit \
                 ce qu'il faut déposer.",
            )),
            (None, _) => {}
        }

        // La discontinuite.
        let intermittent = self.intermittent(request, &symbol, &mut findings)?;
        if let Some(intermittent) = &intermittent {
            sentences.push(format!(
                "Discontinue{} : {} éléments de {} mm, espacés de {} mm — {} mm soudés au total.",
                if request.staggered {
                    " et alternée"
                } else {
                    ""
                },
                intermittent.count,
                format::mm_trimmed(intermittent.length),
                format::mm_trimmed(intermittent.spacing),
                format::mm_trimmed(intermittent.welded_length)
            ));
        } else if request.staggered {
            findings.push(Finding::new(
                "staggered_without_intermittent",
                Severity::Error,
                "« Alternée » qualifie une soudure discontinue : il manque n × l (e).",
            ));
        }
        if request.staggered && request.side != Side::Both {
            findings.push(Finding::new(
                "staggered_one_side",
                Severity::Error,
                "Une soudure alternée l'est d'un côté à l'autre : elle suppose une soudure des \
                 deux côtés.",
            ));
        }

        // Les symboles supplementaires.
        let supplementary = self.supplementary(request, &symbol, &mut findings)?;
        for extra in &supplementary {
            sentences.push(match &extra.meaning {
                Some(meaning) => {
                    format!("{} : {}", capitalise(&extra.name), lowercase_first(meaning))
                }
                None => format!("{}.", capitalise(&extra.name)),
            });
        }
        if request.all_around {
            sentences.push("Sur tout le pourtour de la pièce.".into());
        }
        if request.field_weld {
            sentences.push("Exécutée sur chantier, et non en atelier.".into());
        }

        // Le procede.
        let mut hybrid_parts: Vec<ProcessReading> = Vec::new();
        let process = match present(&request.process) {
            Some(text) => {
                let mut readings = self.read_process(text)?;
                let hybrid = readings[0].hybrid.clone();
                if let Some(whole) = &hybrid {
                    // Chaque procede du melange doit etre vise par le niveau de
                    // qualite : les suivants sont gardes pour le controle du
                    // domaine, plus bas.
                    sentences.push(format!(
                        "Procédé hybride {whole} : {}.",
                        readings
                            .iter()
                            .map(|r| format!("{} ({})", r.process.number, r.process.name))
                            .collect::<Vec<_>>()
                            .join(" + ")
                    ));
                    hybrid_parts = readings[1..].to_vec();
                } else if readings.len() > 1 {
                    findings.push(Finding::new(
                        "ambiguous_process",
                        Severity::Caution,
                        format!(
                            "« {text} » désigne {} numéros : {}. Le symbole porte un numéro, \
                             pas un nom d'atelier.",
                            readings.len(),
                            readings
                                .iter()
                                .map(|r| r.process.number.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ));
                }
                let reading = readings.remove(0);
                if reading.is_group {
                    findings.push(Finding::new(
                        "process_group",
                        Severity::Caution,
                        format!(
                            "{} désigne un groupe de procédés ({}). Précisez lequel : {}.",
                            reading.process.number,
                            reading.process.name,
                            reading
                                .children
                                .iter()
                                .map(|c| c.number.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ));
                }
                if hybrid.is_none() {
                    sentences.push(format!(
                        "Procédé {} : {}.",
                        reading.code, reading.process.name
                    ));
                }
                Some(reading)
            }
            None => None,
        };

        // Le niveau de qualite.
        let quality = match present(&request.level) {
            Some(level) => {
                let thickness = parse_mm(&request.thickness_mm, "Épaisseur t")?;
                let width = parse_mm(&request.width_mm, "Largeur b")?;
                // Un procede hybride : chaque procede du melange doit etre vise.
                for process in process.iter().chain(&hybrid_parts) {
                    match &process.quality_scope {
                        ScopeVerdict::Excluded { reason } => findings.push(Finding::new(
                            "quality_scope_excluded",
                            Severity::Error,
                            format!(
                                "Le niveau de qualité ISO 5817 ne s'applique pas au procédé {} : \
                                 {reason}.",
                                process.process.number
                            ),
                        )),
                        ScopeVerdict::Unknown => {
                            let scope = self.quality.process_scope();
                            let covered = scope.covered_within(&process.process.number);
                            let message = if covered.is_empty() {
                                format!(
                                    "Niveau de qualité ISO 5817 et procédé {} : {}.",
                                    process.process.number, scope.not_listed
                                )
                            } else {
                                format!(
                                    "Le groupe {} n'est visé par l'ISO 5817 que par ses \
                                     procédés {} (article 1 g). Précisez le procédé.",
                                    process.process.number,
                                    covered.join(", ")
                                )
                            };
                            findings.push(Finding::new(
                                "quality_scope_unknown",
                                Severity::Caution,
                                message,
                            ))
                        }
                        ScopeVerdict::InScope => {
                            if let Some(condition) = self
                                .quality
                                .process_scope()
                                .condition(&process.process.number)
                            {
                                findings.push(Finding::new(
                                    "quality_scope_condition",
                                    Severity::Caution,
                                    format!(
                                        "Niveau de qualité ISO 5817 et procédé {} : {condition}.",
                                        process.process.number
                                    ),
                                ));
                            }
                        }
                    }
                }
                // Choix de MecaTool, pas de l'ISO 2553 : les soudures evasees,
                // bout a bout mais jamais a pleine penetration par defaut,
                // restent hors des limites embarquees, comme les soudures
                // rangees hors des deux familles.
                if symbol.family == JointFamily::Other
                    || (symbol.family == JointFamily::Butt && !symbol.full_penetration)
                {
                    findings.push(Finding::new(
                        "quality_not_applicable",
                        Severity::Error,
                        format!(
                            "MecaTool n'applique les limites embarquées qu'aux soudures bout à \
                             bout à pleine pénétration par défaut et aux soudures d'angle, pas \
                             à une {}.",
                            symbol.name
                        ),
                    ));
                    None
                } else {
                    let (weld_size, mut notes) =
                        self.weld_size(&symbol, size.as_ref(), equivalent.as_ref(), thickness);
                    if let (Some(s), Some(t)) = (weld_size, thickness) {
                        if symbol.family == JointFamily::Butt && s > t {
                            findings.push(Finding::new(
                                "penetration_exceeds_thickness",
                                Severity::Error,
                                format!(
                                    "s = {} mm dépasse l'épaisseur t = {} mm : la pénétration ne \
                                     peut pas excéder la pièce.",
                                    format::mm_trimmed(s),
                                    format::mm_trimmed(t)
                                ),
                            ));
                        }
                    }
                    let mut assessment =
                        self.assess_quality(level, symbol.family, thickness, weld_size, width)?;
                    notes.append(&mut assessment.notes);
                    assessment.notes = notes;
                    sentences.push(format!(
                        "Niveau de qualité {} ({}) selon l'ISO 5817.",
                        assessment.level.id, assessment.level.name
                    ));
                    Some(assessment)
                }
            }
            None => None,
        };

        let designation = self.designation(
            &symbol,
            request.side,
            size.as_ref(),
            intermittent.as_ref(),
            process.as_ref(),
            quality.as_ref(),
        );

        let provenance = self.provenance();
        let mut conclusion = match findings.iter().find(|f| f.severity == Severity::Error) {
            Some(error) => Conclusion::new(Verdict::Incompatible, error.message.clone()),
            // Toujours prudent : les donnees ne sont pas confrontees a leur
            // source, et une lecture n'est pas une validation.
            None => Conclusion::new(Verdict::Caution, sentences.join(" ")),
        };
        conclusion.why = self.reasoning(&symbol, request.side, size.as_ref(), quality.as_ref());
        conclusion.warnings = provenance.warnings_fr();

        Ok(WeldReading {
            designation,
            symbol,
            side: request.side,
            sentences,
            size,
            equivalent,
            intermittent,
            supplementary,
            process,
            quality,
            findings,
            conclusion,
            provenance,
        })
    }

    /// Ce que tolere un niveau, pour une geometrie donnee.
    ///
    /// Chaque grandeur est facultative : une limite qui depend d'une grandeur
    /// absente reste ecrite sous sa forme litterale, avec ce qui manque pour la
    /// chiffrer. Rien n'est suppose.
    pub fn assess_quality(
        &self,
        level: &str,
        joint: JointFamily,
        thickness: Option<Length>,
        weld_size: Option<Length>,
        width: Option<Length>,
    ) -> Result<QualityAssessment> {
        let level_def = self
            .quality
            .level(level)
            .ok_or_else(|| EngineError::Unparsable {
                input: level.to_string(),
                hint: "Niveau de qualité inconnu : B, C ou D.".into(),
            })?
            .clone();
        if joint == JointFamily::Other {
            return Err(EngineError::Unparsable {
                input: level.to_string(),
                hint: "Les limites embarquées portent sur les soudures bout à bout et d'angle."
                    .into(),
            });
        }
        if let Some(t) = thickness {
            if t < self.quality.minimum_thickness() {
                return Err(EngineError::Unparsable {
                    input: format!("t = {} mm", format::mm_trimmed(t)),
                    hint: format!(
                        "L'ISO 5817 s'applique à partir de {} mm d'épaisseur. En dessous, \
                         MecaTool refuse de conclure.",
                        format::mm_trimmed(self.quality.minimum_thickness())
                    ),
                });
            }
        }

        let values = Values {
            joint,
            t: thickness,
            weld: weld_size,
            b: width,
        };
        let limits = self
            .quality
            .imperfections()
            .iter()
            .filter(|i| i.applies_to.includes(joint))
            .map(|i| evaluate(i, &level_def.id, &values))
            .collect();

        Ok(QualityAssessment {
            level: level_def,
            joint,
            thickness,
            weld_size,
            width,
            limits,
            notes: vec![format!(
                "Sélection de {} lignes du Tableau 1 de l'ISO 5817:2014 : défauts superficiels \
                 et défauts géométriques. Les défauts internes, les défauts multiples, les \
                 projections et la coloration du revenu ne sont pas embarqués — une \
                 imperfection absente n'est pas une imperfection admise.",
                self.quality.imperfections().len()
            )],
        })
    }

    /// La grandeur `s` ou `a` a employer, et d'ou elle vient.
    fn weld_size(
        &self,
        symbol: &ElementarySymbol,
        size: Option<&SizeReading>,
        equivalent: Option<&SizeReading>,
        thickness: Option<Length>,
    ) -> (Option<Length>, Vec<String>) {
        match symbol.family {
            JointFamily::Butt => match size {
                Some(size) => (Some(size.value), vec![]),
                None => (
                    thickness,
                    thickness
                        .map(|t| {
                            vec![format!(
                                "s = t = {} mm : sans cote, la soudure est à pleine \
                                 pénétration.",
                                format::mm_trimmed(t)
                            )]
                        })
                        .unwrap_or_default(),
                ),
            },
            JointFamily::Fillet => match (size, equivalent) {
                (Some(size), _) if size.letter == "a" => (Some(size.value), vec![]),
                (Some(_), Some(a)) => (
                    Some(a.value),
                    vec![format!(
                        "a déduit de z : {} (z / √2, arrondi par défaut au nanomètre).",
                        a.label
                    )],
                ),
                _ => (None, vec![]),
            },
            JointFamily::Other => (None, vec![]),
        }
    }

    fn size(
        &self,
        request: &WeldRequest,
        symbol: &ElementarySymbol,
        findings: &mut Vec<Finding>,
    ) -> Result<Option<SizeReading>> {
        let value = parse_mm(&request.size_mm, "Cote")?;
        let letter = present(&request.size_letter).map(str::to_string);
        let (letter, value) = match (letter, value) {
            (None, None) => return Ok(None),
            (Some(letter), Some(value)) => (letter, value),
            (None, Some(_)) => {
                return Err(EngineError::Unparsable {
                    input: request.size_mm.clone().unwrap_or_default(),
                    hint: format!(
                        "Une cote se préfixe de sa lettre. Pour une {} : {}.",
                        symbol.name,
                        if symbol.sizes.is_empty() {
                            "aucune cote principale".to_string()
                        } else {
                            symbol.sizes.join(" ou ")
                        }
                    ),
                })
            }
            (Some(letter), None) => {
                return Err(EngineError::Unparsable {
                    input: letter.clone(),
                    hint: format!("La cote {letter} attend une valeur en millimètres."),
                })
            }
        };
        let definition = self
            .symbols
            .size(&letter)
            .ok_or_else(|| EngineError::Unparsable {
                input: letter.clone(),
                hint: "Lettres de cote connues : a, z, s, d, c.".into(),
            })?;
        if !value.is_positive() {
            return Err(EngineError::Unparsable {
                input: format::mm_trimmed(value),
                hint: "Une cote de soudure est strictement positive.".into(),
            });
        }
        if !symbol.sizes.contains(&letter) {
            findings.push(Finding::new(
                "size_not_for_symbol",
                Severity::Error,
                format!(
                    "{letter} ({}) ne s'applique pas à une {}. {}",
                    definition.name,
                    symbol.name,
                    if symbol.sizes.is_empty() {
                        "Ce symbole ne porte pas de cote principale.".to_string()
                    } else {
                        format!("Cotes admises : {}.", symbol.sizes.join(", "))
                    }
                ),
            ));
        }
        // Une meme lettre peut nommer une autre grandeur selon le symbole : s
        // est l'epaisseur du rechargement sur une soudure de rechargement.
        let name = symbol
            .size_name(&letter)
            .map_or_else(|| definition.name.clone(), str::to_string);
        Ok(Some(SizeReading {
            letter: letter.clone(),
            name,
            value,
            label: format!("{letter} = {} mm", format::mm_trimmed(value)),
            rounded: false,
        }))
    }

    fn intermittent(
        &self,
        request: &WeldRequest,
        symbol: &ElementarySymbol,
        findings: &mut Vec<Finding>,
    ) -> Result<Option<IntermittentReading>> {
        let count = present(&request.count);
        let length = parse_mm(&request.length_mm, "Longueur l")?;
        let spacing = parse_mm(&request.spacing_mm, "Intervalle e")?;
        let (count, length, spacing) = match (count, length, spacing) {
            (None, None, None) => return Ok(None),
            (Some(count), Some(length), Some(spacing)) => (count, length, spacing),
            _ => {
                return Err(EngineError::Unparsable {
                    input: "n × l (e)".into(),
                    hint: "Une soudure discontinue se décrit entièrement : nombre d'éléments n, \
                           longueur l et intervalle e."
                        .into(),
                })
            }
        };
        let count: u32 = count
            .trim()
            .parse()
            .ok()
            .filter(|n| *n >= 2)
            .ok_or_else(|| EngineError::Unparsable {
                input: count.to_string(),
                hint: "Le nombre d'éléments est un entier d'au moins 2 : un seul élément n'est \
                       pas une soudure discontinue."
                    .into(),
            })?;
        if !length.is_positive() || !spacing.is_positive() {
            return Err(EngineError::Unparsable {
                input: "n × l (e)".into(),
                hint: "La longueur et l'intervalle sont strictement positifs.".into(),
            });
        }
        if symbol.family == JointFamily::Other {
            findings.push(Finding::new(
                "intermittent_unusual",
                Severity::Caution,
                format!(
                    "La notation n × l (e) se rencontre sur les soudures bout à bout et d'angle. \
                     Sur une {}, MecaTool la restitue sans la juger.",
                    symbol.name
                ),
            ));
        }
        let welded_length = Length::from_nanometres(length.nanometres() * i64::from(count));
        Ok(Some(IntermittentReading {
            count,
            length,
            spacing,
            welded_length,
            notation: format!(
                "{} × {} ({})",
                count,
                format::mm_trimmed(length),
                format::mm_trimmed(spacing)
            ),
        }))
    }

    fn supplementary(
        &self,
        request: &WeldRequest,
        symbol: &ElementarySymbol,
        findings: &mut Vec<Finding>,
    ) -> Result<Vec<SupplementarySymbol>> {
        let mut chosen = Vec::new();
        for id in &request.supplementary {
            let extra = self
                .symbols
                .supplementary_symbol(id.trim())
                .ok_or_else(|| EngineError::Unparsable {
                    input: id.clone(),
                    hint: format!(
                        "Symbole supplémentaire inconnu. MecaTool connaît : {}.",
                        self.symbols
                            .supplementary()
                            .iter()
                            .map(|s| s.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                })?;
            if !extra.shown_on(symbol) {
                findings.push(Finding::new(
                    "supplementary_unusual",
                    Severity::Caution,
                    format!(
                        "L'ISO 2553 ne montre le symbole « {} » sur aucune {}. Ce n'est pas une \
                         interdiction : vérifiez qu'il s'applique.",
                        extra.name, symbol.name
                    ),
                ));
            }
            if !chosen
                .iter()
                .any(|c: &SupplementarySymbol| c.id == extra.id)
            {
                chosen.push(extra.clone());
            }
        }

        // Une surface ne peut pas etre a la fois plate, convexe et concave.
        let has = |id: &str| chosen.iter().any(|c| c.id == id);
        let shapes = ["flush", "convex", "concave"]
            .iter()
            .filter(|id| has(id))
            .count();
        if shapes > 1 {
            findings.push(Finding::new(
                "contradictory_shape",
                Severity::Error,
                "Une surface de soudure est plate, convexe ou concave : pas plusieurs à la fois.",
            ));
        }
        Ok(chosen)
    }

    fn designation(
        &self,
        symbol: &ElementarySymbol,
        side: Side,
        size: Option<&SizeReading>,
        intermittent: Option<&IntermittentReading>,
        process: Option<&ProcessReading>,
        quality: Option<&QualityAssessment>,
    ) -> String {
        let mut parts = Vec::new();
        let shape = match (side, &symbol.both_sides_name) {
            (Side::Both, Some(double)) => double.clone(),
            _ => symbol.name.clone(),
        };
        match size {
            Some(size) => parts.push(format!(
                "{}{} {shape}",
                size.letter,
                format::mm_trimmed(size.value)
            )),
            None => parts.push(shape),
        }
        // Le nom de la forme double dit deja les deux cotes ; sans nom, on le
        // precise.
        if side != Side::Both || symbol.both_sides_name.is_none() {
            parts.push(side.label_fr().to_string());
        }
        if let Some(intermittent) = intermittent {
            parts.push(intermittent.notation.clone());
        }
        let mut tail = Vec::new();
        if let Some(process) = process {
            tail.push(
                process
                    .hybrid
                    .clone()
                    .unwrap_or_else(|| process.code.clone()),
            );
        }
        if let Some(quality) = quality {
            tail.push(format!("ISO 5817-{}", quality.level.id));
        }
        if !tail.is_empty() {
            parts.push(tail.join(" / "));
        }
        parts.join(" · ")
    }

    fn reasoning(
        &self,
        symbol: &ElementarySymbol,
        side: Side,
        size: Option<&SizeReading>,
        quality: Option<&QualityAssessment>,
    ) -> Vec<ReasoningStep> {
        let mut steps = vec![
            ReasoningStep::new("Symbole élémentaire")
                .with_expression(format!("n° {} — {}", symbol.number, symbol.name))
                .with_value(match (symbol.family, symbol.size_required) {
                    (JointFamily::Butt, true) => {
                        "soudure bout à bout : se cote en s, et doit toujours l'être (5.4.4)"
                    }
                    (JointFamily::Butt, false) => "soudure bout à bout : se cote en s",
                    (JointFamily::Fillet, _) => "soudure d'angle : se cote en a ou en z",
                    (JointFamily::Other, _) => "ni bout à bout, ni d'angle",
                }),
            ReasoningStep::new("Côté")
                .with_expression(side.label_fr())
                .with_value(format!(
                    "Lu tel que déclaré : le côté se lit selon le système du dessin. {} {}",
                    self.symbols
                        .systems()
                        .iter()
                        .map(|s| format!("{} : {}.", capitalise(&s.name), s.reference_line))
                        .collect::<Vec<_>>()
                        .join(" "),
                    self.symbols
                        .system_rules()
                        .first()
                        .cloned()
                        .unwrap_or_default()
                )),
        ];
        if let Some(size) = size {
            steps.push(
                ReasoningStep::new("Cote principale")
                    .with_expression(size.name.clone())
                    .with_value(size.label.clone()),
            );
        }
        if let Some(quality) = quality {
            steps.push(
                ReasoningStep::new("Niveau de qualité")
                    .with_expression(format!("ISO 5817, niveau {}", quality.level.id))
                    .with_value(
                        "Le niveau est fixé par la norme d'application ou par le concepteur. \
                         MecaTool dit ce qu'il tolère, il ne le choisit pas.",
                    ),
            );
            let counted =
                |status: LimitStatus| quality.limits.iter().filter(|l| l.status == status).count();
            steps.push(
                ReasoningStep::new("Limites chiffrées")
                    .with_expression(format!("{} lignes examinées", quality.limits.len()))
                    .with_value(format!(
                        "{} non admises, {} bornées, {} angles minimaux, {} admises, {} en \
                         attente d'une grandeur",
                        counted(LimitStatus::NotPermitted),
                        counted(LimitStatus::Bounded),
                        counted(LimitStatus::MinAngle),
                        counted(LimitStatus::Permitted),
                        counted(LimitStatus::NeedsInput)
                    )),
            );
        }
        steps
    }
}

/// Les grandeurs disponibles pour chiffrer une limite.
struct Values {
    joint: JointFamily,
    t: Option<Length>,
    weld: Option<Length>,
    b: Option<Length>,
}

impl Values {
    fn get(&self, basis: Basis) -> Option<Length> {
        match basis {
            Basis::T => self.t,
            Basis::B => self.b,
            Basis::A | Basis::S | Basis::Weld => self.weld,
        }
    }

    /// Le symbole a afficher pour « s ou a », selon le joint.
    fn symbol(&self, basis: Basis) -> &'static str {
        match (basis, self.joint) {
            (Basis::Weld, JointFamily::Butt) => "s",
            (Basis::Weld, _) => "a",
            (other, _) => other.symbol(),
        }
    }
}

/// Chiffre une imperfection pour un niveau.
fn evaluate(imperfection: &Imperfection, level: &str, values: &Values) -> ImperfectionLimit {
    let base = ImperfectionLimit {
        reference: imperfection.reference.clone(),
        iso6520: imperfection.iso6520.clone(),
        name: imperfection.name.clone(),
        remark: imperfection.remark.clone(),
        thickness_label: String::new(),
        status: LimitStatus::NeedsInput,
        formula: String::new(),
        value: None,
        value_label: None,
        missing: None,
        rounded: false,
        short: false,
    };

    // La ligne a retenir depend de t. Sans t, une imperfection a plusieurs
    // lignes ne se chiffre pas : on montre chaque forme, et ce qui manque.
    let row = match (values.t, imperfection.rows.len()) {
        (Some(t), _) => imperfection.rows.iter().find(|r| r.thickness.contains(t)),
        (None, 1) => imperfection.rows.first(),
        (None, _) => None,
    };
    let Some(row) = row else {
        let formula = imperfection
            .rows
            .iter()
            .filter_map(|r| {
                r.limit(level).map(|limit| {
                    format!(
                        "{} : {}",
                        thickness_label(&r.thickness),
                        describe(limit, values)
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(" ; ");
        return ImperfectionLimit {
            thickness_label: "selon t".into(),
            formula,
            missing: Some("t".into()),
            ..base
        };
    };
    let Some(limit) = row.limit(level) else {
        return base;
    };
    let formula = describe(limit, values);
    let thickness_label = thickness_label(&row.thickness);

    match limit {
        Limit::NotPermitted => ImperfectionLimit {
            thickness_label,
            status: LimitStatus::NotPermitted,
            formula,
            ..base
        },
        Limit::Permitted { .. } => ImperfectionLimit {
            thickness_label,
            status: LimitStatus::Permitted,
            formula,
            ..base
        },
        Limit::MinAngle { .. } => ImperfectionLimit {
            thickness_label,
            status: LimitStatus::MinAngle,
            formula,
            ..base
        },
        Limit::Bound {
            measure,
            constant,
            factor_hundredths,
            of,
            max,
            short,
        } => {
            let mut total = i128::from(constant.nanometres()) * 100;
            if *factor_hundredths != 0 {
                let basis = of.expect("valide au chargement");
                match values.get(basis) {
                    Some(value) => {
                        total += i128::from(value.nanometres()) * i128::from(*factor_hundredths)
                    }
                    None => {
                        return ImperfectionLimit {
                            thickness_label,
                            formula,
                            missing: Some(values.symbol(basis).to_string()),
                            short: *short,
                            ..base
                        }
                    }
                }
            }
            // Une borne admissible s'arrondit par defaut : c'est le sens qui ne
            // tolere jamais davantage que la norme. Et l'arrondi est signale.
            let rounded = total % 100 != 0;
            let mut value = Length::from_nanometres((total / 100) as i64);
            if let Some(max) = max {
                value = value.min(*max);
            }
            ImperfectionLimit {
                thickness_label,
                status: LimitStatus::Bounded,
                formula,
                value_label: Some(format!("{measure} ≤ {} mm", format::mm_trimmed(value))),
                value: Some(value),
                rounded,
                short: *short,
                ..base
            }
        }
    }
}

/// La limite telle que la norme l'ecrit.
fn describe(limit: &Limit, values: &Values) -> String {
    match limit {
        Limit::NotPermitted => "non admis".into(),
        Limit::Permitted { condition: None } => "admis".into(),
        Limit::Permitted {
            condition: Some(condition),
        } => format!("admis {condition}"),
        Limit::MinAngle { degrees } => format!("α ≥ {degrees}°"),
        Limit::Bound {
            measure,
            constant,
            factor_hundredths,
            of,
            max,
            short,
        } => {
            let mut terms = Vec::new();
            if !constant.is_zero() {
                terms.push(format!("{} mm", format::mm_trimmed(*constant)));
            }
            if *factor_hundredths != 0 {
                let basis = of.map(|b| values.symbol(b)).unwrap_or("?");
                terms.push(format!("{} {basis}", hundredths_label(*factor_hundredths)));
            }
            let mut text = format!("{measure} ≤ {}", terms.join(" + "));
            if let Some(max) = max {
                text.push_str(&format!(", max. {} mm", format::mm_trimmed(*max)));
            }
            if *short {
                text.push_str(" (défauts courts)");
            }
            text
        }
    }
}

fn thickness_label(range: &mecatool_standards::ThicknessRange) -> String {
    match (range.from, range.above, range.to) {
        (Some(from), _, Some(to)) => format!(
            "{} ≤ t ≤ {} mm",
            format::mm_trimmed(from),
            format::mm_trimmed(to)
        ),
        (Some(from), _, None) => format!("t ≥ {} mm", format::mm_trimmed(from)),
        (None, Some(above), Some(to)) => format!(
            "{} < t ≤ {} mm",
            format::mm_trimmed(above),
            format::mm_trimmed(to)
        ),
        (None, Some(above), None) => format!("t > {} mm", format::mm_trimmed(above)),
        (None, None, _) => "toute épaisseur".into(),
    }
}

fn hundredths_label(value: i64) -> String {
    let whole = value / 100;
    let fraction = value % 100;
    if fraction == 0 {
        format!("{whole}")
    } else if fraction % 10 == 0 {
        format!("{whole}.{}", fraction / 10)
    } else {
        format!("{whole}.{fraction:02}")
    }
}

/// L'autre cote d'un cordon d'angle a cotes egales et a 90°.
///
/// `z = a·√2` et `a = z/√2`. La racine n'est pas exacte : la valeur est
/// calculee par defaut au nanometre, et annoncee arrondie au centieme.
fn fillet_equivalent(size: &SizeReading) -> Option<SizeReading> {
    let nm = i128::from(size.value.nanometres());
    let (letter, name, value) = match size.letter.as_str() {
        "a" => ("z", "longueur du côté", integer_sqrt(2 * nm * nm)),
        "z" => ("a", "épaisseur de gorge", integer_sqrt(nm * nm / 2)),
        _ => return None,
    };
    let value = Length::from_nanometres(value as i64);
    Some(SizeReading {
        letter: letter.to_string(),
        name: name.to_string(),
        value,
        label: format!(
            "{letter} ≈ {} mm",
            value.to_decimal_string(Unit::Millimetre, 2)
        ),
        rounded: true,
    })
}

/// Racine carree entiere, arrondie vers le bas.
fn integer_sqrt(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }
    let mut x = (value as f64).sqrt() as i128;
    // L'estimation flottante est corrigee en entiers : le resultat est exact.
    while x * x > value {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= value {
        x += 1;
    }
    x
}

fn present(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

fn parse_mm(value: &Option<String>, label: &str) -> Result<Option<Length>> {
    match present(value) {
        None => Ok(None),
        Some(text) => Length::parse(&text.replace(',', "."), Unit::Millimetre)
            .map(Some)
            .map_err(|source| EngineError::Unparsable {
                input: text.to_string(),
                hint: format!("{label} illisible : {source}."),
            }),
    }
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> &'a str {
    if text.len() >= prefix.len()
        && text.is_char_boundary(prefix.len())
        && text[..prefix.len()].eq_ignore_ascii_case(prefix)
    {
        &text[prefix.len()..]
    } else {
        text
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> WeldingEngine {
        WeldingEngine::new().expect("les donnees soudure doivent charger")
    }

    fn mm(text: &str) -> Length {
        Length::parse(text, Unit::Millimetre).unwrap()
    }

    fn fillet_a5() -> WeldRequest {
        WeldRequest {
            symbol: "fillet".into(),
            size_letter: Some("a".into()),
            size_mm: Some("5".into()),
            ..WeldRequest::default()
        }
    }

    fn limit<'a>(quality: &'a QualityAssessment, iso6520: &str) -> &'a ImperfectionLimit {
        quality
            .limits
            .iter()
            .find(|l| l.iso6520 == iso6520)
            .unwrap_or_else(|| panic!("{iso6520} absent"))
    }

    #[test]
    fn chaque_procede_dun_hybride_passe_le_domaine_de_liso_5817() {
        // 135 est vise, 72 ne l'est pas : le second ne doit pas passer
        // inapercu derriere le premier.
        let request = WeldRequest {
            process: Some("135+72".into()),
            level: Some("C".into()),
            thickness_mm: Some("10".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(
            reading
                .findings
                .iter()
                .any(|f| f.code == "quality_scope_unknown" && f.message.contains("72")),
            "{:?}",
            reading.findings
        );
        assert!(
            reading.designation.contains("135+72"),
            "{}",
            reading.designation
        );
    }

    #[test]
    fn lit_un_numero_et_sa_hierarchie() {
        let readings = engine().read_process("135").unwrap();
        assert_eq!(readings.len(), 1);
        let reading = &readings[0];
        assert!(
            reading
                .process
                .name
                .starts_with("soudage MAG avec fil-électrode fusible"),
            "{}",
            reading.process.name
        );
        assert_eq!(reading.designation, "ISO 4063 - 135");
        assert_eq!(reading.lineage.len(), 3);
        assert!(!reading.is_group);
        assert!(reading.explanation.contains("13 ("));
        assert_eq!(reading.quality_scope, ScopeVerdict::InScope);
    }

    #[test]
    fn un_nom_datelier_rend_toutes_ses_lectures() {
        // « MAG » designe trois numeros : MecaTool ne choisit pas.
        let readings = engine().read_process("MAG").unwrap();
        let numbers: Vec<&str> = readings.iter().map(|r| r.process.number.as_str()).collect();
        assert_eq!(numbers, ["135", "136", "138"]);
    }

    #[test]
    fn un_sous_groupe_dit_quil_reste_a_preciser() {
        let reading = &engine().read_process("13").unwrap()[0];
        assert!(reading.is_group);
        assert!(reading.children.iter().any(|c| c.number == "135"));
    }

    #[test]
    fn un_numero_remplace_renvoie_a_lannexe_a() {
        // 137 etait le MIG avec fil fourre : l'edition 2009 l'a remplace.
        let err = engine().read_process("137").unwrap_err().to_string();
        assert!(err.contains("Annexe A"), "{err}");
        assert!(err.contains("documents anciens"), "{err}");
    }

    #[test]
    fn un_numero_hors_norme_est_dit_inexistant() {
        // La nomenclature est complete : l'absence vaut inexistence.
        let err = engine().read_process("139").unwrap_err().to_string();
        assert!(err.contains("n'existe pas dans l'ISO 4063:2009"), "{err}");
        assert!(err.contains("13 —"), "{err}");
    }

    #[test]
    fn lit_les_variantes_de_procede() {
        // Les trois exemples de l'article 2.2.
        let short = &engine().read_process("ISO 4063 - 131-D").unwrap()[0];
        assert_eq!(short.code, "131-D");
        assert_eq!(short.designation, "ISO 4063 - 131-D");
        assert_eq!(
            short.variant.transfer.as_ref().unwrap().name,
            "transfert par court-circuit"
        );
        let two = &engine().read_process("131-2").unwrap()[0];
        assert_eq!(two.variant.electrodes, Some(2));
        let cold = &engine().read_process("121-c").unwrap()[0];
        assert_eq!(cold.code, "121-C");
        assert_eq!(cold.variant.additional.as_ref().unwrap().name, "fil froid");
        assert!(cold.explanation.contains("fil froid"));
    }

    #[test]
    fn une_variante_inconnue_dit_lesquelles_existent() {
        let err = engine().read_process("131-X").unwrap_err().to_string();
        assert!(err.contains("D (transfert par court-circuit)"), "{err}");
        let err = engine().read_process("131-D-S").unwrap_err().to_string();
        assert!(err.contains("Un seul mode de transfert"), "{err}");
        let err = engine().read_process("131-1").unwrap_err().to_string();
        assert!(err.contains("plus d'une"), "{err}");
    }

    #[test]
    fn lit_un_procede_hybride() {
        // L'exemple de l'article 2.3 : laser et plasma ensemble.
        let readings = engine().read_process("522+15").unwrap();
        let numbers: Vec<&str> = readings.iter().map(|r| r.process.number.as_str()).collect();
        assert_eq!(numbers, ["522", "15"]);
        assert!(readings
            .iter()
            .all(|r| r.hybrid.as_deref() == Some("522+15")));
        assert_eq!(readings[0].designation, "ISO 4063 - 522+15");
    }

    #[test]
    fn une_designation_us_se_lit() {
        let readings = engine().read_process("GTAW").unwrap();
        assert_eq!(readings[0].process.number, "14");
    }

    #[test]
    fn accepte_le_prefixe_de_la_norme() {
        let readings = engine().read_process("ISO 4063-141").unwrap();
        assert_eq!(readings[0].process.number, "141");
    }

    #[test]
    fn un_cordon_dangle_donne_son_cote_equivalent_arrondi_et_annonce() {
        let reading = engine().read_weld(&fillet_a5()).unwrap();
        let z = reading.equivalent.unwrap();
        assert_eq!(z.letter, "z");
        // 5·√2 = 7,0710678... : par defaut au nanometre, affiche au centieme.
        assert_eq!(z.value, Length::from_nanometres(7_071_067));
        assert_eq!(z.label, "z ≈ 7.07 mm");
        assert!(z.rounded);
        assert!(reading.sentences.iter().any(|s| s.contains("z ≈ 7.07 mm")));
    }

    #[test]
    fn la_relation_inverse_tombe_juste() {
        let request = WeldRequest {
            size_letter: Some("z".into()),
            size_mm: Some("7".into()),
            ..fillet_a5()
        };
        let a = engine().read_weld(&request).unwrap().equivalent.unwrap();
        assert_eq!(a.value, Length::from_nanometres(4_949_747));
    }

    #[test]
    fn une_cote_etrangere_au_symbole_est_une_faute() {
        let request = WeldRequest {
            symbol: "single_v".into(),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Incompatible);
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "size_not_for_symbol" && f.message.contains("Cotes admises : s")));
    }

    #[test]
    fn une_soudure_bout_a_bout_sans_cote_est_a_pleine_penetration() {
        let request = WeldRequest {
            symbol: "single_v".into(),
            side: Side::Both,
            ..WeldRequest::default()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "full_penetration"));
        // Et un V des deux cotes se nomme double V (tableau 2).
        assert!(reading
            .designation
            .contains("soudure bout à bout en double V"));
        // La regle vient de la norme, mot pour mot.
        let finding = reading
            .findings
            .iter()
            .find(|f| f.code == "full_penetration")
            .unwrap();
        assert!(finding.message.contains("sauf spécification contraire"));
    }

    #[test]
    fn une_soudure_dangle_sans_cote_est_signalee() {
        let request = WeldRequest {
            symbol: "fillet".into(),
            ..WeldRequest::default()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "fillet_without_size"));
    }

    #[test]
    fn la_discontinuite_se_lit_et_se_totalise() {
        let request = WeldRequest {
            count: Some("3".into()),
            length_mm: Some("100".into()),
            spacing_mm: Some("50".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        let intermittent = reading.intermittent.unwrap();
        assert_eq!(intermittent.notation, "3 × 100 (50)");
        assert_eq!(intermittent.welded_length, mm("300"));
        assert!(reading.designation.contains("3 × 100 (50)"));
    }

    #[test]
    fn une_discontinuite_incomplete_est_refusee() {
        let request = WeldRequest {
            count: Some("3".into()),
            ..fillet_a5()
        };
        assert!(engine().read_weld(&request).is_err());
    }

    #[test]
    fn alternee_dun_seul_cote_est_une_faute() {
        let request = WeldRequest {
            count: Some("3".into()),
            length_mm: Some("100".into()),
            spacing_mm: Some("50".into()),
            staggered: true,
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "staggered_one_side"));
    }

    #[test]
    fn deux_formes_de_surface_se_contredisent() {
        let request = WeldRequest {
            supplementary: vec!["convex".into(), "concave".into()],
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "contradictory_shape"));
    }

    #[test]
    fn un_nom_datelier_dans_la_queue_est_ambigu() {
        let request = WeldRequest {
            process: Some("MAG".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "ambiguous_process"));
    }

    #[test]
    fn le_niveau_c_se_chiffre_pour_un_cordon_a5() {
        let request = WeldRequest {
            process: Some("135".into()),
            level: Some("C".into()),
            thickness_mm: Some("10".into()),
            width_mm: Some("10".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Caution);
        assert!(reading.designation.ends_with("135 / ISO 5817-C"));
        let quality = reading.quality.unwrap();

        // Convexite excessive : h ≤ 1 + 0,15 × 10 = 2,5 mm (plafond 4).
        let convexity = limit(&quality, "503");
        assert_eq!(convexity.value, Some(mm("2.5")));
        assert_eq!(convexity.value_label.as_deref(), Some("h ≤ 2.5 mm"));
        assert_eq!(convexity.formula, "h ≤ 1 mm + 0.15 b, max. 4 mm");

        // Epaisseur de gorge excessive : 1 + 0,2 × 5 = 2 mm.
        assert_eq!(limit(&quality, "5214").value, Some(mm("2")));

        // Caniveau, t > 3 : 0,1 × 10 = 1, plafonne a 0,5.
        assert_eq!(limit(&quality, "5011, 5012").value, Some(mm("0.5")));

        // Fissure : jamais.
        assert_eq!(limit(&quality, "100").status, LimitStatus::NotPermitted);

        // Les lignes propres aux soudures bout a bout n'apparaissent pas.
        assert!(quality.limits.iter().all(|l| l.iso6520 != "502"));
    }

    #[test]
    fn soufflure_s_pour_bout_a_bout_a_pour_angle() {
        let butt = engine()
            .assess_quality("D", JointFamily::Butt, Some(mm("10")), Some(mm("10")), None)
            .unwrap();
        let pore = limit(&butt, "2017");
        assert_eq!(pore.formula, "d ≤ 0.3 s, max. 3 mm");
        assert_eq!(pore.value, Some(mm("3")));

        let fillet = engine()
            .assess_quality(
                "D",
                JointFamily::Fillet,
                Some(mm("10")),
                Some(mm("4")),
                None,
            )
            .unwrap();
        let pore = limit(&fillet, "2017");
        assert_eq!(pore.formula, "d ≤ 0.3 a, max. 3 mm");
        assert_eq!(pore.value, Some(mm("1.2")));
    }

    #[test]
    fn une_grandeur_absente_laisse_la_limite_litterale() {
        // Sans b, la surepaisseur ne se chiffre pas : elle reste ecrite, avec ce
        // qui manque. Rien n'est suppose.
        let quality = engine()
            .assess_quality("B", JointFamily::Butt, Some(mm("8")), None, None)
            .unwrap();
        let excess = limit(&quality, "502");
        assert_eq!(excess.status, LimitStatus::NeedsInput);
        assert_eq!(excess.missing.as_deref(), Some("b"));
        assert_eq!(excess.formula, "h ≤ 1 mm + 0.1 b, max. 5 mm");
    }

    #[test]
    fn sans_epaisseur_les_lignes_a_plusieurs_plages_attendent_t() {
        let quality = engine()
            .assess_quality("C", JointFamily::Butt, None, None, None)
            .unwrap();
        let undercut = limit(&quality, "5011, 5012");
        assert_eq!(undercut.missing.as_deref(), Some("t"));
        assert!(undercut.formula.contains("t > 3 mm"));
        // Une ligne a plage unique, elle, se lit sans t.
        assert_eq!(limit(&quality, "100").status, LimitStatus::NotPermitted);
    }

    #[test]
    fn en_dessous_de_un_demi_millimetre_la_norme_ne_sapplique_pas() {
        let err = engine()
            .assess_quality("C", JointFamily::Butt, Some(mm("0.4")), None, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("refuse de conclure"), "{err}");
    }

    #[test]
    fn les_bornes_restent_dans_lordre_des_niveaux() {
        // Un controle bout en bout : pour une meme geometrie, B ne tolere jamais
        // davantage que C, ni C que D.
        let value = |level: &str| {
            engine()
                .assess_quality(
                    level,
                    JointFamily::Fillet,
                    Some(mm("12")),
                    Some(mm("6")),
                    Some(mm("14")),
                )
                .unwrap()
        };
        let (b, c, d) = (value("B"), value("C"), value("D"));
        for ((lb, lc), ld) in b.limits.iter().zip(&c.limits).zip(&d.limits) {
            if let (Some(vb), Some(vc), Some(vd)) = (lb.value, lc.value, ld.value) {
                assert!(vb <= vc && vc <= vd, "{}", lb.name);
            }
        }
    }

    #[test]
    fn le_brasage_nest_pas_vise_par_la_norme() {
        let request = WeldRequest {
            process: Some("91".into()),
            level: Some("C".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Incompatible);
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "quality_scope_excluded"));
    }

    #[test]
    fn le_soudage_oxygaz_nest_vise_que_pour_lacier() {
        let request = WeldRequest {
            process: Some("311".into()),
            level: Some("C".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Caution);
        let finding = reading
            .findings
            .iter()
            .find(|f| f.code == "quality_scope_condition")
            .expect("la reserve de l'article 1 g) doit etre dite");
        assert!(finding.message.contains("acier"), "{}", finding.message);
        assert!(reading.quality.is_some());
    }

    #[test]
    fn un_procede_non_cite_nest_ni_vise_ni_exclu() {
        // Le soudage sous laitier n'est pas dans l'article 1 g) ; l'Annexe B
        // admet d'autres procedes de soudage par fusion, le cas echeant.
        let request = WeldRequest {
            process: Some("72".into()),
            level: Some("C".into()),
            ..fillet_a5()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Caution);
        let finding = reading
            .findings
            .iter()
            .find(|f| f.code == "quality_scope_unknown")
            .unwrap();
        assert!(finding.message.contains("Annexe B"), "{}", finding.message);
    }

    #[test]
    fn la_fissure_de_cratere_nest_admise_a_aucun_niveau() {
        // Tableau 1, 1.2 : non autorisee en D comme en C et B.
        for level in ["B", "C", "D"] {
            let quality = engine()
                .assess_quality(level, JointFamily::Butt, Some(mm("10")), None, None)
                .unwrap();
            assert_eq!(limit(&quality, "104").status, LimitStatus::NotPermitted);
        }
    }

    #[test]
    fn la_penetration_ne_depasse_pas_la_piece() {
        let request = WeldRequest {
            symbol: "single_v".into(),
            size_letter: Some("s".into()),
            size_mm: Some("12".into()),
            level: Some("C".into()),
            thickness_mm: Some("10".into()),
            ..WeldRequest::default()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "penetration_exceeds_thickness"));
    }

    #[test]
    fn une_soudure_par_points_ne_recoit_pas_les_limites() {
        let request = WeldRequest {
            symbol: "resistance_spot".into(),
            level: Some("C".into()),
            ..WeldRequest::default()
        };
        let reading = engine().read_weld(&request).unwrap();
        assert!(reading.quality.is_none());
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "quality_not_applicable"));
    }

    #[test]
    fn le_moteur_ne_recommande_aucun_niveau() {
        // Comme le module geometrique ne propose aucune valeur : le niveau est
        // l'affaire de la norme d'application ou du concepteur.
        let reading = engine()
            .read_weld(&WeldRequest {
                level: Some("B".into()),
                ..fillet_a5()
            })
            .unwrap();
        let text = reading
            .conclusion
            .why
            .iter()
            .filter_map(|s| s.value.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("il ne le choisit pas"));
        for finding in &reading.findings {
            assert!(
                !finding.message.contains("recommand"),
                "{}",
                finding.message
            );
        }
    }

    #[test]
    fn la_reserve_suit_chaque_lecture() {
        // Quatre sources : chacune qui n'est pas verifiee porte sa reserve,
        // et seulement celles-la.
        let reading = engine().read_weld(&fillet_a5()).unwrap();
        assert_eq!(reading.provenance.references.len(), 4);
        let unverified = reading
            .provenance
            .references
            .iter()
            .filter(|r| !r.verification.is_verified())
            .count();
        assert_eq!(reading.conclusion.warnings.len(), unverified);
    }

    #[test]
    fn le_nom_du_symbole_est_celui_de_la_norme() {
        let reading = engine()
            .read_weld(&WeldRequest {
                symbol: "single_v".into(),
                ..WeldRequest::default()
            })
            .unwrap();
        assert_eq!(reading.symbol.name, "soudure bout à bout en V");
        assert_eq!(reading.symbol.number, 2);
    }

    #[test]
    fn une_soudure_evasee_sans_cote_est_une_faute() {
        // ISO 2553:2013, 5.4.4 : toujours cotee, jamais a pleine penetration
        // par defaut.
        let reading = engine()
            .read_weld(&WeldRequest {
                symbol: "flare_v".into(),
                ..WeldRequest::default()
            })
            .unwrap();
        assert_eq!(reading.conclusion.verdict, Verdict::Incompatible);
        assert!(reading.findings.iter().any(|f| f.code == "size_required"));
        assert!(!reading
            .findings
            .iter()
            .any(|f| f.code == "full_penetration"));
        // Cotee en s, elle se lit ; les limites embarquees ne s'y appliquent
        // pas.
        let reading = engine()
            .read_weld(&WeldRequest {
                symbol: "flare_bevel".into(),
                size_letter: Some("s".into()),
                size_mm: Some("3".into()),
                level: Some("C".into()),
                ..WeldRequest::default()
            })
            .unwrap();
        assert!(!reading.findings.iter().any(|f| f.code == "size_required"));
        assert!(reading.quality.is_none());
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "quality_not_applicable"));
    }

    #[test]
    fn la_cote_s_prend_le_nom_que_lui_donne_le_symbole() {
        let reading = engine()
            .read_weld(&WeldRequest {
                symbol: "surfacing".into(),
                size_letter: Some("s".into()),
                size_mm: Some("2".into()),
                ..WeldRequest::default()
            })
            .unwrap();
        assert!(!reading
            .findings
            .iter()
            .any(|f| f.code == "size_not_for_symbol"));
        assert_eq!(reading.size.unwrap().name, "épaisseur du rechargement");
        // Les bords releves n'exigent pas de cotation (5.4.3) : une cote y est
        // etrangere.
        let reading = engine()
            .read_weld(&WeldRequest {
                symbol: "flanged".into(),
                size_letter: Some("s".into()),
                size_mm: Some("2".into()),
                ..WeldRequest::default()
            })
            .unwrap();
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "size_not_for_symbol"));
    }

    #[test]
    fn un_cordon_dangle_des_deux_cotes_se_lit_sans_nom_invente() {
        let reading = engine()
            .read_weld(&WeldRequest {
                side: Side::Both,
                ..fillet_a5()
            })
            .unwrap();
        // La norme admet la soudure d'angle des deux cotes (5.5.1) sans la
        // nommer : pas de reserve, et la designation dit les deux cotes.
        assert!(!reading.findings.iter().any(|f| f.code == "no_double_form"));
        assert!(reading
            .designation
            .contains("a5 soudure d'angle · des deux côtés"));
        // Chaque cote porte sa cote, meme identique.
        assert!(reading
            .findings
            .iter()
            .any(|f| f.code == "both_sides_sizes" && f.message.contains("5.5.1")));
        // L'equivalent porte le nom de la norme.
        assert_eq!(reading.equivalent.unwrap().name, "côté");
    }

    #[test]
    fn un_symbole_supplementaire_hors_des_exemples_est_signale_sans_etre_refuse() {
        let reading = engine()
            .read_weld(&WeldRequest {
                supplementary: vec!["flush".into()],
                ..fillet_a5()
            })
            .unwrap();
        let finding = reading
            .findings
            .iter()
            .find(|f| f.code == "supplementary_unusual")
            .unwrap();
        assert_eq!(finding.severity, Severity::Caution);
        // En systeme B, la surepaisseur a la racine accompagne la soudure sur
        // chant (tableau 4) : pas de reserve.
        let reading = engine()
            .read_weld(&WeldRequest {
                symbol: "edge".into(),
                supplementary: vec!["root_reinforcement".into()],
                ..WeldRequest::default()
            })
            .unwrap();
        assert!(!reading
            .findings
            .iter()
            .any(|f| f.code == "supplementary_unusual"));
    }

    #[test]
    fn les_coefficients_saffichent_comme_la_norme_les_ecrit() {
        assert_eq!(hundredths_label(15), "0.15");
        assert_eq!(hundredths_label(10), "0.1");
        assert_eq!(hundredths_label(5), "0.05");
        assert_eq!(hundredths_label(100), "1");
    }

    #[test]
    fn la_racine_entiere_est_exacte() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(144), 12);
        assert_eq!(integer_sqrt(143), 11);
        assert_eq!(integer_sqrt(50_000_000_000_000), 7_071_067);
    }
}
