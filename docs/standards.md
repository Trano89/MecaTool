# Données normatives et protocole de vérification

Ce document décrit comment Mecatol stocke les valeurs issues des normes, et
comment une valeur passe de « saisie » à « vérifiée ». C'est le document le plus
important du projet : tout le reste est de la mécanique logicielle, celui-ci
porte la règle absolue de Mecatol.

## La règle

> Ne jamais inventer une règle normative.
> Ne jamais compléter une table par intuition.
> Ne jamais présenter une approximation comme une valeur ISO.

Cette règle est appliquée par le système de types, pas seulement par discipline.

## Pourquoi les tables et non les formules

L'ISO 286-1 publie à la fois des formules et des tables. On pourrait croire que
les formules suffisent et que les tables n'en sont que le résultat imprimé.
**C'est faux**, et c'est la première chose qui a été vérifiée dans ce projet.

Les valeurs publiées sont arrondies vers des nombres normalisés. La formule ne
les reproduit pas :

| Cas             | Formule ISO 286-1 | Table publiée | Écart |
|-----------------|-------------------|---------------|-------|
| IT7, Ø0..3      | 8,67 µm           | 10 µm         | +15 % |
| IT7, Ø6..10     | 14,37 µm          | 15 µm         | +4 %  |
| IT7, Ø30..50    | 24,98 µm          | 25 µm         | ~0 %  |
| es(d), Ø18..30  | −63,86 µm         | −65 µm        | +2 %  |
| ei(m), Ø0..3    | 4 µm (IT7−IT6)    | 2 µm          | −50 % |

**Conséquence architecturale :** les tables sont la donnée primaire, transcrites
et vérifiées case par case. Les formules ne servent qu'à repérer une case
aberrante — une faute de frappe déplace une valeur d'un facteur 10, là où
l'arrondi normatif ne dépasse jamais quelques pour cent.

Un test (`formula::tests::lecart_darrondi_reste_modere_mais_non_nul`) échoue
volontairement si toutes les valeurs coïncident avec la formule : cela
signifierait que quelqu'un a régénéré la table au lieu de la transcrire.

## Où vivent les données

```
data/
└── iso286/
    ├── iso286-1-2010.it-grades.json          degrés IT01 à IT18
    └── iso286-1-2010.shaft-deviations.json   écarts fondamentaux des arbres
```

Les fichiers sont **séparés du code** et **inclus à la compilation**
(`include_str!`). Mecatol calcule donc sans accès disque ni réseau, et ne peut
pas démarrer avec une table manquante.

Le nom d'un fichier porte la norme, la partie et l'**édition**. Une nouvelle
édition donne un nouveau fichier, jamais une modification en place : un calcul
archivé doit rester reproductible avec les données qui l'ont produit.

## Le champ `verification`

Chaque jeu de données porte son état :

```json
"verification": {
  "state": "unverified",
  "pending": "À confronter case par case au tableau ... fourni par l'utilisateur."
}
```

ou, une fois le contrôle fait :

```json
"verification": {
  "state": "verified",
  "against": "VSM 15500, tableau des degrés IT, édition 2018, page 42",
  "on": "2026-09-15"
}
```

Cet état remonte jusqu'à l'utilisateur. Un résultat calculé à partir d'un jeu
`unverified` affiche systématiquement :

```
/!\  ISO 286-1:2010 (degrés de tolérance normalisés IT) : Donnée normative non
     vérifiée. À confronter case par case au tableau ...
```

`Provenance::is_fully_verified()` n'est vrai que si **toutes** les sources d'un
calcul le sont. Une seule source douteuse contamine le résultat entier.

## État actuel

| Jeu de données                | État | Couverture | Vérifié contre |
|---|---|---|---|
| Degrés IT01..IT18, Ø0..500 mm | ✅ vérifié | 13 échelons × 20 degrés = **260 valeurs** | ISO 286-2:2010, tableau 1 |
| Écarts fondamentaux arbres    | ✅ vérifié | 10 lettres : d, e, f, g, h, js, k, m, n, p | ISO 286-2:2010, tableaux 18 à 26 |

**Vérifié ne veut pas dire complet.** Les valeurs présentes ont été confrontées à
la source ; il en manque encore beaucoup (voir ci-dessous).

### Comment la vérification a été faite

```bash
pip install pypdf
python scripts/verify-iso286-tables.py chemin/vers/ISO_286-2.pdf
```

```
Tableau 1, degrés de tolérance  : 260 cases comparées — aucun écart.
Tableaux 18 à 26, écarts arbres : 104 cases comparées — aucun écart.
```

Le script lit la couche texte du PDF de la norme et compare case par case. Il
n'écrit rien : corriger une donnée et déclarer une source vérifiée reste une
décision humaine.

Deux lettres échappent à l'automatisation, et le script le dit :

- **`js`** n'est pas tabulé comme un écart fondamental — c'est la règle
  « ± IT/2 », couverte par les tests du moteur ;
- **`k`** a un écart qui dépend du degré, donc sa ligne n'est pas une plage
  constante. Le tableau 24 le confirme par lecture directe : la ligne des écarts
  inférieurs vaut `0` partout, sauf sous les quatre colonnes IT4 à IT7 où elle
  porte la valeur tabulée. C'est exactement le modèle implémenté.

### Ce qui reste à saisir

| Manque | Pourquoi ce n'est pas un oubli |
|---|---|
| Lettres a, b, c et r à zc | Échelons de dimensions plus fins que les 13 standards (30..40 et 40..50, 50..65 et 65..80…). Les saisir sur les échelons standards produirait des valeurs fausses d'apparence correcte. |
| Dimensions au-delà de 500 mm | La norme va jusqu'à 3 150 mm. Les échelons existent dans le tableau 1 ; ils n'ont pas encore été saisis ni vérifiés. |
| Écarts des alésages | Dérivés par la règle du delta plutôt que tabulés. Les tableaux 2 à 16 de l'ISO 286-2 permettraient un contrôle indépendant de cette dérivation — contrôle qui reste à écrire. |

Le moteur refuse explicitement tout ce qui manque, plutôt que de l'approximer.

### Lettres volontairement absentes

Les lettres **a, b, c** et **r à zc** ne sont pas saisies. Ce n'est pas un oubli.

Ces lettres emploient des échelons de dimensions nominales **plus fins** que les
13 échelons standards : la plage 30..50 y est scindée en 30..40 et 40..50, la
plage 50..80 en 50..65 et 65..80, et ainsi de suite. Les saisir sur les 13
échelons standards produirait des valeurs fausses tout en ayant l'air correctes.

Le moteur les refuse explicitement (`StandardsError::LetterUnavailable`) plutôt
que de les approximer. Elles seront ajoutées avec leurs propres échelons.

## Contrôles automatiques

Ces contrôles tournent au chargement, donc à chaque démarrage et à chaque test.
Une table fausse doit être détectable sans relecture humaine.

**Structurels** — échelons contigus et croissants, lignes de longueur correcte,
valeurs strictement positives pour les degrés IT.

**Monotonie** — à degré constant, la tolérance ne peut pas diminuer quand la
pièce grossit ; à dimension constante, un degré plus grossier est strictement
plus large ; un écart fondamental s'éloigne de la ligne zéro quand la pièce
grossit ; les lettres ne se croisent jamais.

**Signes** — `es ≤ 0` pour les lettres a..h, `ei ≥ 0` pour les lettres j..zc.

**Recoupements croisés** — deux tables saisies séparément doivent s'accorder sur
les identités normatives qui les relient :

- `ei(m) = IT7 − IT6` sur tous les échelons sauf le premier ;
- `ei(n) ≈ 2 × |es(g)|`, les deux dérivant de la même puissance de D.

Si l'une des deux tables comportait une faute de frappe, ces égalités
tomberaient. C'est la vérification automatique la plus utile du projet.

**Exactitude de lecture** — un nombre JSON transite par un `f64`. Mecatol ne
convertit jamais le flottant : il reprend son écriture décimale et la relit avec
l'analyseur exact. Une table contenant `0.30000000000000004` est **rejetée** au
chargement au lieu d'être silencieusement arrondie.

## Protocole de vérification

Pour faire passer un jeu de `unverified` à `verified` :

1. **Automatiser la comparaison** si la source est exploitable par machine.
   `scripts/verify-iso286-tables.py` sert de modèle : il lit le PDF de la norme
   et compare case par case. Une vérification faite à l'œil ne se rejoue pas et
   personne ne peut la contrôler.
2. Corriger les écarts constatés dans le JSON.
3. Lancer `cargo test` : les contrôles automatiques doivent tous passer.
4. Remplacer le bloc `verification` par l'état `verified`, en citant précisément
   la source — norme, édition, tableau, page — **et le moyen du contrôle**.
   « Vérifié » sans référence ne vaut pas mieux que « non vérifié » : un test
   l'exige désormais.
5. Committer le fichier de données **seul**, avec en message la source utilisée.

Le point 5 compte : l'historique Git devient la trace de qui a vérifié quoi,
contre quelle source et à quelle date.

Ce qui échappe à l'automatisation doit être **nommé**, avec l'observation qui
fonde le contrôle manuel — comme pour `k` ci-dessus. Un point non vérifiable
automatiquement et passé sous silence est un point non vérifié.

## Règle de dérivation des alésages

L'ISO 286-1 ne tabule pas deux fois les écarts fondamentaux. Ceux des alésages
se déduisent de ceux des arbres de même lettre.

**Règle générale**

```
lettres A..H     EI = −es(arbre correspondant)
lettres J..ZC    ES = −ei(arbre correspondant)
```

**Règle spéciale, dite règle du delta**

Pour K, M, N jusqu'au degré IT8, et pour P à ZC jusqu'au degré IT7 :

```
ES = −ei + Δ        avec  Δ = IT(n) − IT(n−1)
```

Sans ce correctif, un alésage et un arbre de même lettre mais de degrés
différents ne donneraient pas le même serrage, ce que la norme cherche
précisément à garantir.

Cette règle est vérifiée dans les tests contre quatre valeurs d'alésage connues
indépendamment — M6, N7, P7 et K7 à Ø20 — ce qui contrôle à la fois la règle et
les tables qui l'alimentent.

## Unités

Toute longueur est stockée comme un entier signé de **nanomètres**. Ce choix
garantit l'exactitude :

- les valeurs ISO sont données au micromètre avec au plus une décimale
  (`IT1 = 0,8 µm` → `800 nm`) ;
- le pouce vaut exactement `25,4 mm` → `25 400 000 nm` ;
- `0,1 + 0,2` vaut exactement `0,3`, et une chaîne de 1000 cotes ne dérive pas.

Une valeur qui ne tomberait pas exactement sur un nanomètre est **rejetée**
plutôt qu'arrondie.
