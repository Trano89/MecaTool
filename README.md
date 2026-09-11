# MecaTool

**Mechanical Tolerance Assistant** — un assistant de tolérancement mécanique,
pas un simple calculateur.

L'objectif n'est pas d'être le calculateur ISO le plus complet, mais le logiciel
qui permet de **comprendre et de choisir** une tolérance mécanique :

```
BESOIN → CALCUL → COMPARAISON → VALIDATION → EXPLICATION → VISUALISATION
```

> ✅ **Données normatives vérifiées.** Les 364 valeurs ISO 286 ont été confrontées
> case par case à l'ISO 286-2:2010 par un script rejouable ; les trois tables de
> l'ISO 2768-1 ont été recoupées entre deux traductions indépendantes du même
> document. Aucun écart.
>
> ⚠️ **Sauf l'ISO 1101**, dont les données viennent d'un recueil technique qui
> reproduit la norme, et non de la norme. L'écran concerné le dit, dès son
> ouverture. La couverture reste partielle et le moteur refuse explicitement ce
> qu'il n'a pas. Voir [docs/standards.md](docs/standards.md).

## État d'avancement

| Composant | État |
|---|---|
| Arithmétique exacte (nanomètres entiers) | ✅ |
| Types du domaine et invariants | ✅ |
| Traçabilité normative | ✅ |
| Données ISO 286 — degrés IT | ✅ vérifiées, 260 valeurs |
| Données ISO 286 — écarts fondamentaux | ✅ vérifiées, 10 lettres sur 28 |
| Moteur : tolérances et ajustements | ✅ |
| Parser d'entrées (`Ø10 H7/g6`) | ✅ |
| Vérification d'une exigence fonctionnelle | ✅ |
| Recherche de solutions à partir d'un besoin | ✅ |
| CLI de vérification | ✅ |
| Graphique des zones de tolérance + export SVG | ✅ |
| Application Tauri + React (accueil, calcul, recherche) | ✅ |
| Thème clair / sombre / système | ✅ |
| Tolérances générales ISO 2768-1 | ✅ vérifiées, 3 tables |
| Comparateur de solutions | ✅ |
| Chaînes de cotes : pire des cas et RSS | ✅ |
| Tolérancement géométrique ISO 1101 | ⚠️ source secondaire, 17 caractéristiques |
| Historique et export PDF | ⏳ |
| États de surface (v0.4) | ⛔ ISO 21920 indisponible |

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

Les **états de surface** restent bloqués : le recueil les couvre, mais son
édition 2022 suit l'ISO 21920 et celle de 2014 l'ISO 4287, qui ne sont pas
interchangeables. Plutôt qu'un module mêlant deux générations de paramètres, il
n'y a pas encore de module d'états de surface.

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
MECATOOL 0.1.0 — Ajustement Ø10 H7/g6
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
| `mecatool-standards` | chargement et validation des tables ISO 286, ISO 2768 et ISO 1101 |
| `mecatool-engine` | calcul des tolérances et ajustements, parser, explications |
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

Chacun de ces refus est couvert par un test.

## Développement

```bash
cargo test --workspace        # 292 tests : moteur
cd src-tauri && cargo test    #  25 tests : frontiere Tauri + echantillons
npm test                      #  63 tests : interface
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
