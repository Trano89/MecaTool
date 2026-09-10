/**
 * Vérifie l'écran des chaînes de cotes.
 *
 * L'échantillon comporte un maillon diminuant, une estimation statistique et
 * des limites fonctionnelles : il couvre tout ce que l'écran doit rendre.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import fixture from "../fixtures/chain-report.json";
import type { ChainReport } from "../types";
import { Chain } from "./Chain";

vi.mock("../api", () => ({
  dimensionChain: vi.fn(),
  compare: vi.fn(),
  analyse: vi.fn(),
  generalTolerances: vi.fn(),
  rescaleDiagram: vi.fn(),
  engineInfo: vi.fn(),
  isDesktop: () => true,
}));

const { dimensionChain } = await import("../api");

async function show(report: ChainReport = fixture as ChainReport) {
  vi.mocked(dimensionChain).mockResolvedValue(report);
  render(<Chain />);
  await waitFor(() => expect(dimensionChain).toHaveBeenCalled());
}

describe("écran des chaînes de cotes", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calcule dès l'ouverture, sur une chaîne d'exemple", async () => {
    await show();
    expect(dimensionChain).toHaveBeenCalledWith("A = 20 ±0.1\nB = 10 ±0.05\n-C = 5 ±0.02", {
      statistical: false,
      minimumMm: undefined,
      maximumMm: undefined,
    });
  });

  it("affiche la résultante et ses limites", async () => {
    await show();
    // 20 + 10 − 5 = 25, tolérance 0,34 → 24,830 à 25,170.
    expect(await screen.findByText("25.000 mm")).toBeInTheDocument();
    expect(screen.getByText("25.170 mm")).toBeInTheDocument();
    expect(screen.getByText("24.830 mm")).toBeInTheDocument();
    expect(screen.getByText("0.340 mm")).toBeInTheDocument();
  });

  it("montre la contribution de chaque maillon", async () => {
    await show();
    const chart = await screen.findByRole("img", { name: /Part de chaque maillon/ });
    // Une barre par maillon.
    expect(chart.querySelectorAll("rect")).toHaveLength(3);
    // A pèse 0,2 sur 0,34, soit 58,8 %.
    expect(within(chart).getByText(/58,8 %/)).toBeInTheDocument();
  });

  it("signale le maillon diminuant par son signe", async () => {
    await show();
    const chart = await screen.findByRole("img", { name: /Part de chaque maillon/ });
    // Le maillon C est diminuant : son repère porte un moins.
    expect(within(chart).getByText(/−/)).toBeInTheDocument();
  });

  /// Le cœur du test : l'estimation statistique ne circule jamais sans ce qui
  /// la rend possible.
  it("affiche les hypothèses avec l'estimation statistique, sans les replier", async () => {
    await show();

    expect(await screen.findByText(/Racine de la somme des carrés/)).toBeInTheDocument();
    const note = screen.getByRole("note");
    expect(note).toHaveTextContent("CE QUE CETTE ESTIMATION SUPPOSE");
    expect(note).toHaveTextContent("indépendantes");
    expect(note).toHaveTextContent("n'est vérifiée par Mecatol");
    // Elles ne sont pas dans un <details> : elles se voient sans action.
    expect(note.closest("details")).toBeNull();
  });

  it("explique pourquoi l'estimation n'est pas faite par défaut", async () => {
    const withoutEstimate = {
      ...(fixture as ChainReport),
      analysis: { ...(fixture as ChainReport).analysis, statistical: null },
    };
    await show(withoutEstimate);

    expect(await screen.findByText(/Le pire des cas ne suppose rien/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Estimer (RSS)" })).toBeInTheDocument();
  });

  it("demande l'estimation au moteur quand on la réclame", async () => {
    const withoutEstimate = {
      ...(fixture as ChainReport),
      analysis: { ...(fixture as ChainReport).analysis, statistical: null },
    };
    await show(withoutEstimate);
    vi.mocked(dimensionChain).mockClear();

    await userEvent.click(screen.getByRole("button", { name: "Estimer (RSS)" }));
    await waitFor(() =>
      expect(dimensionChain).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({ statistical: true }),
      ),
    );
  });

  it("rend le verdict face aux limites fonctionnelles", async () => {
    await show();
    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent(/COMPATIBLE|ATTENTION|NON COMPATIBLE/);
  });

  it("relaie la piste du moteur sur une chaîne illisible", async () => {
    vi.mocked(dimensionChain).mockRejectedValue({
      message: "Entrée illisible : « A = 20 »",
      hint: "Écarts absents. Un maillon s'écrit « A = 20 ±0.1 ».",
    });
    render(<Chain />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("illisible");
    expect(alert).toHaveTextContent("Écarts absents");
  });
});
