/**
 * Rendu du diagramme de zones de tolérance.
 *
 * Ce composant **ne calcule aucune position**. Toutes les coordonnées viennent
 * du moteur, qui les dérive des valeurs exactes et les teste. Ici, on ne fait
 * que tracer des rectangles et des lignes aux abscisses et ordonnées fournies.
 *
 * Il ajoute deux choses que le moteur ne peut pas connaître : les couleurs du
 * thème en cours, et l'accessibilité du document (titre, description, texte de
 * remplacement).
 */

import { useId } from "react";

import type { Diagram } from "../types";

interface Props {
  diagram: Diagram;
}

export function DiagramView({ diagram }: Props) {
  // Au-delà d'une paire, l'espace par zone se réduit : les étiquettes d'écarts
  // se chevaucheraient. Le dessin porte alors les positions, le tableau porte
  // les valeurs.
  const dense = diagram.bands.length > 2;

  // Deux diagrammes sur une même page ne doivent pas partager leurs motifs.
  const uid = useId().replace(/:/g, "");
  const holeHatch = `hole-${uid}`;
  const shaftHatch = `shaft-${uid}`;
  const titleId = `title-${uid}`;
  const descId = `desc-${uid}`;

  const gapStart = (diagram.bands[0]?.x ?? 0) + (diagram.bands[0]?.width ?? 0);
  const gapEnd = diagram.bands[1]?.x ?? diagram.width;
  const gap = gapEnd - gapStart;

  const description = [
    diagram.title,
    ...diagram.bands.map(
      (band) => `${band.label} de ${band.lower_label} à ${band.upper_label}`,
    ),
    ...diagram.clearances.map((marker) => marker.label),
  ].join(". ");

  return (
    <figure className="diagram">
      <svg
        viewBox={`0 0 ${diagram.width} ${diagram.height}`}
        width="100%"
        role="img"
        aria-labelledby={`${titleId} ${descId}`}
        preserveAspectRatio="xMidYMid meet"
      >
        <title id={titleId}>Zones de tolérance {diagram.title}</title>
        <desc id={descId}>{description}</desc>

        <defs>
          {/* Les hachures portent l'information autant que les couleurs : le
              dessin reste lisible en noir et blanc. */}
          <pattern
            id={holeHatch}
            width="6"
            height="6"
            patternTransform="rotate(45)"
            patternUnits="userSpaceOnUse"
          >
            <line x1="0" y1="0" x2="0" y2="6" stroke="var(--hole)" strokeWidth="2.5" opacity="0.5" />
          </pattern>
          <pattern
            id={shaftHatch}
            width="6"
            height="6"
            patternTransform="rotate(-45)"
            patternUnits="userSpaceOnUse"
          >
            <line x1="0" y1="0" x2="0" y2="6" stroke="var(--shaft)" strokeWidth="2.5" opacity="0.5" />
          </pattern>
        </defs>

        {/* Ligne zéro : la référence de tout le dessin. */}
        <line
          x1="0"
          y1={diagram.zero_line_y}
          x2={diagram.width}
          y2={diagram.zero_line_y}
          stroke="var(--ink)"
          strokeWidth="1.25"
        />
        <text x="2" y={diagram.zero_line_y} dy="-5" fill="var(--ink)" fontWeight="700" fontSize="11">
          0
        </text>

        {diagram.bands.map((band, index) => {
          const isHole = band.feature === "hole";
          const colour = isHole ? "var(--hole)" : "var(--shaft)";
          // En mode fidèle, la hauteur réelle peut tomber sous le pixel : sans
          // minimum, le dessin effacerait purement et simplement les zones.
          const height = Math.max(band.bottom - band.top, 0.75);
          const centre = band.x + band.width / 2;
          const labelX = isHole ? band.x - 6 : band.x + band.width + 6;
          const anchor = isHole ? "end" : "start";

          // Sur un comparatif, les écarts de chaque zone se chevaucheraient :
          // ils se lisent dans le tableau, le dessin porte les positions.
          const showDeviations = !dense;
          // Le nom du groupe ne s'écrit qu'une fois par paire, sous l'alésage.
          const showGroup = dense && isHole && band.group !== null;

          return (
            <g key={`${band.group ?? ""}-${band.label}-${index}`}>
              <rect
                x={band.x}
                y={band.top}
                width={band.width}
                height={height}
                fill={`url(#${isHole ? holeHatch : shaftHatch})`}
                stroke={colour}
                strokeWidth="1.5"
              />
              <text
                x={centre}
                y={band.top - (dense ? 8 : 22)}
                fill={colour}
                textAnchor="middle"
                fontWeight="700"
                fontSize={dense ? 11 : 13}
              >
                {band.label}
              </text>
              {!dense ? (
                <text
                  x={centre}
                  y={band.top - 9}
                  fill="var(--ink-faint)"
                  textAnchor="middle"
                  fontSize="10"
                >
                  {band.it_label}
                </text>
              ) : null}
              {showGroup ? (
                <text
                  x={band.x + band.width}
                  y={diagram.height - 6}
                  fill="var(--ink-muted)"
                  textAnchor="middle"
                  fontWeight="600"
                  fontSize="11"
                >
                  {band.group}
                </text>
              ) : null}
              {showDeviations ? (
                <>
                  <text
                    x={labelX}
                    y={band.top}
                    dy="-2"
                    fill={colour}
                    textAnchor={anchor}
                    fontSize="10"
                  >
                    {band.upper_label}
                  </text>
                  <text
                    x={labelX}
                    y={band.top + height}
                    dy="9"
                    fill={colour}
                    textAnchor={anchor}
                    fontSize="10"
                  >
                    {band.lower_label}
                  </text>
                </>
              ) : null}
            </g>
          );
        })}

        {diagram.clearances.map((marker, index) => {
          const x = gapStart + gap * (index === 0 ? 0.34 : 0.72);
          const top = Math.min(marker.hole_y, marker.shaft_y);
          const bottom = Math.max(marker.hole_y, marker.shaft_y);
          const stroke = marker.is_interference ? "var(--shaft)" : "var(--ink)";

          return (
            <g key={marker.kind}>
              {/* Lignes d'attache : elles montrent *quelles* arêtes sont mesurées. */}
              <line
                x1={gapStart}
                y1={marker.hole_y}
                x2={x}
                y2={marker.hole_y}
                stroke={stroke}
                strokeWidth="0.6"
                opacity="0.45"
              />
              <line
                x1={x}
                y1={marker.shaft_y}
                x2={gapEnd}
                y2={marker.shaft_y}
                stroke={stroke}
                strokeWidth="0.6"
                opacity="0.45"
              />
              <line x1={x} y1={top} x2={x} y2={bottom} stroke={stroke} strokeWidth="1" />
              {[top, bottom].map((y) => (
                <line
                  key={y}
                  x1={x - 3.5}
                  y1={y}
                  x2={x + 3.5}
                  y2={y}
                  stroke={stroke}
                  strokeWidth="1.4"
                />
              ))}
              <text
                x={x}
                y={top}
                dy="-6"
                fill={stroke}
                textAnchor="middle"
                fontSize="10"
                fontWeight="600"
              >
                {marker.label}
              </text>
            </g>
          );
        })}
      </svg>

      {/* L'annonce d'échelle n'est jamais masquée : sans elle, le lecteur peut
          croire que le dessin respecte les proportions de la pièce. */}
      <figcaption>{diagram.scale_note}</figcaption>
    </figure>
  );
}
