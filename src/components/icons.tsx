/**
 * Les pictogrammes de la navigation.
 *
 * Tracés au trait, sur une grille de 16, à `currentColor` : ils prennent la
 * couleur du texte à côté d'eux et suivent donc les trois thèmes sans une seule
 * variante. Aucune police d'icônes, aucun fichier distant — le dessin est dans
 * le code, et l'application démarre sans réseau.
 *
 * Ils **doublent** un libellé, ils ne le remplacent jamais : `aria-hidden` est
 * posé sur chacun, et le nom accessible vient du texte du bouton. Dans le rail
 * étroit, où le libellé est masqué visuellement, c'est `aria-label` qui prend le
 * relais — jamais le pictogramme.
 *
 * Le vocabulaire est celui du dessin technique plutôt que celui des interfaces
 * grand public : une cote diamétrale pour les ajustements, un symbole de
 * perpendicularité pour la géométrie, une coupe hachurée pour les matières. Un
 * mécanicien doit reconnaître l'icône avant d'avoir lu le mot.
 */

import type { ReactElement, ReactNode } from "react";

interface IconProps {
  /** Taille en pixels. 16 dans la navigation, 20 sur les cartes d'accueil. */
  size?: number;
}

function Icon({ size = 16, children }: IconProps & { children: ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

export type IconComponent = (props: IconProps) => ReactElement;

/** Accueil. */
export const HomeIcon: IconComponent = (props) => (
  <Icon {...props}>
    <path d="M2.5 6.8 8 2.4l5.5 4.4V13a.9.9 0 0 1-.9.9H3.4a.9.9 0 0 1-.9-.9z" />
  </Icon>
);

/** Ajustements — une cote diamétrale. */
export const FitIcon: IconComponent = (props) => (
  <Icon {...props}>
    <circle cx="8" cy="8" r="5.9" />
    <path d="M3.6 8h8.8" />
    <path d="M5.3 6.5 3.7 8l1.6 1.5M10.7 6.5 12.3 8l-1.6 1.5" />
  </Icon>
);

/** Comparateur — des plages de longueurs différentes, alignées. */
export const CompareIcon: IconComponent = (props) => (
  <Icon {...props}>
    <path d="M2.6 4.2h10.8M2.6 8h6.4M2.6 11.8h8.6" />
  </Icon>
);

/** Tolérances générales — le cartouche où s'inscrit la classe. */
export const GeneralIcon: IconComponent = (props) => (
  <Icon {...props}>
    <rect x="2.2" y="3.2" width="11.6" height="9.6" rx="1.1" />
    <path d="M2.2 6.6h11.6M8 6.6v6.2" />
  </Icon>
);

/** Chaîne de cotes — deux maillons. */
export const ChainIcon: IconComponent = (props) => (
  <Icon {...props}>
    <rect x="1.4" y="5.6" width="7.2" height="4.8" rx="2.4" />
    <rect x="7.4" y="5.6" width="7.2" height="4.8" rx="2.4" />
  </Icon>
);

/** Tolérancement géométrique — le symbole de perpendicularité. */
export const GeometryIcon: IconComponent = (props) => (
  <Icon {...props}>
    <path d="M4.4 2.6v10.8M2.4 13.4h11.2" />
  </Icon>
);

/** États de surface — le profil d'une surface rugueuse sous sa ligne moyenne. */
export const SurfaceIcon: IconComponent = (props) => (
  <Icon {...props}>
    <path d="M2 5.4h12" />
    <path d="M2 11.6l2.5-3.2 2.5 3.2 2.5-3.2 2.5 3.2" />
  </Icon>
);

/** Roulements — une bague, ses éléments roulants, son alésage. */
export const BearingIcon: IconComponent = (props) => (
  <Icon {...props}>
    <circle cx="8" cy="8" r="5.9" />
    <circle cx="8" cy="8" r="2.5" />
    <circle cx="8" cy="3.6" r="0.85" />
    <circle cx="12.4" cy="8" r="0.85" />
    <circle cx="8" cy="12.4" r="0.85" />
    <circle cx="3.6" cy="8" r="0.85" />
  </Icon>
);

/** Soudure — le symbole du cordon d'angle sur sa ligne de référence. */
export const WeldingIcon: IconComponent = (props) => (
  <Icon {...props}>
    <path d="M1.6 11.8h12.8" />
    <path d="M4.6 11.8V6.4l4.4 5.4" />
  </Icon>
);

/** Visserie — une vis à tête, filetée. */
export const FastenerIcon: IconComponent = (props) => (
  <Icon {...props}>
    <rect x="4.4" y="1.9" width="7.2" height="2.6" rx="0.6" />
    <path d="M6 4.5v6.4L8 13.6l2-2.7V4.5" />
    <path d="M6 7h4M6 9.4h4" />
  </Icon>
);

/** Matières — une coupe hachurée. */
export const MaterialsIcon: IconComponent = (props) => (
  <Icon {...props}>
    <rect x="2.2" y="3.6" width="11.6" height="8.8" rx="1.1" />
    <path d="M4.2 12.4 9 7.6M7.6 12.4l4.4-4.4M2.6 9.2l5.4-5.4" />
  </Icon>
);

/** Recherche — la palette de commandes. */
export const SearchIcon: IconComponent = (props) => (
  <Icon {...props}>
    <circle cx="7.1" cy="7.1" r="4.5" />
    <path d="M10.4 10.4 13.8 13.8" />
  </Icon>
);

/** Le repère de la marque : une cible de référence spécifiée. */
export const MarkIcon: IconComponent = (props) => (
  <Icon {...props}>
    <circle cx="8" cy="8" r="4.6" />
    <path d="M8 1.6v12.8M1.6 8h12.8" />
  </Icon>
);

/**
 * Le pictogramme de repli.
 *
 * Un domaine que le moteur annoncerait sans que l'interface connaisse son
 * icône reste navigable : il prend ce carré plutôt que de disparaître.
 */
export const GenericIcon: IconComponent = (props) => (
  <Icon {...props}>
    <rect x="3" y="3" width="10" height="10" rx="2" />
  </Icon>
);
