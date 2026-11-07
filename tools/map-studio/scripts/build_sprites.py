#!/usr/bin/env python3
"""Build accurate previews for the editor's supported NPC choices.

Resolve sprite, animation frame dimensions, and palette via Emerald's actual
C graphic tables. Unresolved choices do not produce invented sprite art.
"""
from pathlib import Path
import re
import sys
from PIL import Image

root=Path(sys.argv[1]).resolve()
out=Path(sys.argv[2]).resolve()
out.mkdir(parents=True,exist_ok=True)
def read(path):
    return (root/path).read_text(encoding="utf-8")
choices=read("tools/map-studio/src/choices.rs")
pointers=read("src/data/object_events/object_event_graphics_info_pointers.h")
information=read("src/data/object_events/object_event_graphics_info.h")
tables=read("src/data/object_events/object_event_pic_tables.h")
gfx=read("src/data/object_events/object_event_graphics.h")
symbols=re.findall(r'\("([^"]+)",\s*"(OBJ_EVENT_GFX_[A-Z0-9_]+)"\)',choices)
symbols=symbols[:25]
if len(symbols)<15:
    raise RuntimeError("Expected source-verified NPC options")
cols=8
cellw,cellh=24,40
atlas=Image.new("RGBA",(cols*cellw,((len(symbols)+cols-1)//cols)*cellh),(0,0,0,0))
resolved=[]
for _,symbol in symbols:
    ptr=re.search(r"\["+re.escape(symbol)+r"\]\s*=\s*&gObjectEventGraphicsInfo_(\w+)",pointers)
    if not ptr:continue
    key=ptr.group(1)
    block=re.search(r"const struct ObjectEventGraphicsInfo gObjectEventGraphicsInfo_"+
                    re.escape(key)+r"\s*=\s*\{(.*?)\};",information,re.S)
    if not block:continue
    pal_tag=re.search(r"\.paletteTag\s*=\s*OBJ_EVENT_PAL_TAG_([A-Z0-9_]+)",block.group(1))
    tab_name=re.search(r"\.images\s*=\s*sPicTable_(\w+)",block.group(1))
    if not pal_tag or not tab_name:continue
    table=re.search(r"sPicTable_"+re.escape(tab_name.group(1))+r"\[\]\s*=\s*\{(.*?)\};",tables,re.S)
    if not table:continue
    pic_symbol=re.search(r"gObjectEventPic_(\w+)",table.group(1))
    if not pic_symbol:continue
    path_ref=re.search(r"gObjectEventPic_"+re.escape(pic_symbol.group(1))+
                       r"\[\][^\n]*",gfx)
    if not path_ref:continue
    image_path=re.search(r'"([^"]+\.png)"',path_ref.group(0))
    if not image_path:continue
    src=root/image_path.group(1)
    pal=root/"graphics/object_events/palettes"/(pal_tag.group(1).lower()+".pal")
    if not src.is_file() or not pal.is_file():continue
    lines=pal.read_text("ascii").splitlines()
    if lines[:3]!=["JASC-PAL","0100","16"]:continue
    colors=[tuple(map(int,ln.split())) for ln in lines[3:19]]
    source=Image.open(src)
    if source.mode not in ("L","P"):continue
    if source.mode=="L":
        image=Image.frombytes("P",source.size,source.tobytes())
    else:image=source.copy()
    table_dim=re.search(r"overworld_\w+_frames?\(\s*gObjectEventPic_\w+\s*,\s*(\d+),\s*(\d+)",table.group(1))
    framew=min(source.width,8*int(table_dim.group(1)) if table_dim else 16)
    frameh=min(source.height,8*int(table_dim.group(2)) if table_dim else 32)
    if framew<=0 or frameh<=0:continue
    image=image.crop((0,0,framew,frameh))
    colors+= [(0,0,0)]*(256-len(colors))
    image.putpalette([v for trip in colors for v in trip])
    # Source indexed color 0 is transparent for overworld sprites.
    image.info["transparency"]=0
    frame=image.convert("RGBA")
    if frame.width>20 or frame.height>36:
        # Explicitly scale only exceptionally large character graphics for UI.
        frame.thumbnail((20,36),Image.Resampling.NEAREST)
    idx=len(resolved)
    x=(idx%cols)*cellw+(cellw-frame.width)//2
    y=(idx//cols)*cellh+cellh-2-frame.height
    atlas.paste(frame,(x,y),frame)
    resolved.append((symbol,idx))
if len(resolved)<15:
    raise RuntimeError(f"Only {len(resolved)} NPCs resolved from source art")
image_file=out/"npc-previews.png"
atlas=atlas.crop((0,0,cols*cellw,((len(resolved)+cols-1)//cols)*cellh))
atlas.save(image_file,optimize=True)
rs_file=out.parent/"npc_sprite_catalog.rs"
refs=",\n".join(f'({symbol!r},{idx})'.replace("'",'"') for symbol,idx in resolved)
rs_file.write_text(
    'pub const NPC_ATLAS_PNG:&[u8]=include_bytes!('+repr(str(image_file)).replace("'",'"')+');\n'
    'pub const NPC_PREVIEW_COUNT:usize='+str(len(resolved))+';\n'
    'pub const NPC_PREVIEW_REFS:&[(&str,usize)]=&['+refs+'];\n'
)
print(f"Generated {len(resolved)} source-verified NPC sprite previews")
