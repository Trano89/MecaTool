/**
 * La barre latérale.
 *
 * Elle ne connaît aucun écran : elle reçoit le registre des domaines et le
 * restitue, groupe par groupe. Le raisonnement qui a conduit à une latérale
 * plutôt qu'à des onglets est dans `navigation.ts`.
 *
 * Un domaine bloqué n'est pas retiré de la liste. Il reste à sa place, porte la
 * mention « à venir » **en toutes lettres** — pas seulement un gris — et garde
 * le focus clavier, avec la raison de son indisponibilité en infobulle. Un
 * élément que l'on ne peut ni voir ni atteindre ne s'explique pas.
 */

import type { Domain } from "../types";
import { HomeIcon, MarkIcon } from "./icons";
import { groupDomains, HOME, iconFor, isAvailable, type Screen } from "./navigation";

interface Props {
  domains: Domain[];
  screen: Screen;
  onNavigate: (screen: Screen) => void;
  /** La version du moteur, quand il a répondu. */
  version: string;
  themeControl: React.ReactNode;
}

export function Sidebar({ domains, screen, onNavigate, version, themeControl }: Props) {
  const groups = groupDomains(domains);

  return (
    <aside className="sidebar">
      <div className="sidebar-head">
        <div className="wordmark">
          <span className="wordmark-mark">
            <MarkIcon size={13} />
          </span>
          <span className="wordmark-text">
            MECATOOL <small>{version}</small>
          </span>
        </div>
      </div>

      <nav className="sidebar-scroll" aria-label="Navigation principale">
        <div className="nav-group">
          <ul className="nav-list">
            <li>
              <button
                type="button"
                className="nav-item"
                aria-label="Accueil"
                aria-current={screen === HOME ? "page" : undefined}
                onClick={() => onNavigate(HOME)}
              >
                <span className="nav-icon">
                  <HomeIcon />
                </span>
                <span className="nav-label">Accueil</span>
              </button>
            </li>
          </ul>
        </div>

        {groups.map((group) => (
          <div className="nav-group" key={group.id}>
            <div className="nav-group-label" id={`nav-group-${group.id}`}>
              {group.label}
            </div>
            <ul className="nav-list" aria-labelledby={`nav-group-${group.id}`}>
              {group.domains.map((domain) => {
                const DomainIcon = iconFor(domain.id);
                const available = isAvailable(domain);
                return (
                  <li key={domain.id}>
                    <button
                      type="button"
                      className="nav-item"
                      aria-label={domain.name}
                      aria-current={screen === domain.id ? "page" : undefined}
                      // `aria-disabled` plutôt que `disabled` : le bouton reste
                      // atteignable au clavier, donc son infobulle aussi.
                      aria-disabled={available ? undefined : true}
                      title={available ? domain.question : (domain.unavailable ?? undefined)}
                      onClick={() => {
                        if (available) onNavigate(domain.id);
                      }}
                    >
                      <span className="nav-icon">
                        <DomainIcon />
                      </span>
                      <span className="nav-label">{domain.name}</span>
                      {available ? null : <span className="nav-flag">à venir</span>}
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
      </nav>

      <div className="sidebar-foot">{themeControl}</div>
    </aside>
  );
}
