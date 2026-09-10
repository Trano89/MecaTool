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

import {
  NM_PER_UM,
  type EngineInfo,
  type FeatureReport,
  type FitReport,
  type SearchReport,
} from "./types";

// Le contrôle de forme se joue ici, à la compilation.
const fit = fitFixture as FitReport;
const feature = featureFixture as FeatureReport;
const search = searchFixture as SearchReport;
const engine = engineFixture as EngineInfo;

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
