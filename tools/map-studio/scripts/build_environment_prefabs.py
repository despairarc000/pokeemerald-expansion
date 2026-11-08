#!/usr/bin/env python3
"""Extract curated Emerald environmental matrices from ORIGINAL map.bin layouts.

The manifest names source maps, intended regions and native evidence. The
preferred coordinates are tried first; if that location does not contain the
verified feature, the closest matching native region is selected deterministically.
Every build re-extracts native packed metatiles, collision and elevation, checks
the required source behavior, and checks that the exact source-pair atlas exists.
The resulting JSON is consumed by both Rust and the thumbnail generator.
"""
import hashlib
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

def find_region(item, raw, width, height, primary, secondary,
                count_p, count_s):
    """Find a valid 2D source map patch, not an invented feature.
    The origin in the manifest is a preferred anchor, never proof of identity.
    Reject candidates without the recorded native behavior or named metatile.
    """
    x, y, w, h = (item[k] for k in ("x", "y", "width", "height"))
    candidates = []
    for yy in range(height - h + 1):
        for xx in range(width - w + 1):
            cells = []
            hits = 0
            normals = 0
            usable = True
            for dy in range(h):
                for dx in range(w):
                    (packed,) = struct.unpack_from("<H", raw,
                        ((yy + dy) * width + xx + dx) * 2)
                    tile_id = packed & 0x03FF
                    local = tile_id if tile_id < 512 else tile_id - 512
                    count = count_p if tile_id < 512 else count_s
                    attrs = primary if tile_id < 512 else secondary
                    if local >= count or (local + 1) * 2 > len(attrs):
                        usable = False
                        break
                    behavior = native_behavior(tile_id, primary, secondary)
                    if behavior == "MB_NORMAL":
                        normals += 1
                    evidence = item["evidence"]
                    if (evidence.startswith("metatile:")
                        and tile_id == int(evidence.split(":",1)[1], 0)) or (
                        not evidence.startswith("metatile:") and behavior == evidence):
                        hits += 1
                    cells.append(dict(x=dx, y=dy, metatile_id=tile_id,
                        collision=(packed >> 10) & 3,
                        elevation=(packed >> 12) & 15))
                if not usable:
                    break
            if not usable or hits < item["min_count"]:
                continue
            # Real pond/coast/grass boundaries must contain neighbors, not
            # just an opaque slab of a single feature.
            evidence = item["evidence"]
            if (("WATER" in evidence or "GRASS" in evidence)
                and hits > w*h-3):
                continue
            if "WATER" in evidence and normals < 2:
                continue
            if ("TALL_GRASS" in evidence or "MOUNTAIN" in evidence) and normals < 2:
                continue
            priority = (abs(xx-x)+abs(yy-y),abs(hits-(w*h//3)),
                        -normals,yy,xx)
            candidates.append((priority,xx,yy,cells,hits))
    if not candidates:
        raise AssertionError(
            f"{item['asset_id']}: no genuine native map region contains "
            f"{item['evidence']} (at least {item['min_count']} cells)")
    _,rx,ry,cells,hits=min(candidates,key=lambda found:found[0])
    return rx,ry,cells,hits

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
    rx,ry,cells,hits=find_region(item,raw,width,height,primary,secondary,
                                 count_p,count_s)
    results.append({
        "name": item["name"], "width": w, "height": h,
        "primary_tileset": layout["primary_tileset"],
        "secondary_tileset": layout["secondary_tileset"],
        "asset_id": asset_id,
        "semantic_class": item["semantic_class"],
        "source_map": f"{name} ({rx},{ry}) [{pair_key}] SHA:{hashlib.sha256(raw).hexdigest()[:10]}",
        "metatiles": cells,
    })
assert len(results) == len(spec["regions"]) and len(results) >= 10
OUTPUT.parent.mkdir(parents=True, exist_ok=True)
OUTPUT.write_text(json.dumps(results, indent=2) + "\n")
print(f"Verified {len(results)} native Emerald environmental matrices from real map binaries")
for p in results:
    print("  ",p["asset_id"],p["source_map"])
