# Map Studio — implementation baseline for the Rust Android APK

**Prepared:** 2026-11-07  
**Target game repository:** [`despairarc000/pokeemerald-expansion`](https://github.com/despairarc000/pokeemerald-expansion), `master`  
**Status:** implementation-ready technical plan; **not** a claim that the APK, backend, or in-game export has been built. This is a tooling plan, not a promotion of any living-world game feature ladder or a change to game core rules.

## 1. Product contract

A portrait-first, offline-capable Android APK that lets humans and agents edit the *same structured Pokémon Emerald world*. The user sees a continuous white grid and draws non-overlapping rectangular maps (one editor grid cell = one native 16×16-pixel metatile). Map frames are tinted gray until painted, have distinct frame outlines, and acquire their native game connections automatically whenever they share an edge. No manual connection mode. The app's output is actual Emerald source-map data, **not** a rendered image or an APK-compiled ROM.

**Visual UX carried forward from HTML v3:** top app bar, grid canvas, bottom scrollable tile browser with ID/category/search sorting, three modes `Terrain | Properties | Events`, long-press/drag create, frame select/move/resize, pinch-to-zoom around the gesture, two-finger pan, faint map-name labels while editing, light property overlays with readable symbols, frame outlines always rendered last, no zoom +/- buttons. Symbols such as `0` (walkable), `1` (blocked), `~` (water), `❄` (ice) are **display categories**, not a replacement for Emerald's real numeric collision/elevation/tileset behavior fields. No made-up generic "Special" behavior.

## 2. Architecture decision

**Native Rust**: `eframe` + `egui` for app UI, a `wgpu`-backed canvas (start simple, use `egui-wgpu` callback with texture atlases/instancing when real tiles are integrated), and a platform-independent Rust core for all durable operations. **Not Tauri/WebView**. Native Android packaging first via `cargo-apk`/NativeActivity; a small Android bridge (Kotlin/Java/JNI) is acceptable for OS file pickers and lifecycle issues—Rust handles map logic. Pin a compatible set of crate versions, toolchains, and Android SDK/NDK in CI after the smallest device APK compiles; avoid hard-coding versions based only on docs.

Suggested location **inside** the fork, isolated from the GBA build:

```
tools/map-studio/
  Cargo.toml                  # Rust workspace
  crates/core/               # world, frames, cell edits, commands, undo, save schema
  crates/emerald/            # import, tilesets, metatile renderer data, export, validation
  crates/render/             # visible tile rendering / GPU atlas + overlays
  crates/app/                # eframe Android UI, gestures, Android file bridge
  crates/cli/                # same Rust operations for AI/CI
  tests/fixtures/            # known Emerald maps, source-output golden files
```

`crates/sync/` and a live-service binary are later additions; do not make them prerequisites for the offline APK. Build a **separate** Android CI workflow that produces an installable debug APK (`arm64-v8a` first), without running the full Emerald ROM compiler on every editor-only change.

## 3. Canonical map data model and constraints

The **Rust document model** is the sole authority for editing, previews, autosave, CLI commands, sync operations, and export. Native `.mapstudio` project files serialize this model with a schema version and stable IDs. Recommended shareable representation: directory of UTF-8 JSON files (`manifest.json`, one numeric row-major map file per frame, `events`/metadata fields), optionally packaged as `.mapstudio` ZIP when transferring from Android. Separate map files minimize GitHub conflicts. Do not store the only source copy as PNG, ROM, or generated `map.bin`.

- World: schema_version, project_id, tileset catalog/version, maps indexed by stable map_id, revision/metadata.
- Frame: map_id, Emerald name and group/section, integer editor origin `(x,y)`, positive dimensions `(width,height)`, primary/secondary tileset identifiers, border metatiles, native map metadata, event records.
- Cell: `None` while gray/unpainted; otherwise `{metatile_id, collision, elevation}` with valid native ranges. Each map uses a row-major grid of `Option<Cell>` values with exactly width × height entries.
- Events: stable event IDs; native object/warp/coord/bg events and referenced script labels. Preserve optional imported fields and raw script definitions without falsely promising arbitrary script compilation.
- Metatile semantics: native Emerald map blocks are little-endian `u16`: `id & 0x3ff | (collision & 3) << 10 | (elevation & 15) << 12`. Behavior (ice, surfable water, ledges, grass, doors, currents, etc.) is an **attribute of the chosen metatile in a tileset**, not a per-cell flag. Renderer decodes actual tilesets, subtiles, palettes, flipping and layers. Houses, trees and bridges are *multi-metatile stamps* rather than fictional one-cell images.

**Editing rule:** reject overlapping frame interiors. Corner-only contact does not connect maps. Internally store editor-world coordinates as signed, overflow-checked integers, without implying that Emerald has its own global world coordinate system. Validate all map limits before exporting; the engine's `MAX_MAP_DATA_SIZE` is 10,240 buffered cells, not a 10,240-metatile playable-map promise. Unpainted cells must be filled with an explicitly chosen *real metatile* or block export; never silently substitute tile 0.

## 4. From input to export: exact processing

```
Touch/AI operation
  -> Rust command validation (frame bounds, tileset, revision)
  -> authoritative Rust document + reversible operation history
  -> live egui/wgpu redraw (visible region only)
  -> debounced local autosave to app-private storage (atomic write/recovery)
  -> [optional future live WebSocket replication]
  -> user selects Export / Publish
  -> deterministic native game-source generator
  -> repo custom_maps/ and map registration pipeline
  -> separate Emerald ROM build/test (GitHub Actions or local PC)
```

Commit logical brush strokes as **batched** operations, not a disk write or network message per pixel/gesture sample. `PaintCells`, `MoveFrame`, `ResizeFrame`, `CreateFrame`, `SetCellProperties`, `PlaceEvent`, `EditEvent` are examples. Undo/redo records inverse commands/patches, not full-world snapshots. Autosave 1–2 seconds after a logical edit, on backgrounding, and on explicit Save; fsync/atomic replacement or journal + recovery to avoid truncated projects. Autosave does not mean ROM export.

### Automatic native connections

Treat frames as half-open rectangles `[x,x+w) × [y,y+h)`. Two frames connect iff exactly one pair of sides touches and their projection onto the other axis overlaps by a **positive** integer number of cells. Derive the offset from the relative top (for left/right) or relative left (for up/down), add the reverse, sort deterministically, and rerun when geometry changes.

**Fixture:** A=(0,0,10,10), B=(10,0,5,5), C=(10,5,5,5). Expected:

```
A -> B: right, offset  0      B -> A: left, offset  0
A -> C: right, offset +5      C -> A: left, offset -5
B -> C: down,  offset  0      C -> B: up,   offset  0
```

The earlier inclusive-end defect in `src/fieldmap.c` (`coord <= max`) was fixed to `coord < max` on `master` (commit `eeec45dc...`), but this alone **does not prove** tiny maps work. `FillNorthConnection` copies 7 neighboring rows and `FillEastConnection` copies 8 columns; a 5×5 neighbor can be unsafe. Verify/fix the camera buffer copying and tests **before shipping 5×5 game export as supported**. No extra normal-user connection warnings or manual offset controls; a build/export validation report may explain unexportable data.

### Native file generation

Read the fork's existing `custom_maps/` structure and `tools/textmap/build_textmaps.py`, which is invoked by `map_data_rules.mk`. The exporter emits **structured numeric layouts** (e.g. `layout.mapstudio.json`), complete `map.json` with generated `connections`, event definitions, and `scripts.inc` references. Extend the existing generator to accept the numeric layout alternative **without breaking `layout.txt`**; it produces `map.bin`, `border.bin`, `data/layouts/layouts.json`, `data/maps/map_groups.json`, map scripts includes, and other expected registrations. Do not commit generated binary files as canonical source. Native binary packing must round-trip byte-for-byte against known compatible fixtures.

For imported original Emerald maps, read the layout JSON, binary map/border, source JSON, tileset IDs and script files. Preserve unknown native metadata where feasible and explicitly classify any unsupported feature; no silent dropping of scripts/events. Multi-frame outdoor edge connections are automatic; **interiors/doors require event warps** and are not implied by two pictures touching on the canvas. Adjacent maps with different tilesets need a visual/game compatibility test across transition borders.

## 5. Online collaboration: intentional rollout

**V1: offline + GitHub shared checkpoints.** User edits and autosaves locally. GitHub Pull/Publish syncs the versioned source files *per map*, uses base commit SHA/concurrency checks and never force-pushes or overwrites a newer remote version blindly. GitHub is the readable, durable published history for humans and AI agents, not a real-time drawing protocol. Do not push every brush stroke. Scope authenticated repository permission minimally; never ship a private token/secret in the APK. An AI currently able to access the GitHub repository sees **published snapshots**, not private uncommitted APK memory.

**V2: true live collaboration.** Deploy an authenticated, persistent Rust WebSocket service; ordered operations (`operation_id`, `project_id`, `map_id`, `base_revision`, payload), idempotent acknowledgements, durable server event log/checkpoints, subscriber broadcasts, reconnect/delta recovery, collision policy, and leases or optimistic revision checks for frame moves/resizes. Same-tile concurrency has deterministic server order; different-cell changes coexist. Offline edits queue locally and reconcile on reconnect; conflicting same-cell histories must be inspectable/recoverable. GitHub receives deliberate batched snapshots **after** durable shared state, not after each message. A ChatGPT agent can make **live** edits only after the collaboration API is reachable through an authorized connector/tool; GitHub-only interactions remain snapshot/publish based. Never imply the connector is already installed.

## 6. Native Android UI behavior

- **Canvas**: pan/zoom in grid-world coordinates, pinch anchored at midpoint; two-finger gesture takes priority over any one-finger paint/create stroke; long press empty grid and drag draws rectangular frame; independent select, paint, erase, resize tools.
- **Visual hierarchy**: white fine grid → gray unpainted frames → real tile graphics → translucent property colors and centered symbols → map name at reduced opacity during editing → frame borders and handles at full opacity **last**. At low zoom, simplify detail but preserve frame bounds.
- **Tile palette**: actual GBA metatile thumbnails from the currently selected primary+secondary tilesets, searchable by ID and filterable by categories. Categories (houses, paths, etc.) are editor-added metadata; metatile IDs alone do not encode these human categories.
- **Terrain**: paint real metatiles and multi-tile stamps; **Properties**: collision/elevation and overlays for native behaviors; **Events**: typed native event placement and script references. No separate Connections tab. No zoom buttons.
- **Phone OS**: start without storage permissions using app-private storage; import/export ZIP through SAF `ACTION_OPEN_DOCUMENT`/`ACTION_CREATE_DOCUMENT` and `content://` URIs. Android lifecycle and IME require on-device acceptance testing, with minimal native bridge if necessary.

## 7. Performance and reliability

Target **60 FPS during ordinary pan/paint on a budget Android phone**, not a guarantee or achieved benchmark. Separate frame time into input, Rust edit, egui layout, canvas GPU, and I/O. Use viewport tile culling, tile atlases (nearest-neighbor filtering), draw batching/instancing, incremental dirty chunks, async/non-render-thread disk/network I/O and idle redraw throttling. No rebuilding all map geometry or reuploading all textures each brush stroke. Test warm/cold startup, switch-out/resume, device rotation policy, low-memory restart, corrupted autosave recovery, and offline export.

## 8. Executable delivery gates (strict order)

| Gate | Deliverable | Proof required before the next gate |
|---|---|---|
| **0 — Toolchain / APK spike** | Minimal eframe/wgpu APK, a static grid, buttons, Android CI debug APK | Installs/launches on target phone; proper resume; working text and multitouch; collect device GPU/backend/logs. If Vulkan fails, try GLES-compatible wgpu backend and measure; stop before full UI rewrite if neither works. |
| **1 — Real Rust editor** | Three-frame portrait UI, non-overlap, pinch/pan, frame create/move/resize, numeric painted cells, local autosave, undo/redo | Reopen restores exactly the edited world; gesture conflicts and crashes tested on device. |
| **2 — Real Pokémon graphics** | Decode tileset graphics/metatiles/palettes/attributes, render original Emerald map fixture | Pixel-comparison/golden tests and correct behavior overlay; no substitute or invented tile art. |
| **3 — Native export** | Numeric layout pipeline, map metadata, auto connections, export ZIP, CLI for AI | A/B/C numerical fixture; map/group registrations; deterministic export; compatibility with existing `layout.txt`; test compiled ROM using safe-sized maps first. |
| **4 — Engine integration & events** | Safe small-map neighbor buffer copy, actual events/scripts/warps | Emerald CI build, emulator crossing every A/B/C edge row, camera/tilesets, warps/NPC triggers and save/load. A 5×5 map may only be accepted after proof. |
| **5 — GitHub shared editing** | Pull/Publish with auth, revisions and conflict protections | Two independent devices/agent clients share published data without lost edits, unauthorized writes or overwrites. |
| **6 — True live collaboration** | Hosted Rust WebSocket authority and authenticated AI/client API | Two clients see edits rapidly, server survives restart, reconnect/replay/competing edits are deterministic; GitHub checkpoints match authoritative state. |

**First actual coding task (Gate 0):** introduce `tools/map-studio/` Rust workspace, create a minimal `cdylib` eframe Android app containing the white canvas and three outlined gray rectangles, add a distinct Android APK workflow, and collect a test APK plus logs. **Do not begin** with live sync, a full GBA tileset parser, or engine modifications.

## 9. Acceptance and outstanding research

- Native source facts verified in fork: `include/global.fieldmap.h` map cell packing, `include/fieldmap.h` buffer size and `MAP_OFFSET=7`, `src/fieldmap.c` connection logic and 7/8-cell copying, `data/maps/Route101/map.json` event schema, `data/tilesets/primary/general/` and `secondary/petalburg/` assets, `tools/textmap/build_textmaps.py` and `map_data_rules.mk` registration workflow.
- External framework evidence: [eframe supported platforms and features](https://github.com/emilk/egui/blob/main/crates/eframe/README.md), [eframe crate Android feature flags](https://docs.rs/eframe/latest/eframe/), [egui multitouch](https://docs.rs/egui/latest/egui/struct.MultiTouchInfo.html), [egui-wgpu callback](https://docs.rs/egui-wgpu/latest/egui_wgpu/trait.CallbackTrait.html), [rust-mobile Android examples](https://github.com/rust-mobile/rust-android-examples), [cargo-apk](https://github.com/rust-mobile/cargo-apk), [Android SAF](https://developer.android.com/guide/topics/providers/document-provider), [GitHub Git trees](https://docs.github.com/en/rest/git/trees).
- **Not yet verified:** installed APK on physical device; Android GPU driver and native keyboard; actual real tileset composition golden outputs; native numerical export compiled by the fork; tiny maps safely crossing edges; online collaboration reliability. These are gate tests, not settled facts.

This document replaces the earlier Tauri-oriented recommendation. It records the chosen direction without claiming that a working APK or live collaboration service currently exists.