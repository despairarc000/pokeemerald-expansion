# World structure

How the world is organized, following the Emerald way at a larger scale. Background design, not a commitment. Items marked **unchecked** have not been verified in the code.

## How Emerald does it (checked in `data/maps/map_groups.json`)

- No \"world\" or \"region\" level in the data. The structure is flat: **map groups**, each holding **maps**.
- `gMapGroup_TownsAndRoutes` holds every outdoor town, city, route and underwater route.
- Each town has an interior group (for example `gMapGroup_IndoorLittleroot` holds the houses and the lab). Routes with buildings get one too.
- Separate groups exist for dungeons, secret bases and other dynamic interiors, and special areas.
- **Names carry the hierarchy:** `LittlerootTown`, then `LittlerootTown_BrendansHouse_1F`. The prefix says which place a map belongs to.

## The hierarchy for design

| Level | What it is | Where it lives |
|---|---|---|
| World | The whole game | Docs |
| Region | One of the 14 regions | Docs and name prefix |
| Area | A natural stretch of a region (valley, coast, forest) | Docs and naming |
| Place | A city, town, village, camp, ruin or cave | Naming (and the location-name system, unchecked) |
| Site | A building or interior inside a place | A map |
| Map | One engine map, the smallest unit | `data/maps/` |

## Proposal for our scale

1. **Put the region in the map name:** `Region_Place_Building_Floor`, for example `Meadowmere_Fairhaven_BakeryHouse_1F`.
2. **Each region gets its own outdoor map group.** Interior groups stay per place, as in Emerald.
3. Dungeons and special areas get groups per region or per area, to be decided when needed.

## Unchecked, must verify before planning hundreds of places

- The engine's limit on the number of map groups and maps per group (Emerald stores them in small fields).
- How the location-name system (region map sections) scales past one region, and the limits on how many names exist.
- The warp and save formats' limits on group and map numbers.
- Whether the existing 941 map entries should be kept as templates or removed.
- ROM cost per map and per tileset.

## Notes

- The Frlg entries in the same file are separate game data in this repo. Their effect on the Emerald build has not been checked.
- Design depth is fine on paper, but only a compact Meadowmere is built first (see `features/regions-and-exploration.md`).
