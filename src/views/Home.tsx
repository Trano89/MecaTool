/**
 * L'écran d'accueil.
 *
 * Une question, des cartes. L'utilisateur n'a pas à connaître la norme ni le
 * vocabulaire pour démarrer : il reconnaît **sa** question et clique dessus.
 * C'est pour cela que la carte met la question en avant autant que le nom du
 * domaine — « Que valent les cotes qui ne portent pas de tolérance ? » se
 * reconnaît, « Tolérances générales » se sait déjà.
 *
 * Rien n'est écrit ici : les cartes sont le registre des domaines du moteur,
 * groupées comme dans la barre latérale. Un domaine ajouté au moteur apparaît
 * sans toucher ce fichier.
 *
 * Les modules non encore construits sont montrés **désactivés** plutôt que
 * masqués, avec la raison : cacher la feuille de route ferait chercher en vain
 * une fonction qui n'existe pas, et laisserait croire qu'elle n'est pas prévue.
 */

import type { Domain } from "../types";
import { groupDomains, iconFor, isAvailable, type Screen } from "../components/navigation";

interface Props {
  domains: Domain[];
  onOpen: (screen: Screen) => void;
}

export function Home({ domains, onOpen }: Props) {
  const groups = groupDomains(domains);

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Que voulez-vous calculer ?</h1>
        <p className="lead">
          Choisissez la question qui est la vôtre. Chaque domaine dit d'où viennent ses valeurs,
          et ce qu'il ne sait pas faire.
        </p>
      </div>

      {groups.length === 0 ? (
        <p className="muted">
          Le registre des domaines n'a pas pu être lu : le moteur n'a pas répondu.
        </p>
      ) : null}

      {groups.map((group) => (
        <section className="stack" key={group.id} style={{ gap: "var(--s-5)" }}>
          <div className="nav-group-label" style={{ padding: 0 }}>
            {group.label}
          </div>
          <div className="cards">
            {group.domains.map((domain) => {
              const DomainIcon = iconFor(domain.id);
              const available = isAvailable(domain);
              return (
                <button
                  key={domain.id}
                  type="button"
                  className="entry-card"
                  disabled={!available}
                  onClick={() => onOpen(domain.id)}
                >
                  <span className="entry-card-head">
                    <span className="nav-icon">
                      <DomainIcon size={18} />
                    </span>
                    <h3>{domain.name}</h3>
                  </span>
                  <p>{domain.question}</p>

                  {/* Les exemples illustrent, ils ne pré-remplissent pas : le
                      registre les donne en clair (« Ø20 + jeu 10..30 »), et
                      tous ne sont pas des saisies que le parseur accepterait. */}
                  {available && domain.examples.length > 0 ? (
                    <span className="entry-examples">
                      {domain.examples.slice(0, 2).map((example) => (
                        <code key={example}>{example}</code>
                      ))}
                    </span>
                  ) : null}

                  {available ? null : (
                    <>
                      <span className="later">à venir</span>
                      {domain.unavailable ? (
                        <span className="entry-reason">{domain.unavailable}</span>
                      ) : null}
                    </>
                  )}
                </button>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}
