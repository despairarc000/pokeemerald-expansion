#!/usr/bin/env python3
"""Build the *actual* primary/secondary atlas pairs referenced by layouts.json.

No generated placeholder tiles, arbitrary remapping or RGB approximations:
each 16x16 metatile is composed from the indexed 8x8 tiles, palette banks,
flip bits and two metatile layers in this repository. Unknown/incomplete
tilesets are reported and excluded, not silently substituted.
"""
import json
import re
import struct
import sys
from pathlib import Path
from PIL import Image

root=Path(sys.argv[1]).resolve()
out=Path(sys.argv[2]).resolve()
out.mkdir(parents=True,exist_ok=True)
tile_root=root/"data/tilesets"
source=json.loads((root/"data/layouts/layouts.json").read_text())
def norm(name):
    return re.sub(r"[^a-z0-9]","",name.removeprefix("gTileset_").lower())
def index_dirs(kind):
    dirs=list((tile_root/kind).iterdir())
    return {norm(d.name):d for d in dirs if d.is_dir()}
primary=index_dirs("primary")
secondary=index_dirs("secondary")
def lookup(symbol,files):
    n=norm(symbol)
    if n in files:return files[n]
    # Some FRLG source symbols omit _Frlg while the corresponding asset
    # directory includes the suffix. Only infer unambiguous names.
    options=[d for k,d in files.items() if k.removesuffix("frlg")==n]
    return options[0] if len(options)==1 else None
pairs=sorted({(e["primary_tileset"],e["secondary_tileset"])
             for e in source["layouts"]})
# Only labels actually declared by the source count as named object hints.
# A name may describe one metatile, never a complete building or prefab.
labels_by_source={}
active_symbol=None
for line in (root/"include/constants/metatile_labels.h").read_text().splitlines():
    section=re.match(r"\s*//\s*(gTileset_[A-Za-z0-9_]+)",line)
    if section:
        active_symbol=section.group(1)
        labels_by_source.setdefault(active_symbol,{})
        continue
    define=re.match(r"\s*#define\s+(METATILE_[A-Za-z0-9_]+)\s+(0x[0-9A-Fa-f]+|[0-9]+)\b",line)
    if define and active_symbol:
        name,value=define.groups()
        if name.startswith("METATILE_"+active_symbol.removeprefix("gTileset_")+"_"):
            labels_by_source[active_symbol][int(value,0)]=name

def read_palette(path):
    rows=path.read_text("ascii").splitlines()
    if rows[:3]!=["JASC-PAL","0100","16"]:
        raise ValueError("Unrecognized palette "+str(path))
    return [tuple(map(int,row.split())) for row in rows[3:19]]
def colored(im,colors):
    cp=Image.frombytes("P",im.size,im.tobytes()) if im.mode=="L" else im.copy()
    colors=colors+[(0,0,0)]*(256-len(colors))
    cp.putpalette([channel for rgb in colors for channel in rgb])
    return cp

def opaque_mask(indexed):
    # A P-mode image carries an RGB palette; convert("L") after point()
    # would re-interpret the result through that palette and erase artwork.
    # Read the literal 4bpp indices as L values *before* building the mask.
    indexes=Image.frombytes("L",indexed.size,indexed.tobytes())
    return indexes.point(lambda value:0 if value==0 else 255)

def generate(main,local,output):
    palettes={}
    for bank in range(16):
        base=main if bank<6 else local
        fn=base/"palettes"/f"{bank:02}.pal"
        if fn.is_file():
            palettes[bank]=read_palette(fn)
    images=[Image.open(d/"tiles.png") for d in (main,local)]
    for img in images:
        if img.mode not in ("P","L") or img.width%8 or img.height%8 or max(img.getdata())>15:
            raise ValueError("Invalid indexed 8x8 tiles "+str(img))
    tile_banks=[{bank:colored(img,pal).convert("RGBA")
                  for bank,pal in palettes.items()} for img in images]
    raw=[(d/"metatiles.bin").read_bytes() for d in (main,local)]
    attrs=[(d/"metatile_attributes.bin").read_bytes() for d in (main,local)]
    count1=min(len(raw[0])//16,512)
    count2=min(len(raw[1])//16,512)
    atlas=Image.new("RGBA",(16*16,64*16),(0,0,0,0))
    cache={}
    valid=[]
    for mid in list(range(count1))+list(range(512,512+count2)):
        src=0 if mid<512 else 1
        sub=mid if src==0 else mid-512
        entries=struct.unpack_from("<8H",raw[src],sub*16)
        # A metatile is only paintable if every referenced graphical
        # subtile exists in its source image and all needed palettes exist.
        source_ok=True
        for entry in entries:
            tile=entry&1023
            bank=(entry>>12)&15
            image_src=0 if tile<512 else 1
            idx=tile if image_src==0 else tile-512
            image=images[image_src]
            if bank not in palettes or idx >= (image.width//8)*(image.height//8):
                source_ok=False
                break
        if not source_ok or (sub+1)*2>len(attrs[src]):
            continue
        valid.append(mid)
        x=(mid%16)*16;y=(mid//16)*16
        # Compose a static approximation of the GBA's three BG layers.
        # All indexed zeros are transparent, but the GBA has an opaque
        # backdrop (BG palette 0, entry 0). NORMAL metatiles also draw
        # primary tile 0x14, palette 3, on BG3 behind the other layers.
        # This prevents apparently missing scenery in the picker without
        # treating arbitrary palette-bank zeros as painted pixels.
        if 0 not in palettes or 3 not in palettes:
            raise ValueError("Primary backdrop palette missing")
        atlas.paste((*palettes[0][0],255),(x,y,x+16,y+16))
        layer_type=(struct.unpack_from("<H",attrs[src],sub*2)[0]>>12)&15
        if layer_type==0:
            filler_index=0x14
            filler_img=tile_banks[0][3]
            tile_x=(filler_index%(filler_img.width//8))*8
            tile_y=(filler_index//(filler_img.width//8))*8
            filler=filler_img.crop((tile_x,tile_y,tile_x+8,tile_y+8))
            indexed=images[0].crop((tile_x,tile_y,tile_x+8,tile_y+8))
            filler.putalpha(opaque_mask(indexed))
            for fy in (0,8):
                for fx in (0,8):
                    atlas.alpha_composite(filler,(x+fx,y+fy))
        for n,entry in enumerate(entries):
            tile=entry&1023
            flipx=bool(entry&0x400)
            flipy=bool(entry&0x800)
            bank=(entry>>12)&15
            image_src=0 if tile<512 else 1
            tile_index=tile if image_src==0 else tile-512
            image=tile_banks[image_src][bank]
            key=(tile,bank,flipx,flipy,n>=4)
            if key not in cache:
                tile_x=(tile_index%(image.width//8))*8
                tile_y=(tile_index//(image.width//8))*8
                cell=image.crop((tile_x,tile_y,tile_x+8,tile_y+8))
                if flipx:cell=cell.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
                if flipy:cell=cell.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
                # Palette index 0 is transparent on *every* GBA BG layer.
                # Only masking the upper layer paints opaque palette-zero
                # rectangles where the base layer should show through.
                idx_img=images[image_src].crop((tile_x,tile_y,tile_x+8,tile_y+8))
                if flipx:idx_img=idx_img.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
                if flipy:idx_img=idx_img.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
                mask=opaque_mask(idx_img)
                cell.putalpha(mask)
                cache[key]=cell
            picture=cache[key]
            dest=(x+(n%2)*8,y+((n//2)%2)*8)
            atlas.alpha_composite(picture,dest)
    atlas.save(output,optimize=True)
    mask=bytearray(128)
    for mid in valid:mask[mid//8]|=1<<(mid%8)
    return count1,count2,bytes(mask),len(valid)

def group_for(primary_name,secondary_name):
    n=secondary_name.lower()
    if "cave" in n or any(v in n for v in ("tunnel","falls","pyramid","ruins","rock","underwater","tower")):
        return "Caves"
    if "house" in n or "building" in n:
        return "Houses"
    if "building" in primary_name or any(v in n for v in ("shop","center","gym","lab","school","mart","hospital","museum","hotel","store","club")):
        return "Interiors"
    if any(v in n for v in ("town","city","petalburg","rustboro","mauville","slateport","lilycove","fortree","sootopolis","dewford","fallarbor","lavaridge","mossdeep","pacifidlog","vermilion","fuchsia","cerulean","saffron","pallet","pewter","viridian","lavender","cinnabar")):
        return "Towns"
    return "Outdoor & Special"

catalog=[]
omitted=[]
for main_sym,sec_sym in pairs:
    main=lookup(main_sym,primary)
    local=lookup(sec_sym,secondary)
    if not main or not local:
        omitted.append((main_sym,sec_sym,"tileset folder missing"))
        continue
    if not all((d/f).is_file() for d in (main,local)
               for f in ("metatiles.bin","metatile_attributes.bin","tiles.png")):
        omitted.append((main_sym,sec_sym,"one or more real graphics/attribute files missing"))
        continue
    key=f"{main.name}/{local.name}"
    output=out/(main.name+"__"+local.name+".png")
    try:
        first,second,mask,usable=generate(main,local,output)
        if usable<40:
            raise ValueError("Fewer than 40 valid metatiles in paired source data")
    except (OSError,ValueError,struct.error) as exc:
        omitted.append((main_sym,sec_sym,str(exc)))
        continue
    catalog.append((key,main,local,output,first,second,group_for(main.name,local.name),
                    main_sym,sec_sym,mask,usable))
if not any(c[0]=="general/petalburg" for c in catalog):
    for p,s,reason in omitted:
        if p=="gTileset_General" and s=="gTileset_Petalburg":
            print("DEFAULT SOURCE FAILURE:",reason,flush=True)
    raise RuntimeError("Default General/Petalburg atlas failed to build")
if len(catalog)<60:
    raise RuntimeError("Too few actual Emerald tileset pairs: "+str(len(catalog)))
catalog.sort(key=lambda e:(e[6],e[2].name,e[1].name))
def literal(s):
    return json.dumps(str(s),ensure_ascii=True)
sourcefile=out.parent/"tileset_catalog.rs"
rows=[]
for key,main,local,img,first,second,category,main_sym,sec_sym,mask,usable in catalog:
    label=local.name.replace("_"," ").title()
    collection=("FireRed/LeafGreen" if main_sym in ("gTileset_General_Frlg","gTileset_BuildingFrlg") else "Emerald")
    labels=[]
    for mid,source_label in labels_by_source.get(main_sym,{}).items():
        if 0<=mid<512 and mask[mid//8]&(1<<(mid%8)):
            labels.append((mid,source_label))
    for mid,source_label in labels_by_source.get(sec_sym,{}).items():
        if 512<=mid<1024 and mask[mid//8]&(1<<(mid%8)):
            labels.append((mid,label))
    labels.sort()
    label_literals=",".join("("+str(mid)+","+literal(label)+")" for mid,label in labels)
    assets=[]
    for kind,folder in (("primary",main),("secondary",local)):
        for entry in sorted((folder/"palettes").glob("*.pal")):
            assets.append("("+literal(f"{kind}/palettes/{entry.name}")+
                ",include_bytes!("+literal(entry)+"))")
    rows.append(
        "TilePair {key:"+literal(key)+", label:"+literal(label)+
        ",collection:"+literal(collection)+",category:"+literal(category)+
        ",primary_symbol:"+literal(main_sym)+",secondary_symbol:"+literal(sec_sym)+
        ",primary:"+literal(main.name)+
        ",secondary:"+literal(local.name)+",primary_count:"+str(first)+
        ",secondary_count:"+str(second)+
        ",valid_mask:&["+(",".join(str(b) for b in mask))+"]"+
        ",atlas:include_bytes!("+literal(img)+")"+
        ",labels:&["+label_literals+"]"+
        ",attr_primary:include_bytes!("+literal(main/"metatile_attributes.bin")+")"+
        ",attr_secondary:include_bytes!("+literal(local/"metatile_attributes.bin")+")"+
        ",raw_primary_tiles:include_bytes!("+literal(main/"tiles.png")+")"+
        ",raw_secondary_tiles:include_bytes!("+literal(local/"tiles.png")+")"+
        ",raw_primary_metatiles:include_bytes!("+literal(main/"metatiles.bin")+")"+
        ",raw_secondary_metatiles:include_bytes!("+literal(local/"metatiles.bin")+")"+
        ",raw_files:&["+(",".join(assets))+"]},")
sourcefile.write_text("pub static TILESET_PAIRS:&[TilePair]=&[\n"+"\n".join(rows)+"\n];\n")
manifest={"available":len(catalog),"omitted":[{"primary":p,"secondary":s,"reason":reason} for p,s,reason in omitted],
    "pairs":[{"key":key,"category":category,"tiles":first+second,"usable":usable}
             for key,_,_,_,first,second,category,_,_,mask,usable in catalog]}
(out/"catalog.json").write_text(json.dumps(manifest,indent=2))
print(f"Generated {len(catalog)} real tile-pair atlases; omitted {len(omitted)} incomplete or unsupported pairs")
for p,s,reason in omitted[:16]:print("SKIPPED",p,s,reason[:140])


# Built-in Ruby source packs. These are genuinely different Ruby graphics and
# source IDs from pret/pokeruby, not an Emerald clone or empty menu entries.
# The playable game engine currently targets pokeemerald; therefore Ruby packs
# stay browse-only until an explicit native tileset integration is tested.
def build_ruby_builtins():
    import zipfile
    root_assets = root / "tools/map-studio/assets"
    ruby = root_assets / "builtin-sources/ruby"
    main = ruby / "primary/general"
    prefabs = json.loads((root_assets / "porymap-ruby-prefabs.json").read_text())
    assert len(prefabs) == 55, "Porymap Ruby built-in definitions changed"
    output = out.parent / "builtin_packs"
    output.mkdir(exist_ok=True)
    for secondary_name, symbol, label in (
        ("petalburg", "gTileset_Petalburg", "Petalburg"),
        ("mauville", "gTileset_Mauville", "Mauville"),
        ("rustboro", "gTileset_Rustboro", "Rustboro"),
    ):
        local = ruby / "secondary" / secondary_name
        atlas = output / f"ruby_{secondary_name}.png"
        primary_count, secondary_count, mask, valid_count = generate(main, local, atlas)
        if valid_count < 40:
            raise ValueError(f"Ruby {secondary_name}: only {valid_count} renderable tiles")
        valid = lambda mid: 0 <= mid < 1024 and bool(mask[mid//8] & (1 << (mid % 8)))
        compatible = [
            p for p in prefabs
            if p.get("primary_tileset", "") in ("", "gTileset_General")
            and p.get("secondary_tileset", "") in ("", symbol)
            and all(valid(tile["metatile_id"]) for tile in p["metatiles"])
        ]
        manifest = {
            "format": "mapstudio-artwork-pack", "version": 1,
            "collection": "Ruby", "engine": "pokeruby",
            "source": "pret/pokeruby native source artwork; Porymap default Ruby prefab data",
            "attribution": "Original Pokemon Ruby game artwork; Porymap prefab definitions LGPL-3.0, huderlem/porymap. Pokemon imagery retains original rights.",
            "label": label, "category": "Towns", "pair_key": f"ruby/general/{secondary_name}",
            "primary_symbol": "gTileset_General", "secondary_symbol": symbol,
            "primary_count": primary_count, "secondary_count": secondary_count,
            "valid_mask": list(mask),
        }
        package = output / f"ruby_{secondary_name}.zip"
        with zipfile.ZipFile(package, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("manifest.json", json.dumps(manifest, ensure_ascii=True))
            z.writestr("prefabs.json", json.dumps(compatible, ensure_ascii=True))
            z.write(atlas, "atlas.png")
            for kind, folder in (("primary", main), ("secondary", local)):
                for filename in ("tiles.png", "metatiles.bin", "metatile_attributes.bin"):
                    z.write(folder / filename, f"{kind}/{filename}")
                for pal in sorted((folder / "palettes").glob("*.pal")):
                    z.write(pal, f"{kind}/palettes/{pal.name}")
            z.write(root_assets / "PORYMAP-LICENSE.md", "LICENSE-PORYMAP.md")
            z.write(root_assets / "PORYMAP-ATTRIBUTION.md", "ATTRIBUTION-PORYMAP.md")
        assert len(zipfile.ZipFile(package).namelist()) <= 90
        print(f"Ruby built-in {label}: {valid_count} metatiles and {len(compatible)} compatible prefabs")

build_ruby_builtins()
