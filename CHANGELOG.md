# Journal des modifications

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), et le
projet adopte le [versionnage sémantique](https://semver.org/lang/fr/).

## [0.3.0] — 2026-10-07

### Vérifié — données confrontées aux normes elles-mêmes

L'utilisateur a fourni plusieurs normes. Chaque jeu concerné a été relu sur
la norme, valeur par valeur, et porte désormais l'état « vérifié » quand tout
ce qu'il contient y a été lu.

- **ISO 4063:2009** (version corrigée 2010, NF EN ISO 4063:2011) : la
  sélection de 46 numéros devient la **nomenclature complète**, 157 numéros.
  Un numéro absent n'appartient donc plus à la norme, et MecaTool le dit.
  - Intitulés corrigés : 81 est le « coupage à la flamme » et non
    l'oxycoupage ; 91 et 94 sont le brasage fort et le brasage tendre **avec
    chauffage local**, non le brasage en général ; 135 et 131 sont « avec
    fil-électrode fusible », non « plein » ; 786 est à amorçage « par
    contact ».
  - Variantes de l'article 2.2 lues : mode de transfert (`131-D`), nombre
    d'électrodes (`131-2`), fil froid ou chaud (`121-C`). Procédés hybrides
    de l'article 2.3 (`522+15`).
  - Numéros remplacés ou dépassés de l'Annexe A (`137`, `181`…) : signalés
    comme tels, non comme inconnus.
  - Désignations US de l'Annexe B (`SMAW`, `GTAW`, `FCAW`…), seulement
    celles que la norme donne pour exactement équivalentes.
  - La hiérarchie admet un procédé sans groupe intermédiaire : la norme range
    185 directement sous 1.
- **EN 10025-2:2019** : les valeurs embarquées étaient justes. La table est
  complétée jusqu'à 400 mm (trois échelons de plus). Elle gagne S185
  (Tableau 7, sans qualité), S460 et S500 (produits longs seulement,
  rappelé à la lecture).

- **ISO 5817:2014** : toutes les lignes embarquées relues sur le Tableau 1.
  - Une limite était fausse : la fissure de cratère (1.2) n'est admise à
    **aucun** niveau, D compris.
  - Le défaut d'alignement (3.1) se lit 5071 (tôles) et 5072 (profils creux),
    non 507.
  - **Procédés visés** : la norme vise les procédés 11 à 15, et 31 pour
    l'acier seulement (article 1). Le soudage sous laitier (72) n'est pas
    visé ; le soudage par résistance n'est pas « exclu » mais non cité.
    Chaque procédé d'un procédé hybride passe ce contrôle.
  - Le « défaut court » se compte sur les 100 mm de soudure qui en
    contiennent le plus, non sur toute longueur de 100 mm.
  - Les niveaux B, C et D ne portent plus de noms inventés : la norme ne dit
    que leur rang.
  - Six lignes ajoutées (1.5 micromanque de fusion, 1.8, 1.14, 1.17, 1.18,
    1.19). Les défauts internes et les défauts multiples ne sont pas encore
    embarqués, et l'écran le dit.

- **EN 10027-1:2005** (édition remplacée par celle de 2016, qui n'a pas été
  confrontée : chaque lecture le rappelle).
  - Erreur de fond corrigée : une seule liste de symboles additionnels
    servait à tous les groupes, alors que la norme en donne une par tableau.
    `P265GH` lisait H « profil creux » au lieu de « température élevée » ;
    `Y1770C` lisait C « formage à froid » au lieu de « fil tréfilé ».
  - Groupes ajoutés : D, H (six formes), T, M (Tableaux 8 à 11). Les groupes
    L, B, R, Y retrouvent leurs propres symboles : `L360NB`, `B500A`,
    `R320Cr`, `Y1770S7` se lisent.
  - Non alliés : `C20D2` et `C2D1` se lisent ; E ou R suivi d'un chiffre donne
    la teneur en soufre. Fortement alliés : `-N5`. Aciers rapides : symbole
    final (`HS6-5-2C`). Préfixe PM (métallurgie des poudres).
  - Les symboles après « + » ne sont plus « lus sans être interprétés » :
    les Tableaux 16 à 18 les définissent (`C45+A` recuit, `DX51D+Z`
    galvanisé, `+AR` brut de laminage).
  - Les 88 exemples des Tableaux 1 à 15 sont testés.
- **ISO 2553:2013, cotation (article 5)** : la surcouche devient vérifiée.
  - Les soudures évasées (8, 9) sont des soudures bout à bout, cotées en `s`,
    et leur cote est **obligatoire** (§ 5.4.4) : MecaTool la réclame.
  - La soudure sur chant (19) et le rechargement (21) se cotent en `s`,
    épaisseur de métal fondu ou du rechargement.
  - Définition de `z` corrigée (§ 3.17) ; `s` est la profondeur de
    pénétration.
  - Formes doubles : seuls le double V, le K et le double U ont un nom dans
    la norme ; les six autres noms, inventés, sont retirés.
  - Symboles supplémentaires rattachés aux familles où la norme les montre.
- **ISO 2553:2019** : comparée à l'édition 2013 sans changer la référence.
  Ses différences (renommage de 8, 9, 11, 21 ; 12 et 20 scindés ; cote `d`
  de la soudure par transparence) sont relevées dans
  [docs/standards.md](docs/standards.md).

### Corrigé

- Toutes les sources de la soudure étant lues dans les normes, l'écran de la
  soudure n'affiche plus de réserve et le domaine passe à « prêt ».

## [0.2.0] — 2026-10-07

### Ajouté — matières

Le dernier domaine prévu s'ouvre, sur données non vérifiées comme les trois
précédents (`data/matieres/`).

- **Désignation des aciers par règles** (EN 10027-1) : groupes d'emploi
  (`S355J2`), résilience lue par règle (`J2` = 27 J à −20 °C), symboles
  additionnels, aciers non alliés (`C45E`), faiblement alliés avec leurs
  facteurs de teneur (`42CrMo4` : Cr = 4 / 4 = 1 %), fortement alliés
  (`X5CrNi18-10`), aciers rapides (`HS6-5-2`), aciers moulés, état de livraison.
  Un faiblement allié dont un élément atteindrait 5 % est signalé : il aurait
  dû s'écrire en X.
- **Aciers de construction** (EN 10025-2) : limite d'élasticité et résistance
  selon l'épaisseur, S235, S275, S355. Le chargement vérifie que la limite
  décroît avec l'épaisseur et que la première valeur est celle de la
  désignation.
- **Propriétés physiques** par famille, ordres de grandeur : E, ν, ρ, α.
- **Ajustement à chaud** : alésage et arbre de matières différentes, la
  variation du jeu se calcule en nanomètres entiers et l'ajustement ISO 286 est
  repris à froid et à chaud. Un serrage qui disparaît ou un jeu qui devient
  négatif est une faute.
- Refus : une désignation bien formée n'est jamais dite exister ; la
  désignation numérique (`1.4301`) se lit sans être devinée.

### Ajouté — choisir plutôt que taper

- Composant de choix par boutons, commun à tous les écrans, dont les options
  viennent du moteur.
- **Chaîne de cotes** : un maillon s'écrit `A = 20 h11` ou `A = 20 ISO 2768-m`,
  et le moteur lit les écarts lui-même ; la chaîne porte la provenance de ses
  maillons. Le composeur est ouvert d'office, et la tolérance d'un maillon s'y
  choisit par boutons.
- **États de surface** : symbole, paramètre, valeur (série des classes N),
  règle, stries et procédé se composent par boutons.
- **Visserie** : diamètre, pas (ceux du diamètre retenu) et classe de qualité
  par boutons.
- **Soudure** : le procédé se choisit dans la nomenclature embarquée.
- Workflow de release : un tag `vX.Y.Z` construit les paquets Windows, macOS et
  Linux et les joint à une release GitHub.


### Ajouté — états de surface, soudure et visserie, sur données non vérifiées

Les états de surface, la soudure et la visserie passent de « à venir » à
**utilisables**. Leurs données ont été saisies **sans document normatif ouvert** :
elles portent l'état `unverified`, chaque écran l'affiche avant toute saisie, et
chaque fichier de `data/` dit contre quoi le confronter. Le registre en déduit
seul l'état « réserve » des trois domaines.

**États de surface** (`data/surface/`)

- Lecture d'une indication : variante du symbole (`APA`, `MRR`, `NMR`),
  paramètre d'amplitude, limite haute ou basse, règle du maximum, sens des
  stries, procédé cité. Une classe N (`N7`) se lit en Ra, avec le rappel que
  l'ISO 1302:1992 qui les définissait est retirée.
- Rugosité par procédé : chaque procédé est situé par rapport à l'exigence —
  d'ordinaire, avec des soins, hors d'atteinte, ou plus fin que nécessaire. Le
  symbole écarte les procédés de l'autre famille, et une contradiction entre
  symbole et procédé cité est une faute.
- Graphique des plages par procédé, géométrie calculée en Rust, sur l'échelle
  des classes N.
- **Le blocage des deux générations de normes est levé en retrécissant le
  périmètre** : seul ce que l'ISO 21920 et l'ISO 4287/1302 ont en commun est
  embarqué. Longueurs de base, filtres et règle d'acceptation par défaut ne le
  sont pas, et le moteur le dit.
- Refus : aucune conversion entre Rz et Ra.

**Soudure** (`data/soudure/`)

- Numéros de procédés ISO 4063, avec leur hiérarchie. Un nom d'atelier (`MAG`,
  `TIG`) rend toutes ses lectures ; un numéro absent n'est pas dit inexistant.
- Symboles de l'**ISO 2553:2013 lue dans la norme** (`verified`) : vingt-deux
  symboles élémentaires, six supplémentaires, systèmes A et B. Le recueil, qui
  reproduit l'édition de 1992, n'est pas suivi. La cotation — famille, cotes
  admises, forme double — est une surcouche non vérifiée, accrochée au tableau
  par son numéro et recoupée avec la pleine pénétration que la norme déclare.
- Lecture d'un symbole complet : symbole élémentaire et côté, cote
  (`a`, `z`, `s`, `d`, `c`), discontinuité `n × l (e)`, symboles
  supplémentaires, procédé, niveau de qualité. Restitution en clair, constats à
  l'appui. `z = a·√2` est calculé en entiers et annoncé arrondi.
- Limites ISO 5817 chiffrées pour la géométrie saisie, niveaux B, C, D. Une
  limite dont la grandeur manque reste écrite sous sa forme littérale.
- La validation au chargement vérifie qu'**un niveau plus exigeant ne tolère
  jamais davantage** : une valeur saisie dans la mauvaise colonne fait tomber le
  chargement.
- Refus : aucun niveau de qualité recommandé ; pas de limites ISO 5817 pour le
  brasage, le soudage par résistance ou par faisceau.

**Visserie** (`data/visserie/`)

- Désignations `M10`, `M12 x 1.5`, `M10-6g`, `M8 8.8`. Pas gros ou fin, choix
  de diamètre, pas non listé signalé mais calculable.
- Diamètres de base par la règle du profil ISO 68-1, en entiers au femtomètre,
  annoncés au micromètre : ils retrouvent les tableaux de l'ISO 724.
- Section résistante As, perçage avant taraudage `D − P` présenté comme règle
  d'atelier.
- Trous de passage ISO 273 en trois séries, **poursuivis jusqu'aux écarts réels
  par le moteur ISO 286** — deux natures de source dans un même résultat, comme
  pour les roulements.
- Classe de qualité lue selon la règle du symbole, écrou associé.
- Refus : aucun effort admissible, les minimums garantis n'étant pas embarqués.

### Corrigé

- Une rugosité fine s'affichait arrondie au dixième de micromètre : Ra 0,025
  devenait « 0 ». Un formateur au nanomètre la remplace sur l'écran concerné.

### Distribution

- Compilation de l'application sur GitHub Actions pour **Windows x64**
  (`.msi`, `.exe`) et **macOS Apple Silicon** (`.dmg`), à chaque pull request
  et à chaque poussée sur `master` (workflow « Compilation »). Les
  installeurs se téléchargent dans les artefacts du run.
- L'application macOS reçoit une signature ad hoc, faute de certificat Apple :
  elle s'ouvre par clic droit → Ouvrir au premier lancement.
- Releases GitHub publiques, téléchargeables par tous : dès qu'un commit poussé
  porte une version qui n'a pas encore de release, le workflow « Release » crée
  le tag, construit les deux installeurs, les joint et publie. Un commit sans
  changement de version ne republie rien.

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
