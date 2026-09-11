/**
 * La navigation, et pourquoi elle est ainsi.
 *
 * ## Pourquoi une barre latérale, et non des onglets en ligne
 *
 * L'ancienne barre supérieure alignait six onglets côte à côte. Ce n'était pas
 * un mauvais choix pour six écrans ; c'en est un pour douze. Trois raisons :
 *
 *  1. **La hauteur passe à l'échelle, la largeur non.** Une fenêtre gagne des
 *     lignes quand elle grandit, pas des colonnes utiles. Dix onglets en ligne
 *     imposent de tronquer les libellés, de les faire défiler, ou de replier les
 *     derniers dans un menu « … » — trois façons de cacher une fonction.
 *  2. **Les libellés français sont longs.** « Tolérancement géométrique »,
 *     « Tolérances générales » : en ligne, ils poussent tout le reste dehors. En
 *     colonne, ils tiennent sans abréviation.
 *  3. **Il y a une taxinomie à montrer.** Les domaines se regroupent —
 *     dimensionnel, géométrie, composants, matières — et ce regroupement aide à
 *     choisir. Une barre d'onglets ne sait pas exprimer un groupe ; une colonne
 *     le fait avec un intertitre.
 *
 * Le coût est une bande de 236 px prise en largeur. Il est compensé : sous
 * 1080 px la barre se réduit à un rail d'icônes, et la palette de commandes
 * (Ctrl+K) donne l'accès direct au clavier, sans passer par la barre.
 *
 * ## Rien n'est écrit à la main ici
 *
 * La liste des domaines vient du moteur (`domains()`). Ce module ne contient
 * que ce que le moteur ne peut pas connaître : quel pictogramme, quel écran, et
 * dans quel ordre présenter les groupes. Ajouter un domaine au registre le fait
 * apparaître dans la barre, dans l'accueil et dans la palette sans toucher un
 * seul composant.
 */

import type { Domain, DomainGroup } from "../types";
import {
  BearingIcon,
  ChainIcon,
  CompareIcon,
  FastenerIcon,
  FitIcon,
  GeneralIcon,
  GenericIcon,
  GeometryIcon,
  type IconComponent,
  MaterialsIcon,
  SurfaceIcon,
  WeldingIcon,
} from "./icons";

/**
 * L'écran affiché : l'accueil, ou l'identifiant d'un domaine du registre.
 *
 * Volontairement une chaîne, et non une union fermée : c'est le moteur qui
 * décide des domaines, et une union écrite ici devrait être rouverte à chaque
 * ajout — exactement ce que ce module cherche à éviter.
 */
export type Screen = string;

export const HOME: Screen = "home";

/**
 * Le pictogramme d'un domaine.
 *
 * Un identifiant absent de cette table retombe sur le pictogramme générique :
 * un domaine que le moteur annonce reste navigable même si l'interface ne le
 * connaît pas encore.
 */
const DOMAIN_ICON: Record<string, IconComponent> = {
  fit: FitIcon,
  compare: CompareIcon,
  general: GeneralIcon,
  chain: ChainIcon,
  geometry: GeometryIcon,
  surface: SurfaceIcon,
  bearing: BearingIcon,
  welding: WeldingIcon,
  fasteners: FastenerIcon,
  materials: MaterialsIcon,
};

export function iconFor(domainId: string): IconComponent {
  return DOMAIN_ICON[domainId] ?? GenericIcon;
}

/**
 * Vrai quand le domaine est utilisable.
 *
 * `blocked` ne masque pas : le domaine reste dans la barre, désactivé, avec sa
 * raison. C'est la règle de l'accueil depuis le début — cacher la feuille de
 * route ferait chercher en vain une fonction qui n'existe pas.
 */
export function isAvailable(domain: Domain): boolean {
  return domain.status !== "blocked";
}

export interface DomainGroupView {
  id: DomainGroup;
  label: string;
  domains: Domain[];
}

/**
 * Range les domaines par groupe.
 *
 * **Tout l'ordre vient du registre**, celui des groupes comme celui des domaines
 * à l'intérieur d'un groupe : un groupe apparaît à la place de son premier
 * domaine. Le frontend ne tient donc ni liste de groupes, ni table de libellés —
 * il n'aurait aucun moyen de les garder d'accord avec le moteur, et un groupe
 * ajouté côté moteur apparaîtrait ici sans rien changer.
 *
 * Le libellé vient lui aussi du registre, via `group_label`.
 */
export function groupDomains(domains: Domain[]): DomainGroupView[] {
  const groups: DomainGroupView[] = [];
  const index = new Map<DomainGroup, DomainGroupView>();

  for (const domain of domains) {
    const existing = index.get(domain.group);
    if (existing) {
      existing.domains.push(domain);
      continue;
    }
    const view: DomainGroupView = {
      id: domain.group,
      label: domain.group_label,
      domains: [domain],
    };
    index.set(domain.group, view);
    groups.push(view);
  }
  return groups;
}

/** Le domaine d'un écran, ou `undefined` pour l'accueil. */
export function domainOf(domains: Domain[], screen: Screen): Domain | undefined {
  return domains.find((domain) => domain.id === screen);
}

/**
 * Filtre les domaines sur une saisie libre.
 *
 * La recherche porte sur le **nom** et sur la **question** : on cherche souvent
 * ce qu'on veut obtenir (« jeu ») plutôt que le nom du module (« ajustements »).
 * Les accents sont neutralisés — personne ne tape « tolérancement » avec son
 * accent dans une palette.
 */
export function matches(domain: Domain, query: string): boolean {
  const needle = fold(query);
  if (needle === "") return true;
  return needle
    .split(/\s+/)
    .every(
      (word) =>
        fold(domain.name).includes(word) || fold(domain.question).includes(word),
    );
}

function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}
