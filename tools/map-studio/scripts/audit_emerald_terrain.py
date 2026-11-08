#!/usr/bin/env python3
"""Audit *real* Lilycove native terrain, not guessed road/shore metatile IDs.

Outputs a source-derived town layout, frequencies and neighbor observations.
These are evidence for a future reviewed Wang/terrain rule set; frequency by
itself never certifies a tile as a pavement or shoreline.
"""
from collections import Counter
from pathlib import Path
import json
import struct
import sys
from PIL import Image

root=Path(sys.argv[1])
atlas=Path(sys.argv[2])
out=Path(sys.argv[3])
layouts=json.loads((root/"data/layouts/layouts.json").read_text())["layouts"]
layout=next(row for row in layouts if row["id"]=="LAYOUT_LILYCOVE_CITY")
assert layout["primary_tileset"]=="gTileset_General"
assert layout["secondary_tileset"]=="gTileset_Lilycove"
width,height=layout["width"],layout["height"]
raw=(root/layout["blockdata_filepath"]).read_bytes()
assert len(raw)==width*height*2,(len(raw),width,height)
packed=struct.unpack(f"<{width*height}H",raw)
ids=[value&0x3ff for value in packed]
source=Image.open(atlas).convert("RGBA")
assert source.size==(256,1024)
target=Image.new("RGBA",(width*16,height*16),(218,230,212,255))
freq=Counter(ids)
neighbors={}
for y in range(height):
    for x in range(width):
        idx=y*width+x
        ident=ids[idx]
        px=(ident%16)*16
        py=(ident//16)*16
        target.alpha_composite(source.crop((px,py,px+16,py+16)),(x*16,y*16))
        near=neighbors.setdefault(ident,{"north":Counter(),"east":Counter(),"south":Counter(),"west":Counter()})
        for dx,dy,dir in ((0,-1,"north"),(1,0,"east"),(0,1,"south"),(-1,0,"west")):
            xx,yy=x+dx,y+dy
            if 0<=xx<width and 0<=yy<height:
                near[dir][ids[yy*width+xx]]+=1
out.mkdir(parents=True,exist_ok=True)
target.save(out/"lilycove-source-layout.png",optimize=True)
rows=[{"metatile_id":ident,"count":count,
    "north":neighbors[ident]["north"].most_common(8),
    "east":neighbors[ident]["east"].most_common(8),
    "south":neighbors[ident]["south"].most_common(8),
    "west":neighbors[ident]["west"].most_common(8)}
    for ident,count in freq.most_common()]
(out/"lilycove-neighbors.json").write_text(json.dumps({
    "source_layout":layout["id"],"tileset":"general/lilycove",
    "width":width,"height":height,"unique_metatiles":len(freq),
    "metatiles":rows},indent=2))
assert len(freq)>30,"Lilycove source layout unexpectedly homogeneous"
print("Audited native Lilycove",width,height,"unique IDs",len(freq))
