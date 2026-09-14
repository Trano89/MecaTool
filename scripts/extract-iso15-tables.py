#!/usr/bin/env python3
"""Extrait les tableaux 1 a 8 de l'ISO 15:2011 (dimensions d'encombrement).

# Pourquoi un script, et non une transcription a la main

Huit tableaux, environ 3 500 valeurs. Une transcription manuelle a cette
echelle ne serait pas plus sure : elle serait seulement plus lente a se
tromper. Le script lit la couche de texte de la norme en mode `-layout`, qui
conserve les colonnes.

# Ce que la norme a d'irregulier, et qu'il ne faut surtout pas supposer

Chaque serie de diametres a SON PROPRE jeu de series de dimensions, et son
propre decoupage de la colonne `rs min` :

    serie 7 : 17 27 37 47                   -> 1 colonne  rs min
    serie 8 : 08 18 28 38 48 58 68          -> 2 colonnes (08 | 18 a 68)
    serie 9 : 09 19 29 39 49 59 69          -> 3 colonnes (09 | 19 a 39 | 49 a 69)
    serie 0 : 00 10 20 30 40 50 60          -> 2 colonnes
    serie 1 : 01 11 21 31 41 51 61          -> 2 colonnes
    serie 2 : 82 02 12 22 32 42 52 62       -> 2 colonnes
    serie 3 : 83 03 13 23 33                -> 2 colonnes
    serie 4 : 04 24                         -> 1 colonne, et le tableau est
                                               imprime en DEUX BLOCS COTE A COTE

Un analyseur qui supposerait une forme unique produirait des lignes decalees
sans rien signaler. D'ou le controle ci-dessous : le nombre de valeurs d'une
ligne doit valoir exactement 2 + len(series) + len(groupes_rs), sinon la ligne
est rejetee et comptee.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

# Releve sur les en-tetes des huit tableaux, pas devine.
TABLES = {
    "7": {"table": 1, "series": ["17", "27", "37", "47"], "rs": ["17 to 47"], "blocs": 1},
    "8": {"table": 2, "series": ["08", "18", "28", "38", "48", "58", "68"],
          "rs": ["08", "18 to 68"], "blocs": 1},
    "9": {"table": 3, "series": ["09", "19", "29", "39", "49", "59", "69"],
          "rs": ["09", "19 to 39", "49 to 69"], "blocs": 1},
    "0": {"table": 4, "series": ["00", "10", "20", "30", "40", "50", "60"],
          "rs": ["00", "10 to 60"], "blocs": 1},
    "1": {"table": 5, "series": ["01", "11", "21", "31", "41", "51", "61"],
          "rs": ["01", "11 to 61"], "blocs": 1},
    "2": {"table": 6, "series": ["82", "02", "12", "22", "32", "42", "52", "62"],
          "rs": ["82", "02 to 62"], "blocs": 1},
    "3": {"table": 7, "series": ["83", "03", "13", "23", "33"],
          "rs": ["83", "03 to 33"], "blocs": 1},
    "4": {"table": 8, "series": ["04", "24"], "rs": ["04 to 24"], "blocs": 2},
}

JETON = re.compile(r"^(?:\d+(?:,\d+)?|—|⎯|-)$")


def cellules(ligne):
    """Decoupe une ligne en cellules, puis recolle les separateurs de milliers.

    La sortie `-layout` distingue les deux usages de l'espace par leur NOMBRE :
    une frontiere de colonne en aligne plusieurs, un separateur de milliers n'en
    pose qu'une seule (« 1 030 »). Le decoupage se fait donc sur deux espaces ou
    plus, et ce qui reste d'espace SIMPLE a l'interieur d'une cellule est un
    separateur de milliers — a condition que la forme s'y prete.

    Une cellule qui garde une espace apres ce recollement est rendue telle
    quelle : elle sera comptee comme plusieurs jetons et fera echouer le
    controle de compte, ce qui vaut mieux qu'un recollement invente.
    """
    brutes = re.split(r"\s{2,}", ligne.strip())
    sorties = []
    for cellule in brutes:
        if not cellule:
            continue
        # « 1 030 » -> « 1030 » ; « 1 030 500 » -> « 1030500 ».
        while (m := re.search(r"(?<![\d,])(\d{1,3}) (\d{3})(?![\d,])", cellule)):
            cellule = cellule[:m.start()] + m.group(1) + m.group(2) + cellule[m.end():]
        sorties.extend(cellule.split())
    return sorties


def recollements_possibles(jetons, attendu):
    """Toutes les lectures d'une ligne qui donnent exactement `attendu` valeurs.

    # Le probleme

    Au-dela de 1000, la norme ecrit « 1 030 » avec une ESPACE ordinaire, la meme
    qui separe deux colonnes. Aucune regle de caractere ne les distingue, et les
    coordonnees ne sauvent pas : la ligne entiere sort du PDF comme un seul
    fragment de texte.

    # Ce que fait cette fonction

    Plutot que de deviner, elle enumere. Un recollement n'est tente que sur un
    couple `(x, yyy)` dont la seconde moitie fait exactement trois chiffres ;
    la ligne n'est acceptee que si UNE SEULE lecture atteint le compte attendu.
    Deux lectures possibles, c'est une ambiguite reelle : la ligne part au
    rejet plutot que de trancher a pile ou face.
    """
    manquants = len(jetons) - attendu
    if manquants < 0:
        return []
    if manquants == 0:
        return [jetons]

    # Positions ou un recollement est formellement licite.
    candidats = [
        i for i in range(len(jetons) - 1)
        if re.fullmatch(r"\d{1,3}", jetons[i]) and re.fullmatch(r"\d{3}", jetons[i + 1])
    ]
    if len(candidats) < manquants:
        return []

    lectures = []
    from itertools import combinations
    for choix in combinations(candidats, manquants):
        # Deux recollements ne peuvent pas se chevaucher.
        if any(b - a == 1 for a, b in zip(choix, choix[1:])):
            continue
        lu, saut = [], set()
        for i, jeton in enumerate(jetons):
            if i in saut:
                continue
            if i in choix:
                lu.append(jeton + jetons[i + 1])
                saut.add(i + 1)
            else:
                lu.append(jeton)
        lectures.append(lu)
    return lectures


def nombre(jeton):
    """Convertit un jeton en flottant, ou None pour un tiret (dimension absente)."""
    if jeton in {"—", "⎯", "-"}:
        return None
    return float(jeton.replace(",", "."))


def lignes_du_pdf(pdf):
    sortie = subprocess.run(
        ["pdftotext", "-layout", "-enc", "UTF-8", str(pdf), "-"],
        capture_output=True, text=True, encoding="utf-8", check=True)
    return sortie.stdout.split("\n")


def decoupe_en_tableaux(lignes):
    """Associe a chaque serie de diametres les lignes qui la concernent."""
    debut = re.compile(r"Table (\d) — Diameter series (\d)")
    suite = re.compile(r"Table (\d) \(continued\)")
    # Apres le tableau 8 vient l'annexe A, dont le tableau A.1 aligne huit
    # nombres — exactement la forme d'une ligne de donnees. Sans cette borne, la
    # serie 4 avalerait la fin du document.
    fin = re.compile(r"^\s*Annex A|Bibliography")
    blocs, serie_courante = {}, None
    for ligne in lignes:
        if (m := debut.search(ligne)):
            serie_courante = m.group(2)
            blocs.setdefault(serie_courante, [])
            continue
        if (m := suite.search(ligne)):
            numero = int(m.group(1))
            serie_courante = next(
                (s for s, d in TABLES.items() if d["table"] == numero), None)
            continue
        if fin.search(ligne):
            serie_courante = None
            continue
        if serie_courante:
            blocs[serie_courante].append(ligne)
    return blocs


def analyse(serie, lignes):
    forme = TABLES[serie]
    attendu = 2 + len(forme["series"]) + len(forme["rs"])
    total = attendu * forme["blocs"]

    rangees, rejets = [], []
    for ligne in lignes:
        jetons = cellules(ligne)
        if not jetons or not all(JETON.match(j) for j in jetons):
            continue
        lectures = recollements_possibles(jetons, total)
        if len(lectures) != 1:
            raison = "ambigue" if len(lectures) > 1 else "compte"
            rejets.append((f"{len(jetons)}/{total} {raison}", ligne.strip()[:80]))
            continue
        jetons = lectures[0]
        for bloc in range(forme["blocs"]):
            part = jetons[bloc * attendu:(bloc + 1) * attendu]
            d, D = nombre(part[0]), nombre(part[1])
            if d is None or D is None:
                rejets.append((len(jetons), ligne.strip()[:90]))
                continue
            largeurs = {
                nom: nombre(v)
                for nom, v in zip(forme["series"], part[2:2 + len(forme["series"])])}
            chanfreins = {
                nom: nombre(v)
                for nom, v in zip(forme["rs"], part[2 + len(forme["series"]):])}
            rangees.append({"bloc": bloc, "d": d, "D": D,
                            "B": largeurs, "rs_min": chanfreins})
    return rangees, rejets


def main():
    pdf = Path(sys.argv[1])
    blocs = decoupe_en_tableaux(lignes_du_pdf(pdf))

    resultat, total, rejets_total = {}, 0, 0
    for serie in TABLES:
        rangees, rejets = analyse(serie, blocs.get(serie, []))
        # Les d doivent croitre strictement DANS CHAQUE BLOC : le tableau 8 est
        # imprime en deux colonnes de tableau cote a cote, et melanger les deux
        # ferait passer un zigzag normal pour une anomalie.
        croissant = True
        for bloc in range(TABLES[serie]["blocs"]):
            suite = [r["d"] for r in rangees if r["bloc"] == bloc]
            croissant &= all(a < b for a, b in zip(suite, suite[1:]))
        # Et D doit toujours depasser d : un recollement mal place le trahirait.
        coherent = all(r["D"] > r["d"] for r in rangees)
        resultat[serie] = rangees
        total += len(rangees)
        rejets_total += len(rejets)
        print(f"serie {serie} (tableau {TABLES[serie]['table']}) : "
              f"{len(rangees):3d} lignes, {len(rejets):2d} rejets, "
              f"d croissant : {croissant}, D > d : {coherent}")
        for n, apercu in rejets[:3]:
            print(f"    rejet ({n} valeurs) : {apercu}")

    print(f"\ntotal {total} lignes, {rejets_total} rejets")
    Path(sys.argv[2]).write_text(
        json.dumps(resultat, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
