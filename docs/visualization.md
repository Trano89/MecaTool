# Visualisation

## Le principe : la géométrie est calculée en Rust

Un graphique qui contredirait les valeurs affichées serait pire qu'absent. La
position et la hauteur de chaque zone sont donc calculées dans
`mecatool-engine::diagram`, à partir des `Length` exactes du moteur, et testées.

L'interface — CLI, application Tauri, export — se contente de tracer les
rectangles qu'on lui donne. **Il n'y a aucune conversion à faire côté
frontend**, donc aucune erreur de conversion possible.

```
Fit (valeurs exactes)
      ↓  fit_diagram()
Diagram (coordonnées + valeurs exactes conservées)
      ↓
   ┌──┴──────────────┐
to_svg()          React SVG
(export)          (thème clair/sombre)
```

Le `Diagram` est sérialisable : il traverse la frontière Tauri tel quel.

## Ce que les tests garantissent

Le contrôle central est un aller-retour : chaque bord de bande, reconverti en
écart via `deviation_at()`, doit redonner l'écart dont il est issu. Si le moteur
change une valeur, le dessin suit ; s'il diverge, le test tombe.

S'y ajoutent :

- la ligne zéro correspond à un écart nul ;
- la hauteur d'une bande est proportionnelle à son IT, et le rapport des
  hauteurs vaut le rapport des tolérances ;
- chaque cote de jeu mesure à l'écran exactement sa valeur ;
- les bandes restent dans leurs marges, sur des nominaux de 3 à 250 mm ;
- deux ajustements différents ne peuvent pas produire la même géométrie
  (garde-fou contre un dessin figé).

## Les deux cotes de jeu

Le jeu n'est pas une distance unique entre les deux zones. Chaque borne relie
une paire d'arêtes précise :

```
        H7
   +----------+  ES = +21  ─────────┐
   |//////////|                     │ jeu maximal = ES − ei
───+----------+──── 0 ────┐         │
        EI = 0            │         │
                          │ jeu     │
              +----------+│ minimal │
              |//////////|│ = EI−es │
   es = −7 ───+----------+┘         │
              |//////////|          │
   ei = −20 ──+----------+──────────┘
                    g6
```

Tracer un seul trait entre les deux zones laisserait croire que la distance
visible représente toute la plage de jeu. C'est faux, et c'est le genre
d'erreur qu'un dessin fait commettre sans qu'on s'en rende compte.

Quand la valeur est négative, la cote est nommée **serrage** plutôt que « jeu
négatif » : le lecteur n'a pas à interpréter un signe.

## L'annonce d'échelle est obligatoire

Les écarts se comptent en micromètres, la pièce en millimètres. Un dessin qui
respecterait les deux échelles à la fois rendrait les zones invisibles : à Ø20,
une zone de 21 µm représente un millième du diamètre.

Deux modes existent donc, et **le diagramme annonce toujours son échelle** :

| Mode | Ce qu'il montre | Annonce |
|---|---|---|
| `Deviations` (défaut) | les écarts occupent la hauteur disponible | « Écarts amplifiés pour la lisibilité : 1 µm = 6,1 px. À cette échelle, le diamètre nominal de 20 mm mesurerait environ 32,5 m. » |
| `TrueToScale` | pièce et écarts au même rapport | « Échelle fidèle : la pièce et ses écarts sont au même rapport. » |

L'annonce du mode `Deviations` traduit le facteur en une grandeur parlante — ce
que mesurerait la pièce si elle était dessinée à la même échelle que ses écarts.
Un lecteur ne peut alors pas prendre le dessin pour une représentation fidèle
des proportions.

En mode fidèle, les zones sont dessinées avec une épaisseur minimale de 0,75 px :
sans cela, le dessin effacerait purement et simplement les tolérances.

## Accessibilité

L'information ne repose jamais sur la seule couleur :

- les deux zones portent des **hachures d'orientation différente** en plus de
  leurs couleurs ;
- chaque zone est **étiquetée** par sa désignation (`H7`, `g6`) et son degré ;
- chaque écart est **écrit** à côté de l'arête qu'il décrit ;
- chaque cote de jeu porte sa valeur en toutes lettres.

Le dessin reste lisible en noir et blanc comme en cas de daltonisme.

## Mise en page

Les étiquettes d'écarts occupent les marges latérales — à gauche pour
l'alésage, à droite pour l'arbre — ce qui laisse l'espace entre les deux zones
libre pour les cotes de jeu. Chaque cote se place à une abscisse distincte et
porte son étiquette au-dessus de sa ligne, jamais au milieu : le milieu d'une
cote longue tombe près de la ligne zéro, où il rencontrerait son étiquette.

L'étiquette de la ligne zéro se réduit à « 0 » : la ligne traverse tout le
dessin, et tout texte plus long finit par croiser une zone ou une cote. Son sens
est rappelé dans la légende, sous le dessin.

## Exemples

```bash
cargo run -p mecatool-cli -- "Ø20 H7/g6" --svg exemple.svg
cargo run -p mecatool-cli -- "Ø20 H7/p6" --svg serrage.svg --fidele
```

- [exemple-h7g6.svg](exemple-h7g6.svg) — ajustement avec jeu
- [exemple-h7p6.svg](exemple-h7p6.svg) — ajustement avec serrage
