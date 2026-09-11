//! Outil de vérification en ligne de commande.
//!
//! Ce binaire n'est pas le produit : c'est le banc d'essai du moteur. Il permet
//! de confronter à la main les résultats de MecaTool aux tables d'un manuel ou
//! d'une norme, sans passer par l'interface graphique.
//!
//! ```text
//!   mecatool "Ø10 H7/g6"
//!   mecatool --expert "Ø20 H7/k6"
//!   mecatool "25 H7"
//! ```

use std::process::ExitCode;

use mecatool_core::{Feature, FeatureTolerance, Length, ReasoningStep, Unit};
use mecatool_engine::chain::analyse_chain;
use mecatool_engine::compare::{compare_fits, nominal_label};
use mecatool_engine::diagram::{fit_diagram, to_svg, DiagramMode, DiagramOptions};
use mecatool_engine::iso286::{classification_conclusion, FitAnalysis, Iso286Engine};
use mecatool_engine::parser::{
    parse, parse_chain, parse_clearance_window, parse_comparison, ParsedInput,
};
use mecatool_engine::requirement::{verify_clearance, ClearanceRequirement};
use mecatool_engine::search::{find_fits, SearchOptions, SearchResult};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    enable_utf8_console();

    let mut expert = false;
    let mut window: Option<String> = None;
    let mut svg_path: Option<String> = None;
    let mut true_to_scale = false;
    let mut statistical = false;
    let mut parts: Vec<String> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--expert" | "-x" => expert = true,
            "--fidele" => true_to_scale = true,
            "--stat" => statistical = true,
            "--jeu" | "-j" => match args.next() {
                Some(value) => window = Some(value),
                None => {
                    eprintln!("\nErreur : --jeu attend une fenêtre, par ex. --jeu 10..30\n");
                    return ExitCode::FAILURE;
                }
            },
            "--svg" => match args.next() {
                Some(value) => svg_path = Some(value),
                None => {
                    eprintln!("\nErreur : --svg attend un chemin de fichier\n");
                    return ExitCode::FAILURE;
                }
            },
            "--help" | "-h" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("mecatool {VERSION}");
                return ExitCode::SUCCESS;
            }
            other => parts.push(other.to_string()),
        }
    }

    if parts.is_empty() {
        print_usage();
        return ExitCode::FAILURE;
    }

    let output = Output {
        expert,
        svg_path,
        true_to_scale,
        statistical,
    };
    match run(&parts.join(" "), window.as_deref(), &output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("\nErreur : {message}\n");
            ExitCode::FAILURE
        }
    }
}

/// Ce que l'utilisateur a demandé en sortie.
struct Output {
    expert: bool,
    svg_path: Option<String>,
    true_to_scale: bool,
    statistical: bool,
}

impl Output {
    fn diagram_mode(&self) -> DiagramMode {
        if self.true_to_scale {
            DiagramMode::TrueToScale
        } else {
            DiagramMode::Deviations
        }
    }
}

fn run(input: &str, window: Option<&str>, output: &Output) -> Result<(), String> {
    let expert = output.expert;
    let engine = Iso286Engine::new().map_err(|e| e.to_string())?;

    // Ces marques ne figurent dans aucune désignation ISO 286 : leur présence
    // dit sans ambiguïté de quelle sorte de calcul il s'agit.
    if input.contains('\u{b1}') || input.contains('=') || input.contains("+/-") {
        return run_chain(input, output);
    }
    if input.contains(',') {
        return run_comparison(&engine, input, window, output);
    }

    let parsed = parse(input).map_err(|e| e.to_string())?;
    let unit = parsed.unit();

    // La fenêtre de jeu, si elle est donnée, transforme le calcul en verdict.
    let requirement = match window {
        Some(text) => {
            let (min, max) = parse_clearance_window(text).map_err(|e| e.to_string())?;
            Some(ClearanceRequirement::new(parsed.nominal(), min, max).map_err(|e| e.to_string())?)
        }
        None => None,
    };

    match parsed {
        ParsedInput::Fit {
            nominal,
            hole,
            shaft,
            ..
        } => {
            let analysis = engine
                .fit(nominal, hole, shaft)
                .map_err(|e| e.to_string())?;
            report_fit(&analysis, nominal, unit, expert, requirement.as_ref());

            if let Some(path) = &output.svg_path {
                let diagram = fit_diagram(
                    &analysis.fit,
                    &DiagramOptions {
                        mode: output.diagram_mode(),
                        ..DiagramOptions::default()
                    },
                );
                std::fs::write(path, to_svg(&diagram))
                    .map_err(|e| format!("écriture de {path} impossible : {e}"))?;
                section("GRAPHIQUE");
                row("Fichier écrit", path);
                row(
                    "Échelle verticale",
                    &format!("1 µm = {:.1} px", diagram.pixels_per_micrometre),
                );
                println!("  {}", diagram.scale_note);
                println!();
            }
        }

        ParsedInput::Feature { nominal, class, .. } => {
            let analysis = engine.feature(nominal, class).map_err(|e| e.to_string())?;
            title(&format!(
                "Élément {} {}",
                trim_number(nominal.to_decimal_string(unit, unit.default_decimals())),
                class
            ));
            feature_block(&analysis.tolerance, unit);
            if expert {
                steps_block("POURQUOI ?", &analysis.steps);
            }
            standards_block(&analysis.provenance);
        }

        ParsedInput::NominalOnly { nominal, .. } => {
            let Some(requirement) = requirement else {
                return Err(format!(
                    "« {input} » ne donne qu'une dimension nominale. Ajoutez une classe de \
                     tolérance (par ex. \"{input} H7/g6\") pour calculer, ou une fenêtre de jeu \
                     (par ex. --jeu 10..30) pour rechercher les solutions possibles."
                ));
            };
            let result = find_fits(&engine, &requirement, &SearchOptions::default())
                .map_err(|e| e.to_string())?;
            report_search(&result, &requirement, nominal, unit);
        }
    }
    Ok(())
}

fn run_chain(input: &str, output: &Output) -> Result<(), String> {
    let links = parse_chain(input).map_err(|e| e.to_string())?;
    let analysis = analyse_chain(&links, output.statistical).map_err(|e| e.to_string())?;

    title(&format!("Chaîne de cotes — {} maillons", links.len()));

    section("MAILLONS");
    println!(
        "  {:<6} {:<20} {:>12} {:>10}  Part",
        "Repère", "Cote", "Tolérance", "Sens"
    );
    println!("  {}", "-".repeat(66));
    for contribution in &analysis.contributions {
        println!(
            "  {:<6} {:<20} {:>12} {:>10}  {}",
            contribution.link.label,
            contribution.designation,
            format!(
                "{} mm",
                trim_number(
                    contribution
                        .tolerance
                        .to_decimal_string(Unit::Millimetre, 4)
                )
            ),
            contribution.link.direction.label_fr(),
            contribution.share_label
        );
    }

    section("RÉSULTANTE");
    row("Cote", &format!("{} mm", analysis.designation()));
    row(
        "Maximale",
        &format!(
            "{} mm",
            analysis.limits.max().to_decimal_string(Unit::Millimetre, 3)
        ),
    );
    row(
        "Minimale",
        &format!(
            "{} mm",
            analysis.limits.min().to_decimal_string(Unit::Millimetre, 3)
        ),
    );
    row(
        "Tolérance (pire des cas)",
        &format!(
            "{} mm",
            analysis.tolerance.to_decimal_string(Unit::Millimetre, 3)
        ),
    );
    if let Some(dominant) = &analysis.dominant {
        row("Maillon dominant", dominant);
    }

    if let Some(estimate) = &analysis.statistical {
        section("ESTIMATION STATISTIQUE");
        println!("  {}", estimate.method);
        println!("  {}", estimate.summary);
        println!();
        println!("  Ce que cette estimation suppose :");
        for assumption in &estimate.assumptions {
            println!("    — {assumption}");
        }
    }

    if output.expert {
        steps_block("POURQUOI ?", &analysis.steps);
    }

    println!();
    Ok(())
}

fn run_comparison(
    engine: &Iso286Engine,
    input: &str,
    window: Option<&str>,
    output: &Output,
) -> Result<(), String> {
    let request = parse_comparison(input).map_err(|e| e.to_string())?;

    let requirement = match window {
        Some(text) => {
            let (min, max) = parse_clearance_window(text).map_err(|e| e.to_string())?;
            Some(ClearanceRequirement::new(request.nominal, min, max).map_err(|e| e.to_string())?)
        }
        None => None,
    };

    let options = DiagramOptions {
        mode: output.diagram_mode(),
        // Un comparatif tient mal dans la largeur d'un ajustement seul.
        width: 900.0,
        ..DiagramOptions::default()
    };

    let comparison = compare_fits(
        engine,
        request.nominal,
        &request.pairs,
        requirement.as_ref(),
        &options,
    )
    .map_err(|e| e.to_string())?;

    title(&format!(
        "Comparaison {} — {} ajustements",
        nominal_label(request.nominal),
        comparison.entries.len()
    ));

    section("VALEURS");
    let has_verdict = requirement.is_some();
    println!(
        "  {:<10} {:>10} {:>10} {:>11}  Type",
        "Ajustement", "Jeu min", "Jeu max", "Dispersion"
    );
    println!("  {}", "-".repeat(if has_verdict { 76 } else { 58 }));
    for entry in &comparison.entries {
        print!(
            "  {:<10} {:>10} {:>10} {:>11}  {}",
            entry.designation,
            plain_um(entry.fit.min_clearance),
            plain_um(entry.fit.max_clearance),
            plain_um(entry.span),
            entry.classification
        );
        match &entry.verification {
            Some(verification) => println!("  {}", verification.conclusion.headline()),
            None => println!(),
        }
    }

    section("CE QUE LE COMPARATIF MONTRE");
    println!("  {}", comparison.summary);
    println!();
    println!("  L'ordre est celui de votre saisie : MecaTool ne reclasse pas.");

    if let Some(path) = &output.svg_path {
        std::fs::write(path, to_svg(&comparison.diagram))
            .map_err(|e| format!("écriture de {path} impossible : {e}"))?;
        section("GRAPHIQUE");
        row("Fichier écrit", path);
        println!("  {}", comparison.diagram.scale_note);
        println!();
    }

    standards_block(&comparison.provenance);
    Ok(())
}

fn report_fit(
    analysis: &FitAnalysis,
    nominal: Length,
    unit: Unit,
    expert: bool,
    requirement: Option<&ClearanceRequirement>,
) {
    let fit = &analysis.fit;
    title(&format!(
        "Ajustement Ø{} {}",
        trim_number(nominal.to_decimal_string(unit, unit.default_decimals())),
        fit.designation()
    ));

    feature_block(&fit.hole, unit);
    feature_block(&fit.shaft, unit);

    section("AJUSTEMENT");
    row("Jeu minimum", &signed_um(fit.min_clearance));
    row("Jeu maximum", &signed_um(fit.max_clearance));
    row("Amplitude du jeu", &plain_um(fit.clearance_span()));
    if let Some(interference) = fit.max_interference() {
        row("Serrage maximum", &plain_um(interference));
    }

    section("CONCLUSION");
    println!("  {}", analysis.classification_fr());

    match requirement {
        // Sans exigence, on décrit ce que l'ajustement produit sans juger.
        None => println!("  {}", classification_conclusion(analysis).detail),
        Some(requirement) => match verify_clearance(fit, requirement) {
            Ok(verification) => {
                println!();
                println!("  Besoin exprimé          {}", requirement.window_fr());
                println!(
                    "  Plage obtenue           {} à {}",
                    plain_um(fit.min_clearance),
                    plain_um(fit.max_clearance)
                );
                println!();
                println!("  {}", verification.conclusion.headline());
                println!("  {}", verification.conclusion.detail);
                if expert {
                    steps_block("POURQUOI ?", &verification.conclusion.why);
                }
            }
            Err(error) => println!("  Vérification impossible : {error}"),
        },
    }

    if expert {
        steps_block("DÉTAIL — AJUSTEMENT", &analysis.fit_steps);
        steps_block(
            &format!("DÉTAIL — ALÉSAGE {}", fit.hole.class),
            &analysis.hole_steps,
        );
        steps_block(
            &format!("DÉTAIL — ARBRE {}", fit.shaft.class),
            &analysis.shaft_steps,
        );
    }

    standards_block(&analysis.provenance);
}

fn report_search(
    result: &SearchResult,
    requirement: &ClearanceRequirement,
    nominal: Length,
    unit: Unit,
) {
    title(&format!(
        "Recherche pour Ø{}, jeu {}",
        trim_number(nominal.to_decimal_string(unit, unit.default_decimals())),
        requirement.window_fr()
    ));

    if result.is_empty() {
        section("AUCUNE SOLUTION");
        println!("  Aucun ajustement normalisé du périmètre exploré ne répond à ce besoin.");
        if let Some(diagnosis) = result.diagnosis_fr(requirement) {
            println!();
            println!("  {diagnosis}");
        }
        perimeter_block(result);
        standards_block(&result.provenance);
        return;
    }

    let compatible = result.fully_compatible().count();
    let partial = result.solutions.len() - compatible;

    // Une absence de solution exacte mérite une explication, pas seulement une
    // liste de solutions approchantes.
    if compatible == 0 {
        section("AUCUNE SOLUTION EXACTE");
        if let Some(diagnosis) = result.diagnosis_fr(requirement) {
            println!("  {diagnosis}");
        }
    }

    section(&format!(
        "SOLUTIONS — {compatible} compatible(s), {partial} partielle(s) sur {} combinaisons examinées",
        result.examined
    ));
    println!(
        "  {:<10} {:>10} {:>10} {:>11}  Verdict",
        "Ajustement", "Jeu min", "Jeu max", "Dispersion"
    );
    println!("  {}", "-".repeat(58));
    for solution in result.solutions.iter().take(20) {
        println!(
            "  {:<10} {:>10} {:>10} {:>11}  {} {}",
            solution.designation(),
            plain_um(solution.fit.min_clearance),
            plain_um(solution.fit.max_clearance),
            plain_um(solution.total_tolerance),
            solution.verification.verdict.badge(),
            solution.verification.verdict.headline_fr(),
        );
    }
    if result.solutions.len() > 20 {
        println!("  ... et {} autres.", result.solutions.len() - 20);
    }

    if let Some(best) = result.recommended() {
        let compatible = best.verification.verdict == mecatool_core::Verdict::Compatible;
        section(if compatible {
            "RECOMMANDATION"
        } else {
            "SOLUTION LA PLUS PROCHE"
        });
        println!("  {}", best.designation());
        println!("  {}", best.verification.conclusion.detail);
        println!();
        if compatible {
            println!(
                "  Retenue parce qu'elle satisfait le besoin avec la dispersion la plus large"
            );
            println!("  du classement, donc la plus facile à tenir en fabrication.");
        } else {
            println!("  Aucune solution du périmètre ne satisfait entièrement le besoin. Celle-ci");
            println!(
                "  s'en approche le plus, avec {} de dépassement cumulé.",
                plain_um(best.violation())
            );
        }
    }

    if let (Some(tightest), Some(widest)) = (result.tightest(), result.widest()) {
        if tightest.designation() != widest.designation() {
            section("EXTRÊMES");
            row(
                "La plus serrée",
                &format!(
                    "{} — dispersion {}",
                    tightest.designation(),
                    plain_um(tightest.total_tolerance)
                ),
            );
            row(
                "La plus large",
                &format!(
                    "{} — dispersion {}",
                    widest.designation(),
                    plain_um(widest.total_tolerance)
                ),
            );
        }
    }

    perimeter_block(result);
    standards_block(&result.provenance);
}

/// Rappelle ce qui a été exploré : une absence de résultat doit se lire
/// « aucune solution dans ce périmètre », jamais « aucune solution n'existe ».
fn perimeter_block(result: &SearchResult) {
    section("PÉRIMÈTRE DE LA RECHERCHE");
    for note in &result.notes {
        println!("  {note}");
    }
}

fn feature_block(tolerance: &FeatureTolerance, unit: Unit) {
    let heading = match tolerance.feature {
        Feature::Hole => format!("ALÉSAGE {}", tolerance.class),
        Feature::Shaft => format!("ARBRE {}", tolerance.class),
    };
    section(&heading);

    let (lower_symbol, upper_symbol) = (
        mecatool_core::Deviations::lower_symbol(tolerance.feature),
        mecatool_core::Deviations::upper_symbol(tolerance.feature),
    );

    row(
        "Dimension nominale",
        &format!("{} {}", tolerance.nominal.to_decimal_string(unit, 3), unit),
    );
    row(
        &format!("Écart inférieur {lower_symbol}"),
        &signed_um(tolerance.deviations.lower()),
    );
    row(
        &format!("Écart supérieur {upper_symbol}"),
        &signed_um(tolerance.deviations.upper()),
    );
    row(
        "Dimension minimale",
        &format!(
            "{} {}",
            tolerance.limits.min().to_decimal_string(unit, 3),
            unit
        ),
    );
    row(
        "Dimension maximale",
        &format!(
            "{} {}",
            tolerance.limits.max().to_decimal_string(unit, 3),
            unit
        ),
    );
    row(
        &format!("Tolérance {}", tolerance.class.grade.name()),
        &plain_um(tolerance.it),
    );
    row(
        "Échelon des tables",
        &format!("{} mm", tolerance.size_range.label_fr()),
    );
}

fn steps_block(heading: &str, steps: &[ReasoningStep]) {
    if steps.is_empty() {
        return;
    }
    section(heading);
    for step in steps {
        println!("  {}", step.label);
        if let Some(expression) = &step.expression {
            println!("      {expression}");
        }
        if let Some(value) = &step.value {
            println!("      = {value}");
        }
    }
}

fn standards_block(provenance: &mecatool_core::Provenance) {
    section("NORMES UTILISÉES");
    for reference in &provenance.references {
        println!("  {}", reference.labelled_citation());
        println!("      {}", reference.title);
    }
    let warnings = provenance.warnings_fr();
    if !warnings.is_empty() {
        println!();
        for warning in warnings {
            println!("  /!\\  {warning}");
        }
    }
    println!();
}

// Pas de codes ANSI : ils ne sont pas interprétés partout sous Windows, et un
// tableau de cotes doit rester lisible même redirigé vers un fichier.
fn title(text: &str) {
    println!("\nMECATOOL {VERSION} — {text}");
    println!("{}", "=".repeat(64));
}

fn section(heading: &str) {
    println!("\n{heading}");
    println!("{}", "-".repeat(heading.chars().count()));
}

fn row(label: &str, value: &str) {
    println!("  {label:<26} {value}");
}

fn plain_um(value: Length) -> String {
    format!(
        "{} {}",
        trim_number(value.to_decimal_string(Unit::Micrometre, 1)),
        Unit::Micrometre
    )
}

fn signed_um(value: Length) -> String {
    if value.is_zero() {
        return "0".to_string();
    }
    let body = plain_um(value);
    if value.is_positive() {
        format!("+{body}")
    } else {
        body
    }
}

fn trim_number(rendered: String) -> String {
    if rendered.contains('.') {
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        rendered
    }
}

fn print_usage() {
    println!(
        "\nMecaTool {VERSION} — vérification du moteur de tolérancement

USAGE
  mecatool [OPTIONS] <DÉSIGNATION>

OPTIONS
  -j, --jeu MIN..MAX   fenêtre de jeu voulue, en µm sauf unité précisée
  -x, --expert         affiche le détail du calcul, formules comprises
      --stat           joint l'estimation statistique RSS à une chaîne de cotes
      --svg FICHIER    écrit le graphique des zones de tolérance
      --fidele         graphique à l'échelle réelle de la pièce
  -h, --help           affiche cette aide
  -V, --version        affiche la version

CALCULER
  mecatool \"Ø10 H7/g6\"
  mecatool --expert \"Ø20 H7/k6\"
  mecatool \"25 H7\"
  mecatool \"1 in H7/g6\"

VÉRIFIER — est-ce que cet ajustement convient ?
  mecatool \"Ø20 H7/g6\" --jeu 10..30
  mecatool \"Ø20 H7/g6\" --jeu 0.01..0.03mm

TROUVER — quels ajustements répondent à mon besoin ?
  mecatool \"Ø20\" --jeu 10..30
  mecatool \"Ø20\" --jeu 5..

COMPARER — laquelle de ces solutions choisir ?
  mecatool \"Ø20 H7/g6, H7/h6, H7/k6, H7/p6\"
  mecatool \"Ø20 H7/g6, H7/k6\" --jeu 5..50 --svg comparatif.svg

CHAÎNE DE COTES — quelle sera la dimension résultante ?
  mecatool \"A = 20 ±0.1; B = 10 ±0.05; -C = 5 ±0.02\"
  mecatool \"A = 20 ±0.1; B = 10 ±0.05\" --stat

Ce binaire est un outil de vérification du moteur, pas le produit fini.
"
    );
}

#[cfg(windows)]
fn enable_utf8_console() {
    // Sans cela, la console Windows rend les accents et les pastilles en
    // caracteres parasites selon la page de codes active.
    extern "system" {
        fn SetConsoleOutputCP(code: u32) -> i32;
    }
    unsafe {
        SetConsoleOutputCP(65001);
    }
}

#[cfg(not(windows))]
fn enable_utf8_console() {}
