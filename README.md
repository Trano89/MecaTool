# Mecatol

**Mechanical Tolerance Assistant** — un assistant de tolérancement mécanique,
pas un simple calculateur.

L'objectif n'est pas d'être le calculateur ISO le plus complet, mais le logiciel
qui permet de **comprendre et de choisir** une tolérance mécanique :

```
BESOIN → CALCUL → COMPARAISON → VALIDATION → EXPLICATION → VISUALISATION
```

> ✅ **Données normatives vérifiées.** Les 364 valeurs ISO 286 embarquées ont été
> confrontées case par case à l'ISO 286-2:2010, par un script rejouable —
> aucun écart. La couverture reste partielle : 10 lettres sur 28, jusqu'à 500 mm.
> Le moteur refuse explicitement ce qu'il n'a pas, plutôt que de l'approximer.
> Voir [docs/standards.md](docs/standards.md).

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
| Comparateur de solutions | ⏳ |
| Historique et export PDF | ⏳ |

## Lancer l'application

```bash
npm install
npm run app
```

L'écran d'accueil pose une question — « Que voulez-vous calculer ? » — et chaque
carte pré-remplit la saisie correspondante. Un seul champ de désignation suffit
ensuite : c'est la saisie qui détermine si Mecatol calcule, vérifie ou cherche.

## Essayer sans l'interface

Le moteur se pilote aussi en ligne de commande, ce qui reste le moyen le plus
direct de confronter un résultat à une table de manuel.

**Calculer** — que va donner cet ajustement ?

```bash
cargo run -p mecatol-cli -- "Ø10 H7/g6"
```

```
MECATOL 0.1.0 — Ajustement Ø10 H7/g6
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
cargo run -p mecatol-cli -- "Ø20 H7/g6" --jeu 5..50
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
cargo run -p mecatol-cli -- "Ø20" --jeu 10..30
```

Quand rien ne convient exactement, Mecatol dit **pourquoi** plutôt que de
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
mecatol-engine      algorithmes, aucune valeur normative
        ↓
mecatol-standards   données normatives versionnées, validées au chargement
        ↓
mecatol-core        types, arithmétique exacte, traçabilité
```

Le frontend ne contient **jamais** de règle normative. Le moteur est totalement
indépendant de l'interface : la CLI et l'application graphique consomment
exactement le même code.

| Crate | Rôle |
|---|---|
| `mecatol-core` | `Length` (nanomètres entiers), `ToleranceClass`, `Fit`, `Provenance` |
| `mecatol-standards` | chargement et validation des tables ISO, règle de dérivation des alésages |
| `mecatol-engine` | calcul des tolérances et ajustements, parser, explications |
| `mecatol-cli` | banc d'essai du moteur, pour confronter les résultats à un manuel |

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
cargo test --workspace        # 164 tests : moteur
cd src-tauri && cargo test    #  10 tests : frontiere Tauri + echantillons
npm test                      #  20 tests : interface
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
