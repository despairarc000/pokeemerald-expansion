# Features: version ladders

Every feature is a **ladder of versions**. Each version builds on the one before it. Nothing is deleted, and only one rung at a time is specified in detail.

Read `design/CORE.md` first. Each feature names the core line it serves.

## Rules

1. **Direct predecessor.** Every outcome needs a cause that already exists in the world. Nothing happens because the game needs it to.
2. **One rung at a time.** Only the **Next** version gets a detailed spec. **Parked** versions stay one-line sketches.
3. **Requires.** Each version lists what must exist first. If a requirement is not built, the version cannot start.
4. **No fake fallbacks.** If a version needs an invented safety net, it waits for the system that would cause it for real.
5. **Nothing is lost.** Old versions are marked Superseded, never deleted.
6. **Name the core line.** A feature that cannot name a core line waits.

## Statuses

| Status | Meaning |
|---|---|
| **Current** | Built and working. The truth about the game. |
| **Next** | The only version specified in detail. |
| **Parked** | A sketch, not committed. |
| **Superseded** | Replaced by a later version, kept for history. |

Nothing is built yet, so no version is Current.

## Template for a feature file

```
# Feature name

**Core line:** which of the five lines it serves

| Ver | Adds | Requires | Status |
|---|---|---|---|
| 1 | ... | ... | Next |
| 2 | ... | ... | Parked |

## Next version spec
Only for the Next row. Keep it short.

## Notes and parked ideas
One-liners.
```

## Index

| Feature | Core line | Next version | Current |
|---|---|---|---|
| [Fainting](fainting.md) | Semi survival | v1: no respawn, no game over | none |
| [Wild Pokemon](wild-pokemon.md) | Not battle locked | v1: visible Pokemon, touch to battle | none |
| [Rare finds](rare-finds.md) | Explore free | none promoted yet (all Parked) | none |
| [Survival and hunger](survival-hunger.md) | Semi survival | v1: player hunger states and berries | none |
| [Traversal](traversal.md) | Explore free | v1: climbable ledges | none |
| [Notable Pokemon](notable-pokemon.md) | Doesn't revolve around the player | v1: one remembered individual | none |
| [NPC life](npc-life.md) | Doesn't revolve around the player | v1: day and night positions | none |
| [Trainers](trainers.md) | Not battle locked | v1: sight challenge with decline | none |
| [Gyms and badges](gyms-and-badges.md) | Not story locked | v1: one optional gym | none |
| [Regions and exploration](regions-and-exploration.md) | Explore free | v1: compact Meadowmere | none |
| [Danger and hazards](danger-and-hazards.md) | Semi survival | v1: zones tagged Safe and Calm | none |
| [World time](world-time.md) | Doesn't revolve around the player | v1: in-game day and night | none |

Older detailed designs live in `design/`. They are background, not commitments. `design/STATUS.md` still describes their status.
