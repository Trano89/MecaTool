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
import chainFixture from "./fixtures/chain-report.json";
import catalogueFixture from "./fixtures/geometric-catalogue.json";
import geometryFixture from "./fixtures/geometric-group.json";
import domainsFixture from "./fixtures/domains.json";
import bearingCatalogueFixture from "./fixtures/bearing-catalogue.json";
import bearingAdviceFixture from "./fixtures/bearing-advice.json";

import {
  NM_PER_UM,
  type ChainReport,
  type ClassComparison,
  type EngineInfo,
  type FeatureReport,
  type FitComparison,
  type FitReport,
  type GeometricCatalogue,
  type BearingCatalogue,
  type Domain,
  type GroupAnalysis,
  type MountingAdvice,
  type SearchReport,
} from "./types";

// Le contrôle de forme se joue ici, à la compilation.
const fit = fitFixture as FitReport;
const feature = featureFixture as FeatureReport;
const search = searchFixture as SearchReport;
const engine = engineFixture as EngineInfo;
const general = generalFixture as ClassComparison;
const comparison = comparisonFixture as FitComparison;
const chain = chainFixture as ChainReport;
const catalogue = catalogueFixture as GeometricCatalogue;
const geometry = geometryFixture as GroupAnalysis;
const registry = domainsFixture as Domain[];
const bearings = bearingCatalogueFixture as BearingCatalogue;
const advice = bearingAdviceFixture as MountingAdvice;

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

  it("décrit une chaîne de cotes", () => {
    // 20 + 10 − 5 = 25.
    expect(chain.analysis.nominal).toBe(25 * 1_000_000);
    expect(chain.designation).toBe("25 ± 0.17");

    // L'identité qui structure le module : la tolérance résultante vaut la
    // somme de toutes les tolérances, quel que soit le sens des maillons.
    const sum = chain.analysis.contributions.reduce(
      (total, contribution) => total + contribution.tolerance,
      0,
    );
    expect(chain.analysis.tolerance).toBe(sum);
    expect(chain.analysis.limits.max - chain.analysis.limits.min).toBe(sum);

    // Un maillon diminuant est bien présent dans l'échantillon.
    const directions = chain.analysis.contributions.map((c) => c.link.direction);
    expect(directions).toContain("decreasing");

    // Le graphique porte une barre par maillon, et un seul dominant.
    expect(chain.chart.bars).toHaveLength(chain.analysis.contributions.length);
    expect(chain.chart.bars.filter((bar) => bar.dominant)).toHaveLength(1);
  });

  it("joint ses hypothèses à toute estimation statistique", () => {
    const estimate = chain.analysis.statistical;
    expect(estimate).not.toBeNull();
    // Le RSS est plus optimiste que le pire des cas — c'est bien pour cela
    // qu'il ne doit jamais circuler sans ses hypothèses.
    expect(estimate!.tolerance).toBeLessThan(chain.analysis.tolerance);
    expect(estimate!.assumptions.length).toBeGreaterThan(0);
    expect(estimate!.assumptions.join(" ")).toMatch(/n'est vérifiée par MecaTool/);
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

  it("décrit le catalogue des caractéristiques géométriques", () => {
    expect(catalogue.families).toHaveLength(4);
    expect(catalogue.characteristics.length).toBeGreaterThan(0);

    // Le classement de la source, porté par les données et non par l'interface :
    // aucune tolérance de forme ne prend de référence, aucune des trois autres
    // familles ne s'en passe.
    for (const characteristic of catalogue.characteristics) {
      if (characteristic.class === "form") {
        expect(characteristic.datum).toBe("none");
      } else {
        expect(characteristic.datum).not.toBe("none");
      }
    }

    // L'emboîtement va dans un seul sens.
    const form = catalogue.families.find((f) => f.id === "form");
    const location = catalogue.families.find((f) => f.id === "location");
    expect(form?.limits).toEqual([]);
    expect(location?.limits).toContain("form");
    expect(location?.limits).toContain("orientation");

    // La donnée est secondaire : la réserve doit accompagner le catalogue.
    expect(catalogue.warnings).toHaveLength(1);
    expect(catalogue.provenance.references[0]?.verification.state).toBe("secondary");

    // Et aucune valeur de tolérance ne doit traverser : l'ISO 1101 n'en donne pas.
    for (const characteristic of catalogue.characteristics) {
      expect(characteristic.zones.length).toBeGreaterThan(0);
      expect(Object.keys(characteristic)).not.toContain("value");
    }
  });

  it("décrit le contrôle de plusieurs spécifications", () => {
    expect(geometry.specs).toHaveLength(3);

    // Une référence manque sur la troisième : le verdict du groupe doit s'en
    // ressentir, et le libellé mener par la faute plutôt que par le recouvrement.
    expect(geometry.conclusion.verdict).toBe("incompatible");
    expect(geometry.conclusion.detail).toMatch(/^1 spécification fautive/);

    const codes = geometry.specs.flatMap((s) => s.findings.map((f) => f.code));
    expect(codes).toContain("datum_missing");

    // Chaque recouvrement nomme les deux spécifications en cause, dans l'ordre :
    // la bornante d'abord, la bornée ensuite.
    expect(geometry.overlaps.length).toBeGreaterThan(0);
    const designations = geometry.specs.map((s) => s.designation);
    for (const overlap of geometry.overlaps) {
      expect(designations).toContain(overlap.wider);
      expect(designations).toContain(overlap.narrower);
      expect(overlap.wider).not.toBe(overlap.narrower);
    }

    // La valeur reste un entier de nanomètres jusqu'au bout.
    expect(geometry.specs[0]!.spec.value).toBe(20 * 1000);
  });

  it("décrit le registre des domaines", () => {
    expect(registry.length).toBeGreaterThanOrEqual(10);

    // Les identifiants routent : deux domaines homonymes feraient afficher
    // l'un pour l'autre.
    const ids = registry.map((domain) => domain.id);
    expect(new Set(ids).size).toBe(ids.length);

    for (const domain of registry) {
      // Le nom nomme, la question aide à choisir.
      expect(domain.question.endsWith("?")).toBe(true);
      // Le libellé de groupe vient du moteur : sans lui, l'interface tiendrait
      // sa propre table de traduction.
      expect(domain.group_label.length).toBeGreaterThan(0);
      // Un domaine bloqué dit pourquoi ; un domaine disponible ne le fait pas.
      expect(domain.status === "blocked").toBe(domain.unavailable !== null);
    }

    // L'état se déduit des sources : l'ISO 286 est confrontée, l'ISO 1101 non.
    expect(registry.find((d) => d.id === "fit")?.status).toBe("ready");
    expect(registry.find((d) => d.id === "geometry")?.status).toBe("reserved");
    expect(registry.find((d) => d.id === "bearing")?.status).toBe("reserved");
  });

  it("décrit le conseil de montage d'un roulement", () => {
    expect(advice.class).toBe("k5");
    expect(advice.designation).toBe("Ø50 k5");

    // La composition : le domaine roulements s'arrête à la classe, le domaine
    // ajustements rend les écarts. Les deux arrivent dans le même rapport.
    expect(advice.shaft.tolerance.deviations.lower).toBe(2 * NM_PER_UM);
    expect(advice.shaft.tolerance.deviations.upper).toBe(13 * NM_PER_UM);
    expect(advice.bore).toBe(50 * 1_000_000);

    // Et la provenance garde les deux natures de source distinctes : une
    // recommandation de fabricant pour la classe, une norme confrontée pour
    // les écarts. Les confondre tromperait dans un sens ou dans l'autre.
    const states = advice.provenance.references.map(
      (reference) => reference.verification.state,
    );
    expect(states).toContain("verified");
    expect(states).toContain("recommended");

    expect(advice.conclusion.warnings.join(" ")).toMatch(/sans caractère normatif/);
  });

  it("porte l'ajustement dans la convention signée du moteur", () => {
    // L'alésage d'un roulement n'est pas h0 : classe Normale à 50 mm, il mesure
    // 0 / −12 µm. C'est ce que la classe k5 rencontre.
    expect(advice.bearing_bore.deviations.upper).toBe(0);
    expect(advice.bearing_bore.deviations.lower).toBe(-12 * NM_PER_UM);
    expect(advice.bearing_bore.tolerance_class).toBe("Normale");

    // La grandeur portée est le JEU signé, jamais le serrage : un serrage
    // garanti se lit donc sur deux bornes négatives. Si le moteur inversait un
    // jour sa convention, cette assertion tomberait avant l'écran.
    expect(advice.fit.kind).toBe("interference");
    expect(advice.fit.min_clearance).toBe(-25 * NM_PER_UM);
    expect(advice.fit.max_clearance).toBe(-2 * NM_PER_UM);
    expect(advice.fit.min_clearance).toBeLessThan(advice.fit.max_clearance);

    // Et la phrase à afficher vient du moteur, pas de l'écran.
    expect(advice.fit.summary).toBe("serrage de 2 µm à 25 µm");
  });

  it("décrit le catalogue des roulements", () => {
    expect(bearings.families).toHaveLength(4);
    expect(bearings.regimes).toHaveLength(2);
    expect(bearings.cases.length).toBeGreaterThan(0);

    // Chaque régime explique ce qu'il impose : c'est ce qui permet de juger si
    // l'on peut s'en écarter.
    for (const regime of bearings.regimes) {
      expect(regime.explanation.length).toBeGreaterThan(0);
    }

    // Deux réserves, et elles ne disent pas la même chose. La règle du symbole
    // d'alésage vient d'un recueil qui reproduit l'ISO 15 ; le tableau de
    // montage ne vient d'aucune norme. Les fondre en un seul avertissement
    // ferait passer l'une pour l'autre.
    expect(bearings.warnings).toHaveLength(2);
    const joined = bearings.warnings.join(" ");
    expect(joined).toMatch(/recueil technique/);
    expect(joined).toMatch(/sans caractère normatif/);
  });

  it("décrit l'état de vérification de ses données", () => {
    expect(engine.available_letters).toContain("g");
    expect(engine.max_nominal_mm).toBe("500");
    expect(engine.provenance.references.length).toBeGreaterThan(0);

    // Le bandeau apparaît si et seulement si une source n'est pas confrontée à
    // la norme elle-même. L'équivalence porte sur « pas pleinement vérifiée »,
    // et non sur un état précis : une source secondaire mérite le bandeau tout
    // autant qu'une source non saisie, et pour une raison différente.
    const reserved = engine.provenance.references.filter(
      (reference) => reference.verification.state !== "verified",
    );
    expect(engine.warnings.length > 0).toBe(reserved.length > 0);

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
