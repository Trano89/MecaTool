# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), et le
projet adopte le [versionnage sémantique](https://semver.org/lang/fr/).

## [0.1.1] — 2026-09-11

### Renommé

- Le projet s'appelle désormais **MecaTool**.

### Ajouté — tolérancement géométrique (ISO 1101)

- **Catalogue des caractéristiques géométriques** : dix-sept entrées, quatre
  familles, vingt-deux modificateurs. Pour chacune, le symbole, l'exigence de
  référence spécifiée, l'exigence de dimension théorique exacte, et la forme de
  la zone que la norme lui attribue.
- **Lecture d'un cadre de tolérance** écrit en clair — `⟂ 0.05 A`,
  `⌖ ø0.2 (M) A B C` — et contrôle de ce que la source permet de contrôler :
  la référence spécifiée, la cotation de la zone, l'emboîtement entre
  spécifications posées sur un même élément.
- Écran dédié, avec le catalogue consultable.

**Aucune valeur de tolérance n'est proposée**, et deux tests échouent si un
message venait à en recommander une, l'un dans le moteur, l'autre dans
l'interface. L'ISO 1101 définit un vocabulaire, pas des chiffres.

### Ajouté — état de vérification « source secondaire »

Les données ISO 1101 viennent d'un **recueil technique** qui reproduit la norme,
et non de la norme. Entre « confronté à la norme » et « pas encore vérifié »,
il manquait ce cas :

| État | Ce qu'il dit |
|---|---|
| `verified` | chaque valeur confrontée à la norme citée |
| `secondary` | transcrite d'un recueil qui reproduit la norme |
| `unverified` | saisie, pas encore confrontée |

`secondary` **compte comme non vérifié** à l'affichage, mais avec son propre
message. Le ranger sous `verified` reviendrait à citer une norme qu'on n'a pas
ouverte ; sous `unverified`, à dire qu'on n'a rien fait.

Deux conséquences :

- **Le millésime peut rester vide.** Le recueil cite « SN EN ISO 1101 » sans
  année, et MecaTool ne l'invente pas : la citation rend `ISO 1101`, pas
  `ISO 1101:`.
- **La réserve ne rejoint pas le bandeau global.** Elle s'affiche sur l'écran
  concerné. Les autres sources sont vérifiées contre leur source primaire ;
  un avertissement affiché partout finirait par ne plus rien vouloir dire
  nulle part.

### Corrigé

- L'ISO 2768-2 était présentée comme « non couverte ». Elle a en réalité été
  **retirée au printemps 2021** et remplacée par l'ISO 22081. La note le dit
  désormais, et précise qu'une transposition directe des valeurs de l'une vers
  l'autre n'est pas possible.
- Une largeur de zone de tolérance s'affiche sans zéro superflu — `0.08`, non
  `0.080`. Une cote garde ses trois décimales : `10.015` se lit d'un coup d'œil
  sur un plan, une tolérance géométrique non, et les zéros ajoutés
  suggéreraient une précision de saisie qui n'est pas celle de la source.

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

En dessous de 0,5 mm, la norme renvoie à une cotation individuelle : MecaTool
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
- Outil de vérification en ligne de commande (`mecatool-cli`).

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
