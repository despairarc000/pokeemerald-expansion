# Meadowmere: natural maps

Map-by-map design for the natural environments of Meadowmere. **No houses and no buildings** in this document. Villages, interiors and structures come later. Written to be tool-agnostic, so it can feed any map maker (human or AI).

**Status:** a proposal by the AI author for the owner. Background design, not a commitment. Follows `design/CORE.md` and the world feel rules in `features/regions-and-exploration.md`: barriers are costs, not locks; no cut-tree or boulder gates; paths follow the land; wide open ground, not tubes; natural boundaries blend gradually.

**Lore is not here.** Curiosity slots are counted per map and filled by the author later. They are not listed on purpose.

Detail above this level: `design/meadowmere-areas.md`, `design/meadowmere-places.md`.

## Reading this document

- **Tile roles, not tile ids.** Maps are described with roles (grass, hedge, shallow water). Mapping roles to real tiles is the map maker's job and depends on the tileset.
- **Sizes are classes, not measurements.** S about 24 by 24, M about 32 by 32, L about 40 by 40 tiles. The engine's real size limit has not been checked, and the ROM budget (about 6.5 MiB free) has not been spent against these.
- **Positions are relative:** north, south, east and west, and thirds (left, center, right).
- **Exits** list which edge connects to which map. Some edges lead out of the region.

## Tile roles (vocabulary)

| Group | Roles |
|---|---|
| Ground | short grass, long grass, tall grass (wild encounters), flowers, dirt track, worn path, gravel, mud, sand bar, heather, scree, bare rock, thin soil |
| Water | shallow water, deep water, reeds, lily pads, stepping stones, stream, pond, spring, waterfall (small) |
| Plants | single tree, tree cluster, willow, fruit tree, berry bush, hedge (thick), bramble, hawthorn, fern, crops (as terrain) |
| Land forms | ledge (climbable, low), terrace wall (climbable), cliff (sheer, not climbable), slope (walkable), rock outcrop, standing stone, boulder (decor only, never a gate) |
| Crossings | ford, footbridge, stepping stones |
| Props (no buildings) | hay bale, stone wall (low), wooden fence with gaps, signpost, cairn, bee box, cart track |

## Map list (20 natural maps)

| # | Map | Area | Size | Danger |
|---|---|---|---|---|
| 1 | Meadowmere_Commons_Pastures | Commons | L | Safe |
| 2 | Meadowmere_RiverBend_UpperBend | River Bend | M | Calm |
| 3 | Meadowmere_RiverBend_Ford | River Bend | M | Calm |
| 4 | Meadowmere_RiverBend_LowerBend | River Bend | M | Calm |
| 5 | Meadowmere_Orchard_LowerTerraces | Orchard Terraces | M | Safe |
| 6 | Meadowmere_Orchard_MidTerraces | Orchard Terraces | M | Safe to Calm |
| 7 | Meadowmere_Orchard_UpperTerrace | Orchard Terraces | S | Calm |
| 8 | Meadowmere_Hedgerow_OuterHedges | Hedgerow Wood | M | Calm |
| 9 | Meadowmere_Hedgerow_Tangle | Hedgerow Wood | M | Calm |
| 10 | Meadowmere_Hedgerow_Knot | Hedgerow Wood | S | Wild |
| 11 | Meadowmere_Meadows_NorthMeadow | Low Meadows | L | Calm |
| 12 | Meadowmere_Meadows_CrossroadsFields | Low Meadows | L | Calm |
| 13 | Meadowmere_Meadows_ThistleFields | Low Meadows | L | Calm |
| 14 | Meadowmere_Meadows_SouthMeadow | Low Meadows | L | Calm |
| 15 | Meadowmere_Downs_LowerDowns | The Downs | L | Calm |
| 16 | Meadowmere_Downs_RidgeTrack | The Downs | M | Calm |
| 17 | Meadowmere_Downs_FarDowns | The Downs | M | Calm to Wild |
| 18 | Meadowmere_Foothill_HeatherSlopes | Foothill Edge | M | Calm |
| 19 | Meadowmere_Foothill_ScreeQuarry | Foothill Edge | M | Calm to Wild |
| 20 | Meadowmere_Foothill_NorthernTrack | Foothill Edge | M | Wild |

Maps for Fairhaven, Wickerby, Hollowmill and Thistledown's farms are separate, not designed here. Where a place will sit, the map reserves an open **gap** and passes around it.

## Connectivity (draft)

```
                 [20 NorthernTrack] -> Cragmoor
                 [19 ScreeQuarry]
 Mistvale <- [2 UpperBend]  [18 HeatherSlopes]
                 |             |
          [8 OuterHedges]-[5 LowerTerr]-[6 MidTerr]-[7 UpperTerr]--[18]
             |    |           |
        [9 Tangle]  \       [1 Commons Pastures]--[11 NorthMeadow]--[15 LowerDowns]--[16 RidgeTrack]--[17 FarDowns] -> Steppe
        [10 Knot]    \         |       |                 |
                  [3 Ford]   (Fairhaven gap)        [13 ThistleFields]
                     |         |
              [4 LowerBend]  [12 CrossroadsFields]--(Wickerby gap)--[14 SouthMeadow] -> Neonport
                     |
                river south to the coast
```

Rules for every connection:
- Edges meet in the same place on both maps (same position along the edge).
- Terrain on both sides of an edge matches (grass meets grass, water meets water), so the join blends.
- Several routes exist between most areas, so no route is the only way.

## Commons

### 1. Commons Pastures (L, Safe)
- **Setting:** the farmland, pasture and streams around Fairhaven. **The village sits in a reserved gap at the center**; this map never contains buildings.
- **Terrain mix:** about 55 percent short grass, 15 percent paddock land (grass with fence gaps), 10 percent garden plots (crops as terrain), 8 percent stream and pond, 7 percent dirt tracks, 5 percent flowers.
- **Elevation:** nearly flat, with two low terraces on the north side.
- **Water:** a stream running west to east from the north terraces, a duck pond in the southeast, a washing pool upstream.
- **Paths:** worn tracks run from the village gap to each edge. They wander, with cut-throughs and desire lines.
- **Features:** an unfenced orchard row, hay bales, a shrine site at the south road junction (marker only), a signpost, grazing ground.
- **Exits:** north to Lower Terraces (5), west to the Ford (3), east to North Meadow (11), south to Crossroads Fields (12).
- **Wildlife behavior:** passive and skittish grazers and birds, wandering through gardens at dusk.
- **Curiosity slots:** 3.

## River Bend

### 2. Upper Bend (M, Calm)
- **Setting:** where the river arrives from the northwest marshland.
- **Terrain mix:** 30 percent river, 20 percent reeds, 20 percent willow thicket, 15 percent short grass bank, 10 percent mud, 5 percent gravel.
- **Elevation:** low, with a gentle bank on the east side.
- **Water:** a slow, wide river with shallow riffles. Deep pools in the middle.
- **Paths:** a bank track on the east side. The west bank is rough going. Willow islands can be reached by shallow stretches.
- **Features:** willow islands, a fallen log, a flat stone for fishing, reed beds hiding small clearings.
- **Exits:** north to Mistvale (region exit), east to Outer Hedges (8), south to the Ford (3).
- **Wildlife behavior:** shy water life, birds in the reeds, a territorial pool dweller in the deepest water (ripples signal it).
- **Curiosity slots:** 1.

### 3. The Ford (M, Calm)
- **Setting:** the crossing point of the river.
- **Terrain mix:** 25 percent river, 15 percent shallow ford, 15 percent reeds, 25 percent grass banks, 10 percent gravel bar, 10 percent track.
- **Water:** the ford is shallow and walkable. Upstream and downstream are deep. Stepping stones give a second, harder crossing.
- **Paths:** tracks come down both banks and meet at the ford. A footbridge site sits to the south.
- **Features:** the ford itself, stepping stones, the footbridge (a crossing feature), a hitching post.
- **Exits:** north to Upper Bend (2), south to Lower Bend (4), east to Commons Pastures (1).
- **Wildlife behavior:** curious river life at the shallows, birds along the banks.
- **Curiosity slots:** 1.

### 4. Lower Bend (M, Calm)
- **Setting:** the river widens and slows toward the coast. **Hollowmill sits in a reserved gap on the east bank.**
- **Terrain mix:** 35 percent river, 15 percent mud bank, 10 percent gravel bar, 15 percent reeds, 20 percent grass, 5 percent track.
- **Water:** wide and slow. A fish weir site near the gap. A rope-ferry crossing in the south.
- **Paths:** bank tracks. Crossing is possible only by the ferry post or by swimming.
- **Features:** a weir site, a ferry post (marker), a mud flat with tracks, sand bars, a quiet backwater.
- **Exits:** north to the Ford (3), east to South Meadow (14), south along the river to the coast (region exit).
- **Wildlife behavior:** passive wading birds, shy amphibious life.
- **Curiosity slots:** 1.

## Orchard Terraces

### 5. Lower Terraces (M, Safe)
- **Setting:** the first steps up from the Commons onto the sunny slope.
- **Terrain mix:** 40 percent short grass, 20 percent fruit trees, 15 percent berry hedge, 10 percent terrace walls, 10 percent tracks, 5 percent flowers.
- **Elevation:** three shallow terraces, each one tile or two up.
- **Paths:** switchback tracks and short stone steps. Every wall can be climbed.
- **Features:** orchard rows, berry hedges, a view over the Commons, a bench spot (marker).
- **Exits:** south to Commons Pastures (1), north to Mid Terraces (6), west to Outer Hedges (8).
- **Wildlife behavior:** curious pollinators, fruit-eaters, all passive.
- **Curiosity slots:** 1.

### 6. Mid Terraces (M, Safe to Calm)
- **Setting:** the heart of the orchards, with a bee yard site.
- **Terrain mix:** 35 percent short grass, 25 percent fruit trees, 15 percent berry bush, 15 percent terrace walls and steps, 10 percent flowers.
- **Elevation:** four terraces, the steepest drops of the area (still climbable).
- **Paths:** stair runs between terraces, shortcuts over low walls.
- **Features:** a bee yard (bee boxes, a low wall), a press site (marker for the pressing house), flower banks.
- **Exits:** south to Lower Terraces (5), north to Upper Terrace (7).
- **Wildlife behavior:** swarms of pollinators near the bee yard. Humming signals territorial insects.
- **Curiosity slots:** 1.

### 7. Upper Terrace (S, Calm)
- **Setting:** the top of the slope, with the broadest view.
- **Terrain mix:** 45 percent short grass, 20 percent heather, 15 percent fruit trees, 10 percent terrace walls, 10 percent bare rock.
- **Elevation:** the highest ground in the Terraces. A lookout platform site on the crest.
- **Paths:** one track from below. The crest is open to wander.
- **Features:** a lookout site (marker), wind-bent trees, a stone-lined spring, first heather.
- **Exits:** south to Mid Terraces (6), north to Heather Slopes (18).
- **Wildlife behavior:** wind-loving birds, passive grazers.
- **Curiosity slots:** 1.

## Hedgerow Wood

### 8. Outer Hedges (M, Calm)
- **Setting:** the edge of the wood, where old hedges begin to thicken.
- **Terrain mix:** 35 percent short grass, 30 percent hedge, 15 percent bramble, 10 percent trees, 10 percent track.
- **Paths:** a woodcutter's path and several animal tracks. Hedges are dense but not walls. They can be pushed through slowly.
- **Features:** a woodcutter's shelter site (marker), a half-buried gate site, small clearings.
- **Exits:** west to Upper Bend (2), east to Lower Terraces (5), north to the Tangle (9).
- **Wildlife behavior:** shy forest creatures, easily startled.
- **Curiosity slots:** 1.

### 9. Tangle (M, Calm)
- **Setting:** the maze of old hedgerows.
- **Terrain mix:** 45 percent hedge, 20 percent bramble, 15 percent trees, 15 percent small clearings, 5 percent track.
- **Layout:** a winding maze. Dead ends are normal. The way through can be found by trial, by animal tracks, and by sound.
- **Features:** a stream crossing, a hidden dell, an old marker post.
- **Exits:** south to Outer Hedges (8), north to the Knot (10).
- **Wildlife behavior:** shy creatures, and signs of the territorial guardian (tracks, scratched bark, sudden quiet).
- **Curiosity slots:** 1.

### 10. The Knot (S, Wild)
- **Setting:** the thickest part, at the center of the wood.
- **Terrain mix:** 40 percent hedge, 25 percent bramble, 20 percent trees, 15 percent central clearing.
- **Layout:** one central clearing ringed by the densest hedge. Reached only through the Tangle.
- **Danger:** the territorial guardian lives here. It warns first and chases only if you stay. You can always retreat.
- **Exits:** south to the Tangle (9). Otherwise a dead end.
- **Curiosity slots:** 2.

## Low Meadows

### 11. North Meadow (L, Calm)
- **Setting:** the wide open grass east of the Commons.
- **Terrain mix:** 55 percent short grass, 15 percent tall grass, 15 percent flowers, 8 percent pond and stream, 7 percent tracks.
- **Water:** one pond with stepping stones, a small stream feeding it.
- **Features:** a lone tree, grazing herds, a roadside well site (marker), a long view south.
- **Exits:** west to Commons Pastures (1), east to Lower Downs (15), south to Thistle Fields (13).
- **Wildlife behavior:** herds that startle and move together. Large grazers can be territorial if cornered.
- **Curiosity slots:** 1.

### 12. Crossroads Fields (L, Calm)
- **Setting:** open ground north of Wickerby. **Wickerby sits in a reserved gap at the south edge.**
- **Terrain mix:** 50 percent short grass, 15 percent crops as terrain, 15 percent tall grass, 10 percent roads and tracks, 10 percent flowers and hedges.
- **Paths:** four roads converge toward the Wickerby gap, with lots of open land between them.
- **Features:** a windbreak of tall trees, a signpost at the junction, cart ruts, a drainage ditch.
- **Exits:** north to Commons Pastures (1), east to Thistle Fields (13), south through the Wickerby gap to South Meadow (14), west to Lower Bend (4).
- **Wildlife behavior:** passive grazers and ground nesters. More traffic, so animals keep their distance from the roads.
- **Curiosity slots:** 0.

### 13. Thistle Fields (L, Calm)
- **Setting:** the farmland edge where Thistledown's farms will sit. **Farms occupy reserved gaps.**
- **Terrain mix:** 45 percent short grass, 25 percent crops as terrain, 10 percent hedge lines, 10 percent tall grass, 10 percent tracks.
- **Features:** a shared well site (marker), a windbreak, hay bales, field boundaries.
- **Exits:** north to North Meadow (11), west to Crossroads Fields (12), east to Lower Downs (15), south to South Meadow (14).
- **Wildlife behavior:** passive and curious around the fields. A rare shy visitor at dusk.
- **Curiosity slots:** 1.

### 14. South Meadow (L, Calm)
- **Setting:** the meadows thinning toward the southern road to Neonport.
- **Terrain mix:** 50 percent short grass, 15 percent tall grass, 10 percent ponds and ditches, 15 percent road and tracks, 10 percent flowers and scrub.
- **Features:** a roadside shrine site (marker), a small pond with reeds, a drainage ditch, wheel ruts.
- **Exits:** north through the Wickerby gap to Crossroads Fields (12), north to Thistle Fields (13), west to Lower Bend (4), south to the road to Neonport (region exit).
- **Wildlife behavior:** passive grazers, shy water life at the pond.
- **Curiosity slots:** 1.

## The Downs

### 15. Lower Downs (L, Calm)
- **Setting:** gentle hills where meadow starts to roll.
- **Terrain mix:** 55 percent short grass, 15 percent tall grass, 10 percent hawthorn and thorn, 10 percent bare rock and standing stones, 10 percent tracks.
- **Elevation:** rolling hills, two or three low rises.
- **Features:** a roadside well site (marker), scattered standing stones, a shepherd's track.
- **Exits:** west to North Meadow (11) and Thistle Fields (13), east to Ridge Track (16).
- **Wildlife behavior:** sturdy grazers, open-air birds, ground nesters.
- **Curiosity slots:** 1.

### 16. Ridge Track (M, Calm)
- **Setting:** the exposed ridge with long views.
- **Terrain mix:** 50 percent short grass, 20 percent heather, 15 percent bare rock, 10 percent thorn, 5 percent track.
- **Elevation:** a high ridge with scarps on both sides (climbable). A shepherd's hut site sits on the crest.
- **Features:** a lone-tree hill site, wind-bent hawthorn, the ridge track itself.
- **Exits:** west to Lower Downs (15), east to Far Downs (17).
- **Wildlife behavior:** fast open-ground runners, wind birds. Gusts matter once flight exists.
- **Curiosity slots:** 1.

### 17. Far Downs (M, Calm to Wild)
- **Setting:** the eastern end, where the land flattens toward the Steppe.
- **Terrain mix:** 55 percent short grass, 15 percent tall grass, 10 percent thorn, 10 percent bare rock, 10 percent heather.
- **Features:** a stone-ringed spring site, wide empty ground, the first signs of Steppe herds.
- **Exits:** west to Ridge Track (16), east to Sunreach Steppe (region exit).
- **Wildlife behavior:** wider ranging grazers and runners. Territorial behavior is stronger near the eastern edge.
- **Curiosity slots:** 1.

## Foothill Edge

### 18. Heather Slopes (M, Calm)
- **Setting:** meadow turning to heather and stone.
- **Terrain mix:** 40 percent heather, 25 percent short grass, 15 percent thin soil and scree, 10 percent bare rock, 10 percent streams.
- **Elevation:** a steady slope upward to the north.
- **Features:** a stream cascade site (small), a high pasture, a cairn site.
- **Exits:** south to Upper Terrace (7), north to Scree and Quarry (19).
- **Wildlife behavior:** hardy climbers and rock dwellers, shy.
- **Curiosity slots:** 1.

### 19. Scree and Quarry (M, Calm to Wild)
- **Setting:** loose stone slopes and an old quarry cut.
- **Terrain mix:** 35 percent scree, 25 percent bare rock, 15 percent heather, 10 percent cliff (sheer), 10 percent ledge, 5 percent track.
- **Elevation:** steep, with short cliffs. Rockfall marks signal danger.
- **Features:** a quarry scar, a tool shed site (marker), loose-stone slopes.
- **Exits:** south to Heather Slopes (18), north to the Northern Track (20).
- **Wildlife behavior:** rock dwellers, territorial when approached close to their ledges.
- **Curiosity slots:** 1.

### 20. Northern Track (M, Wild)
- **Setting:** the climb toward Cragmoor.
- **Terrain mix:** 30 percent scree, 30 percent bare rock, 15 percent heather, 15 percent ledge and slope, 10 percent track.
- **Features:** a cave mouth leading up (and later toward Undervale), a stream cascade, the first cairn on the track.
- **Exits:** south to Scree and Quarry (19), north to Cragmoor (region exit).
- **Danger:** wild, signaled by tracks and rockfall marks. Retreat is always possible.
- **Wildlife behavior:** shy mountain creatures wandering down, territorial near the cave mouth.
- **Curiosity slots:** 1.

## Curiosity slots

| Area | Slots | Maps |
|---|---|---|
| Commons | 3 | Commons Pastures (3) |
| River Bend | 3 | Upper Bend, Ford, Lower Bend (1 each) |
| Orchard Terraces | 3 | Lower, Mid, Upper (1 each) |
| Hedgerow Wood | 4 | Outer Hedges (1), Tangle (1), Knot (2) |
| Low Meadows | 3 | North Meadow, Thistle Fields, South Meadow (1 each) |
| The Downs | 3 | Lower Downs, Ridge Track, Far Downs (1 each) |
| Foothill Edge | 3 | Heather Slopes, Scree and Quarry, Northern Track (1 each) |
| **Total** | **22** | |

Places (villages, farms) have their own slots, counted separately.

## Open questions

- Real map size limits and the ROM budget for 20 large and medium maps.
- Exact edge positions, to be fixed in the map maker so neighbors line up.
- Which existing tiles match each role, and which roles need new tiles.
- Species assignment per zone. The species list has not been checked in the repo.
- Which map is built first (suggestion: Commons Pastures with the village gap left empty).
