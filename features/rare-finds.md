# Rare finds

**Core line:** Explore free (with Doesn't revolve around the player)

Finding a rare Pokemon should feel like finding a shiny or an easter egg: a surprise you earned, never a stop on a guaranteed route.

| Ver | Adds | Requires | Status |
|---|---|---|---|
| 1 | One hand-placed rare Pokemon that appears only under a world condition (for example weather or time of day). No marker, no guaranteed spot | world time v1, wild Pokemon v1 | Parked |
| 2 | The world hints at it: tracks, sounds, an NPC remark. Nothing points to it on a map | v1, NPC life v1 | Parked |
| 3 | Finding it is a moment: a sound or music cue, and later an NPC may mention it if they saw | v2 | Parked |
| 4 | Special tier: Pokemon found through condition chains (food, tide, other creatures nearby) | v3, hunger v1 | Parked |
| 5 | Unique tier: one-of-a-kind creatures in one-of-a-kind places | v4, notable Pokemon v1 | Parked |
| 6 | The Pokedex records that you found a Pokemon, not where | v1 | Parked |

## Rules for rare finds

1. **No guaranteed place and no schedule.** A rare Pokemon is never "at the end of Route X".
2. **Clues, not markers.** Tracks, sounds, weather and rumors, never an icon or a location page.
3. **Conditions caused by the world.** If a condition is needed, the world produces it (weather, tide, time, food, who else is nearby). Nothing is timed to the player.
4. **A specific individual.** Very Rare and above are tracked individuals. Finding one means finding that one. Catching it removes it from the wild.
5. **Not gated by story or badges.** Anyone can find anything if they notice the right thing.
6. **No fake pity.** No invented guarantee after many tries.

## Rarity tiers: planning targets (not measured)

These numbers are AI estimates, set by the owner's delegation. The actual species count and list have not been checked in the repo. Starting figure: about 1,025 species through Generation 9.

| Tier | Species | Share | Meaning |
|---|---|---|---|
| Common | about 245 | 24% | Everywhere in their habitat. Mostly early evolution stages |
| Uncommon | about 270 | 26% | Specific biomes. Mid stages and many final forms |
| Rare | about 230 | 22% | A few spots per region. Shy or territorial |
| Very Rare | about 120 | 12% | Hard-to-reach habitats. A handful in the world. Tracked individuals |
| Special | about 60 | 6% | Found by a condition or clue chain, not by a place and a roll |
| Unique | about 100 | 10% | Legendaries and Mythicals. Exactly one each, in one place |

- Split mostly by evolution stage and habitat. Early stages are common and strong final forms are rarer.
- A region's native roster is roughly 100 to 180 species, so most species live in about two regions.
- No more regions are needed. The 14 biomes cover every type. Habitats inside a region (coast, river, cave, canopy) can host more species.
- About 510 species are Rare or above. Tracking them all as named individuals will not fit the measured RAM. v1 tracks a handful of Unique Pokemon and keeps the rest abstract until met.

## Special tier: the idea

A Special Pokemon is found because of something you noticed or caused, not because you reached a spot. Examples of the kind of thing (not a final list): a Pokemon that appears only in a certain weather or tide, one that only shows up near a certain food, one that hides until you are still, one said to appear before a disaster.

## Open questions

- Whether a Unique Pokemon can ever be permanently missed by ordinary play, and what "fled" means.
- Which actual species are Special, decided later with the real species list.
- How Very Rare individuals are placed without a visible spawn table.
