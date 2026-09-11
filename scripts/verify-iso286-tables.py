"""Confronte les tables normatives de MecaTool au texte de l'ISO 286-2:2010.

Une vérification faite à la main n'est pas une vérification : elle ne se rejoue
pas, et personne ne peut la contrôler. Ce script lit le PDF de la norme, en
extrait les valeurs, et les compare case par case aux fichiers de `data/`.

    python scripts/verify-iso286-tables.py chemin/vers/286-2_ISO.pdf

Il n'écrit rien. Corriger une donnée et déclarer une source vérifiée reste une
décision humaine ; le script dit seulement où ça diverge.

Dépendance : `pip install pypdf`.

## Comment il lit une table

L'extraction textuelle simple entremêle les colonnes des tables à deux lettres.
Le script relève donc la **position** de chaque fragment et reconstitue les
lignes telles qu'elles sont imprimées.

Chaque échelon de dimensions occupe deux lignes de valeurs : l'écart supérieur
au-dessus du libellé, l'écart inférieur en dessous. L'écart *fondamental* est
celui que la lettre fixe — l'écart supérieur pour a..h, l'inférieur pour j..zc —
et il est **constant sur tous les degrés**. Une table à deux lettres présente
donc deux plages de valeurs constantes accolées, ce qui suffit à les séparer
sans avoir à deviner les frontières de colonnes.

## Ce qu'il ne vérifie pas

`js` n'est pas tabulé comme un écart fondamental : c'est la règle « ± IT/2 »,
vérifiée par les tests du moteur.

`k` a un écart qui dépend du degré — nul hors des degrés IT4 à IT7 — donc sa
ligne n'est pas une plage constante. Le tableau 24 de la norme le confirme
directement : la ligne des écarts inférieurs y vaut `0` partout sauf sous les
quatre colonnes IT4 à IT7.
"""

from __future__ import annotations

import json
import re
import sys
from collections import defaultdict
from decimal import Decimal
from pathlib import Path

try:
    from pypdf import PdfReader
except ImportError:  # pragma: no cover - message d'aide, pas de logique
    sys.exit("pypdf est requis : pip install pypdf")

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "iso286"

IT_TABLE_PAGE = 10
GRADES = ["IT01", "IT0"] + [f"IT{n}" for n in range(1, 19)]
# Les treize premières colonnes du tableau 1 sont en micromètres, les sept
# dernières en millimètres. C'est le piège principal de cette table.
MICROMETRE_COLUMNS = 13

RANGES: list[tuple[str, str]] = [
    ("—", "3"), ("3", "6"), ("6", "10"), ("10", "18"), ("18", "30"),
    ("30", "50"), ("50", "80"), ("80", "120"), ("120", "180"),
    ("180", "250"), ("250", "315"), ("315", "400"), ("400", "500"),
]

# Où trouver chaque lettre, et laquelle des plages constantes la porte.
#
# `letters` est le nombre de lettres que la table présente côte à côte : il borne
# le nombre de plages constantes attendu sur une ligne d'écarts fondamentaux, et
# sert donc à distinguer cette ligne de celle qui varie avec le degré.
SHAFT_LETTERS: dict[str, dict[str, str | int]] = {
    "D": {"page": 32, "letters": 2, "run": "last"},   # table « cd et d »
    "E": {"page": 33, "letters": 2, "run": "first"},  # table « e et ef »
    "F": {"page": 34, "letters": 2, "run": "first"},  # table « f et fg »
    "G": {"page": 35, "letters": 1, "run": "only"},
    "H": {"page": 36, "letters": 1, "run": "only"},
    "M": {"page": 39, "letters": 2, "run": "first"},  # table « m et n »
    "N": {"page": 39, "letters": 2, "run": "last"},
    "P": {"page": 40, "letters": 1, "run": "only"},
}

SIGNS = {"+": 1, "-": -1, "−": -1, "–": -1}


def positioned_rows(pdf: Path, page_number: int, tolerance: float = 3.0):
    """Reconstitue les lignes imprimées d'une page, de haut en bas."""
    fragments: list[tuple[float, float, str]] = []

    def visit(text, _cm, tm, _font, _size):
        content = text.strip()
        if content:
            fragments.append((round(tm[5], 1), round(tm[4], 1), content))

    PdfReader(str(pdf)).pages[page_number - 1].extract_text(visitor_text=visit)

    buckets: dict[float, list[tuple[float, str]]] = defaultdict(list)
    for y, x, content in fragments:
        key = next((k for k in buckets if abs(k - y) <= tolerance), y)
        buckets[key].append((x, content))

    for y in sorted(buckets, reverse=True):
        yield y, [c for _, c in sorted(buckets[y])]


def merge_signs(cells: list[str]) -> list[str]:
    """Recolle les signes détachés de leur nombre par la mise en page."""
    merged: list[str] = []
    pending = ""
    for cell in cells:
        if cell in SIGNS:
            pending = cell
            continue
        merged.append(pending + cell)
        pending = ""
    return merged


def as_number(token: str) -> Decimal | None:
    if not re.fullmatch(r"[+\-−–]?\d+(,\d+)?", token):
        return None
    sign = SIGNS.get(token[0], 1) if token[0] in SIGNS else 1
    digits = token.lstrip("+-−–").replace(",", ".")
    return Decimal(digits) * sign


def value_rows(pdf: Path, page: int) -> list[list[Decimal]]:
    """Les lignes entièrement numériques, dans l'ordre d'impression."""
    rows: list[list[Decimal]] = []
    for _y, cells in positioned_rows(pdf, page):
        merged = merge_signs(cells)
        numbers = [as_number(c) for c in merged]
        if merged and all(n is not None for n in numbers):
            rows.append([n for n in numbers if n is not None])
    return rows


def constant_runs(values: list[Decimal]) -> list[Decimal]:
    """Les valeurs distinctes de plages consécutives identiques."""
    runs: list[Decimal] = []
    for value in values:
        if not runs or runs[-1] != value:
            runs.append(value)
    return runs


def read_shaft_letter(pdf: Path, letter: str) -> list[Decimal]:
    """Les écarts fondamentaux d'une lettre, échelon par échelon.

    On ne cherche pas la ligne à une position donnée : la mise en page du PDF
    n'est pas assez régulière pour cela d'une table à l'autre. On s'appuie sur
    une propriété du contenu — **l'écart fondamental ne dépend pas du degré**,
    donc sa ligne est faite de plages de valeurs constantes, là où la ligne de
    l'autre écart varie à chaque colonne. Une ligne d'écarts fondamentaux
    présente donc au plus autant de plages que la table compte de lettres.
    """
    spec = SHAFT_LETTERS[letter]
    expected_runs = int(spec["letters"])

    selected = [
        row
        for row in value_rows(pdf, int(spec["page"]))
        if len(row) >= 3 and len(constant_runs(row)) <= expected_runs
    ][: len(RANGES)]

    if len(selected) < len(RANGES):
        raise SystemExit(
            f"{letter} : {len(selected)} échelons lus sur {len(RANGES)} (page {spec['page']})"
        )

    values: list[Decimal] = []
    for row in selected:
        runs = constant_runs(row)
        if spec["run"] == "first":
            values.append(runs[0])
        else:
            # La seconde lettre d'une table disparaît aux grandes dimensions ;
            # il ne reste alors qu'une plage, qui est bien celle recherchée.
            values.append(runs[-1])
    return values


def compare_it_grades(pdf: Path) -> list[str]:
    page = PdfReader(str(pdf)).pages[IT_TABLE_PAGE - 1]
    try:
        raw = page.extract_text(extraction_mode="layout")
    except Exception:
        raw = page.extract_text()

    tokens = (raw or "").split()
    tokens = tokens[tokens.index("ing") + 1 :] if "ing" in tokens else tokens

    published: dict[str, list[Decimal]] = {}
    cursor = 0
    for above, up_to in RANGES:
        while cursor < len(tokens) - 1 and not (
            tokens[cursor] == above and tokens[cursor + 1] == up_to
        ):
            cursor += 1
        if cursor >= len(tokens) - 1:
            return [f"échelon {above}..{up_to} introuvable dans le tableau 1"]
        cursor += 2

        row: list[Decimal] = []
        while len(row) < len(GRADES) and cursor < len(tokens):
            value = as_number(tokens[cursor])
            cursor += 1
            if value is not None:
                row.append(value)
        published[up_to] = [
            v if i < MICROMETRE_COLUMNS else v * 1000 for i, v in enumerate(row)
        ]

    dataset = json.loads((DATA / "iso286-1-2010.it-grades.json").read_text(encoding="utf-8"))
    bounds = [str(pair[1]) for pair in dataset["size_ranges"]]

    problems: list[str] = []
    checked = 0
    for grade_index, grade in enumerate(GRADES):
        for range_index, up_to in enumerate(bounds):
            expected = published[up_to][grade_index]
            actual = Decimal(str(dataset["grades"][grade][range_index]))
            checked += 1
            if expected != actual:
                problems.append(
                    f"{grade}, jusqu'à {up_to} mm : MecaTool {actual}, ISO {expected}"
                )
    print(f"Tableau 1, degrés de tolérance  : {checked} cases comparées", end="")
    print(" — aucun écart." if not problems else f" — {len(problems)} écart(s).")
    return problems


def compare_shaft_deviations(pdf: Path) -> list[str]:
    dataset = json.loads(
        (DATA / "iso286-1-2010.shaft-deviations.json").read_text(encoding="utf-8")
    )
    problems: list[str] = []
    checked = 0

    for letter, entry in sorted(dataset["letters"].items()):
        if letter not in SHAFT_LETTERS:
            print(f"  {letter:<3} non vérifiable automatiquement (voir l'en-tête du script)")
            continue
        published = read_shaft_letter(pdf, letter)
        ours = entry["values"]
        for index, (expected, actual) in enumerate(zip(published, ours)):
            checked += 1
            if Decimal(str(actual)) != expected:
                bound = RANGES[index][1]
                problems.append(
                    f"{letter.lower()}, jusqu'à {bound} mm : "
                    f"MecaTool {actual} µm, ISO {expected} µm"
                )

    print(f"Tableaux 18 à 26, écarts arbres : {checked} cases comparées", end="")
    print(" — aucun écart." if not problems else f" — {len(problems)} écart(s).")
    return problems


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    pdf = Path(sys.argv[1])
    if not pdf.is_file():
        print(f"introuvable : {pdf}")
        return 2

    print(f"Source : {pdf.name}\n")
    problems = compare_it_grades(pdf) + compare_shaft_deviations(pdf)

    if problems:
        print(f"\n{len(problems)} écart(s) à traiter :")
        for line in problems:
            print(f"  • {line}")
        return 1
    print("\nToutes les valeurs comparées correspondent à la source.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
