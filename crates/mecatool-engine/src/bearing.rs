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
//!         │
//!         ▼  ISO 492 : l'alesage du roulement lui-meme, 0 / −12 µm
//!    serrage +2 a +25 µm (jeu de −2 a −25 µm)
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
//!
//! # Ce que l'ISO 492 a ferme
//!
//! Le module s'arretait aux ecarts de l'arbre. Il disait ce que l'arbre mesure
//! sans dire ce qu'il rencontre — et donc sans pouvoir enoncer le serrage, qui
//! est pourtant la seule chose qui interesse celui qui monte.
//!
//! Un alesage de roulement n'est pas `h0` : il porte son propre ecart
//! normalise, d'ecart superieur toujours nul. Le serrage se lit alors sur la
//! difference des deux jeux d'ecarts, rapportes au meme nominal — aucune
//! dimension absolue n'entre dans le calcul, donc aucune soustraction de grands
//! nombres presque egaux.

use mecatool_core::{
    Conclusion, FitKind, Length, Provenance, ReasoningStep, ToleranceClass, Verdict,
};
use mecatool_standards::iso15::{BoundarySize, BoundaryTable};
use mecatool_standards::iso492::{BearingToleranceTable, Ring, RingTolerance};
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
    /// La tolerance propre de l'alesage du roulement, selon l'ISO 492.
    pub bearing_bore: RingTolerance,
    /// L'ajustement qui resulte des deux.
    pub fit: BearingFit,
    pub conclusion: Conclusion,
    pub provenance: Provenance,
}

/// L'ajustement entre l'arbre et l'alesage du roulement.
///
/// C'est le resultat que le domaine ne savait pas produire avant d'avoir la
/// tolerance propre du roulement.
///
/// # Pourquoi un type propre, et non `Fit`
///
/// `Fit::assemble` prend deux `FeatureTolerance`, qui portent chacun une classe
/// ISO 286. L'alesage d'un roulement n'en a pas : sa tolerance vient de
/// l'ISO 492, dont la classe « Normale » n'appartient pas au systeme ISO 286.
/// Lui forger une classe pour entrer dans `Fit` reviendrait a inventer une
/// donnee normative.
///
/// # Pourquoi la meme convention de signe malgre tout
///
/// La grandeur portee est le **jeu** signe, dont le negatif est un serrage —
/// exactement comme dans `fit.rs`, qui dit deja pourquoi : deux notions
/// concurrentes dans le meme depot seraient une source d'erreurs de signe. Le
/// serrage, positif, se lit par `min_interference` et `max_interference`,
/// comme sur un `Fit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BearingFit {
    /// `EI - es` : l'alesage au plus petit, l'arbre au plus grand.
    pub min_clearance: Length,
    /// `ES - ei` : l'alesage au plus grand, l'arbre au plus petit.
    pub max_clearance: Length,
    pub kind: FitKind,
    /// L'ajustement en toutes lettres, redige ici et non a l'ecran.
    ///
    /// Le frontend afficherait sinon la meme phrase dans sa propre langue, et
    /// les deux redactions divergeraient au premier changement de convention.
    pub summary: String,
}

impl BearingFit {
    fn between(ring: &RingTolerance, shaft: &FeatureAnalysis) -> Self {
        // Les deux jeux d'ecarts se rapportent au meme nominal, donc l'ajustement
        // se lit directement sur leur difference — aucune dimension absolue
        // n'entre dans le calcul, et rien ne s'annule de travers.
        let bore = ring.deviations;
        let shaft = shaft.tolerance.deviations;
        let min_clearance = bore.lower() - shaft.upper();
        let max_clearance = bore.upper() - shaft.lower();
        let kind = FitKind::classify(min_clearance, max_clearance);
        BearingFit {
            min_clearance,
            max_clearance,
            kind,
            summary: describe_fr(kind, min_clearance, max_clearance),
        }
    }

    /// Serrage maximal, positif, ou `None` si le montage ne serre jamais.
    pub fn max_interference(&self) -> Option<Length> {
        self.min_clearance
            .is_negative()
            .then(|| -self.min_clearance)
    }

    /// Serrage minimal, positif, ou `None` si le montage peut ne pas serrer.
    pub fn min_interference(&self) -> Option<Length> {
        self.max_clearance
            .is_negative()
            .then(|| -self.max_clearance)
    }
}

/// L'ajustement en toutes lettres.
///
/// Les trois cas sont rediges separement plutot que ramenes a une formule signee
/// unique : « un serrage de −2 µm » ne veut rien dire pour celui qui monte la
/// piece. La convention signee sert au calcul, pas a la lecture.
fn describe_fr(kind: FitKind, min_clearance: Length, max_clearance: Length) -> String {
    match kind {
        FitKind::Interference => format!(
            "serrage de {} à {}",
            format::um(-max_clearance),
            format::um(-min_clearance)
        ),
        FitKind::Clearance => format!(
            "jeu de {} à {}",
            format::um(min_clearance),
            format::um(max_clearance)
        ),
        FitKind::Transition => format!(
            "selon les pièces, de {} de jeu à {} de serrage",
            format::um(max_clearance),
            format::um(-min_clearance)
        ),
    }
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

/// Une taille normalisee, avec ce que la designation en dirait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandardSize {
    pub size: BoundarySize,
    /// La taille en clair, par ex. `"50 × 90 × 20"`.
    pub label: String,
    /// Le symbole d'alesage, par ex. `"10"` pour 50 mm.
    ///
    /// # Pourquoi le symbole seul, et pas la designation complete
    ///
    /// Assembler une designation demanderait deux regles que MecaTool n'a PAS
    /// en source : le chiffre du type — 6 pour une bille a gorge profonde, N
    /// pour un rouleau cylindrique — et la facon dont la serie de dimensions
    /// s'y ecrit, qui n'est pas uniforme. « 6210 » n'ecrit que le diametre de
    /// la serie 02, mais « 6004 » n'ecrit aussi que le diametre de la serie
    /// **10** : la largeur disparait dans les deux cas, alors qu'elle differe.
    /// Un « 22210 », lui, ecrit sa serie 22 en entier.
    ///
    /// Une regle qui tombe juste sur 6210 et faux sur 6004 n'est pas une regle.
    /// Le moteur rend donc les pieces qu'il tient de ses sources — serie de
    /// diametres et symbole d'alesage — et laisse l'assemblage a qui detient la
    /// convention du fabricant.
    ///
    /// Vaut `None` lorsque le diametre n'a pas de symbole d'alesage.
    pub bore_code: Option<String>,
}

/// Ce qui existe a un diametre d'alesage donne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeSearch {
    pub bore: Length,
    /// De la plus compacte a la plus encombrante.
    pub sizes: Vec<StandardSize>,
    /// Les alesages normalises qui encadrent, quand celui-ci n'existe pas.
    pub nearest: Vec<Length>,
    /// Ce qu'il faut dire d'un resultat vide, redige par le moteur.
    pub note: Option<String>,
    pub provenance: Provenance,
}

/// Le moteur du domaine roulements.
#[derive(Debug)]
pub struct BearingEngine {
    bore: &'static BoreDesignation,
    mounting: &'static ShaftMountingTable,
    tolerances: &'static BearingToleranceTable,
    dimensions: &'static BoundaryTable,
    iso286: Iso286Engine,
}

impl BearingEngine {
    pub fn new() -> Result<Self> {
        Ok(BearingEngine {
            bore: BoreDesignation::embedded()?,
            mounting: ShaftMountingTable::embedded()?,
            tolerances: BearingToleranceTable::embedded()?,
            dimensions: BoundaryTable::embedded()?,
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

    /// Tous les diametres d'alesage normalises, pour une liste de selection.
    pub fn bore_diameters(&self) -> Vec<Length> {
        self.dimensions.bore_diameters()
    }

    /// Les roulements normalises qui existent a un diametre d'alesage donne.
    ///
    /// # Le chemin que le domaine ne savait pas prendre
    ///
    /// Jusqu'ici, il fallait connaitre « 6210 » pour obtenir quoi que ce soit.
    /// Personne ne part de la : on part d'un arbre, et on cherche ce qui va
    /// dessus. Cette fonction repond a cette question-la.
    pub fn sizes_for_bore(&self, bore: Length) -> SizeSearch {
        let sizes: Vec<StandardSize> = self
            .dimensions
            .sizes_for_bore(bore)
            .into_iter()
            .map(|size| self.describe_size(size))
            .collect();

        // Quand rien n'existe a ce diametre, dire lesquels existent autour vaut
        // mieux qu'une liste vide : c'est presque toujours la question suivante.
        let nearest = if sizes.is_empty() {
            self.nearest_bores(bore)
        } else {
            Vec::new()
        };

        SizeSearch {
            bore,
            note: self.search_note(bore, &sizes, &nearest),
            sizes,
            nearest,
            provenance: Provenance::new()
                .with(self.dimensions.standard().clone())
                .with(self.bore.standard().clone()),
        }
    }

    /// Les deux alesages normalises qui encadrent un diametre absent.
    fn nearest_bores(&self, bore: Length) -> Vec<Length> {
        let tous = self.dimensions.bore_diameters();
        let dessous = tous.iter().rev().find(|d| **d < bore).copied();
        let dessus = tous.iter().find(|d| **d > bore).copied();
        dessous.into_iter().chain(dessus).collect()
    }

    /// Ce qu'il faut dire du resultat, redige ici et non a l'ecran.
    fn search_note(
        &self,
        bore: Length,
        sizes: &[StandardSize],
        nearest: &[Length],
    ) -> Option<String> {
        if !sizes.is_empty() {
            return None;
        }
        let voisins = nearest
            .iter()
            .map(|d| format!("{} mm", format::mm_trimmed(*d)))
            .collect::<Vec<_>>()
            .join(" et ");
        Some(if voisins.is_empty() {
            format!(
                "{} mm n'est pas un alésage normalisé par l'ISO 15.",
                format::mm_trimmed(bore)
            )
        } else {
            format!(
                "{} mm n'est pas un alésage normalisé par l'ISO 15. Les diamètres                  voisins qui le sont : {voisins}.",
                format::mm_trimmed(bore)
            )
        })
    }

    /// Complete une taille par le symbole d'alesage qui lui correspond.
    fn describe_size(&self, size: BoundarySize) -> StandardSize {
        StandardSize {
            label: size.label_fr(),
            bore_code: self.bore.code_for(size.bore),
            size,
        }
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

        // Et la troisieme etape : l'alesage du roulement lui-meme. Sans elle, on
        // saurait ce que mesure l'arbre sans savoir ce qu'il rencontre, donc
        // sans pouvoir dire le serrage.
        let ring = self.tolerances.tolerance(Ring::Inner, bore)?;
        let fit = BearingFit::between(&ring, &shaft);

        let mut provenance = self.provenance();
        provenance.merge(&shaft.provenance);
        provenance.push(self.tolerances.standard().clone());

        // Le conseil s'assemble d'abord, la conclusion le decrit ensuite. Le
        // faire dans l'autre sens obligeait a passer huit valeurs separees au
        // redacteur du raisonnement — huit occasions d'en oublier une le jour
        // ou le conseil s'enrichit.
        let mut advice = MountingAdvice {
            family: family_def.clone(),
            regime: regime_def.clone(),
            condition: case.condition.clone(),
            examples: case.examples.clone(),
            bore,
            class: row.class.clone(),
            designation,
            shaft,
            bearing_bore: ring,
            fit,
            conclusion: Conclusion::new(Verdict::Caution, String::new()),
            provenance,
        };
        advice.conclusion = self.conclude(&advice);
        Ok(advice)
    }

    /// La conclusion d'un conseil, redigee a partir du conseil lui-meme.
    fn conclude(&self, advice: &MountingAdvice) -> Conclusion {
        // Une recommandation n'est pas une validation : sans exigence
        // fonctionnelle, le verdict reste prudent quel que soit le serrage.
        let mut conclusion = Conclusion::new(
            Verdict::Caution,
            format!(
                "{} — classe {} recommandée pour {} sous {}. Sur l'arbre : {}.",
                advice.designation,
                advice.class,
                advice.family.name,
                advice.regime.name,
                advice.fit.summary.clone()
            ),
        );
        conclusion.why = self.reasoning(advice);
        conclusion.warnings = advice.provenance.warnings_fr();
        conclusion
    }

    /// Le raisonnement, du symbole jusqu'aux ecarts.
    /// Le raisonnement, du symbole d'alesage jusqu'au serrage.
    fn reasoning(&self, advice: &MountingAdvice) -> Vec<ReasoningStep> {
        let ring = &advice.bearing_bore;
        let fit = &advice.fit;

        let mut steps = vec![
            ReasoningStep::new("Alésage du roulement")
                .with_expression(advice.family.name.clone())
                .with_value(format!("{} mm", format::mm_trimmed(advice.bore))),
            ReasoningStep::new("Régime de charge")
                .with_expression(advice.regime.name.clone())
                .with_value(advice.regime.explanation.clone()),
            ReasoningStep::new("Cas d'emploi")
                .with_expression(advice.condition.clone())
                .with_value(advice.examples.clone()),
            // L'etape qui dit ou s'arrete la recommandation.
            ReasoningStep::new("Classe recommandée")
                .with_expression(advice.class.clone())
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
                    format::um_signed(advice.shaft.tolerance.deviations.lower()),
                    format::um_signed(advice.shaft.tolerance.deviations.upper()),
                )),
        );

        steps.push(
            ReasoningStep::new("Tolérance de l'alésage")
                .with_expression(format!(
                    "{} — classe {}",
                    ring.characteristic, ring.tolerance_class
                ))
                .with_value(format!(
                    "{} à {}",
                    format::um_signed(ring.deviations.lower()),
                    format::um_signed(ring.deviations.upper())
                )),
        );

        // L'etape qui n'existait pas : l'ajustement effectif. Les deux moities du
        // calcul viennent de normes confrontees, l'ISO 286 pour l'arbre et
        // l'ISO 492 pour le roulement ; seul le CHOIX de la classe reste une
        // recommandation.
        steps.push(
            ReasoningStep::new("Ajustement obtenu")
                .with_expression("écarts de l'alésage − écarts de l'arbre".to_string())
                .with_value(fit.summary.clone()),
        );

        if let Some(note) = &advice.family.note {
            steps.push(
                ReasoningStep::new("Réserve de la source")
                    .with_expression(advice.family.name.clone())
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

    #[test]
    fn le_conseil_va_maintenant_jusquau_serrage() {
        // Ce que le domaine ne savait pas faire avant d'avoir l'ISO 492 : dire
        // ce que l'arbre rencontre, et donc quel serrage en resulte.
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();

        // L'alesage d'un roulement de classe Normale a 50 mm : 0 / −12 µm.
        assert_eq!(advice.bearing_bore.deviations.upper(), Length::ZERO);
        assert_eq!(
            advice.bearing_bore.deviations.lower(),
            Length::from_micrometres(-12)
        );

        // L'arbre k5 mesure +2 / +13. Le serrage va donc de +2 à +25 µm —
        // c'est-à-dire, dans la convention signée du dépôt, un jeu de −2 à −25.
        assert_eq!(advice.fit.kind, FitKind::Interference);
        assert_eq!(
            advice.fit.min_interference(),
            Some(Length::from_micrometres(2))
        );
        assert_eq!(
            advice.fit.max_interference(),
            Some(Length::from_micrometres(25))
        );
        assert_eq!(advice.fit.max_clearance, Length::from_micrometres(-2));
        assert_eq!(advice.fit.min_clearance, Length::from_micrometres(-25));

        // Et la phrase rendue à l'utilisateur parle bien de serrage positif :
        // « un serrage de −2 µm » ne voudrait rien dire pour qui monte la pièce.
        assert_eq!(advice.fit.summary.clone(), "serrage de 2 µm à 25 µm");
    }

    #[test]
    fn une_bague_libre_laisse_du_jeu_et_le_dit() {
        // Roue folle : la bague doit coulisser, donc g6. Le jeu calculé doit
        // rester positif au maximum — le montage ne serre pas toujours — et le
        // moteur doit le DIRE, pas laisser lire « serrage » avec un signe
        // contraire. Le signe porte l'information, le texte la rend lisible.
        let advice = engine()
            .advise(
                "stationary_inner",
                "La bague intérieure doit pouvoir coulisser facilement sur l'arbre",
                "ball_radial",
                mm(50),
            )
            .unwrap();
        assert_ne!(advice.fit.kind, FitKind::Interference);
        assert_eq!(advice.fit.min_interference(), None);
        assert!(advice.fit.max_clearance > Length::ZERO);
        assert!(advice.fit.summary.clone().contains("jeu"));
    }

    #[test]
    fn le_serrage_ne_depend_que_des_ecarts() {
        // Le calcul soustrait deux jeux d'écarts rapportés au même nominal.
        // Aucune dimension absolue n'y entre, donc aucune soustraction de
        // grands nombres presque égaux : l'exactitude est structurelle.
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();
        let shaft = advice.shaft.tolerance.deviations;
        let bore = advice.bearing_bore.deviations;
        assert_eq!(advice.fit.min_clearance, bore.lower() - shaft.upper());
        assert_eq!(advice.fit.max_clearance, bore.upper() - shaft.lower());
    }

    #[test]
    fn la_provenance_porte_les_trois_sources() {
        // Le resultat croise desormais TROIS natures : une recommandation de
        // fabricant pour la classe, l'ISO 286 pour les ecarts de l'arbre, et
        // l'ISO 492 pour ceux du roulement. Les deux dernieres sont confrontees
        // a leur source primaire ; la premiere ne l'est pas, et rien ne doit
        // laisser croire le contraire.
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();

        let citations: Vec<String> = advice
            .provenance
            .references
            .iter()
            .map(|r| r.citation())
            .collect();
        assert!(
            citations.iter().any(|c| c.contains("ISO 286")),
            "{citations:?}"
        );
        assert!(
            citations.iter().any(|c| c.contains("ISO 492")),
            "{citations:?}"
        );

        assert!(!advice.provenance.is_fully_verified());
        assert!(advice
            .conclusion
            .warnings
            .join(" ")
            .contains("sans caractère normatif"));
    }

    #[test]
    fn le_raisonnement_montre_les_deux_moities_du_calcul() {
        let advice = engine()
            .advise("rotating_inner", NORMALE, "ball_radial", mm(50))
            .unwrap();
        let labels: Vec<&str> = advice
            .conclusion
            .why
            .iter()
            .map(|s| s.label.as_str())
            .collect();
        assert!(labels.contains(&"Écarts de la classe"));
        assert!(labels.contains(&"Tolérance de l'alésage"));
        assert!(labels.contains(&"Ajustement obtenu"));
    }

    #[test]
    fn un_arbre_de_50_donne_ce_qui_existe_dessus() {
        // Le chemin que le domaine ne savait pas prendre. Personne ne part de
        // « 6210 » : on part d'un arbre.
        let recherche = engine().sizes_for_bore(mm(50));
        assert!(recherche.note.is_none(), "50 mm est un alésage normalisé");
        assert!(
            recherche.sizes.len() > 10,
            "{} tailles",
            recherche.sizes.len()
        );

        let taille = recherche
            .sizes
            .iter()
            .find(|t| t.size.dimension_series == "02")
            .expect("la série 02 existe à 50 mm");
        assert_eq!(taille.label, "50 × 90 × 20");
        assert_eq!(taille.size.diameter_series, "2");

        // Et la provenance porte l'ISO 15, confrontee a la norme.
        assert!(recherche
            .provenance
            .references
            .iter()
            .any(|r| r.id == "ISO 15"));
    }

    #[test]
    fn un_diametre_absent_dit_lesquels_existent_autour() {
        // 51 mm n'est pas un alesage normalise. Rendre une liste vide sans rien
        // dire laisserait croire a une panne ; la question suivante est
        // toujours « alors quoi, a cote ? ».
        let recherche = engine().sizes_for_bore(mm(51));
        assert!(recherche.sizes.is_empty());
        assert_eq!(recherche.nearest, vec![mm(50), mm(55)]);
        let note = recherche.note.expect("une absence doit se dire");
        assert!(note.contains("50 mm") && note.contains("55 mm"), "{note}");
    }

    #[test]
    fn le_symbole_dalesage_ne_designe_pas_un_roulement() {
        // Pourquoi le moteur n'assemble PAS de designation. A 50 mm, le symbole
        // d'alesage vaut « 10 » pour TOUTES les series — mais les roulements
        // different : 6210 mesure 90 × 20, 6010 mesure 80 × 16. Le symbole ne
        // designe que l'alesage.
        //
        // Et la facon dont la serie s'ecrit dans la designation n'est pas
        // uniforme : « 6210 » n'ecrit que le diametre de la serie 02, « 6004 »
        // n'ecrit aussi que le diametre de la serie 10. Une regle qui tombe
        // juste sur l'un et faux sur l'autre n'est pas une regle — le moteur
        // rend donc les pieces, pas l'assemblage.
        let recherche = engine().sizes_for_bore(mm(50));
        let series: Vec<&str> = recherche
            .sizes
            .iter()
            .map(|t| t.size.dimension_series.as_str())
            .collect();
        assert!(series.contains(&"02"), "{series:?}");
        assert!(series.contains(&"10"), "{series:?}");

        for taille in &recherche.sizes {
            assert_eq!(taille.bore_code.as_deref(), Some("10"));
        }

        let par_serie = |nom: &str| {
            recherche
                .sizes
                .iter()
                .find(|t| t.size.dimension_series == nom)
                .map(|t| t.label.clone())
        };
        assert_eq!(par_serie("02").as_deref(), Some("50 × 90 × 20"));
        assert_eq!(par_serie("10").as_deref(), Some("50 × 80 × 16"));
    }

    #[test]
    fn deux_etapes_ne_portent_jamais_le_meme_libelle() {
        // Le raisonnement a porte un temps deux etapes « Alésage du roulement »,
        // l'une pour le diametre, l'autre pour sa tolerance. Deux lignes de meme
        // nom dans une explication ne s'expliquent plus : elles se contredisent
        // en apparence. Le cas se reproduira a chaque etape ajoutee, d'ou ce
        // garde-fou plutot qu'une simple correction.
        for family in ["ball_radial", "needle_with_inner_ring"] {
            let advice = engine()
                .advise("rotating_inner", NORMALE, family, mm(50))
                .unwrap();
            let mut vus: Vec<&str> = advice
                .conclusion
                .why
                .iter()
                .map(|s| s.label.as_str())
                .collect();
            let total = vus.len();
            vus.sort_unstable();
            vus.dedup();
            assert_eq!(vus.len(), total, "libellés répétés dans {family} : {vus:?}");
        }
    }
}
