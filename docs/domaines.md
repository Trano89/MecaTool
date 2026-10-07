# Le modèle de domaines

Ce document explique ce qu'est MecaTool à partir de la version 0.3, et pourquoi
sa forme a changé.

## Ce que MecaTool était

Un assistant de tolérancement. Six écrans, tous sur le même sujet : ajustements
ISO 286, tolérances générales ISO 2768, chaînes de cotes, tolérancement
géométrique ISO 1101. L'utilisateur choisissait un onglet, puis saisissait.

Cette forme a une limite qui n'apparaît qu'en voulant l'étendre : **elle suppose
que l'utilisateur sait déjà dans quel onglet se trouve sa réponse.** C'est vrai
quand il y en a six et qu'ils traitent tous du même sujet. Ce l'est beaucoup
moins avec douze domaines couvrant roulements, soudure, visserie et matériaux.

## Ce que MecaTool devient

Un **atelier** : un outil qui accompagne les décisions de conception mécanique,
dont le tolérancement n'est qu'un domaine parmi d'autres.

Le cahier des charges posait dès le départ la bonne trajectoire :

```
BESOIN → CALCUL → COMPARAISON → VALIDATION → EXPLICATION → VISUALISATION
```

Cette trajectoire n'a jamais eu quoi que ce soit de spécifique au
tolérancement. Choisir un roulement la suit. Choisir un cordon de soudure la
suit. C'est le **squelette de l'outil**, pas celui d'un module.

## La découverte qui a décidé de la forme

En relevant les sources disponibles pour les roulements, un tableau a changé le
plan : le tableau 237/1 du VSM, « Champs de tolérance usuelles pour le montage
des roulements ». Il donne, selon les conditions d'utilisation — bague tournante
ou fixe, charge légère, normale ou forte — la classe de tolérance à porter sur
l'arbre. Le tableau 238/1 fait de même pour l'alésage du logement.

Autrement dit : **le domaine « roulements » produit une entrée du domaine
« ajustements »**.

```
« je monte un 6210, bague intérieure tournante, charge normale »
        │
        ▼   domaine roulements : décodage de la désignation
   alésage d = 50 mm
        │
        ▼   domaine roulements : tableau 237/1
   classe k6 sur l'arbre
        │
        ▼   domaine ajustements : moteur ISO 286 existant
   Ø50 k6 → écarts, ajustement avec le roulement, graphique, verdict
```

Aucune de ces trois étapes n'est nouvelle en soi. Ce qui est nouveau, c'est
qu'elles s'enchaînent. Un outil « à tout faire » n'est pas une collection de
calculateurs côte à côte : c'est un ensemble de domaines **qui se passent des
résultats**.

Cela a trois conséquences d'architecture.

## Conséquence 1 — les domaines sont déclarés, pas codés en dur

Un domaine se décrit : un identifiant, la question qu'il répond, les formes de
saisie qu'il reconnaît, les normes dont il dépend. L'interface lit cette
déclaration au lieu de porter une liste d'onglets écrite à la main.

Ajouter un domaine ne doit toucher ni la navigation, ni l'accueil, ni la
recherche. Tant que ce n'est pas vrai, le douzième domaine coûtera douze fois
plus cher que le deuxième.

## Conséquence 2 — une seule entrée, pas douze onglets

Le cahier des charges prévoyait une « recherche universelle » (§34) restée non
implémentée. Elle cesse d'être un agrément pour devenir la pièce maîtresse :
c'est elle qui dispense l'utilisateur de savoir dans quel domaine chercher.

```
Ø10 H7/g6      → ajustement
25 ±0.17       → cote seule
6210           → roulement
⟂ 0.05 A       → spécification géométrique
ISO 2768-m     → tolérances générales
141            → procédé de soudage
```

Le routage appartient au **moteur**, pas à l'interface : reconnaître qu'une
saisie est une désignation de roulement suppose de connaître la règle du
symbole d'alésage, et cette règle est normative.

Une saisie ambiguë ne doit pas être tranchée au hasard. `6210` est une
désignation de roulement, mais c'est aussi un nombre. Le moteur rend alors
**plusieurs lectures**, et c'est l'utilisateur qui choisit — le même principe
que pour le symbole `⌒`, qui désigne trois caractéristiques géométriques et dont
le moteur rend les trois.

## Conséquence 3 — un résultat peut en appeler un autre

Une conclusion n'est plus forcément terminale. Le domaine roulements conclut
« classe k6 sur l'arbre », et cette conclusion porte de quoi relancer le domaine
ajustements. L'utilisateur suit la chaîne au lieu de recopier une valeur d'un
écran à l'autre.

C'est aussi ce qui rend l'outil pédagogique sans mode « apprentissage » séparé :
la chaîne montre d'où vient chaque décision.

## Ce que cela ne change pas

**Les sept principes tiennent.** L'arithmétique reste exacte, la provenance
reste obligatoire, le frontend reste dépourvu de règle normative, et le moteur
continue de refuser ce qu'il n'a pas.

**La règle absolue tient d'autant plus.** Élargir le périmètre multiplie les
occasions d'inventer une règle. Chaque domaine ajouté doit donc dire, dans sa
déclaration, sur quelle source il repose et à quel titre — norme confrontée,
recueil, ou recommandation de fabricant.

## Les domaines, et l'état de leur source

| Domaine | Question | Source | État |
|---|---|---|---|
| Ajustements | Que donne cet ajustement ? Lequel choisir ? | ISO 286-1/-2 | ✅ confrontée |
| Tolérances générales | Que valent les cotes sans tolérance ? | ISO 2768-1 | ✅ confrontée |
| Chaînes de cotes | Que donne cet empilement ? | — (géométrie) | ✅ sans source externe |
| Tolérancement géométrique | Que dit ce cadre, que lui manque-t-il ? | ISO 1101 | ⚠️ recueil |
| **Roulements** | **Quel alésage, quelle tolérance de portée ?** | **ISO 15, fabricants** | **✅ construit** |
| **Soudure** | **Que dit ce symbole, que tolère son niveau de qualité ?** | **ISO 2553:2013, 4063, 5817** | **🟠 construit ; symboles ✅ vérifiés, le reste non** |
| **Visserie** | **Quel filetage, quel trou de passage ?** | **ISO 261, 68-1, 273, 898-1** | **🟠 construit, données non vérifiées** |
| Matériaux | Quelle nuance, quelles propriétés ? | EN 10027 et suivantes | ⏳ à relever |
| **États de surface** | **Que dit cette indication, quel procédé l'obtient ?** | **ISO 21920-1, ISO 1302:1992** | **🟠 construit, données non vérifiées** |

## Ouvrir un domaine sur des données non vérifiées

États de surface, soudure et visserie ont été construits avant que leurs
sources soient confrontées. C'est permis — la règle absolue interdit de faire
passer une donnée non vérifiée pour établie, pas de s'en servir — à trois
conditions, toutes tenues par le code :

1. **L'état se voit.** Les jeux portent `unverified`, le registre en déduit
   l'état « réserve », et chaque écran affiche les réserves avant la saisie.
2. **Chaque fichier dit quoi faire.** Le champ `pending` nomme la source contre
   laquelle confronter le jeu, et le champ `source` dit honnêtement comment il a
   été saisi.
3. **Le périmètre est une sélection, et le dit.** Un numéro de procédé, un
   diamètre ou une imperfection absents ne sont pas déclarés inexistants : le
   moteur dit qu'il ne les connaît pas.

Le jour où une source est confrontée, seul le bloc `verification` change ; le
domaine passe de « réserve » à « prêt » sans que personne ait à y penser.

## Les états de surface, et la frontière des générations

Le domaine était bloqué pour une bonne raison : l'ISO 21920 et l'ISO 4287/1302
ne sont pas interchangeables. Il ne s'ouvre pas en choisissant l'une contre
l'autre, mais en **ne portant que leur tronc commun** — trois variantes du
symbole, sept sens des stries, la signification des paramètres d'amplitude. Ce
qui les distingue reste dehors, et le raisonnement de chaque lecture le dit sous
« Ce que MecaTool ne fournit pas ».

## Un quatrième état de source

Le tableau 237/1 porte une mention qui a imposé une distinction de plus :
« **Dimensions du fabricant** ». Ces classes de tolérance ne sont pas une
exigence normative — ce sont les pratiques de montage recommandées par les
fabricants de roulements, relayées par le recueil.

La différence compte pour l'utilisateur. Une valeur ISO s'impose ; une
recommandation de fabricant s'écarte, si on sait pourquoi. Les présenter de la
même façon serait trompeur dans les deux sens : cela durcirait la seconde et
banaliserait la première.

D'où un quatrième état, à côté de `verified`, `secondary` et `unverified` :

```json
"verification": {
  "state": "recommended",
  "by": "fabricants de roulements, via le VSM « Extrait de normes » 2014, tableau 237/1",
  "on": "2026-09-11"
}
```

Il compte comme non vérifié à l'affichage, et son message dit ce qu'il est :
une pratique recommandée, pas une règle. Voir [standards.md](standards.md).
