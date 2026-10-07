import { describe, expect, it } from "vitest";

import { deviation, mm, nominal, um, umFine, umLabel } from "./format";

describe("mise en forme des longueurs", () => {
  it("écrit les cotes en millimètres comme sur un plan", () => {
    expect(mm(10_015_000)).toBe("10.015");
    expect(mm(9_986_000)).toBe("9.986");
    expect(mm(10_000_000)).toBe("10.000");
    expect(mm(0)).toBe("0.000");
  });

  it("ne laisse pas le flottant polluer la conversion", () => {
    // 10 015 000 nm / 1 000 000 vaut 10,014999999999999 en flottant naïf.
    expect(mm(10_015_000)).toBe("10.015");
    // Une chaîne de valeurs consécutives doit rester exacte.
    for (let i = 0; i < 1000; i += 1) {
      const nm = 20_000_000 + i * 1_000;
      expect(mm(nm)).toBe(`20.${String(i).padStart(3, "0")}`);
    }
  });

  it("arrondit la moitié à l'opposé de zéro, comme le moteur", () => {
    // 0,0005 mm sur 3 décimales : 0,001, et non 0,000.
    expect(mm(500, 3)).toBe("0.001");
    // Le cas qui distingue cette convention de Math.round : le négatif.
    expect(mm(-500, 3)).toBe("-0.001");
    expect(mm(-1_500, 3)).toBe("-0.002");
  });

  it("écrit les micromètres sans décimale superflue", () => {
    expect(um(15_000)).toBe("15");
    expect(um(10_500)).toBe("10.5");
    expect(um(0)).toBe("0");
    expect(umLabel(15_000)).toBe("15 µm");
  });

  it("écrit les écarts avec leur signe, et le zéro sans signe", () => {
    expect(deviation(15_000)).toBe("+15 µm");
    expect(deviation(-5_000)).toBe("-5 µm");
    expect(deviation(-14_000)).toBe("-14 µm");
    expect(deviation(0)).toBe("0");
    // js7 à Ø20 : ± 10,5 µm, une demi-valeur qui doit rester exacte.
    expect(deviation(10_500)).toBe("+10.5 µm");
    expect(deviation(-10_500)).toBe("-10.5 µm");
  });

  it("écrit les dimensions nominales sans zéro superflu", () => {
    expect(nominal(20_000_000)).toBe("20");
    expect(nominal(25_400_000)).toBe("25.4");
    expect(nominal(10_000_000)).toBe("10");
  });

  it("écrit les rugosités fines sans les écraser à zéro", () => {
    // Ra 0,025 arrondi au dixième deviendrait « 0 » : faux, et d'apparence exacte.
    expect(umFine(25)).toBe("0.025");
    expect(umFine(1_600)).toBe("1.6");
    expect(umFine(50_000)).toBe("50");
  });
});
