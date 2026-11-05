# Wild Pokemon

**Core line:** Not battle locked

| Ver | Adds | Requires | Status |
|---|---|---|---|
| 1 | Visible wild Pokemon on the map. Touching or approaching one starts a normal battle | Gate B (existing overworld-encounter code works) | Next |
| 2 | Simple behaviors: passive, skittish, territorial | v1 | Parked |
| 3 | Pokemon react to you: flee, approach, calm down when fed | v2, hunger v1 | Parked |
| 4 | Notable individuals persist (see Notable Pokemon) | notable Pokemon v1 | Parked |
| 5 | Predators and prey interact with each other off-screen | v2, world time v1 | Parked |

## Next version spec (v1)

- Use the overworld-encounter code Expansion already has (`WE_OW_ENCOUNTERS`, currently off).
- Prove on a test branch that visible Pokemon appear, move, start a normal battle, and return to the map.
- No new behaviors yet.

## Notes and parked ideas

- Avoiding a wild Pokemon must always be possible.
- Placement follows habitat, not quotas (design background in `design/hazards.md`).
