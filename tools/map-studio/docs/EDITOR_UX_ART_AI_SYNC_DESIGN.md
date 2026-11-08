# Map Studio: UX, source artwork, semantic editing and GitHub sync design

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
