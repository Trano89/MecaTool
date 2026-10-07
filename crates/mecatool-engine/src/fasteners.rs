//! Visserie : lecture d'une designation de filetage, et ce qui en decoule.
//!
//! # Ce que ce module fait
//!
//! A partir de `M10`, `M12 × 1,5` ou `M8 8.8`, il rend :
//!
//! * le **pas**, gros ou fin, et ce que la designation aurait du ecrire ;
//! * les **diametres de base** — d2, d1, d3 — par la regle du profil ISO 68-1 ;
//! * la **section resistante** As, par la formule de l'ISO 898-1 ;
//! * le **perçage avant taraudage** par la regle d'atelier D − P, presentee
//!   comme telle ;
//! * les **trous de passage** des trois series de l'ISO 273, et — c'est la
//!   composition — leurs **ecarts reels**, que le moteur ISO 286 calcule a
//!   partir de la classe H12, H13 ou H14 de chaque serie ;
//! * la lecture de la **classe de qualite**, quand elle est donnee, et l'ecrou
//!   qui lui correspond.
//!
//! # La provenance melangee
//!
//! Comme pour les roulements, le resultat croise deux natures de source : les
//! donnees de visserie ne sont pas encore confrontees a leur norme, les ecarts
//! ISO 286 le sont. Les deux voyagent ensemble, et l'ecran les tient separes.
//!
//! # Ce que ce module refuse
//!
//! Il ne calcule **aucun effort admissible**. La classe de qualite donne des
//! valeurs nominales ; les minimums garantis sont tabules dans l'ISO 898-1, qui
//! n'est pas embarquee. Un effort calcule sur la resistance nominale serait un
//! chiffre plausible et faux.
//!
//! # Exactitude
//!
//! `H = (√3/2)·P` n'est pas decimal. Les diametres de base se calculent au
//! femtometre pres en entiers, puis s'annoncent arrondis au micrometre, comme
//! dans les tableaux de l'ISO 724. `π` non plus : la section resistante
//! s'annonce arrondie au dixieme de millimetre carre.

use mecatool_core::{Conclusion, Length, Provenance, ReasoningStep, ToleranceClass, Unit, Verdict};
use mecatool_standards::visserie::{
    BasicProfile, BoltClass, ClearanceTable, MetricThread, StrengthTable, ThreadTable,
};
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::format;
use crate::geometric::{Finding, Severity};
use crate::iso286::{FeatureAnalysis, Iso286Engine};

/// La nature du pas lu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PitchKind {
    Coarse,
    Fine,
    /// Un pas que la table embarquee ne liste pas pour ce diametre.
    NotListed,
}

impl PitchKind {
    pub const fn label_fr(self) -> &'static str {
        match self {
            PitchKind::Coarse => "pas gros",
            PitchKind::Fine => "pas fin",
            PitchKind::NotListed => "pas non listé",
        }
    }
}

/// Une designation lue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadDesignation {
    pub d: Length,
    pub pitch: Length,
    pub pitch_kind: PitchKind,
    /// Vrai quand le pas etait ecrit dans la saisie.
    pub explicit_pitch: bool,
    /// La classe de tolerance de filetage, lue mais non calculee : `6g`.
    pub tolerance_class: Option<String>,
    /// La classe de qualite : `8.8`.
    pub strength_class: Option<String>,
    pub input: String,
}

/// Un diametre de base, calcule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadDimension {
    pub symbol: String,
    pub name: String,
    /// Arrondi au micrometre.
    pub value: Length,
    /// `d2 = 9.026 mm`.
    pub label: String,
    /// `d − 3/4·H`.
    pub formula: String,
}

/// Un trou de passage, et ses ecarts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearanceHole {
    pub series: String,
    pub series_name: String,
    pub diameter: Length,
    pub class: String,
    /// La cote telle qu'elle s'inscrirait sur le plan : `Ø11 H13`.
    pub designation: String,
    /// Les ecarts, calcules par le moteur ISO 286.
    pub tolerance: Option<FeatureAnalysis>,
    /// Pourquoi les ecarts manquent, le cas echeant.
    pub unavailable: Option<String>,
}

/// La classe de qualite, lue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrengthReading {
    pub class: BoltClass,
    /// L'ecrou de classe minimale qui lui correspond.
    pub nut_class: Option<u32>,
    pub explanation: String,
}

/// Le resultat complet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadReport {
    pub designation: ThreadDesignation,
    /// La designation normalisee : `M10`, `M10 × 1.25`.
    pub normalised: String,
    pub thread: MetricThread,
    pub h: ThreadDimension,
    pub dimensions: Vec<ThreadDimension>,
    /// Section resistante, en centiemes de millimetre carre.
    pub stress_area_hundredths_mm2: i64,
    pub stress_area_label: String,
    /// Perçage avant taraudage, par la regle D − P.
    pub tap_drill: Length,
    pub tap_drill_label: String,
    pub clearance_holes: Vec<ClearanceHole>,
    pub strength: Option<StrengthReading>,
    pub findings: Vec<Finding>,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Le moteur du domaine visserie.
#[derive(Debug)]
pub struct FastenerEngine {
    threads: &'static ThreadTable,
    profile: &'static BasicProfile,
    clearance: &'static ClearanceTable,
    strength: &'static StrengthTable,
    iso286: Iso286Engine,
}

/// Femtometres par nanometre : la precision des calculs irrationnels.
const FM_PER_NM: i128 = 1_000_000;
/// Femtometres par micrometre : la precision annoncee.
const FM_PER_UM: i128 = 1_000_000_000;

impl FastenerEngine {
    pub fn new() -> Result<Self> {
        Ok(FastenerEngine {
            threads: ThreadTable::embedded()?,
            profile: BasicProfile::embedded()?,
            clearance: ClearanceTable::embedded()?,
            strength: StrengthTable::embedded()?,
            iso286: Iso286Engine::new()?,
        })
    }

    pub fn thread_table(&self) -> &'static ThreadTable {
        self.threads
    }

    pub fn clearance_table(&self) -> &'static ClearanceTable {
        self.clearance
    }

    pub fn strength_table(&self) -> &'static StrengthTable {
        self.strength
    }

    /// La provenance des seules donnees de visserie.
    pub fn provenance(&self) -> Provenance {
        Provenance::new()
            .with(self.threads.standard().clone())
            .with(self.profile.standard().clone())
            .with(self.clearance.standard().clone())
            .with(self.strength.standard().clone())
    }

    /// Lit puis analyse une designation.
    pub fn read(&self, input: &str) -> Result<ThreadReport> {
        let designation = self.parse(input)?;
        self.analyse(&designation)
    }

    /// Lit une designation : `M10`, `M10x1.25`, `M10 × 1,25-6g`, `M12 8.8`.
    pub fn parse(&self, input: &str) -> Result<ThreadDesignation> {
        let unparsable = |hint: String| EngineError::Unparsable {
            input: input.to_string(),
            hint,
        };
        let mut text = input
            .trim()
            .replace(',', ".")
            .replace(['×', 'X'], "x")
            .replace('*', "x");
        // « M10 x 1.25 » : on recolle le pas a son diametre.
        while text.contains(" x") || text.contains("x ") {
            text = text.replace(" x", "x").replace("x ", "x");
        }
        let mut tokens = text.split_whitespace();
        let Some(head) = tokens.next() else {
            return Err(unparsable(
                "Indiquez un filetage métrique, par exemple « M10 » ou « M12 x 1.5 ».".into(),
            ));
        };

        // « M 10 » : le M est detache.
        let head_owned;
        let head = if head.eq_ignore_ascii_case("m") {
            head_owned = format!("M{}", tokens.next().unwrap_or(""));
            head_owned.as_str()
        } else {
            head
        };

        let Some(body) = head.strip_prefix(['M', 'm']) else {
            return Err(unparsable(
                "Une désignation de filetage métrique ISO commence par M : « M10 ».".into(),
            ));
        };
        let (body, tolerance_class) = match body.split_once('-') {
            Some((body, tolerance)) if !tolerance.is_empty() => (body, Some(tolerance.to_string())),
            Some(_) => {
                return Err(unparsable(
                    "Classe de tolérance vide après le tiret.".into(),
                ))
            }
            None => (body, None),
        };
        let (d_text, pitch_text) = match body.split_once('x') {
            Some((d, p)) => (d, Some(p)),
            None => (body, None),
        };
        let d = Length::parse(d_text, Unit::Millimetre)
            .map_err(|source| unparsable(format!("Diamètre « {d_text} » illisible : {source}.")))?;
        let pitch = match pitch_text {
            Some(p) => Some(
                Length::parse(p, Unit::Millimetre)
                    .map_err(|source| unparsable(format!("Pas « {p} » illisible : {source}.")))?,
            ),
            None => None,
        };

        let mut strength_class = None;
        for token in tokens {
            if BoltClass::read(token).is_some() {
                if strength_class.is_some() {
                    return Err(unparsable("Une seule classe de qualité à la fois.".into()));
                }
                strength_class = Some(token.to_string());
            } else {
                return Err(unparsable(format!(
                    "« {token} » n'est ni un pas, ni une classe de qualité (8.8, 10.9…)."
                )));
            }
        }

        let thread = self.threads.by_diameter(d).ok_or_else(|| {
            unparsable(format!(
                "M{} ne figure pas dans la sélection embarquée : diamètres de premier et de \
                 deuxième choix, de M{} à M{}. Ce n'est pas dire qu'il n'existe pas.",
                format::mm_trimmed(d),
                format::mm_trimmed(self.threads.threads()[0].d),
                format::mm_trimmed(self.threads.threads().last().unwrap().d)
            ))
        })?;

        let (pitch, explicit_pitch) = match pitch {
            Some(p) => (p, true),
            None => (thread.coarse, false),
        };
        if !pitch.is_positive() || pitch >= d {
            return Err(unparsable(
                "Le pas est strictement positif et plus petit que le diamètre.".into(),
            ));
        }
        let pitch_kind = if pitch == thread.coarse {
            PitchKind::Coarse
        } else if thread.fine.contains(&pitch) {
            PitchKind::Fine
        } else {
            PitchKind::NotListed
        };

        Ok(ThreadDesignation {
            d,
            pitch,
            pitch_kind,
            explicit_pitch,
            tolerance_class,
            strength_class,
            input: input.trim().to_string(),
        })
    }

    /// Analyse une designation deja lue.
    pub fn analyse(&self, designation: &ThreadDesignation) -> Result<ThreadReport> {
        let thread = self
            .threads
            .by_diameter(designation.d)
            .ok_or_else(|| EngineError::Unparsable {
                input: designation.input.clone(),
                hint: "Diamètre absent de la sélection embarquée.".into(),
            })?
            .clone();
        let mut findings = Vec::new();
        let d = designation.d;
        let p = designation.pitch;

        let normalised = match designation.pitch_kind {
            PitchKind::Coarse => format!("M{}", format::mm_trimmed(d)),
            _ => format!("M{} × {}", format::mm_trimmed(d), format::mm_trimmed(p)),
        };

        match designation.pitch_kind {
            PitchKind::Coarse if designation.explicit_pitch => findings.push(Finding::new(
                "coarse_pitch_written",
                Severity::Note,
                format!(
                    "{} mm est le pas gros de M{} : il s'omet dans la désignation, qui s'écrit \
                     « {normalised} ».",
                    format::mm_trimmed(p),
                    format::mm_trimmed(d)
                ),
            )),
            PitchKind::NotListed => findings.push(Finding::new(
                "pitch_not_listed",
                Severity::Caution,
                format!(
                    "Le pas de {} mm n'est pas listé pour M{} dans la table embarquée (pas gros \
                     {} ; pas fins {}). Les diamètres de base restent calculables : le profil ne \
                     dépend que du pas.",
                    format::mm_trimmed(p),
                    format::mm_trimmed(d),
                    format::mm_trimmed(thread.coarse),
                    if thread.fine.is_empty() {
                        "aucun".to_string()
                    } else {
                        thread
                            .fine
                            .iter()
                            .map(|f| format::mm_trimmed(*f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ),
            )),
            _ => {}
        }
        if thread.choice == 2 {
            findings.push(Finding::new(
                "second_choice",
                Severity::Note,
                format!(
                    "M{} est un diamètre de deuxième choix : un diamètre de premier choix est à \
                     préférer quand rien ne s'y oppose.",
                    format::mm_trimmed(d)
                ),
            ));
        }
        if let Some(class) = &designation.tolerance_class {
            findings.push(Finding::new(
                "thread_tolerance_not_computed",
                Severity::Note,
                format!(
                    "La classe de tolérance de filetage « {class} » est lue, mais pas calculée : \
                     l'ISO 965 n'est pas embarquée."
                ),
            ));
        }

        // Le profil de base.
        let h = ThreadDimension {
            symbol: "H".into(),
            name: "hauteur du triangle générateur".into(),
            value: round_um(sqrt3_fraction(1, 2, p)),
            label: format!("H = {} mm", format::mm(round_um(sqrt3_fraction(1, 2, p)))),
            formula: self.profile.triangle().to_string(),
        };
        let d_fm = i128::from(d.nanometres()) * FM_PER_NM;
        let mut exact = std::collections::BTreeMap::new();
        let dimensions: Vec<ThreadDimension> = self
            .profile
            .diameters()
            .iter()
            .map(|basic| {
                let [num, den] = basic.h_fraction;
                // k·H = (num/den)·(√3/2)·P
                let value_fm = d_fm - sqrt3_fraction(num, 2 * den, p);
                exact.insert(basic.symbol.clone(), value_fm);
                let value = round_um(value_fm);
                ThreadDimension {
                    symbol: basic.symbol.clone(),
                    name: basic.name.clone(),
                    value,
                    label: format!("{} = {} mm", basic.symbol, format::mm(value)),
                    formula: format!("d − {num}/{den}·H"),
                }
            })
            .collect();

        // La section resistante : As = π/4 · ((d2 + d3)/2)².
        let mean_fm = (exact["d2"] + exact["d3"]) / 2;
        let stress_area_hundredths_mm2 = stress_area_hundredths(mean_fm);
        let stress_area_label = format!(
            "As ≈ {} mm²",
            tenths_label((stress_area_hundredths_mm2 + 5) / 10)
        );

        // La regle d'atelier : D − P, exacte en entiers.
        let tap_drill = d - p;
        let tap_drill_label = format!("Ø{} mm", format::mm_trimmed(tap_drill));

        // La composition : chaque trou de passage passe au moteur ISO 286.
        let clearance_holes = self.clearance_holes(d);

        let strength = match &designation.strength_class {
            Some(text) => Some(self.strength_reading(text, d, &mut findings)?),
            None => None,
        };

        let mut provenance = self.provenance();
        for hole in &clearance_holes {
            if let Some(tolerance) = &hole.tolerance {
                provenance.merge(&tolerance.provenance);
            }
        }

        let medium = clearance_holes.iter().find(|h| h.series == "medium");
        let mut conclusion = match findings.iter().find(|f| f.severity == Severity::Error) {
            Some(error) => Conclusion::new(Verdict::Incompatible, error.message.clone()),
            None => Conclusion::new(
                Verdict::Caution,
                format!(
                    "{normalised} — {} de {} mm. Trou de passage moyen {} ; perçage avant \
                     taraudage {} par la règle d'atelier D − P.",
                    designation.pitch_kind.label_fr(),
                    format::mm_trimmed(p),
                    medium.map(|h| h.designation.as_str()).unwrap_or("—"),
                    tap_drill_label
                ),
            ),
        };
        conclusion.why =
            self.reasoning(designation, &h, &dimensions, &stress_area_label, tap_drill);
        conclusion.warnings = provenance.warnings_fr();

        Ok(ThreadReport {
            designation: designation.clone(),
            normalised,
            thread,
            h,
            dimensions,
            stress_area_hundredths_mm2,
            stress_area_label,
            tap_drill,
            tap_drill_label,
            clearance_holes,
            strength,
            findings,
            conclusion,
            provenance,
        })
    }

    fn clearance_holes(&self, d: Length) -> Vec<ClearanceHole> {
        let Some(row) = self.clearance.by_diameter(d) else {
            return Vec::new();
        };
        self.clearance
            .series()
            .iter()
            .filter_map(|series| {
                let diameter = row.by_series(&series.id)?;
                let designation = format!("Ø{} {}", format::mm_trimmed(diameter), series.class);
                let computed = ToleranceClass::parse(&series.class)
                    .map_err(EngineError::from)
                    .and_then(|class| self.iso286.feature(diameter, class));
                let (tolerance, unavailable) = match computed {
                    Ok(analysis) => (Some(analysis), None),
                    // Un refus du moteur ISO 286 se transmet tel quel : on ne
                    // remplace pas des ecarts manquants par une estimation.
                    Err(error) => (None, Some(error.to_string())),
                };
                Some(ClearanceHole {
                    series: series.id.clone(),
                    series_name: series.name.clone(),
                    diameter,
                    class: series.class.clone(),
                    designation,
                    tolerance,
                    unavailable,
                })
            })
            .collect()
    }

    fn strength_reading(
        &self,
        text: &str,
        d: Length,
        findings: &mut Vec<Finding>,
    ) -> Result<StrengthReading> {
        let class = self
            .strength
            .bolt_class(text)
            .ok_or_else(|| EngineError::Unparsable {
                input: text.to_string(),
                hint: format!(
                    "Classe de qualité inconnue. Classes embarquées : {}.",
                    self.strength
                        .bolt_classes()
                        .iter()
                        .map(|c| c.class.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?
            .clone();
        if let Some(max) = class.max_d {
            if d > Length::from_millimetres(i64::from(max)) {
                findings.push(Finding::new(
                    "class_size_limit",
                    Severity::Error,
                    format!(
                        "La classe {} n'est définie que jusqu'à M{max}.",
                        class.class
                    ),
                ));
            }
        }
        let nut_class = self.strength.nut_for(&class);
        let explanation = format!(
            "{} : Rm nominale = {} × 100 = {} MPa ; Re nominale = {} × {} × 10 = {} MPa. Valeurs \
             nominales : les minimums garantis sont tabulés dans l'ISO 898-1, non embarquée.",
            class.class, class.first, class.tensile_mpa, class.first, class.second, class.yield_mpa
        );
        Ok(StrengthReading {
            class,
            nut_class,
            explanation,
        })
    }

    fn reasoning(
        &self,
        designation: &ThreadDesignation,
        h: &ThreadDimension,
        dimensions: &[ThreadDimension],
        stress_area: &str,
        tap_drill: Length,
    ) -> Vec<ReasoningStep> {
        let mut steps = vec![
            ReasoningStep::new("Pas")
                .with_expression(designation.pitch_kind.label_fr())
                .with_value(format!("P = {} mm", format::mm_trimmed(designation.pitch))),
            ReasoningStep::new("Triangle générateur")
                .with_expression(h.formula.clone())
                .with_value(format!(
                    "{} — arrondi au micromètre, comme les tableaux de l'ISO 724",
                    h.label
                )),
        ];
        for dimension in dimensions {
            steps.push(
                ReasoningStep::new(capitalise(&dimension.name))
                    .with_expression(dimension.formula.clone())
                    .with_value(dimension.label.clone()),
            );
        }
        steps.push(
            ReasoningStep::new("Section résistante")
                .with_expression("As = π/4 · ((d2 + d3)/2)²")
                .with_value(format!("{stress_area}, arrondie au dixième")),
        );
        steps.push(
            ReasoningStep::new("Perçage avant taraudage")
                .with_expression("D − P")
                .with_value(format!(
                    "{} mm — règle d'atelier, sans caractère normatif. Le foret retenu est le \
                     plus proche disponible.",
                    format::mm_trimmed(tap_drill)
                )),
        );
        steps.push(
            ReasoningStep::new("Trous de passage")
                .with_expression("ISO 273, puis ISO 286")
                .with_value(
                    "Le diamètre et la classe viennent de l'ISO 273 ; les écarts de la classe, du \
                     moteur ISO 286.",
                ),
        );
        steps
    }
}

/// `(num/den)·√3·P`, en femtometres, par defaut.
///
/// Le calcul reste en entiers : `√(3·(num·P)²) / den`, la racine etant prise
/// sur des femtometres pour que l'arrondi final au micrometre soit sur.
fn sqrt3_fraction(num: i64, den: i64, pitch: Length) -> i128 {
    let scaled = i128::from(num) * i128::from(pitch.nanometres()) * FM_PER_NM;
    integer_sqrt(3 * scaled * scaled) / i128::from(den)
}

/// Arrondit des femtometres au micrometre le plus proche.
fn round_um(fm: i128) -> Length {
    let um = (fm + FM_PER_UM / 2).div_euclid(FM_PER_UM);
    Length::from_micrometres(um as i64)
}

/// `π/4 · m²`, en centiemes de millimetre carre, au plus proche.
///
/// `π` est pris a vingt decimales : l'erreur relative, de l'ordre de 1e-20, ne
/// peut pas deplacer un centieme de millimetre carre.
fn stress_area_hundredths(mean_fm: i128) -> i64 {
    const PI_E20: i128 = 314_159_265_358_979_323_846;
    // On redescend au picometre avant d'elever au carre, pour rester dans un
    // i128 : (6.4e10 pm)² × 3.1e20 ≈ 1.3e42 deborderait en femtometres.
    let mean_pm = mean_fm / 1_000;
    let square_pm2 = mean_pm * mean_pm; // pm², 1 mm² = 1e18 pm²
                                        // centiemes de mm² = π/4 · pm² · 100 / 1e18
    let numerator = square_pm2 / 1_000_000 * PI_E20; // pm²·1e-6 × π·1e20
                                                     // = π · pm² · 1e14 ; on veut π · pm² · 100 / (4 · 1e18) = π · pm² · 1e14 / 4e30
    let denominator: i128 = 4 * 10_i128.pow(30);
    ((numerator + denominator / 2) / denominator) as i64
}

fn tenths_label(tenths: i64) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

fn integer_sqrt(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }
    let mut x = (value as f64).sqrt() as i128;
    while x * x > value {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= value {
        x += 1;
    }
    x
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

    fn engine() -> FastenerEngine {
        FastenerEngine::new().expect("les donnees de visserie doivent charger")
    }

    fn mm(text: &str) -> Length {
        Length::parse(text, Unit::Millimetre).unwrap()
    }

    fn dimension<'a>(report: &'a ThreadReport, symbol: &str) -> &'a ThreadDimension {
        report
            .dimensions
            .iter()
            .find(|d| d.symbol == symbol)
            .unwrap()
    }

    #[test]
    fn lit_les_ecritures_courantes() {
        let e = engine();
        for input in ["M10", "m10", "M 10", "M10x1.5", "M10 x 1,5", "M10×1.5"] {
            let designation = e
                .parse(input)
                .unwrap_or_else(|err| panic!("{input} : {err}"));
            assert_eq!(designation.d, mm("10"), "{input}");
            assert_eq!(designation.pitch, mm("1.5"), "{input}");
            assert_eq!(designation.pitch_kind, PitchKind::Coarse, "{input}");
        }
        let fine = e.parse("M12x1.5-6g 10.9").unwrap();
        assert_eq!(fine.pitch_kind, PitchKind::Fine);
        assert_eq!(fine.tolerance_class.as_deref(), Some("6g"));
        assert_eq!(fine.strength_class.as_deref(), Some("10.9"));
    }

    #[test]
    fn les_diametres_de_base_retrouvent_les_tableaux_iso_724() {
        // Valeurs de reference d'usage : M10 → d2 9,026 et d1 8,376 ;
        // M6 → 5,350 et 4,917 ; M8 → 7,188 et 6,647.
        for (input, d2, d1) in [
            ("M10", "9.026", "8.376"),
            ("M6", "5.35", "4.917"),
            ("M8", "7.188", "6.647"),
            ("M12", "10.863", "10.106"),
        ] {
            let report = engine().read(input).unwrap();
            assert_eq!(dimension(&report, "d2").value, mm(d2), "{input} d2");
            assert_eq!(dimension(&report, "d1").value, mm(d1), "{input} d1");
        }
    }

    #[test]
    fn la_section_resistante_retrouve_les_valeurs_dusage() {
        for (input, area) in [
            ("M6", "As ≈ 20.1 mm²"),
            ("M8", "As ≈ 36.6 mm²"),
            ("M10", "As ≈ 58.0 mm²"),
            ("M12", "As ≈ 84.3 mm²"),
            ("M16", "As ≈ 156.7 mm²"),
        ] {
            assert_eq!(
                engine().read(input).unwrap().stress_area_label,
                area,
                "{input}"
            );
        }
    }

    #[test]
    fn les_diametres_sont_ordonnes() {
        let report = engine().read("M20").unwrap();
        let d3 = dimension(&report, "d3").value;
        let d1 = dimension(&report, "d1").value;
        let d2 = dimension(&report, "d2").value;
        assert!(d3 < d1 && d1 < d2 && d2 < mm("20"));
    }

    #[test]
    fn le_percage_avant_taraudage_est_exact_et_dit_dou_il_vient() {
        let report = engine().read("M10").unwrap();
        assert_eq!(report.tap_drill, mm("8.5"));
        // M8 : 6,75 — exact. Le foret de 6,8 est un choix d'atelier, que
        // MecaTool ne fait pas a la place de l'utilisateur.
        assert_eq!(engine().read("M8").unwrap().tap_drill, mm("6.75"));
        let step = report
            .conclusion
            .why
            .iter()
            .find(|s| s.label == "Perçage avant taraudage")
            .unwrap();
        assert!(step
            .value
            .as_deref()
            .unwrap()
            .contains("sans caractère normatif"));
    }

    #[test]
    fn les_trous_de_passage_vont_jusquaux_ecarts_reels() {
        // La composition : ISO 273 donne Ø11 H13, ISO 286 en calcule les ecarts.
        let report = engine().read("M10").unwrap();
        assert_eq!(report.clearance_holes.len(), 3);
        let medium = report
            .clearance_holes
            .iter()
            .find(|h| h.series == "medium")
            .unwrap();
        assert_eq!(medium.designation, "Ø11 H13");
        let tolerance = medium.tolerance.as_ref().unwrap();
        // H : EI = 0 ; IT13 sur 10..18 vaut 270 µm.
        assert_eq!(tolerance.tolerance.deviations.lower(), Length::ZERO);
        assert_eq!(
            tolerance.tolerance.deviations.upper(),
            Length::from_micrometres(270)
        );
    }

    #[test]
    fn la_provenance_porte_les_deux_natures_de_source() {
        let report = engine().read("M10").unwrap();
        let states: Vec<bool> = report
            .provenance
            .references
            .iter()
            .map(|r| r.verification.is_verified())
            .collect();
        assert!(states.contains(&true), "l'ISO 286 doit figurer, verifiee");
        assert!(
            states.contains(&false),
            "la visserie doit figurer, non verifiee"
        );
        assert_eq!(report.conclusion.verdict, Verdict::Caution);
    }

    #[test]
    fn le_pas_gros_ecrit_est_signale() {
        let report = engine().read("M10x1.5").unwrap();
        assert_eq!(report.normalised, "M10");
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "coarse_pitch_written"));
    }

    #[test]
    fn un_pas_non_liste_reste_calculable_mais_signale() {
        let report = engine().read("M10x0.5").unwrap();
        assert_eq!(report.designation.pitch_kind, PitchKind::NotListed);
        assert_eq!(report.normalised, "M10 × 0.5");
        assert!(report.findings.iter().any(|f| f.code == "pitch_not_listed"));
    }

    #[test]
    fn un_diametre_absent_nest_pas_dit_inexistant() {
        let err = engine().read("M11").unwrap_err().to_string();
        assert!(err.contains("sélection embarquée"), "{err}");
        assert!(err.contains("n'est pas dire qu'il n'existe pas"), "{err}");
    }

    #[test]
    fn la_classe_de_qualite_se_lit_et_trouve_son_ecrou() {
        let report = engine().read("M12 8.8").unwrap();
        let strength = report.strength.unwrap();
        assert_eq!(strength.class.tensile_mpa, 800);
        assert_eq!(strength.class.yield_mpa, 640);
        assert_eq!(strength.nut_class, Some(8));
        assert!(strength.explanation.contains("nominales"));
    }

    #[test]
    fn la_classe_9_8_au_dela_de_m16_est_une_faute() {
        let report = engine().read("M20 9.8").unwrap();
        assert_eq!(report.conclusion.verdict, Verdict::Incompatible);
        assert!(report.findings.iter().any(|f| f.code == "class_size_limit"));
        assert!(engine().read("M16 9.8").unwrap().findings.is_empty());
    }

    #[test]
    fn aucun_effort_nest_calcule() {
        // Un effort sur la resistance nominale serait plausible et faux : les
        // minimums garantis de l'ISO 898-1 ne sont pas embarques.
        let report = engine().read("M10 8.8").unwrap();
        let text = serde_json::to_string(&report).unwrap();
        assert!(!text.contains(" kN"));
        assert!(!text.contains("charge de rupture"));
    }

    #[test]
    fn les_saisies_fautives_disent_quoi_faire() {
        let e = engine();
        for (input, expected) in [
            ("", "M10"),
            ("10", "commence par M"),
            ("Mabc", "illisible"),
            ("M10 bidule", "ni un pas"),
            ("M10 8.8 10.9", "Une seule classe"),
            ("M10x12", "plus petit que le diamètre"),
            ("M10 7.7", "Classe de qualité inconnue"),
        ] {
            let err = e
                .read(input)
                .err()
                .unwrap_or_else(|| panic!("{input:?} aurait dû échouer"))
                .to_string();
            assert!(err.contains(expected), "{input:?} : {err}");
        }
    }

    #[test]
    fn la_classe_de_tolerance_de_filetage_nest_pas_inventee() {
        let report = engine().read("M10-6H").unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "thread_tolerance_not_computed"));
    }

    #[test]
    fn un_diametre_de_deuxieme_choix_est_signale() {
        let report = engine().read("M14").unwrap();
        assert!(report.findings.iter().any(|f| f.code == "second_choice"));
    }
}
