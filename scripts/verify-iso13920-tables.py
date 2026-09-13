"""Confronte les tables ISO 13920 embarquées à la norme elle-même.

Usage :
    pip install pypdf
    python scripts/verify-iso13920-tables.py chemin/vers/EN_ISO_13920.pdf

Le script n'écrit rien. Corriger une donnée et déclarer une source vérifiée
reste une décision humaine ; ce script ne fait que dire où regarder.

# Deux tableaux, deux méthodes

La couche de texte de ce PDF sort les tableaux sans séparateurs de colonnes.
Les deux tableaux ne s'y prêtent pas de la même façon, et il a fallu une méthode
par tableau plutôt qu'une seule approximative.

**Tableau 1 (longueurs).** Ses valeurs portent toutes le signe « ± ». Ce signe
sert donc de séparateur : les extraire dans l'ordre rend les quarante cases,
classe après classe, sans avoir à deviner où passent les colonnes. La cellule
fusionnée du premier échelon — un seul « ±1 » pour les quatre classes —
n'apparaît pas dans ce flux, ce qui tombe bien : elle est donc exclue de la
comparaison, et vérifiée à part.

**Tableau 3 (forme).** Ses valeurs sont nues : « 0,511,52345678 » est la ligne E
entière. Aucune expression régulière ne peut la découper — « 11,5 » se lit
« 1 puis 1,5 » ou « 11,5 » selon ce qu'on cherche.

On prend donc le problème à l'envers : au lieu de découper la chaîne de la
norme, on **reconstruit** la même chaîne depuis les valeurs embarquées et on
compare les deux. Une seule valeur fausse, et les chaînes diffèrent. C'est une
vérification aussi stricte qu'une comparaison case par case, et elle ne suppose
rien de la mise en page.

# Ce que ce script ne vérifie pas

Le tableau 2, angulaire : ses valeurs sont des angles en degrés et minutes, que
la couche de texte rend avec une océrisation trop instable pour être comparée
sans risque de faux écart. Il a été relu à l'image, à 250 points par pouce.

Les bornes des échelons, qui se lisent dans un en-tête sur trois lignes et ne
s'extraient pas dans l'ordre. Relues à l'image elles aussi.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

DATASET = (
    pathlib.Path(__file__).resolve().parent.parent
    / "data"
    / "soudure"
    / "iso13920.general-tolerances.json"
)


def render(value: float) -> str:
    """La valeur telle que la norme l'imprime : virgule décimale, sans zéro vain."""
    if value == int(value):
        return str(int(value))
    return f"{value}".replace(".", ",")


def section(text: str, start: str, stop: str) -> str:
    begin = text.find(start)
    if begin < 0:
        return ""
    end = text.find(stop, begin + len(start))
    return text[begin + len(start) : end if end > 0 else len(text)]


def check_linear(text: str, dataset: dict) -> tuple[int, int]:
    """Le tableau 1, découpé sur le signe ±."""
    chunk = section(text, "Tolerances t (en mm)", "4.2")
    # L'océrisation rend parfois « 5 » par « S » : c'est la seule substitution
    # admise, et elle ne peut pas créer d'ambiguïté puisque S n'est pas un
    # chiffre.
    raw = re.findall(r"±\s*([0-9S][0-9S,.\s]*?)(?=±|\n|$)", chunk)
    found = [float(v.strip().replace("S", "5").replace(",", ".")) for v in raw]

    expected: list[float] = []
    for letter in "ABCD":
        # La première colonne est la cellule fusionnée : elle n'est pas dans le
        # flux extrait, on la retire donc aussi de l'attendu.
        expected += [float(v) for v in dataset["linear"]["rows"][letter][1:]]

    if found == expected:
        return len(expected), 0

    print("Tableau 1 — les longueurs ne concordent pas.")
    for index, (a, b) in enumerate(zip(found, expected)):
        if a != b:
            letter = "ABCD"[index // 10]
            print(f"  classe {letter}, colonne {index % 10 + 2} : "
                  f"norme {a}, embarqué {b}")
    if len(found) != len(expected):
        print(f"  {len(found)} valeurs extraites, {len(expected)} attendues")
    return len(expected), 1


def check_form(text: str, dataset: dict) -> tuple[int, int]:
    """Le tableau 3, reconstruit puis comparé."""
    # Les deux tableaux portent le même en-tête « Tolerances t (en mm) » : on
    # ancre donc sur le titre du tableau 3, qui lui est unique.
    chunk = section(text, "Tableau 3", "Page 7")
    problems = 0
    compared = 0

    for letter in "EFGH":
        # La première classe est collée à l'en-tête — « (en mm)E » — alors que
        # les suivantes ouvrent leur ligne. Les deux cas sont admis.
        match = re.search(rf"[\n)]{letter}\n([0-9][0-9,\s]*)", chunk)
        if not match:
            print(f"Tableau 3, classe {letter} : ligne introuvable dans le PDF.")
            problems += 1
            continue

        from_pdf = re.sub(r"\s+", "", match.group(1))
        row = dataset["form"]["rows"][letter]
        rebuilt = "".join(render(v) for v in row)
        compared += len(row)

        if from_pdf != rebuilt:
            problems += 1
            print(f"Tableau 3, classe {letter} — la ligne ne se reconstruit pas.")
            print(f"  norme    : {from_pdf}")
            print(f"  embarqué : {rebuilt}")

    return compared, problems


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2

    try:
        import pypdf
    except ImportError:
        print("pypdf manquant : pip install pypdf")
        return 2

    reader = pypdf.PdfReader(sys.argv[1])
    text = "\n".join((page.extract_text() or "") for page in reader.pages)

    if "13920" not in text:
        print("Ce PDF ne semble pas être l'ISO 13920.")
        return 2

    dataset = json.loads(DATASET.read_text(encoding="utf-8"))

    linear_cells, linear_bad = check_linear(text, dataset)
    form_cells, form_bad = check_form(text, dataset)

    total = linear_cells + form_cells
    problems = linear_bad + form_bad

    print()
    print(f"Tableau 1, longueurs : {linear_cells} cases comparées.")
    print(f"Tableau 3, forme     : {form_cells} cases comparées.")
    print(f"{total} cases au total — "
          f"{'aucun écart' if not problems else f'{problems} tableau(x) en écart'}.")
    print()
    print("Non couverts, relus à l'image : le tableau 2 angulaire, et les")
    print("bornes des échelons.")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
