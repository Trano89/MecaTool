/**
 * Vérifie que les types TypeScript décrivent bien ce que sérialise le moteur.
 *
 * Les échantillons sont produits par un test Rust
 * (`exporte_les_echantillons_pour_typescript`). Les affecter à leurs types
 * force `tsc` à comparer les deux descriptions : un champ renommé, supprimé ou
 * de type différent côté Rust casse la compilation, pas seulement l'exécution.
 *
 * Les assertions ci-dessous ajoutent le contrôle que le typage seul ne fait pas :
 * les valeurs elles-mêmes sont cohérentes.
 */

import { describe, expect, it } from "vitest";

import fitFixture from "./fixtures/fit-report.json";
import featureFixture from "./fixtures/feature-report.json";
import searchFixture from "./fixtures/search-report.json";
import engineFixture from "./fixtures/engine-info.json";
import generalFixture from "./fixtures/general-tolerances.json";
import comparisonFixture from "./fixtures/fit-comparison.json";

import {
  NM_PER_UM,
  type ClassComparison,
  type EngineInfo,
  type FeatureReport,
  type FitComparison,
  type FitReport,
  type SearchReport,
} from "./types";

// Le contrôle de forme se joue ici, à la compilation.
const fit = fitFixture as FitReport;
const feature = featureFixture as FeatureReport;
const search = searchFixture as SearchReport;
const engine = engineFixture as EngineInfo;
const general = generalFixture as ClassComparison;
const comparison = comparisonFixture as FitComparison;

describe("échantillons du moteur", () => {
  it("décrit un ajustement Ø10 H7/g6 exact", () => {
    expect(fit.kind).toBe("fit");
    expect(fit.analysis.fit.hole.class.grade).toBe("IT7");
    expect(fit.analysis.fit.shaft.class.letter).toBe("G");

    // Les longueurs arrivent en nanomètres entiers, jamais en millimètres flottants.
    expect(fit.analysis.fit.hole.limits.max).toBe(10_015_000);
    expect(fit.analysis.fit.shaft.limits.min).toBe(9_986_000);
    expect(fit.analysis.fit.min_clearance).toBe(5 * NM_PER_UM);
    expect(fit.analysis.fit.max_clearance).toBe(29 * NM_PER_UM);
    expect(fit.analysis.fit.kind).toBe("clearance");
  });

  it("porte un verdict quand une exigence est fournie", () => {
    expect(fit.verification).not.toBeNull();
    expect(fit.verification?.verdict).toBe("compatible");
    expect(fit.verification?.conclusion.why.length).toBeGreaterThan(0);
  });

  it("fournit un diagramme cohérent avec les valeurs", () => {
    const { diagram } = fit;
    expect(diagram.bands).toHaveLength(2);
    expect(diagram.clearances).toHaveLength(2);

    // La promesse du graphique : chaque cote mesure sa valeur à l'écran.
    for (const marker of diagram.clearances) {
      const measured = marker.shaft_y - marker.hole_y;
      const expected = (marker.value / NM_PER_UM) * diagram.pixels_per_micrometre;
      expect(measured).toBeCloseTo(expected, 6);
    }

    // L'annonce d'échelle ne doit jamais manquer.
    expect(diagram.scale_note.length).toBeGreaterThan(0);
  });

  it("décrit un élément seul", () => {
    expect(feature.kind).toBe("feature");
    expect(feature.analysis.tolerance.class.grade).toBe("IT7");
    expect(feature.analysis.steps.length).toBeGreaterThan(0);
  });

  it("décrit une recherche, périmètre compris", () => {
    expect(search.kind).toBe("search");
    expect(search.result.notes).toHaveLength(3);
    // Ce scénario n'a pas de solution exacte : le diagnostic doit l'expliquer.
    expect(search.diagnosis).not.toBeNull();
    expect(search.requirement.min_clearance).toBe(10 * NM_PER_UM);
  });

  it("décrit un comparatif d'ajustements", () => {
    expect(comparison.entries).toHaveLength(4);
    // L'ordre de saisie est conservé jusque dans l'échantillon.
    expect(comparison.entries.map((entry) => entry.designation)).toEqual([
      "H7/g6",
      "H7/h6",
      "H7/k6",
      "H7/p6",
    ]);

    // Une échelle unique : deux zones par ajustement, un seul facteur.
    expect(comparison.diagram.bands).toHaveLength(8);
    expect(comparison.diagram.pixels_per_micrometre).toBeGreaterThan(0);

    // Chaque zone sait à quel ajustement elle appartient.
    for (const band of comparison.diagram.bands) {
      expect(band.group).not.toBeNull();
    }

    // Le comparatif ne trace pas de cotes de jeu : elles sont dans le tableau.
    expect(comparison.diagram.clearances).toHaveLength(0);

    // L'exigence fournie donne un verdict par ligne.
    expect(comparison.requirement).not.toBeNull();
    for (const entry of comparison.entries) {
      expect(entry.verification).not.toBeNull();
    }
  });

  it("décrit les tolérances générales des quatre classes", () => {
    expect(general.kind).toBe("linear");
    expect(general.nominal).toBe(2 * 1_000_000);
    expect(general.rows).toHaveLength(4);

    // Les classes vont du plus fin au plus grossier.
    expect(general.rows.map((row) => row.symbol)).toEqual(["f", "m", "c", "v"]);

    // Une classe est soit définie, soit expliquée — jamais ni l'un ni l'autre.
    for (const row of general.rows) {
      expect(row.deviation === null).toBe(row.unavailable !== null);
      expect(row.deviation === null).toBe(row.deviation_label === null);
    }

    // À 2 mm, la norme ne définit pas la classe v.
    const veryCoarse = general.rows.find((row) => row.symbol === "v");
    expect(veryCoarse?.deviation).toBeNull();
    expect(veryCoarse?.unavailable).toMatch(/ne définit pas/);
  });

  it("décrit l'état de vérification de ses données", () => {
    expect(engine.available_letters).toContain("g");
    expect(engine.max_nominal_mm).toBe("500");
    expect(engine.provenance.references.length).toBeGreaterThan(0);

    const unverified = engine.provenance.references.filter(
      (reference) => reference.verification.state === "unverified",
    );

    // Le bandeau apparaît si et seulement si une source n'est pas vérifiée.
    expect(engine.warnings.length > 0).toBe(unverified.length > 0);

    // Une source déclarée vérifiée doit dire contre quoi : « vérifié » sans
    // référence ne vaudrait pas mieux que « non vérifié ».
    for (const reference of engine.provenance.references) {
      if (reference.verification.state === "verified") {
        expect(reference.verification.against).toMatch(/ISO/);
        expect(reference.verification.on).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      }
    }
  });
});
