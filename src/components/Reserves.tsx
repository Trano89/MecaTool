/**
 * Les réserves de source d'un domaine, affichées avant toute saisie.
 *
 * C'est la règle de la géométrie et des roulements, étendue aux domaines
 * suivants : une donnée insuffisamment vérifiée se signale **avant** qu'on
 * s'en serve, pas sous le résultat. Chaque réserve vient du moteur, mot pour
 * mot — l'interface n'en reformule aucune.
 */

interface Props {
  warnings: string[];
}

export function Reserves({ warnings }: Props) {
  if (warnings.length === 0) return null;
  return (
    <>
      {warnings.map((warning) => (
        <div key={warning} className="status caution" role="note">
          <span className="status-badge" aria-hidden="true">
            🟠
          </span>
          <div>
            <div className="status-headline">Origine des données</div>
            <div className="status-detail">{warning}</div>
          </div>
        </div>
      ))}
    </>
  );
}
