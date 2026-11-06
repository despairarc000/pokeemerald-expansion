# custom_maps

Maps authored as plain text. A build-time tool turns each folder into the engine's binary map data, so a new map is a new folder. No Porymap and no binary files in the repo.

The tool is `tools/textmap/build_textmaps.py`. It runs automatically when `make` reads `map_data_rules.mk`, and it is safe to run repeatedly.

## Folder layout

```
custom_maps/<Region>_<Place>[_<Building>_<Floor>]/
  layout.txt   the tile grid (required)
  map.json     the map definition, same format as data/maps/*/map.json (required)
  scripts.inc  the map's scripts (optional, a minimal stub is made if missing)
```

The folder name is the map name. See `design/world-structure.md` for naming.

## layout.txt

Header lines start with `#`, then a line with `---`, then the grid.

```
# id: LAYOUT_MEADOWMERE_FAIRHAVEN
# name: Meadowmere_Fairhaven_Layout
# primary: gTileset_General
# secondary: gTileset_Petalburg
# group: gMapGroup_Meadowmere
# border = 0x170 0x170 0x170 0x170
# tile . = 0x001 e=3
# tile ~ = 0x170 e=1
---
~~~~
~..~
~~~~
```

- `id`, `name`, `primary`, `secondary`, `group` are required. `name` must be `<MapName>_Layout`. The id must match `layout` in `map.json`.
- `border =` takes 4 metatile ids (a 2x2 border).
- `tile X = <metatile id> [c=<collision>] [e=<elevation>]` defines a grid character. Defaults are collision 0 and elevation 3. Metatile ids are 0 to 0x3FF.
- Every row of the grid must be the same width. Width and height come from the grid.
- Output tile value = metatile id, plus collision shifted left 10, plus elevation shifted left 12, stored as little-endian 16 bits (the Porymap block format).

Metatile ids for the General tileset come from `include/constants/metatile_labels.h`.

## What the tool changes

It writes or updates these files. Do not commit the generated changes to the shared files from a local build; they are regenerated every time.

- `data/layouts/<Name>/map.bin` and `border.bin` (generated)
- `data/maps/<Name>/map.json` and `scripts.inc` (copied)
- `data/layouts/layouts.json` (entry added or replaced)
- `data/maps/map_groups.json` (group and map added)
- `data/event_scripts.s` (one `.include` line added before the `shared_secret_base.inc` line)

## Unverified assumptions

- The border size is 2x2 (8 bytes). Not checked against the engine.
- Collision and elevation defaults (0 and 3 for ground, elevation 1 for water) follow what Emerald maps usually use, not a verified rule.
- Whether the engine's map group numbering handles a new group appended after the existing ones is untested.
