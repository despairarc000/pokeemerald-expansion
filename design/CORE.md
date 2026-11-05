# Core Game

**Read this first.** Every feature, system and map must serve this core. If a design conflicts with it, the design changes, not the core. Changes to this file need an explicit decision by the project owner.

## The core, in five lines

1. **Not story locked.** No compulsory plot.
2. **Not battle locked.** Battles are one way to play, not the only one.
3. **Semi survival.** Managing how you live matters, but it never ends the game.
4. **Explore free.** Go anywhere, in any order.
5. **Doesn't revolve around the player.** The world lives on without you.

## What sits under each line

### Not story locked
- There is lore, but it is never forced on you. You find it, or you don't.
- No required order of towns, gyms or events.

### Not battle locked
- Most trainers can be declined with no penalty.
- A minority are persistent battlers. They may keep pursuing a fight, but they are the exception, not the rule.
- Wild Pokemon are visible. You choose whether to fight, avoid, feed or befriend them.
- Battling, collecting, caring, exploring and surviving are all legitimate ways to play.

### Semi survival
- You do not die. Starving or exhausted means weakness and collapse, never a lost save.
- You do need to manage your way of life: food, rest, supplies, and your party's needs.
- Safe places are easy to live in. Dangerous places demand preparation.

### Explore free
- You can go wherever you want. You just need to find a way to do so.
- Finding a way is part of the game: climbing, swimming, riding, boats, tunnels, asking locals, noticing signs.
- No quest markers. Discovery over direction.

### Doesn't revolve around the player
- NPCs do their own thing: homes, routines, wants, travel.
- Pokemon are individuals with territories, needs and histories. Notable ones remember you.
- Gym leaders can simply be elsewhere. Nothing is scripted to wait for you.
- The ecosystem, weather and time keep going whether or not you are watching.

## Under those (the feature docs)

| Core line | Main feature docs |
|---|---|
| Not story locked | `regions.md`, `exploration.md`, `root/hybrid-world-geography.md` |
| Not battle locked | `trainers-and-gyms.md`, `root/hybrid-wild-encounters.md`, `root/balanced-pokemon-sandbox.md`, `root/parallel-player-progression.md` |
| Semi survival | `survival.md`, `danger-and-ecosystem.md`, `hazards.md` |
| Explore free | `traversal.md`, `riding-and-mounts.md`, `exploration.md` |
| Doesn't revolve around the player | `living-npcs.md`, `ecosystem-simulation.md`, `root/individual-persistence.md`, `root/world-clock.md` |

## Constraint

The real hardware leaves only about 35 KB of free EWRAM and 4 KB of IWRAM (see `root/baseline-measurements.md`). Every feature starts as a small v1 that serves one of the five lines. Richer versions come after the first playable slice proves the budget.

## Known reconciliation needed

- `trainers-and-gyms.md` says trainers "may challenge" and can be declined. The core adds that a **minority are persistent battlers**. That doc should be updated to match.
