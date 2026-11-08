# Map Studio: UX, source artwork, semantic editing and GitHub sync design


## 2026-11-08 follow-up: AI mapping + live updates (current over earlier snapshot)

This section supersedes the historical measurements and outdated "research only"
status in sections below. The historical analysis remains for provenance.

**Code audited:** \`tools/map-studio/src/{model,prefabs,emerald_library,tiles,ui,github_sync,android_export}.rs\`,
\`tools/map-studio/src/bin/cli.rs\`, bundled 71 source Emerald Porymap prefabs,
\`tools/map-studio/Cargo.toml\`, and the Android workflow.

**Current facts:** v0.20 introduced synchronized prefab references and replaceable
source-backed matrices; GitHub Sync is a whole-\`project.json\` snapshot transport
using GitHub Contents API, SHA and ETag checks. Authenticated foreground polling
is 15 seconds, anonymous polling 300 seconds, and remote edits cannot stream
before they are published to GitHub. The CLI lists assets and can place/move/
remove indexed source prefabs, but does not yet provide one structured
discover-preview-validate-commit transaction protocol. Do not call this live
collaboration. v0.20.1 adds *explicit Android Paste buttons* for repository,
branch, token and clone UUID; phone verification is still pending.

### Priorities and design choices

1. **Real source artwork first.** The existing 71 Porymap Emerald default
   prefabs are not a comprehensive environment library. Build a pair-specific
   semantic catalog from native source tiles, attributes and layouts:
   background (grass/sand/path), macro terrain (water/shore/river/cliff/tree
   formation), structures (house/cave/gate), and micro assets (rock/lamp/
   flowers/sign). These are *metadata categories*, not assumptions about a
   metatile's native engine behavior. A category never substitutes a made-up
   graphic or grants an invalid tileset conversion.
2. **Verified matrix recipes.** A \`recipe\` references an exact native primary
   and secondary tileset, stable asset ID, sparse \`(x,y,id,collision,elevation)\`
   cells, optional required neighbor rules, anchors, exits and source revision.
   Transparent/absent cells are no-op; unrecognized existing cells remain
   unchanged. The editor should support extracting candidate repeated patterns
   from native source maps but every bundled recipe is reviewed visually and
   checked against the real atlas. Do not bundle ROM-hack content without
   demonstrated source compatibility and reuse permission.
3. **Topology for real roads and water.** Road/pond/shoreline editing should
   operate on a semantic area or path and derive *only* source-verified corner,
   side, junction and inner-corner tiles using a neighbor mask. When a required
   transition is missing, say which transition is unsupported rather than
   drawing a bogus rectangle. Preserve manual raw metatile mode as advanced.
4. **One editing engine for AI and touch.** Each operation begins with
   \`project_id\`, \`map_id\`, \`base_revision\`, and either a stable asset ID or
   explicit native-pair tile data. Provide \`inspect_region\`,
   \`search_assets\`, \`preview_transaction\`, \`apply_transaction\`,
   \`draw_terrain\`, \`place_object\`, \`edit_object\`, \`place_event\`,
   \`validate_map\`, \`undo_transaction\`, and \`export_project\`.
   The same Rust validators must serve the UI, CLI and network layer.
   Multi-step transactions are atomic, deterministic and return a machine-
   readable diff + warnings. Agents shouldn't need to reverse-engineer UI
   controls or guess raw ID numbers.
5. **Live collaboration is not GitHub polling.** Split transmission into:
   **(a)** immediate, authorized operations delivered to online devices on
   a private WebSocket channel, with a durable per-project revision journal
   and snapshots to recover missed messages; **(b)** GitHub checkpoints
   written after completed transactions, using existing SHA conflict checks.
   A GitHub push webhook -> relay -> device notification can accelerate
   *checkpoint detection* first, but it cannot stream uncommitted AI edits.
   Never put a GitHub token in a broadcast payload or a publicly reachable
   client. Use a dedicated authenticated relay/project isolated from the
   unrelated game and business backends.
6. **Sync conflict policy:** every operation has unique \`op_id\`, map
   revision, touched tile/object set, source pair, inverse patch and author.
   The receiver applies idempotently and in revision order. Missing
   revisions require resync from durable history; overlapping concurrent
   edits require explicit conflict handling (or scoped map/region locking).
   Offline Android edits queue locally and replay only after revision
   validation. No silent last-writer-wins and no unreviewed remote scripts.
7. **Simple phone UX:** fewer gallery steps; first compatible, verified,
   useful assets; a compact native preview/ghost; one tap to place; select
   object to move/edit/remove; explicit, reversible transitions; a small
   connection indicator with Synced, Receiving, Pending, Offline, Conflict.
   One project/pair is visible; no ghost placeholders or unsupported assets
   pretending to be usable.

### Acceptance simulation before declaring finished

- Create a 32×32 town using a background, native paths, a *verified*
  shoreline/pond, a house, cave access, and micro decorations; check that
  the visual result actually resembles an Emerald location.
- Test 71 source prefabs and all sparse holes; compare atlas render with
  source preview and check all collision/elevation values.
- Place, edit a roof tile, replace, move, remove and undo a prefab; verify
  *no ghost occupancy*, stable IDs and exact underlying cell restoration.
- Run AI commands to discover, select, preview and commit without requiring
  magic prefab indexes or manually constructing a source-pair table.
- While the phone is viewing the town, run 100 AI operations and record
  operation-to-visible-pixel latency, revision ordering, frame rate, losses,
  conflicts, duplicated events, and recovery after a disconnected session.
  Use **measured** latency; a sub-3-second connected P95 is a desired target,
  never a claim until proven on device and network.
- Verify an actual GitHub-backed session, a rejected unsafe edit, an
  intentional simultaneous user/AI conflict, a rollback, project import,
  APK reinstall, and a game compiler/emulator export smoke test.
- On Android specifically, test Paste on all four GitHub fields, the native
  keyboard, small-screen split mode, closing/reopening the dialog, and ensure
  the token stays masked and is not serialized or logged.

**Recommended build order:** verify clipboard APK on phone; record asset
inventory and renderer gaps; complete one fully usable natural terrain family
and prefab selection flow; add structured Rust agent transactions and
simulation fixtures; add fast webhook notification for GitHub checkpoints;
then introduce durable WebSocket operation streams for true live map work.
Avoid adding a separate UI for agents.



**Status:** Research/design only. No APK or project data is changed by this file.
**Evidence:** Android 15 / Samsung A06 5G screenshots supplied 2026-11-08; source audited on
\`feature/map-studio-v0-17-artwork-library\` at map editing commit \`dd2a0c643\`.
**Repository rule:** Do not push to the original Pokémon Expansion upstream.
**Priority:** Reliable content and controls before expanding AI edit privileges.

## 1. Why the current experience fails

- The screenshot of the generated 32x32 Eastvale Gardens map contains accurate source
  building stamps but scarcely any road system, many isolated long-grass rectangles and a
  sharply rectangular pond. This is a quality-of-editing failure, not just missing tiles.
- The header's Project/Undo/Redo controls intersect or nearly intersect the system status
  region. \`ui.rs\` currently uses fixed 12pt top and 20pt bottom spacers, conditional
  on viewport height, rather than Android system-bar / cutout / gesture insets.
  Android 15 with target SDK 35 enforces edge-to-edge; egui exposes
  \`InputState::content_rect\` and \`safe_area_insets\`, but raw Android inset
  propagation still needs testing.
- The palette defaults to 280pt, pins several control rows, uses a 32pt drag
  handle and 108pt tall prefab list cards, each repeating a label, dimensions
  and “Tap to stamp”. The amount of fixed chrome is disproportionate to a
  small Android portrait viewport. Compact-mode scroll behavior is also
  different from standard mode.
- \`prefab_gallery\` displays 6 compatible out of 71 Emerald assets (for
  some current tilesets), hides alternatives under “Browse 60 more prefabs”
  and makes users click “View”, then potentially “Use”, before painting.
  This is an asset-discovery failure compounded by legitimate source-pair
  compatibility constraints.
- Screenshots of the tilesheet contain pink, black and blank/checkerboard
  artifacts. Root cause has NOT yet been proven. It may be an invalid
  subtile/palette, a transparent layer, the wrong texture pair, or a
  preview-only package. Audit actual IDs and source atlases before making
  any assertion. Do not disguise invalid slots with fake art.
- \`World::paint_prefab\` paints every matching cell individually. Tiles beyond
  the map are skipped, and existing occupied cells can be overwritten;
  there is no atomic full-footprint preflight. The canonical World stores
  flattened cells, so after placement a building has no identity, entrance
  position, move-as-unit behavior, or asset provenance.
- \`github_sync.rs\` uses GitHub's Contents API with per-file SHA conflict
  checks; \`ui.rs\` polls every 45s when authenticated, 300s anonymously.
  It has no push notification, HTTP conditional request caching, or
  automatic remote-to-device event channel. Auto pull runs only while the
  app is open and local state is clean. Tokens are in-memory per session.

## 2. Final design principles

1. **Content first:** bad source art, broken transitions and partial buildings
   cannot be fixed by a smarter AI command.
2. **Canvas first:** keep frequently used tools within thumb reach and minimize
   permanent chrome. Never place critical buttons beneath system UI.
3. **One truth:** \`Editor/projects/<uuid>/project.json\` remains the interoperable,
   versioned working document; \`Compiled/projects/<uuid>\` remains output-only.
4. **No silent format or engine conversions:** an Emerald map can contain
   source-compatible Lilycove assets; an unrelated FireRed or Ruby preview
   is not thereby usable in Emerald.
5. **Explicit and reversible changes:** one gesture / one AI transaction
   commits a fully validated change or none at all.
6. **Discover everything; only enable what is usable:** visible preview and
   compatibility status belong in one catalog, not hidden menus.
7. **GitHub holds revisions; the app owns local undo and autosave.**
   Notifications announce changes, not the contents or commands to execute.
8. **Source-derived, not imaginary:** preserve real metatile IDs, collision,
   elevation, layers, maps, prefab licensing and target version.

## 3. Android UX architecture

### Safe window / responsive layout

- Read actual Android WindowInsets for status, system gestures, navigation,
  caption bar and display cutout. Confirm the egui winit Android backend
  passes correct \`safe_area_insets\`; bridge native insets if it does not.
  Remove fixed 12/20pt hacks and arbitrary “if height > 580” behavior.
- Layout the top toolbar in the safe region: first row contains active project
  selector, compact Undo/Redo and menu; second row contains Terrain,
  Properties, Events plus a context-specific Pan tool. The selected map /
  space appears in a small contextual chip, not an extra permanent row.
- Protect tap targets with >=44–48 logical pixel hit boxes, while allowing
  smaller glyphs. Make the project menu reachable with one hand.
- Bottom dock snaps into **peek** (selected brush/prefab only),
  **browse** (one gallery row plus controls), and **expanded** (full library).
  Model these as semantic states; preserve user choice per device orientation.
  It must never sink below the navigation gesture or hide critical controls.
- In split-screen / narrow freeform view, allow only the gallery to scroll,
  not several nested selectors plus entire dock; shrink chrome before
  shrinking hit areas. Use gestures only on the grab handle for resizing.
- Avoid the duplicate “Load / Use” flow: the normal source-pair is tied to
  the selected map; choosing another pair for a painted map is either
  preview-only or requires an explicit safe conversion. Do not silently
  reinterpret IDs.
- Display compact visual cards (around 80–100pt wide, responsive columns),
  image first, short source label second; remove repeated “Tap to stamp”.
  Group by object type and location. Support Recent, Favorites and filter
  chips with one compact contextual control.
- The primary quick-pick gallery starts with compatible assets; scrolling
  reveals the full **same index** of other assets with source/preview-only
  status. No “Browse 60 more” expansion. An incompatible item can be viewed
  without activating it for the wrong map.

### Touch model

- Tap chooses item; hovering/ghost placement is shown as finger moves,
  including 1-tile apron and collision/door projection.
- One tap stamps; long-press on a selected placed object opens move/edit;
  pan has a distinct state and can be activated on empty canvas.
- Canvas remains pan/zoom capable in every mode; gestures should never
  paint unless the active mode explicitly permits it.
- Failed placement reports a precise reason, not “No change”.
  No partial building pastes by default.

## 4. Asset correctness and metatile library

### Source audit before categorization

Inventory each engine/tileset pair from checked-in native assets and metadata:
- primary/secondary symbols and original source commit
- metatiles, their 8 subtiles, graphics tiles, palette bank, flips, layers
- \`metatile_attributes.bin\`: behavior and layer type
- per-cell collision / elevation (separate from metatile behavior)
- atlas UVs, valid-mask, transparent background assumptions
- prefab JSON and underlying source license / attribution

Run deterministic **headless atlas tests** for every selectable source pair
used by the active project. For each ID, verify graphic references, palette
indexes, valid mask, 16x16 output, correct transparency/layer handling and
reference render. Capture screenshots on Android for representative families
(General/Lilycove; Town; Cave; Interior; preview-only Ruby assets).

Never label blank slots “paintable”. Distinguish intentional transparency
from missing resources with a separate visible invalid/unsupported overlay.
Log exact \`collection/pair/id\` when a renderer cannot resolve a tile.
Native source validation must use the exact pair, not the default atlas.

### Unified metadata / catalog

Keep existing bundled Porymap Emerald (71) and FireRed (45) prefab definitions
without falsely claiming every prefab fits every pair. Show all in the gallery
with statuses: Compatible / Different source pair / Preview only / Invalid.

Add a **reviewed semantic catalog** per source-pair:
- stable \`asset_id\`, human name, category, tags, source reference, license
- exact tileset compatibility and fixed width/height
- occupied/metatile footprint, non-overwrite mask, entrance/interaction cells,
  nearby walkable apron, collision/elevation, placement anchors
- optional rotation/flip ONLY if true source graphics support it
- validation status, source link, review/test version

Add user-defined prefabs created from actual map selections; preserve their
source pair and metadata. Never replace source-backed art with invented pixels.

### Correct roads, ponds, paths, borders

Introduce reviewed **terrain families** with topology, not just raw tile IDs.
Start with one complete Emerald/Lilycove walkway or road family and one
verified pond/water-shore family. Each family must declare its real center,
edge, corner, inner corner, intersection and allowed neighbors (if present);
if the source lacks a required transition, flag it and prevent claiming a
polished auto-join. Use a neighbor-aware terrain brush analogous to
Tiled/Wang terrain sets; automate only source-verified transitions.
Let raw metatile brush remain available under Advanced.

## 5. Safe shared editing engine (human and AI)

Replace direct per-cell prefab stamping with \`preview_place_prefab\` and
\`commit_place_prefab\`, both using a single Rust transaction engine.

Minimum batch operations:
- \`inspect_project\`, \`inspect_region\`, \`list_compatible_assets\`
- \`create_map\`, \`place_prefab\`, \`move_prefab\`, \`remove_prefab\`
- \`draw_path\`, \`paint_terrain_region\`, \`create_pond\`
- \`place_event\`, \`validate_transaction\`, \`apply_transaction\`
- \`undo_transaction\`, \`export_game_source\`

Every operation carries project UUID, map ID, world revision and
source asset ID, not an ambiguous human-readable name.
The Rust engine checks geometry, full footprint, target tileset, layer
semantics, map boundaries, blocked cells, event/warp references,
neighbor transition rules and reachability as applicable.
Successful batches produce deterministic diffs and one undo step.
Any invalid step rejects the entire batch, without changing the saved map.

Preserve \`cells\` as canonical for compatibility during the transition,
and introduce optional, versioned placement/region provenance metadata
so stamped structures can be moved and edited as objects. **Do not create
two competing sources of truth:** metadata must either match a verified
underlying tile checksum or be explicitly detached when manually repainted.
Only after a reversible migration and roundtrip tests should a compositional
semantic graph become the sole source for regenerated tile grids.

### Map-quality validation

A generated town must be evaluated for actual *design quality*:
- entrance cells not embedded inside collision terrain
- navigable routes between each designated landmark and map connection
- no clipped buildings or unwanted overwrites
- connected paths with verified corner/border art
- sensible margins between structures and visual landmarks
- water bodies with real shoreline transitions; no unexplained geometric
  water rectangles unless intentionally selected
- no unfinished scripted events in “game-ready” output
- explicit warning for any requested feature without native support

The user may override aesthetic suggestions, but never structural
corruption or target-engine incompatibility.

## 6. GitHub sync design

### V1.1: improve current polling reliability (no infrastructure)

- Keep the selected UUID and validated manifest fixed throughout each request.
- Add ETag/If-None-Match and exponential backoff; honor HTTP retry and
  polling constraints. Prefer a small stable revision marker to downloading
  large unchanged \`project.json\` on each check.
- Surface explicit state in the top bar: Offline / Checking / Synced /
  Pending upload / Conflict / Needs authentication.
- Auto-publish **only as an opt-in** after local autosave is complete.
  Debounce rapid touch edits; preserve every failed upload in local queue
  and require no manual retries for ordinary transient network errors.
- Never silently overwrite. Use GitHub blob SHA preconditions, project UUID
  and canonical content hashes, with user-visible conflict review.
- Keep PAT in memory in current V1. Design later GitHub App OAuth/pairing
  plus hardware-backed secure credential storage for least privilege.
- A private repository requires appropriate GitHub authorization.
  No tokens inside repo, screenshot, config, or AI operation.

### V2: nearly instant update notification when editor is foreground

**GitHub push -> HTTPS webhook receiver -> authenticated outbound
WebSocket connection to phone -> phone GET latest GitHub revision -> Rust
validation -> local snapshot -> repaint.**

GitHub webhooks cannot reach an Android private-LAN address directly; a
publicly reachable HTTPS handler is needed. A small stateless Cloudflare
Worker + a Durable Object WebSocket channel is one possible deployment.
Use HMAC-SHA256 GitHub webhook signature verification; strict owner/repo,
branch and path allowlists; scoped project channels; authentication, replay
protection and reconnect/backoff. Send only notifications containing the
revision / project ID. The phone independently fetches data from GitHub and
checks conflicts. Do not run arbitrary webhook instructions.

This is **near-real-time while online and foreground**, not guaranteed
sub-second editing; commits, webhook delivery, cellular latency and Android
lifecycle constraints remain. GitHub continues as versioned persistent
storage; this thin service only alerts subscribers and requires no
new mandatory database.

Do not build collaborative multi-person simultaneous editing / CRDT now.
The present goal is one Android editor + one AI editing via authorized
GitHub access. Full shared object-level merge is a later project.

## 7. Simulation and acceptance gates

| Scenario | Pass criterion |
| --- | --- |
| Android 15 status bar + camera cutout | Project, Undo, Redo and panel handle are always visible and tappable, no hardcoded inset compensation |
| Samsung A06 5G portrait | Gallery opens in one action, default panel uses substantially less height, pan/zoom remains smooth |
| Samsung A06 5G split-screen or resized window | No dock below screen; buttons >=44pt effective hitboxes; scrolling remains bounded |
| Prefab discovery | All compatible art is immediately accessible; no “Browse N more” / “Load” navigation trap; incompatible items explain why |
| Existing four buildings and Eastvale Gardens | Old maps and metatile cells remain byte-equivalent after migration / roundtrip |
| Source catalog / screenshot artifacts | Every usable metatile and prefab has a correct, source-matched preview; invalid slots cannot be selected |
| Stamp over map edge or occupied footprint | No partial paste or silent overwrite; before/after map bytes equal |
| New road / pond | Correct neighbor borders and walkable routes; no obvious rectangular holes or false shoreline transitions |
| AI creates town | Deterministic structured diff; valid buildings, entrances and connected streets; no raw magic metatile IDs in agent instructions |
| Two different GBA projects | Completely isolated IDs, source palettes, revisions and exports |
| Live sync with no local changes | On webhook notification the new validated GitHub revision appears promptly when app is foreground |
| Offline or stale GitHub publish | Local edit retained; explicit status; retry is safe |
| Phone and AI edit simultaneously | Conflict surfaced; neither branch of data lost; recovery snapshot exists |
| ROM export | Source-generated output kept separate from executable game; game-side compilation verified independently |

## 8. Implementation order and stop gates

**Phase 0 — UX and asset safety.**
Fix Android insets, condensed panel, remove browse/load maze, fix reported
magenta/blank atlas defects, implement prefab placement preflight. Ship a
separate preview APK and pass real Samsung A06 5G portrait and split-screen
checks. Stop if existing project recovery or renderer regress.

**Phase 1 — trustworthy artwork tools.**
Source-backed catalogs, compatibility metadata, user prefab creation and
verified visual source variants. Build a tested Lilycove terrain family
and automatic paths/shorelines; preserve raw tile editor for advanced edits.
Stop if a tile family cannot be linked to real native IDs.

**Phase 2 — semantic AI and human editing.**
Single transaction API, object provenance, preview/undo/validate, native
reachability analysis and deterministic AI scene generation. Test actual
town results, not only valid JSON. No direct unrestricted AI file writes.

**Phase 3 — GitHub sync usability.**
Better polling and status first; then an optional verified webhook +
WebSocket signal relay, secure pairing and end-to-end physical device tests.
Maintain GitHub fallback for when relay is down.

Avoid making separate branches or new project formats for each small
feature. Commit source and tests on the existing development branch; do
not modify the original upstream. Save APK artifacts in Actions/Releases,
not the Compiled/ map-source directory.

## Research references

- Android 15 edge-to-edge enforcement: https://developer.android.com/about/versions/15/behavior-changes-15
- Android WindowInsets: https://developer.android.com/develop/ui/views/layout/edge-to-edge
- egui input safe area: https://docs.rs/egui/latest/egui/struct.InputState.html
- Porymap prefab workflow: https://huderlem.github.io/porymap/manual/editing-map-tiles.html
- Tiled terrain families: https://docs.mapeditor.org/en/latest/manual/terrain/
- Tiled automapping: https://doc.mapeditor.org/en/latest/manual/automapping/
- GitHub REST polling guidance: https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api
- GitHub webhooks: https://docs.github.com/en/webhooks/about-webhooks
- GitHub webhook signature verification: https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries
- Cloudflare WebSocket Hibernation: https://developers.cloudflare.com/durable-objects/examples/websocket-hibernation-server/


---

## Verified implementation decisions — deeper source and packaged-APK audit

**2026-11-08 second review. This section is the implementation baseline, not a completed code change.**

### A. Root-cause evidence for broken atlas thumbnails (stronger than screenshots)

The existing \`MapStudio-Projects-GitHub-Sync-v0.17.apk\` contains native
\`libmap_studio.so\` with embedded, already-generated 256x1024 atlases.
Inspecting the PNG byte streams (valid PNG chunks through IEND) produces:

- 130 valid 256x1024 PNG atlases
- 82 with **opaque** \`RGBA(255,0,255,255)\` pixels (not screenshot compression)
- 39 with an opaque magenta pixel at atlas coordinate (0,0)
- all 130 with transparent unpopulated source slots
- 321,544 total opaque exact-magenta pixels among those atlases

The existing \`build_catalog.py\` explicitly does:

\`\`\`py
atlas.paste((*palettes[0][0],255),(x,y,x+16,y+16))
\`\`\`

That makes palette-zero, often a magenta placeholder, an *opaque backdrop*
before drawing subtiles. The game's \`field_camera.c\` has three layer modes;
NORMAL uses tile \`0x3014\` in BG3, COVERED and SPLIT route the eight
source subtiles differently. A one-image per-metatile approximation loses
the distinction between transparent layer pixels, opaque backdrop, missing
source graphics, and intentionally unoccupied ID slots.

**Concrete fix:** preserve native atlas/tileset data but revise generation to
emit separately testable bottom/middle/top alpha layers, a source-faithful
composite, and an editor-friendly context preview. Never simply repaint
all magenta to green: any magenta truly visible under the game's real
layer composition must be flagged as a broken or incomplete source
metatile, not covered up. The editor's guided picker must not present
empty slots as ready-to-paint tiles. The advanced raw sheet may preserve
the original ID positions and explicitly show holes.

Run a deterministic pixel-level comparison against the fork's native
\`DrawMetatile\` algorithm for NORMAL, COVERED and SPLIT with a handful
of known General/Lilycove, cave and indoor IDs, including transparency
cases and the \`0x3014\` fallback. Snapshot output at 1x and scaled nearest
neighbor on Android. **Pass:** no unintended opaque placeholder color,
no hidden invalid layers, real native metatile IDs unchanged.

### B. A workable prefab solution that doesn't violate GBA tilesets

Porymap's native prefab workflow intentionally binds prefab data to its
primary and secondary tileset symbols (see the Porymap manual). The user
experience can be made single-click **without pretending incompatibility
doesn't exist**.

**Default asset view:** a compact row/grid of compatible prefabs from
the selected map, categorized by Home/Service/Civic/Nature/Paths,
single tap selects. A unified gallery shows all other prefab entries
immediately by scrolling, with colored-less textual source tags and an
explicit disabled/select-other-map action. No expandable “Browse 60 more”
and no “Load/Use” on the normal compatible path.

Changing the **browser selection** is not the same as changing a painted
map's **source tileset**. If a chosen prefab requires a different native
pair, the editor can show its full preview and offer:
(a) choose an existing compatible map;
(b) make a new adjacent map with that pair;
(c) view-only until a reviewed lossless conversion exists.
Do not silently remap the map's existing IDs.

Prefab placement must first check the complete nonempty footprint,
map boundary, existing protected footprints, type/engine, and
any invalid metatiles or elevation/collision. Apply all cells in a
single transaction or apply none. Porymap-style user prefab creation
from a rectangular map selection is the next extension; it stores
the exact native source pair and tile values, not pixels alone.

**Source test:** placement near any of four map edges, touching a
protected object, and across a map connection must leave both
original maps byte-for-byte unchanged when rejected.

### C. Concrete Android viewport layout

The current app targets SDK 35 and uses fixed \`add_space(12.0)\` on
tall Android viewports and \`Panel::bottom(...).exact_size(20.0)\` for
gesture inset; these are not actual OS insets. Native \`WindowInsets\`
should be forwarded into \`egui::RawInput.safe_area_insets\` (or first
verify the specific eframe/winit 0.37/0.30 Android backend supplies them).
Do not both pad the native view and pad egui: one source of truth for
safe-area measurement and density conversion (px -> egui logical points).
Consider caption bar and display cutout in freeform mode.

Top header: only two rows in the safe area. **Row 1** selected project
menu + Undo/Redo + overflow, **row 2** Terrain/Properties/Events + Pan
and one contextual filter control. The selected Space/map appears
contextually rather than as a persistent third header row.

Bottom palette snapped states:
- **Peek**: only drag handle and current selected tile/prefab, about
  52–64 logical points.
- **Browse**: asset-type chips + a single gallery row, about
  145–190 logical points on a standard portrait viewport.
- **Expanded**: large, virtualized asset grid capped by available safe
  content area, never overlaps system gestures.

Only *gallery content* scrolls; the grab handle, source-pair control
and selected asset state remain reachable. Bounded scrolling on
split-screen and short freeform viewports takes precedence over
preserving a fixed 280-point panel. Touch hit targets should be at least
44–48 logical points while visual chrome may be more compact.

Prefabs: precompute thumbnail textures once per source-pair/build rather
than drawing every subtile every frame; use lazy-loading and
row virtualization, plus NEAREST sampling for pixel art.

### D. Gameplay-quality scene creation rather than a tile-ID flood

The existing Eastvale Gardens map is a textbook test: it is a 32x32
fully painted grid with ten prefab buildings and a pond, but it has no
authored connected street system; the path “design” was green grass
rather than validated road family. Fix via a reviewed **Lilycove Walkway
v1** terrain brush and **Pond/shore v1** family, not by guessing paths
from any tile with “grass” in its name.

A terrain-family definition has explicit native IDs for center, 4 edges,
4 corners, inner corners, T junctions and cross junction if available;
each case has a source screenshot and tests against real target native
palettes. If a required transition is missing, reject that shape or
offer a human-reviewed alternative. No fictitious edges.

A building prefab \`asset_id\` must be unique and derived from
collection+target pair+source prefab index+source content hash, not
the ambiguous name “House”. Store entrance/interaction tile metadata
only after verifying each game's actual event/warp definitions.
Maintain optional provenance against flattened \`World.maps[].cells\`;
manual edits can invalidate/remove provenance without destroying the
canonical tile content.

Put \`plan\`, \`preview\`, \`apply\` and \`undo\` on one Rust transaction path.
Examples: \`place_prefab\`, \`draw_road\`, \`build_pond\`,
\`add_map_east_of\`, \`place_event\`. A transaction references
project UUID, map ID, asset ID and base revision, validates all proposed
tiles and constraints before mutation and gives a deterministic
per-map cell diff. Applying one proposal must create exactly one undo
step. No arbitrary direct JSON mutation in the AI-facing API.

Quality acceptance is stricter than \`check_integrity\`: all intended
entrances have reachable walkable paths to the town center and the
neighboring map transition, no unplanned clipping, accurate shoreline,
no inaccessible building entrances, correct engine target, and
no runtime-silent unsupported scripts.

### E. GitHub connectivity: concrete minimum and upgrade route

**First:** leave the existing Contents API transport in place and
improve it before deploying a server. Add per-selected-project
\`ETag\`/conditional requests for stable files, prevent duplicate
inflight requests and preserve SHA-constrained uploads. Show a tiny
fixed state icon: checking/synced/local pending/conflict/auth/offline.
Automatic publishing is opt-in after a successful atomic autosave;
debounce strokes; never publish half-painted gestures.

**Authentication:** replace repeatedly pasting a long-lived PAT with
GitHub App **device flow**, which is compatible with a headless or
limited-input device and supports expiring user access tokens and
refresh-token rotation. Protect refresh credentials using Android
Keystore-backed authenticated encryption and exclude them from
Android backup. Keep existing manual fine-grained PAT as an optional
advanced escape hatch, not the default UX.

**Then:** if an open phone must react faster than 45-second polling,
deploy *one* HTTPS webhook receiver + one authenticated WebSocket
notification channel (Cloudflare Worker + Durable Object is a concrete
candidate). Validate GitHub \`X-Hub-Signature-256\` using HMAC-SHA256,
verify owner/repo/branch and an exact
\`Editor/projects/<uuid>/project.json\` changed path, scope channel to
paired project, and send only \`{project_id,commit_sha}\` signal. Phone
pulls from GitHub and runs the same validation/conflict logic; the
relay never stores or rewrites project JSON.

**Acceptance:** verify a phone update is visible after an AI commit
while foreground and online; no guaranteed absolute delay due GitHub
webhook/network delivery. On connection loss, foreground resume, or
restart, a single GitHub revision check catches missed updates; no
permanent event subscription is required when the app is closed.

### F. Release gate and scope

1. **Fix the preview pipeline + atomic prefab preflight** with
   reproducible pixel/hash fixtures. This is non-negotiable before
   another ambitious AI town generation.
2. **Fix insets and strip the dock** while preserving touch gestures.
3. **Implement native Lilycove road and shoreline families** only
   after every source-tile transition is independently inspected.
4. **Add one transaction engine** shared by human/AI controls.
5. **Upgrade GitHub**: reliable conditional polling and device
   login first, optional near-real-time signal transport later.
6. Test with the existing project JSON as read-only reference and
   snapshot every existing map. No destructive migration or
   cross-package uninstall. Build a separately installable preview
   APK, publish Actions artifact and both direct/GitHub links.

**Research sources**

- https://grunt-lucas.github.io/porytiles-user-docs/gba-decomp-tileset-system.html
- https://github.com/rh-hideout/pokeemerald-expansion/blob/master/src/field_camera.c
- https://huderlem.github.io/porymap/manual/editing-map-tiles.html
- https://developer.android.com/develop/ui/views/layout/edge-to-edge
- https://docs.rs/egui/latest/egui/struct.InputState.html
- https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app
- https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api
- https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries
- https://developers.cloudflare.com/durable-objects/best-practices/websockets/
