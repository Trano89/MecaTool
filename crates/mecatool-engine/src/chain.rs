//! Chaines de cotes.
//!
//! Une chaine de cotes calcule ce que devient une dimension **resultante**
//! lorsqu'elle depend de plusieurs cotes tolerancees. C'est le module ou
//! l'arithmetique entiere paie le plus : additionner cent cotes en flottants
//! ferait deriver le resultat, ici il tombe juste.
//!
//! # Le sens compte
//!
//! Une chaine n'est pas une somme. Chaque maillon **augmente** ou **diminue** la
//! cote resultante :
//!
//! ```text
//!   jeu = A - B - C
//!         ^   ^   ^
//!         |   +---+-- maillons diminuants
//!         +---------- maillon augmentant
//! ```
//!
//! Pour le nominal, le signe s'applique directement. Pour les **limites**, il
//! s'inverse : la resultante est maximale quand les maillons diminuants sont a
//! leur **minimum**.
//!
//! ```text
//!   max = somme(max des augmentants) - somme(min des diminuants)
//!   min = somme(min des augmentants) - somme(max des diminuants)
//! ```
//!
//! # L'identite qui structure tout
//!
//! ```text
//!   tolerance resultante = max - min = somme de TOUTES les tolerances
//! ```
//!
//! Le sens n'y change rien : un maillon diminuant apporte autant d'incertitude
//! qu'un maillon augmentant. C'est le resultat le plus contre-intuitif de la
//! cotation, et le plus utile — il dit qu'on ne gagne rien a reorganiser une
//! chaine, seulement a resserrer ses maillons ou a en supprimer.

use serde::{Deserialize, Serialize};

use mecatool_core::{Conclusion, Deviations, Length, LimitsOfSize, ReasoningStep, Unit, Verdict};

use crate::error::{EngineError, Result};

/// Le sens d'un maillon dans la chaine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkDirection {
    /// Une augmentation de ce maillon augmente la cote resultante.
    Increasing,
    /// Une augmentation de ce maillon diminue la cote resultante.
    Decreasing,
}

impl LinkDirection {
    pub const fn sign(self) -> i64 {
        match self {
            LinkDirection::Increasing => 1,
            LinkDirection::Decreasing => -1,
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            LinkDirection::Increasing => "+",
            LinkDirection::Decreasing => "\u{2212}",
        }
    }

    pub const fn label_fr(self) -> &'static str {
        match self {
            LinkDirection::Increasing => "augmentant",
            LinkDirection::Decreasing => "diminuant",
        }
    }
}

/// Un maillon de la chaine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// Repere porte sur le plan : `A`, `B`, `L1`...
    pub label: String,
    pub nominal: Length,
    pub deviations: Deviations,
    pub direction: LinkDirection,
}

impl Link {
    pub fn new(
        label: impl Into<String>,
        nominal: Length,
        deviations: Deviations,
        direction: LinkDirection,
    ) -> Result<Self> {
        let label = label.into();
        if !nominal.is_positive() {
            return Err(EngineError::ContradictoryRequirement {
                detail: format!(
                    "le maillon {label} a une cote nominale non positive ({nominal}) ; \
                     le sens s'exprime par la direction, pas par le signe du nominal"
                ),
            });
        }
        Ok(Link {
            label,
            nominal,
            deviations,
            direction,
        })
    }

    /// Largeur de la tolerance du maillon.
    pub fn tolerance(&self) -> Length {
        self.deviations.width()
    }

    /// Dimensions limites du maillon lui-meme.
    pub fn limits(&self) -> LimitsOfSize {
        LimitsOfSize::new(
            self.nominal + self.deviations.lower(),
            self.nominal + self.deviations.upper(),
        )
        .expect("les ecarts d'un maillon sont ordonnes a la construction")
    }

    /// Ecriture du maillon telle qu'elle figurerait sur un plan.
    pub fn designation(&self) -> String {
        let nominal = trim(self.nominal.to_decimal_string(Unit::Millimetre, 4));
        let (lower, upper) = (self.deviations.lower(), self.deviations.upper());
        if lower == -upper {
            format!(
                "{nominal} \u{b1} {}",
                trim(upper.to_decimal_string(Unit::Millimetre, 4))
            )
        } else {
            format!("{nominal} {} / {}", signed(upper), signed(lower))
        }
    }
}

/// La part d'un maillon dans l'incertitude totale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    pub link: Link,
    pub tolerance: Length,
    /// Part de la tolerance resultante, en millemes. Entier, donc exact.
    pub share_per_mille: u32,
    pub share_label: String,
    pub designation: String,
}

/// L'estimation statistique, quand elle est demandee.
///
/// Elle repose sur des hypotheses que la geometrie ne garantit pas : elles sont
/// donc enoncees avec le resultat, jamais separement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticalEstimate {
    pub method: String,
    /// Tolerance resultante estimee, arrondie au nanometre.
    pub tolerance: Length,
    /// Part de la tolerance au pire des cas, en millemes.
    pub share_of_worst_case_per_mille: u32,
    pub summary: String,
    /// Ce que l'estimation suppose, et que le calcul ne verifie pas.
    pub assumptions: Vec<String>,
}

/// Le resultat complet d'une chaine de cotes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainAnalysis {
    pub contributions: Vec<Contribution>,
    /// Cote resultante nominale.
    pub nominal: Length,
    /// Limites au pire des cas.
    pub limits: LimitsOfSize,
    /// Tolerance resultante au pire des cas, egale a la somme des tolerances.
    pub tolerance: Length,
    /// Ecarts de la resultante par rapport a son nominal.
    pub deviations: Deviations,
    pub statistical: Option<StatisticalEstimate>,
    pub steps: Vec<ReasoningStep>,
    /// Le maillon qui pese le plus lourd.
    pub dominant: Option<String>,
}

impl ChainAnalysis {
    /// Ecriture de la resultante telle qu'elle figurerait sur un plan.
    pub fn designation(&self) -> String {
        let nominal = trim(self.nominal.to_decimal_string(Unit::Millimetre, 4));
        let (lower, upper) = (self.deviations.lower(), self.deviations.upper());
        if lower == -upper {
            format!(
                "{nominal} \u{b1} {}",
                trim(upper.to_decimal_string(Unit::Millimetre, 4))
            )
        } else {
            format!("{nominal} {} / {}", signed(upper), signed(lower))
        }
    }
}

/// Calcule une chaine de cotes au pire des cas.
///
/// L'estimation statistique est jointe lorsque `statistical` est vrai. Elle
/// n'est jamais calculee en silence : c'est une hypothese sur la fabrication,
/// pas une propriete de la geometrie.
pub fn analyse_chain(links: &[Link], statistical: bool) -> Result<ChainAnalysis> {
    if links.is_empty() {
        return Err(EngineError::Unparsable {
            input: String::new(),
            hint: "Une chaîne de cotes demande au moins un maillon, par exemple \
                   « A = 20 ±0.1 »."
                .to_string(),
        });
    }

    let mut labels = Vec::with_capacity(links.len());
    for link in links {
        if labels.contains(&link.label) {
            return Err(EngineError::Ambiguous {
                input: link.label.clone(),
                question: format!(
                    "Deux maillons portent le repère « {} ». Lequel désigne quoi ?",
                    link.label
                ),
            });
        }
        labels.push(link.label.clone());
    }

    // Nominal : le signe s'applique directement.
    let mut nominal = Length::ZERO;
    // Limites : le signe s'inverse. La resultante est maximale quand les
    // maillons diminuants sont a leur minimum.
    let mut max = Length::ZERO;
    let mut min = Length::ZERO;
    let mut total_tolerance = Length::ZERO;

    for link in links {
        let limits = link.limits();
        match link.direction {
            LinkDirection::Increasing => {
                nominal += link.nominal;
                max += limits.max();
                min += limits.min();
            }
            LinkDirection::Decreasing => {
                nominal -= link.nominal;
                max -= limits.min();
                min -= limits.max();
            }
        }
        total_tolerance += link.tolerance();
    }

    let limits = LimitsOfSize::new(min, max)?;
    let deviations = Deviations::new(min - nominal, max - nominal)?;

    let contributions = links
        .iter()
        .map(|link| {
            let tolerance = link.tolerance();
            let share = per_mille(tolerance, total_tolerance);
            Contribution {
                designation: link.designation(),
                tolerance,
                share_per_mille: share,
                share_label: format_per_mille(share),
                link: link.clone(),
            }
        })
        .collect::<Vec<_>>();

    let dominant = contributions
        .iter()
        .max_by_key(|c| c.tolerance.nanometres())
        .map(|c| c.link.label.clone());

    let statistical = statistical.then(|| estimate_statistically(links, total_tolerance));

    let steps = reasoning(links, nominal, &limits, total_tolerance);

    Ok(ChainAnalysis {
        contributions,
        nominal,
        limits,
        tolerance: total_tolerance,
        deviations,
        statistical,
        steps,
        dominant,
    })
}

/// Confronte la resultante a une exigence fonctionnelle.
pub fn verify_chain(
    analysis: &ChainAnalysis,
    minimum: Option<Length>,
    maximum: Option<Length>,
) -> Conclusion {
    let obtained = format!(
        "{} à {} mm",
        analysis.limits.min().to_decimal_string(Unit::Millimetre, 3),
        analysis.limits.max().to_decimal_string(Unit::Millimetre, 3)
    );

    if minimum.is_none() && maximum.is_none() {
        return Conclusion::insufficient_data(format!(
            "La chaîne donne une résultante de {obtained}. Indiquez les limites \
             fonctionnelles attendues pour que MecaTool puisse conclure."
        ));
    }

    let mut steps = vec![ReasoningStep::new("Résultante calculée")
        .with_expression("pire des cas")
        .with_value(obtained.clone())];

    let respects_min = minimum.is_none_or(|m| analysis.limits.min() >= m);
    let respects_max = maximum.is_none_or(|m| analysis.limits.max() <= m);

    if let Some(m) = minimum {
        steps.push(
            ReasoningStep::new("Minimum fonctionnel")
                .with_expression("résultante minimale − minimum exigé")
                .with_value(format!(
                    "{} − {} = {}",
                    analysis.limits.min().to_decimal_string(Unit::Millimetre, 3),
                    m.to_decimal_string(Unit::Millimetre, 3),
                    signed(analysis.limits.min() - m)
                )),
        );
    }
    if let Some(m) = maximum {
        steps.push(
            ReasoningStep::new("Maximum fonctionnel")
                .with_expression("maximum exigé − résultante maximale")
                .with_value(format!(
                    "{} − {} = {}",
                    m.to_decimal_string(Unit::Millimetre, 3),
                    analysis.limits.max().to_decimal_string(Unit::Millimetre, 3),
                    signed(m - analysis.limits.max())
                )),
        );
    }

    let overlaps = minimum.is_none_or(|m| analysis.limits.max() >= m)
        && maximum.is_none_or(|m| analysis.limits.min() <= m);

    let (verdict, detail) = if respects_min && respects_max {
        (
            Verdict::Compatible,
            format!(
                "La résultante ({obtained}) reste entièrement dans les limites exigées. \
                 Toute pièce conforme au plan conviendra."
            ),
        )
    } else if overlaps {
        (
            Verdict::Caution,
            format!(
                "La résultante ({obtained}) déborde des limites exigées. Certaines pièces \
                 conformes au plan seront hors spécification."
            ),
        )
    } else {
        (
            Verdict::Incompatible,
            format!(
                "La résultante ({obtained}) ne rencontre jamais les limites exigées. \
                 Aucune pièce conforme au plan ne conviendra."
            ),
        )
    };

    Conclusion::new(verdict, detail).with_steps(steps)
}

/// Estimation statistique par la racine de la somme des carres.
fn estimate_statistically(links: &[Link], worst_case: Length) -> StatisticalEstimate {
    // Somme des carres en i128 : une tolerance de 1 mm vaut 1e6 nm, son carre
    // 1e12, et cent maillons restent tres loin de la capacite.
    let sum_of_squares: i128 = links
        .iter()
        .map(|link| {
            let t = i128::from(link.tolerance().nanometres());
            t * t
        })
        .sum();

    let tolerance = Length::from_nanometres(integer_sqrt(sum_of_squares) as i64);
    let share = per_mille(tolerance, worst_case);

    StatisticalEstimate {
        method: "Racine de la somme des carrés (RSS)".to_string(),
        tolerance,
        share_of_worst_case_per_mille: share,
        summary: format!(
            "Estimation : {} au lieu de {} au pire des cas, soit {}.",
            crate::format::mm(tolerance),
            crate::format::mm(worst_case),
            format_per_mille(share)
        ),
        // Ces hypotheses ne sont pas des precautions de style : aucune n'est
        // verifiee par le calcul, et chacune peut etre fausse en atelier.
        assumptions: vec![
            "Les cotes sont indépendantes les unes des autres.".to_string(),
            "Chaque cote est centrée sur son nominal.".to_string(),
            "Chaque cote suit une distribution normale dont la tolérance couvre ± 3 écarts-types."
                .to_string(),
            "Le procédé est stable et sous contrôle.".to_string(),
            "Aucune de ces hypothèses n'est vérifiée par MecaTool : sans relevé de \
             fabrication, l'estimation reste une hypothèse."
                .to_string(),
        ],
    }
}

fn reasoning(
    links: &[Link],
    nominal: Length,
    limits: &LimitsOfSize,
    tolerance: Length,
) -> Vec<ReasoningStep> {
    let expression = links
        .iter()
        .enumerate()
        .map(|(index, link)| {
            let sign = match (index, link.direction) {
                (0, LinkDirection::Increasing) => String::new(),
                (_, direction) => format!("{} ", direction.symbol()),
            };
            format!("{sign}{}", link.label)
        })
        .collect::<Vec<_>>()
        .join(" ");

    vec![
        ReasoningStep::new("Chaîne")
            .with_expression("les maillons diminuants portent un signe moins")
            .with_value(expression),
        ReasoningStep::new("Cote résultante nominale")
            .with_expression("somme signée des nominaux")
            .with_value(format!(
                "{} mm",
                nominal.to_decimal_string(Unit::Millimetre, 3)
            )),
        ReasoningStep::new("Résultante maximale")
            .with_expression("max des augmentants − min des diminuants")
            .with_value(format!(
                "{} mm",
                limits.max().to_decimal_string(Unit::Millimetre, 3)
            )),
        ReasoningStep::new("Résultante minimale")
            .with_expression("min des augmentants − max des diminuants")
            .with_value(format!(
                "{} mm",
                limits.min().to_decimal_string(Unit::Millimetre, 3)
            )),
        ReasoningStep::new("Tolérance résultante")
            .with_expression("somme de toutes les tolérances, quel que soit leur sens")
            .with_value(format!(
                "{} mm",
                tolerance.to_decimal_string(Unit::Millimetre, 3)
            )),
    ]
}

/// Une barre du graphique de contribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContributionBar {
    pub label: String,
    pub designation: String,
    pub direction: LinkDirection,
    pub y: f64,
    pub height: f64,
    /// Largeur proportionnelle a la part du maillon.
    pub width: f64,
    pub share_label: String,
    pub tolerance_label: String,
    /// Vrai pour le maillon qui pese le plus lourd.
    pub dominant: bool,
}

/// La contribution de chaque maillon, prete a tracer.
///
/// Comme pour les zones de tolerance, la geometrie est calculee ici : un
/// graphique qui contredirait les valeurs serait pire qu'absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContributionChart {
    pub width: f64,
    pub height: f64,
    /// Abscisse ou commencent les barres, apres la colonne des reperes.
    pub bar_origin: f64,
    pub bars: Vec<ContributionBar>,
    pub caption: String,
}

/// Largeur reservee aux reperes, a gauche des barres.
const LABEL_GUTTER: f64 = 92.0;
/// Largeur reservee aux valeurs, a droite des barres.
const VALUE_GUTTER: f64 = 132.0;
/// Hauteur d'une barre et de son interligne.
const ROW_HEIGHT: f64 = 26.0;

/// Construit le graphique de contribution d'une chaine.
///
/// Les maillons gardent l'ordre de la chaine. Trier par taille rendrait le
/// dominant plus visible, mais on ne pourrait plus relier une barre a sa
/// position dans l'assemblage — et c'est la que se prend la decision.
pub fn contribution_chart(analysis: &ChainAnalysis, width: f64) -> ContributionChart {
    let usable = (width - LABEL_GUTTER - VALUE_GUTTER).max(1.0);
    let widest = analysis
        .contributions
        .iter()
        .map(|c| c.share_per_mille)
        .max()
        .unwrap_or(1)
        .max(1);

    let bars = analysis
        .contributions
        .iter()
        .enumerate()
        .map(|(index, contribution)| ContributionBar {
            label: contribution.link.label.clone(),
            designation: contribution.designation.clone(),
            direction: contribution.link.direction,
            y: index as f64 * ROW_HEIGHT + 4.0,
            height: ROW_HEIGHT - 10.0,
            // Normalise sur le plus gros maillon : sur une chaine de vingt
            // cotes, des barres normalisees sur 100 % seraient toutes invisibles.
            width: usable * f64::from(contribution.share_per_mille) / f64::from(widest),
            share_label: contribution.share_label.clone(),
            tolerance_label: format!(
                "{} mm",
                trim(
                    contribution
                        .tolerance
                        .to_decimal_string(Unit::Millimetre, 4)
                )
            ),
            dominant: analysis.dominant.as_deref() == Some(contribution.link.label.as_str()),
        })
        .collect::<Vec<_>>();

    ContributionChart {
        width,
        height: bars.len() as f64 * ROW_HEIGHT + 8.0,
        bar_origin: LABEL_GUTTER,
        bars,
        caption: format!(
            "Part de chaque maillon dans la tolérance résultante de {} mm. \
             Les barres sont proportionnelles entre elles, normalisées sur le maillon \
             le plus large.",
            trim(analysis.tolerance.to_decimal_string(Unit::Millimetre, 4))
        ),
    }
}

/// Part en millemes, arrondie au plus proche.
fn per_mille(part: Length, total: Length) -> u32 {
    if total.nanometres() <= 0 {
        return 0;
    }
    let numerator = i128::from(part.nanometres()) * 1000 + i128::from(total.nanometres()) / 2;
    (numerator / i128::from(total.nanometres())).clamp(0, 1000) as u32
}

fn format_per_mille(value: u32) -> String {
    format!("{},{} %", value / 10, value % 10)
}

/// Racine carree entiere, par la methode de Newton.
///
/// Ecrite plutot qu'empruntee a la bibliotheque standard pour que le calcul
/// reste deterministe et independant de la version du compilateur.
fn integer_sqrt(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }
    let mut guess = value;
    let mut next = (guess + 1) / 2;
    while next < guess {
        guess = next;
        next = (guess + value / guess) / 2;
    }
    guess
}

fn signed(value: Length) -> String {
    let rendered = trim(value.abs().to_decimal_string(Unit::Millimetre, 4));
    if value.is_negative() {
        format!("\u{2212}{rendered}")
    } else {
        format!("+{rendered}")
    }
}

fn trim(rendered: String) -> String {
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

    fn mm(value: &str) -> Length {
        Length::parse(value, Unit::Millimetre).unwrap()
    }

    fn link(label: &str, nominal: &str, tolerance: &str, direction: LinkDirection) -> Link {
        Link::new(
            label,
            mm(nominal),
            Deviations::symmetric(mm(tolerance)),
            direction,
        )
        .unwrap()
    }

    /// Le scénario du cahier des charges.
    #[test]
    fn la_chaine_du_cahier_des_charges() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.05", LinkDirection::Increasing),
            link("C", "5", "0.02", LinkDirection::Increasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();

        assert_eq!(analysis.nominal, mm("35"));
        assert_eq!(analysis.limits.min(), mm("34.83"));
        assert_eq!(analysis.limits.max(), mm("35.17"));
        // 0,2 + 0,1 + 0,04 = 0,34
        assert_eq!(analysis.tolerance, mm("0.34"));
        assert_eq!(analysis.designation(), "35 ± 0.17");
    }

    /// L'identité qui structure le module : le sens ne change rien à la
    /// tolérance résultante.
    #[test]
    fn la_tolerance_resultante_vaut_la_somme_de_toutes_les_tolerances() {
        for direction in [LinkDirection::Increasing, LinkDirection::Decreasing] {
            let links = [
                link("A", "50", "0.1", LinkDirection::Increasing),
                link("B", "20", "0.05", direction),
                link("C", "10", "0.02", direction),
            ];
            let analysis = analyse_chain(&links, false).unwrap();
            let sum: Length = links.iter().map(|l| l.tolerance()).sum();
            assert_eq!(
                analysis.tolerance, sum,
                "le sens des maillons a changé la tolérance résultante"
            );
            assert_eq!(analysis.limits.width(), sum);
        }
    }

    #[test]
    fn un_maillon_diminuant_inverse_ses_limites() {
        // jeu = A − B, avec A = 20 ±0,1 et B = 10 ±0,05.
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.05", LinkDirection::Decreasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();

        assert_eq!(analysis.nominal, mm("10"));
        // Le jeu est maximal quand A est au plus grand et B au plus petit.
        assert_eq!(analysis.limits.max(), mm("10.15"));
        assert_eq!(analysis.limits.min(), mm("9.85"));
        assert_eq!(analysis.tolerance, mm("0.3"));
    }

    #[test]
    fn les_ecarts_asymetriques_sont_traites() {
        let links = [Link::new(
            "A",
            mm("20"),
            Deviations::new(mm("-0.05"), mm("0.1")).unwrap(),
            LinkDirection::Increasing,
        )
        .unwrap()];
        let analysis = analyse_chain(&links, false).unwrap();

        assert_eq!(analysis.limits.min(), mm("19.95"));
        assert_eq!(analysis.limits.max(), mm("20.1"));
        assert_eq!(analysis.designation(), "20 +0.1 / \u{2212}0.05");
    }

    /// Le point où l'arithmétique exacte paie : mille maillons sans dérive.
    #[test]
    fn mille_maillons_ne_font_pas_deriver_le_resultat() {
        let links: Vec<Link> = (0..1000)
            .map(|index| {
                link(
                    &format!("L{index}"),
                    "0.1",
                    "0.001",
                    LinkDirection::Increasing,
                )
            })
            .collect();
        let analysis = analyse_chain(&links, false).unwrap();

        assert_eq!(analysis.nominal, mm("100"));
        assert_eq!(analysis.tolerance, mm("2"));
        assert_eq!(analysis.limits.min(), mm("99"));
        assert_eq!(analysis.limits.max(), mm("101"));
    }

    #[test]
    fn les_contributions_totalisent_cent_pour_cent() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.05", LinkDirection::Increasing),
            link("C", "5", "0.02", LinkDirection::Increasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();

        let total: u32 = analysis
            .contributions
            .iter()
            .map(|c| c.share_per_mille)
            .sum();
        // Les arrondis peuvent décaler d'un millième par maillon.
        assert!(
            (995..=1005).contains(&total),
            "les parts totalisent {total} pour mille"
        );

        // A pèse 0,2 sur 0,34, soit 58,8 %.
        assert_eq!(analysis.contributions[0].share_per_mille, 588);
        assert_eq!(analysis.contributions[0].share_label, "58,8 %");
        assert_eq!(analysis.dominant.as_deref(), Some("A"));
    }

    #[test]
    fn lestimation_statistique_est_facultative_et_assortie_de_ses_hypotheses() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.1", LinkDirection::Increasing),
        ];

        // Par défaut, aucune hypothèse sur la fabrication n'est faite.
        assert!(analyse_chain(&links, false).unwrap().statistical.is_none());

        let estimate = analyse_chain(&links, true)
            .unwrap()
            .statistical
            .expect("une estimation était demandée");

        // Deux tolérances de 0,2 : RSS = racine(0,04 + 0,04) = 0,283 mm.
        assert_eq!(estimate.tolerance, Length::from_nanometres(282_842));
        assert!(
            estimate.tolerance < mm("0.4"),
            "le RSS doit réduire la tolérance"
        );
        assert_eq!(estimate.assumptions.len(), 5);
        assert!(estimate
            .assumptions
            .last()
            .unwrap()
            .contains("n'est vérifiée par MecaTool"));
    }

    #[test]
    fn la_racine_entiere_est_exacte_sur_les_carres_parfaits() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(144), 12);
        assert_eq!(integer_sqrt(1_000_000_000_000), 1_000_000);
        // Et arrondit vers le bas ailleurs.
        assert_eq!(integer_sqrt(143), 11);
        assert_eq!(integer_sqrt(-5), 0);
    }

    #[test]
    fn le_graphique_de_contribution_reste_fidele_aux_valeurs() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.05", LinkDirection::Increasing),
            link("C", "5", "0.02", LinkDirection::Increasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();
        let chart = contribution_chart(&analysis, 600.0);

        assert_eq!(chart.bars.len(), 3);
        // L'ordre de la chaîne est conservé.
        assert_eq!(
            chart
                .bars
                .iter()
                .map(|b| b.label.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );

        // Les largeurs suivent exactement les parts.
        let widths: Vec<f64> = chart.bars.iter().map(|b| b.width).collect();
        let shares: Vec<f64> = analysis
            .contributions
            .iter()
            .map(|c| f64::from(c.share_per_mille))
            .collect();
        for index in 1..widths.len() {
            let width_ratio = widths[index] / widths[0];
            let share_ratio = shares[index] / shares[0];
            assert!(
                (width_ratio - share_ratio).abs() < 1e-9,
                "barre {index} : rapport {width_ratio} au lieu de {share_ratio}"
            );
        }

        // Le maillon dominant est signalé, une seule fois.
        assert_eq!(chart.bars.iter().filter(|b| b.dominant).count(), 1);
        assert!(chart.bars[0].dominant);

        // Les barres ne débordent pas.
        for bar in &chart.bars {
            assert!(chart.bar_origin + bar.width <= chart.width);
            assert!(bar.y + bar.height <= chart.height);
        }
        assert!(chart.caption.contains("0.34"));
    }

    #[test]
    fn les_barres_sont_normalisees_sur_le_plus_gros_maillon() {
        // Sur une chaîne où un maillon écrase les autres, normaliser sur 100 %
        // rendrait les petits invisibles.
        let links = [
            link("A", "20", "0.5", LinkDirection::Increasing),
            link("B", "10", "0.01", LinkDirection::Increasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();
        let chart = contribution_chart(&analysis, 600.0);

        // Le plus gros occupe toute la largeur utile.
        assert!(chart.bars[0].width > chart.bars[1].width);
        assert!(chart.bars[1].width > 0.0, "le petit maillon reste visible");
    }

    #[test]
    fn une_chaine_vide_est_refusee() {
        assert!(matches!(
            analyse_chain(&[], false),
            Err(EngineError::Unparsable { .. })
        ));
    }

    #[test]
    fn deux_maillons_de_meme_repere_posent_une_question() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("A", "10", "0.05", LinkDirection::Increasing),
        ];
        match analyse_chain(&links, false).unwrap_err() {
            EngineError::Ambiguous { question, .. } => {
                assert!(question.contains("Lequel désigne quoi"));
            }
            other => panic!("une question était attendue, obtenu {other}"),
        }
    }

    #[test]
    fn un_nominal_negatif_est_refuse_avec_une_explication() {
        let error = Link::new(
            "A",
            mm("-20"),
            Deviations::symmetric(mm("0.1")),
            LinkDirection::Increasing,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("le sens s'exprime par la direction"));
    }

    #[test]
    fn la_resultante_se_confronte_a_une_exigence() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.05", LinkDirection::Decreasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();

        // Résultante 9,85 à 10,15. Exigence 9,8 à 10,2 : elle tient.
        let conclusion = verify_chain(&analysis, Some(mm("9.8")), Some(mm("10.2")));
        assert_eq!(conclusion.verdict, Verdict::Compatible);

        // Exigence 9,9 à 10,1 : elle déborde des deux côtés.
        let conclusion = verify_chain(&analysis, Some(mm("9.9")), Some(mm("10.1")));
        assert_eq!(conclusion.verdict, Verdict::Caution);
        assert!(conclusion.detail.contains("hors spécification"));

        // Exigence 12 à 13 : aucune rencontre.
        let conclusion = verify_chain(&analysis, Some(mm("12")), Some(mm("13")));
        assert_eq!(conclusion.verdict, Verdict::Incompatible);

        // Sans limites, le moteur refuse de conclure.
        let conclusion = verify_chain(&analysis, None, None);
        assert_eq!(conclusion.verdict, Verdict::InsufficientData);
    }

    #[test]
    fn le_raisonnement_montre_la_chaine_et_ses_deux_regles_de_signe() {
        let links = [
            link("A", "20", "0.1", LinkDirection::Increasing),
            link("B", "10", "0.05", LinkDirection::Decreasing),
        ];
        let analysis = analyse_chain(&links, false).unwrap();

        assert_eq!(analysis.steps.len(), 5);
        assert_eq!(analysis.steps[0].value.as_deref(), Some("A \u{2212} B"));
        assert!(analysis.steps[2]
            .expression
            .as_deref()
            .unwrap()
            .contains("min des diminuants"));
        assert!(analysis.steps[4]
            .expression
            .as_deref()
            .unwrap()
            .contains("quel que soit leur sens"));
    }
}
