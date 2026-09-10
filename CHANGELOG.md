# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), et le
projet adopte le [versionnage sémantique](https://semver.org/lang/fr/).

## [Non publié]

### Ajouté — tolérances générales (ISO 2768-1)

- **Type `Angle` exact**, en millisecondes d'arc entières. Le choix n'est pas
  cosmétique : `0°20′` vaut un tiers de degré, inexprimable en degrés décimaux,
  mais exact en secondes d'arc. Sans lui, les tolérances angulaires générales
  seraient fausses dès la première valeur.
- Les **trois tables** de l'ISO 2768-1 : dimensions linéaires, arêtes abattues,
  dimensions angulaires, pour les quatre classes f, m, c, v.
- Écran dédié montrant **les quatre classes ensemble**. Choisir une classe
  suppose de voir ce que les autres donneraient ; afficher `m` seule répondrait
  à la mauvaise question.
- Les combinaisons que la norme ne définit pas — classe `f` au-delà de 2000 mm,
  classe `v` en dessous de 3 mm — restent visibles **avec leur raison**, plutôt
  que d'être masquées ou prises pour des zéros.

Trois particularités de cette norme, portées par le code plutôt que subies :

| Particularité | Conséquence |
|---|---|
| Borne basse du premier échelon **incluse** (« de 0,5 à 3 ») | Type d'échelon distinct de celui de l'ISO 286 |
| Échelons **ouverts** (« au-delà de 6 ») | Borne haute optionnelle, validée comme dernière seulement |
| Tolérances angulaires **décroissantes** avec la taille | Monotonie inversée dans la validation de cette table |

En dessous de 0,5 mm, la norme renvoie à une cotation individuelle : Mecatol
refuse de conclure et le dit, en citant la norme.

### Ajouté — version 0.1

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

**Vérification des données**

- Les 364 valeurs ISO 286 embarquées ont été confrontées à l'ISO 286-2:2010 :
  260 degrés de tolérance (tableau 1) et 104 écarts fondamentaux d'arbres
  (tableaux 18 à 26). **Aucun écart.**
- `scripts/verify-iso286-tables.py` rejoue la comparaison depuis le PDF de la
  norme. Une vérification qui ne se rejoue pas n'en est pas une.
- `k` et `js` échappent à l'automatisation, pour des raisons nommées dans le
  script : `k` varie avec le degré, `js` relève d'une règle et non d'une table.
  Les deux sont contrôlés autrement, et le script le dit.

### Notes

**Vérifié ne veut pas dire complet.** Les lettres a, b, c et r à zc restent
absentes : elles emploient des échelons de dimensions nominales plus fins que
les 13 échelons standards, et les saisir sur ces derniers produirait des valeurs
fausses d'apparence correcte. La couverture s'arrête à 500 mm là où la norme va
jusqu'à 3 150 mm.

Le moteur refuse explicitement tout ce qui manque, et chaque refus est couvert
par un test. Voir [docs/standards.md](docs/standards.md).
