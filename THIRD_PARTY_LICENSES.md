# Licences des dépendances

Mecatol est distribué sous double licence MIT ou Apache-2.0. Ce fichier recense
les dépendances **directes** et leurs licences.

## Comment régénérer la liste complète

Les dépendances transitives ne sont pas listées à la main : une liste écrite à
la main devient fausse au premier `cargo update`. Utilisez les outils :

```bash
cargo install cargo-about
cargo about generate --output-file licenses-rust.html

npx license-checker --production --summary
```

## Rust — moteur

| Dépendance | Licence | Rôle |
|---|---|---|
| `serde`, `serde_derive` | MIT OR Apache-2.0 | sérialisation |
| `serde_json` | MIT OR Apache-2.0 | lecture des tables normatives |
| `thiserror` | MIT OR Apache-2.0 | types d'erreur |

Le moteur de calcul ne dépend de rien d'autre. C'est délibéré : moins de
dépendances, moins de surface à auditer sur un logiciel dont les résultats
partent en fabrication.

## Rust — application de bureau

| Dépendance | Licence | Rôle |
|---|---|---|
| `tauri`, `tauri-build` | MIT OR Apache-2.0 | coquille applicative |

Tauri entraîne un arbre de dépendances important (wry, tao, et leurs
transitives). Employez `cargo about` pour la liste exhaustive.

## JavaScript — interface

| Dépendance | Licence | Rôle |
|---|---|---|
| `react`, `react-dom` | MIT | rendu de l'interface |
| `@tauri-apps/api` | MIT OR Apache-2.0 | appels au moteur |
| `vite`, `@vitejs/plugin-react` | MIT | build |
| `typescript` | Apache-2.0 | types |
| `vitest`, `jsdom` | MIT | tests |
| `@testing-library/*` | MIT | tests de composants |

## Données normatives

Les valeurs des tables ISO 286 embarquées dans `data/` sont des **données de
fait** : des grandeurs mesurables issues d'un système normalisé, saisies pour
permettre le calcul. Elles ne reproduisent ni le texte, ni la mise en page, ni
la structure éditoriale des normes ISO, qui restent la propriété de l'ISO et de
ses organismes membres.

Ce logiciel n'est ni approuvé ni certifié par l'ISO. Il ne remplace pas la
consultation des normes elles-mêmes, notamment pour les prescriptions que
Mecatol n'implémente pas.

## Polices et icônes

Aucune police n'est distribuée avec Mecatol : l'interface utilise les polices du
système. L'icône est générée par `scripts/make-icon.mjs`, sans ressource
externe, et reprend le motif du diagramme de zones de tolérance.
