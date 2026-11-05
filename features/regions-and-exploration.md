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

### What is map design and what is engine

Most of this is **map design**, not engine work:

- **Map design alone covers:** the shape of the land, winding trails, getting lost in a dense forest by layout, dead ends, wide open areas, where settlements sit, no artificial gates, and sheer cliffs or deep water as real obstacles.
- **Engine work is only needed for "cost over time":** making a tile slow you down, tire you, or drain hunger. That depends on the stamina and hunger systems (`features/traversal.md`, `features/survival-hunger.md`), which are not built.
- **The engine already defines many terrain behaviors.** `include/constants/metatile_behaviors.h` lists, among others, deep sand, ice, thin ice, cracked ice, muddy and bumpy slopes, slide tiles, water currents, fast water, waterfalls, ledge jumps, rock stairs, ladders and `MB_ROCK_CLIMB`. **What each one actually does in code has not been checked.**
- So v1 can start with map design only. Cost (stamina, slowing) is added later as those systems arrive.

## Notes and parked ideas

- 14 regions are planned (`design/regions.md`), but none are committed before v1 works.
- ROM has about 6.5 MiB free, so every map and tileset needs budgeting. Replacing the existing Hoenn maps might free space, but that is not measured.
- World layout sketch (not confirmed): 8 walkable mainland regions, 3 sea regions, 2 layers (Undervale below, Skyreach above), and Ur-Kalah at the far edge. See diary log 10.
- The mainland is continuous, with natural barriers that cost something, not locks.
