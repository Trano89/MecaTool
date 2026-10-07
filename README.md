# MecaTool

**L'atelier de l'ingénieur mécanicien** — un outil qui accompagne les décisions
de conception, dont le tolérancement n'est qu'un domaine parmi d'autres.

L'objectif n'est pas d'être le calculateur ISO le plus complet, mais le logiciel
qui permet de **comprendre et de choisir** :

```
BESOIN → CALCUL → COMPARAISON → VALIDATION → EXPLICATION → VISUALISATION
```

Cette trajectoire n'a rien de spécifique aux tolérances. Choisir un roulement la
suit ; choisir un cordon de soudure la suit. C'est le squelette de l'outil, pas
celui d'un module — et c'est pourquoi MecaTool est organisé en **domaines** qui
se passent des résultats plutôt qu'en calculateurs côte à côte.

```
« je monte un 6210, bague intérieure tournante, charge normale »
      → règle ISO 15      : symbole d'alésage 10 → alésage 50 mm
      → tableau fabricant : conditions d'emploi → classe k5
      → moteur ISO 286    : Ø50 k5 → écarts réels, graphique, verdict
```

Aucune de ces trois étapes n'est neuve. Ce qui l'est, c'est leur enchaînement.
Voir [docs/domaines.md](docs/domaines.md).

> ✅ **Données normatives vérifiées.** Les 364 valeurs ISO 286 ont été confrontées
> case par case à l'ISO 286-2:2010 par un script rejouable ; les trois tables de
> l'ISO 2768-1 ont été recoupées entre deux traductions indépendantes du même
> document. Aucun écart.
>
> ⚠️ **Sauf l'ISO 1101**, dont les données viennent d'un recueil technique qui
> reproduit la norme, et non de la norme. L'écran concerné le dit, dès son
> ouverture. La couverture reste partielle et le moteur refuse explicitement ce
> qu'il n'a pas. Voir [docs/standards.md](docs/standards.md).
>
> ✅ **Soudure : données lues dans les normes.** ISO 2553:2013 (symboles et
> cotation), ISO 4063:2009 (nomenclature complète) et ISO 5817:2014.
>
> ✅ **Matières : désignation et aciers de construction lus dans les normes.**
> EN 10027-1 (édition 2005, remplacée depuis par celle de 2016 : chaque lecture
> le rappelle) et EN 10025-2:2019. Les propriétés physiques par famille restent
> des ordres de grandeur non vérifiés.
>
> 🟠 **États de surface et visserie : données non vérifiées.** Elles ont été
> saisies sans document normatif ouvert. Elles portent l'état `unverified`, que
> chaque écran affiche avant toute saisie, et chaque fichier dit contre quoi le
> confronter. Ne pas s'en servir pour réceptionner un ouvrage avant cette
> confrontation.

## Les domaines

Chaque domaine dit sur quelle source il repose, et à quel titre. Cet état n'est
pas saisi à la main : il se **déduit** de la provenance des données, de sorte
qu'un jeu qui changerait de statut déplacerait le domaine sans que personne ait
à y penser.

| Domaine | Question | Source | État |
|---|---|---|---|
| Ajustements | Que donne cet ajustement, lequel choisir ? | ISO 286-1/-2 | ✅ confrontée |
| Comparateur | Lequel convient le mieux ? | ISO 286-1/-2 | ✅ confrontée |
| Tolérances générales | Que valent les cotes sans tolérance ? | ISO 2768-1 | ✅ confrontée |
| Chaîne de cotes | Que donne cet empilement ? | aucune — géométrie seule | ✅ |
| Tolérancement géométrique | Que dit ce cadre, que lui manque-t-il ? | ISO 1101 | ⚠️ recueil |
| **Roulements** | **Quel alésage, quelle tolérance de portée ?** | **ISO 15 + fabricants** | **⚠️ recueil + recommandation** |
| **États de surface** | **Que dit cette indication, quel procédé l'obtient ?** | **ISO 21920-1, ISO 1302:1992, ordres de grandeur d'atelier** | **🟠 non vérifiée** |
| **Soudure** | **Que dit ce symbole, que tolère son niveau de qualité ?** | **ISO 2553:2013, ISO 4063:2009, ISO 5817:2014** | **✅ confrontée** |
| **Visserie** | **Quel filetage, quel trou de passage ?** | **ISO 261, 68-1, 273, 898-1 + ISO 286** | **🟠 non vérifiée + ✅ confrontée** |
| **Matières** | **Quelle nuance, quelles propriétés, et que devient l'ajustement à chaud ?** | **EN 10027-1:2005, EN 10025-2:2019, ordres de grandeur + ISO 286** | **✅ confrontée + 🟠 ordres de grandeur non vérifiés** |

Un domaine sans source **reste visible**, désactivé, avec sa raison. Le masquer
laisserait croire qu'il n'existe pas ; l'afficher sans réserve laisserait croire
qu'il fonctionne.

## Le socle

| Composant | État |
|---|---|
| Arithmétique exacte (nanomètres et millisecondes d'arc entiers) | ✅ |
| Traçabilité normative sur quatre états de source | ✅ |
| Registre de domaines, navigation pilotée par les données | ✅ |
| Graphique des zones de tolérance + export SVG | ✅ |
| Application Tauri + React, thème clair / sombre / système | ✅ |
| CLI de vérification | ✅ |
| Historique et export PDF | ⏳ |

## Le module géométrique, et ce qu'il ne fait pas

L'ISO 1101 n'était pas disponible. Elle ne l'est toujours pas : ce qui a été
obtenu, c'est un **recueil technique** qui la reproduit — le VSM « Extrait de
normes », éditions 2022 et 2014. MecaTool ne présente donc jamais ces données
comme lues dans l'ISO 1101, et l'écran affiche la réserve avant toute saisie.

Les deux éditions ont été confrontées l'une à l'autre : mêmes familles, mêmes
symboles, même exigence de référence spécifiée, même règle d'emboîtement. Les
deux seuls écarts sont des évolutions connues de la norme, pas des désaccords.

**Le module ne propose aucune valeur de tolérance**, et un test échoue si un
message venait à en recommander une. L'ISO 1101 définit un vocabulaire, pas des
chiffres : la valeur relève du concepteur. Ce que le module fait, c'est lire un
cadre et dire ce qui cloche.

```
⟂ 0.05
🔴 À corriger — Perpendicularité est une tolérance d'orientation : elle exige
   au moins une référence spécifiée. Sans référence, il n'y a rien par rapport
   à quoi mesurer l'écart.
```

```
// 0.02 A
⏥ 0.05
🟠 À vérifier — ⏥ 0.05 n'ajoute rien : ∥ 0.02 A borne déjà la forme de cet
   élément, et le fait plus serré (0.02 contre 0.05).
```

Le second constat n'est pas une trouvaille de MecaTool : il découle d'une phrase
de la source, citée sous « Pourquoi ? ». Et il ne se déclenche qu'entre zones de
**même nature** — une zone cylindrique ø0,02 et une zone de 0,05 entre deux
plans ne sont pas commensurables, et comparer leurs seuls nombres conclurait de
travers. Quand les natures diffèrent, MecaTool le dit au lieu de trancher.

Les **états de surface** sont restés bloqués tant que la seule source existait
en deux générations incompatibles — ISO 21920 d'un côté, ISO 4287 et ISO 1302 de
l'autre. Ils s'ouvrent désormais en **se restreignant** à ce que les deux ont en
commun, plutôt qu'en tranchant. Voir plus bas.

## Le module roulements, et la frontière qu'il rend visible

C'est le premier domaine qui en alimente un autre. Il ne s'arrête pas à
« classe k5 » : il poursuit jusqu'aux écarts réels en passant la main au moteur
ISO 286. Mais les deux moitiés du résultat n'ont **pas le même statut**, et
c'est le point.

La règle du symbole d'alésage vient de l'ISO 15, et elle est énoncée en une
phrase — donc appliquée, jamais recopiée en quatre-vingt-seize correspondances.
Seuls les quatre codes spéciaux s'énumèrent, parce qu'ils échappent justement à
la règle. Au-delà du symbole 96, soit 480 mm, la source ne dit rien et le moteur
refuse de prolonger.

Le choix de la classe, lui, vient d'un tableau portant la mention **« Dimensions
du fabricant »**. Aucune norme ne l'impose : ce sont les pratiques de montage
recommandées. D'où un quatrième état de source, à côté de « confrontée »,
« recueil » et « non vérifiée » :

```
🟠  Pratique recommandée, sans caractère normatif : aucune norme ne l'impose,
    et s'en écarter reste légitime si la raison en est connue.
```

La distinction compte dans les deux sens. Présenter ce tableau comme normatif
durcirait une recommandation ; le présenter comme une donnée douteuse
banaliserait une pratique bien établie. Le raisonnement pose les deux étapes
côte à côte, pour que la frontière se voie :

```
Classe recommandée   k5          Pratique de montage des fabricants,
                                 sans caractère normatif.
Écarts de la classe  ISO 286-1   ei = +2 µm, es = +13 µm
```

Le tableau laisse des cases vides, et elles le restent. Sous charges faibles, la
source ne recommande rien pour les roulements à rotule : MecaTool refuse de
conclure. Combler par la classe voisine serait inventer une recommandation.

Enfin, lire une désignation rend **plusieurs lectures** plutôt qu'une. La source
dit comment un symbole se traduit en diamètre ; elle ne dit pas comment découper
`623` en série et symbole — série 6 + symbole 23, ou série 62 + symbole 3. Les
deux sont formellement licites, et seule la connaissance des séries existantes
trancherait.

## États de surface, soudure, visserie, matières — et leur réserve

Ces domaines fonctionnent de bout en bout : moteur, commandes, écrans, tests.
La soudure, l'EN 10027-1 et l'EN 10025-2 ont été confrontées aux normes
elles-mêmes. Les
autres données ont été **saisies sans document normatif ouvert** : elles
portent l'état `unverified`, et c'est ce que chaque écran dit avant toute
saisie. Ce n'est pas une formalité : la règle absolue autorise
une donnée non vérifiée, à condition qu'elle se voie.

**États de surface.** Le module lit une indication — `MRR Ra 0.8 ⊥`, `N7`,
`Rz 6.3 max` — et situe chaque procédé du tableau par rapport à elle : atteinte
d'ordinaire, avec des soins particuliers, hors d'atteinte, ou *plus fin que
nécessaire*, ce qui signale un procédé probablement trop cher. Le graphique des
plages, calculé en Rust, répond à « quelle rugosité pour ce procédé ? » dès
l'ouverture de l'écran.

```
Rz 6.3
🔵 Le tableau des procédés est exprimé en Ra. MecaTool ne convertit pas Rz en
   Ra : aucune relation fixe ne lie les deux paramètres.
```

Le blocage sur les deux générations de normes se lève en **retrécissant** le
périmètre : symboles, sens des stries et paramètres d'amplitude, communs aux
deux, sont embarqués ; longueurs de base, filtres et règle d'acceptation par
défaut, qui les distinguent, ne le sont pas — et le raisonnement le dit.

**Soudure.** ✅ Toutes les données de la soudure sont **lues dans les normes
elles-mêmes**. Les symboles et leur cotation viennent de l'ISO 2553:2013 —
vingt-deux symboles élémentaires, six supplémentaires, les systèmes A et B,
l'article 5 — et non du recueil, qui reproduit l'édition de 1992. La
nomenclature ISO 4063:2009 est embarquée **en entier** : un numéro se lit avec
sa hiérarchie (`135` relève de `13`, qui relève de `1`) et ses variantes
(`131-D`, `121-C`, `522+15`) ; un numéro remplacé (`137`) renvoie à l'Annexe A ;
un nom d'atelier rend toutes ses lectures (`MAG` → 135, 136, 138), une
désignation US la sienne (`SMAW` → 111). Les niveaux de qualité viennent de
l'ISO 5817:2014, avec les procédés qu'elle vise. Un symbole complet se restitue phrase par phrase, avec ce qui
cloche : une cote `s` sur une soudure d'angle, une soudure alternée d'un seul
côté, une soudure évasée sans sa cote obligatoire, un niveau ISO 5817 appliqué à
du brasage ou à un procédé qu'elle ne vise pas. Les limites du niveau se
chiffrent pour la géométrie saisie :

```
convexité excessive (503)    h ≤ 1 mm + 0.15 b, max. 4 mm    →  h ≤ 2.5 mm
```

Comme le module géométrique ne propose aucune valeur, le module soudure **ne
recommande aucun niveau de qualité** : c'est l'affaire de la norme d'application
ou du concepteur. Un test échoue si un constat venait à en recommander un.

**Visserie.** `M10` se déplie en pas, profil de base, section résistante,
perçage avant taraudage et trous de passage. Les diamètres de base suivent la
*règle* du profil ISO 68-1 plutôt qu'une table, calculés en entiers au
femtomètre puis annoncés arrondis au micromètre — et retrouvent les valeurs de
l'ISO 724 (`d2 = 9.026`, `d1 = 8.376`). Les trous de passage passent ensuite au
moteur ISO 286 :

```
Trou moyen   Ø11 H13       ISO 273, non vérifiée
Écarts       EI = 0, ES = +270 µm     ISO 286, confrontée
```

C'est la frontière déjà rendue visible par les roulements : deux natures de
source dans un même résultat, chacune avec son étiquette. Aucun effort
admissible n'est calculé : la classe de qualité donne des valeurs nominales, et
les minimums garantis de l'ISO 898-1 ne sont pas embarqués.

## Matières, et l'ajustement à chaud

Une désignation d'acier se **lit par règles** : `S355J2` donne un acier de
construction, 355 MPa pour la plus petite épaisseur, 27 J à −20 °C ; `42CrMo4`
donne 0,42 % de carbone et 1 % de chrome (4 / 4) ; `X5CrNi18-10` donne 18 % de
chrome et 10 % de nickel, sans facteur. Pour un acier de construction embarqué,
la limite d'élasticité se lit **pour l'épaisseur choisie** : S355 ne garantit
355 MPa que jusqu'à 16 mm.

Le domaine alimente ensuite celui des ajustements :

```
Logement aluminium, arbre acier, Ø50 H7/p6, ΔT = +80 K
  jeu à froid   −42 µm à −1 µm    serré
  jeu à chaud    +2 µm à +43 µm   avec jeu
🔴 Le serrage peut disparaître : la pièce frettée peut tourner à cette température.
```

## Choisir plutôt que taper

Une tolérance, une classe, un pas ou une valeur de rugosité se **choisissent**
parmi ce que le moteur connaît, par boutons ou par listes. Un maillon de chaîne
de cotes s'écrit `A = 20 h11` ou `A = 20 ISO 2768-m` : le moteur lit les écarts
lui-même, et la chaîne dit d'où vient chacun. Restent saisis au clavier les
diamètres et les exigences fonctionnelles — une liste de diamètres « usuels »
serait une valeur inventée — et la valeur d'une tolérance géométrique, que
l'ISO 1101 laisse au concepteur.

## Lancer l'application

```bash
npm install
npm run app
```

L'écran d'accueil pose une question — « Que voulez-vous calculer ? » — et chaque
carte pré-remplit la saisie correspondante. Un seul champ de désignation suffit
ensuite : c'est la saisie qui détermine si MecaTool calcule, vérifie ou cherche.

## Essayer sans l'interface

Le moteur se pilote aussi en ligne de commande, ce qui reste le moyen le plus
direct de confronter un résultat à une table de manuel.

**Calculer** — que va donner cet ajustement ?

```bash
cargo run -p mecatool-cli -- "Ø10 H7/g6"
```

```
MECATOOL 0.3.0 — Ajustement Ø10 H7/g6
================================================================

ALÉSAGE H7
----------
  Dimension nominale         10.000 mm
  Écart inférieur EI         0
  Écart supérieur ES         +15 µm
  Dimension minimale         10.000 mm
  Dimension maximale         10.015 mm
  Tolérance IT7              15 µm
  Échelon des tables         au-dessus de 6 jusqu'à 10 mm

ARBRE g6
--------
  ...

AJUSTEMENT
----------
  Jeu minimum                +5 µm
  Jeu maximum                +29 µm
  Amplitude du jeu           24 µm

CONCLUSION
----------
  🟢 AJUSTEMENT AVEC JEU
  L'arbre restera toujours plus petit que l'alésage : le jeu varie de 5 µm à 29 µm.
```

L'option `--expert` déplie tout le calcul, formules comprises.

**Vérifier** — est-ce que ça convient ?

```bash
cargo run -p mecatool-cli -- "Ø20 H7/g6" --jeu 5..50
```

```
  Besoin exprimé          5 µm à 50 µm
  Plage obtenue           7 µm à 41 µm

  🟢 COMPATIBLE
  La plage de jeu calculée (7 µm à 41 µm) reste entièrement dans votre fenêtre
  fonctionnelle (5 µm à 50 µm). Toute pièce conforme au plan conviendra.
```

**Trouver** — quels ajustements répondent à mon besoin ?

```bash
cargo run -p mecatool-cli -- "Ø20" --jeu 10..30
```

Quand rien ne convient exactement, MecaTool dit **pourquoi** plutôt que de
renvoyer une liste vide :

```
AUCUNE SOLUTION EXACTE
----------------------
  La fenêtre demandée mesure 20 µm, et le plus fin ajustement du périmètre
  disperse de 18 µm : une solution serait géométriquement possible. Mais aucune
  des lettres disponibles ne positionne sa plage à l'intérieur de la fenêtre.
  La solution la plus proche est G5/h6, à 3 µm de dépassement cumulé.
```

Le périmètre exploré est toujours rappelé : une recherche infructueuse se lit
« aucune solution dans ce périmètre », jamais « aucune solution n'existe ».

## Architecture

```
Frontend (React + TypeScript)
        ↓
Commandes Tauri
        ↓
mecatool-engine      algorithmes, aucune valeur normative
        ↓
mecatool-standards   données normatives versionnées, validées au chargement
        ↓
mecatool-core        types, arithmétique exacte, traçabilité
```

Le frontend ne contient **jamais** de règle normative. Le moteur est totalement
indépendant de l'interface : la CLI et l'application graphique consomment
exactement le même code.

| Crate | Rôle |
|---|---|
| `mecatool-core` | `Length` et `Angle` (entiers exacts), `ToleranceClass`, `Fit`, `Provenance` |
| `mecatool-standards` | chargement et validation des données normatives, un module par norme |
| `mecatool-engine` | un module par domaine, et le registre qui les déclare |
| `mecatool-cli` | banc d'essai du moteur, pour confronter les résultats à un manuel |

Toute grandeur est un entier : les longueurs en **nanomètres**, les angles en
**millisecondes d'arc**. Le second choix compte autant que le premier —
`0°20′` vaut un tiers de degré, inexprimable en degrés décimaux, mais exact en
secondes d'arc. Les tolérances angulaires générales tombent donc juste.

## Les sept principes

1. **Exactitude** — calculs déterministes, aucune valeur inventée.
2. **Transparence** — chaque résultat porte le raisonnement qui l'a produit.
3. **Visualisation** — un résultat mécanique doit se comprendre graphiquement.
4. **Pédagogie** — un débutant doit comprendre le résultat.
5. **Expert** — un ingénieur doit accéder au détail du calcul.
6. **Local first** — les calculs fonctionnent sans Internet.
7. **Open source** — le code est publiable et maintenable.

## Ce que le moteur refuse de faire

- Extrapoler une table hors de sa plage.
- Fournir une lettre d'écart fondamental non saisie.
- Deviner une entrée ambiguë : il pose une question.
- Arrondir silencieusement une valeur non représentable.
- Présenter une donnée non vérifiée comme une valeur ISO établie.
- Conclure à une compatibilité sans exigence fonctionnelle.
- Convertir une rugosité Rz en Ra, ou l'inverse.
- Recommander un niveau de qualité de soudure.
- Calculer un effort admissible à partir de valeurs nominales.
- Affirmer qu'une nuance d'acier existe parce que sa désignation est bien formée.

Chacun de ces refus est couvert par un test.

## Développement

```bash
cargo test --workspace        # 476 tests : moteur
cd src-tauri && cargo test    #  38 tests : frontiere Tauri + echantillons
npm test                      # 116 tests : interface
cargo clippy --workspace --all-targets
cargo fmt --all
```

Prérequis : Rust stable, Node.js LTS, et sous Windows les Build Tools C++.

## Documentation

- [docs/architecture.md](docs/architecture.md) — les couches, la frontière Tauri,
  et comment la dérive de types est empêchée.
- [docs/visualization.md](docs/visualization.md) — pourquoi la géométrie du graphique
  est calculée en Rust, et ce que les tests en garantissent.
- [docs/standards.md](docs/standards.md) — données normatives et protocole de
  vérification. **À lire en premier.**

## Licence

Double licence MIT ou Apache-2.0, au choix.
