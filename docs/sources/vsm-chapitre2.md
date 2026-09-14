# Relevé du chapitre 2 du VSM — « Spécification géométrique des produits »

Ce fichier est un **relevé de lecture**, pas une donnée normative. Les données
normatives extraites de ces pages vivent dans `data/`, avec leur provenance.

## Ce qu'est la source

Trois PDF fournis par l'utilisateur, photographies d'un même exemplaire :

| Fichier | Planches | Pages du livre |
|---|---|---|
| `Document numérisé 16.pdf` | 24 | 110 – 157 |
| `Document numérisé 17.pdf` | 24 | 158 – 205 |
| `Document numérisé 18.pdf` |  3 | 206 – 211 |

Un seul balayage continu : 51 planches, 102 pages, sans trou. La règle de
correspondance est `page = 108 + 2k` (doc 16), `156 + 2k` (doc 17),
`204 + 2k` (doc 18), vérifiée par sondage aux deux extrémités.

Aucune couche de texte : ce sont des photographies. Toute valeur en est tirée
par lecture visuelle, ce qui est **moins sûr** qu'une extraction automatique
confrontée à un original — voir « Statut » plus bas.

## Statut de cette source

Le VSM est un **recueil technique** qui reproduit le contenu normatif ; ce
n'est pas la norme. Pour l'ISO 1101, le VSM cite lui-même sa source :
« 2.8 Tolérancement géométrique (SN EN ISO 1101) », page 174.

MecaTool ne présente donc jamais ces données comme lues dans l'ISO 1101. Elles
portent une provenance distincte, qui nomme le recueil, la page, et la norme
que le recueil déclare reproduire. Voir `docs/standards.md`.

## Plan du chapitre (sommaire relevé page 111)

| § | Sujet | Page |
|---|---|---|
| 2.1 | Système ISO GPS | 112 |
| 2.2 | Norme ISO GPS fondamentale ISO 8015 | 114 |
| 2.3 | Système ISO de tolérances | 120 |
| 2.4 | Sélection des classes de tolérances | 135 |
| 2.5 | Ajustements recommandés | 136 |
| 2.6 | Cotation | 138 |
| 2.7 | Tolérances générales | 152 |
| 2.8 | **Tolérancement géométrique** | **174** |
| 2.9 | **États de surface** | **204** |

Détail de 2.8 : principes 174, vue d'ensemble de la symbolique 175, explication
des symboles 185, composants d'une indication 190, éléments tolérancés 194,
références 198, étendue de la zone 201, TED 202, exemples 203, CAO 3D 204.

Détail de 2.9 : symboles 204, symbole complet 205, indications sur les dessins
208, procédés d'usinage et rugosité 211.

## 2.7 — recoupement de l'ISO 2768-1 (page 153)

Le tableau 153/1 « Écarts limites pour dimensions linéaires » et le tableau
153/2 « rayons et hauteurs de chanfreins » ont été confrontés case par case aux
données déjà embarquées par MecaTool. **Aucun écart.** C'est un troisième
recoupement indépendant, après les éditions française et allemande de la norme.

Le VSM ajoute une précision utile que la norme laisse implicite : la classe
« f (fine) est à éviter pour des pièces façonnées sans enlèvement de matière ».

## 2.8.2 — classement et symboles (page 175)

Quatre classes, dix-sept entrées. Deux attributs par entrée : la référence
spécifiée (aucune / requise / au choix) et l'exigence d'une TED explicite.

| Classe | Caractéristique | Référence | TED |
|---|---|---|---|
| Forme | Rectitude, Planéité, Circularité, Cylindricité | aucune | non |
| Forme | Profil d'une ligne, Profil d'une surface | aucune | oui |
| Orientation | Parallélisme, Perpendicularité | requise | non |
| Orientation | Inclinaison, Profil d'une ligne, Profil d'une surface | requise | oui |
| Position | Localisation | au choix | oui |
| Position | Coaxialité, Symétrie | requise | non |
| Position | Profil d'une ligne, Profil d'une surface | requise | oui |
| Battement | Battement simple, Battement total | requise | non |

Note 1) du tableau, portant sur les deux profils de la colonne « forme » :
« Pour la description de la géométrie nominale. »

## 2.8.1.1 à 2.8.1.3.4 — hiérarchie des zones (page 174)

Les notes du texte énoncent un emboîtement que MecaTool peut vérifier :

- une tolérance d'orientation « limite aussi l'écart de forme » ;
- une tolérance de position « limite aussi la tolérance de forme et la
  tolérance d'orientation » ;
- une tolérance de battement « limite par exemple la tolérance de cylindricité
  et la tolérance de concentricité ».

## 2.8.2.2 à 2.8.2.5 — modificateurs (pages 176 à 183)

Combinaison de zones : `SZ` zones séparées, `CZ` zone combinée.
Zones inégales : `UZ` offset spécifié.
Contrainte : `OZ` offset non spécifié linéaire, `VA` offset non spécifié
angulaire (angle variable).

Élément tolérancé associé : `C` minimax (Tchebychev), `G` moindres carrés
(Gaussien), `N` minimal circonscrit, `T` tangent, `X` maximal inscrit.

Élément dérivé `A`, zone de tolérance projetée `P`.

Association d'élément d'évaluation : `C`, `CE`, `CI`, `G`, `GE`, `GI`, `N`, `X`
(E = contrainte extérieure matière, I = contrainte intérieure matière).

Paramètre : `T` étendue totale des écarts, `P` hauteur de pic, `V` profondeur
du creux, `Q` écart type — avec `T = P + V`.

Indicateurs d'éléments tolérancés : « entre », `UF` élément unifié,
`LD` diamètre intérieur, `MD` diamètre extérieur, tout autour, sur toute la
pièce.

Indicateurs : de tolérance, `ACS` toute section droite, de plan d'intersection,
de plan d'orientation, d'éléments de direction, de plan de collection.

TED (page 182) : valeur encadrée, dimensions linéaires ou angles entre les
références spécifiées.

Symboles définis dans d'autres normes (page 183) :
`M` maximum de matière (ISO 2692), `L` minimum de matière (ISO 2692),
`R` réciprocité (ISO 2692), `F` état libre, parties non rigides (ISO 10579).

## 2.8.3 — forme des zones de tolérance (pages 185 à 189)

Quatre tableaux, un par classe : 185/1 forme, 186/1 orientation, 188/1
position, 189/1 battement. Chacun donne, pour une caractéristique, la phrase
d'interprétation. C'est cette phrase qui fixe la **géométrie de la zone**, donc
si la valeur se préfixe d'un `ø`.

| Caractéristique | Élément | Zone |
|---|---|---|
| Rectitude | ligne | deux droites parallèles distantes de `t` |
| Rectitude | axe | zone cylindrique `øt` |
| Planéité | surface | deux plans parallèles distants de `t` |
| Circularité | section droite | deux cercles coplanaires concentriques, différence de rayons `t` |
| Cylindricité | surface | deux cylindres coaxiaux, différence de rayons `t` |
| Profil d'une ligne | ligne | deux lignes équidistantes, enveloppes des cercles `øt` |
| Profil d'une surface | surface | deux surfaces équidistantes, enveloppes des sphères `øt` |
| Parallélisme | axe / axe de référence | zone cylindrique `øt` |
| Parallélisme | axe / plan de référence | deux plans parallèles distants de `t` |
| Parallélisme | surface / plan de référence | deux plans parallèles distants de `t` |
| Perpendicularité | axe / plan de référence | zone cylindrique `øt` |
| Perpendicularité | surface / plan ou axe de référence | deux plans parallèles distants de `t` |
| Inclinaison | axe ou surface | deux plans parallèles distants de `t`, inclinés de l'angle TED |
| Localisation | axe | zone cylindrique `øt`, axe sur la position théorique exacte |
| Localisation | surface | deux plans parallèles distants de `t`, symétriques de la position théorique exacte |
| Coaxialité | axe | zone cylindrique `øt` d'axe la référence commune |
| Symétrie | plan médian ou axe | deux plans parallèles distants de `t`, symétriques du plan médian de référence |
| Battement circulaire radial | surface de révolution | `t` mesuré dans chaque plan de mesurage, sur une révolution complète |
| Battement circulaire axial | surface | `t` mesuré à chaque point de mesurage, sur une révolution complète |
| Battement total radial | surface | deux cylindres coaxiaux, différence de rayons `t`, axes sur la référence |

Le battement se subdivise dans le tableau 189/1 en radial et axial, alors que la
vue d'ensemble de la page 175 n'expose que « simple » et « total ». Les deux
lectures s'accordent : le symbole est le même, c'est la géométrie du dessin qui
distingue radial d'axial. MecaTool garde donc les deux caractéristiques de la
page 175, et rattache l'orientation au commentaire, pas au symbole.

Remarque de la page 187, sur le profil quand un élément de direction est
indiqué : « L'angle doit être indiqué même s'il est égal à 90°. »

## 2.8.2.5 (suite) — symboles de référence spécifiée (page 184)

`E` indicateur d'élément de référence (ISO 5459), indicateur de référence
partielle (ISO 5459), `CF` élément de contact (ISO 5459), `><` modificateur
pour contrainte d'orientation seulement (ISO 5459), `E` entouré : exigence
d'enveloppe (ISO 14405-1).

## Recoupement avec l'édition 2014

L'utilisateur a fourni en complément un VSM « Extrait de normes » **2014**
complet (432 pages), ce qui a aussi permis de dater les planches photographiées :
elles viennent de l'édition **2022**.

L'édition 2014 n'a pas non plus de couche de texte, mais c'est un balayage
propre et non des photographies, et sa pagination PDF coïncide avec celle du
livre. Son chapitre 2 est organisé autrement : « Tolérances géométriques » y est
le § 2.11, pages 90 à 101, et « États de surface » le § 2.12, page 102.

Le classement du § 2.11.1.7 (page 91) a été confronté à celui du § 2.8.2.1
(page 175 de 2022) :

- mêmes quatre familles, mêmes symboles, même exigence de référence spécifiée ;
- la note du § 2.11.1.4 énonce le même emboîtement que celle du § 2.8.1.3.2.

Deux écarts, qui sont des évolutions de la norme et non des désaccords :

| | 2014 | 2022 |
|---|---|---|
| Profils | dans la famille forme seulement, nommés « forme ligne / surface quelconque » | aussi en orientation et en position, nommés « profil d'une ligne / d'une surface » |
| Localisation | avec référence | avec **ou sans** référence |

MecaTool retient l'édition 2022. Le fait que les deux lectures concordent partout
ailleurs, et divergent exactement là où la norme a changé, est un argument de
plus en faveur de la transcription : une erreur de lecture n'aurait aucune
raison de tomber juste sur la structure de l'édition suivante.

## Ce qui reste disponible et n'est pas exploité

- **Tableau 157/1 (2022)** : proposition de transposition ISO 2768 → ISO 22081.
  C'est une *recommandation du recueil*, pas une valeur normative — le texte dit
  « montre une proposition ». À traiter comme telle le jour où elle sera reprise.
- **§ 2.9 (2022, pages 204 à 211)** : états de surface.
- **§ 2.12 (2014, pages 102 à 109)** : états de surface, édition antérieure à
  l'ISO 21920 ; les deux ne sont donc pas interchangeables.

## Relevé du chapitre soudure

Deux endroits, dans les deux éditions, et ils ne se recouvrent pas.

### Édition 2014, § 3.1 « Raccords soudés », pages 134 à 147

| Sujet | Pages | Norme citée | État de la source |
|---|---|---|---|
| Symboles élémentaires | 135 | SN EN 22553 (ISO 2553) | **exploitable** — 15 symboles, lisibles |
| Symboles complémentaires | 135 | ISO 2553 | **exploitable** |
| Règles d'insertion | 137 | ISO 2553 | exemples, pas l'énoncé complet |
| Indications complémentaires | 138 | ISO 2553 | exemples |
| Numéros de procédés | 138 | SN EN ISO 4063 | **partiel** — annonce huit groupes principaux, en montre quatre |
| Classification des irrégularités | 139 | SN EN ISO 6520-1 | groupes seulement |
| Niveaux de qualité B / C / D | 139 | SN EN ISO 5817 | **deux exemples** sur des dizaines |
| Cotation des cordons | 140 | — | exploitable |
| Préparation des joints | 141-147 | SN EN ISO 9692-1 à -4 | matrices de dessins, transcription risquée |

La numérotation des symboles élémentaires saute de 13 à 18 : les numéros 14 à 17
existent dans l'ISO 2553 mais ne figurent pas au recueil.

Le tableau 139/2 est titré « **Exemples** d'indications des limites
d'irrégularité ». C'est la limite décisive : la question « quel niveau de
qualité exiger » ne peut pas se répondre avec deux lignes d'exemple.

### Édition 2022, § 2.7.4, pages 163 et 164

| Sujet | Page | Norme | État |
|---|---|---|---|
| Tolérances linéaires, classes A à D | 163 | SN EN ISO 13920 | **complet** — 4 classes × 10 échelons |
| Tolérances angulaires, classes A à D | 163 | SN EN ISO 13920 | **complet** — en degrés et minutes, et en mm/m |
| Rectitude, planéité, parallélisme, classes E à H | 164 | SN EN ISO 13920 | **complet** — 4 classes × 9 échelons |

C'est le seul bloc de valeurs numériques complet de tout le chapitre, et il
tombe juste : les tolérances angulaires s'expriment en degrés et minutes
(`±20′`, `±1°30′`), ce que le type `Angle` en millisecondes d'arc représente
exactement — il avait été construit pour l'ISO 2768-1, qui pose le même problème.

Le recueil précise que l'ISO 13920 ne spécifie **pas** la coaxialité ni la
symétrie : « si de telles tolérances sont exigées pour des raisons
fonctionnelles, elles doivent être indiquées sur les dessins ». Une case vide
qui restera vide.

### Trouvaille de bord

Les pages 164 et 165 de l'édition 2022 portent aussi l'**ISO 8062-3** pour les
pièces moulées — classes DCTG et GCTG, tableaux complets. Hors sujet ici, mais
noté : c'est un domaine entier, adossable en l'état.

## Les quatre PDF fournis pour la soudure

Trois d'entre eux ne sont **pas** les normes : ce sont des fiches de
correspondance d'une page, issues de la gestion documentaire de l'entreprise,
qui renvoient vers l'équivalent européen.

| Fichier | Ce que c'est | Millésime attesté |
|---|---|---|
| `5817_ISO.pdf` | fiche de correspondance | ISO 5817 éd. 2003 + corr. 2005 + corr. 2006 → EN ISO 5817 août 2007 |
| `13920_ISO.pdf` | fiche de correspondance | ISO 13920 éd. 1996 → EN ISO 13920 août 1996 |
| `4063_ISO.pdf` | fiche de correspondance | ISO 4063 éd. 2009 + corr. 2010 → EN ISO 4063 déc. 2010 |
| `2553_ISO_(F)_2013…pdf` | **la norme**, 64 pages, français, couche de texte | ISO 2553:2013, quatrième édition |

Une fiche atteste du millésime **existant**, pas de celui que le recueil
transcrit. Elle ne remplit donc pas le champ d'édition ; elle est consignée en
note.

## ISO 2553:2013 — et ce qu'elle révèle du recueil

C'est la **première source primaire** du domaine soudure, et elle corrige une
erreur qu'aucune relecture du recueil n'aurait détectée.

Le tableau 135/1 du VSM 2014 porte quinze symboles, numérotés 1 à 13 puis 18 et
19, sous des désignations comme « Soudure en I » ou « Soudure sur bords
relevés ». J'avais noté ce saut de 13 à 18 comme une lacune du recueil.

Ce n'en était pas une. Le recueil cite SN EN 22553, c'est-à-dire l'**ISO
2553:1992**. La quatrième édition, de 2013, a renuméroté et renommé : elle
compte **vingt-deux** symboles élémentaires, et « Soudure en I » y devient
« soudure bout à bout à bords droits ».

Autrement dit : le recueil reproduit fidèlement une édition périmée. C'est
exactement le risque que l'état `secondary` était censé signaler, et il s'est
matérialisé. MecaTool suit la norme.

Le changement de fond de 2013 est l'existence de **deux systèmes**, A et B, qui
désignent différemment le côté de la soudure — double trait de référence contre
trait unique. La norme interdit de les mélanger et exige que le dessin dise
lequel il emploie. C'est une règle vérifiable.

## Le recueil confronté à la norme : ISO 13920

L'utilisateur a fourni la norme elle-même, NF EN ISO 13920 d'octobre 1996. La
transcription faite depuis le VSM 2022 lui a été confrontée par un script
rejouable — `scripts/verify-iso13920-tables.py`. **Quatre-vingts cases, aucun
écart de valeur sur les échelons communs.** Mais deux différences de couverture
et une erreur :

**Le recueil tronque.** La norme porte un échelon de plus dans les deux tableaux
de longueurs et de forme : « > 20 000 », ouvert, absent du recueil. Le tableau
des longueurs compte onze échelons et non dix ; celui de la forme, dix et non
neuf.

**Une valeur était fausse.** La classe E, échelon 400 à 1000, vaut **1,5 mm** dans
la norme. Elle avait été transcrite **1,6** depuis la photographie du recueil. Que
l'erreur vienne du recueil ou de ma lecture de la photo, elle est corrigée et
c'est la norme qui fait foi.

Le script vaut d'être expliqué, parce que les deux tableaux ont demandé deux
méthodes. Les longueurs portent toutes le signe `±`, qui sert donc de
séparateur. Les valeurs de forme sont nues, et la couche de texte les rend
collées : `0,511,52345678` est la ligne E entière, qu'aucune expression
régulière ne peut découper — `11,5` se lit « 1 puis 1,5 » ou « 11,5 » selon ce
qu'on cherche. On prend alors le problème à l'envers : au lieu de découper la
chaîne de la norme, on **reconstruit** la même chaîne depuis les valeurs
embarquées et on compare. Une seule valeur fausse, et les chaînes diffèrent.

Le script a été éprouvé en y injectant deux erreurs : il les a nommées, classe et
colonne comprises.

## L'ISO 15 confrontée : une attribution fausse

L'utilisateur a fourni l'ISO 15:2011(E), troisième édition. Elle corrige deux
choses, et aucune n'était une erreur de transcription.

**Le millésime n'était pas 1998.** L'utilisateur l'avait indiqué de mémoire, et
c'était vraisemblable. La norme dit **2011**. Le champ d'édition étant resté vide
par discipline, rien de faux n'avait été écrit.

**La règle n'est pas dans l'ISO 15.** Son domaine d'application est explicite :
« preferred boundary dimensions for radial bearings of the diameter series 7, 8,
9, 0, 1, 2, 3 and 4 ». Ce sont des **dimensions**. Ses tableaux donnent le
diamètre `d` en millimètres, directement, sans aucune colonne de symbole.

La règle « symbole = d/5 » que le recueil énonce sous un titre citant l'ISO 15
vient donc d'ailleurs — vraisemblablement d'une pratique de désignation, le
§ 4.14.1 du même recueil citant par ailleurs la DIN 623-1. Le jeu de données ne
s'attribue plus de norme, et un test empêche l'attribution de revenir.

**Ce que l'ISO 15 apporte en propre**, et qui est désormais embarqué comme donnée
vérifiée : les **soixante-treize diamètres d'alésage normalisés**, de 0,6 à
950 mm.

Confrontés à la règle du recueil, dix-huit d'entre eux n'ont aucun symbole :

| Diamètres | Pourquoi |
|---|---|
| 0,6 / 1,5 / 2,5 | non entiers, sous le domaine des symboles à un chiffre |
| **22 / 28 / 32** | entiers dans le domaine multiplié, mais pas multiples de cinq |
| 500 à 950 | au-delà du plafond de 480 mm |

Ce n'est pas une lacune de transcription : c'est une limite de la règle
elle-même, et MecaTool peut maintenant distinguer deux refus qui n'ont pas le
même sens — « 11 mm n'est pas un alésage normalisé » et « 22 mm en est un, mais
la règle ne sait pas l'écrire ».
