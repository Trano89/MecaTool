/**
 * Vérifie l'écran des tolérances générales.
 *
 * L'échantillon employé est celui d'une cote de 2 mm, où la norme ne définit
 * pas la classe v : le cas intéressant, puisqu'il faut afficher l'absence sans
 * la faire passer pour un zéro.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import fixture from "../fixtures/general-tolerances.json";
import type { ClassComparison } from "../types";
import { GeneralTolerances } from "./GeneralTolerances";

vi.mock("../api", () => ({
  generalTolerances: vi.fn(),
  analyse: vi.fn(),
  rescaleDiagram: vi.fn(),
  engineInfo: vi.fn(),
  isDesktop: () => true,
}));

const { generalTolerances } = await import("../api");

async function show(comparison: ClassComparison = fixture as ClassComparison) {
  vi.mocked(generalTolerances).mockResolvedValue(comparison);
  render(<GeneralTolerances />);
  await waitFor(() => expect(generalTolerances).toHaveBeenCalled());
}

describe("écran des tolérances générales", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("interroge le moteur dès l'ouverture, sur une cote linéaire", async () => {
    await show();
    expect(generalTolerances).toHaveBeenCalledWith("linear", "50");
  });

  it("affiche les quatre classes avec leurs écarts", async () => {
    await show();

    const table = within(await screen.findByRole("table"));
    expect(table.getByText("± 0.05 mm")).toBeInTheDocument();
    expect(table.getByText("± 0.1 mm")).toBeInTheDocument();
    expect(table.getByText("± 0.2 mm")).toBeInTheDocument();

    // Les désignations telles qu'elles s'inscrivent au cartouche.
    expect(table.getByText("ISO 2768-f")).toBeInTheDocument();
    expect(table.getByText("ISO 2768-m")).toBeInTheDocument();
  });

  it("montre les dimensions limites d'une cote linéaire", async () => {
    await show();
    const table = within(await screen.findByRole("table"));
    // 2 mm en classe f : 1,950 à 2,050.
    expect(table.getByText("1.950 mm")).toBeInTheDocument();
    expect(table.getByText("2.050 mm")).toBeInTheDocument();
  });

  /// Le cœur du test : une classe non définie doit rester visible, avec sa raison.
  it("garde visible une classe que la norme ne définit pas", async () => {
    await show();

    const table = within(await screen.findByRole("table"));
    // La classe v figure toujours dans le tableau.
    expect(table.getByText("très grossier")).toBeInTheDocument();
    // ... et son absence est expliquée, pas remplacée par un tiret muet.
    expect(table.getByText(/Non définie/)).toBeInTheDocument();
    expect(table.getByText(/ne définit pas/)).toBeInTheDocument();
  });

  it("change de table quand on change de type de cote", async () => {
    await show();
    vi.mocked(generalTolerances).mockClear();

    await userEvent.click(screen.getByRole("button", { name: "Dimension angulaire" }));
    await waitFor(() => expect(generalTolerances).toHaveBeenCalledWith("angular", "50"));

    // L'intitulé du champ change : la norme prend une longueur, pas un angle.
    expect(screen.getByLabelText("Longueur du côté le plus court")).toBeInTheDocument();
  });

  it("expose les réserves de la norme sans les imposer", async () => {
    await show();
    const notes = await screen.findByText("Ce que dit la norme, et ce qu'elle ne dit pas");
    // Repliées par défaut : elles éclairent, elles n'encombrent pas.
    expect(notes.closest("details")).not.toHaveAttribute("open");

    await userEvent.click(notes);
    expect(screen.getByText(/0,5 mm INCLUS/)).toBeInTheDocument();
  });

  it("signale une dimension refusée avec la piste du moteur", async () => {
    vi.mocked(generalTolerances).mockRejectedValue({
      message: "Dimension illisible : « abc ».",
      hint: "caractere invalide dans la valeur numerique",
    });
    render(<GeneralTolerances />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Dimension illisible");
    expect(alert).toHaveTextContent("caractere invalide");
  });
});
