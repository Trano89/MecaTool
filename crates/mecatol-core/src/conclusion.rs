//! Conclusions et justifications.
//!
//! Tout calcul Mecatol se termine par une conclusion explicite, et toute
//! conclusion transporte le raisonnement qui y mene. Le « pourquoi » n'est pas
//! un texte redige apres coup : c'est la trace des grandeurs effectivement
//! comparees par le moteur.

use core::fmt;

use serde::{Deserialize, Serialize};

/// Le verdict d'un calcul, sur quatre etats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// L'exigence est entierement satisfaite.
    Compatible,
    /// Partiellement satisfaite, ou satisfaite sous reserve.
    Caution,
    /// Non satisfaite.
    Incompatible,
    /// Le moteur ne dispose pas de quoi conclure.
    InsufficientData,
}

impl Verdict {
    /// Pastille de statut, toujours accompagnee du libelle : la couleur seule ne
    /// doit jamais porter l'information (accessibilite).
    pub const fn badge(self) -> &'static str {
        match self {
            Verdict::Compatible => "\u{1f7e2}",
            Verdict::Caution => "\u{1f7e0}",
            Verdict::Incompatible => "\u{1f534}",
            Verdict::InsufficientData => "\u{1f535}",
        }
    }

    pub const fn headline_fr(self) -> &'static str {
        match self {
            Verdict::Compatible => "COMPATIBLE",
            Verdict::Caution => "ATTENTION",
            Verdict::Incompatible => "NON COMPATIBLE",
            Verdict::InsufficientData => "INFORMATIONS INSUFFISANTES",
        }
    }

    /// Identifiant stable, destine aux tests et au CSS de l'interface.
    pub const fn slug(self) -> &'static str {
        match self {
            Verdict::Compatible => "compatible",
            Verdict::Caution => "caution",
            Verdict::Incompatible => "incompatible",
            Verdict::InsufficientData => "insufficient-data",
        }
    }

    /// Le pire des deux verdicts, pour agreger plusieurs criteres.
    ///
    /// Un manque d'information l'emporte sur tout le reste : mieux vaut dire
    /// qu'on ne sait pas que d'affirmer une compatibilite non etablie.
    pub fn worst(self, other: Verdict) -> Verdict {
        use Verdict::*;
        match (self, other) {
            (InsufficientData, _) | (_, InsufficientData) => InsufficientData,
            (Incompatible, _) | (_, Incompatible) => Incompatible,
            (Caution, _) | (_, Caution) => Caution,
            _ => Compatible,
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.badge(), self.headline_fr())
    }
}

/// Une etape du raisonnement, telle qu'affichee sous « Pourquoi ? ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// Ce qui est calcule, par ex. `"Jeu minimal"`.
    pub label: String,
    /// La formule employee, par ex. `"EI - es"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// L'application numerique, par ex. `"0 - (-0,005) = 0,005 mm"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl ReasoningStep {
    pub fn new(label: impl Into<String>) -> Self {
        ReasoningStep {
            label: label.into(),
            expression: None,
            value: None,
        }
    }

    pub fn with_expression(mut self, expression: impl Into<String>) -> Self {
        self.expression = Some(expression.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

/// La conclusion d'un calcul, avec sa justification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conclusion {
    pub verdict: Verdict,
    /// Phrase courte expliquant le verdict a un lecteur non specialiste.
    pub detail: String,
    /// Le detail du raisonnement, deplie a la demande.
    #[serde(default)]
    pub why: Vec<ReasoningStep>,
    /// Reserves a afficher meme quand le verdict est favorable, notamment
    /// lorsqu'une donnee normative employee n'est pas verifiee.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl Conclusion {
    pub fn new(verdict: Verdict, detail: impl Into<String>) -> Self {
        Conclusion {
            verdict,
            detail: detail.into(),
            why: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn compatible(detail: impl Into<String>) -> Self {
        Conclusion::new(Verdict::Compatible, detail)
    }

    pub fn caution(detail: impl Into<String>) -> Self {
        Conclusion::new(Verdict::Caution, detail)
    }

    pub fn incompatible(detail: impl Into<String>) -> Self {
        Conclusion::new(Verdict::Incompatible, detail)
    }

    pub fn insufficient_data(detail: impl Into<String>) -> Self {
        Conclusion::new(Verdict::InsufficientData, detail)
    }

    pub fn with_step(mut self, step: ReasoningStep) -> Self {
        self.why.push(step);
        self
    }

    pub fn with_steps(mut self, steps: impl IntoIterator<Item = ReasoningStep>) -> Self {
        self.why.extend(steps);
        self
    }

    pub fn with_warnings(mut self, warnings: impl IntoIterator<Item = String>) -> Self {
        self.warnings.extend(warnings);
        self
    }

    /// Titre complet, par ex. `"\u{1f7e2} COMPATIBLE"`.
    pub fn headline(&self) -> String {
        self.verdict.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_verdicts_ont_un_libelle_et_une_pastille() {
        assert_eq!(Verdict::Compatible.headline_fr(), "COMPATIBLE");
        assert_eq!(Verdict::Incompatible.headline_fr(), "NON COMPATIBLE");
        // La pastille ne remplace jamais le texte.
        assert!(Verdict::Caution.to_string().contains("ATTENTION"));
        assert!(Verdict::InsufficientData
            .to_string()
            .contains("INFORMATIONS INSUFFISANTES"));
    }

    #[test]
    fn lagregation_retient_le_pire_verdict() {
        use Verdict::*;
        assert_eq!(Compatible.worst(Caution), Caution);
        assert_eq!(Caution.worst(Incompatible), Incompatible);
        assert_eq!(Compatible.worst(Compatible), Compatible);
        // Le manque d'information prime sur tout, y compris sur l'incompatibilite.
        assert_eq!(Incompatible.worst(InsufficientData), InsufficientData);
        assert_eq!(InsufficientData.worst(Compatible), InsufficientData);
    }

    #[test]
    fn une_conclusion_transporte_son_raisonnement() {
        let c = Conclusion::incompatible("Le jeu minimal calcule est inferieur au besoin.")
            .with_step(
                ReasoningStep::new("Jeu minimal acceptable")
                    .with_value("5 \u{b5}m"),
            )
            .with_step(
                ReasoningStep::new("Jeu minimal calcule")
                    .with_expression("EI - es")
                    .with_value("3 \u{b5}m"),
            );

        assert_eq!(c.verdict, Verdict::Incompatible);
        assert_eq!(c.why.len(), 2);
        assert_eq!(c.why[1].expression.as_deref(), Some("EI - es"));
        assert!(c.headline().contains("NON COMPATIBLE"));
    }

    #[test]
    fn les_reserves_survivent_a_un_verdict_favorable() {
        let c = Conclusion::compatible("Tout va bien.")
            .with_warnings(["Donnee normative non verifiee.".to_string()]);
        assert_eq!(c.verdict, Verdict::Compatible);
        assert_eq!(c.warnings.len(), 1);
    }
}
