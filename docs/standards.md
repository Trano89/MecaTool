# Données normatives et protocole de vérification

Ce document décrit comment MecaTool stocke les valeurs issues des normes, et
comment une valeur passe de « saisie » à « vérifiée ». C'est le document le plus
important du projet : tout le reste est de la mécanique logicielle, celui-ci
porte la règle absolue de MecaTool.

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
├── iso286/
│   ├── iso286-1-2010.it-grades.json          degrés IT01 à IT18
│   └── iso286-1-2010.shaft-deviations.json   écarts fondamentaux des arbres
├── iso2768/
│   └── iso2768-1-1989.general-tolerances.json   tolérances générales
├── iso1101/
│   └── iso1101.geometric-characteristics.json   caractéristiques géométriques
├── roulements/                                  symbole d'alésage, classes de montage
├── surface/          non vérifiés
│   ├── iso21920.indication.json                 symboles, stries, paramètres
│   ├── iso1302-1992.classes-n.json              classes N1 à N12 (norme retirée)
│   └── procedes.rugosite.json                   Ra par procédé, ordres de grandeur
├── soudure/          vérifiés sur la norme
│   ├── iso4063.procedes.json                    nomenclature — VÉRIFIÉE sur la norme
│   ├── iso2553-2013.symbols.json                symboles — VÉRIFIÉS sur la norme
│   ├── iso2553-2013.cotation.json               cotation (article 5) — VÉRIFIÉE sur la norme
│   └── iso5817.niveaux-qualite.json             niveaux B, C, D — VÉRIFIÉS sur la norme
├── matieres/         non vérifiés
│   ├── en10027-1.designation.json               désignation — VÉRIFIÉE (édition 2005)
│   ├── en10025-2.aciers-construction.json       ReH et Rm — VÉRIFIÉS sur la norme
│   └── proprietes-physiques.json                E, ν, ρ, α par famille
└── visserie/         non vérifiés
    ├── iso261.filetages-metriques.json          diamètres et pas
    ├── iso68-1.profil.json                      profil de base, en fractions de H
    ├── iso273.trous-de-passage.json             trous de passage, séries et classes
    └── iso898.classes-qualite.json              classes de qualité vis et écrous
```

Le fichier ISO 1101 n'a **pas de millésime dans son nom**, à la différence des
autres. Ce n'est pas un oubli : la source le cite sans année, et MecaTool ne
l'invente pas. Voir « Sources secondaires » plus bas.

Les fichiers sont **séparés du code** et **inclus à la compilation**
(`include_str!`). MecaTool calcule donc sans accès disque ni réseau, et ne peut
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

## Sources secondaires

Entre « confronté à la norme » et « pas encore vérifié » il existe un troisième
cas, et l'ignorer conduirait à mentir dans un sens ou dans l'autre.

Une donnée peut venir d'un **recueil technique** qui reproduit la norme : un
manuel professionnel, lu avec soin, mais qui n'est pas la norme. La ranger sous
`verified` reviendrait à citer une norme qu'on n'a pas ouverte. La ranger sous
`unverified` reviendrait à dire qu'on n'a rien fait. D'où un troisième état :

```json
"verification": {
  "state": "secondary",
  "from": "VSM « Extrait de normes » 2022, chapitre 2, pages 174 à 189",
  "reproduces": "SN EN ISO 1101",
  "on": "2026-09-11"
}
```

Il **compte comme non vérifié** pour l'affichage, mais avec son propre message :

```
/!\  ISO 1101 (classement, symboles, exigence de référence et forme des zones) :
     Donnée transcrite d'un recueil technique, non confrontée à la norme
     elle-même. Source : VSM « Extrait de normes » 2022, chapitre 2, pages 174
     à 189, qui reproduit SN EN ISO 1101.
```

Deux conséquences de forme s'ensuivent :

**Le millésime peut rester vide.** Le recueil cite « SN EN ISO 1101 » sans
année. `citation()` rend alors `ISO 1101` tout court, et non `ISO 1101:` — un
deux-points pendant se lirait comme un millésime perdu, pas comme un millésime
inconnu. Le contenu relevé (élément unifié UF, zones combinées CZ et séparées
SZ, composants d'association C, G, N, T, X) correspond visiblement à une édition
récente, mais MecaTool n'en déduit pas d'année.

**La réserve ne remonte pas au bandeau global.** Elle s'affiche sur l'écran
concerné, dès son ouverture. L'ISO 286 et l'ISO 2768 sont confrontées à leur
source primaire : jeter un doute sur elles diluerait celui qui est mérité
ailleurs. Un avertissement affiché partout finit par ne plus rien vouloir dire
nulle part.

## État actuel

| Jeu de données                | État | Couverture | Vérifié contre |
|---|---|---|---|
| Degrés IT01..IT18, Ø0..500 mm | ✅ vérifié | 13 échelons × 20 degrés = **260 valeurs** | ISO 286-2:2010, tableau 1 |
| Écarts fondamentaux arbres    | ✅ vérifié | 10 lettres : d, e, f, g, h, js, k, m, n, p | ISO 286-2:2010, tableaux 18 à 26 |
| Tolérances générales ISO 2768-1 | ✅ vérifié | 3 tables × 4 classes | DIN ISO 2768-1:1991-06, tableaux 1 à 3 |
| Caractéristiques géométriques ISO 1101 | ⚠️ source secondaire | 17 entrées, 4 familles, 22 modificateurs | VSM « Extrait de normes » 2022, p. 174-189, recoupé avec l'édition 2014, p. 90-91 |
| Indication des états de surface ISO 21920-1 | 🟠 non vérifié | 3 symboles, 7 sens des stries, 6 paramètres | à confronter : ISO 21920-1/-2, ou VSM 2022 § 2.9 |
| Classes N ISO 1302:1992 | 🟠 non vérifié | N1 à N12 | à confronter : ISO 1302:1992 |
| Rugosité par procédé | 🟠 non vérifié, non normatif | 22 procédés | à confronter : VSM 2022, p. 211 |
| Procédés de soudage ISO 4063 | ✅ vérifié | nomenclature complète : 157 numéros, variantes (modes de transfert, électrodes, fil froid/chaud), 13 numéros remplacés (Annexe A), désignations US (Annexe B) | ISO 4063:2009 version corrigée 2010 (NF EN ISO 4063:2011), articles 2 et 3, Annexes A et B |
| Symboles de soudure ISO 2553:2013 | ✅ vérifié | 22 symboles élémentaires, 6 supplémentaires, systèmes A et B | ISO 2553:2013(F), tableaux 1 et 3, § 4.2 à 4.4 |
| Cotation des symboles ISO 2553:2013 | ✅ vérifié | famille, cotes principales admises et forme double des 22 symboles ; emploi des 6 symboles supplémentaires | ISO 2553:2013(F), article 5, tableaux 2, 3 et 5, § 3.16 à 3.21 |
| Niveaux de qualité ISO 5817 | ✅ vérifié | 27 lignes : défauts de surface (1.1 à 1.22) et de géométrie (3.1, 3.2) ; procédés visés (article 1) ; défauts internes et multiples non embarqués | ISO 5817:2014 (EN ISO 5817:2014), article 1, Tableau 1, Annexe B |
| Filetages ISO 261 | 🟠 non vérifié | M1 à M64, 1er et 2e choix | à confronter : ISO 261 |
| Profil de base ISO 68-1 | 🟠 non vérifié | une règle : d2, d1, d3 en fractions de H | à confronter : ISO 68-1, et ISO 724 pour les valeurs |
| Trous de passage ISO 273 | 🟠 non vérifié | 31 diamètres × 3 séries | à confronter : ISO 273 |
| Classes de qualité ISO 898-1 | 🟠 non vérifié | 9 classes de vis, 5 d'écrous | à confronter : ISO 898-1 et 898-2 |
| Désignation des aciers EN 10027-1 | ✅ vérifié (édition 2005, remplacée par 2016) | règles : 11 groupes d'emploi (Tableaux 2 à 11) et leurs symboles additionnels, résilience, non alliés, facteurs de teneur, fortement alliés, aciers rapides, PM, symboles après « + » (Tableaux 16 à 18) | EN 10027-1:2005, articles 4 à 7, Tableaux 1 à 18 — l'édition 2016 n'a pas été confrontée |
| Aciers de construction EN 10025-2 | ✅ vérifié | S185, S235, S275, S355, S460, S500 × jusqu'à 9 échelons d'épaisseur (3 à 400 mm) | EN 10025-2:2019, Tableaux 6 et 7, article 1 |
| Propriétés physiques par famille | 🟠 non vérifié, non normatif | 12 familles : E, ν, ρ, α | à confronter : recueil de propriétés des matériaux |

**Non vérifié ne veut pas dire inventé, ni fiable.** Les jeux marqués 🟠 ont été
saisis sans document normatif ouvert : leur champ `source` le dit tel quel, et
leur champ `pending` nomme la source contre laquelle les confronter. Les
contrôles de chargement y repèrent déjà les incohérences internes — un niveau
ISO 5817 plus exigeant qui tolérerait davantage, une borne de rugosité hors de
la série N, un filetage sans trou de passage — mais ils ne remplacent pas la
confrontation. Le protocole ci-dessous s'applique à eux comme aux autres.

Deux recoupements indépendants existent déjà, et ne valent pas vérification :
les diamètres de base calculés par la règle de l'ISO 68-1 retrouvent les valeurs
d'usage de l'ISO 724 (M6, M8, M10, M12), et la section résistante celles de
l'ISO 898-1 (M6 à M12).

**Vérifié ne veut pas dire complet.** Les valeurs présentes ont été confrontées à
la source ; il en manque encore beaucoup (voir ci-dessous).

### ISO 2553 : l'édition 2019 face à l'édition 2013

MecaTool suit l'ISO 2553:2013. L'édition 2019 (NF EN ISO 2553:2019) a été lue
et comparée, sans changer la référence ; voici ce qui la distingue, pour le
jour où l'on basculera :

- **Tableau 1** : 8 devient « soudure en V à bords évasés », 9 « en demi-V à
  bord évasé », 11 « soudure en bouchon », 21 « soudure de rechargement ».
  12 est scindé en 12.1 (résistance par points) et 12.2 (bossage), 20 en 20.1
  (bout à bout à bords relevés) et 20.2 (angle extérieur) : une numérotation
  entière ne suffit plus. 12.1, 14 et 22 gagnent la mention « plus de deux
  parties ». 1 à 7, 10, 17 et 18 sont inchangés.
- **Tableau 3** : 1 devient « finition affleurée (finition plate) ».
- **Cotation** : le tableau 5 devient le tableau 6. Nouveau § 5.13 : la
  soudure par transparence (22) se cote en `d`. `d` devient le diamètre du
  trou, `c` la largeur du trou allongé.
- **Systèmes A et B** : règles inchangées.

Deux incohérences de la norme elle-même sont notées dans les données, non
tranchées : le § 5.4.1 place la profondeur de pénétration à droite du
symbole, le § 5.2 et le tableau 5 à gauche (2013 comme 2019) ; le § 3.16 de
2013 imprime « α » pour « a ».

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

### ISO 2768-1 : deux rendus indépendants du même document

Les trois tables ont été saisies depuis la traduction **française** (couche
texte du PDF), puis recoupées cellule par cellule avec la traduction
**allemande** (page 3, un scan sans couche texte, rendu en image et relu).

Les deux chemins n'ont rien en commun : l'un passe par l'extraction de texte,
l'autre par la lecture d'une image. Une erreur de transcription devrait se
produire à l'identique dans les deux pour passer inaperçue. Les deux rendus
concordent sur chaque cellule.

Trois particularités de cette norme, toutes portées par le code :

- **La borne basse du premier échelon est incluse.** L'ISO 286 écrit
  « au-dessus de 6 jusqu'à 10 » ; l'ISO 2768 écrit « de 0,5 à 3 ». Une cote de
  0,5 mm appartient donc bien au premier échelon.
- **Deux cases sont vides** dans le tableau 1 : la classe `f` au-delà de
  2000 mm, la classe `v` en dessous de 3 mm. Ce ne sont pas des zéros — la norme
  ne définit rien. MecaTool refuse ces combinaisons et le dit.
- **Les tolérances angulaires se resserrent quand la pièce grandit**, à
  l'inverse des linéaires : elles dépendent de la longueur du côté le plus court
  de l'angle, et le même écart linéaire rapporté à un bras plus long donne un
  angle plus petit. La validation des tables applique donc une monotonie inverse
  sur cette table.

Les tolérances générales **géométriques** relevaient de l'ISO 2768-2, **retirée
au printemps 2021** et remplacée par l'ISO 22081, conforme à l'ISO GPS. MecaTool
ne couvre ni l'une ni l'autre.

Ce retrait est établi par le VSM « Extrait de normes » 2022, § 2.7.3.1, page 156.
La même page prévient qu'une transposition directe des valeurs de l'ISO 2768-2
vers l'ISO 22081 n'est pas possible : il ne s'agit donc pas d'une table à
recopier ailleurs, mais d'un changement d'approche. Le tableau 157/1 du recueil
propose une correspondance, mais son propre texte la qualifie de *proposition* —
ce n'est pas une valeur normative, et MecaTool ne la reprend pas.

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

**Exactitude de lecture** — un nombre JSON transite par un `f64`. MecaTool ne
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
