# Technical feasibility audit: initial repository inspection

**Date:** 2026-11-04 (CI result and measurements added 2026-11-05)  
**Status:** SOURCE-INSPECTED. CI BUILD AND TEST PASSED. ROM/RAM NUMBERS RECORDED. EMULATOR SMOKE TEST STILL PENDING.  
**Baseline commit:** `173dd8b43b9c578c68792b3cab9ee6eae1c8587c`  
**Scope:** Actual fork source and configuration; this is not a claim of functional living-world implementation.

## Confirmed source facts

| Area | Evidence | Implication |
|---|---|---|
| Toolchain | `.github/workflows/build.yml` uses ARM GCC/binutils, Python, libpng and `make`; Emerald/FRLG/test jobs are defined | Build pipeline exists and ran (see Gate A result below) |
| Species | `include/config/species_enabled.h`: `P_GEN_1_POKEMON` through `P_GEN_9_POKEMON` all `TRUE` | Broad generation support configured; exact species count not measured |
| Visible Pokemon graphics | `include/config/overworld.h`: `OW_POKEMON_OBJECT_EVENTS TRUE` | Necessary sprite infrastructure enabled |
| Generated overworld wild encounters | `include/config/wild_encounter.h`: `WE_OW_ENCOUNTERS FALSE` | Built-in encounter system exists but is currently off |
| Vanilla random encounters | `WE_VANILLA_RANDOM TRUE` | Existing random encounters remain on |
| Overworld graphics compression | `OW_GFX_COMPRESS TRUE` | OWE config warns about VRAM pressure; compare compression modes |
| Overworld encounter code | `src/wild_encounter_ow.c` exists | Inspect/reuse before writing new spawn and battle-transition code |
| NPC followers | `include/config/follower_npc.h`: `FNPC_ENABLE_NPC_FOLLOWERS FALSE` | Optional feature, not active |
| Map data | `data/maps/` lists 941 entries (includes non-playable metadata entry) | Numerous existing maps; does not establish capacity for 14 custom regions |
| Map structures | `include/global.fieldmap.h` defines `MapLayout`, `MapEvents`, `MapConnection` and object-event templates | Supports map-based geography; actual dynamic map/object ceilings unmeasured |
| Save config | `include/config/save.h`: all `FREE_*` reclaim toggles are `FALSE` | Optional save-space reclamation exists, but no free-byte amount established |
| Save structure audit | `design/root/save-budget.md` records sector containers and compile-time asserts | Container capacities are **not** unused capacity |

## Gate A result (2026-11-05)

GitHub Actions run `41925848317` on pull request #1 (head `65946109a93431eb324f709da0b56fb0f0dcff92`). That commit changes documentation only, so the compiled code is the unchanged baseline.

| Job | Result |
|---|---|
| build-emerald | success |
| build-firered | success |
| build-leafgreen | success |
| test | success |
| release | success |
| docs_validate | success |
| build | success |

**Measured from the build-emerald linker output** (full table and analysis in `design/root/baseline-measurements.md`):

| Region | Used | % of region | Free |
|---|---|---|---|
| EWRAM | 226,420 B of 256 KB | 86.37% | 35,724 B |
| IWRAM | 28,392 B of 32 KB | 86.65% | 4,376 B |
| ROM | 26,752,416 B of 32 MB | 79.73% | 6,802,016 B |

**Gate A is MOSTLY met.** The unchanged fork compiles, its tests pass, and ROM/RAM usage is recorded. The one item left is an **emulator smoke test** (movement, an ordinary wild battle, save and reload), which needs a run of the built ROM.

## Immediate engineering implications

1. **RAM is the tight resource.** About 35 KB of EWRAM and 4 KB of IWRAM are free in the default configuration. Budget every runtime structure against that.
2. **Do not rewrite overworld encounters first.** Inspect `src/wild_encounter_ow.c`, enable `WE_OW_ENCOUNTERS` on a controlled test branch, and test sprite/VRAM behavior.
3. **Retain traditional battle compatibility.** Verify generated overworld Pokemon initiate and return from battles correctly; then identify extension points for observing, feeding and identity reconciliation.
4. **Measure rather than guess.** Next measurements: `sizeof` save structures, object-event occupancy, and the effect of disabling unused features.
5. **Do not confuse feature flags with complete functionality.** Generation toggles and sprite infrastructure are evidence of available code paths, not proof of performance, content coverage or living-world AI.
6. **Separate baseline from custom work.** Existing Expansion mechanics are foundation; independent ecology, tiered individual persistence, autonomous NPC lives and world simulation still need implementation.

## Test sequence and acceptance

### Gate A: reproducible baseline
- Compile unchanged fork; record commit, ROM size and linker memory usage. **Done (CI).**
- Run smoke test in a GBA emulator, including movement, ordinary wild battle and save/load. **Pending.**
- Record pass/fail from the real run, not inferred success.

### Gate B: existing OWE system
- Enable `WE_OW_ENCOUNTERS` in an isolated branch; compare `OW_GFX_COMPRESS` settings.
- Verify Pokemon appear, move, engage, flee and resolve a traditional battle without crashes.
- Count simultaneous active objects, sprites and memory use; repeat across map transitions, and compare against the baseline numbers above.

### Gate C: persistence
- Measure `SaveBlock1`, `SaveBlock2`, `SaveBlock3`, `PokemonStorage` with concrete build settings.
- Prototype one persistent notable wild Pokemon through unload/reload/save/load/capture.
- Assert one canonical identity, no duplication or unintended disappearance.

### Gate D: first living-world slice
- Compact Meadowmere map and one connected route.
- At least one observable wild Pokemon, a meaningful noncombat interaction, one battle/capture, one independent NPC, and a save/reload check.
- Demonstrate more than one legitimate player path without a compulsory badge.

## Open blockers

- No emulator smoke test yet.
- Exact free save bytes unknown.
- Max practical simultaneous wild/NPC objects unknown.
- Real OWE battle lifecycle and integration seams not yet traced.
- Species count and sprite asset completeness not yet verified.

**Next action:** run the emulator smoke test to close Gate A, then start Gate B. Do not promise all 14 regions or the full ecosystem before those results exist.
