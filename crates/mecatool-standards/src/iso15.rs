//! Dimensions d'encombrement des roulements radiaux — ISO 15.
//!
//! # Le blocage que ce module leve
//!
//! Le domaine roulements ne savait partir que d'une designation deja connue.
//! Il fallait ecrire « 6210 » pour obtenir quoi que ce soit — or personne ne
//! part de la. On part d'un arbre : « j'ai un Ø50, qu'est-ce qui existe ? ».
//!
//! Ces huit tableaux permettent le chemin inverse. Ils indexent, par serie de
//! diametres et serie de dimensions, le triplet `d` / `D` / `B` que porte tout
//! roulement radial normalise.
//!
//! ```text
//!   « j'ai un arbre Ø50 »
//!         │
//!         ▼  ISO 15, huit tableaux
//!    les tailles qui existent a 50 mm
//!    dont serie 2 / dimension 02 : 50 × 90 × 20
//!         │
//!         ▼  pratique de designation des fabricants
//!    « 6210 »
//! ```
//!
//! # Ce que la norme a d'irregulier
//!
//! Chaque serie de diametres a **son propre** jeu de series de dimensions, et
//! son propre decoupage de la colonne `rs min` : une colonne pour la serie 7,
//! deux pour la plupart, **trois** pour la serie 9. Le module ne suppose donc
//! aucune forme commune : il lit celle que chaque tableau declare, et verifie
//! au chargement que chaque serie de dimensions tombe dans exactement un
//! groupe de chanfrein.
//!
//! # Ce qu'il ne porte pas, et le dit
//!
//! Les roulements a rouleaux coniques relevent de l'ISO 355, absente : une
//! designation en 3xxxx ne trouvera rien ici, et c'est un silence declare et
//! non un resultat faux. L'extrapolation au-dela des tableaux (annexe A) n'est
//! pas reprise : le module rend ce que les tableaux portent.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mecatool_core::{Length, StandardReference};
use serde::{Deserialize, Serialize};

use crate::error::{Result, StandardsError};

/// Une ligne telle qu'elle se lit dans le fichier.
///
/// Les cotes s'y ecrivent en millimetres decimaux parce que c'est ainsi que la
/// norme les imprime. Elles sont converties en nanometres entiers des le
/// chargement : aucun flottant ne survit dans la table chargee.
#[derive(Debug, Clone, Deserialize)]
struct RawRow {
    d: f64,
    #[serde(rename = "D")]
    outside: f64,
    #[serde(rename = "B")]
    widths: BTreeMap<String, Option<f64>>,
    rs_min: BTreeMap<String, Option<f64>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSeries {
    id: String,
    table: String,
    dimension_series: Vec<String>,
    chamfer_groups: Vec<String>,
    rows: Vec<RawRow>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTable {
    dataset: String,
    standard: StandardReference,
    diameter_series: Vec<RawSeries>,
}

/// Une taille normalisee, telle que les tableaux la portent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundarySize {
    /// La serie de diametres, par ex. `"2"`.
    pub diameter_series: String,
    /// La serie de dimensions, par ex. `"02"`.
    pub dimension_series: String,
    /// Diametre d'alesage `d`.
    pub bore: Length,
    /// Diametre exterieur `D`.
    pub outside: Length,
    /// Largeur `B`.
    pub width: Length,
    /// Plus petit chanfrein simple `rs min`, si le tableau en donne un.
    ///
    /// La norme avertit qu'il ne s'applique pas toujours — cote gorge de
    /// segment d'arret, epaulement rapporte, bague exterieure a contact
    /// oblique. C'est pourquoi il reste separe des trois cotes principales.
    pub chamfer: Option<Length>,
    /// Le tableau d'ou sort la ligne, tel qu'il s'intitule.
    pub table: String,
}

impl BoundarySize {
    /// La taille en clair, par ex. `"50 × 90 × 20"`.
    pub fn label_fr(&self) -> String {
        format!(
            "{} × {} × {}",
            mm_trimmed(self.bore),
            mm_trimmed(self.outside),
            mm_trimmed(self.width)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    bore: Length,
    outside: Length,
    /// Une entree par serie de dimensions declaree, dans l'ordre du tableau.
    widths: Vec<Option<Length>>,
    /// Une entree par serie de dimensions : le chanfrein de son groupe.
    chamfers: Vec<Option<Length>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Series {
    id: String,
    table: String,
    dimension_series: Vec<String>,
    rows: Vec<Row>,
}

/// Les dimensions d'encombrement des roulements radiaux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryTable {
    dataset: String,
    standard: StandardReference,
    series: Vec<Series>,
}

const EMBEDDED: &str = include_str!("../../../data/roulements/iso15-2011.boundary-dimensions.json");

impl BoundaryTable {
    pub fn embedded() -> Result<&'static BoundaryTable> {
        static CACHE: OnceLock<core::result::Result<BoundaryTable, StandardsError>> =
            OnceLock::new();
        CACHE
            .get_or_init(|| BoundaryTable::parse(EMBEDDED))
            .as_ref()
            .map_err(Clone::clone)
    }

    pub fn standard(&self) -> &StandardReference {
        &self.standard
    }

    fn parse(text: &str) -> Result<BoundaryTable> {
        let raw: RawTable =
            serde_json::from_str(text).map_err(|source| StandardsError::Malformed {
                dataset: "iso15-2011.boundary-dimensions".into(),
                detail: source.to_string(),
            })?;

        let mut series = Vec::with_capacity(raw.diameter_series.len());
        for raw_series in &raw.diameter_series {
            series.push(convert(&raw.dataset, raw_series)?);
        }

        let table = BoundaryTable {
            dataset: raw.dataset,
            standard: raw.standard,
            series,
        };
        table.validate()?;
        Ok(table)
    }

    fn validate(&self) -> Result<()> {
        let bad = |detail: String| StandardsError::Inconsistent {
            dataset: self.dataset.clone(),
            detail,
        };

        if self.series.is_empty() {
            return Err(bad("aucune serie de diametres".into()));
        }

        for series in &self.series {
            if series.rows.is_empty() {
                return Err(bad(format!("serie {} : aucune ligne", series.id)));
            }
            for row in &series.rows {
                // Un diametre exterieur qui ne depasse pas l'alesage trahirait
                // un decalage de colonnes a la transcription.
                if row.outside <= row.bore {
                    return Err(bad(format!(
                        "serie {} : D ({}) n'excede pas d ({})",
                        series.id,
                        mm_trimmed(row.outside),
                        mm_trimmed(row.bore)
                    )));
                }
                // Une ligne sans aucune largeur ne decrit aucun roulement.
                if row.widths.iter().all(Option::is_none) {
                    return Err(bad(format!(
                        "serie {} : la ligne d = {} ne porte aucune largeur",
                        series.id,
                        mm_trimmed(row.bore)
                    )));
                }
                if row.widths.len() != series.dimension_series.len()
                    || row.chamfers.len() != series.dimension_series.len()
                {
                    return Err(bad(format!(
                        "serie {} : la ligne d = {} n'a pas une valeur par serie de dimensions",
                        series.id,
                        mm_trimmed(row.bore)
                    )));
                }
            }
        }
        Ok(())
    }

    /// Tous les diametres d'alesage que les tableaux portent, tries et sans doublon.
    ///
    /// C'est la liste qu'un ecran propose a la selection : partir de l'arbre,
    /// et non d'une designation qu'il faudrait deja connaitre.
    pub fn bore_diameters(&self) -> Vec<Length> {
        let mut tous: Vec<Length> = self
            .series
            .iter()
            .flat_map(|s| s.rows.iter().map(|r| r.bore))
            .collect();
        tous.sort_unstable();
        tous.dedup();
        tous
    }

    /// Toutes les tailles normalisees existant a ce diametre d'alesage.
    ///
    /// Rend un vecteur vide lorsque aucun tableau ne porte ce diametre : c'est
    /// une absence, pas une erreur, et l'appelant doit la dire comme telle.
    pub fn sizes_for_bore(&self, bore: Length) -> Vec<BoundarySize> {
        let mut trouvees = Vec::new();
        for series in &self.series {
            let Some(row) = series.rows.iter().find(|r| r.bore == bore) else {
                continue;
            };
            for (index, dimension) in series.dimension_series.iter().enumerate() {
                let Some(width) = row.widths[index] else {
                    continue;
                };
                trouvees.push(BoundarySize {
                    diameter_series: series.id.clone(),
                    dimension_series: dimension.clone(),
                    bore: row.bore,
                    outside: row.outside,
                    width,
                    chamfer: row.chamfers[index],
                    table: series.table.clone(),
                });
            }
        }
        // Du plus compact au plus encombrant : c'est l'ordre dans lequel on
        // choisit quand la place manque, et c'est presque toujours le cas.
        trouvees.sort_by(|a, b| {
            (a.outside, a.width, &a.dimension_series).cmp(&(
                b.outside,
                b.width,
                &b.dimension_series,
            ))
        });
        trouvees
    }

    /// La taille d'une serie de dimensions precise, a un alesage donne.
    pub fn size(&self, dimension_series: &str, bore: Length) -> Result<BoundarySize> {
        self.sizes_for_bore(bore)
            .into_iter()
            .find(|s| s.dimension_series == dimension_series)
            .ok_or_else(|| StandardsError::SizeUnavailable {
                dataset: self.dataset.clone(),
                dimension_series: dimension_series.to_string(),
                bore: format!("{} mm", mm_trimmed(bore)),
            })
    }

    /// Toutes les series de dimensions declarees, tous tableaux confondus.
    pub fn dimension_series(&self) -> Vec<String> {
        let mut toutes: Vec<String> = self
            .series
            .iter()
            .flat_map(|s| s.dimension_series.iter().cloned())
            .collect();
        toutes.sort_unstable();
        toutes.dedup();
        toutes
    }
}

/// Convertit une serie brute, en resolvant les groupes de chanfrein.
///
/// # Le point delicat
///
/// La colonne `rs min` n'est pas decoupee de la meme facon d'un tableau a
/// l'autre : « 09 | 19 to 39 | 49 to 69 » pour la serie 9, une seule colonne
/// pour la serie 7. Un groupe « X to Y » se lit **par position** dans la liste
/// des series de dimensions du tableau, et non par comparaison numerique :
/// c'est ainsi que la norme l'imprime, et cela evite d'inventer un ordre.
fn convert(dataset: &str, raw: &RawSeries) -> Result<Series> {
    let bad = |detail: String| StandardsError::Inconsistent {
        dataset: dataset.to_string(),
        detail,
    };

    // Pour chaque serie de dimensions, le groupe de chanfrein dont elle releve.
    let mut groupe_de: Vec<Option<&str>> = vec![None; raw.dimension_series.len()];
    for groupe in &raw.chamfer_groups {
        let (debut, fin) = match groupe.split_once(" to ") {
            Some((a, b)) => (a, b),
            None => (groupe.as_str(), groupe.as_str()),
        };
        let index = |nom: &str| {
            raw.dimension_series
                .iter()
                .position(|d| d == nom)
                .ok_or_else(|| {
                    bad(format!(
                        "serie {} : le groupe « {groupe} » cite {nom}, absent des series de dimensions",
                        raw.id
                    ))
                })
        };
        let (i, j) = (index(debut)?, index(fin)?);
        if j < i {
            return Err(bad(format!(
                "serie {} : le groupe « {groupe} » va a rebours du tableau",
                raw.id
            )));
        }
        for emplacement in groupe_de.iter_mut().take(j + 1).skip(i) {
            if let Some(deja) = emplacement {
                return Err(bad(format!(
                    "serie {} : une serie de dimensions releve de deux groupes, « {deja} » et « {groupe} »",
                    raw.id
                )));
            }
            *emplacement = Some(groupe.as_str());
        }
    }
    // Une serie de dimensions sans groupe serait une colonne rs min manquante :
    // mieux vaut refuser la table que rendre un chanfrein absent par accident.
    if let Some(index) = groupe_de.iter().position(Option::is_none) {
        return Err(bad(format!(
            "serie {} : la serie de dimensions {} ne releve d'aucun groupe de chanfrein",
            raw.id, raw.dimension_series[index]
        )));
    }

    let mut rows = Vec::with_capacity(raw.rows.len());
    for raw_row in &raw.rows {
        let mut widths = Vec::with_capacity(raw.dimension_series.len());
        let mut chamfers = Vec::with_capacity(raw.dimension_series.len());
        for (index, dimension) in raw.dimension_series.iter().enumerate() {
            let width = raw_row.widths.get(dimension).copied().ok_or_else(|| {
                bad(format!(
                    "serie {} : la ligne d = {} ne dit rien de la serie {dimension}",
                    raw.id, raw_row.d
                ))
            })?;
            widths.push(width.map(millimetres));

            let groupe = groupe_de[index].expect("chaque serie a son groupe, verifie ci-dessus");
            let chamfer = raw_row.rs_min.get(groupe).copied().ok_or_else(|| {
                bad(format!(
                    "serie {} : la ligne d = {} ne dit rien du groupe « {groupe} »",
                    raw.id, raw_row.d
                ))
            })?;
            chamfers.push(chamfer.map(millimetres));
        }
        rows.push(Row {
            bore: millimetres(raw_row.d),
            outside: millimetres(raw_row.outside),
            widths,
            chamfers,
        });
    }

    Ok(Series {
        id: raw.id.clone(),
        table: raw.table.clone(),
        dimension_series: raw.dimension_series.clone(),
        rows,
    })
}

fn millimetres(value: f64) -> Length {
    // Les cotes de ces tableaux ont un centieme au plus fin (0,15 ; 30,2), donc
    // exactement representables en nanometres entiers.
    Length::from_nanometres((value * 1_000_000.0).round() as i64)
}

/// Une cote en millimetres, sans zero decimal superflu.
fn mm_trimmed(value: Length) -> String {
    let rendu = format!("{:.3}", value.nanometres() as f64 / 1_000_000.0);
    rendu
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(value: i64) -> Length {
        Length::from_millimetres(value)
    }

    fn table() -> &'static BoundaryTable {
        BoundaryTable::embedded().expect("les dimensions ISO 15 doivent se charger")
    }

    #[test]
    fn la_table_se_charge_et_porte_les_huit_series() {
        let table = table();
        assert_eq!(table.series.len(), 8);
        let total: usize = table.series.iter().map(|s| s.rows.len()).sum();
        assert_eq!(total, 555, "555 lignes relevees dans la norme");
    }

    #[test]
    fn un_arbre_de_50_donne_les_tailles_qui_existent() {
        // Le chemin que le domaine ne savait pas prendre : partir de l'arbre.
        let tailles = table().sizes_for_bore(mm(50));
        assert!(
            tailles.len() > 10,
            "50 mm est un diametre courant, {} tailles seulement",
            tailles.len()
        );
        // Et elles arrivent de la plus compacte a la plus encombrante.
        let diametres: Vec<Length> = tailles.iter().map(|t| t.outside).collect();
        assert!(diametres.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn la_taille_du_6210_est_celle_que_tout_le_monde_connait() {
        // 6210 : serie de dimensions 02, alesage 50 mm. Cette cote est connue de
        // tout mecanicien, et ne depend en rien de la facon dont le releve a
        // decoupe les colonnes de la norme.
        let taille = table().size("02", mm(50)).unwrap();
        assert_eq!(taille.bore, mm(50));
        assert_eq!(taille.outside, mm(90));
        assert_eq!(taille.width, mm(20));
        assert_eq!(taille.diameter_series, "2");
        assert_eq!(taille.label_fr(), "50 × 90 × 20");
    }

    #[test]
    fn les_cotes_connues_de_plusieurs_series_tombent_juste() {
        // Un temoin par serie de diametres qui en porte un d'usage courant.
        // Le tableau 8 (serie 4) est imprime en DEUX BLOCS cote a cote : s'il
        // avait ete lu a la file, le 6403 ne tomberait pas.
        for (dimension, d, attendu) in [
            ("02", 25, "25 × 52 × 15"),
            ("03", 25, "25 × 62 × 17"),
            ("03", 40, "40 × 90 × 23"),
            ("10", 20, "20 × 42 × 12"),
            ("10", 60, "60 × 95 × 18"),
            ("04", 17, "17 × 62 × 17"),
        ] {
            let taille = table().size(dimension, mm(d)).unwrap();
            assert_eq!(taille.label_fr(), attendu, "serie {dimension} a {d} mm");
        }
    }

    #[test]
    fn un_diametre_absent_rend_une_liste_vide_et_non_une_erreur() {
        // 51 mm n'est pas un alesage normalise. Ce n'est pas une panne : c'est
        // une absence, et l'ecran doit pouvoir le dire sans afficher d'erreur.
        assert!(table().sizes_for_bore(mm(51)).is_empty());
    }

    #[test]
    fn la_liste_des_alesages_est_triee_et_sans_doublon() {
        let alesages = table().bore_diameters();
        assert!(alesages.windows(2).all(|p| p[0] < p[1]));
        assert!(alesages.contains(&mm(50)));
        // Les huit tableaux se recoupent largement : la liste dedoublonnee doit
        // etre bien plus courte que les 555 lignes.
        assert!(
            alesages.len() < 200,
            "{} alesages distincts",
            alesages.len()
        );
    }

    #[test]
    fn chaque_taille_porte_un_chanfrein_ou_declare_son_absence() {
        // Le chanfrein est facultatif dans la norme, mais il ne doit jamais etre
        // absent par accident : la conversion echoue si un groupe manque.
        let table = table();
        let avec = table
            .bore_diameters()
            .iter()
            .flat_map(|d| table.sizes_for_bore(*d))
            .filter(|t| t.chamfer.is_some())
            .count();
        assert!(avec > 1000, "seulement {avec} tailles avec chanfrein");
    }

    #[test]
    fn un_groupe_de_chanfrein_qui_cite_une_serie_absente_est_refuse() {
        // Le controle qui protege de l'irregularite de la norme : si une edition
        // future decalait les groupes, la table doit etre refusee et non lue de
        // travers.
        let text = r#"{
          "dataset": "essai",
          "standard": {
            "id": "ISO 15", "edition": "2011", "title": "t", "scope": "s",
            "source": "s",
            "verification": {"state": "unverified", "pending": "essai"}
          },
          "unit": "millimetre",
          "diameter_series": [{
            "id": "2", "table": "t",
            "dimension_series": ["02", "12"],
            "chamfer_groups": ["02", "99"],
            "rows": [{"d": 50, "D": 90, "B": {"02": 20, "12": 23},
                      "rs_min": {"02": 1.1, "99": 1.1}}]
          }]
        }"#;
        let erreur = BoundaryTable::parse(text).unwrap_err();
        assert!(
            erreur.to_string().contains("99"),
            "l'erreur doit nommer le groupe fautif : {erreur}"
        );
    }
}
