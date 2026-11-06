# Diary 02 (logs 11 to 20)

The story of the project in order. Short on purpose. Dates are by session. For the routine, see `ONBOARDING.md`. Earlier history is in `diary01.md`.

## Log 11: The world should feel real (2026-11-05)
The owner clarified what "free exploration" means for the map: the world should feel like the real world, not like Pokemon-coded maps. Terrain restricts movement because of what it is (a dense forest is slow, a waterfall drop is insane), never to guide or gate the player. We recorded this as "world feel rules" in `features/regions-and-exploration.md`: barriers are costs, not locks, no cut-tree or boulder gates, paths follow valleys and rivers, settlements sit where people would live. The mainland is continuous, with natural barriers that cost something.

## Log 12: Fix shared-edge map connection boundary (2026-11-06)
The owner asked to fix the Emerald engine's off-by-one connection lookup directly on `master`, without a branch, so it would not be forgotten during mobile map-editor work. In `src/fieldmap.c`, `IsCoordInIncomingConnectingMap` now treats the upper bound as exclusive (`coord < max`), matching the engine's other connection range check. Example: a 10-high map with two 5-high neighbors on the right now selects the first for rows 0–4 and the second for rows 5–9 without double-matching row 5. Engine commit: `eeec45dc613d8486c3cf1639619ce8e1a9f466a6`. Boundary-case logic was checked; native build and emulator behavior are **not yet verified**. This direct-to-master engine fix was an explicit owner-requested exception to the usual branch rule.

## State (as of log 12)

**Open**
- Owner to confirm the world clock policy (in-game time only, no offline catch-up).
- Emulator smoke test to close Gate A.
- Verify the shared-edge connection fix in a native build/emulator when the map-editor implementation is tested (engine change is committed, not yet gameplay-tested).
- Confirm upstream pull request #11032 is closed.
- Owner to confirm the world layout sketch (diary log 10) and whether the "world feel rules" become a core design rule in `design/CORE.md`.
- Rarity tier numbers are AI estimates. The real species count and list are unchecked.
- Decide which regions have gyms.
- Ladder rows were drafted by the AI and need owner review.
- `design/trainers-and-gyms.md` still needs the "minority are persistent battlers" line.

**Next**
- Write region profiles, starting with Meadowmere, following the world feel rules. Review the feature ladders and promote the first row to Next.
