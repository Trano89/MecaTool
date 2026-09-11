//! Rendu SVG d'un diagramme de zones de tolerance.
//!
//! Ce rendu sert a l'export (fichier joint a un dossier, insere dans un
//! rapport). L'application graphique, elle, trace son propre SVG a partir du
//! meme [`Diagram`], avec ses couleurs de theme : la geometrie est partagee, la
//! presentation ne l'est pas.
//!
//! Accessibilite : chaque zone porte une hachure differente **en plus** de sa
//! couleur, et toutes les valeurs sont ecrites en toutes lettres. Le dessin
//! reste lisible en noir et blanc comme en cas de daltonisme.

use core::fmt::Write as _;

use mecatool_core::Feature;

use super::{Band, Diagram};

/// Hauteur reservee sous le dessin pour l'annonce d'echelle.
const FOOTER_HEIGHT: f64 = 74.0;
/// Longueur maximale d'une ligne de l'annonce, en caracteres.
const NOTE_WRAP: usize = 92;

const HOLE_COLOUR: &str = "#2563eb";
const SHAFT_COLOUR: &str = "#c2410c";
const INK: &str = "#1f2937";
const MUTED: &str = "#6b7280";

/// Produit un SVG autonome.
pub fn to_svg(diagram: &Diagram) -> String {
    let total_height = diagram.height + FOOTER_HEIGHT;
    let mut svg = String::with_capacity(4096);

    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="system-ui, -apple-system, 'Segoe UI', sans-serif" font-size="11">"#,
        w = diagram.width,
        h = total_height
    );

    // Hachures : l'information ne doit jamais reposer sur la seule couleur.
    let _ = write!(
        svg,
        r##"<defs>
<pattern id="hole-hatch" width="6" height="6" patternTransform="rotate(45)" patternUnits="userSpaceOnUse">
<line x1="0" y="0" x2="0" y2="6" stroke="{HOLE_COLOUR}" stroke-width="2.5" opacity="0.55"/>
</pattern>
<pattern id="shaft-hatch" width="6" height="6" patternTransform="rotate(-45)" patternUnits="userSpaceOnUse">
<line x1="0" y="0" x2="0" y2="6" stroke="{SHAFT_COLOUR}" stroke-width="2.5" opacity="0.55"/>
</pattern>
</defs>"##
    );

    let _ = write!(
        svg,
        r#"<title>{}</title>"#,
        escape(&format!("Zones de tolérance {}", diagram.title))
    );

    // Ligne zero : la reference de tout le dessin.
    let _ = write!(
        svg,
        r#"<line x1="0" y1="{y}" x2="{w}" y2="{y}" stroke="{INK}" stroke-width="1.25"/>"#,
        y = round(diagram.zero_line_y),
        w = diagram.width
    );
    // L'etiquette se reduit a « 0 » : la ligne zero traverse tout le dessin et
    // n'importe quel texte plus long finirait par croiser une zone ou une cote.
    // Son sens est rappele dans la legende, sous le dessin.
    let _ = write!(
        svg,
        r#"<text x="2" y="{y}" fill="{INK}" dy="-5" font-weight="700">0</text>"#,
        y = round(diagram.zero_line_y)
    );

    // Au-dela d'une paire, l'espace par zone se reduit : les etiquettes d'ecarts
    // se chevaucheraient. Le dessin porte alors les positions, le tableau qui
    // l'accompagne porte les valeurs. Meme regle que dans le rendu de
    // l'interface : les deux tracent le meme `Diagram`, ils doivent le tracer
    // de la meme facon.
    let dense = diagram.bands.len() > 2;
    for band in &diagram.bands {
        render_band(&mut svg, band, dense, diagram.height);
    }

    // Les deux cotes de jeu occupent le vide entre les bandes, a des abscisses
    // distinctes pour ne pas se superposer.
    let gap_start = diagram.bands[0].x + diagram.bands[0].width;
    let gap_end = diagram.bands[1].x;
    let gap = gap_end - gap_start;
    for (index, marker) in diagram.clearances.iter().enumerate() {
        let fraction = if index == 0 { 0.34 } else { 0.72 };
        render_clearance(
            &mut svg,
            marker,
            gap_start,
            gap_end,
            gap_start + gap * fraction,
        );
    }

    // Legende puis annonce d'echelle. L'annonce est obligatoire et jamais tronquee.
    let mut y = diagram.height + 16.0;
    let _ = write!(
        svg,
        r#"<text x="0" y="{y}" fill="{INK}" font-size="10" font-weight="600">{label}</text>"#,
        label = escape(&format!(
            "Ligne zéro = dimension nominale {}. Zones hachurées : alésage à gauche, arbre à droite.",
            diagram.title.split_whitespace().next().unwrap_or("")
        ))
    );
    y += 15.0;
    for line in wrap(&diagram.scale_note, NOTE_WRAP) {
        let _ = write!(
            svg,
            r#"<text x="0" y="{y}" fill="{MUTED}" font-size="10">{}</text>"#,
            escape(&line)
        );
        y += 13.0;
    }

    svg.push_str("</svg>");
    svg
}

fn render_band(svg: &mut String, band: &Band, dense: bool, diagram_height: f64) {
    let (colour, fill) = match band.feature {
        Feature::Hole => (HOLE_COLOUR, "url(#hole-hatch)"),
        Feature::Shaft => (SHAFT_COLOUR, "url(#shaft-hatch)"),
    };
    // Une zone d'epaisseur nulle a l'ecran reste visible : sinon le mode fidele
    // effacerait purement et simplement les tolerances.
    let height = band.height().max(0.75);

    let _ = write!(
        svg,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" stroke="{colour}" stroke-width="1.5"/>"#,
        x = round(band.x),
        y = round(band.top),
        w = round(band.width),
        h = round(height)
    );

    let centre = band.x + band.width / 2.0;
    let _ = write!(
        svg,
        r#"<text x="{x}" y="{y}" fill="{colour}" text-anchor="middle" font-weight="700" font-size="{size}">{label}</text>"#,
        x = round(centre),
        y = round(band.top - if dense { 8.0 } else { 22.0 }),
        size = if dense { 11 } else { 13 },
        label = escape(&band.label)
    );

    if dense {
        // Le nom de l'ajustement ne s'ecrit qu'une fois par couple, sous
        // l'alesage, a la limite des deux zones.
        if let (Feature::Hole, Some(group)) = (band.feature, band.group.as_deref()) {
            let _ = write!(
                svg,
                r#"<text x="{x}" y="{y}" fill="{MUTED}" text-anchor="middle" font-weight="600" font-size="11">{label}</text>"#,
                x = round(band.x + band.width),
                y = round(diagram_height - 6.0),
                label = escape(group)
            );
        }
        return;
    }

    let _ = write!(
        svg,
        r#"<text x="{x}" y="{y}" fill="{MUTED}" text-anchor="middle" font-size="10">{label}</text>"#,
        x = round(centre),
        y = round(band.top - 9.0),
        label = escape(&band.it_label)
    );

    // Ecarts : a gauche pour l'alesage, a droite pour l'arbre. Ils occupent
    // ainsi les marges laterales au lieu d'empieter sur l'espace entre les zones,
    // reserve aux cotes de jeu.
    let (x, anchor) = match band.feature {
        Feature::Hole => (band.x - 6.0, "end"),
        Feature::Shaft => (band.x + band.width + 6.0, "start"),
    };
    let _ = write!(
        svg,
        r#"<text x="{x}" y="{y}" fill="{colour}" dy="-2" text-anchor="{anchor}" font-size="10">{label}</text>"#,
        x = round(x),
        y = round(band.top),
        label = escape(&band.upper_label)
    );
    let _ = write!(
        svg,
        r#"<text x="{x}" y="{y}" fill="{colour}" dy="9" text-anchor="{anchor}" font-size="10">{label}</text>"#,
        x = round(x),
        y = round(band.top + height),
        label = escape(&band.lower_label)
    );
}

/// Trace une cote de jeu a la maniere d'un dessin technique : deux lignes
/// d'attache partant des aretes concernees, une ligne de cote entre elles.
fn render_clearance(
    svg: &mut String,
    marker: &super::ClearanceMarker,
    gap_start: f64,
    gap_end: f64,
    x: f64,
) {
    let stroke = if marker.is_interference {
        SHAFT_COLOUR
    } else {
        INK
    };
    let (top, bottom) = if marker.hole_y <= marker.shaft_y {
        (marker.hole_y, marker.shaft_y)
    } else {
        (marker.shaft_y, marker.hole_y)
    };

    // Lignes d'attache : elles relient chaque arete a la ligne de cote, ce qui
    // rend visible *quels* bords sont mesures.
    let _ = write!(
        svg,
        r#"<line x1="{a}" y1="{y}" x2="{x}" y2="{y}" stroke="{stroke}" stroke-width="0.6" opacity="0.5"/>"#,
        a = round(gap_start),
        x = round(x),
        y = round(marker.hole_y)
    );
    let _ = write!(
        svg,
        r#"<line x1="{x}" y1="{y}" x2="{b}" y2="{y}" stroke="{stroke}" stroke-width="0.6" opacity="0.5"/>"#,
        b = round(gap_end),
        x = round(x),
        y = round(marker.shaft_y)
    );

    let _ = write!(
        svg,
        r#"<line x1="{x}" y1="{top}" x2="{x}" y2="{bottom}" stroke="{stroke}" stroke-width="1"/>"#,
        x = round(x),
        top = round(top),
        bottom = round(bottom)
    );
    // Extremites de la ligne de cote.
    for y in [top, bottom] {
        let _ = write!(
            svg,
            r#"<line x1="{a}" y1="{y}" x2="{b}" y2="{y}" stroke="{stroke}" stroke-width="1.4"/>"#,
            a = round(x - 3.5),
            b = round(x + 3.5),
            y = round(y)
        );
    }

    // L'etiquette se place toujours au-dessus de la cote, jamais au milieu :
    // le milieu d'une cote longue tombe pres de la ligne zero, ou elle
    // rencontrerait l'etiquette de celle-ci.
    let _ = write!(
        svg,
        r#"<text x="{x}" y="{y}" fill="{stroke}" dy="-6" text-anchor="middle" font-size="10" font-weight="600">{label}</text>"#,
        x = round(x),
        y = round(top),
        label = escape(&marker.label)
    );
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Coupe un texte en lignes, SVG ne sachant pas le faire lui-meme.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(core::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{fit_diagram, DiagramMode, DiagramOptions};
    use crate::iso286::Iso286Engine;
    use mecatool_core::{Length, ToleranceClass, Unit};

    fn svg_of(nominal: &str, hole: &str, shaft: &str) -> String {
        let fit = Iso286Engine::new()
            .unwrap()
            .fit(
                Length::parse(nominal, Unit::Millimetre).unwrap(),
                ToleranceClass::parse(hole).unwrap(),
                ToleranceClass::parse(shaft).unwrap(),
            )
            .unwrap()
            .fit;
        to_svg(&fit_diagram(&fit, &DiagramOptions::default()))
    }

    #[test]
    fn le_svg_est_bien_forme_et_complet() {
        let svg = svg_of("10", "H7", "g6");
        assert!(svg.starts_with("<svg "));
        assert!(svg.ends_with("</svg>"));
        assert_eq!(svg.matches("<rect").count(), 2, "une bande par élément");
        assert!(svg.contains("viewBox"));
    }

    #[test]
    fn les_valeurs_du_moteur_apparaissent_dans_le_dessin() {
        let svg = svg_of("10", "H7", "g6");
        for expected in ["H7", "g6", "+15 \u{b5}m", "-5 \u{b5}m", "-14 \u{b5}m"] {
            assert!(svg.contains(expected), "{expected:?} absent du SVG");
        }
        assert!(svg.contains("IT7 = 15 \u{b5}m"));
    }

    #[test]
    fn lannonce_dechelle_est_dans_le_dessin() {
        let svg = svg_of("20", "H7", "g6");
        assert!(svg.contains("amplifi"), "annonce d'échelle absente");
    }

    #[test]
    fn linformation_ne_repose_pas_que_sur_la_couleur() {
        let svg = svg_of("10", "H7", "g6");
        // Deux hachures distinctes, en plus des deux couleurs.
        assert!(svg.contains("hole-hatch"));
        assert!(svg.contains("shaft-hatch"));
        // Et les designations sont ecrites.
        assert!(svg.contains(">H7<"));
        assert!(svg.contains(">g6<"));
    }

    #[test]
    fn le_mode_fidele_garde_les_zones_visibles() {
        let fit = Iso286Engine::new()
            .unwrap()
            .fit(
                Length::from_millimetres(20),
                ToleranceClass::parse("H7").unwrap(),
                ToleranceClass::parse("g6").unwrap(),
            )
            .unwrap()
            .fit;
        let diagram = fit_diagram(
            &fit,
            &DiagramOptions {
                mode: DiagramMode::TrueToScale,
                ..DiagramOptions::default()
            },
        );
        let svg = to_svg(&diagram);
        // Une hauteur reelle de 0,02 px serait invisible : on impose un minimum.
        assert!(svg.contains(r#"height="0.75""#), "zone effacée du dessin");
    }

    #[test]
    fn les_caracteres_speciaux_sont_echappes() {
        assert_eq!(escape("a & b < c"), "a &amp; b &lt; c");
        let svg = svg_of("10", "H7", "g6");
        // Le titre contient Ø, qui est valide en UTF-8 et ne doit pas être échappé.
        assert!(svg.contains("Ø10 H7/g6"), "titre absent du SVG");
    }

    #[test]
    fn le_texte_long_est_coupe_en_lignes() {
        let lines = wrap("un deux trois quatre cinq six sept", 12);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.chars().count() <= 12, "ligne trop longue : {line:?}");
        }
        assert_eq!(lines.join(" "), "un deux trois quatre cinq six sept");
    }
}
