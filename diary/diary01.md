# Diary 01 (logs 1 to 10)

The story of the project in order. Short on purpose. Dates are by session. For the routine, see `ONBOARDING.md`.

## Log 1: A Pokemon game on the pret decompilation (2026-11-04)
The owner wanted to build a custom Pokemon game from the decompiled Emerald source. We forked the project into a repo called `pokeclaude` and the AI read its structure: scripts, maps, Pokemon data. Nothing was wiped. A clean slate would have broken the build.

## Log 2: Open and not story-first (2026-11-04)
The owner set the direction: not a story-first game, no linear path, explore anywhere. We rejected difficulty scaling in favor of a "natural" world where strong, average and weak things simply exist, trainers may be declined, and gyms are sought out and read your badges.

## Log 3: A living world (2026-11-04)
Big ideas were added: every notable Pokemon is a specific individual that faints, recovers, grows and remembers (inspired by Rain World), everyone eats berries and food, NPCs live their own lives, ledges can be climbed, and Pokemon can be ridden on land, water and air. Danger comes from the world (storms, whirlpools, predators), not from artificial rarity.

## Log 4: Fresh world, all species (2026-11-04)
We dropped the official regions and settled on 14 original ones: gentle Meadowmere as the start, Neonport as a hub, and the near-uncivilized Ur-Kalah. All 1,000+ species would come through the community `pokeemerald-expansion` project, so we moved to a fork of it and the design docs moved with us.

## Log 5: We over-specified, then cleaned up (2026-11-05)
Problem: dozens of detailed specs had been marked "locked" before any build existed or any memory was measured, mostly written by the AI. An audit demoted them. We created `design/STATUS.md` as the single source of truth and marked 20 speculative specs as archived and off-limits in `AGENTS.md`. Mistake along the way: pull requests were opened against the public upstream project by accident. The fork-only rule was added.

## Log 6: Gate A, the first real numbers (2026-11-05)
The unchanged fork compiled and its tests passed in CI. The build log gave real limits: about 35 KB of EWRAM and 4 KB of IWRAM free, and about 6.5 MiB of ROM free. RAM, not ROM, is the tight resource, which supported keeping the heavy simulation ideas aspirational. An emulator smoke test is still pending.

## Log 7: The core, and version ladders (2026-11-05)
The owner restated the core in five lines (not story locked, not battle locked, semi survival, explore free, doesn't revolve around the player) and named the risk: scope creep and production-level systems with no path to get there. Result: `design/CORE.md`, the rules "no respawn" and "direct predecessor", and the method of version ladders, where every feature is a table of versions and only the Next one is specified. Eleven ladders were drafted in `features/`.

## Log 8: How we work (2026-11-05)
The owner disliked pull requests for every docs change, so docs now go straight to `master` while code uses a branch. We added `PLACES.md` (where things are), `OPERATING.md` (how we work together), `ONBOARDING.md` (the session routine) and this diary, replacing the earlier `LOG.md`.

## Log 9: The routine can be corrected (2026-11-05)
The owner added a rule: if an AI notices the onboarding routine does not match how we really work, it tells the owner first and fixes the process once they agree, so a wrong process is not passed on to future AIs. Added to `ONBOARDING.md` (section 4) and `AGENTS.md` (rule 11). It covers process files only. The core, ladders and locked decisions still need the owner's explicit decision.

## Log 10: World first, and rare finds (2026-11-05)
The owner chose to work on the world first, starting at a high level. The AI sketched a world layout (8 walkable mainland regions, 3 sea regions, 2 layers above and below, and Ur-Kalah at the edge), still a sketch to be confirmed. Asked to estimate how 1,000+ species spread across rarity tiers, the AI proposed six tiers (Common to Unique, about 1,025 species, estimates only) and concluded no more regions are needed. The owner wants rare finds to feel like finding a shiny or an easter egg, not a guaranteed stop, so a new ladder, `features/rare-finds.md`, holds the rules: no guaranteed place or schedule, clues instead of markers, conditions caused by the world.

## State (as of log 10)

**Open**
- Owner to confirm the world clock policy (in-game time only, no offline catch-up).
- Emulator smoke test to close Gate A.
- Confirm upstream pull request #11032 is closed.
- Owner to confirm the world layout sketch, and how continuous the mainland is (walkable end to end, or broken by barriers).
- Rarity tier numbers are AI estimates. The real species count and list are unchecked.
- Decide which regions have gyms.
- Ladder rows were drafted by the AI and need owner review.
- `design/trainers-and-gyms.md` still needs the "minority are persistent battlers" line.

**Next**
- Confirm the world layout, then write region profiles, starting with Meadowmere. Review the feature ladders and promote the first row to Next.

_This page is full (logs 1 to 10). The next log goes on `diary02.md`, with this State block copied forward._
