/**
 * L'ossature de l'application.
 *
 * Trois pièces, et rien d'autre : une barre latérale qui restitue le registre
 * des domaines, une barre supérieure qui dit où l'on est, et le plan de travail
 * qui porte l'écran courant.
 *
 * **Aucun écran n'est nommé ici.** La liste des domaines vient du moteur
 * (`domains()`) ; ce fichier ne fait que dire quel composant rend quel
 * identifiant. Ajouter un domaine au registre le fait apparaître dans la barre,
 * dans l'accueil et dans la palette sans toucher à la navigation — c'est la
 * condition pour que les roulements, la soudure, les matières et le reste
 * n'imposent pas de refonte à chaque fois.
 *
 * Le bandeau sur l'état des données normatives est permanent et non masquable.
 * Les domaines qui portent une réserve propre — géométrie, roulements — la
 * répètent sur leur écran, avant toute saisie : c'est là qu'elle est utile, et
 * un avertissement affiché partout finirait par ne plus rien vouloir dire nulle
 * part.
 */

import { useCallback, useEffect, useState } from "react";

import { domains as loadDomains, engineInfo, isDesktop } from "./api";
import { CommandPalette } from "./components/CommandPalette";
import { HOME, type Screen } from "./components/navigation";
import { SearchIcon } from "./components/icons";
import { Sidebar } from "./components/Sidebar";
import type { Domain, EngineInfo } from "./types";
import { THEME_LABEL, useTheme, type Theme } from "./useTheme";
import { Bearings } from "./views/Bearings";
import { Calculate, type Query } from "./views/Calculate";
import { Chain } from "./views/Chain";
import { Compare } from "./views/Compare";
import { GeneralTolerances } from "./views/GeneralTolerances";
import { Geometry } from "./views/Geometry";
import { Home } from "./views/Home";

const THEMES: readonly Theme[] = ["light", "dark", "system"];

export function App() {
  const [screen, setScreen] = useState<Screen>(HOME);
  const [query, setQuery] = useState<Query>({ input: "", clearance: "" });
  const [info, setInfo] = useState<EngineInfo | null>(null);
  const [catalogue, setCatalogue] = useState<Domain[]>([]);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [theme, setTheme] = useTheme();

  useEffect(() => {
    if (!isDesktop()) {
      // Hors de la fenêtre Tauri, le moteur est injoignable et le registre avec
      // lui. La coquille resterait alors sans navigation, ce qui rend
      // l'application impossible à relire pendant son développement. En mode
      // développement seulement, on retombe sur l'échantillon que le moteur
      // exporte lui-même pour les tests — jamais dans une version livrée, où
      // `import.meta.env.DEV` vaut `false` et où ce bloc disparaît au bundling.
      // Le bandeau « moteur inaccessible » reste affiché : rien n'est masqué.
      if (import.meta.env.DEV) {
        void import("./fixtures/domains.json").then((sample) => {
          setCatalogue(sample.default as Domain[]);
        });
      }
      return;
    }
    engineInfo()
      .then(setInfo)
      .catch(() => {
        // L'application reste utilisable : le bandeau d'état sera simplement
        // absent, et chaque résultat porte de toute façon ses propres réserves.
      });
    loadDomains()
      .then(setCatalogue)
      .catch(() => {
        // Sans registre, seul l'accueil reste atteignable — et il dira lui-même
        // que le moteur n'a pas répondu. Mieux vaut cela qu'une liste de
        // domaines écrite en dur, qui mentirait le jour où le moteur change.
      });
  }, []);

  // Ctrl+K / ⌘K, la convention de toutes les palettes de commandes.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const navigate = useCallback((next: Screen) => {
    setScreen(next);
    // Un changement d'écran repart du haut : la position de défilement de
    // l'écran précédent n'a aucun sens dans le suivant.
    document.querySelector(".main")?.scrollTo({ top: 0 });
  }, []);

  const current = catalogue.find((domain) => domain.id === screen);

  const themeControl = (
    <fieldset className="field" style={{ border: 0, padding: 0, margin: 0 }}>
      <legend className="field-label" style={{ padding: 0 }}>
        Thème
      </legend>
      <div className="segmented">
        {THEMES.map((option) => (
          <button
            key={option}
            type="button"
            aria-pressed={theme === option}
            onClick={() => setTheme(option)}
          >
            {THEME_LABEL[option]}
          </button>
        ))}
      </div>
    </fieldset>
  );

  return (
    <div className="app">
      <Sidebar
        domains={catalogue}
        screen={screen}
        onNavigate={navigate}
        version={info?.app_version ?? "0.1.0"}
        themeControl={themeControl}
      />

      <div className="workspace">
        <header className="topbar">
          <div className="crumbs">
            {current ? (
              <>
                <span>{current.group_label}</span>
                <span className="crumb-sep" aria-hidden="true">
                  /
                </span>
                <span className="crumb-current">{current.name}</span>
              </>
            ) : (
              <span className="crumb-current">Accueil</span>
            )}
          </div>

          <div className="topbar-actions">
            <button
              type="button"
              className="cmd-trigger"
              onClick={() => setPaletteOpen(true)}
            >
              <span className="nav-icon">
                <SearchIcon size={14} />
              </span>
              <span className="cmd-trigger-label">Aller à…</span>
              <kbd>Ctrl</kbd>
              <kbd>K</kbd>
            </button>
          </div>
        </header>

        <main className="main">
          <div className="page">
            {!isDesktop() ? (
              <div className="banner" role="alert">
                <span className="banner-icon" aria-hidden="true">
                  ⚠
                </span>
                <div>
                  <strong>Moteur de calcul inaccessible.</strong> Cette page est ouverte hors de
                  l'application. Lancez MecaTool avec « npm run app » pour accéder au moteur.
                </div>
              </div>
            ) : null}

            {info && info.warnings.length > 0 ? (
              <div className="banner" role="note">
                <span className="banner-icon" aria-hidden="true">
                  ⚠
                </span>
                <div>
                  {/* Le bandeau ne qualifie plus les réserves lui-même : depuis
                      qu'une source peut être « recommandée » sans être
                      normative, une phrase unique les décrirait de travers.
                      Chaque réserve dit sa propre nature. */}
                  <strong>Réserves sur les sources embarquées.</strong> Les résultats sont
                  calculés exactement, mais certaines de leurs valeurs de base appellent une
                  réserve. Chacune dit laquelle.
                  <details className="why">
                    <summary>Sources concernées</summary>
                    <div className="steps">
                      {info.warnings.map((warning) => (
                        <div key={warning}>
                          <p className="step-expression" style={{ margin: 0 }}>
                            {warning}
                          </p>
                        </div>
                      ))}
                    </div>
                  </details>
                </div>
              </div>
            ) : null}

            <Screen
              screen={screen}
              domain={current}
              domains={catalogue}
              query={query}
              onQueryChange={setQuery}
              onNavigate={navigate}
            />
          </div>
        </main>
      </div>

      <CommandPalette
        open={paletteOpen}
        domains={catalogue}
        onClose={() => setPaletteOpen(false)}
        onNavigate={navigate}
      />
    </div>
  );
}

interface ScreenProps {
  screen: Screen;
  domain: Domain | undefined;
  domains: Domain[];
  query: Query;
  onQueryChange: (query: Query) => void;
  onNavigate: (screen: Screen) => void;
}

/**
 * Le routage, et lui seul.
 *
 * La correspondance entre un identifiant de domaine et un composant est la
 * seule chose que le moteur ne peut pas transmettre. Tout le reste — nom,
 * question, groupe, état — vient de lui.
 */
function Screen({ screen, domain, domains, query, onQueryChange, onNavigate }: ScreenProps) {
  switch (screen) {
    case HOME:
      return <Home domains={domains} onOpen={onNavigate} />;
    case "fit":
      return <Calculate query={query} onQueryChange={onQueryChange} />;
    case "compare":
      return <Compare />;
    case "general":
      return <GeneralTolerances />;
    case "chain":
      return <Chain />;
    case "geometry":
      return <Geometry />;
    case "bearing":
      return <Bearings />;
    default:
      // Un domaine que le registre annonce et que cette version de l'interface
      // ne sait pas encore rendre. Le dire vaut mieux qu'un écran blanc.
      return (
        <div className="status insufficient-data" role="status">
          <span className="status-badge" aria-hidden="true">
            🔵
          </span>
          <div>
            <div className="status-headline">ÉCRAN INDISPONIBLE</div>
            <div className="status-detail">
              Le moteur annonce le domaine « {domain?.name ?? screen} », mais cette version de
              l'interface n'a pas encore d'écran pour lui.
            </div>
          </div>
        </div>
      );
  }
}
