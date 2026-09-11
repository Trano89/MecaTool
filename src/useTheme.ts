/**
 * Thème clair, sombre ou système.
 *
 * Le choix est conservé entre deux lancements. « Système » n'écrit aucun
 * attribut : la feuille de style suit alors `prefers-color-scheme`, ce qui
 * évite d'avoir à observer le réglage du système en JavaScript.
 */

import { useCallback, useEffect, useState } from "react";

export type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "mecatool.theme";

export const THEME_LABEL: Record<Theme, string> = {
  light: "Clair",
  dark: "Sombre",
  system: "Système",
};

function read(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") {
      return stored;
    }
  } catch {
    // Stockage indisponible (fenêtre privée, permissions) : le défaut suffit.
  }
  return "system";
}

function apply(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

export function useTheme(): [Theme, (next: Theme) => void] {
  const [theme, setTheme] = useState<Theme>(read);

  useEffect(() => {
    apply(theme);
  }, [theme]);

  const choose = useCallback((next: Theme) => {
    setTheme(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Le thème s'appliquera quand même, il ne survivra simplement pas au
      // redémarrage.
    }
  }, []);

  return [theme, choose];
}
