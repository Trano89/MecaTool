/**
 * Vérifie l'écran des roulements.
 *
 * Deux points tiennent à la règle absolue sur les sources, et non à l'ergonomie :
 *
 *  1. La réserve doit être lisible **avant** toute saisie, et elle doit dire que
 *     le tableau des classes de montage n'a aucun caractère normatif.
 *  2. Le résultat croise deux natures de source — une recommandation de
 *     fabricant, puis un calcul ISO 286 — et l'écran doit les tenir séparées.
 *     Un panneau unique durcirait la recommandation en règle.
 *
 * Les données ne sont pas inventées : ce sont les échantillons que le moteur
 * exporte. Les cas d'emploi, pour lesquels il n'exporte pas d'échantillon, sont
 * dérivés du catalogue lui-même plutôt que rédigés à la main.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import adviceFixture from "../fixtures/bearing-advice.json";
import catalogueFixture from "../fixtures/bearing-catalogue.json";
import type {
  BearingCatalogue,
  DesignationReading,
  MountingAdvice,
  MountingOption,
} from "../types";
import { Bearings } from "./Bearings";

vi.mock("../api", () => ({
  bearingCatalogue: vi.fn(),
  bearingRead: vi.fn(),
  bearingOptions: vi.fn(),
  bearingAdvise: vi.fn(),
  isDesktop: () => true,
}));

const { bearingAdvise, bearingCatalogue, bearingOptions, bearingRead } =
  await import("../api");

const catalogue = catalogueFixture as BearingCatalogue;
const advice = adviceFixture as MountingAdvice;

/** Les cas d'emploi du premier régime, tels que le catalogue les porte. */
const OPTIONS: MountingOption[] = catalogue.cases
  .filter((mounting) => mounting.regime === catalogue.regimes[0]!.id)
  .map((mounting) => ({
    regime: mounting.regime,
    condition: mounting.condition,
    examples: mounting.examples,
    class: mounting.rows[0]?.class ?? null,
    unavailable: null,
  }));

const READINGS: DesignationReading[] = [
  {
    designation: "6210",
    series: "62",
    bore_code: "10",
    bore: 50_000_000,
    explanation: "série 62, symbole d'alésage 10 : 10 × 5 = 50 mm",
  },
  {
    designation: "6210",
    series: "621",
    bore_code: "0",
    bore: 10_000_000,
    explanation: "série 621, symbole d'alésage 0",
  },
];

async function show() {
  vi.mocked(bearingCatalogue).mockResolvedValue(catalogue);
  vi.mocked(bearingOptions).mockResolvedValue(OPTIONS);
  vi.mocked(bearingAdvise).mockResolvedValue(advice);
  vi.mocked(bearingRead).mockResolvedValue(READINGS);
  render(<Bearings />);
  await waitFor(() => expect(bearingCatalogue).toHaveBeenCalled());
}

describe("écran des roulements", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("montre l'origine des données avant toute saisie", async () => {
    await show();

    const notes = await screen.findAllByRole("note");
    expect(notes.length).toBe(catalogue.warnings.length);

    // Le point décisif : la table des classes de montage n'est pas normative,
    // et l'écran doit le dire avant qu'on s'en serve.
    const texts = notes.map((note) => note.textContent ?? "").join(" ");
    expect(texts).toMatch(/sans caractère normatif/);
  });

  it("propose les familles et les régimes du catalogue, pas une liste écrite à la main", async () => {
    await show();

    // La liste est fermée au départ : on l'ouvre pour voir ce qu'elle propose.
    const families = screen.getByLabelText("Type de roulement");
    await userEvent.click(families);

    const listbox = await screen.findByRole("listbox");
    for (const family of catalogue.families) {
      expect(within(listbox).getByText(family.name)).toBeInTheDocument();
    }
    // Et rien de plus : une option que le moteur n'a pas annoncée serait une
    // famille inventée par l'interface.
    expect(within(listbox).getAllByRole("option")).toHaveLength(
      catalogue.families.length,
    );

    // Le régime porte son explication : c'est elle qui permet de choisir, pas
    // son nom.
    for (const regime of catalogue.regimes) {
      expect(screen.getByText(regime.explanation)).toBeInTheDocument();
    }
  });

  it("rend les lectures possibles d'une désignation sans en choisir une", async () => {
    await show();

    await userEvent.click(screen.getByRole("button", { name: "Lire la désignation" }));
    await waitFor(() => expect(bearingRead).toHaveBeenCalledWith("6210"));

    // Les deux découpages sont proposés. La source ne dit pas comment trancher :
    // c'est l'utilisateur qui reconnaît le sien.
    for (const reading of READINGS) {
      expect(screen.getByText(reading.explanation)).toBeInTheDocument();
    }
    expect(screen.getAllByRole("button", { name: "Retenir" })).toHaveLength(2);
  });

  it("demande au moteur les cas d'emploi du régime retenu", async () => {
    await show();
    await waitFor(() =>
      expect(bearingOptions).toHaveBeenCalledWith(
        catalogue.regimes[0]!.id,
        catalogue.families[0]!.id,
        "50",
      ),
    );
    for (const option of OPTIONS) {
      expect(await screen.findByText(option.condition)).toBeInTheDocument();
    }
  });

  /// Le cœur du test : la frontière entre les deux natures de source.
  it("sépare la classe recommandée des écarts qui en découlent", async () => {
    await show();

    const open = await screen.findAllByRole("button", { name: "Voir les écarts" });
    await userEvent.click(open[0]!);
    await waitFor(() => expect(bearingAdvise).toHaveBeenCalled());

    // Deux panneaux numérotés, dans cet ordre.
    expect(screen.getByText("1 — Classe recommandée")).toBeInTheDocument();
    expect(screen.getByText("2 — Écarts de cette classe")).toBeInTheDocument();

    // La classe est donnée pour ce qu'elle est : une pratique, pas une norme.
    expect(screen.getAllByText("Pratique recommandée").length).toBeGreaterThan(0);
    // Les écarts, eux, sont normatifs.
    expect(screen.getAllByText("Vérifié sur la norme").length).toBeGreaterThan(0);

    // Et les écarts eux-mêmes sont ceux du moteur, au micromètre près.
    const shaft = within(screen.getByRole("region", { name: "Arbre k5" }));
    expect(shaft.getByText("+2 µm")).toBeInTheDocument();
    expect(shaft.getByText("+13 µm")).toBeInTheDocument();
    expect(screen.getByText(advice.designation)).toBeInTheDocument();
  });

  it("porte le verdict par un libellé, pas seulement par une couleur", async () => {
    await show();
    const open = await screen.findAllByRole("button", { name: "Voir les écarts" });
    await userEvent.click(open[0]!);

    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent("ATTENTION");
    expect(status.querySelector("[aria-hidden='true']")).not.toBeNull();
  });

  it("relaie l'erreur du moteur avec sa piste d'action", async () => {
    vi.mocked(bearingCatalogue).mockResolvedValue(catalogue);
    vi.mocked(bearingOptions).mockRejectedValue({
      message: "Alésage illisible : « abc ».",
      hint: "caractère invalide dans la valeur numérique",
    });
    render(<Bearings />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Alésage illisible");
    expect(alert).toHaveTextContent("caractère invalide");
  });
});
