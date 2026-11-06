# Region brief: Meadowmere

A design brief for the region, written so another AI or artist can work from it without the rest of the project. High level first. Each area and place gets its own detail later, one at a time.

**Status:** a proposal by the AI author for the owner. Background design, not a commitment. Only a compact slice is built first (see `features/regions-and-exploration.md`).

Detail below this level: `design/meadowmere-areas.md` (areas), `design/meadowmere-places.md` (places), `design/meadowmere-interiors.md` (interiors).

**Lore is not in this brief.** The world's hidden history is kept separate and is not needed for visuals or layout. Do not invent lore. Small unexplained details are placed by the author later.

## In one sentence

A warm, gentle farming country of meadows, orchards, hedgerows and a slow river: the safe, easy-living heart of the mainland, and where the player starts.

## Where it sits

- Near the center of the mainland, with every neighbor within a few days' walk.
- **North:** the foothills of Cragmoor, where meadow slowly turns to stony slopes.
- **East:** the open grass of Sunreach Steppe, where hedges and fields thin out into wide plains.
- **West:** the wet lowlands of Mistvale, where the river spreads into marsh and fog.
- **South:** the road and river to the coast and Neonport, the harbor city.

Borders are natural and gradual. Biomes blend over several screens, never a hard line.

## Role in the game

- The gentle starting region and the easiest place to live: food is plentiful, danger is low.
- A calm place to learn how the world works, never a tutorial.
- Danger rating: **Safe** in settlements, **Calm** in the countryside, one small **Wild** pocket in the hedgerow wood.
- Never gated. A player can leave in any direction from the first hour.

## Areas

| Area | What it is | Feel |
|---|---|---|
| **The Commons** | The village heartland: greens, wells, small lanes, cottages | Lived-in, cozy, a little untidy |
| **The River Bend** | A slow river with a ford, a footbridge and a mill | Calm water, reeds, stepping stones |
| **The Orchard Terraces** | Terraced hillside orchards and berry gardens | Sweet, bright, busy with insects |
| **The Hedgerow Wood** | Thick old hedgerows grown into a tangle | Slow, easy to lose your way, a little eerie |
| **The Low Meadows** | Wide open grass, wildflowers, grazing animals | Airy, lots of sky, long views |
| **The Downs** | Rolling grass hills rising east toward the Steppe | Windy, big horizons, scattered stone |
| **The Foothill Edge** | The north border where soil turns to stone | Rougher, cooler, hints of the mountains |

## Places (settlements)

Names are working names. One place is detailed at a time.

| Place | Type | Role |
|---|---|---|
| **Fairhaven** | Village | The start. A few dozen homes around a green and a well. Safe. |
| **Wickerby** | Market town | Crossroads where roads from every direction meet. Busiest place in the region. |
| **Hollowmill** | Mill hamlet | A mill on the river with a ford. Quiet and friendly. |
| **Thistledown** | Scattered farms | Isolated farmsteads with their own gardens and animals. |

Between places: roadside shrines and wells, hay barns, a few shelters, orchard huts. No cities in Meadowmere.

## Look and feel (visual guide)

- **Light:** warm late-morning sun, soft shadows, clear sky with a few clouds.
- **Palette:** fresh greens in several shades, honey gold (hay, thatch, ripe fruit), clay red roofs, whitewashed walls, soft river blues, flower accents (yellow, lilac, white).
- **Buildings:** timber frames, whitewash, thatch or clay tile, low stone walls, small chimneys, flower boxes.
- **Props:** hay bales, stone walls, hedgerows, wells, a watermill, scarecrows, laundry lines, fences with gaps, wooden carts, bee boxes.
- **Paths:** worn dirt tracks that follow how people really walk, not straight streets.
- **Water:** slow, clear and shallow in places, deep in others.
- **Season:** a single settled season for now (late spring to early summer).
- **Mood:** safe and lived in. Quiet detail everywhere, nothing loud.

## Sound and weather (feel)

- Music: soft, pastoral and warm. Gentle instruments, slow tempo.
- Weather: mostly clear, occasional light rain. No harsh weather.

## Wildlife and ecology (high level)

No specific species yet. The species list has not been checked in the repo.

- **Meadows:** small and medium grazers and ground-dwellers, mostly shy or passive.
- **Orchards:** pollinators and fruit-eaters drawn to the berries.
- **River and reeds:** water and amphibious life, calm and curious.
- **Hedgerow wood:** shy forest creatures, and one territorial animal that guards the thickest part. Signaled by tracks and quiet.
- Rare finds exist here too, hidden by condition and clue, never by place. See `features/rare-finds.md`.

## Food and survival

- Berries and fruit are abundant. Gardens, orchards and wild bushes.
- The easiest region in which to stay fed. A safe place to build good habits.

## Traversal

Follows the world feel rules in `features/regions-and-exploration.md`.

- Low ledges and terraces can be climbed.
- The river crosses by a shallow ford and a footbridge. Elsewhere it is deep.
- The hedgerow wood is dense and slow, and never walled off.
- No cut trees, no boulder gates, no invisible walls.

## People (flavor only, no scripts yet)

Farmers, millers, orchard keepers, traders on the roads, children, a few travelers passing through. They have homes and routines. Gym leaders and notable characters come later and are not scripted to wait for the player.

## Technical constraints for anyone producing assets

- Target hardware: Game Boy Advance. 8x8 pixel tiles, 4 bits per pixel, 16-color palettes (15 plus transparent), 16x16 metatiles built from tiles.
- The engine is pokeemerald-expansion. Maps are tile grids, authored as text in `custom_maps/` (format in `custom_maps/README.md`).
- **New tiles are allowed and planned.** The owner's chosen tool will generate tile art. Existing Emerald tilesets can still be used where they fit. Known existing tile ids come from `include/constants/metatile_labels.h`.
- **A tileset is more than a picture.** It needs the tile image (`tiles.png`), palettes, a metatile definition (`metatiles.bin`), and terrain behavior data (`metatile_attributes.bin`), plus registration in the tileset headers. The current text-map tool does **not** build tilesets yet. How new tilesets enter the build is an open problem (see below).
- **Art must be original.** Do not copy sprites, tiles or maps from any Pokemon game.
- Whatever is produced must be checked against these constraints and the real build before it is accepted. Nothing here has been tested with an outside tool yet.

## Hidden detail

Each area has a few small curiosities placed by the author. They are not listed here on purpose.

## Open questions

- Final names for places and areas.
- Exact map count and sizes within the ROM and RAM budget (about 6.5 MiB ROM free, measured).
- **How new tilesets get into the build.** The repo tools cannot write binary files, so generated images would have to be uploaded by the owner, and the metatile and attribute data would need a text-based authoring format and a converter, like the one used for maps.
- The palette limit (16 colors per palette) shapes how rich the art can be. Not yet planned per area.
