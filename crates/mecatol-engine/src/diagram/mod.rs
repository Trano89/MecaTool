//! Geometrie des diagrammes de zones de tolerance.
//!
//! # Pourquoi la geometrie est calculee ici et non dans l'interface
//!
//! Un graphique qui contredirait les valeurs affichees serait pire qu'absent.
//! La position et la hauteur de chaque bande sont donc derivees des `Length`
//! exactes du moteur, dans du code teste, et l'interface se contente de tracer
//! les rectangles qu'on lui donne. Une erreur de conversion ne peut pas
//! s'introduire cote frontend, puisqu'il n'y a plus de conversion a y faire.
//!
//! # Les deux modes, et l'honnetete de l'echelle
//!
//! ```text
//!         H7
//!    +----------+  ES = +15 um
//!    |//////////|
//! ---+----------+------------------  ligne zero (dimension nominale)
//!                    +----------+    es = -5 um
//!                    |//////////|
//!                    +----------+    ei = -14 um
//!                         g6
//! ```
//!
//! Les ecarts se comptent en micrometres, la piece en millimetres. Un dessin
//! qui respecterait les deux echelles a la fois rendrait les zones invisibles :
//! a Ø20, une zone de 15 um represente moins d'un millieme du diametre.
//!
//! D'ou deux modes, et surtout une **annonce d'echelle systematique** : le
//! diagramme dit toujours ce que mesurerait la piece a l'echelle du dessin. Le
//! lecteur ne peut pas croire a un dessin fidele quand il ne l'est pas.

use mecatol_core::{Feature, Fit, Length, Unit};
use serde::{Deserialize, Serialize};

use crate::format::{um, um_signed};

pub mod svg;

pub use svg::to_svg;

/// Comment le diagramme traite le rapport entre la piece et ses ecarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramMode {
    /// Les ecarts occupent toute la hauteur disponible.
    ///
    /// La dimension nominale n'est pas representee ; seule la ligne zero l'est.
    /// C'est le diagramme utile, celui qu'on lit pour comparer deux zones.
    Deviations,
    /// La piece et ses ecarts partagent la meme echelle.
    ///
    /// Fidele, mais les zones deviennent invisibles des que la piece depasse
    /// quelques millimetres. Sert a montrer l'ordre de grandeur reel.
    TrueToScale,
}

/// Reglages de trace.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DiagramOptions {
    pub width: f64,
    pub height: f64,
    pub margin_x: f64,
    pub margin_y: f64,
    /// Largeur d'une bande, en pixels.
    pub band_width: f64,
    pub mode: DiagramMode,
}

impl Default for DiagramOptions {
    fn default() -> Self {
        DiagramOptions {
            width: 560.0,
            height: 340.0,
            margin_x: 56.0,
            margin_y: 44.0,
            band_width: 130.0,
            mode: DiagramMode::Deviations,
        }
    }
}

/// Une zone de tolerance, positionnee.
///
/// Les coordonnees suivent la convention SVG : `y` croit vers le bas, donc un
/// ecart positif donne un `y` plus petit. Les valeurs exactes sont conservees a
/// cote des pixels, pour que l'etiquetage n'ait jamais a repasser par le flottant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub label: String,
    pub feature: Feature,
    pub x: f64,
    pub width: f64,
    /// Bord superieur, correspondant a l'ecart superieur.
    pub top: f64,
    /// Bord inferieur, correspondant a l'ecart inferieur.
    pub bottom: f64,
    pub upper_deviation: Length,
    pub lower_deviation: Length,
    pub upper_label: String,
    pub lower_label: String,
    pub it_label: String,
}

impl Band {
    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

/// Lequel des deux jeux un marqueur represente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClearanceKind {
    /// `EI - es` : alesage au plus petit, arbre au plus grand.
    Minimum,
    /// `ES - ei` : alesage au plus grand, arbre au plus petit.
    Maximum,
}

impl ClearanceKind {
    pub const fn label_fr(self) -> &'static str {
        match self {
            ClearanceKind::Minimum => "jeu minimal",
            ClearanceKind::Maximum => "jeu maximal",
        }
    }

    /// La formule dont ce marqueur est la lecture graphique.
    pub const fn formula(self) -> &'static str {
        match self {
            ClearanceKind::Minimum => "EI - es",
            ClearanceKind::Maximum => "ES - ei",
        }
    }
}

/// Un jeu, materialise entre les deux bords qui le produisent.
///
/// Chaque jeu relie une paire d'aretes precise, et pas simplement « les deux
/// bandes » : le jeu minimal se lit entre le bas de l'alesage et le haut de
/// l'arbre, le jeu maximal entre le haut de l'alesage et le bas de l'arbre.
/// Tracer un seul trait entre les deux zones laisserait croire que la distance
/// visible represente toute la plage de jeu, ce qui est faux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClearanceMarker {
    pub kind: ClearanceKind,
    /// `y` du bord d'alesage concerne.
    pub hole_y: f64,
    /// `y` du bord d'arbre concerne.
    pub shaft_y: f64,
    pub value: Length,
    pub label: String,
    /// Vrai si la valeur est negative, c'est-a-dire s'il s'agit d'un serrage.
    pub is_interference: bool,
}

/// Une graduation de l'axe vertical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisTick {
    pub y: f64,
    pub deviation: Length,
    pub label: String,
    /// La graduation zero porte la ligne nominale et se trace differemment.
    pub is_zero: bool,
}

/// Un diagramme pret a tracer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub width: f64,
    pub height: f64,
    pub mode: DiagramMode,
    /// Ordonnee de la ligne zero, c'est-a-dire de la dimension nominale.
    pub zero_line_y: f64,
    /// Facteur de conversion, unique pour tout le diagramme.
    pub pixels_per_micrometre: f64,
    pub bands: Vec<Band>,
    /// Les deux jeux, chacun entre les aretes qui le produisent.
    pub clearances: Vec<ClearanceMarker>,
    pub axis_ticks: Vec<AxisTick>,
    /// Annonce d'echelle, a afficher sous le dessin. Jamais vide.
    pub scale_note: String,
    pub title: String,
}

impl Diagram {
    /// Ordonnee correspondant a un ecart donne.
    ///
    /// C'est la seule conversion du module ; tout le reste en decoule.
    pub fn y_of(&self, deviation: Length) -> f64 {
        self.zero_line_y - micrometres(deviation) * self.pixels_per_micrometre
    }

    /// L'ecart correspondant a une ordonnee, pour les infobulles.
    pub fn deviation_at(&self, y: f64) -> f64 {
        (self.zero_line_y - y) / self.pixels_per_micrometre
    }
}

/// Valeur en micrometres, sous forme flottante.
///
/// Seul point du moteur ou une longueur exacte devient un flottant, et
/// uniquement parce qu'une coordonnee de dessin n'a pas besoin d'etre exacte.
fn micrometres(value: Length) -> f64 {
    value.nanometres() as f64 / mecatol_core::NM_PER_UM as f64
}

/// Construit le diagramme d'un ajustement.
pub fn fit_diagram(fit: &Fit, options: &DiagramOptions) -> Diagram {
    let usable_height = (options.height - 2.0 * options.margin_y).max(1.0);

    // L'etendue verticale couvre les deux zones et la ligne zero, qui doit
    // toujours rester visible meme si aucune zone ne la traverse.
    let highest = fit
        .hole
        .deviations
        .upper()
        .max(fit.shaft.deviations.upper())
        .max(Length::ZERO);
    let lowest = fit
        .hole
        .deviations
        .lower()
        .min(fit.shaft.deviations.lower())
        .min(Length::ZERO);

    let span_um = (micrometres(highest) - micrometres(lowest)).max(f64::MIN_POSITIVE);

    let pixels_per_micrometre = match options.mode {
        DiagramMode::Deviations => usable_height / span_um,
        // A l'echelle de la piece : la hauteur utile represente le diametre nominal.
        DiagramMode::TrueToScale => usable_height / micrometres(fit.hole.nominal),
    };

    let zero_line_y = options.margin_y + micrometres(highest) * pixels_per_micrometre;

    let gap = (options.width - 2.0 * options.margin_x - 2.0 * options.band_width).max(0.0);
    let hole_x = options.margin_x;
    let shaft_x = options.margin_x + options.band_width + gap;

    let band = |tolerance: &mecatol_core::FeatureTolerance, x: f64| -> Band {
        let upper = tolerance.deviations.upper();
        let lower = tolerance.deviations.lower();
        Band {
            label: tolerance.class.to_string(),
            feature: tolerance.feature,
            x,
            width: options.band_width,
            top: zero_line_y - micrometres(upper) * pixels_per_micrometre,
            bottom: zero_line_y - micrometres(lower) * pixels_per_micrometre,
            upper_deviation: upper,
            lower_deviation: lower,
            upper_label: um_signed(upper),
            lower_label: um_signed(lower),
            it_label: format!("{} = {}", tolerance.class.grade.name(), um(tolerance.it)),
        }
    };

    let hole_band = band(&fit.hole, hole_x);
    let shaft_band = band(&fit.shaft, shaft_x);

    let marker = |kind: ClearanceKind, hole_y: f64, shaft_y: f64, value: Length| ClearanceMarker {
        kind,
        hole_y,
        shaft_y,
        value,
        label: if value.is_negative() {
            // Un jeu negatif est un serrage : le nommer ainsi evite d'avoir a
            // interpreter un signe.
            format!("serrage {}", um(-value))
        } else {
            format!("{} {}", kind.label_fr(), um(value))
        },
        is_interference: value.is_negative(),
    };

    let clearances = vec![
        marker(
            ClearanceKind::Minimum,
            hole_band.bottom,
            shaft_band.top,
            fit.min_clearance,
        ),
        marker(
            ClearanceKind::Maximum,
            hole_band.top,
            shaft_band.bottom,
            fit.max_clearance,
        ),
    ];

    let axis_ticks = ticks(
        &[
            fit.hole.deviations.upper(),
            fit.hole.deviations.lower(),
            fit.shaft.deviations.upper(),
            fit.shaft.deviations.lower(),
            Length::ZERO,
        ],
        zero_line_y,
        pixels_per_micrometre,
    );

    Diagram {
        width: options.width,
        height: options.height,
        mode: options.mode,
        zero_line_y,
        pixels_per_micrometre,
        bands: vec![hole_band, shaft_band],
        clearances,
        axis_ticks,
        scale_note: scale_note(fit.hole.nominal, pixels_per_micrometre, options.mode),
        title: format!(
            "Ø{} {}",
            trim(fit.hole.nominal.to_decimal_string(Unit::Millimetre, 3)),
            fit.designation()
        ),
    }
}

/// Graduations, une par ecart remarquable, dedoublonnees.
fn ticks(deviations: &[Length], zero_line_y: f64, ppum: f64) -> Vec<AxisTick> {
    let mut unique: Vec<Length> = deviations.to_vec();
    unique.sort();
    unique.dedup();
    unique
        .into_iter()
        .rev()
        .map(|deviation| AxisTick {
            y: zero_line_y - micrometres(deviation) * ppum,
            deviation,
            label: um_signed(deviation),
            is_zero: deviation.is_zero(),
        })
        .collect()
}

/// L'annonce d'echelle, obligatoire.
///
/// Elle traduit le facteur de conversion en une grandeur parlante : ce que
/// mesurerait la piece si elle etait dessinee a la meme echelle que ses ecarts.
/// Un lecteur ne peut alors pas prendre le dessin pour une representation
/// fidele des proportions.
fn scale_note(nominal: Length, ppum: f64, mode: DiagramMode) -> String {
    match mode {
        DiagramMode::TrueToScale => format!(
            "Échelle fidèle : la pièce et ses écarts sont au même rapport. \
             À cette échelle, une zone de tolérance de 10 µm mesure {:.2} px, \
             ce qui est le rapport réel.",
            10.0 * ppum
        ),
        DiagramMode::Deviations => {
            let nominal_px = micrometres(nominal) * ppum;
            let nominal_metres = nominal_px / PIXELS_PER_METRE;
            format!(
                "Écarts amplifiés pour la lisibilité : 1 µm = {:.1} px. À cette échelle, \
                 le diamètre nominal de {} mesurerait environ {:.1} m. Le dessin n'est donc \
                 pas à l'échelle de la pièce ; les deux zones sont en revanche au même \
                 rapport l'une que l'autre.",
                ppum,
                trim(nominal.to_decimal_string(Unit::Millimetre, 3)) + " mm",
                nominal_metres
            )
        }
    }
}

/// Densite de reference pour traduire des pixels en metres a l'ecran.
///
/// 96 px par pouce est la convention CSS ; elle suffit pour donner un ordre de
/// grandeur parlant, ce qui est le seul objectif de l'annonce d'echelle.
const PIXELS_PER_METRE: f64 = 96.0 / 0.0254;

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
    use crate::iso286::Iso286Engine;
    use mecatol_core::ToleranceClass;

    const EPS: f64 = 1e-9;

    fn fit_of(nominal_mm: &str, hole: &str, shaft: &str) -> Fit {
        Iso286Engine::new()
            .unwrap()
            .fit(
                Length::parse(nominal_mm, Unit::Millimetre).unwrap(),
                ToleranceClass::parse(hole).unwrap(),
                ToleranceClass::parse(shaft).unwrap(),
            )
            .unwrap()
            .fit
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// Le controle central : chaque bord de bande doit se reconvertir en l'ecart
    /// dont il est issu. Si le moteur change une valeur, le dessin suit.
    #[test]
    fn chaque_bord_de_bande_correspond_a_son_ecart() {
        let fit = fit_of("10", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());

        for band in &d.bands {
            assert!(
                close(d.deviation_at(band.top), micrometres(band.upper_deviation)),
                "{} : bord haut a {} px correspond a {} µm et non a {}",
                band.label,
                band.top,
                d.deviation_at(band.top),
                micrometres(band.upper_deviation)
            );
            assert!(
                close(
                    d.deviation_at(band.bottom),
                    micrometres(band.lower_deviation)
                ),
                "{} : bord bas incoherent",
                band.label
            );
        }
    }

    #[test]
    fn la_ligne_zero_correspond_a_un_ecart_nul() {
        let fit = fit_of("10", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());
        assert!(close(d.deviation_at(d.zero_line_y), 0.0));
        assert!(close(d.y_of(Length::ZERO), d.zero_line_y));
    }

    #[test]
    fn la_hauteur_dune_bande_est_proportionnelle_a_son_it() {
        let fit = fit_of("10", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());
        let (hole, shaft) = (&d.bands[0], &d.bands[1]);

        // IT7 = 15 µm, IT6 = 9 µm a Ø10.
        assert!(close(
            hole.height(),
            micrometres(fit.hole.it) * d.pixels_per_micrometre
        ));
        assert!(close(
            shaft.height(),
            micrometres(fit.shaft.it) * d.pixels_per_micrometre
        ));
        // Le rapport des hauteurs doit valoir celui des tolerances.
        let ratio_px = hole.height() / shaft.height();
        let ratio_um = micrometres(fit.hole.it) / micrometres(fit.shaft.it);
        assert!(
            (ratio_px - ratio_um).abs() < 1e-9,
            "rapport {ratio_px} au lieu de {ratio_um}"
        );
    }

    /// La promesse du graphique : chaque jeu se mesure a l'ecran, entre les deux
    /// aretes qui le produisent, et vaut exactement ce que le moteur a calcule.
    #[test]
    fn chaque_marqueur_de_jeu_mesure_la_bonne_distance() {
        let fit = fit_of("10", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());

        for marker in &d.clearances {
            let measured = marker.shaft_y - marker.hole_y;
            let expected = micrometres(marker.value) * d.pixels_per_micrometre;
            assert!(
                close(measured, expected),
                "{:?} : {measured} px a l'ecran pour {} µm",
                marker.kind,
                micrometres(marker.value)
            );
        }
    }

    #[test]
    fn les_deux_marqueurs_relient_les_aretes_attendues() {
        let fit = fit_of("10", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());
        let (hole, shaft) = (&d.bands[0], &d.bands[1]);

        let min = &d.clearances[0];
        assert_eq!(min.kind, ClearanceKind::Minimum);
        // Jeu minimal : bas de l'alesage (EI) et haut de l'arbre (es).
        assert!(close(min.hole_y, hole.bottom));
        assert!(close(min.shaft_y, shaft.top));
        assert_eq!(min.value, fit.min_clearance);

        let max = &d.clearances[1];
        assert_eq!(max.kind, ClearanceKind::Maximum);
        // Jeu maximal : haut de l'alesage (ES) et bas de l'arbre (ei).
        assert!(close(max.hole_y, hole.top));
        assert!(close(max.shaft_y, shaft.bottom));
        assert_eq!(max.value, fit.max_clearance);
    }

    #[test]
    fn un_serrage_est_nomme_serrage_et_non_jeu_negatif() {
        // p6 a Ø20 : ei = +22 µm, donc l'arbre depasse le bas de l'alesage.
        let fit = fit_of("20", "H7", "p6");
        let d = fit_diagram(&fit, &DiagramOptions::default());
        let min = &d.clearances[0];

        assert!(min.is_interference);
        assert!(
            min.label.starts_with("serrage"),
            "etiquette trompeuse : {}",
            min.label
        );
        // Le haut de l'arbre passe au-dessus du bas de l'alesage : y plus petit.
        assert!(min.shaft_y < min.hole_y);
    }

    #[test]
    fn les_formules_des_marqueurs_sont_celles_du_moteur() {
        assert_eq!(ClearanceKind::Minimum.formula(), "EI - es");
        assert_eq!(ClearanceKind::Maximum.formula(), "ES - ei");
    }

    #[test]
    fn le_dessin_tient_dans_ses_marges() {
        let options = DiagramOptions::default();
        for (nominal, hole, shaft) in [
            ("10", "H7", "g6"),
            ("20", "H7", "p6"),
            ("250", "H11", "d11"),
            ("3", "H6", "h5"),
        ] {
            let fit = fit_of(nominal, hole, shaft);
            let d = fit_diagram(&fit, &options);
            for band in &d.bands {
                assert!(
                    band.top >= options.margin_y - EPS,
                    "{} {hole}/{shaft} : bande au-dessus de la marge",
                    nominal
                );
                assert!(
                    band.bottom <= options.height - options.margin_y + EPS,
                    "{} {hole}/{shaft} : bande sous la marge",
                    nominal
                );
                assert!(band.x >= 0.0 && band.x + band.width <= options.width);
            }
        }
    }

    #[test]
    fn les_graduations_couvrent_tous_les_ecarts_remarquables() {
        let fit = fit_of("10", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());

        // 0, +15, -5, -14 : quatre valeurs distinctes.
        assert_eq!(d.axis_ticks.len(), 4);
        assert!(d.axis_ticks.iter().filter(|t| t.is_zero).count() == 1);
        // Classees du haut vers le bas.
        for pair in d.axis_ticks.windows(2) {
            assert!(pair[0].y < pair[1].y, "graduations mal ordonnees");
        }
        for tick in &d.axis_ticks {
            assert!(close(d.deviation_at(tick.y), micrometres(tick.deviation)));
        }
    }

    #[test]
    fn les_graduations_sont_dedoublonnees() {
        // H7/h6 : l'alesage part de 0 et l'arbre y arrive, donc un zero partage.
        let fit = fit_of("10", "H7", "h6");
        let d = fit_diagram(&fit, &DiagramOptions::default());
        let mut values: Vec<i64> = d
            .axis_ticks
            .iter()
            .map(|t| t.deviation.nanometres())
            .collect();
        let before = values.len();
        values.sort();
        values.dedup();
        assert_eq!(before, values.len(), "graduations en double");
    }

    /// Le dessin ne doit jamais laisser croire qu'il est a l'echelle de la piece.
    #[test]
    fn lannonce_dechelle_est_toujours_presente_et_explicite() {
        let fit = fit_of("20", "H7", "g6");
        let d = fit_diagram(&fit, &DiagramOptions::default());

        assert!(!d.scale_note.is_empty());
        assert!(
            d.scale_note.contains("amplifiés"),
            "annonce trompeuse : {}",
            d.scale_note
        );
        assert!(
            d.scale_note.contains("n'est donc pas à l'échelle"),
            "annonce trompeuse : {}",
            d.scale_note
        );
        // L'ordre de grandeur annonce doit etre parlant, donc tres superieur au metre.
        assert!(d.scale_note.contains(" m."));
    }

    #[test]
    fn le_mode_fidele_annonce_quil_est_fidele() {
        let fit = fit_of("20", "H7", "g6");
        let d = fit_diagram(
            &fit,
            &DiagramOptions {
                mode: DiagramMode::TrueToScale,
                ..DiagramOptions::default()
            },
        );
        assert!(d.scale_note.contains("fidèle"));
        // A l'echelle de la piece, une zone de 21 µm sur un Ø20 est infime.
        assert!(
            d.bands[0].height() < 1.0,
            "hauteur {} px, attendue infime",
            d.bands[0].height()
        );
    }

    /// Garde-fou contre un diagramme fige : deux ajustements differents ne
    /// peuvent pas produire la meme geometrie.
    #[test]
    fn deux_ajustements_differents_donnent_deux_dessins_differents() {
        let a = fit_diagram(&fit_of("10", "H7", "g6"), &DiagramOptions::default());
        let b = fit_diagram(&fit_of("10", "H7", "h6"), &DiagramOptions::default());
        assert_ne!(a.bands, b.bands);
        assert_ne!(a.clearances, b.clearances);
    }

    #[test]
    fn le_titre_reprend_la_designation() {
        let d = fit_diagram(&fit_of("10", "H7", "g6"), &DiagramOptions::default());
        assert_eq!(d.title, "Ø10 H7/g6");
    }

    #[test]
    fn les_etiquettes_reprennent_les_valeurs_exactes() {
        let d = fit_diagram(&fit_of("10", "H7", "g6"), &DiagramOptions::default());
        let (hole, shaft) = (&d.bands[0], &d.bands[1]);
        assert_eq!(hole.upper_label, "+15 \u{b5}m");
        assert_eq!(hole.lower_label, "0");
        assert_eq!(hole.it_label, "IT7 = 15 \u{b5}m");
        assert_eq!(shaft.upper_label, "-5 \u{b5}m");
        assert_eq!(shaft.lower_label, "-14 \u{b5}m");
        assert_eq!(shaft.it_label, "IT6 = 9 \u{b5}m");
    }

    #[test]
    fn lalesage_precede_larbre_et_ils_ne_se_chevauchent_pas() {
        let d = fit_diagram(&fit_of("10", "H7", "g6"), &DiagramOptions::default());
        assert_eq!(d.bands[0].feature, Feature::Hole);
        assert_eq!(d.bands[1].feature, Feature::Shaft);
        assert!(d.bands[0].x + d.bands[0].width <= d.bands[1].x);
    }
}
