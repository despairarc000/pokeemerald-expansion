# Map Studio — Source-Grounded Feature Gap Audit (v0.16)

Reviewed source: this fork's `data/maps/LittlerootTown/map.json`,
`data/maps/OldaleTown/map.json`, `data/maps/PetalburgCity/map.json`,
`include/constants/event_object_movement.h`,
`include/constants/event_bg.h`, `include/constants/map_scripts.h`,
`include/constants/map_types.h`, plus current
`tools/map-studio/src/{model,ui,storage,export,android_export,choices,tiles}.rs`
and `src/bin/cli.rs`. This is an **editor-feature audit**, not a claim
that a generated GBA ROM has been boot-tested.

## Support matrix

| Engine/source feature | Map Studio status | Source-backed gap or limitation |
| --- | --- | --- |
| Metatile catalog & native metatile IDs | **Partial** | Source images for 129/137 layout-declared pairs; remaining pairs not verified. Per-metatile behavior and drawing layer are read-only; normal/split/covered composition parity needs game comparison. |
| Paint, rectangular stamp, fill, undo | **Implemented editor operations** | Canvas gestures require device QA; no autotiling or intelligent corner/roof/edge stamps. |
| Collision and elevation | **Implemented** | Source 0–3 collision data preserved but ordinary brush exposes only 0/1. Legacy 2/3 appear only as read-only color overlays; elevation remains 0–15 and affects map-block packing. |
| Multiple maps, placement, resize, connections | **Partial** | Connection export uses adjacent same-Space maps. Space labels are editor-only, not GBA map groups or continuous game coordinates. |
| Door/warp/coordinate travel | **Partial** | Native numbered warps, arrival markers and direct script `warp` supported. Manual linking and target validity exist; no end-to-end emulator verification. |
| NPC graphics | **Partial** | Preview subset is limited to source-verified sprites, not every object graphic. |
| NPC movement | **Limited native support (v0.16)** | A verified subset of `MOVEMENT_TYPE_*` and range X/Y 0–15 now serializes and exports. No live NPC movement simulation, patrol route authoring, animations, collision/pathfinding tests or custom sequences. |
| NPC dialogue | **Limited** | One short ASCII message per NPC. No branching, text codes, variables, shops, giveaways, trainer challenges or general event scripts. |
| NPC trainer & scripted events | **Missing** | Engine object events have `trainer_type`, `trainer_sight_or_berry_tree_id`, flags and scripts; exporter currently sets trainer none, script from a generated message and flag 0. Source: `data/maps/PetalburgCity/map.json` and `data/maps/LittlerootTown/map.json`. |
| Conditional event visibility / object local IDs | **Missing** | Native `flag` and optional `local_id` controls are not authored. Special on-load object placement and hide/show flags not supported. |
| Sign interaction direction | **Missing** | `BG_EVENT_PLAYER_FACING_ANY/NORTH/SOUTH/EAST/WEST` exist in `include/constants/event_bg.h`; exporter currently hardcodes ANY. |
| Background events & special zones | **Partial** | Simple signs and one-time hidden items supported; `BG_EVENT_SECRET_BASE` is not. No ordinary source-script background links. |
| Coordinate triggers | **Limited** | Simple immediate messages and direct warp scripts only; native conditional `var`, `var_value`, distinct scripts and trainer/plot conditions not editable. See `data/maps/OldaleTown/map.json`. |
| Map-level scripts | **Missing** | `MAP_SCRIPT_ON_LOAD`, `ON_TRANSITION`, `ON_FRAME_TABLE`, `ON_WARP_INTO_MAP_TABLE`, `ON_RESUME`, `ON_DIVE_WARP`, `ON_RETURN_TO_FIELD` are defined in `include/constants/map_scripts.h`; exporter writes a basic empty MapScripts table. |
| Map header / gameplay metadata | **Missing** | Export currently hardcodes Littleroot music, weather sunny, map type town, section Littleroot, cycling/running/escaping flags, region, battle scene, flash status. The source map JSON includes individual fields; cannot claim new interiors/caves have correct game metadata. |
| Sound/music, encounters, weather & visual effects | **Missing** | No map-specific editing of encounters, sound, ambient weather, script-specific effects or wild Pokémon tables. Metatile behaviors alone do not supply all such content. |
| Project autosave and lossless native editing format | **Implemented** | Versioned JSON project with atomic saves, backup-before-import and integrity validation. Cross-install app-private data not shared automatically. |
| Portable editable project import/export | **Improved in v0.16** | Editable ZIP contains `mapstudio-project.json`, can be imported even when maps are unfinished. Existing game-source ZIP embeds project snapshot as well. Restore checks schema/integrity, saves current work to a recovery file first. |
| Android file picker / arbitrary outside documents | **Not implemented** | Android import currently lists this installation's readable named files in `Downloads/MapStudio` via MediaStore. This is **not** an Android Storage Access Framework document picker and cannot promise access to another installation's private or protected data. |
| Import native source maps from `data/maps/*` | **Missing** | Import expects a Map Studio project JSON or ZIP containing `mapstudio-project.json`. It does not reconstruct an existing Emerald layout, scripts and objects from arbitrary native map JSON + bin files. |
| Source ZIP export & Python map integration | **Partial** | Source ZIP contains custom map JSON/layout/scripts and project snapshot; validation catches unpainted tiles and unsupported events. Actual ROM compilation and playable emulator smoke test are separate gates, not established by Android APK CI. |
| GitHub sync / collaborative editing | **Missing** | No authenticated GitHub writeback, conflict handling, live coediting or stable release signing. |

## Prioritized engineering backlog

1. **P0: Import UX and save portability.** Add Android SAF `ACTION_OPEN_DOCUMENT` integration with a reliable activity-result bridge; allow explicit user-selected JSON/ZIP from other preview apps. Keep schema validation, undo/recovery, atomic replacement and size bounds. Do not fake a picker if the native bridge is unavailable.
2. **P0: Stable project continuity.** Replace ephemeral CI signing with a managed signing setup before claiming upgrade-in-place. Add device QA for portrait, freeform/split-screen, bottom gesture-bar and touch scrolling.
3. **P1: Complete source map metadata.** Use the fork's source constants to offer valid `map_type`, `weather`, `music`, `region_map_section`, flash, bike/run/escape, scene settings. Default from verified source templates, not an arbitrary "town" value for every exported map.
4. **P1: Object event fidelity.** Source-backed local IDs, flags, trainer definitions, visibility, sprite fields, movement patterns and scripts; export only options actually implemented/verified.
5. **P1: Coordinate/background/script fidelity.** Real conditional triggers, script-variable values, sign facing direction, secret bases and map-level hooks, plus script references with safe validation.
6. **P1: Source map import.** Explicit import of native Emerald maps from their layout/map/script components with round-trip tests; do not confuse art tileset selection with importing a playable town layout.
7. **P2: Graphics parity & UX.** Validate primary/secondary atlas layer composition against emulator; implement optional smart multi-metatile structures; improve visual hitboxes with device tests.
8. **Gate A: playable ROM smoke test.** Validate a generated map ZIP through the actual map compiler and GBA emulator on a checked build before marking game export production-ready.

## Explicit safety and integrity boundaries

- No removal or migration of existing 2/3 collision values in old projects.
- No invented Pokémon event behaviors, weather values, trainer state or game map connections.
- The editor can modify a valid project even when a source/game export is not yet possible.
- Android downloaded ZIPs are not self-installing ROMs. APK compilation alone does
  not validate gameplay exports.
