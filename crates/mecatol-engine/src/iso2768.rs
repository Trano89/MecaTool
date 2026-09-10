//! Tolerances generales selon l'ISO 2768-1.
//!
//! Il n'y a rien a calculer ici : la norme donne directement l'ecart limite en
//! fonction de la classe, du type de cote et de la dimension nominale. Le
//! moteur consulte la table, en deduit les dimensions limites, et rend le
//! raisonnement complet.
//!
//! Sa valeur pedagogique est ailleurs : [`Iso2768Engine::across_classes`] montre
//! d'un coup ce que les quatre classes donnent sur la meme cote. C'est ce qui
//! permet de choisir une classe, plutot que de la subir.

use serde::{Deserialize, Serialize};

use mecatol_core::{Length, LimitsOfSize, Provenance, ReasoningStep, Unit};
use mecatol_standards::iso2768::{
    GeneralClass, GeneralDeviation, GeneralLookup, GeneralToleranceTable, MeasureKind, ALL_CLASSES,
};

use crate::error::Result;

/// Le resultat d'une consultation des tolerances generales.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralAnalysis {
    pub lookup: GeneralLookup,
    pub nominal: Length,
    /// Dimensions limites. Absentes pour une cote angulaire : la table donne un
    /// ecart, mais l'angle nominal auquel il s'applique n'est pas connu ici.
    pub limits: Option<LimitsOfSize>,
    pub steps: Vec<ReasoningStep>,
    pub provenance: Provenance,
}

impl GeneralAnalysis {
    /// Ecriture de la cote telle qu'elle figurerait sur un plan.
    pub fn designation(&self) -> String {
        match self.lookup.deviation {
            GeneralDeviation::Linear { magnitude } => format!(
                "{} {} mm",
                trim(self.nominal.to_decimal_string(Unit::Millimetre, 3)),
                symmetric(magnitude)
            ),
            GeneralDeviation::Angular { magnitude } => magnitude.to_symmetric_string(),
        }
    }
}

/// Ce que les quatre classes donnent sur une meme cote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassComparison {
    pub kind: MeasureKind,
    pub nominal: Length,
    /// Une entree par classe, dans l'ordre du plus fin au plus grossier.
    pub rows: Vec<ClassRow>,
    pub provenance: Provenance,
}

/// Une classe, et ce qu'elle donne — ou pourquoi elle ne donne rien.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassRow {
    pub class: GeneralClass,
    pub symbol: String,
    pub name: String,
    pub designation: String,
    /// L'ecart, quand la norme en definit un pour cette combinaison.
    pub deviation: Option<GeneralDeviation>,
    pub deviation_label: Option<String>,
    pub limits: Option<LimitsOfSize>,
    /// Pourquoi la norme ne definit rien, le cas echeant.
    pub unavailable: Option<String>,
}

/// Le moteur des tolerances generales.
#[derive(Debug, Clone, Copy)]
pub struct Iso2768Engine {
    table: &'static GeneralToleranceTable,
}

impl Iso2768Engine {
    pub fn new() -> Result<Self> {
        Ok(Iso2768Engine {
            table: GeneralToleranceTable::embedded()?,
        })
    }

    pub fn provenance(&self) -> Provenance {
        Provenance::new().with(self.table.standard().clone())
    }

    /// Consulte les tables pour une cote et une classe.
    ///
    /// Pour une cote angulaire, `nominal` est la **longueur du cote le plus
    /// court** de l'angle. C'est ce que la norme prend comme entree, et non la
    /// valeur de l'angle : le meme ecart lineaire, rapporte a un bras plus long,
    /// donne un angle plus petit.
    pub fn general(
        &self,
        kind: MeasureKind,
        nominal: Length,
        class: GeneralClass,
    ) -> Result<GeneralAnalysis> {
        let lookup = self.table.lookup(kind, nominal, class)?;

        let mut steps = vec![
            ReasoningStep::new("Classe retenue")
                .with_expression(class.designation())
                .with_value(format!("{} — {}", class.symbol(), class.name_fr())),
            ReasoningStep::new("Table consultée")
                .with_expression(lookup.table.clone())
                .with_value(format!("échelon {} mm", lookup.range.label_fr())),
            ReasoningStep::new("Écart limite")
                .with_expression(match kind {
                    MeasureKind::Angular => {
                        "lu selon la longueur du côté le plus court de l'angle".to_string()
                    }
                    _ => "lu selon la dimension nominale".to_string(),
                })
                .with_value(lookup.deviation.to_symmetric_string()),
        ];

        let limits = match lookup.deviation {
            GeneralDeviation::Linear { magnitude } => {
                let limits = LimitsOfSize::new(nominal - magnitude, nominal + magnitude)?;
                steps.push(
                    ReasoningStep::new("Dimensions limites")
                        .with_expression("nominal ± écart")
                        .with_value(format!(
                            "{} .. {} mm",
                            limits.min().to_decimal_string(Unit::Millimetre, 3),
                            limits.max().to_decimal_string(Unit::Millimetre, 3)
                        )),
                );
                Some(limits)
            }
            GeneralDeviation::Angular { .. } => {
                steps.push(
                    ReasoningStep::new("Portée")
                        .with_expression("l'écart s'applique à l'angle porté par ce côté")
                        .with_value(
                            "la valeur de l'angle nominal n'entre pas dans la table".to_string(),
                        ),
                );
                None
            }
        };

        Ok(GeneralAnalysis {
            lookup,
            nominal,
            limits,
            steps,
            provenance: self.provenance(),
        })
    }

    /// Ce que les quatre classes donnent sur la meme cote.
    ///
    /// Les classes qu'aucune valeur ne couvre restent dans le tableau, avec la
    /// raison : les retirer laisserait croire qu'elles n'existent pas, alors
    /// qu'elles ne sont simplement pas definies a cette dimension.
    pub fn across_classes(&self, kind: MeasureKind, nominal: Length) -> ClassComparison {
        let rows = ALL_CLASSES
            .into_iter()
            .map(|class| match self.general(kind, nominal, class) {
                Ok(analysis) => ClassRow {
                    class,
                    symbol: class.symbol().to_string(),
                    name: class.name_fr().to_string(),
                    designation: analysis.designation(),
                    deviation: Some(analysis.lookup.deviation),
                    deviation_label: Some(analysis.lookup.deviation.to_symmetric_string()),
                    limits: analysis.limits,
                    unavailable: None,
                },
                Err(error) => ClassRow {
                    class,
                    symbol: class.symbol().to_string(),
                    name: class.name_fr().to_string(),
                    designation: class.designation(),
                    deviation: None,
                    deviation_label: None,
                    limits: None,
                    unavailable: Some(error.to_string()),
                },
            })
            .collect();

        ClassComparison {
            kind,
            nominal,
            rows,
            provenance: self.provenance(),
        }
    }
}

fn symmetric(magnitude: Length) -> String {
    format!(
        "\u{b1} {}",
        trim(magnitude.to_decimal_string(Unit::Millimetre, 3))
    )
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
    use mecatol_core::Angle;

    fn engine() -> Iso2768Engine {
        Iso2768Engine::new().unwrap()
    }

    fn mm(value: &str) -> Length {
        Length::parse(value, Unit::Millimetre).unwrap()
    }

    #[test]
    fn une_cote_lineaire_donne_ses_dimensions_limites() {
        let a = engine()
            .general(MeasureKind::Linear, mm("50"), GeneralClass::Medium)
            .unwrap();
        let limits = a.limits.expect("des limites étaient attendues");
        assert_eq!(limits.min(), mm("49.7"));
        assert_eq!(limits.max(), mm("50.3"));
        assert_eq!(a.designation(), "50 ± 0.3 mm");
    }

    #[test]
    fn une_cote_angulaire_donne_un_ecart_sans_limites() {
        let a = engine()
            .general(MeasureKind::Angular, mm("100"), GeneralClass::Coarse)
            .unwrap();
        assert!(a.limits.is_none(), "un angle n'a pas de dimensions limites");
        assert_eq!(
            a.lookup.deviation,
            GeneralDeviation::Angular {
                magnitude: Angle::from_dms(0, 30, 0)
            }
        );
        assert_eq!(a.designation(), "\u{b1} 0\u{b0}30'");
    }

    #[test]
    fn le_raisonnement_cite_la_classe_la_table_et_lechelon() {
        let a = engine()
            .general(MeasureKind::Linear, mm("50"), GeneralClass::Medium)
            .unwrap();
        assert_eq!(a.steps.len(), 4);
        assert_eq!(a.steps[0].expression.as_deref(), Some("ISO 2768-m"));
        assert!(a.steps[1]
            .expression
            .as_deref()
            .unwrap()
            .contains("Tableau 1"));
        assert!(a.steps[1].value.as_deref().unwrap().contains("30"));
    }

    #[test]
    fn le_tableau_des_classes_garde_celles_qui_ne_sont_pas_definies() {
        // A 2 mm, la classe v n'est pas definie ; elle doit rester visible avec
        // sa raison, sans quoi on croirait qu'elle n'existe pas.
        let comparison = engine().across_classes(MeasureKind::Linear, mm("2"));
        assert_eq!(comparison.rows.len(), 4);

        let very_coarse = comparison
            .rows
            .iter()
            .find(|r| r.class == GeneralClass::VeryCoarse)
            .unwrap();
        assert!(very_coarse.deviation.is_none());
        let reason = very_coarse.unavailable.as_deref().unwrap();
        assert!(
            reason.contains("ne définit pas"),
            "raison peu claire : {reason}"
        );

        // Les trois autres classes sont bien renseignees.
        assert_eq!(
            comparison
                .rows
                .iter()
                .filter(|r| r.deviation.is_some())
                .count(),
            3
        );
    }

    #[test]
    fn le_tableau_des_classes_va_du_plus_fin_au_plus_grossier() {
        let comparison = engine().across_classes(MeasureKind::Linear, mm("50"));
        let magnitudes: Vec<i64> = comparison
            .rows
            .iter()
            .filter_map(|r| match r.deviation {
                Some(GeneralDeviation::Linear { magnitude }) => Some(magnitude.nanometres()),
                _ => None,
            })
            .collect();
        assert_eq!(magnitudes.len(), 4);
        for pair in magnitudes.windows(2) {
            assert!(pair[0] <= pair[1], "classes mal ordonnées : {magnitudes:?}");
        }
    }

    #[test]
    fn les_quatre_classes_dun_angle_se_comparent_aussi() {
        let comparison = engine().across_classes(MeasureKind::Angular, mm("30"));
        let labels: Vec<&str> = comparison
            .rows
            .iter()
            .filter_map(|r| r.deviation_label.as_deref())
            .collect();
        assert_eq!(
            labels,
            vec![
                "\u{b1} 0\u{b0}30'",
                "\u{b1} 0\u{b0}30'",
                "\u{b1} 1\u{b0}",
                "\u{b1} 2\u{b0}"
            ]
        );
    }

    #[test]
    fn une_cote_trop_petite_renvoie_a_la_cotation_individuelle() {
        let error = engine()
            .general(MeasureKind::Linear, mm("0.2"), GeneralClass::Medium)
            .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("directement à côté"),
            "message : {message}"
        );
    }

    #[test]
    fn le_resultat_porte_sa_provenance() {
        let a = engine()
            .general(MeasureKind::Linear, mm("50"), GeneralClass::Medium)
            .unwrap();
        assert_eq!(a.provenance.references.len(), 1);
        assert!(a.provenance.is_fully_verified());
        assert_eq!(a.provenance.references[0].id, "ISO 2768-1");
    }
}
