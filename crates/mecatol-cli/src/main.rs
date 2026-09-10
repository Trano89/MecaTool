//! Outil de vérification en ligne de commande.
//!
//! Ce binaire n'est pas le produit : c'est le banc d'essai du moteur. Il permet
//! de confronter à la main les résultats de Mecatol aux tables d'un manuel ou
//! d'une norme, sans passer par l'interface graphique.
//!
//! ```text
//!   mecatol "Ø10 H7/g6"
//!   mecatol --expert "Ø20 H7/k6"
//!   mecatol "25 H7"
//! ```

use std::process::ExitCode;

use mecatol_core::{Feature, FeatureTolerance, Length, ReasoningStep, Unit};
use mecatol_engine::iso286::{classification_conclusion, FitAnalysis, Iso286Engine};
use mecatol_engine::parser::{parse, ParsedInput};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    enable_utf8_console();

    let mut expert = false;
    let mut parts: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--expert" | "-x" => expert = true,
            "--help" | "-h" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("mecatol {VERSION}");
                return ExitCode::SUCCESS;
            }
            other => parts.push(other.to_string()),
        }
    }

    if parts.is_empty() {
        print_usage();
        return ExitCode::FAILURE;
    }

    match run(&parts.join(" "), expert) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("\nErreur : {message}\n");
            ExitCode::FAILURE
        }
    }
}

fn run(input: &str, expert: bool) -> Result<(), String> {
    let engine = Iso286Engine::new().map_err(|e| e.to_string())?;
    let parsed = parse(input).map_err(|e| e.to_string())?;
    let unit = parsed.unit();

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
            report_fit(&analysis, nominal, unit, expert);
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
    }
    Ok(())
}

fn report_fit(analysis: &FitAnalysis, nominal: Length, unit: Unit, expert: bool) {
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

    let conclusion = classification_conclusion(analysis);
    section("CONCLUSION");
    println!("  {}", analysis.classification_fr());
    println!("  {}", conclusion.detail);

    if expert {
        steps_block("POURQUOI ?", &analysis.fit_steps);
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

fn feature_block(tolerance: &FeatureTolerance, unit: Unit) {
    let heading = match tolerance.feature {
        Feature::Hole => format!("ALÉSAGE {}", tolerance.class),
        Feature::Shaft => format!("ARBRE {}", tolerance.class),
    };
    section(&heading);

    let (lower_symbol, upper_symbol) = (
        mecatol_core::Deviations::lower_symbol(tolerance.feature),
        mecatol_core::Deviations::upper_symbol(tolerance.feature),
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

fn standards_block(provenance: &mecatol_core::Provenance) {
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
    println!("\nMECATOL {VERSION} — {text}");
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
        "\nMecatol {VERSION} — vérification du moteur de tolérancement

USAGE
  mecatol [OPTIONS] <DÉSIGNATION>

OPTIONS
  -x, --expert     affiche le détail du calcul, formules comprises
  -h, --help       affiche cette aide
  -V, --version    affiche la version

EXEMPLES
  mecatol \"Ø10 H7/g6\"
  mecatol --expert \"Ø20 H7/k6\"
  mecatol \"25 H7\"
  mecatol \"1 in H7/g6\"

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
