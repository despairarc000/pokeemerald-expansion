# Traversal

**Core line:** Explore free

| Ver | Adds | Requires | Status |
|---|---|---|---|
| 1 | Ledges can be climbed up and down by the player | nothing | Next |
| 2 | Climbing costs stamina | v1, hunger v1 | Parked |
| 3 | Pokemon help with climbing and swimming | v2, party Pokemon traits | Parked |
| 4 | Ride a Pokemon on land and water | v3 | Parked |
| 5 | Free flight on a Pokemon | v4 | Parked |
| 6 | Hazards affect travel (falls, whirlpools, storms) | danger and hazards v2, v2 | Parked |

## Next version spec (v1)

- One-way ledge tiles become climbable in both directions, with a short animation.
- Player only, no stamina, no Pokemon involvement.
- **Check what already exists first.** The engine defines ledge jump behaviors (`MB_JUMP_*`), `MB_ROCK_CLIMB`, `MB_ROCK_STAIRS` and `MB_LADDER` (see `include/constants/metatile_behaviors.h`). How they work in code has not been checked, so the amount of new engine work is unknown. It may be smaller than first assumed.
- Check any change against the RAM numbers.

## Notes and parked ideas

- Good mounts are rare because of where they live, not because of a quota.
- Background design is in `design/traversal.md`, `design/riding-and-mounts.md` and `design/hazards.md`.
