#!/usr/bin/env python3
"""Compose actual source item icons (indexed pixels + item-specific palettes)."""
import json,sys
from pathlib import Path
from PIL import Image
root=Path(sys.argv[1])
out=Path(sys.argv[2])
out.mkdir(parents=True,exist_ok=True)
icons=root/"graphics/items/icons"
palettes=root/"graphics/items/icon_palettes"
# Shared artwork with item-specific palette is intentional in Emerald.
items=[
 ("ITEM_POKE_BALL","poke_ball","poke_ball"),
 ("ITEM_POTION","potion","potion"),
 ("ITEM_SUPER_POTION","potion","super_potion"),
 ("ITEM_ANTIDOTE","potion","antidote"),
 ("ITEM_FULL_HEAL","full_heal","full_heal"),
 ("ITEM_REVIVE","revive","revive"),
 ("ITEM_ESCAPE_ROPE","escape_rope","escape_rope"),
 ("ITEM_ETHER","ether","ether"),
 ("ITEM_RARE_CANDY","rare_candy","rare_candy"),
 ("ITEM_NUGGET","nugget","nugget"),
 ("ITEM_TINY_MUSHROOM","tiny_mushroom","mushroom"),
]
atlas=Image.new("RGBA",(8*32,2*32),(0,0,0,0))
for index,(symbol,art,palette) in enumerate(items):
    source=Image.open(icons/(art+".png"))
    if source.mode not in ("P","L") or max(source.getdata())>15:
        raise ValueError("Indexed source item icon required: "+art)
    colors=(palettes/(palette+".pal")).read_text().splitlines()
    if colors[:3]!=["JASC-PAL","0100","16"]:
        raise ValueError("Invalid source item palette: "+palette)
    rgb=[tuple(map(int,line.split())) for line in colors[3:19]]
    pixels=Image.frombytes("P",source.size,source.tobytes())
    pixels.putpalette([c for triplet in rgb for c in triplet]+[0]*(256*3-16*3))
    image=pixels.convert("RGBA")
    mask=Image.frombytes("L",source.size,source.tobytes()).point(
        lambda p: 0 if p==0 else 255)
    image.putalpha(mask)
    if image.width>32 or image.height>32: raise ValueError("Unexpected item size")
    x=(index%8)*32+(32-image.width)//2
    y=(index//8)*32+(32-image.height)//2
    atlas.alpha_composite(image,(x,y))
atlas.save(out/"item-previews.png",optimize=True)
source_code="pub const HIDDEN_ICON_REFS:&[(&str,usize)]=&[\n"+ "".join(
    "    ("+json.dumps(symbol)+","+str(i)+"),\n" for i,(symbol,_,_) in enumerate(items)) +"];\n"
(out/"item_icon_catalog.rs").write_text(source_code)
print("Verified",len(items),"source item icons")
