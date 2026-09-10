/**
 * Mise en forme des longueurs.
 *
 * Le moteur transmet des **nanomètres entiers**. Convertir en millimètres est
 * de la présentation, pas une règle normative — c'est la seule raison pour
 * laquelle ce calcul a le droit d'exister côté interface.
 *
 * Deux précautions, sans lesquelles l'interface contredirait le moteur :
 *
 * 1. **Arithmétique entière.** `10015000 / 1000000` en flottant peut produire
 *    `10.014999999999999`. On multiplie donc avant de diviser, sur des entiers
 *    qui restent très en deçà de `Number.MAX_SAFE_INTEGER`.
 * 2. **Arrondi au plus proche, moitié à l'opposé de zéro**, comme en Rust.
 *    `Math.round` arrondit vers +∞ et donnerait `-0.000` là où le moteur écrit
 *    `-0.001`.
 */

import type { Nanometres } from "./types";

const NM_PER_MM = 1_000_000;
const NM_PER_UM = 1_000;

/** Arrondi au plus proche, moitié à l'opposé de zéro. */
function roundHalfAwayFromZero(value: number): number {
  return value < 0 ? -Math.round(-value) : Math.round(value);
}

function toDecimalString(nm: Nanometres, scale: number, decimals: number): string {
  const factor = 10 ** decimals;
  const magnitude = Math.abs(nm);

  const quotient = roundHalfAwayFromZero((magnitude * factor) / scale);
  const sign = nm < 0 && quotient !== 0 ? "-" : "";

  if (decimals === 0) return `${sign}${quotient}`;

  const whole = Math.floor(quotient / factor);
  const frac = quotient - whole * factor;
  return `${sign}${whole}.${String(frac).padStart(decimals, "0")}`;
}

/** Une cote en millimètres, comme sur un plan : `"10.015"`. */
export function mm(nm: Nanometres, decimals = 3): string {
  return toDecimalString(nm, NM_PER_MM, decimals);
}

/** Une valeur en micromètres, sans zéro décimal superflu : `"15"`, `"10.5"`. */
export function um(nm: Nanometres): string {
  return trim(toDecimalString(nm, NM_PER_UM, 1));
}

/** Une valeur en micromètres, unité comprise : `"15 µm"`. */
export function umLabel(nm: Nanometres): string {
  return `${um(nm)} µm`;
}

/**
 * Un écart, signe explicite : `"+15 µm"`, `"−5 µm"`, `"0"`.
 *
 * Le zéro s'écrit sans signe ni unité : un écart nul n'est ni positif ni
 * négatif.
 */
export function deviation(nm: Nanometres): string {
  if (nm === 0) return "0";
  return nm > 0 ? `+${umLabel(nm)}` : umLabel(nm);
}

/** Une dimension nominale, sans zéro décimal superflu : `"20"`, `"25.4"`. */
export function nominal(nm: Nanometres): string {
  return trim(mm(nm, 4));
}

function trim(rendered: string): string {
  if (!rendered.includes(".")) return rendered;
  return rendered.replace(/0+$/, "").replace(/\.$/, "");
}
