# Built-in Porymap Emerald and FireRed prefabs

Emerald: https://github.com/huderlem/porymap/blob/master/resources/text/prefabs_default_emerald.json
FireRed: https://github.com/huderlem/porymap/blob/master/resources/text/prefabs_default_firered.json
Original Porymap project: https://github.com/huderlem/porymap
Retrieved default prefab definitions: 71 Emerald entries and 45 FireRed entries. The imported definitions include
names, dimensions, source tileset symbols, native metatile IDs, collision and
elevation. The compiled textures are rendered from the existing repository's
Emerald source art rather than copying PNGs from a separate ROM hack.

Porymap's source repository declares LGPL-3.0 in its LICENSE.md. A copy
of that license is retained at assets/PORYMAP-LICENSE.md.
Pokémon artwork rights are separate from the editor/prefab file license.

Map Studio filters these vanilla prefab definitions against the current source
symbols and validates metatile ID presence; modified layouts may not match,
so the editor never assumes all 71 are safe on every map.

FireRed's upstream `gTileset_General` prefab symbol is mapped to this
fork's `gTileset_General_Frlg`; source-local metatile IDs are never globally
renumbered. The FireRed/LeafGreen collection uses the real FRLG graphics
already present in this expansion fork. This does not include Unbound art.
