/**
 * Vérifie l'écran des matières.
 *
 * Ce qui compte :
 *
 *  1. les réserves se lisent avant toute saisie ;
 *  2. **rien ne se tape qui puisse se choisir** : les nuances embarquées,
 *     l'épaisseur, la famille et l'écart de température sont des boutons dont
 *     les options viennent du moteur ;
 *  3. la limite d'élasticité suit l'épaisseur choisie ;
 *  4. l'ajustement à chaud montre le jeu à froid et à chaud, et ce qui change.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import catalogueFixture from "../fixtures/materials-catalogue.json";
import readingFixture from "../fixtures/steel-reading.json";
import thermalFixture from "../fixtures/thermal-fit.json";
import classesFixture from "../fixtures/tolerance-classes.json";
import type { ClassCatalogue, MaterialsCatalogue, SteelReading, ThermalFit } from "../types";
import { Materials } from "./Materials";

vi.mock("../api", () => ({
  materialsCatalogue: vi.fn(),
  materialsRead: vi.fn(),
  thermalFit: vi.fn(),
  toleranceClasses: vi.fn(),
  isDesktop: () => true,
}));

const { materialsCatalogue, materialsRead, thermalFit, toleranceClasses } = await import("../api");

const catalogue = catalogueFixture as MaterialsCatalogue;
const reading = readingFixture as SteelReading;
const thermal = thermalFixture as ThermalFit;

async function show() {
  vi.mocked(materialsCatalogue).mockResolvedValue(catalogue);
  vi.mocked(materialsRead).mockResolvedValue(reading);
  vi.mocked(thermalFit).mockResolvedValue(thermal);
  vi.mocked(toleranceClasses).mockResolvedValue(classesFixture as ClassCatalogue);
  render(<Materials />);
  await waitFor(() => expect(materialsCatalogue).toHaveBeenCalled());
}

describe("écran des matières", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("montre l'origine des données avant toute saisie", async () => {
    await show();
    const notes = await screen.findAllByRole("note");
    expect(notes).toHaveLength(catalogue.warnings.length);
    expect(notes.map((n) => n.textContent ?? "").join(" ")).toMatch(/non vérifiée/);
  });

  it("propose les nuances embarquées en boutons, et les lit d'un clic", async () => {
    await show();
    const grades = await screen.findByRole("group", { name: "Aciers de construction embarqués" });
    const expected = catalogue.structural_grades.flatMap((grade) => grade.qualities).length;
    expect(within(grades).getAllByRole("button")).toHaveLength(expected);

    await userEvent.click(within(grades).getByRole("button", { name: /S355J2$/ }));
    await waitFor(() => expect(materialsRead).toHaveBeenCalledWith("S355J2"));
    expect(await screen.findByText("Lecture de la désignation")).toBeInTheDocument();
  });

  it("donne la limite d'élasticité de l'épaisseur choisie par bouton", async () => {
    await show();
    await userEvent.click(screen.getByRole("button", { name: "Lire la désignation" }));
    const bands = await screen.findByRole("group", { name: "Épaisseur nominale" });
    const rows = reading.structural!.rows;

    expect(screen.getByText(`${rows[0]!.yield_mpa} MPa`)).toBeInTheDocument();
    await userEvent.click(within(bands).getAllByRole("button")[1]!);
    expect(screen.getByText(`${rows[1]!.yield_mpa} MPa`)).toBeInTheDocument();
  });

  it("choisit les matières et l'écart de température par boutons", async () => {
    await show();
    const delta = await screen.findByRole("group", { name: "Écart de température" });
    await userEvent.click(within(delta).getByRole("button", { name: /\+100 K/ }));
    const hole = screen.getByRole("group", { name: "Matière de l'alésage" });
    await userEvent.click(within(hole).getByRole("button", { name: /alliage de titane/ }));

    await userEvent.click(screen.getByRole("button", { name: "Calculer à chaud" }));
    await waitFor(() => expect(thermalFit).toHaveBeenCalled());
    const request = vi.mocked(thermalFit).mock.calls[0]![0];
    expect(request.deltaT).toBe("100");
    expect(request.holeFamily).toBe("titanium");
    expect(request.shaftFamily).toBe("steel");
    expect(request.holeClass).toBe("H7");
  });

  it("montre le jeu à froid et à chaud, et le verdict par un libellé", async () => {
    await show();
    await userEvent.click(await screen.findByRole("button", { name: "Calculer à chaud" }));
    expect(await screen.findByText(`Jeu ${thermal.clearance_shift_label}`)).toBeInTheDocument();
    expect(screen.getByText("serré")).toBeInTheDocument();
    expect(screen.getByText("avec jeu")).toBeInTheDocument();
    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("NON COMPATIBLE");
  });

  it("relaie l'erreur du moteur avec sa piste d'action", async () => {
    vi.mocked(materialsCatalogue).mockResolvedValue(catalogue);
    vi.mocked(toleranceClasses).mockResolvedValue(classesFixture as ClassCatalogue);
    vi.mocked(materialsRead).mockRejectedValue({
      message: "Entrée illisible : « Z300 »",
      hint: "« Z300 » ne commence ni par une lettre de groupe d'emploi…",
    });
    render(<Materials />);
    await userEvent.click(await screen.findByRole("button", { name: "Lire la désignation" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("ne commence");
  });
});
