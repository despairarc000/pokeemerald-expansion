# Regions and exploration

**Core line:** Explore free

| Ver | Adds | Requires | Status |
|---|---|---|---|
| 1 | A compact Meadowmere with a few maps, dense with things to find | Gate A, Gate B | Next |
| 2 | A second area connected to it, and a hub (Neonport) | v1, ROM budget | Parked |
| 3 | Each region gets a defining quirk | v2 | Parked |
| 4 | More regions, one at a time, fully dense | v3 | Parked |
| 5 | Ur-Kalah, the almost-uncivilized continent | v4, traversal v4 | Parked |

## Next version spec (v1)

- A handful of maps, not a full region.
- The density rule applies: something worth stopping for every few screens (curiosity, trace of life, a choice, a resource, a lore fragment).
- No quest markers.
- The world feel rules below apply from the first map.

## World feel rules

The world should feel like the real world, not like a Pokemon-coded map. Terrain restricts movement because of what it is. It is never designed to guide or to gate.

1. **Barriers are costs, not locks.** A dense forest is slow and easy to get lost in. A cliff or a waterfall drop is dangerous. A river is crossable where it is shallow. Each is passable at a price (time, stamina, hunger, risk) or avoidable. Nobody goes through them because they are a bad idea, not because a wall says no.
2. **Nothing blocks the player for the game's convenience.** No cut-tree or boulder gates, no invisible walls, no line of trainers acting as a door.
3. **Paths exist for a reason.** Trails follow valleys, rivers and ridges, made by people and animals. No numbered corridor routes.
4. **Settlements sit where people would live:** water, fertile land, harbors, crossings.
5. **Wide areas, not tubes.** Open ground with trails and varied terrain, not a narrow strip you must follow.
6. **Natural boundaries are real features.** Mountain ranges, rivers, forests and seas separate regions, and biomes blend gradually across them.
7. **Dead ends are fine.** Nothing exists only to guide the player.
8. **Natural shapes.** Avoid grid-straight edges and symmetrical layouts where the tile grid allows.

### Engine honesty (unmeasured)

- Costs such as slow terrain, getting lost and stamina need terrain behaviors and the stamina system (see `features/traversal.md`). Until those exist, v1 uses plain impassable terrain only where physics demands it (sheer cliffs, deep water) and avoids artificial gates.
- Tile-based maps limit how organic shapes can look. How far we can push this is untested.

## Notes and parked ideas

- 14 regions are planned (`design/regions.md`), but none are committed before v1 works.
- ROM has about 6.5 MiB free, so every map and tileset needs budgeting. Replacing the existing Hoenn maps might free space, but that is not measured.
- World layout sketch (not confirmed): 8 walkable mainland regions, 3 sea regions, 2 layers (Undervale below, Skyreach above), and Ur-Kalah at the far edge. See diary log 10.
- Open: how continuous the mainland is, now answered in spirit: continuous, with natural barriers that cost something, not locks.
