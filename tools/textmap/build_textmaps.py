#!/usr/bin/env python3
"""Build custom maps from text sources. Safe to run repeatedly (idempotent).

For each folder custom_maps/<MapName>/ this tool:
  * writes data/layouts/<MapName>/map.bin and border.bin from layout.txt
  * copies map.json and scripts.inc into data/maps/<MapName>/
  * adds the layout entry to data/layouts/layouts.json
  * adds the map to its group in data/maps/map_groups.json
  * adds an .include line for the map's scripts to data/event_scripts.s

It only rewrites a file when the content would change. Format: custom_maps/README.md
"""
import json
import os
import re
import struct
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CUSTOM = os.path.join(ROOT, "custom_maps")
LAYOUTS_JSON = os.path.join(ROOT, "data", "layouts", "layouts.json")
GROUPS_JSON = os.path.join(ROOT, "data", "maps", "map_groups.json")
EVENT_SCRIPTS = os.path.join(ROOT, "data", "event_scripts.s")
SCRIPT_ANCHOR = 'data/scripts/shared_secret_base.inc'

DEFAULT_COLLISION = 0
DEFAULT_ELEVATION = 3
BORDER_TILES = 4  # 2x2, as in Emerald


def fail(msg):
    sys.stderr.write("textmap: error: %s\n" % msg)
    sys.exit(1)


def write_if_changed(path, data):
    if isinstance(data, str):
        data = data.encode("utf-8")
    if os.path.exists(path):
        with open(path, "rb") as f:
            if f.read() == data:
                return False
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    return True


def parse_int(text, what):
    try:
        return int(text, 0)
    except ValueError:
        fail("bad number %r for %s" % (text, what))


def parse_layout(path, name):
    header = {}
    legend = {}
    default_c = DEFAULT_COLLISION
    default_e = DEFAULT_ELEVATION
    border = None
    rows = []
    in_grid = False
    with open(path, encoding="utf-8") as f:
        for lineno, raw in enumerate(f, 1):
            line = raw.rstrip("\n")
            if not in_grid:
                if line.strip() == "---":
                    in_grid = True
                    continue
                if not line.strip():
                    continue
                if not line.startswith("#"):
                    fail("%s:%d: header lines must start with '#'" % (path, lineno))
                body = line[1:].strip()
                if body.startswith("tile "):
                    m = re.match(r"tile (\S) = (\S+)((?: [ce]=\d+)*)$", body)
                    if not m:
                        fail("%s:%d: bad tile line" % (path, lineno))
                    ch, mid, opts = m.group(1), parse_int(m.group(2), "tile id"), m.group(3)
                    c, e = default_c, default_e
                    for opt in opts.split():
                        key, val = opt.split("=")
                        if key == "c":
                            c = int(val)
                        else:
                            e = int(val)
                    if not (0 <= mid < 0x400):
                        fail("%s:%d: metatile id must be 0..0x3FF" % (path, lineno))
                    if not (0 <= c < 4 and 0 <= e < 16):
                        fail("%s:%d: collision 0..3, elevation 0..15" % (path, lineno))
                    legend[ch] = (mid, c, e)
                elif body.startswith("border ="):
                    border = [parse_int(t, "border tile") for t in body.split("=", 1)[1].split()]
                    if len(border) != BORDER_TILES:
                        fail("%s:%d: border needs %d tiles" % (path, lineno, BORDER_TILES))
                else:
                    key, _, val = body.partition(":")
                    header[key.strip()] = val.strip()
            else:
                if line:
                    rows.append(line)
    for key in ("id", "name", "primary", "secondary", "group"):
        if key not in header:
            fail("%s: missing header '%s'" % (path, key))
    if border is None:
        fail("%s: missing 'border =' line" % path)
    if not rows:
        fail("%s: empty grid" % path)
    width = len(rows[0])
    for i, row in enumerate(rows):
        if len(row) != width:
            fail("%s: grid row %d has width %d, expected %d" % (path, i + 1, len(row), width))
        for ch in row:
            if ch not in legend:
                fail("%s: grid row %d uses '%s' which is not in the legend" % (path, i + 1, ch))
    return header, legend, border, rows


def pack_cells(rows, legend):
    out = bytearray()
    for row in rows:
        for ch in row:
            mid, c, e = legend[ch]
            out += struct.pack("<H", mid | (c << 10) | (e << 12))
    return bytes(out)


def pack_border(border):
    return b"".join(struct.pack("<H", t) for t in border)


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def dump_json(data):
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def main():
    if not os.path.isdir(CUSTOM):
        return
    names = sorted(
        d for d in os.listdir(CUSTOM)
        if os.path.isfile(os.path.join(CUSTOM, d, "layout.txt"))
    )
    if not names:
        return
    layouts = load_json(LAYOUTS_JSON)
    groups = load_json(GROUPS_JSON)
    changed = []

    for name in names:
        src = os.path.join(CUSTOM, name)
        header, legend, border, rows = parse_layout(os.path.join(src, "layout.txt"), name)
        if header["name"] != name + "_Layout":
            fail("%s: layout name must be '%s_Layout'" % (name, name))
        map_json = os.path.join(src, "map.json")
        if not os.path.isfile(map_json):
            fail("%s: missing map.json" % name)
        mj = load_json(map_json)
        if mj.get("name") != name:
            fail("%s: map.json name is %r" % (name, mj.get("name")))
        if mj.get("layout") != header["id"]:
            fail("%s: map.json layout %r does not match layout id %r" % (name, mj.get("layout"), header["id"]))

        height, width = len(rows), len(rows[0])
        if write_if_changed(os.path.join(ROOT, "data", "layouts", name, "map.bin"), pack_cells(rows, legend)):
            changed.append("layouts/%s/map.bin" % name)
        if write_if_changed(os.path.join(ROOT, "data", "layouts", name, "border.bin"), pack_border(border)):
            changed.append("layouts/%s/border.bin" % name)

        with open(map_json, "rb") as f:
            if write_if_changed(os.path.join(ROOT, "data", "maps", name, "map.json"), f.read()):
                changed.append("maps/%s/map.json" % name)
        scripts_src = os.path.join(src, "scripts.inc")
        if os.path.isfile(scripts_src):
            with open(scripts_src, "rb") as f:
                scripts_data = f.read()
        else:
            scripts_data = ("%s_MapScripts::\n\t.byte 0\n" % name).encode("utf-8")
        if write_if_changed(os.path.join(ROOT, "data", "maps", name, "scripts.inc"), scripts_data):
            changed.append("maps/%s/scripts.inc" % name)

        entry = {
            "id": header["id"],
            "name": header["name"],
            "width": width,
            "height": height,
            "primary_tileset": header["primary"],
            "secondary_tileset": header["secondary"],
            "border_filepath": "data/layouts/%s/border.bin" % name,
            "blockdata_filepath": "data/layouts/%s/map.bin" % name,
            "layout_version": "emerald",
        }
        lst = layouts["layouts"]
        for i, existing in enumerate(lst):
            if existing.get("id") == entry["id"]:
                lst[i] = entry
                break
        else:
            lst.append(entry)

        group = header["group"]
        if group not in groups["group_order"]:
            groups["group_order"].append(group)
        members = groups.setdefault(group, [])
        if name not in members:
            members.append(name)

    if write_if_changed(LAYOUTS_JSON, dump_json(layouts)):
        changed.append("layouts/layouts.json")
    if write_if_changed(GROUPS_JSON, dump_json(groups)):
        changed.append("maps/map_groups.json")

    with open(EVENT_SCRIPTS, encoding="utf-8") as f:
        text = f.read()
    add = []
    for name in names:
        line = '\t.include "data/maps/%s/scripts.inc"\n' % name
        if line not in text:
            add.append(line)
    if add:
        anchor = '\t.include "%s"\n' % SCRIPT_ANCHOR
        if anchor not in text:
            fail("anchor line for event_scripts.s not found: %s" % SCRIPT_ANCHOR)
        text = text.replace(anchor, "".join(add) + anchor, 1)
        if write_if_changed(EVENT_SCRIPTS, text):
            changed.append("event_scripts.s")

    if changed:
        sys.stderr.write("textmap: updated %s\n" % ", ".join(changed))


if __name__ == "__main__":
    main()
