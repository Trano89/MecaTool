/**
 * Vérifie que l'écran de calcul restitue fidèlement ce que le moteur renvoie.
 *
 * Les données ne sont pas inventées pour le test : ce sont les échantillons
 * produits par le moteur lui-même. Un écart entre ce que Mecatol calcule et ce
 * que Mecatol affiche fait donc échouer ce fichier.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import fitFixture from "../fixtures/fit-report.json";
import searchFixture from "../fixtures/search-report.json";
import type { Report } from "../types";
import { Calculate } from "./Calculate";

vi.mock("../api", () => ({
  analyse: vi.fn(),
  rescaleDiagram: vi.fn(),
  engineInfo: vi.fn(),
  isDesktop: () => true,
}));

const { analyse } = await import("../api");

async function calculate(report: Report, query = { input: "Ø10 H7/g6", clearance: "2..40" }) {
  vi.mocked(analyse).mockResolvedValue(report);
  render(<Calculate query={query} onQueryChange={() => {}} />);
  await userEvent.click(screen.getByRole("button", { name: "Calculer" }));
  await waitFor(() => expect(analyse).toHaveBeenCalled());
}

describe("écran de calcul", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("affiche les deux éléments d'un ajustement avec leurs cotes exactes", async () => {
    await calculate(fitFixture as Report);

    const hole = within(await screen.findByRole("region", { name: "Alésage H7" }));
    const shaft = within(screen.getByRole("region", { name: "Arbre g6" }));

    // Les dimensions limites doivent être celles du moteur, au millième près.
    expect(hole.getByText("10.015 mm")).toBeInTheDocument();
    expect(hole.getByText("10.000 mm")).toBeInTheDocument();
    expect(shaft.getByText("9.995 mm")).toBeInTheDocument();
    expect(shaft.getByText("9.986 mm")).toBeInTheDocument();

    // Les symboles d'écart suivent la casse normative : majuscules pour
    // l'alésage, minuscules pour l'arbre.
    expect(hole.getByText("Écart supérieur ES")).toBeInTheDocument();
    expect(hole.getByText("Écart inférieur EI")).toBeInTheDocument();
    expect(shaft.getByText("Écart supérieur es")).toBeInTheDocument();
    expect(shaft.getByText("Écart inférieur ei")).toBeInTheDocument();

    // Et les écarts eux-mêmes.
    expect(hole.getByText("+15 µm")).toBeInTheDocument();
    expect(shaft.getByText("-5 µm")).toBeInTheDocument();
    expect(shaft.getByText("-14 µm")).toBeInTheDocument();
  });

  it("affiche le jeu calculé et sa classification", async () => {
    await calculate(fitFixture as Report);

    expect(await screen.findByText("5 µm")).toBeInTheDocument();
    expect(screen.getByText("29 µm")).toBeInTheDocument();
    expect(screen.getByText(/AJUSTEMENT AVEC JEU/)).toBeInTheDocument();
  });

  it("rend le diagramme avec son annonce d'échelle", async () => {
    await calculate(fitFixture as Report);

    const figure = await screen.findByRole("img", { name: /Zones de tolérance/ });
    expect(figure).toBeInTheDocument();

    // L'annonce d'échelle ne doit jamais manquer : sans elle, le lecteur peut
    // croire que le dessin respecte les proportions de la pièce.
    expect(screen.getByText(/amplifiés pour la lisibilité/)).toBeInTheDocument();
  });

  it("porte le verdict par un libellé, pas seulement par une couleur", async () => {
    await calculate(fitFixture as Report);

    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent("COMPATIBLE");
    // La pastille est décorative : elle double le libellé, ne le remplace pas.
    const badge = status.querySelector("[aria-hidden='true']");
    expect(badge).not.toBeNull();
    expect(status.textContent).toContain("COMPATIBLE");
  });

  it("expose le raisonnement, replié par défaut", async () => {
    await calculate(fitFixture as Report);

    const why = await screen.findByText("Comment ce jeu est calculé");
    expect(why.closest("details")).not.toHaveAttribute("open");

    await userEvent.click(why);
    expect(screen.getByText("Jeu minimal")).toBeInTheDocument();
  });

  it("explique une recherche sans solution exacte et rappelle son périmètre", async () => {
    await calculate(searchFixture as Report, { input: "Ø20", clearance: "10..30" });

    expect(await screen.findByText("AUCUNE SOLUTION EXACTE")).toBeInTheDocument();
    expect(screen.getByText(/géométriquement possible/)).toBeInTheDocument();

    // Le périmètre doit être rappelé : une recherche infructueuse ne prouve pas
    // qu'aucune solution n'existe.
    expect(screen.getByText(/Systèmes explorés/)).toBeInTheDocument();
    expect(screen.getByText(/ne sont pas encore saisies/)).toBeInTheDocument();
  });

  it("affiche les solutions dans l'ordre rendu par le moteur", async () => {
    await calculate(searchFixture as Report, { input: "Ø20", clearance: "10..30" });

    const rows = await screen.findAllByRole("row");
    // En-tête plus les solutions de l'échantillon.
    expect(rows.length).toBeGreaterThan(1);
    expect(rows[1]).toHaveTextContent("G5/h6");
  });

  it("signale une entrée refusée avec la piste d'action du moteur", async () => {
    vi.mocked(analyse).mockRejectedValue({
      message: "Entrée illisible : « bonjour »",
      hint: "La désignation doit commencer par la dimension nominale.",
    });
    render(<Calculate query={{ input: "bonjour", clearance: "" }} onQueryChange={() => {}} />);
    await userEvent.click(screen.getByRole("button", { name: "Calculer" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Entrée illisible");
    expect(alert).toHaveTextContent("doit commencer par la dimension nominale");
  });
});
