/**
 * Vérifie l'écran des états de surface.
 *
 * Trois points tiennent à la règle absolue sur les sources, et non à
 * l'ergonomie :
 *
 *  1. Les réserves se lisent **avant** toute saisie : aucune des trois sources
 *     n'est encore confrontée à sa norme.
 *  2. Le graphique des procédés répond à « quelle rugosité pour ce procédé ? »
 *     dès l'ouverture, sans attendre une saisie.
 *  3. Quand le moteur refuse de confronter une exigence au tableau — un Rz, par
 *     exemple — l'écran affiche le refus, et non une liste vide qu'on lirait
 *     « aucun procédé ne convient ».
 *
 * Les données sont les échantillons que le moteur exporte.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import analysisFixture from "../fixtures/surface-analysis.json";
import catalogueFixture from "../fixtures/surface-catalogue.json";
import type { SurfaceAnalysis, SurfaceCatalogue } from "../types";
import { Surface } from "./Surface";

vi.mock("../api", () => ({
  surfaceCatalogue: vi.fn(),
  surfaceRead: vi.fn(),
  isDesktop: () => true,
}));

const { surfaceCatalogue, surfaceRead } = await import("../api");

const catalogue = catalogueFixture as SurfaceCatalogue;
const analysis = analysisFixture as SurfaceAnalysis;

async function show() {
  vi.mocked(surfaceCatalogue).mockResolvedValue(catalogue);
  vi.mocked(surfaceRead).mockResolvedValue(analysis);
  render(<Surface />);
  await waitFor(() => expect(surfaceCatalogue).toHaveBeenCalled());
}

describe("écran des états de surface", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("montre l'origine des données avant toute saisie", async () => {
    await show();
    const notes = await screen.findAllByRole("note");
    expect(notes).toHaveLength(catalogue.warnings.length);
    const texts = notes.map((note) => note.textContent ?? "").join(" ");
    expect(texts).toMatch(/non vérifiée/);
  });

  it("répond à « quelle rugosité pour ce procédé ? » dès l'ouverture", async () => {
    await show();
    expect(await screen.findByText("Quelle rugosité pour ce procédé ?")).toBeInTheDocument();
    const chart = screen.getByRole("img", { name: catalogue.chart.caption });
    // Une ligne par procédé, et ses noms viennent du moteur.
    for (const row of catalogue.chart.rows) {
      expect(within(chart).getByText(row.name)).toBeInTheDocument();
    }
  });

  it("lit l'indication saisie et classe les procédés", async () => {
    await show();
    await userEvent.click(screen.getByRole("button", { name: "Lire l'indication" }));
    await waitFor(() => expect(surfaceRead).toHaveBeenCalledWith("Ra 0.8"));

    expect(screen.getAllByText(analysis.designation).length).toBeGreaterThan(0);
    expect(screen.getByRole("heading", { name: /Atteinte d'ordinaire/ })).toBeInTheDocument();

    // Le verdict reste prudent, et se lit sans la couleur.
    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("ATTENTION");
  });

  it("affiche le refus du moteur plutôt qu'une liste vide", async () => {
    vi.mocked(surfaceCatalogue).mockResolvedValue(catalogue);
    const note =
      "Le tableau des procédés est exprimé en Ra. MecaTool ne convertit pas Rz en Ra.";
    vi.mocked(surfaceRead).mockResolvedValue({
      ...analysis,
      processes: [],
      process_note: note,
    });
    render(<Surface />);
    await userEvent.click(await screen.findByRole("button", { name: "Lire l'indication" }));

    expect(await screen.findByText("Pas de confrontation au tableau")).toBeInTheDocument();
    expect(screen.getByText(note)).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /Atteinte d'ordinaire/ })).toBeNull();
  });

  it("un exemple se lit d'un clic", async () => {
    await show();
    await userEvent.click(screen.getByRole("button", { name: "N7" }));
    await waitFor(() => expect(surfaceRead).toHaveBeenCalledWith("N7"));
  });

  it("relaie l'erreur du moteur avec sa piste d'action", async () => {
    vi.mocked(surfaceCatalogue).mockResolvedValue(catalogue);
    vi.mocked(surfaceRead).mockRejectedValue({
      message: "Entrée illisible : « bidule »",
      hint: "« bidule » n'est ni un paramètre, ni une classe N.",
    });
    render(<Surface />);
    await userEvent.click(await screen.findByRole("button", { name: "Lire l'indication" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("illisible");
    expect(alert).toHaveTextContent("ni un paramètre");
  });

  it("compose l'indication par boutons, sans rien taper", async () => {
    await show();
    const symbol = await screen.findByRole("group", { name: "Variante du symbole" });
    await userEvent.click(within(symbol).getByRole("button", { name: /^MRR/ }));
    await waitFor(() => expect(surfaceRead).toHaveBeenLastCalledWith("MRR Ra 0.8"));

    const values = screen.getByRole("group", { name: "Valeur (µm)" });
    // Les valeurs proposées sont celles de la série du moteur, et elles seules.
    expect(within(values).getAllByRole("button")).toHaveLength(catalogue.grades.length);
    await userEvent.click(within(values).getByRole("button", { name: "1.6" }));
    await waitFor(() => expect(surfaceRead).toHaveBeenLastCalledWith("MRR Ra 1.6"));

    const lays = screen.getByRole("group", { name: "Sens des stries" });
    await userEvent.click(within(lays).getByRole("button", { name: /⊥/ }));
    await waitFor(() => expect(surfaceRead).toHaveBeenLastCalledWith("MRR Ra 1.6 ⊥"));
    expect(screen.getByLabelText("Exigence de rugosité")).toHaveValue("MRR Ra 1.6 ⊥");
  });
});
