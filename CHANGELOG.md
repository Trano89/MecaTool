# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), et le
projet adopte le [versionnage sémantique](https://semver.org/lang/fr/).

## [Non publié]

### Ajouté

**Socle**

- Arithmétique exacte : toute longueur est un entier signé de nanomètres.
  `0,1 + 0,2` vaut exactement `0,3`, une chaîne de 1000 cotes ne dérive pas, et
  le pouce tombe juste. Une valeur non représentable est rejetée, jamais
  arrondie en silence.
- Types du domaine avec invariants vérifiés à la construction : écarts, zones,
  échelons, classes de tolérance, ajustements.
- `Provenance` : un résultat ne peut pas exister sans dire d'où viennent ses
  chiffres. Une seule source non vérifiée contamine le résultat entier.

**ISO 286**

- Degrés de tolérance IT01 à IT18, 13 échelons jusqu'à 500 mm.
- Écarts fondamentaux des arbres pour 10 lettres : d, e, f, g, h, js, k, m, n, p.
- Règle de dérivation des alésages, correctif delta compris.
- Validation automatique des tables au chargement, et recoupements croisés entre
  tables saisies séparément.

**Fonctions**

- Calcul d'un ajustement ou d'un élément seul, avec le détail du raisonnement.
- Vérification d'une exigence de jeu, avec verdict et marges.
- Recherche des ajustements normalisés répondant à un besoin, avec diagnostic
  quand aucun ne convient exactement.
- Parser des désignations d'atelier (`Ø10 H7/g6`, `10 H7 g6`, `1 in H7/g6`), qui
  pose une question plutôt que de deviner.
- Diagramme des zones de tolérance, calculé en Rust et donc testable, avec
  export SVG et annonce d'échelle systématique.

**Application**

- Application de bureau Tauri + React : accueil, calcul, recherche.
- Thème clair, sombre ou système.
- Bandeau permanent sur l'état de vérification des données normatives.
- Outil de vérification en ligne de commande (`mecatol-cli`).

### Notes

Les tables normatives embarquées sont marquées `unverified` : elles ont été
saisies mais pas encore confrontées à une source primaire. L'application le
signale à chaque résultat. Voir [docs/standards.md](docs/standards.md).

Les lettres a, b, c et r à zc sont volontairement absentes : elles emploient des
échelons de dimensions nominales plus fins que les 13 échelons standards, et les
saisir sur ces derniers produirait des valeurs fausses d'apparence correcte.
