/**
 * Vérifie l'écran de la soudure.
 *
 * Ce qui compte :
 *
 *  1. les réserves se lisent avant toute saisie ;
 *  2. les symboles et les niveaux viennent du catalogue du moteur, pas d'une
 *     liste écrite dans l'interface ;
 *  3. **aucun niveau de qualité n'est présélectionné** : MecaTool dit ce qu'un
 *     niveau tolère, il ne le choisit pas ;
 *  4. un nom d'atelier qui désigne plusieurs numéros les montre tous ;
 *  5. une limite se lit sous sa forme normative et chiffrée.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import catalogueFixture from "../fixtures/welding-catalogue.json";
import processFixture from "../fixtures/welding-process.json";
import readingFixture from "../fixtures/weld-reading.json";
import type { ProcessReading, WeldingCatalogue, WeldReading } from "../types";
import { Welding } from "./Welding";

vi.mock("../api", () => ({
  weldingCatalogue: vi.fn(),
  weldingProcess: vi.fn(),
  weldingRead: vi.fn(),
  isDesktop: () => true,
}));

const { weldingCatalogue, weldingProcess, weldingRead } = await import("../api");

const catalogue = catalogueFixture as WeldingCatalogue;
const readings = processFixture as ProcessReading[];
const reading = readingFixture as WeldReading;

async function show() {
  vi.mocked(weldingCatalogue).mockResolvedValue(catalogue);
  vi.mocked(weldingProcess).mockResolvedValue(readings);
  vi.mocked(weldingRead).mockResolvedValue(reading);
  render(<Welding />);
  await waitFor(() => expect(weldingCatalogue).toHaveBeenCalled());
}

describe("écran de la soudure", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("n'affiche aucune réserve : toutes ses sources sont lues dans les normes", async () => {
    await show();
    await screen.findByLabelText("Symbole élémentaire");
    expect(catalogue.warnings).toHaveLength(0);
    expect(screen.queryAllByRole("note")).toHaveLength(0);
  });

  it("propose les symboles du catalogue, et eux seuls", async () => {
    await show();
    await userEvent.click(await screen.findByLabelText("Symbole élémentaire"));
    const listbox = await screen.findByRole("listbox");
    expect(within(listbox).getAllByRole("option")).toHaveLength(catalogue.elementary.length);
  });

  it("ne présélectionne aucun niveau de qualité", async () => {
    await show();
    const none = await screen.findByRole("button", { name: "Aucun" });
    expect(none).toHaveAttribute("aria-pressed", "true");
    for (const level of catalogue.levels) {
      expect(screen.getByRole("button", { name: `${level.id} — ${level.name}` })).toHaveAttribute(
        "aria-pressed",
        "false",
      );
    }
  });

  it("montre toutes les lectures d'un nom d'atelier", async () => {
    await show();
    await userEvent.click(screen.getByRole("button", { name: "Lire le procédé" }));
    await waitFor(() => expect(weldingProcess).toHaveBeenCalledWith("135"));
    for (const entry of readings) {
      expect(screen.getByText(entry.process.name)).toBeInTheDocument();
    }
    expect(screen.getByText(/désigne plusieurs numéros/)).toBeInTheDocument();
  });

  it("envoie le symbole tel que saisi, grandeurs en texte", async () => {
    await show();
    await userEvent.click(await screen.findByRole("button", { name: "Lire le symbole" }));
    await waitFor(() => expect(weldingRead).toHaveBeenCalled());
    const request = vi.mocked(weldingRead).mock.calls[0]![0];
    expect(request.symbol).toBe("fillet");
    expect(request.size_letter).toBe("a");
    expect(request.size_mm).toBe("5");
    expect(request.level).toBeNull();
  });

  it("restitue la lecture, et chiffre les limites du niveau", async () => {
    await show();
    await userEvent.click(await screen.findByRole("button", { name: "Lire le symbole" }));

    expect(await screen.findByText(reading.designation)).toBeInTheDocument();
    for (const sentence of reading.sentences) {
      expect(screen.getByText(sentence)).toBeInTheDocument();
    }

    const quality = reading.quality!;
    expect(
      screen.getByText(`Niveau de qualité ${quality.level.id} — ${quality.level.name}`),
    ).toBeInTheDocument();
    // La convexité : la forme de la norme ET sa valeur pour cette soudure.
    const convexity = quality.limits.find((limit) => limit.iso6520 === "503")!;
    expect(screen.getByText(convexity.formula)).toBeInTheDocument();
    expect(screen.getAllByText(convexity.value_label!).length).toBeGreaterThan(0);
  });

  it("relaie l'erreur du moteur avec sa piste d'action", async () => {
    vi.mocked(weldingCatalogue).mockResolvedValue(catalogue);
    vi.mocked(weldingProcess).mockRejectedValue({
      message: "Entrée illisible : « 137 »",
      hint: "MecaTool ne connaît pas le numéro 137. Ce numéro peut exister.",
    });
    render(<Welding />);
    await userEvent.click(await screen.findByRole("button", { name: "Lire le procédé" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("peut exister");
  });
});
