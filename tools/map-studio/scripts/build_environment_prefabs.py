#!/usr/bin/env python3
"""Extract curated Emerald environmental matrices from ORIGINAL map.bin layouts.

The manifest stores source coordinates, not copied artwork or guessed ID sequences.
Every build re-extracts native packed metatiles, collision and elevation, checks
the required source behavior, and checks that the exact source-pair atlas exists.
The resulting JSON is consumed by both Rust and the thumbnail generator.
"""
import json
import re
import struct
import sys
from pathlib import Path

ROOT = Path(sys.argv[1])
ATLAS_DIR = Path(sys.argv[2])
OUTPUT = Path(sys.argv[3])
SOURCE = ROOT / "tools/map-studio/assets/emerald-environment-regions.json"
spec = json.loads(SOURCE.read_text())
assert spec["format"] == "mapstudio-native-environment-regions" and spec["version"] == 1
catalog = json.loads((ATLAS_DIR / "catalog.json").read_text())
available = {entry["key"] for entry in catalog["pairs"]}
layouts = {
    entry["blockdata_filepath"].split("/")[-2]: entry
    for entry in json.loads((ROOT / "data/layouts/layouts.json").read_text())["layouts"]
    if entry["layout_version"] == "emerald"
}
behavior_lines = (ROOT / "include/constants/metatile_behaviors.h").read_text().splitlines()
behaviors = []
for line in behavior_lines:
    matched = re.match(r"^\s*(MB_[A-Z0-9_]+)\s*,", line)
    if matched:
        behaviors.append(matched.group(1))
assert len(behaviors) > 150

def symbol_key(symbol):
    return symbol.removeprefix("gTileset_").lower()

def attributes(symbol, role):
    path = ROOT / "data/tilesets" / role / symbol_key(symbol) / "metatile_attributes.bin"
    return path.read_bytes()

def metatile_count(symbol, role):
    path = ROOT / "data/tilesets" / role / symbol_key(symbol) / "metatiles.bin"
    return min(len(path.read_bytes()) // 16, 512)

def native_behavior(tile_id, primary, secondary):
    if tile_id < 512:
        raw = primary[tile_id * 2]
    else:
        raw = secondary[(tile_id - 512) * 2]
    return behaviors[raw] if raw < len(behaviors) else "UNKNOWN"

seen = set()
results = []
for item in spec["regions"]:
    asset_id = item["asset_id"]
    assert asset_id.startswith("emerald:environment:") and asset_id not in seen
    seen.add(asset_id)
    assert item["semantic_class"] in ("background", "macro", "micro")
    name = item["map"]
    layout = layouts[name]
    width, height = layout["width"], layout["height"]
    x, y, w, h = (item[k] for k in ("x", "y", "width", "height"))
    assert all(isinstance(v, int) for v in (x, y, w, h))
    assert 0 <= x and 0 <= y and 1 <= w <= 16 and 1 <= h <= 16
    assert x + w <= width and y + h <= height
    pair_key = f"{symbol_key(layout['primary_tileset'])}/{symbol_key(layout['secondary_tileset'])}"
    assert pair_key in available, f"{asset_id}: missing native source pair {pair_key}"
    raw = (ROOT / layout["blockdata_filepath"]).read_bytes()
    assert len(raw) == width * height * 2, f"{asset_id}: layout binary size differs"
    primary = attributes(layout["primary_tileset"], "primary")
    secondary = attributes(layout["secondary_tileset"], "secondary")
    count_p = metatile_count(layout["primary_tileset"], "primary")
    count_s = metatile_count(layout["secondary_tileset"], "secondary")
    cells = []
    hits = 0
    for yy in range(h):
        for xx in range(w):
            (packed,) = struct.unpack_from("<H", raw, ((y + yy) * width + x + xx) * 2)
            tile_id = packed & 0x03FF
            collision = (packed >> 10) & 3
            elevation = (packed >> 12) & 15
            local = tile_id if tile_id < 512 else tile_id - 512
            count = count_p if tile_id < 512 else count_s
            attrs = primary if tile_id < 512 else secondary
            assert local < count and (local + 1) * 2 <= len(attrs), (
                f"{asset_id}: source tile {tile_id} not defined in {pair_key}"
            )
            evidence = item["evidence"]
            if evidence.startswith("metatile:"):
                if tile_id == int(evidence.split(":", 1)[1], 0):
                    hits += 1
            elif native_behavior(tile_id, primary, secondary) == evidence:
                hits += 1
            cells.append(dict(x=xx, y=yy, metatile_id=tile_id,
                              collision=collision, elevation=elevation))
    assert hits >= item["min_count"], (
        f"{asset_id}: expected {item['evidence']} >= {item['min_count']}, got {hits}"
    )
    results.append({
        "name": item["name"], "width": w, "height": h,
        "primary_tileset": layout["primary_tileset"],
        "secondary_tileset": layout["secondary_tileset"],
        "asset_id": asset_id,
        "semantic_class": item["semantic_class"],
        "source_map": f"{name} ({x},{y}) [{pair_key}]",
        "metatiles": cells,
    })
assert len(results) == len(spec["regions"]) and len(results) >= 10
OUTPUT.parent.mkdir(parents=True, exist_ok=True)
OUTPUT.write_text(json.dumps(results, indent=2) + "\n")
print(f"Verified {len(results)} native Emerald environmental matrices from real map binaries")
