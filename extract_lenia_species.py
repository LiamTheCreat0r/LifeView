#!/usr/bin/env python3
"""Extract Lenia species from official animals.json using the correct RLE decoder."""
import json
import os
from fractions import Fraction

print("Loading animals.json...")
with open("/tmp/animals.json") as f:
    animals = json.loads(f.read())

print(f"Found {len(animals)} entries")

REQUESTED_GENERA = {
    "Orbium", "Gyrorbium", "Vagorbium",
    "Scutium", "Discutium", "Triscutium",
    "Flos", "Lacuna", "Spirillum", "Angula",
    "Caterpillar", "Platyhelminthes",
    "Asteria", "Medusa", "Anemone",
    "Hydrogeminium", "Tessellatium",
    "Synorbium", "Parorbium", "Trisynorbium",
    "Tetrasynorbium", "Triparorbium",
}

def ch2val(c):
    """Official Lenia ch2val decoder."""
    if c in '.b':
        return 0
    elif c == 'o':
        return 255
    elif len(c) == 1:
        return ord(c) - ord('A') + 1
    else:
        return (ord(c[0]) - ord('p')) * 24 + (ord(c[1]) - ord('A') + 25)

def rle2cells_2d(st):
    """Official Lenia RLE decoder for 2D grids."""
    st = st.rstrip('!') + '$'  # Append row delimiter
    rows = []
    current_row = []
    last = ''
    count = ''

    for ch in st:
        if ch.isdigit():
            count += ch
        elif ch in 'pqrstuvwxy@':
            last = ch
        elif ch == '$':
            # End of row - push current_row to rows
            rows.append(current_row)
            current_row = []
            last = ''
            count = ''
        else:
            # Cell value
            val = ch2val(last + ch) / 255.0
            repeat = int(count) if count else 1
            current_row.extend([val] * repeat)
            last = ''
            count = ''

    # Remove last empty row if present
    if rows and not rows[-1]:
        rows.pop()

    if not rows:
        return [[]]

    # Pad rows to same width
    max_w = max(len(r) for r in rows)
    for r in rows:
        while len(r) < max_w:
            r.append(0.0)

    return rows

def lenia_to_our_params(entry):
    p = entry.get("params", {})
    R = p.get("R", 13)
    T = p.get("T", 10)
    b_str = p.get("b", "1")
    m = p.get("m", 0.15)
    s = p.get("s", 0.015)
    kn = p.get("kn", 1)

    peaks = [float(Fraction(x.strip())) for x in b_str.split(",")]
    delta = 1.0 / T
    polynomial = (kn == 1)

    kernel = {
        "mu": m,
        "sigma": s,
        "base_radius": R,
        "relative_radius": 1.0,
        "height": 1.0,
        "peaks": peaks,
        "c0": 0,
        "c1": 0,
        "use_target": False,
        "sum_mode": False,
        "polynomial": polynomial,
        "alpha": 4.0,
    }

    rule = {
        "state_type": "continuous",
        "delta": delta,
        "num_channels": 1,
        "kernels": [kernel],
    }
    return rule

species_data = {}
current_class = ""
current_order = ""
current_family = ""
current_subfamily = ""

for entry in animals:
    code = entry.get("code", "")
    name = entry.get("name", "")

    if code.startswith(">"):
        if "class:" in name.lower():
            current_class = name.split(":")[1].strip()
        elif "order:" in name.lower():
            current_order = name.split(":")[1].strip()
        elif "family:" in name.lower():
            current_family = name.split(":")[1].strip()
        elif "subfamily:" in name.lower():
            current_subfamily = name.split(":")[1].strip()
        continue

    if "cells" not in entry:
        continue

    parts = name.split(" ", 1)
    genus = parts[0] if parts else "Unknown"

    if genus not in REQUESTED_GENERA:
        continue

    if genus not in species_data:
        species_data[genus] = []

    try:
        grid = rle2cells_2d(entry["cells"])
        if not grid or not grid[0]:
            print(f"  Empty grid for {name}")
            continue
    except Exception as e:
        print(f"  Failed to decode RLE for {name}: {e}")
        continue

    try:
        rule = lenia_to_our_params(entry)
    except Exception as e:
        print(f"  Failed to convert params for {name}: {e}")
        continue

    species_data[genus].append({
        "name": name,
        "code": code,
        "rule": rule,
        "grid": grid,
    })

print("\n=== Species found ===")
for genus in sorted(species_data.keys()):
    count = len(species_data[genus])
    print(f"  {genus}: {count} species")

OUT_DIR = "assets/shapes/patterns"
os.makedirs(OUT_DIR, exist_ok=True)

written = 0
for genus in sorted(species_data.keys()):
    for sp in species_data[genus]:
        safe_name = sp["name"].lower().replace(" ", "_").replace(".", "")
        filename = f"{OUT_DIR}/{safe_name}.json"

        shape = {
            "name": sp["name"],
            "rule": sp["rule"],
            "channels": [sp["grid"]],
        }

        with open(filename, "w") as f:
            json.dump(shape, f, indent=2)

        grid_h = len(sp["grid"])
        grid_w = len(sp["grid"][0]) if sp["grid"] else 0
        # Show value range
        all_vals = [v for row in sp["grid"] for v in row if v > 0]
        val_range = f"[{min(all_vals):.3f}, {max(all_vals):.3f}]" if all_vals else "empty"
        print(f"  Wrote {filename} ({grid_w}x{grid_h}) values={val_range}")
        written += 1

print(f"\nTotal: {written} shape files written to {OUT_DIR}")
