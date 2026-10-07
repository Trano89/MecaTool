//! Le registre des domaines.
//!
//! # Pourquoi un registre
//!
//! MecaTool a commence avec six ecrans traitant tous du tolerancement. Cette
//! forme supposait que l'utilisateur sache deja dans quel onglet se trouve sa
//! reponse — vrai a six onglets sur un meme sujet, beaucoup moins a douze
//! domaines couvrant roulements, soudure, visserie et matieres.
//!
//! Un domaine se **declare** donc : un identifiant, la question qu'il repond,
//! le groupe auquel il appartient, et l'etat de sa source. L'interface lit cette
//! declaration au lieu de porter une liste ecrite a la main. Ajouter un domaine
//! ne touche alors ni la navigation, ni l'accueil, ni la recherche.
//!
//! # Pourquoi c'est ici et non dans l'interface
//!
//! Parce que l'etat de la source en fait partie. Dire qu'un domaine s'appuie sur
//! une norme confrontee, sur un recueil ou sur une recommandation de fabricant
//! est une affirmation normative, et le frontend n'en porte aucune.
//!
//! Un domaine annonce en plus ce qu'il **ne fait pas encore** : un domaine prevu
//! mais sans source figure au registre, marque indisponible, avec la raison. Le
//! masquer laisserait croire qu'il n'existe pas ; l'afficher sans reserve
//! laisserait croire qu'il fonctionne.

use mecatool_core::Provenance;
use serde::{Deserialize, Serialize};

use crate::bearing::BearingEngine;
use crate::error::Result;
use crate::fasteners::FastenerEngine;
use crate::geometric::GeometricEngine;
use crate::iso2768::Iso2768Engine;
use crate::iso286::Iso286Engine;
use crate::surface::SurfaceEngine;
use crate::welding::WeldingEngine;

/// L'etat d'un domaine dans l'outil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    /// Utilisable, et ses sources sont confrontees a la norme.
    Ready,
    /// Utilisable, mais au moins une source appelle une reserve.
    Reserved,
    /// Prevu, sans source exploitable a ce jour.
    Blocked,
}

/// Le groupe de navigation auquel un domaine appartient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainGroup {
    /// Dimensions, ajustements, empilements.
    Dimensional,
    /// Geometrie et etats de surface.
    Geometry,
    /// Composants et assemblages.
    Components,
    /// Matieres.
    Materials,
}

impl DomainGroup {
    pub const fn label_fr(self) -> &'static str {
        match self {
            DomainGroup::Dimensional => "Dimensionnel",
            DomainGroup::Geometry => "Géométrie",
            DomainGroup::Components => "Composants",
            DomainGroup::Materials => "Matières",
        }
    }
}

/// L'ordre d'affichage des groupes.
pub const GROUPS: [DomainGroup; 4] = [
    DomainGroup::Dimensional,
    DomainGroup::Geometry,
    DomainGroup::Components,
    DomainGroup::Materials,
];

/// Un domaine declare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Domain {
    /// Identifiant stable, celui que l'interface emploie pour router.
    pub id: String,
    /// Le nom court, celui de la navigation.
    pub name: String,
    /// La question a laquelle ce domaine repond, a la premiere personne de
    /// l'utilisateur. C'est elle qui aide a choisir, pas le nom.
    pub question: String,
    pub group: DomainGroup,
    /// Le libelle du groupe, deja traduit.
    ///
    /// Redondant avec `group` en apparence, mais il evite que l'interface tienne
    /// sa propre table de traduction : un libelle est un texte metier, et le
    /// frontend n'en ecrit aucun. Sans ce champ, les quatre libelles et leur
    /// ordre se retrouveraient ecrits des deux cotes de la frontiere, libres de
    /// diverger.
    pub group_label: String,
    pub status: DomainStatus,
    /// Les normes ou sources mobilisees, citation courte.
    pub sources: Vec<String>,
    /// La reserve a afficher, quand il y en a une.
    pub reserve: Option<String>,
    /// Pourquoi le domaine n'est pas disponible, le cas echeant.
    pub unavailable: Option<String>,
    /// Exemples de saisie, pour la recherche et l'accueil.
    pub examples: Vec<String>,
}

impl Domain {
    pub fn is_available(&self) -> bool {
        self.status != DomainStatus::Blocked
    }
}

/// Construit un domaine disponible a partir de sa provenance.
///
/// L'etat se **deduit** des sources au lieu d'etre saisi a la main : un jeu de
/// donnees qui passerait de « verifie » a « recueil » changerait alors l'etat du
/// domaine sans que personne ait a y penser.
fn from_provenance(
    id: &str,
    name: &str,
    question: &str,
    group: DomainGroup,
    provenance: &Provenance,
    examples: &[&str],
) -> Domain {
    let warnings = provenance.warnings_fr();
    Domain {
        id: id.to_string(),
        name: name.to_string(),
        question: question.to_string(),
        group,
        group_label: group.label_fr().to_string(),
        status: if warnings.is_empty() {
            DomainStatus::Ready
        } else {
            DomainStatus::Reserved
        },
        sources: provenance.references.iter().map(|r| r.citation()).collect(),
        reserve: warnings.first().cloned(),
        unavailable: None,
        examples: examples.iter().map(|e| e.to_string()).collect(),
    }
}

fn blocked(id: &str, name: &str, question: &str, group: DomainGroup, reason: &str) -> Domain {
    Domain {
        id: id.to_string(),
        name: name.to_string(),
        question: question.to_string(),
        group,
        group_label: group.label_fr().to_string(),
        status: DomainStatus::Blocked,
        sources: Vec::new(),
        reserve: None,
        unavailable: Some(reason.to_string()),
        examples: Vec::new(),
    }
}

/// Le registre complet, disponible et a venir.
pub fn registry() -> Result<Vec<Domain>> {
    let iso286 = Iso286Engine::new()?;
    let iso2768 = Iso2768Engine::new()?;
    let geometric = GeometricEngine::new()?;
    let bearing = BearingEngine::new()?;
    let surface = SurfaceEngine::new()?;
    let welding = WeldingEngine::new()?;
    let fasteners = FastenerEngine::new()?;

    let dimensional = iso286.provenance();

    Ok(vec![
        from_provenance(
            "fit",
            "Ajustements",
            "Que donne cet ajustement, et lequel choisir ?",
            DomainGroup::Dimensional,
            &dimensional,
            &["Ø10 H7/g6", "Ø20 H7", "Ø20 + jeu 10..30"],
        ),
        from_provenance(
            "compare",
            "Comparateur",
            "Lequel de ces ajustements convient le mieux ?",
            DomainGroup::Dimensional,
            &dimensional,
            &["Ø20 H7/g6, H7/h6, H7/k6"],
        ),
        from_provenance(
            "general",
            "Tolérances générales",
            "Que valent les cotes qui ne portent pas de tolérance ?",
            DomainGroup::Dimensional,
            &iso2768.provenance(),
            &["ISO 2768-m", "50 mm en classe moyenne"],
        ),
        Domain {
            // La chaine de cotes ne mobilise aucune donnee normative : elle ne
            // fait qu'additionner ce que l'utilisateur fournit. Elle n'a donc
            // pas de provenance, et c'est une propriete a afficher, pas un oubli.
            id: "chain".into(),
            name: "Chaîne de cotes".into(),
            question: "Que donne cet empilement de cotes ?".into(),
            group: DomainGroup::Dimensional,
            group_label: DomainGroup::Dimensional.label_fr().to_string(),
            status: DomainStatus::Ready,
            sources: vec!["aucune source externe : géométrie seule".into()],
            reserve: None,
            unavailable: None,
            examples: vec!["A = 20 ±0.1 / B = 10 ±0.05 / -C = 5 ±0.02".into()],
        },
        from_provenance(
            "geometry",
            "Tolérancement géométrique",
            "Que dit ce cadre de tolérance, et que lui manque-t-il ?",
            DomainGroup::Geometry,
            &Provenance::new().with(geometric.table().standard().clone()),
            &["⟂ 0.05 A", "perp 0.05 A", "⌖ ø0.2 (M) A B C"],
        ),
        // Les etats de surface sont restes bloques tant que la seule source
        // existait en deux generations incompatibles. Le domaine s'ouvre en se
        // restreignant a ce qu'elles ont en commun, et le dit dans ses notes.
        from_provenance(
            "surface",
            "États de surface",
            "Que dit cette indication de rugosité, et quel procédé l'obtient ?",
            DomainGroup::Geometry,
            &surface.provenance(),
            &["Ra 0.8", "MRR Ra 1.6 ⊥", "N7"],
        ),
        from_provenance(
            "bearing",
            "Roulements",
            "Quel alésage, et quelle tolérance sur la portée d'arbre ?",
            DomainGroup::Components,
            &bearing.provenance(),
            &["6210", "6203-2RS", "NU2313"],
        ),
        from_provenance(
            "welding",
            "Soudure",
            "Que dit ce symbole de soudure, et que tolère son niveau de qualité ?",
            DomainGroup::Components,
            &welding.provenance(),
            &["135", "MAG", "a5 · ISO 5817-C"],
        ),
        from_provenance(
            "fasteners",
            "Visserie",
            "Quel filetage, et quel trou de passage ?",
            DomainGroup::Components,
            &fasteners.provenance(),
            &["M10", "M12 x 1.5", "M8 8.8"],
        ),
        blocked(
            "materials",
            "Matières",
            "Quelle nuance, et quelles propriétés ?",
            DomainGroup::Materials,
            "Source non encore relevée.",
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domains() -> Vec<Domain> {
        registry().expect("le registre doit se construire")
    }

    fn by_id(id: &str) -> Domain {
        domains()
            .into_iter()
            .find(|d| d.id == id)
            .unwrap_or_else(|| panic!("domaine {id} absent"))
    }

    #[test]
    fn le_registre_se_construit() {
        assert!(domains().len() >= 10);
    }

    #[test]
    fn les_identifiants_sont_uniques() {
        // Deux domaines de meme identifiant feraient router au hasard.
        let mut ids: Vec<String> = domains().into_iter().map(|d| d.id).collect();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }

    #[test]
    fn letat_se_deduit_des_sources() {
        // L'ISO 286 est confrontee a sa source primaire : aucune reserve.
        let fit = by_id("fit");
        assert_eq!(fit.status, DomainStatus::Ready);
        assert!(fit.reserve.is_none());

        // L'ISO 1101 vient d'un recueil : reserve.
        let geometry = by_id("geometry");
        assert_eq!(geometry.status, DomainStatus::Reserved);
        assert!(geometry.reserve.as_deref().unwrap().contains("recueil"));

        // Les roulements croisent un recueil ET une recommandation.
        let bearing = by_id("bearing");
        assert_eq!(bearing.status, DomainStatus::Reserved);
        assert!(bearing.reserve.is_some());
    }

    #[test]
    fn un_domaine_bloque_dit_pourquoi() {
        // Le masquer laisserait croire qu'il n'existe pas ; l'afficher sans
        // raison laisserait croire a un oubli.
        for domain in domains().iter().filter(|d| !d.is_available()) {
            let reason = domain
                .unavailable
                .as_deref()
                .unwrap_or_else(|| panic!("{} est bloque sans raison", domain.id));
            assert!(reason.len() > 20, "raison trop vague pour {}", domain.id);
        }
    }

    #[test]
    fn les_etats_de_surface_ne_melent_pas_deux_generations() {
        // Le domaine etait bloque parce que deux generations de normes ne sont
        // pas interchangeables. Il s'ouvre en se restreignant a leur tronc
        // commun : la nuance doit rester lisible dans ses sources.
        let surface = by_id("surface");
        assert!(surface.is_available());
        let engine = SurfaceEngine::new().unwrap();
        let notes = engine.provenance().references[0].notes.join(" ");
        assert!(notes.contains("21920"));
        assert!(notes.contains("4287"));
    }

    #[test]
    fn les_trois_nouveaux_domaines_disent_que_leur_source_nest_pas_verifiee() {
        // Saisies sans document ouvert : utilisables, mais jamais presentees
        // comme etablies. La reserve doit le dire en toutes lettres.
        for id in ["surface", "welding", "fasteners"] {
            let domain = by_id(id);
            assert_eq!(domain.status, DomainStatus::Reserved, "{id}");
            let reserve = domain.reserve.as_deref().unwrap();
            assert!(reserve.contains("non vérifiée"), "{id} : {reserve}");
        }
    }

    #[test]
    fn un_domaine_disponible_annonce_ses_sources() {
        for domain in domains().iter().filter(|d| d.is_available()) {
            assert!(!domain.sources.is_empty(), "{} sans source", domain.id);
            assert!(!domain.examples.is_empty(), "{} sans exemple", domain.id);
        }
    }

    #[test]
    fn la_chaine_de_cotes_annonce_navoir_aucune_source_externe() {
        // Une propriete a afficher, pas un oubli : l'empilement n'additionne
        // que ce que l'utilisateur fournit.
        let chain = by_id("chain");
        assert_eq!(chain.status, DomainStatus::Ready);
        assert!(chain.sources[0].contains("aucune source externe"));
    }

    #[test]
    fn chaque_domaine_pose_une_question() {
        // Le nom nomme, la question aide a choisir. Un domaine sans question
        // obligerait l'utilisateur a deviner ce qu'il y trouverait.
        for domain in domains() {
            assert!(domain.question.ends_with('?'), "{}", domain.id);
        }
    }

    #[test]
    fn chaque_domaine_porte_le_libelle_de_son_groupe() {
        // Sans ce champ, l'interface tiendrait sa propre table de traduction :
        // quatre libelles metier ecrits des deux cotes de la frontiere, libres
        // de diverger le jour ou l'un change.
        for domain in domains() {
            assert_eq!(domain.group_label, domain.group.label_fr(), "{}", domain.id);
            assert!(!domain.group_label.is_empty());
        }
    }

    #[test]
    fn tous_les_groupes_declares_sont_peuples() {
        for group in GROUPS {
            assert!(
                domains().iter().any(|d| d.group == group),
                "groupe vide : {group:?}"
            );
        }
    }
}
