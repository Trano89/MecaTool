# Architecture

## La règle qui structure tout

```
Interface (React + TypeScript)
        ↓  commandes Tauri
mecatool-app         traduction, aucune décision
        ↓
mecatool-engine      algorithmes, aucune valeur normative
        ↓
mecatool-standards   valeurs normatives, aucun algorithme
        ↓
mecatool-core        types, arithmétique exacte, traçabilité
```

Chaque couche ne connaît que celle du dessous. Une règle normative ne peut donc
pas se retrouver dans l'interface : elle n'y a pas accès.

## Les crates

| Crate | Contient | Ne contient jamais |
|---|---|---|
| `mecatool-core` | `Length`, `ToleranceClass`, `Fit`, `Provenance` | de valeur issue d'une norme |
| `mecatool-standards` | les tables ISO, leur validation, la règle de dérivation des alésages | d'algorithme de calcul |
| `mecatool-engine` | composition des écarts, parser, recherche, géométrie du diagramme | de valeur en dur |
| `mecatool-app` | les commandes Tauri | de calcul |
| `mecatool-cli` | le banc d'essai du moteur | — |

`src-tauri` est **hors de l'espace de travail Cargo**. `cargo test --workspace`
teste ainsi le moteur seul, sans avoir à compiler un moteur de rendu web ni à
installer de dépendance système sur un runner d'intégration continue.

## L'arithmétique exacte

Toute longueur est un entier signé de **nanomètres** (`Length`). Ce n'est pas
un détail d'implémentation, c'est la condition d'exactitude :

- `0,1 + 0,2` vaut exactement `0,3` ;
- une chaîne de 1000 cotes ne dérive pas ;
- le pouce (`25,4 mm`) tombe juste ;
- une valeur qui ne tomberait pas sur un nanomètre est **rejetée**, jamais
  arrondie en silence.

Les seuls flottants du projet vivent dans deux endroits explicitement nommés :
le contrôle de vraisemblance des tables (un diagnostic, jamais une source de
valeur) et les coordonnées de dessin.

## La frontière Tauri

Les champs traversent en `snake_case`, exactement comme en Rust. Renommer à la
volée ferait diverger les deux côtés le jour où un champ change de nom.

### Comment la dérive de types est empêchée

Un test Rust écrit un échantillon de chaque rapport dans `src/fixtures/`. Un
test TypeScript les relit **en les typant** avec les interfaces de `src/types.ts`.
Si un champ est renommé, supprimé ou change de type côté Rust, `tsc` échoue sur
l'échantillon — pas seulement à l'exécution, mais à la compilation.

```
cargo test -p mecatool-app   →  src/fixtures/*.json
                                      ↓
npx tsc --noEmit            →  src/types.ts confronté aux échantillons
```

Les mêmes échantillons servent de données aux tests de composants : ce que
l'interface affiche est donc confronté à ce que le moteur calcule, sans données
inventées pour le test.

### Pourquoi les commandes vivent dans un sous-module

`#[tauri::command]` génère une macro marquée `#[macro_export]`, donc publiée à
la racine du crate, puis un `pub use` du même nom dans le module courant. Placé
à la racine, le second entre en collision avec le premier (E0255) avec
`tauri-macros` 2.6.3 et rustc 1.98. Le sous-module `commands` n'est donc pas une
préférence de style : c'est la seule disposition qui compile.

## Ce que le moteur refuse de faire

Chacun de ces refus est couvert par un test :

- extrapoler une table hors de sa plage ;
- fournir une lettre d'écart fondamental non saisie ;
- deviner une entrée ambiguë — il pose une question ;
- arrondir silencieusement une valeur non représentable ;
- présenter une donnée non vérifiée comme une valeur ISO établie ;
- conclure à une compatibilité sans exigence fonctionnelle.

## Traçabilité

`Provenance` porte la règle absolue sur les normes dans le système de types :
un résultat ne peut pas exister sans dire d'où viennent ses chiffres.
`is_fully_verified()` n'est vrai que si **toutes** les sources le sont — une
seule source douteuse contamine le résultat entier, et l'interface affiche le
bandeau correspondant en permanence.

Voir [standards.md](standards.md) pour le protocole de vérification.

## Tests

| Suite | Portée | Commande |
|---|---|---|
| Moteur | 164 tests : arithmétique, tables, règles, géométrie | `cargo test --workspace` |
| Commandes | 10 tests : frontière Tauri, export des échantillons | `cargo test` dans `src-tauri` |
| Interface | 20 tests : formatage, types, rendu des écrans | `npm test` |

Le formatage côté TypeScript reprend la convention d'arrondi de Rust — au plus
proche, **moitié à l'opposé de zéro**. `Math.round` arrondit vers +∞ et
donnerait `-0.000` là où le moteur écrit `-0.001` ; le cas négatif est testé des
deux côtés.
