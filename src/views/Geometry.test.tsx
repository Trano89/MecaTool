/**
 * Vérifie l'écran de tolérancement géométrique.
 *
 * L'échantillon employé porte trois spécifications sur un même élément : une
 * correcte, une à qui manque sa référence, et une que la première rend
 * inopérante. Il couvre donc les trois sortes de constats d'un seul tenant.
 *
 * Deux points tiennent à la règle absolue sur les normes, et non à l'ergonomie :
 * la réserve de source doit être visible avant toute saisie, et aucun message
 * ne doit suggérer une valeur de tolérance.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import catalogueFixture from "../fixtures/geometric-catalogue.json";
import groupFixture from "../fixtures/geometric-group.json";
import type { GeometricCatalogue, GroupAnalysis } from "../types";
import { Geometry } from "./Geometry";

vi.mock("../api", () => ({
  geometric: vi.fn(),
  geometricCatalogue: vi.fn(),
  isDesktop: () => true,
}));

const { geometric, geometricCatalogue } = await import("../api");

const catalogue = catalogueFixture as GeometricCatalogue;
const group = groupFixture as GroupAnalysis;

async function show() {
  vi.mocked(geometricCatalogue).mockResolvedValue(catalogue);
  vi.mocked(geometric).mockResolvedValue(group);
  render(<Geometry />);
  await waitFor(() => expect(geometricCatalogue).toHaveBeenCalled());
}

async function control() {
  await show();
  await userEvent.click(screen.getByRole("button", { name: "Contrôler" }));
  await waitFor(() => expect(geometric).toHaveBeenCalled());
}

describe("écran de tolérancement géométrique", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("montre l'origine des données avant toute saisie", async () => {
    await show();

    // La donnée vient d'un recueil, pas de la norme. L'utilisateur doit le
    // savoir avant de s'appuyer sur ce que l'écran lui dira, pas après.
    const note = await screen.findByRole("note");
    expect(within(note).getByText(/recueil technique/)).toBeInTheDocument();
    expect(within(note).getByText(/ISO 1101/)).toBeInTheDocument();
  });

  it("annonce qu'il ne propose aucune valeur", async () => {
    await show();
    expect(screen.getByText(/ne propose aucune valeur/)).toBeInTheDocument();
  });

  it("envoie une ligne par spécification, sans les lignes vides", async () => {
    await show();

    const field = screen.getByLabelText(/Spécifications posées/);
    await userEvent.clear(field);
    await userEvent.type(field, "⏥ 0.05{enter}{enter}perp 0.03 A");
    await userEvent.click(screen.getByRole("button", { name: "Contrôler" }));

    await waitFor(() =>
      expect(geometric).toHaveBeenCalledWith(["⏥ 0.05", "perp 0.03 A"]),
    );
  });

  it("affiche chaque spécification sous sa désignation normalisée", async () => {
    await control();
    for (const spec of group.specs) {
      expect(
        screen.getByRole("heading", { name: spec.designation }),
      ).toBeInTheDocument();
    }
  });

  it("montre la gravité en toutes lettres, pas seulement en couleur", async () => {
    await control();

    // Une référence manque dans l'échantillon : le constat doit être lisible
    // sans distinguer les couleurs.
    expect(screen.getAllByText("À corriger").length).toBeGreaterThan(0);
    expect(screen.getByText(/exige au moins une référence spécifiée/)).toBeInTheDocument();
  });

  it("énonce les recouvrements et la règle qui les fonde", async () => {
    await control();

    const section = screen.getByRole("heading", { name: "Recouvrements" })
      .parentElement as HTMLElement;
    expect(within(section).getByText(/borne aussi l'écart de forme/)).toBeInTheDocument();
    expect(within(section).getAllByText(/n'ajoute rien/).length).toBe(
      group.overlaps.length,
    );
  });

  it("mène par la faute plutôt que par le recouvrement", async () => {
    await control();
    // L'échantillon a les deux. Une référence manquante rend le dessin faux ;
    // un recouvrement le rend seulement bavard.
    expect(screen.getByText(/spécification fautive/)).toBeInTheDocument();
    expect(screen.getByText("NON COMPATIBLE")).toBeInTheDocument();
  });

  it("catalogue les caractéristiques avec leur exigence de référence", async () => {
    await show();

    const tables = await screen.findAllByRole("table");
    const forme = within(tables[0] as HTMLElement);
    expect(forme.getByText("rectitude")).toBeInTheDocument();
    expect(forme.getByText("cylindricité")).toBeInTheDocument();

    // Aucune tolérance de forme ne prend de référence : la colonne doit le dire
    // pour chacune, et non une seule fois en tête de famille.
    const aucune = forme.getAllByText("aucune");
    expect(aucune.length).toBe(
      catalogue.characteristics.filter((c) => c.class === "form").length,
    );
  });

  it("dit quelle norme définit chaque modificateur", async () => {
    await show();

    // Le maximum de matière vient de l'ISO 2692, pas de l'ISO 1101. Le porter
    // au crédit de l'ISO 1101 serait une erreur de source.
    const tables = await screen.findAllByRole("table");
    const modifiers = within(tables[tables.length - 1] as HTMLElement);
    expect(modifiers.getByText("exigence du maximum de matière")).toBeInTheDocument();
    expect(modifiers.getAllByText("ISO 2692").length).toBe(3);
    expect(modifiers.getByText("ISO 14405-1")).toBeInTheDocument();
  });

  it("ne suggère jamais de valeur de tolérance", async () => {
    await control();
    // Garde-fou de bout en bout : ce que le moteur refuse de faire ne doit pas
    // réapparaître dans un libellé d'interface.
    expect(screen.queryByText(/il conviendrait/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/valeur recommandée/i)).not.toBeInTheDocument();
  });

  it("affiche l'erreur du moteur avec sa piste d'action", async () => {
    vi.mocked(geometricCatalogue).mockResolvedValue(catalogue);
    vi.mocked(geometric).mockRejectedValue({
      message: "Entrée illisible : « bidule 0.1 »",
      hint: "« bidule » n'est pas une caractéristique géométrique connue.",
    });
    render(<Geometry />);
    await userEvent.click(screen.getByRole("button", { name: "Contrôler" }));

    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText(/Entrée illisible/)).toBeInTheDocument();
    expect(within(alert).getByText(/n'est pas une caractéristique/)).toBeInTheDocument();
  });

  it("reste utilisable si le catalogue ne charge pas", async () => {
    // Le catalogue n'est qu'une référence dépliable : son absence ne doit pas
    // empêcher de contrôler une spécification.
    vi.mocked(geometricCatalogue).mockRejectedValue(new Error("indisponible"));
    vi.mocked(geometric).mockResolvedValue(group);
    render(<Geometry />);

    await userEvent.click(screen.getByRole("button", { name: "Contrôler" }));
    await waitFor(() => expect(geometric).toHaveBeenCalled());
    expect(
      screen.getByRole("heading", { name: group.specs[0]!.designation }),
    ).toBeInTheDocument();
  });
});
