# Contribuer à Mecatol

Merci de l'intérêt porté au projet. Ce document dit surtout **ce qui est
non négociable**, parce que Mecatol calcule des cotes que des gens vont usiner.

## La règle absolue

> Ne jamais inventer une règle normative.
> Ne jamais compléter une table par intuition.
> Ne jamais présenter une approximation comme une valeur ISO.

Une contribution qui ajoute une valeur normative doit dire **d'où elle vient** :
norme ou recueil, édition, tableau, page. Sans source, la valeur reste marquée
`unverified` et l'interface le signale à l'utilisateur. C'est acceptable ; faire
passer une valeur non vérifiée pour établie ne l'est pas.

Voir [docs/standards.md](docs/standards.md) pour le protocole complet.

## Où va quoi

| Vous ajoutez | Ça va dans |
|---|---|
| une valeur issue d'une norme | `data/`, en JSON versionné par édition |
| une règle de composition normative | `mecatol-standards` |
| un algorithme, un parser, une recherche | `mecatol-engine` |
| un type ou un invariant du domaine | `mecatol-core` |
| une commande exposée à l'interface | `src-tauri/src/commands.rs` |
| de l'affichage | `src/` |

Une règle normative dans le frontend est un défaut, pas un raccourci. Le
frontend n'a pas accès aux tables : c'est voulu.

## Avant d'ouvrir une pull request

```bash
cargo fmt --all
cargo clippy --workspace --all-targets     # doit être silencieux
cargo test --workspace
cd src-tauri && cargo test && cd ..
npx tsc --noEmit
npm test
```

L'intégration continue rejoue tout cela sur Linux, Windows et macOS.

## Tests attendus

Une correction de bug arrive avec le test qui échouait avant elle.

Un nouveau calcul arrive avec :

- au moins un **cas de référence** vérifiable dans un manuel (`Ø10 H7/g6`,
  `Ø20 H7/k6`, …) ;
- les **cas limites** : bornes d'échelon, hors plage, combinaison invalide ;
- le **refus** attendu quand le moteur ne peut pas conclure.

Un refus explicite vaut mieux qu'un résultat plausible. Si votre code peut
produire une valeur approximative sans le dire, il n'est pas fini.

## Modifier une table normative

1. Corrigez le JSON dans `data/`.
2. `cargo test` — les contrôles automatiques (échelons contigus, monotonie,
   signes, recoupements croisés entre tables) doivent tous passer.
3. Mettez à jour le bloc `verification` en citant précisément la source.
4. Committez **le fichier de données seul**, avec la source en message.

L'historique Git devient ainsi la trace de qui a vérifié quoi, contre quelle
source et à quelle date.

## Style

Le code suit `rustfmt` et les conventions de son voisinage. Deux habitudes
propres au projet :

- **Identifiants et commentaires en ASCII**, chaînes destinées à l'utilisateur
  en français accentué. La frontière est nette et évite les surprises d'encodage.
- **Les commentaires expliquent le pourquoi**, pas le quoi. `// incrémente i`
  n'apprend rien ; « la borne haute est incluse, comme dans les tables ISO »
  évite une régression.

## Langue

Le projet est écrit en français : documentation, messages d'erreur, noms de
tests. Les rapports de bug et discussions en anglais sont les bienvenus.
