# Region brief: Mistvale (natural lands and waters)

The second region, west of Meadowmere. **Natural lands and waters only**: no houses, no buildings, no interiors. Depth is kept coarse on purpose, so the design is cheap to revise. Detail below the area level is added only when the map maker is ready.

**Status:** a proposal by the AI author for the owner. Background design, not a commitment. Follows `design/CORE.md` and the world feel rules in `features/regions-and-exploration.md`: barriers are costs, not locks, no artificial gates, paths follow the land.

**Lore is not here.** Curiosity slots are counted and filled by the author later. They are not listed on purpose.

## In one sentence

A vast, fog-wrapped country of reed seas, drowned woods, still lakes and sucking bogs, where the land is as much water as earth and you learn to read the ground before you step.

## Where it sits

- **East:** Meadowmere. The river arrives from the Upper Bend and spreads into the Reed Sea.
- **West:** the sea, through tidal creeks and mudflats.
- **North:** low hills that rise toward Cragmoor's western spurs.
- **South:** wet ground that dries slowly into the rim of Sandara's highlands.
- **Hidden links (found, not marked):** a fog road toward Duskhollow, and a cave mouth that leads down to Undervale.

Borders blend gradually. Meadow turns to reed, reed to fen, fen to moor.

## Role in the game

- A natural next step from Meadowmere: harder to read, slower, quieter.
- Danger: **Calm** along the edges and old paths, **Wild** in the interior, no Perilous ground.
- Never gated. Every area can be reached by more than one route, at a price in time, effort or risk.

## Look and feel

- **Light:** diffuse and silver, with fog that thickens at dawn and dusk. Sun comes through in shafts.
- **Palette:** muted greens, olive, peat brown, slate and pearl blues, pale reed gold, flashes of marsh flowers.
- **Weather:** frequent mist and drizzle, rare clear days. Fog is the main weather feature.
- **Sound and mood:** hushed, with water, wind in reeds and distant calls. Quiet, a little eerie, never frightening.
- **Season:** a single settled season, early autumn.

## Costs that define travel (no locks)

| Terrain | Cost |
|---|---|
| Deep reed beds | Slow to push through, easy to lose your way |
| Waterlogged ground | Wading slows you. Soft mud can hold you (a real time cost, never a trap you cannot leave) |
| Peat bog | Dangerous footing. Safe paths follow firm ground and tussocks |
| Open lake | Deep water is dangerous to swim. Shores and islets reward careful routes |
| Tidal creeks | Tides open and close routes. A route may be dry at low tide and under water at high tide |
| Fog | Hides landmarks. Sound and tracks help you navigate |

These are costs, not walls. Engine support for slow terrain and stamina is not built yet (see `features/regions-and-exploration.md`), so early versions use layout alone.

## Areas

| # | Area | What it is | Danger |
|---|---|---|---|
| 1 | **The Reed Sea** | A sea of reeds and shallow channels where the river spreads out | Calm |
| 2 | **The Fenwood** | A waterlogged forest on raised hummocks, hung with moss and mist | Calm to Wild |
| 3 | **The Mere** | A large still lake with islets and thick mist | Wild |
| 4 | **The Peat Moors** | Open boggy moorland of cotton-grass, black pools and firm tussocks | Wild |
| 5 | **The Tidal Creeks** | The west coast: salt marsh, mudflats, creeks and tides | Calm to Wild |
| 6 | **The Pale Verge** | A band where fog thickens toward the sea of mist, quiet and still | Wild |

### 1. The Reed Sea
- **Position:** east edge, where the river from Meadowmere spreads.
- **Land:** reed beds, shallow channels, gravel bars, small dry islands, willow stands.
- **Routes:** a dry rim path along the east and many wading routes through the channels.
- **Life (families):** wading birds, amphibious creatures, insects, shy fish.
- **Curiosity slots:** 3.

### 2. The Fenwood
- **Position:** center-north, on slightly higher ground.
- **Land:** hummocks of firm ground among black water, crooked trees, moss, ferns, fallen logs.
- **Routes:** hummock hopping, root bridges, a few firm trails. Easy to get turned around.
- **Life (families):** forest and bog dwellers, insects, creatures that avoid fog.
- **Notes:** a cave mouth here leads down toward Undervale.
- **Curiosity slots:** 3.

### 3. The Mere
- **Position:** center of the region.
- **Land:** a wide, still lake with islets, a reed shore and shallow margins. Deep in the middle.
- **Routes:** shore paths on three sides, shallow causeways of stone and root to the nearest islets.
- **Life (families):** water creatures, wading birds, one large territorial water dweller in the deepest part (ripples signal it).
- **Curiosity slots:** 3.

### 4. The Peat Moors
- **Position:** south and southwest.
- **Land:** open moorland, cotton-grass, black pools, firm tussocks, scattered stones.
- **Routes:** old firm paths between tussocks. Straight lines are risky, winding ones are safe.
- **Life (families):** ground nesters, bog dwellers, fast runners on firm ground.
- **Notes:** the biggest natural barrier in the region, crossed at a price in time and care.
- **Curiosity slots:** 3.

### 5. The Tidal Creeks
- **Position:** the west coast.
- **Land:** salt marsh, mudflats, branching creeks, sandbars, driftwood, a low shingle beach.
- **Routes:** tide-dependent. Many routes are open only at low tide. A high-tide route follows the inland edge.
- **Life (families):** shore birds, crustacean-like and burrowing creatures, tide-pool life, creatures that follow the tide.
- **Curiosity slots:** 3.

### 6. The Pale Verge
- **Position:** the far west and northwest, where fog gathers.
- **Land:** low, bare ground, sparse reeds, still pools, a very quiet shore.
- **Routes:** a faint track that fades into fog. Sound and tracks lead the way.
- **Life (families):** fog-shy creatures, silent birds, things that watch from far away.
- **Notes:** a hidden route toward Duskhollow begins here. It is found, not marked.
- **Curiosity slots:** 3.

## Natural maps (coarse list, 17 maps)

Only name, size, danger and one line each. Sizes are classes (S about 24, M about 32, L about 40 tiles), not measured against the engine limit. Detailed terrain mixes are left for later.

| # | Map | Area | Size | Danger | Character |
|---|---|---|---|---|---|
| 1 | Mistvale_ReedSea_Rim | Reed Sea | L | Calm | The dry eastern rim, arriving from Meadowmere |
| 2 | Mistvale_ReedSea_Channels | Reed Sea | L | Calm | Wading routes through reeds |
| 3 | Mistvale_ReedSea_Islands | Reed Sea | M | Calm | Willow islands and gravel bars |
| 4 | Mistvale_Fenwood_Edge | Fenwood | M | Calm | Where reed becomes forest |
| 5 | Mistvale_Fenwood_Hummocks | Fenwood | L | Wild | Hopping between firm ground |
| 6 | Mistvale_Fenwood_Hollow | Fenwood | S | Wild | A dry hollow with a cave mouth |
| 7 | Mistvale_Mere_Shore | The Mere | L | Calm | Reed shore and shallow margins |
| 8 | Mistvale_Mere_Causeway | The Mere | M | Wild | Stone and root paths to islets |
| 9 | Mistvale_Mere_Deep | The Mere | M | Wild | Open deep water with mist |
| 10 | Mistvale_Moors_Edge | Peat Moors | L | Calm | Firm ground and cotton-grass |
| 11 | Mistvale_Moors_Bog | Peat Moors | L | Wild | Black pools and tussock paths |
| 12 | Mistvale_Moors_Standing | Peat Moors | M | Wild | Open moor with scattered stones |
| 13 | Mistvale_Creeks_Marsh | Tidal Creeks | L | Calm | Salt marsh and shallow creeks |
| 14 | Mistvale_Creeks_Flats | Tidal Creeks | L | Calm to Wild | Mudflats that change with the tide |
| 15 | Mistvale_Creeks_Shingle | Tidal Creeks | M | Calm | A low beach and driftwood |
| 16 | Mistvale_Verge_Approach | Pale Verge | M | Wild | Thinning ground and thickening fog |
| 17 | Mistvale_Verge_Stillwater | Pale Verge | M | Wild | Quiet pools at the edge of the mist |

### How the maps connect (draft)

```
            [Northern hills toward Cragmoor]
  Meadowmere -> [1 Rim] -> [4 Fenwood Edge] -> [5 Hummocks] -> [6 Hollow] (cave to Undervale)
                 |  \                  |
              [2 Channels]         [7 Mere Shore] -> [8 Causeway] -> [9 Deep]
                 |                     |
              [3 Islands]          [10 Moors Edge] -> [11 Bog] -> [12 Standing]
                                         |                              |
                                  [13 Marsh] -> [14 Flats] -> [15 Shingle] -> sea (west)
                                         |
                                  [16 Verge Approach] -> [17 Stillwater] -> toward Duskhollow
```

Rules: edges meet at the same position on both maps and terrain matches across them. Most areas have more than one route. The route to the Pale Verge can be reached from the tidal creeks or from the Moors.

Curious slots: 18 across the six areas (3 each). Filled by the author later.

## Wildlife and food (high level)

No specific species yet. The species list has not been checked in the repo.

- Families: water and wading life, bog dwellers, insects, shy forest creatures, fog-shy creatures, shore life.
- Food is thinner than in Meadowmere: fish, water plants, mushrooms, a few hardy berries. A good place to feel the survival side of the game.
- Rare finds follow `features/rare-finds.md`: hidden by condition and clue, never by place.

## Connections to the world

- **Meadowmere:** through the River Bend at the Upper Bend (map 2 of Meadowmere), arriving at Reed Sea Rim.
- **Cragmoor:** low northern hills toward its western spurs.
- **Sandara:** the southern moors dry out toward its highlands.
- **Duskhollow:** the hidden route from the Pale Verge.
- **Undervale:** the cave mouth in the Fenwood Hollow.

## Open questions

- Map sizes and ROM budget for 17 maps. Not measured.
- Real tiles for reed, fen, moss, bog and tide, and the 16-color palette limit for a misty, muted look.
- How tides are shown in a tile-based map (two versions of a map, or animated tiles). Not decided.
- Species per zone, once the species list is checked.
