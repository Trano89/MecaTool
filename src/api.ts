/**
 * Appels au moteur.
 *
 * L'interface ne connaît que ces trois fonctions. Elle n'invoque jamais Tauri
 * directement, ce qui garde en un seul endroit la traduction des erreurs et
 * permet de faire tourner les composants sous test sans Tauri.
 */

import { invoke } from "@tauri-apps/api/core";

import type { AppError, Diagram, EngineInfo, Report } from "./types";

/** Vrai lorsque l'application tourne dans la fenêtre Tauri. */
export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Normalise ce que rejette `invoke`.
 *
 * Le moteur rejette un `AppError` sérialisé ; une panne d'un autre ordre
 * (commande absente, fenêtre fermée) rejette une chaîne ou une `Error`. Les
 * trois doivent arriver au composant sous la même forme.
 */
function toAppError(cause: unknown): AppError {
  if (
    typeof cause === "object" &&
    cause !== null &&
    "message" in cause &&
    typeof (cause as AppError).message === "string"
  ) {
    const error = cause as AppError;
    return { message: error.message, hint: error.hint ?? null };
  }
  if (typeof cause === "string") {
    return { message: cause, hint: null };
  }
  return {
    message: "Le moteur n'a pas répondu.",
    hint: cause instanceof Error ? cause.message : null,
  };
}

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isDesktop()) {
    throw {
      message: "Le moteur de calcul n'est pas accessible.",
      hint: "Mecatol doit être lancé comme application de bureau : « npm run app ».",
    } satisfies AppError;
  }
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    throw toAppError(cause);
  }
}

/**
 * Analyse une saisie, avec ou sans fenêtre de jeu.
 *
 * Une seule commande couvre le calcul, la vérification et la recherche : c'est
 * la saisie qui les distingue, et seul le moteur sait la lire.
 */
export function analyse(
  input: string,
  clearance?: string,
  trueToScale?: boolean,
): Promise<Report> {
  return call<Report>("analyse", { input, clearance, trueToScale });
}

/** Redessine un ajustement dans l'autre mode d'échelle, sans tout recalculer. */
export function rescaleDiagram(input: string, trueToScale: boolean): Promise<Diagram> {
  return call<Diagram>("rescale_diagram", { input, trueToScale });
}

/** État du moteur et de ses données, lu une fois au démarrage. */
export function engineInfo(): Promise<EngineInfo> {
  return call<EngineInfo>("engine_info", {});
}
