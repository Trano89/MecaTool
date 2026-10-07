/**
 * Vérifie l'écran de la visserie.
 *
 * Le point central est celui des roulements : un trou de passage croise deux
 * natures de source. Son diamètre et sa classe viennent de l'ISO 273, non
 * encore confrontée ; ses écarts viennent de l'ISO 286, confrontée. L'écran
 * doit montrer les deux étiquettes, pas une seule pour tout le tableau.
 *
 * Et rien ne doit ressembler à un effort admissible : la classe de qualité ne
 * donne que des valeurs nominales.
 */

import { render, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import catalogueFixture from "../fixtures/fastener-catalogue.json";
import reportFixture from "../fixtures/thread-report.json";
import type { FastenerCatalogue, ThreadReport } from "../types";
import { Fasteners } from "./Fasteners";

vi.mock("../api", () => ({
  fastenerCatalogue: vi.fn(),
  fastenerRead: vi.fn(),
  isDesktop: () => true,
}));

const { fastenerCatalogue, fastenerRead } = await import("../api");

const catalogue = catalogueFixture as FastenerCatalogue;
const report = reportFixture as ThreadReport;

async function showReport() {
  vi.mocked(fastenerCatalogue).mockResolvedValue(catalogue);
  vi.mocked(fastenerRead).mockResolvedValue(report);
  render(<Fasteners />);
  await userEvent.click(await screen.findByRole("button", { name: "Lire le filetage" }));
  await waitFor(() => expect(fastenerRead).toHaveBeenCalledWith("M10"));
}

describe("écran de la visserie", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("montre l'origine des données avant toute saisie", async () => {
    vi.mocked(fastenerCatalogue).mockResolvedValue(catalogue);
    render(<Fasteners />);
    const notes = await screen.findAllByRole("note");
    expect(notes).toHaveLength(catalogue.warnings.length);
    expect(notes.map((n) => n.textContent ?? "").join(" ")).toMatch(/non vérifiée/);
  });

  it("déplie la désignation : pas, profil, section, perçage", async () => {
    await showReport();
    // La désignation normalisée figure en titre du résultat — elle figure aussi
    // dans le tableau des filetages embarqués, d'où la recherche par le titre.
    const headline = await screen.findByText(report.normalised, { selector: ".headline strong" });
    expect(headline).toBeInTheDocument();
    expect(screen.getByText(report.stress_area_label)).toBeInTheDocument();
    expect(screen.getByText(report.tap_drill_label)).toBeInTheDocument();
    // Le perçage est donné pour ce qu'il est.
    expect(screen.getByText("règle d'atelier D − P")).toBeInTheDocument();
  });

  it("sépare la source du trou de celle de ses écarts", async () => {
    await showReport();
    for (const hole of report.clearance_holes) {
      expect(await screen.findByText(hole.designation)).toBeInTheDocument();
    }
    // ISO 273 non confrontée, ISO 286 confrontée : les deux étiquettes.
    expect(screen.getAllByText("Non vérifié").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Vérifié sur la norme").length).toBeGreaterThan(0);
    // H13 sur Ø11 : EI = 0, ES = +270 µm, au micromètre près du moteur.
    expect(screen.getByText("+270 µm")).toBeInTheDocument();
  });

  it("lit la classe de qualité en valeurs nominales, sans effort", async () => {
    await showReport();
    const strength = report.strength!;
    expect(
      await screen.findByText(`3 — Classe de qualité ${strength.class.class}`),
    ).toBeInTheDocument();
    expect(screen.getByText(`${strength.class.tensile_mpa} MPa`)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/kN/);
  });

  it("relaie l'erreur du moteur avec sa piste d'action", async () => {
    vi.mocked(fastenerCatalogue).mockResolvedValue(catalogue);
    vi.mocked(fastenerRead).mockRejectedValue({
      message: "Entrée illisible : « M11 »",
      hint: "M11 ne figure pas dans la sélection embarquée. Ce n'est pas dire qu'il n'existe pas.",
    });
    render(<Fasteners />);
    await userEvent.click(await screen.findByRole("button", { name: "Lire le filetage" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("n'existe pas");
  });
});
