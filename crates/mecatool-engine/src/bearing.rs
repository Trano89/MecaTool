//! Roulements : lecture d'une designation, et choix de la portee d'arbre.
//!
//! # Ce que ce module demontre
//!
//! C'est le premier domaine de MecaTool qui **en alimente un autre**. Il ne se
//! contente pas de rendre « classe k5 » : il poursuit le calcul jusqu'aux ecarts
//! reels, en passant la main au moteur ISO 286 deja present.
//!
//! ```text
//!   « 6210, bague interieure tournante, charge normale »
//!         │
//!         ▼  regle ISO 15 : symbole d'alesage → diametre
//!    alesage 50 mm
//!         │
//!         ▼  tableau des fabricants : conditions → classe
//!    k5 sur l'arbre
//!         │
//!         ▼  moteur ISO 286 : classe + nominal → ecarts
//!    Ø50 k5, ei = +2 µm, es = +13 µm
//! ```
//!
//! # La provenance melangee, et pourquoi elle importe
//!
//! Le resultat croise deux natures de source. La classe vient d'une
//! **recommandation de fabricant** : rien ne l'impose. Les ecarts qui en
//! decoulent viennent de l'**ISO 286**, confrontee a sa source primaire : ceux-la
//! s'imposent, une fois la classe choisie.
//!
//! Les deux voyagent ensemble dans la provenance du resultat, de sorte que
//! l'utilisateur voie exactement ou s'arrete la recommandation et ou commence la
//! norme. Presenter l'ensemble comme normatif durcirait le choix de la classe ;
//! le presenter comme une simple suggestion banaliserait les ecarts.

use mecatool_core::{Conclusion, Length, Provenance, ReasoningStep, ToleranceClass, Verdict};
use mecatool_standards::roulements::{
    BearingFamily, BoreDesignation, LoadRegime, MountingCase, ShaftMountingTable,
};
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::format;
use crate::iso286::{FeatureAnalysis, Iso286Engine};

/// Une lecture possible d'une designation de roulement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignationReading {
    /// La designation telle qu'elle a ete saisie.
    pub designation: String,
    /// Ce qui precede le symbole d'alesage : serie, type, suffixes de tete.
    pub series: String,
    /// Le symbole d'alesage retenu par cette lecture.
    pub bore_code: String,
    /// Le diametre d'alesage qui en decoule.
    pub bore: Length,
    /// Comment cette lecture se lit en clair.
    pub explanation: String,
}

/// Le conseil de portee d'arbre, ecarts compris.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountingAdvice {
    pub family: BearingFamily,
    pub regime: LoadRegime,
    /// La condition d'emploi retenue dans le tableau.
    pub condition: String,
    pub examples: String,
    pub bore: Length,
    /// La classe recommandee, par ex. `"k5"`.
    pub class: String,
    /// La cote telle qu'elle s'inscrirait sur le plan, par ex. `"Ø50 k5"`.
    pub designation: String,
    /// Les ecarts reels, calcules par le moteur ISO 286.
    ///
    /// C'est la composition : le domaine roulements s'arrete a la classe, le
    /// domaine ajustements prend le relais.
    pub shaft: FeatureAnalysis,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// Un cas d'emploi propose a l'utilisateur, avec ce qu'il donnerait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountingOption {
    pub regime: String,
    pub condition: String,
    pub examples: String,
    /// La classe, ou la raison de son absence.
    pub class: Option<String>,
    /// Pourquoi la source ne conclut pas, le cas echeant.
    pub unavailable: Option<String>,
}

/// Le moteur du domaine roulements.
#[derive(Debug)]
pub struct BearingEngine {
    bore: &'static BoreDesignation,
    mounting: &'static ShaftMountingTable,
    iso286: Iso286Engine,
}

impl BearingEngine {
    pub fn new() -> Result<Self> {
        Ok(BearingEngine {
            bore: BoreDesignation::embedded()?,
            mounting: ShaftMountingTable::embedded()?,
            iso286: Iso286Engine::new()?,
        })
    }

    pub fn families(&self) -> &'static [BearingFamily] {
        self.mounting.families()
    }

    pub fn regimes(&self) -> &'static [LoadRegime] {
        self.mounting.regimes()
    }

    pub fn cases(&self) -> &'static [MountingCase] {
        self.mounting.cases()
    }

    /// La provenance des seules donnees roulements.
    pub fn provenance(&self) -> Provenance {
        Provenance::new()
            .with(self.bore.standard().clone())
            .with(self.mounting.standard().clone())
    }

    /// Le diametre d'alesage que designe un symbole.
    pub fn bore_diameter(&self, code: &str) -> Result<Length> {
        Ok(self.bore.bore_diameter(code)?)
    }

    /// Les lectures possibles d'une designation.
    ///
    /// # Pourquoi plusieurs
    ///
    /// La source dit comment un symbole d'alesage se traduit en diametre. Elle
    /// ne dit **pas** comment decouper une designation en serie et symbole.
    /// `6203` se lit serie 62 + symbole 03, soit 17 mm ; `623` se lit serie 62 +
    /// symbole 3, soit 3 mm. Les deux decoupages sont formellement licites, et
    /// seule la connaissance des series existantes permettrait de trancher —
    /// connaissance que MecaTool n'a pas relevee.
    ///
    /// Il rend donc les lectures licites plutot que d'en choisir une au hasard,
    /// comme il le fait deja pour un symbole geometrique partage.
    pub fn read_designation(&self, input: &str) -> Result<Vec<DesignationReading>> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(EngineError::Unparsable {
                input: input.to_string(),
                hint: "Indiquez une désignation de roulement, par exemple « 6210 ».".into(),
            });
        }

        // On ne lit que le premier bloc : « 6203-2RS-P63 » porte ses suffixes
        // apres un tiret, et ceux-la ne concernent ni la serie ni l'alesage.
        let head = trimmed
            .split(['-', ' ', '/'])
            .next()
            .unwrap_or(trimmed)
            .trim();

        let digits: String = head.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return Err(EngineError::Unparsable {
                input: input.to_string(),
                hint: format!(
                    "« {head} » ne porte aucun chiffre : une désignation de roulement se \
                     termine par son symbole d'alésage."
                ),
            });
        }

        let mut readings = Vec::new();
        // Le symbole a deux chiffres couvre 10 a 480 mm, le symbole a un chiffre
        // couvre 1 a 9 mm. On essaie les deux decoupages, le plus courant
        // d'abord.
        for take in [2usize, 1] {
            if digits.len() <= take {
                // Il ne resterait aucune serie devant le symbole : une
                // designation qui ne serait qu'un symbole d'alesage n'en est
                // pas une.
                continue;
            }
            let split = digits.len() - take;
            let (series, code) = digits.split_at(split);
            let Ok(bore) = self.bore.bore_diameter(code) else {
                continue;
            };
            readings.push(DesignationReading {
                designation: trimmed.to_string(),
                series: series.to_string(),
                bore_code: code.to_string(),
                bore,
                explanation: format!(
                    "série {series}, symbole d'alésage {code} — alésage {} mm",
                    format::mm_trimmed(bore)
                ),
            });
        }

        if readings.is_empty() {
            return Err(EngineError::Unparsable {
                input: input.to_string(),
                hint: format!(
                    "Aucun découpage de « {head} » ne donne un symbole d'alésage connu. \
                     La règle couvre les alésages de 1 à {} mm.",
                    format::mm_trimmed(self.bore.max_bore())
                ),
            });
        }
        Ok(readings)
    }

    /// Les cas d'emploi d'un regime, avec ce que chacun donnerait.
    ///
    /// Rendus ensemble et non un a un : on choisit un montage en voyant ce que
    /// les cas voisins donneraient, comme on choisit une classe de tolerance
    /// generale en voyant les quatre a la fois.
    pub fn options(&self, regime: &str, family: &str, bore: Length) -> Result<Vec<MountingOption>> {
        self.family(family)?;
        self.regime(regime)?;

        Ok(self
            .mounting
            .cases_for(regime)
            .into_iter()
            .map(|case| {
                let row = self.mounting.class_for(case, family, bore);
                MountingOption {
                    regime: case.regime.clone(),
                    condition: case.condition.clone(),
                    examples: case.examples.clone(),
                    class: row.map(|r| r.class.clone()),
                    unavailable: row.is_none().then(|| {
                        format!(
                            "La source ne recommande rien pour un alésage de {} mm \
                             dans ce cas d'emploi.",
                            format::mm_trimmed(bore)
                        )
                    }),
                }
            })
            .collect())
    }

    /// Le conseil complet pour un cas d'emploi donne, ecarts compris.
    pub fn advise(
        &self,
        regime: &str,
        condition: &str,
        family: &str,
        bore: Length,
    ) -> Result<MountingAdvice> {
        let family_def = self.family(family)?;
        let regime_def = self.regime(regime)?;

        let case = self
            .mounting
            .cases_for(regime)
            .into_iter()
            .find(|c| c.condition == condition)
            .ok_or_else(|| EngineError::Unparsable {
                input: condition.to_string(),
                hint: "Ce cas d'emploi ne figure pas dans le tableau.".into(),
            })?;

        let row =
            self.mounting
                .class_for(case, family, bore)
                .ok_or_else(|| EngineError::Unparsable {
                    input: format!("{condition} / {} mm", format::mm_trimmed(bore)),
                    hint: format!(
                        "La source ne recommande aucune classe pour un {} d'alésage {} mm \
                     dans ce cas d'emploi. MecaTool ne comble pas une case laissée vide.",
                        family_def.name,
                        format::mm_trimmed(bore)
                    ),
                })?;

        // La composition : on quitte le domaine roulements pour le domaine
        // ajustements, et le second travaille sur des donnees confrontees a leur
        // source primaire.
        let class = ToleranceClass::parse(&row.class)?;
        let shaft = self.iso286.feature(bore, class)?;

        let designation = format!("Ø{} {}", format::mm_trimmed(bore), row.class);

        let mut provenance = self.provenance();
        provenance.merge(&shaft.provenance);

        let mut conclusion = Conclusion::new(
            Verdict::Caution,
            format!(
                "{designation} — classe {} recommandée pour {} sous {}.",
                row.class, family_def.name, regime_def.name
            ),
        );
        conclusion.why = self.reasoning(family_def, regime_def, case, bore, &row.class, &shaft);
        conclusion.warnings = provenance.warnings_fr();

        Ok(MountingAdvice {
            family: family_def.clone(),
            regime: regime_def.clone(),
            condition: case.condition.clone(),
            examples: case.examples.clone(),
            bore,
            class: row.class.clone(),
            designation,
            shaft,
            conclusion,
            provenance,
        })
    }

    /// Le raisonnement, du symbole jusqu'aux ecarts.
    fn reasoning(
        &self,
        family: &BearingFamily,
        regime: &LoadRegime,
        case: &MountingCase,
        bore: Length,
        class: &str,
        shaft: &FeatureAnalysis,
    ) -> Vec<ReasoningStep> {
        let mut steps = vec![
            ReasoningStep::new("Alésage du roulement")
                .with_expression(family.name.clone())
                .with_value(format!("{} mm", format::mm_trimmed(bore))),
            ReasoningStep::new("Régime de charge")
                .with_expression(regime.name.clone())
                .with_value(regime.explanation.clone()),
            ReasoningStep::new("Cas d'emploi")
                .with_expression(case.condition.clone())
                .with_value(case.examples.clone()),
            // L'etape qui dit ou s'arrete la recommandation.
            ReasoningStep::new("Classe recommandée")
                .with_expression(class.to_string())
                .with_value(
                    "Pratique de montage des fabricants, sans caractère normatif.".to_string(),
                ),
        ];

        // ... et celle qui dit ou commence la norme.
        steps.push(
            ReasoningStep::new("Écarts de la classe")
                .with_expression("ISO 286-1".to_string())
                .with_value(format!(
                    "ei = {}, es = {}",
                    format::um_signed(shaft.tolerance.deviations.lower()),
                    format::um_signed(shaft.tolerance.deviations.upper()),
                )),
        );

        if let Some(note) = &family.note {
            steps.push(
                ReasoningStep::new("Réserve de la source")
                    .with_expression(family.name.clone())
                    .with_value(note.clone()),
            );
        }
        steps
    }

    fn family(&self, id: &str) -> Result<&'static BearingFamily> {
        self.mounting
            .family(id)
            .ok_or_else(|| EngineError::Unparsable {
                input: id.to_string(),
                hint: format!(
                    "Famille de roulement inconnue. MecaTool en connaît {} : {}.",
                    self.mounting.families().len(),
                    self.mounting
                        .families()
                        .iter()
                        .map(|f| f.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })
    }

    fn regime(&self, id: &str) -> Result<&'static LoadRegime> {
        self.mounting
            .regime(id)
            .ok_or_else(|| EngineError::Unparsable {
                input: id.to_string(),
                hint: "Régime de charge inconnu.".into(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> BearingEngine {
        BearingEngine::new().expect("les donnees roulements doivent charger")
    }

    fn mm(value: i64) -> Length {
        Length::from_millimetres(value)
    }

    const NORMALE: &str = "Charges normales et grandes";

    #[test]
    fn lit_une_designation_courante() {
        let readings = engine().read_designation("6210").unwrap();
        let principal = &readings[0];
        assert_eq!(principal.series, "62");
        assert_eq!(principal.bore_code, "10");
        assert_eq!(principal.bore, mm(50));
    }

    #[test]
    fn ecarte_les_suffixes() {
        // « 6203-2RS-P63 » : les suffixes ne concernent ni la serie ni
        // l'alesage, et les lire ferait echouer le decoupage.
        let readings = engine().read_designation("6203-2RS-P63").unwrap();
        assert_eq!(readings[0].bore_code, "03");
        assert_eq!(readings[0].bore, mm(17));
    }

    #[test]
    fn rend_les_lectures_licites_sans_trancher() {
        // La source ne dit pas comment decouper une designation. « 623 » se lit
        // serie 6 + symbole 23, ou serie 62 + symbole 3. Les deux sont
        // formellement licites : MecaTool les rend, il n'en choisit pas.
        let readings = engine().read_designation("623").unwrap();
        assert_eq!(readings.len(), 2);

        let bores: Vec<Length> = readings.iter().map(|r| r.bore).collect();
        assert!(bores.contains(&mm(115)));
        assert!(bores.contains(&mm(3)));
    }

    #[test]
    fn une_designation_reduite_a_un_symbole_nen_est_pas_une() {
        // « 10 » seul n'est pas une designation : il ne resterait aucune serie
        // devant le symbole.
        assert!(engine().read_designation("10").is_err());
    }

    #[test]
    fn refuse_une_designation_sans_chiffre() {
        let err = engine().read_designation("NU").unwrap_err();
        assert!(err.to_string().contains("illisible") || err.to_string().contains("NU"));
    }

    #[test]
    fn le_conseil_va_jusquaux_ecarts_reels() {
        // Le point entier du module : il ne s'arrete pas a « k5 ».
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();

        assert_eq!(advice.class, "k5");
        assert_eq!(advice.designation, "Ø50 k5");

        // Les ecarts viennent du moteur ISO 286, sur une portee de 50 mm.
        let limits = advice.shaft.tolerance.limits;
        assert!(limits.min() > mm(50), "un arbre k5 est serrant");
        assert!(limits.max() > limits.min());
    }

    #[test]
    fn la_provenance_porte_les_deux_natures_de_source() {
        // C'est ce qui distingue ce resultat d'un calcul ordinaire : la classe
        // est une recommandation, les ecarts sont normatifs, et l'utilisateur
        // doit voir ou passe la frontiere.
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();

        let states: Vec<bool> = advice
            .provenance
            .references
            .iter()
            .map(|r| r.verification.is_verified())
            .collect();
        assert!(states.contains(&true), "l'ISO 286 doit figurer, verifiee");
        assert!(states.contains(&false), "la recommandation doit figurer");

        assert!(!advice.provenance.is_fully_verified());
        let warnings = advice.conclusion.warnings.join(" ");
        assert!(warnings.contains("sans caractère normatif"));
    }

    #[test]
    fn le_raisonnement_dit_ou_sarrete_la_recommandation() {
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();

        let labels: Vec<&str> = advice
            .conclusion
            .why
            .iter()
            .map(|s| s.label.as_str())
            .collect();
        assert!(labels.contains(&"Classe recommandée"));
        assert!(labels.contains(&"Écarts de la classe"));

        // L'etape de la classe doit dire que rien ne l'impose ; celle des ecarts
        // doit citer la norme. Les deux cote a cote montrent la frontiere.
        let classe = advice
            .conclusion
            .why
            .iter()
            .find(|s| s.label == "Classe recommandée")
            .unwrap();
        assert!(classe
            .value
            .as_deref()
            .unwrap()
            .contains("sans caractère normatif"));

        let ecarts = advice
            .conclusion
            .why
            .iter()
            .find(|s| s.label == "Écarts de la classe")
            .unwrap();
        assert_eq!(ecarts.expression.as_deref(), Some("ISO 286-1"));
    }

    #[test]
    fn le_verdict_reste_prudent() {
        // Une recommandation ne vaut pas une validation : le verdict ne doit
        // jamais annoncer « compatible », faute d'exigence fonctionnelle.
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();
        assert_eq!(advice.conclusion.verdict, Verdict::Caution);
    }

    #[test]
    fn une_case_vide_du_tableau_est_un_refus_explicite() {
        let err = engine()
            .advise(
                "rotating_inner",
                "Charges faibles et variables",
                "spherical_roller",
                mm(50),
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("laissée vide")
                || err.to_string().contains("recommande aucune"),
            "{err}"
        );
    }

    #[test]
    fn les_cas_demploi_se_comparent_dun_bloc() {
        // On choisit un montage en voyant ce que les cas voisins donneraient.
        let options = engine()
            .options("rotating_inner", "ball_radial", mm(50))
            .unwrap();
        assert_eq!(options.len(), 3);

        let classes: Vec<Option<&str>> = options.iter().map(|o| o.class.as_deref()).collect();
        assert!(classes.contains(&Some("k5")));

        // Une option sans classe doit dire pourquoi, jamais rester muette.
        for option in &options {
            assert_eq!(option.class.is_none(), option.unavailable.is_some());
        }
    }

    #[test]
    fn la_charge_fixe_donne_du_jeu() {
        // Bague chargee en un point fixe : elle peut coulisser, d'ou g6.
        let advice = engine()
            .advise(
                "stationary_inner",
                "La bague intérieure doit pouvoir coulisser facilement sur l'arbre",
                "ball_radial",
                mm(50),
            )
            .unwrap();
        assert_eq!(advice.class, "g6");
        // Un arbre g est plus petit que le nominal : la bague glisse.
        assert!(advice.shaft.tolerance.limits.max() < mm(50));
    }

    #[test]
    fn une_famille_inconnue_dit_lesquelles_existent() {
        let err = engine()
            .advise("rotating_inner", NORMALE, "bidule", mm(50))
            .unwrap_err();
        assert!(err.to_string().contains("ball_radial"), "{err}");
    }

    #[test]
    fn la_reserve_sur_les_aiguilles_remonte_dans_le_raisonnement() {
        let advice = engine()
            .advise("rotating_inner", NORMALE, "needle_with_inner_ring", mm(40))
            .unwrap();
        let reserve = advice
            .conclusion
            .why
            .iter()
            .find(|s| s.label == "Réserve de la source");
        assert!(reserve.is_some(), "la note sur les aiguilles doit suivre");
    }
}
