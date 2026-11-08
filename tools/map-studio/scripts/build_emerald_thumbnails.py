#!/usr/bin/env python3
"""Emerald-only prefab thumbnails generated from real source-pair atlases.

A 96x96 preview for each Porymap prefab, indexed by its original JSON order.
No borrowed art, invented pixels, or different-source palette substitution.
All unavailable assets get an empty alpha slot, clearly disabled in the UI.
"""
import json
import re
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(sys.argv[1])
ATLAS_DIR = Path(sys.argv[2])
OUTPUT = Path(sys.argv[3])
THUMB = 96
COLS = 8
ORIGINAL = json.loads((ROOT / "tools/map-studio/assets/porymap-emerald-prefabs.json").read_text())
assert len(ORIGINAL) == 71
ENVIRONMENT = json.loads(Path(sys.argv[4]).read_text())
SOURCE = ORIGINAL + ENVIRONMENT
CATALOG = json.loads((ATLAS_DIR / "catalog.json").read_text())["pairs"]
available = {item["key"]: item for item in CATALOG}


def normalized(symbol):
    value = symbol.removeprefix("gTileset_")
    return re.sub(r"[^a-z0-9]", "", value.lower())


def matching_pair(prefab):
    pri = normalized(prefab.get("primary_tileset", ""))
    sec = normalized(prefab.get("secondary_tileset", ""))
    candidates = []
    for key in sorted(available):
        primary, secondary = key.split("/", 1)
        if pri and normalized(primary) != pri:
            continue
        if sec and normalized(secondary) != sec:
            continue
        candidates.append(key)
    if not candidates:
        return None
    # Prefer the actual General tileset for source wildcard prefabs.
    return next((key for key in candidates if key.startswith("general/")), candidates[0])


output = Image.new("RGBA", (COLS * THUMB, ((len(SOURCE)+COLS-1)//COLS)*THUMB), (0,0,0,0))
cache = {}
entries = []
for index, prefab in enumerate(SOURCE):
    key = matching_pair(prefab)
    ready = key is not None
    if ready:
        atlas_path = ATLAS_DIR / (key.replace("/", "__") + ".png")
        if key not in cache:
            cache[key] = Image.open(atlas_path).convert("RGBA")
        atlas = cache[key]
        pw,ph = prefab["width"],prefab["height"]
        art=Image.new("RGBA",(pw*16,ph*16),(0,0,0,0))
        for tile in prefab["metatiles"]:
            tid=tile["metatile_id"]
            if not (0<=tid<1024) or not (0<=tile["x"]<pw and 0<=tile["y"]<ph):
                ready=False
                break
            source_box=((tid%16)*16,(tid//16)*16,((tid%16)+1)*16,((tid//16)+1)*16)
            art.alpha_composite(atlas.crop(source_box),(tile["x"]*16,tile["y"]*16))
        if ready:
            factor=min((THUMB-8)/art.width,(THUMB-8)/art.height,1.5)
            w=max(1,round(art.width*factor))
            h=max(1,round(art.height*factor))
            art=art.resize((w,h),Image.Resampling.NEAREST)
            x=(index%COLS)*THUMB+(THUMB-w)//2
            y=(index//COLS)*THUMB+(THUMB-h)//2
            output.alpha_composite(art,(x,y))
    entries.append({"index":index,"name":prefab["name"],"asset_id":prefab.get("asset_id",f"emerald:porymap:{index:03d}"),"pair_key":key,
                    "preview_ready":bool(ready)})
OUTPUT.parent.mkdir(parents=True,exist_ok=True)
output.save(OUTPUT,optimize=True)
manifest=OUTPUT.with_suffix(".json")
manifest.write_text(json.dumps({"source":"Porymap Emerald","entries":entries},indent=2))
if len(entries) != len(ORIGINAL) + len(ENVIRONMENT):
    raise SystemExit("Missing source-backed environmental thumbnails")
if len(ENVIRONMENT) < 10:
    raise SystemExit("Expected native environmental source recipes")
print(f"Built native Emerald prefab thumbnails: {sum(x['preview_ready'] for x in entries)}/{len(entries)} "
      f"(71 upstream + {len(ENVIRONMENT)} native environment regions)")
