# Mecatol

**Mechanical Tolerance Assistant** — un assistant de tolérancement mécanique,
pas un simple calculateur.

L'objectif n'est pas d'être le calculateur ISO le plus complet, mais le logiciel
qui permet de **comprendre et de choisir** une tolérance mécanique :

```
BESOIN → CALCUL → COMPARAISON → VALIDATION → EXPLICATION → VISUALISATION
```

> ⚠️ **Version 0.1 en construction.** Les données normatives embarquées sont
> saisies mais **pas encore vérifiées** contre une source primaire. Mecatol le
> signale à chaque résultat. Voir [docs/standards.md](docs/standards.md).

## État d'avancement

| Composant | État |
|---|---|
| Arithmétique exacte (nanomètres entiers) | ✅ |
| Types du domaine et invariants | ✅ |
| Traçabilité normative | ✅ |
| Données ISO 286 — degrés IT | ⚠️ saisies, non vérifiées |
| Données ISO 286 — écarts fondamentaux | ⚠️ saisies, non vérifiées, 10 lettres sur 28 |
| Moteur : tolérances et ajustements | ✅ |
| Parser d'entrées (`Ø10 H7/g6`) | ✅ |
| CLI de vérification | ✅ |
| Vérification d'une exigence fonctionnelle | ⏳ |
| Recherche de solutions, comparaison | ⏳ |
| Graphique des zones de tolérance | ⏳ |
| Interface Tauri + React | ⏳ |

## Essayer

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
cargo test --workspace      # 110 tests
cargo clippy --workspace --all-targets
cargo fmt --all
```

Prérequis : Rust stable, Node.js LTS, et sous Windows les Build Tools C++.

## Documentation

- [docs/standards.md](docs/standards.md) — données normatives et protocole de
  vérification. **À lire en premier.**

## Licence

Double licence MIT ou Apache-2.0, au choix.
