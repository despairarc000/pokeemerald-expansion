# Status of design decisions

**This file is the single source of truth for decision status.** If a document under `design/` or `design/root/` says "LOCKED" and this file says otherwise, this file wins. Older per-file "locked" labels are being retired as files are touched.

Why this exists: many systems were marked locked before the baseline build ran and before save space was measured. A decision that depends on unmeasured limits cannot be final.

## Status levels

| Level | Meaning |
|---|---|
| **Locked** | A principle or pillar that does not depend on measurement. Changing it needs an explicit decision by the project owner. |
| **Proposed** | A design direction we intend to pursue. Details and feasibility depend on Gates A to D (see `technical-feasibility-audit.md`). |
| **Aspirational** | A good idea, not committed. Revisit only after the first vertical slice proves the budget. |
| **Open** | Not decided. |

## Locked

| Decision | Detail doc |
|---|---|
| Codebase is this `pokeemerald-expansion` fork (no rebase) | `overview.md` |
| 14 original regions, open world, no compulsory story or badge gating | `regions.md`, `root/hybrid-world-geography.md` |
| Safe by default, dangerous by choice. Danger is signaled and avoidable. No ambushes | `danger-and-ecosystem.md` |
| Balanced sandbox: exploration, collection, battles, bonding, survival and world participation are all legitimate paths | `root/balanced-pokemon-sandbox.md` |
| Parallel progression, no single mandatory ladder. Gyms are optional | `root/parallel-player-progression.md` |
| Wild Pokemon are visible; traditional battle and capture stay compatible | `root/hybrid-wild-encounters.md` |
| Tiered persistence: notable individuals are tracked, ordinary populations are abstracted | `root/individual-persistence.md` |
| Everyone eats. Hunger never kills permanently and never erases a save | `survival.md` |
| Climbable ledges, Pokemon-assisted traversal, mounts with stamina, hazards as the balance (not artificial rarity) | `traversal.md`, `riding-and-mounts.md`, `hazards.md` |
| Process: `STATUS.md` is the status source of truth; 20 speculative specs are archived off-limits via `AGENTS.md` | `AGENTS.md` |

## Proposed (needs measurement)

| Decision | Detail doc | Waiting on |
|---|---|---|
| World clock: in-game time only, no offline catch-up | `root/world-clock.md` | **Owner confirmation**, then RTC/berry/weather integration |
| Utility-based wild AI | `root/utility-ai.md` | Gate B |
| Hybrid navigation for Pokemon | `root/hybrid-navigation.md` | Gate B |
| Sensory perception for wild Pokemon (simple version first) | `root/multi-sensory-perception.md` | Gate B |
| Off-screen scheduler, catch-up, adaptive priority | `root/offscreen-scheduler.md`, `root/hybrid-catchup.md`, `root/adaptive-scheduling.md` | Gates B and C, and the RAM budget |
| Population ecology: abstract births, decline, recovery | `root/population-ecology.md`, `root/hybrid-reproduction.md` | Gate C |
| Ecological migration | `root/ecological-migration.md` | Map topology |
| Shared NPC and Pokemon events | `root/shared-world-events.md` | Gate C |
| Tiered NPC persistence | `root/npc-persistence.md` | Gate C |
| NPC routines with utility interruptions | `root/npc-routine-utility.md`, `living-npcs.md` | Gate D |
| Per-region gym circuits and leader lore | `trainers-and-gyms.md` | Which regions have gyms |

## Aspirational (after the first slice)

| Idea | Detail doc |
|---|---|
| Persistent family trees for notable Pokemon | `root/hybrid-reproduction.md` |
| Pokemon social memory | `root/social-memory.md` |
| Evolving Pokemon personality | `root/evolving-personality.md` |
| NPC social memory | `root/npc-social-memory.md` |
| Evolving NPC personality | `root/npc-evolving-personality.md` |
| NPC perception | `root/npc-contextual-perception.md` |
| NPC navigation across the world graph | `root/npc-hybrid-navigation.md` |
| NPC information sharing and rumors | `root/npc-information-sharing.md` |
| Dynamic NPC small groups | `root/npc-dynamic-groups.md` |
| NPC long-term goals | `root/npc-long-term-goals.md` |
| Hierarchical NPC planning | `root/npc-hierarchical-planning.md` |
| NPC resource management | `root/npc-resource-management.md` |
| Regional dynamic economy | `root/regional-dynamic-economy.md` |

These were written as locked decisions. They are well specified, but they stack a lot of simulation on a GBA save and RAM budget. Measured baseline headroom (see `root/baseline-measurements.md`) is about 35 KB of EWRAM and 4 KB of IWRAM, which supports keeping them aspirational.

## Open

- Free save bytes (flash sectors), individual record layout, population caps, day length.
- Object-event and sprite limits (Gate B).
- Which of the 13 non-Ur-Kalah regions have gyms.
- Species and habitat tagging, map topology and layouts.
- Free flight implementation and player-facing menus.

## Gate progress

| Gate | State |
|---|---|
| A: reproducible baseline | **Mostly met.** CI compile and tests pass on the unchanged fork (run `41925848317`). Measured: EWRAM 226,420 B (86.37%), IWRAM 28,392 B (86.65%), ROM 26,752,416 B (79.73%). Emulator smoke test still to do. See `root/baseline-measurements.md`. |
| B: existing overworld encounters | Not started |
| C: persistence and save measurement | Not started |
| D: Meadowmere vertical slice | Not started |

## Next action

Close Gate A with an emulator smoke test (movement, a wild battle, save and reload) on the built ROM. Then Gate B. Do not build against any Proposed or Aspirational item before the gate it waits on passes.
