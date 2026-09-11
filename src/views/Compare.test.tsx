/**
 * Vérifie l'écran de comparaison.
 *
 * L'échantillon est celui des quatre ajustements du cahier des charges, avec une
 * exigence de jeu : il couvre à la fois le tableau, le dessin et les verdicts.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import fixture from "../fixtures/fit-comparison.json";
import type { FitComparison } from "../types";
import { Compare } from "./Compare";

vi.mock("../api", () => ({
  compare: vi.fn(),
  analyse: vi.fn(),
  generalTolerances: vi.fn(),
  rescaleDiagram: vi.fn(),
  engineInfo: vi.fn(),
  isDesktop: () => true,
}));

const { compare } = await import("../api");

async function show(comparison: FitComparison = fixture as FitComparison) {
  vi.mocked(compare).mockResolvedValue(comparison);
  render(<Compare />);
  await waitFor(() => expect(compare).toHaveBeenCalled());
}

describe("écran de comparaison", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("compare dès l'ouverture, sans écran vide", async () => {
    await show();
    expect(compare).toHaveBeenCalledWith("Ø20 H7/g6, H7/h6, H7/k6, H7/p6", undefined);
  });

  /// Le cœur du test : l'ordre de saisie ne doit jamais être réarrangé.
  it("conserve l'ordre de saisie", async () => {
    await show();

    const rows = await screen.findAllByRole("row");
    // La première ligne est l'en-tête.
    const designations = rows.slice(1).map((row) => row.querySelector("th")?.textContent);
    expect(designations).toEqual(["H7/g6", "H7/h6", "H7/k6", "H7/p6"]);
  });

  it("montre les jeux et la dispersion de chaque ajustement", async () => {
    await show();
    const table = within(await screen.findByRole("table"));

    // H7/g6 à Ø20 : 7 à 41 µm, dispersion 34 µm.
    expect(table.getByText("7 µm")).toBeInTheDocument();
    expect(table.getByText("41 µm")).toBeInTheDocument();
    // H7/h6 : 0 à 34 µm.
    expect(table.getByText("0 µm")).toBeInTheDocument();
  });

  it("montre la progression du jeu vers le serrage", async () => {
    await show();
    const table = within(await screen.findByRole("table"));
    expect(table.getAllByText(/AJUSTEMENT AVEC JEU/).length).toBe(2);
    expect(table.getByText(/AJUSTEMENT INCERTAIN/)).toBeInTheDocument();
    expect(table.getByText(/AJUSTEMENT AVEC SERRAGE/)).toBeInTheDocument();
  });

  it("ajoute un verdict par ligne quand une exigence est fournie", async () => {
    await show();
    const table = within(await screen.findByRole("table"));
    expect(table.getByText("Verdict")).toBeInTheDocument();
    // H7/p6 serre : il ne peut pas satisfaire une exigence de jeu.
    expect(table.getAllByText(/NON COMPATIBLE/).length).toBeGreaterThan(0);
  });

  it("dessine toutes les zones sur une échelle unique", async () => {
    await show();
    const figure = await screen.findByRole("img", { name: /Zones de tolérance/ });
    expect(figure).toBeInTheDocument();
    // Huit zones : deux par ajustement.
    expect(figure.querySelectorAll("rect")).toHaveLength(8);
    // L'annonce d'échelle reste obligatoire.
    expect(screen.getByText(/amplifiés pour la lisibilité/)).toBeInTheDocument();
  });

  it("dit explicitement qu'il ne reclasse pas", async () => {
    await show();
    expect(await screen.findByText(/MecaTool ne reclasse pas/)).toBeInTheDocument();
  });

  it("relaie la question du moteur quand la saisie est ambiguë", async () => {
    vi.mocked(compare).mockRejectedValue({
      message: "Entrée ambiguë : « 7 ajustements »",
      hint: "MecaTool compare au plus 6 ajustements à la fois : au-delà, les zones deviennent trop étroites pour être lues.",
    });
    render(<Compare />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("ambiguë");
    expect(alert).toHaveTextContent("trop étroites");
  });

  it("relance la comparaison sur un exemple choisi", async () => {
    await show();
    vi.mocked(compare).mockClear();

    await userEvent.click(screen.getByRole("button", { name: "Ø50 H7/f7, H8/f7, H7/g6" }));
    await waitFor(() =>
      expect(compare).toHaveBeenCalledWith("Ø50 H7/f7, H8/f7, H7/g6", undefined),
    );
  });
});
