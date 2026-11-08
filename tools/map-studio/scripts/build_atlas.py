#!/usr/bin/env python3
"""Compile native Emerald primary-General + secondary-Petalburg metatiles into a
16x64 RGBA atlas. Real .bin subtile entries + palette data; no placeholder art.
Layering is approximate for metatile layer types that need special background
priority, which must be tested against in-game render before claiming parity.
"""
import sys
import struct
from pathlib import Path
from PIL import Image

ROOT=Path(sys.argv[1])
OUTPUT=Path(sys.argv[2])
PRIMARY=ROOT/"data/tilesets/primary/general"
SECONDARY=ROOT/"data/tilesets/secondary/petalburg"
COUNT=1024
COLS=16

def palette(path):
    lines=path.read_text(encoding="ascii").splitlines()
    if lines[:3]!=["JASC-PAL","0100","16"]:
        raise ValueError(f"Unsupported palette {path}")
    return [tuple(map(int,s.split())) for s in lines[3:19]]

palettes=[palette(PRIMARY/"palettes"/f"{i:02}.pal") if i<6
    else palette(SECONDARY/"palettes"/f"{i:02}.pal") for i in range(16)]
tilesets=[Image.open(p/"tiles.png") for p in [PRIMARY,SECONDARY]]
for im in tilesets:
    if im.mode not in ("P","L"):
        raise ValueError(f"Tiles must be indexed for exact 4bpp colors: {im.mode}")
    if im.width%8 or im.height%8:
        raise ValueError("Tileset image dimensions are not multiples of 8")
    if max(im.getdata())>15:
        raise ValueError("Tileset pixel index exceeds GBA 4bpp palette range")
raws=[(p/"metatiles.bin").read_bytes() for p in [PRIMARY,SECONDARY]]
atlas=Image.new("RGBA",(COLS*16, ((COUNT+COLS-1)//COLS)*16), (0,0,0,0))
pix=atlas.load()

def draw_subtile(subtile,piece,metax,metay,top):
    tileidx=subtile&0x3ff
    flipx=bool(subtile&0x400)
    flipy=bool(subtile&0x800)
    bank=subtile>>12
    im=tilesets[0] if tileidx<512 else tilesets[1]
    local=tileidx if tileidx<512 else tileidx-512
    tiles_w=im.width//8
    if local>=tiles_w*(im.height//8):
        return
    tx=(local%tiles_w)*8
    ty=(local//tiles_w)*8
    x0=metax+(piece%2)*8
    y0=metay+((piece//2)%2)*8
    for sy in range(8):
        for sx in range(8):
            color_index=im.getpixel((tx+(7-sx if flipx else sx),ty+(7-sy if flipy else sy)))
            # Index zero is transparent on all GBA background layers.
            if color_index==0:
                continue
            color=palettes[bank][color_index]
            pix[x0+sx,y0+sy]=(*color,255)

for metatile in range(COUNT):
    if metatile<512:
        bank=0; local=metatile
    else:
        bank=1; local=metatile-512
    raw=raws[bank]
    if (local+1)*16>len(raw):
        continue
    entries=struct.unpack_from("<8H",raw,local*16)
    x=(metatile%COLS)*16
    y=(metatile//COLS)*16
    # Source palette index zero is transparent on GBA background layers.
    # The previous opaque palette-zero fill created bright magenta patches.
    # Leave unset pixels transparent and draw only genuine source subtiles.
    attrs_path=(PRIMARY if metatile<512 else SECONDARY)/"metatile_attributes.bin"
    # Attributes stay in the checked-in source, parallel to metatiles.bin.
    attrs=attrs_path.read_bytes()
    if (local+1)*2<=len(attrs):
        layer=(struct.unpack_from("<H",attrs,local*2)[0]>>12)&15
        if layer==0:
            for corner in range(4):
                draw_subtile(0x3014,corner,x,y,False)
    for n,sub in enumerate(entries):
        draw_subtile(sub,n%4,x,y,n>=4)

OUTPUT.parent.mkdir(parents=True,exist_ok=True)
atlas.save(OUTPUT,optimize=True)
print(f"Generated actual Emerald atlas: {OUTPUT} ({OUTPUT.stat().st_size} bytes)")
