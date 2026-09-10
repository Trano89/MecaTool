/**
 * L'ossature de l'application.
 *
 * Deux écrans en version 0.1 : l'accueil et le calcul. Le bandeau d'état des
 * données normatives est permanent et non masquable — tant que les tables n'ont
 * pas été confrontées à une source primaire, aucun écran ne doit laisser croire
 * le contraire.
 */

import { useEffect, useState } from "react";

import { engineInfo, isDesktop } from "./api";
import type { EngineInfo } from "./types";
import { THEME_LABEL, useTheme, type Theme } from "./useTheme";
import { Calculate, type Query } from "./views/Calculate";
import { Home } from "./views/Home";

type Screen = "home" | "calculate";

const THEMES: readonly Theme[] = ["light", "dark", "system"];

export function App() {
  const [screen, setScreen] = useState<Screen>("home");
  const [query, setQuery] = useState<Query>({ input: "", clearance: "" });
  const [info, setInfo] = useState<EngineInfo | null>(null);
  const [theme, setTheme] = useTheme();

  useEffect(() => {
    if (!isDesktop()) return;
    engineInfo()
      .then(setInfo)
      .catch(() => {
        // L'application reste utilisable : le bandeau d'état sera simplement
        // absent, et chaque résultat porte de toute façon ses propres réserves.
      });
  }, []);

  const start = (next: Query) => {
    setQuery(next);
    setScreen("calculate");
  };

  return (
    <div className="app">
      <header className="topbar">
        <div className="wordmark">
          MECATOL <small>{info?.app_version ?? "0.1.0"}</small>
        </div>

        <nav aria-label="Navigation principale">
          <button
            type="button"
            className="tab"
            aria-current={screen === "home" ? "page" : undefined}
            onClick={() => setScreen("home")}
          >
            Accueil
          </button>
          <button
            type="button"
            className="tab"
            aria-current={screen === "calculate" ? "page" : undefined}
            onClick={() => setScreen("calculate")}
          >
            Calculer
          </button>
        </nav>

        <div className="spacer" />

        <fieldset
          style={{ border: 0, padding: 0, margin: 0, display: "flex", gap: "0.25rem" }}
        >
          <legend className="visually-hidden">Thème de l'interface</legend>
          {THEMES.map((option) => (
            <button
              key={option}
              type="button"
              className="btn-quiet"
              aria-pressed={theme === option}
              onClick={() => setTheme(option)}
            >
              {THEME_LABEL[option]}
            </button>
          ))}
        </fieldset>
      </header>

      <main className="main">
        <div className="page">
          {!isDesktop() ? (
            <div className="banner" role="alert">
              <span aria-hidden="true">⚠</span>
              <div>
                <strong>Moteur de calcul inaccessible.</strong> Cette page est ouverte hors de
                l'application. Lancez Mecatol avec « npm run app » pour accéder au moteur.
              </div>
            </div>
          ) : null}

          {info && info.warnings.length > 0 ? (
            <div className="banner" role="note">
              <span aria-hidden="true">⚠</span>
              <div>
                <strong>Données normatives non vérifiées.</strong> Les tables embarquées ont été
                saisies mais pas encore confrontées à une source primaire. Les résultats sont
                calculés exactement, mais leurs valeurs de base restent à contrôler.
                <details className="why" style={{ marginTop: "0.5rem", background: "transparent" }}>
                  <summary>Sources concernées</summary>
                  <div className="steps">
                    {info.warnings.map((warning) => (
                      <p key={warning} className="step-expression" style={{ margin: 0 }}>
                        {warning}
                      </p>
                    ))}
                  </div>
                </details>
              </div>
            </div>
          ) : null}

          {screen === "home" ? (
            <Home onStart={start} />
          ) : (
            <Calculate query={query} onQueryChange={setQuery} />
          )}
        </div>
      </main>
    </div>
  );
}
