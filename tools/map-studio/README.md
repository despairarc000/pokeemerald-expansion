# Map Studio — Native Rust Android editor


## Native sparse matrix editing (development branch)

The original source JSON and native project files are preserved. The editor
converts Porymap and source-derived environment assets into **real row-major
matrices**, using `null` for transparent cells. Unlike the old coordinate-list
output, `asset-json` emits `matrix[y][x]` rows, where every authored cell
carries exact native metatile ID, collision, and elevation. Source pair
compatibility remains mandatory; transparent positions never paint over the
background. Rust placement preflights and stamps the same matrices.

AI agents may fill a new section with a single `paint_matrix` JSON transaction,
then use `preview-transaction` and `render-region` to inspect its actual
source-atlas appearance before committing. This uses the existing World model
and native validation, not a competing sliced/AI-only map format. See
[Agent editing protocol](docs/AGENT_EDIT_PROTOCOL.md).

**Verification scope:** unit tests and CI are required for this development
branch; this does not claim a tested phone build or complete artwork coverage.

## v0.22 Shared AI + human editing engine (preview)

**This adds a versioned structured transaction interface to the existing
native Rust World model; it does not create a second map format.**

- AI agents can inspect any map region as exact source cells or a rendered
  native tileset PNG. Supported commands: revision, inspect-region and
  render-region. Source art and prefab discovery from v0.21 are retained.
- An agent can write one JSON plan with several create-map, fill, paint,
  erase, collision, elevation, place-asset, move/remove-object, Lilycove
  walkway and basic sign/NPC operations.
- preview-transaction applies the plan **only to an in-memory draft**,
  validates it, reports changed cells and objects, and can render the
  uncommitted result to PNG.
- apply-transaction checks the project revision again, validates every
  operation using existing World methods, saves atomically, and preserves
  the previous state for a **single locally guarded Undo**. Invalid or
  stale requests do not partially alter the original project.
- Prefab instance UUIDs created by a transaction are deterministic
  for the transaction ID and operation index: preview and commit have
  the same object references and resulting revision.
- No JSON migration. Old Emerald project cells and original Porymap
  indexes remain intact. v0.21 environment assets remain available.

See [Agent editing protocol](docs/AGENT_EDIT_PROTOCOL.md) for JSON schemas,
exact CLI commands, supported operations, and safety limits.

**Boundary:** The touchscreen UI and headless transaction layer invoke
the same World primitives, but Android does not yet accept incoming
remote operation messages. Live synchronization and multi-editor
cross-device conflict resolution are the next, separate milestone.
Native phone touch testing and full ROM/emulator verification remain.




## v0.21 Native Emerald environments (preview)

**Artwork milestone, not a claim of full autotiling or gameplay-ready maps.**

- The existing 71 Porymap Emerald prefabs keep their exact source indexes
  for old project compatibility. Fourteen new native background, environment
  and decoration **matrix recipes** are appended (85 total).
- Recipes are extracted at build time from the fork's own Emerald map.bin
  layouts, with the exact original primary/secondary tileset symbols,
  metatile IDs, collision and elevation. The manifest contains stable IDs
  and preferred map regions; the build locates matching native features,
  records resolved source coordinates and a source-map checksum, and
  rejects missing or unsupported artwork rather than inventing metatiles.
- Examples include genuine pond-bank sections from Routes 102 and 104,
  a Lilycove coast and rock shelf, actual grass/tall-grass regions, trees,
  and berry soil. They are **source-map selections**, not independently
  complete terrain autotilers; source-neighbor transitions and game
  movement still require visual/device tests.
- One Android gallery combines native thumbnail previews with the original
  buildings, organized as **Background, Environment, Buildings, Details**.
  Incompatible assets stay visible but disabled, never remapped to another
  source pair. A source recipe acts as one editable, replaceable prefab
  without invisible occupancy restrictions.
- AI/CLI: \`assets-json <project> <map-id>\` lists stable IDs and native
  compatibility, \`asset-json <project> <asset-id>\` returns the original
  native cell matrix, and \`place-asset <project> <map-id> <asset-id> <x> <y>\`
  places it through the same Rust preflight as finger placement.
  Numeric Porymap \`place-prefab\` commands remain supported.
- GitHub Sync's explicit Android clipboard Paste buttons are retained
  from v0.20.1. No new live synchronization service is part of this
  artwork-focused release.

**Verification:** source extraction, Rust tests, gallery generation,
desktop compilation and APK signing are checked by GitHub Actions.
The source gallery is published as a CI visual-audit artifact. A signed
APK does not prove good appearance on a real phone or in the Pokémon ROM.

**Compatibility:** this preview uses a separate Android package ID.
Project schema remains \`1\`, but old previews do not understand new
environment prefab indexes, so keep backups and do not roundtrip v0.21
projects through an older APK.



## v0.20 Editable prefab matrices (preview)

This iteration makes native Emerald prefabs replaceable and fixes the v0.19
ghost-footprint bug. Saved \`schema_version: 1\` JSON projects, source IDs,
collision/elevation, source tileset pairs, and native map export remain unchanged.

- **Place/re-place:** one prefab placement may replace another building, a
  hand-painted tile, or a partially erased building. The complete source-backed
  sparse matrix is checked before changing cells. Native event protection,
  out-of-map bounds and tileset-pair compatibility remain enforced. Empty
  matrix slots are transparent, never invented filler terrain.
- **Ownership follows edits:** brush, stamp, eraser, bucket, collision and
  elevation changes update optional prefab references. Fully erased or
  replaced prefab cells no longer occupy hidden positions.
- **Reversible replacement:** when replacing one object with another, removal
  reveals the original underlying native cells rather than resurrecting a
  discarded building. Customized prefab cells can move with their object.
- **Old projects:** on JSON import, stale v0.19 prefab stamps are reconciled
  against the actual map without repainting it. Flattened historical artwork
  without object metadata remains native cells; identity is never guessed.
- **Simulation:** all 71 bundled Porymap Emerald definitions were checked as
  sparse matrices, including the transparent gaps. Regression tests cover
  replacement, moving changed buildings, erasing, preserving events, and
  stale-project restoration.

**Unverified:** Android touchscreen behavior, CI-signed APK packaging, true
end-to-end source export in the game compiler/emulator, shoreline autotiling,
and fully automatic adoption of old flattened houses.

This preview uses a different Android package name from v0.19 because the
CI test-signing certificate varies. Keep the previous APK installed and
back up/clone projects before trying this preview.


## v0.19 Smart Terrain and Building Objects (preview)

**Emerald-only milestone; saved native metatile IDs and the existing JSON
project schema remain usable.** The last verified APK for this milestone
is available from the GitHub Actions artifact after CI passes.

- **Source-accurate paved roads:** \`Paths\` now paints a **three-tile-wide**
  Lilycove path with automatically chosen 3x3 native top/middle/bottom,
  corners and joins. All nine \`0x22A–0x23C\` source tiles exist in
  LilycoveCity and were visually checked against the actual source atlas.
  The painter preflights the **entire** edited region, protects events,
  buildings, cliffs and water, and can replace ordinary source grass and
  long grass. No partial path when an obstacle blocks a stroke.
- **Tracked Emerald buildings:** newly placed original Porymap Emerald
  prefabs now have a stable instance UUID and saved underlay. On the
  phone, open **Terrain → Buildings → Select placed**, tap the building,
  then choose **Move** (tap destination top-left) or **Remove**. The
  editor highlights the selection and the pending move footprint. Actions
  are atomic and undoable. If you manually edit a building tile, its
  move/removal is blocked rather than deleting the new artwork.
- **Old maps still work:** existing \`project.json\` files have no tracked
  prefab metadata and load as before. Previous hand-painted buildings
  and v0.18 stamps are **not automatically adopted as movable objects**.
  New tracked objects are stored in an optional, backwards-readable
  world field; older APK versions may discard this field if they resave
  that same synchronized project.
- **Agent CLI:** \`tilesets\`, \`set-tileset\`, \`assets\`, \`place-prefab\`,
  \`objects\`, \`move-prefab\`, \`remove-prefab\`, and \`walkway\` expose the
  native source rules to AI agents. Numeric map tiles remain canonical,
  while extra source provenance allows precise object operations.
  After a map has any painted cells, switching native tileset sources
  is blocked to prevent visual reinterpretation of identical IDs.
- **Sync:** the existing opt-in 15-second authenticated GitHub polling,
  ETag conditional reads and SHA conflict checks remain; no webhook
  relay was deployed and true push is still a separate milestone.
- **Safe preview installation:** package
  \`dev.despairarc.mapstudio.smartterrainpreview\` is separate from
  v0.18 because CI-signed APKs cannot be reliably installed as upgrades.
  Clone an existing project via GitHub Sync using its original UUID.
  Never uninstall the old preview before verifying backups and cloning.

**Not implemented yet:** source-verified pond/shoreline autotiling,
independent AI transaction previews / full planned-diff API,
automatic adoption of previously hand-stamped buildings, full drag-to-move
building gesture, and WebSocket notifications. Native map output still
requires verification by the Pokémon game compiler/emulator.

## v0.18 Emerald-first rebuild (active development)

The artwork picker has been **rebuilt for Emerald**, with the old
multi-collection / per-tileset browse-load menu removed from the main
editing workflow. Existing project JSON, source metatile IDs, primary and
secondary tileset assignments, and other historical artwork files remain
loadable. This is a new interaction layer, not a destructive migration.

- **Compact Android UI:** the palette defaults to 184 logical points
  instead of 280, has a short drag grip, and snaps between peek/browse/
  expanded on double-tap. The source and asset view are pinned; only the
  actual content gallery scrolls. API35 edge-to-edge handling now queries
  native Android WindowInsets rather than a constant 12pt top spacer.
- **Source-backed prefab gallery:** all 71 Porymap Emerald prefabs appear
  in one index with native thumbnails generated from the source atlas.
  Compatible items are shown first, with no extra "Browse more",
  "View", or "Use" steps. Other source-pair assets remain discoverable
  but cannot be stamped into an incompatible map.
- **Source-based placement:** complete prefab bounds are checked before
  painting. Only unpainted/native-grass terrain or an exact matching tile
  can be replaced; occupied structures, events, source-incompatible IDs
  and cross-map clipping reject the entire action. One touch = one stamp.
- **Metatile preview correction:** the atlas builder no longer fills every
  transparent source pixel with an opaque palette-zero backdrop, which
  previously introduced visible magenta on many tilesets. Original
  layered/native rendering remains a separate fidelity test.
- **AI editing:** \`map-studio-cli assets <project.json> <map-id>\`
  lists source-compatible Emerald prefabs with stable upstream indexes;
  \`map-studio-cli place-prefab <project.json> <map-id>
  <source-prefab-index> <local-x> <local-y>\` safely stamps the full
  footprint using the same Rust checks as the Android UI.
- **Emerald terrain:** the new **Paths** picker exposes source-verified
  Lilycove paved walkway metatile `0x22B` (555), present in the native
  Lilycove City layout. Drawing a path respects map bounds, existing
  buildings and events; blocked segments do not overwrite protected tiles.
  AI can run `map-studio-cli walkway <project.json> <map-id> <x1> <y1>
  <x2> <y2>` using map-local coordinates. This is real native pavement,
  **not yet** full automatic junction/shoreline topology.
- **GitHub:** opt-in auto-pull every 15 seconds when authenticated,
  conditional ETag reads of unchanged project data, and opt-in
  auto-publish of completed local autosaves. Both retain GitHub revision
  preconditions and reject simultaneous edits. Auto-publish is disabled
  by default and tokens are still kept only in memory.
- **Verification:** CI generates a visual Lilycove source-layout audit
  from the actual Emerald \`LilycoveCity/map.bin\`, source atlas and
  tile attributes before any road/pond auto-tiling rules are promoted
  into the editor.

**Existing project migration:** use **Project → GitHub sync → Clone existing
project** and paste the project UUID from your previous app's
`Editor/projects/<uuid>/` on GitHub. The new preview validates the
remote manifest and map and creates a local copy with the **same** UUID,
without replacing the old APK or its private saves.

**Current limits:** smart road and shoreline painting, persistent
building-as-object moves, secure one-tap GitHub device authorization,
and a webhook-to-WebSocket live notification relay remain unfinished.
The GitHub polling upgrade is low latency but **not true instant push**.
No remote server is deployed by this release. A generated APK must
still pass CI and actual on-device UI/network tests before being called
fully verified.

**Safe install:** v0.18 uses a *different* Android package
(\`dev.despairarc.mapstudio.emeraldrebuildpreview\`) so the user's
previous preview and its private autosaves remain untouched. Older
Android package storage is not automatically readable from this one;
use a compatible exported project or GitHub pairing to transfer data.

See [concrete engineering review and acceptance gates](docs/EDITOR_UX_ART_AI_SYNC_DESIGN.md).

Development branch: `feature/map-studio-native-android`.
Engineering baseline: [design/map-studio-implementation.md](../../design/map-studio-implementation.md).

This is a native `eframe` / `egui` / `wgpu` Android editor with source-generated visual tileset atlases for 129 of the 137 layout-declared primary/secondary combinations currently resolvable in the repository. Its nested editing Spaces and per-map tilesets do not change the Emerald engine's map-local coordinates. It is **not yet** a fully playtested GBA ROM tool or a live GitHub collaboration client.

## Source update after the v0.17 Art Library Preview

- **Built-in collections:** Emerald and FireRed/LeafGreen. The latter uses the
  native FRLG tilesets already included in this expansion fork; it is not
  renamed Emerald art or a fake gallery entry.
- **Real Porymap prefabs:** 71 Emerald definitions and 45 FireRed definitions
  (source-tileset IDs, collision and elevation preserved). The Prefabs browser
  now shows compatible stamps and provides navigation to valid stamps from
  other tilesets, rather than silently presenting an empty gallery.
- **Not included:** Ruby/Unbound/Team Aqua/ORAS external source packages are
  additional collection candidates, not yet a bundled, game-paintable format.
  Full foreign tileset compilation, credit checks and ROM round-trip remain
  separate requirements.
- **APK status:** the previously saved v0.17 APK predates this source update.
  Use a successful newer Android workflow artifact for device testing; do not
  assume a repository commit itself creates a downloadable APK.

## Multi-project workspaces — newest development branch

Map Studio now uses isolated local \`Editor/projects/<UUID>/\` workspaces with
a \`manifest.json\` and the existing versioned \`project.json\`. Use
**Project → New project** to create an independent GBA world, or
**Project → Switch project** to select an existing one. The current world
is saved before switching; per-project undo history, imported package
selection and active map state are reset.

The very first app launch safely **copies**, rather than deletes, a previous
single-file \`map-studio-project.json\` when it exists in this installation's
private data directory. If that legacy file is corrupt, project bootstrap
fails without creating a replacement. This Projects Preview uses the **new**
Android package \`dev.despairarc.mapstudio.projectspreview\` so previous
preview installations are not replaced or uninstalled. Consequently it cannot
read another Android package's private legacy autosave automatically; export a
backup from the old app first. The old MediaStore import browser is not a full
cross-package document picker yet, and old app storage remains private.

Project IDs are UUIDs; in-world map IDs are local to each project. The
manifest records the target game (Emerald Expansion or FireRed), its GitHub
source repository and optional pinned source commit. FireRed projects can
be created and edited but **their native game-source export is intentionally
blocked** until the engine-specific exporter is implemented and verified.
FRLG artwork present in the Emerald game fork remains usable as Emerald art;
it does not imply FireRed ROM compatibility.

\`Compiled/projects/<UUID>\` contains output-only game source generated
by \`workspace-publish\` via the CLI: native \`map.bin\`, \`border.bin\`,
\`map.json\`, \`scripts.inc\`, and numeric-layout source inputs. A project
UUID is incorporated into all compiled map symbols. Output is staged and
atomically swapped only after validation. This is **not an APK** and is not
equivalent to a compiled/verified GBA ROM. Integration with the target game's
map registrations and a real GBA build remain separate steps.

See [the review, tests and simulated workflows](docs/PROJECT_WORKSPACES.md),
[Editor/](../../Editor/README.md) and [Compiled/](../../Compiled/README.md).

**GitHub project sync is now opt-in.** Open **Project → GitHub sync** to
configure an \`owner/repo\`, branch and fine-grained token for the selected
UUID project. Publishing and pulling use the GitHub Contents API with
optimistic revision checks, local backups and 45-second authenticated
auto-checks (5 minutes unauthenticated). Tokens remain only in memory.
Phone-to-GitHub network behavior still needs real-device acceptance testing;
the app clones an existing UUID by explicit ID; automatic remote project discovery is not yet implemented.
See [GitHub Sync V1](docs/GITHUB_SYNC_V1.md).

## What's implemented

- White-grid canvas, adjoining colored map frames, touch painting, pinch zoom, panning and frame creation/movement/resizing.
- Real Emerald visual tile galleries across the available 129 source-defined tileset combinations, filters by collection and verified behavior, collision/elevation editing, frame renaming and safe deletion.
- Native NPC, sign, warp and trigger event records; basic NPC/sign dialogue script generation, with explicit validation for incomplete script references or missing warp targets.
- Offline versioned JSON project autosaves, undo/redo, and a **Backup** action for unfinished projects.
- **Export** creates a ZIP with `custom_maps/<MapName>/layout.mapstudio.json`, map headers, scripts, connections and the project snapshot. All cells must be painted and validation must pass.
- The existing Python custom-map builder accepts numeric layouts and emits real map binaries and repository map registrations.
- A headless Rust CLI supports reproducible map edits and exports for AI agents/CI. The Android app and CLI share the same map model.

## Side-by-side Android preview builds

The current build is **Map Studio Spaces Preview** (`dev.despairarc.mapstudio.spacespreview`). This is a separate package from the earlier base, UX and Events Previews; it cannot overwrite or directly share their private saved projects. Preserve and export any valuable maps before uninstalling an older build.

CI still uses a temporary test signing key on every run. A later **Preview** APK may require uninstalling the previous Preview before installation. Do not uninstall an app containing valuable unsaved/unexported maps. Stable signing and safe project import/restore are separate unfinished tasks.

## Android file access

On Android 10+, **Backup** publishes a JSON file and **Export** publishes the game-source ZIP to **Downloads/MapStudio** through Android MediaStore, not app-private Android/data. The app's own working autosave remains private. Android 8–9 shared-file publishing is not supported yet; desktop builds save to a local directory. The Android Downloads bridge has passed compilation but needs a physical-device export test.

## Build

GitHub Actions: [Map Studio Android workflow](../../.github/workflows/map-studio-android.yml).
The workflow runs Rust tests, the CLI-to-Python map generator smoke test, desktop checks and an ARM64 optimized APK build.

```sh
cd tools/map-studio
cargo test --lib
cargo test --bin map-studio-cli
cargo check --all-targets
cargo apk build --release --lib --target aarch64-linux-android
```

For Android builds, install Rust ARM64 target, SDK/NDK, cargo-apk, Python 3 and Pillow. A release signing key is required. CI generates a disposable sideload certificate for each run; **uninstall a previous test APK before installing an APK signed by a different CI run**.

### Headless project example

```sh
cargo run --bin map-studio-cli -- new /tmp/village.json
cargo run --bin map-studio-cli -- add-map /tmp/village.json Village 0 0 10 10
cargo run --bin map-studio-cli -- fill /tmp/village.json 1 0x170
cargo run --bin map-studio-cli -- validate /tmp/village.json
cargo run --bin map-studio-cli -- export /tmp/village.json /tmp/village.zip
```

Run `cargo run --bin map-studio-cli -- help` for other commands.

## Remaining work

- Verify Android startup of this new version, actual writing to Downloads, autosave after restart, pinch behavior and performance.
- Add an Android system document picker for importing projects across unrelated app packages; local multi-project creation/switching and importing an accessible backup as a separate project are implemented.
- Expand per-tileset metadata, script editing, warps and native events; prove exported maps in an actual GBA ROM/emulator.
- Fix and verify safe small-map camera-neighbor copying before promising 5x5 map traversal.
- Verify **GitHub Contents Pull/Publish** on a real Android device, implement a secure user-friendly GitHub OAuth pairing flow (the current V1 uses an in-memory fine-grained token), add remote project discovery/clone; full collaboration is separate and still missing.

Keep this tool isolated from the Emerald ROM runtime.
## Phone editor UX (next build)

- Drag empty white canvas to pan in any mode; choose **Pan** only to move the view while touching a painted map.
- **+ Map** opens a compact Size panel. Set exact width/height and the direction to grow, then tap empty canvas to place the frame. The tapped point is the fixed opposite corner.
- Select a frame and choose **Size** to resize it in any of four directions; existing occupied cells/events stay at their world coordinates. **Fit** centers all map frames.
- **Properties** shows C0–C3 collision overlays or E0–E15 elevation overlays, with the selected cell's exact values. Properties require a painted metatile.
- **Events**: tap a cell to select/place; tap a type to change it; double-tap or use Edit details for the extended form.
- New installs start with an empty canvas; existing autosaves still load.
- The CLI supports `place-map` and directional `resize` using the same Rust geometry rules.

## Project recovery / import (v0.5 preview)

- Open **Project → Backup to Downloads** to save a versioned JSON snapshot, including unfinished maps.
- Open **Project → Restore backup** to browse backups in this app's Downloads/MapStudio and local recovery files. Game-source ZIPs containing `mapstudio-project.json` can also be restored.
- Open **Project → New blank project** to replace the canvas only after a local recovery snapshot is saved.
- All imports are validated *before* replacing the current project (20 MiB input cap, schema, map geometry, cells and event positions). The app saves the previous canvas to `MapStudio-recovery-*.json` before replacement; import failure leaves it unchanged.
- Android MediaStore only lists files owned/readable by the installed Preview package. It is **not** a general Android Storage Access Framework file picker and may not show backups from the older Map Studio package. Full system document picker and stable signing are still needed.
- The CLI supports `import <new-project.json> <backup.json|source.zip>` and `restore <project.json> <backup.json|source.zip>`. CLI restores also save recovery copies before overwriting.
- **Uninstalling Preview removes private autosaves and local recovery files.** Export important backups to Downloads, confirm they are accessible from a file manager, and keep a copy elsewhere before uninstalling. CI Preview test signing keys still change between builds.

## v0.6 gesture-first UX preview

- This build uses a separate **Map Studio UX Preview** package (`dev.despairarc.mapstudio.uxpreview`) so it does **not** overwrite older Preview installations or delete older projects. Its storage is independent.
- **Hold on empty canvas (about half a second), then drag** to draw a map rectangle. A red preview warns about overlap before release; invalid shapes cannot be committed. Drag immediately on empty space to pan.
- **Hold on an existing map, then move your finger** to drag the frame. Moving into another map is rejected while the warning appears. Quick drags across a map continue to paint.
- Select any map, then press **Size** to choose width, height and expansion direction. Potential overlaps or discarded content are warned about *while adjusting the values*, before Apply.
- Drag the wide **handle at the top of the lower panel** to set the palette height. Tapping the handle toggles expanded/collapsed heights.
- Collision uses the original numeric values **0, 1, 2, 3** (0 walkable, 1 blocking). Read-only ~, ice and ledge icons derive from the actual General/Petalburg metatile behavior attributes, not fabricated editable values. Elevation is a grid of choices 0–15, without a slider.
- NPCs use verified selectable characters with friendly names. Arbitrary character IDs are not editable. Default dialogue is empty; users write actual NPC/sign text. Warps and triggers are intentionally deferred.

**Still requires a physical phone test for gestures and panel sizing.** Emulator correctness and actual full ROM runtime behavior remain separate gates.

## Events preview: gestural editing and warp travel

**Map Studio Events Preview** uses package `dev.despairarc.mapstudio.eventspreview` and has independent storage. Older working builds need not be uninstalled. New CI test keys are temporary.

Gestures: quick drag on blank canvas pans; long hold then drag creates a map. Hold an existing map then drag moves it. **Existing event markers take priority over maps**: tap selects an event, long hold then drag moves it inside the same map, double-tap opens its properties. Double-tap a painted tile for its tile properties; double-tap an unpainted map tile for map settings; double-tap empty canvas to zoom. Drag the palette handle to resize; double-tap its handle to toggle compact/expanded. Palette tiles show details on long hold.

Events: **Person**, **Sign**, **Warp**, **Step message**, and **Hidden item**. Warp settings offer (1) link an existing native warp, (2) create a destination arrival-only warp marker on painted non-warp terrain, or (3) scripted **direct arrival** at a safe tile with no destination warp event. Linking can optionally install a reverse warp. Native warps show the **destination warp ID** on their source map tile, direct warp triggers show an arrow, and arrival-only markers show their own ID. Selected warps have a map-to-map line overlay and a readable destination label. Changing/removing native warp references preserves indexes or refuses unsafe operations.

Direct arrivals use Emerald's `warp MAP, x, y` script macro through an immediate coordinate event, not fabricated `warp_events` coordinate fields. Hidden items use verified item constants and a small reserved pool of source-declared unused **persistent** pickup flags, with unique allocation in the editor. Step messages generate a legitimate immediate trigger script. Manual entry of engine symbol IDs is not exposed. Map game export checks missing event configuration, map/warp references, destination tile safety, unique pickup flags and dialogue encoding.

**Caveats:** Touch gestures require on-device verification. A scripted coordinate event needs in-game ROM testing to confirm interaction timing on door/walk tiles. Reserved unused flags must be checked for conflicts when integrating with other mods. This test tool does not yet support every specialized NPC, weather trigger, hidden-item variant or secret-base event. Do not treat the green Rust tests as proof the generated ROM boots and travels correctly.

## v0.8 — source-defined tilesets, editing Spaces and responsive phone palette

**Spaces Preview** (Android package `dev.despairarc.mapstudio.spacespreview`) is a fresh side-by-side installation with separate autosaves. **Do not uninstall older preview builds containing maps you haven't exported.** Temporary CI signing certificates still prevent direct upgrades from other CI versions.

**Spaces are editor-only**, not Emerald dimension offsets. Every created map remains a separate native Emerald map with local (0,0) tile coordinates. The Surface editing Space groups outdoor maps so shared-edge walking connections are detected. **New Interior**, **New Cave floor**, and **Other** Spaces can visually overlap Surface or one another while staying independent; only maps in the *same* Space can form walking connections. Assigning parent maps visually anchors a child Space near a doorway. The **Overlay** control shows the parent map as faded, **non-editable** reference art. Warp events work across Spaces using their persistent map IDs. Existing saved project maps silently migrate into Surface with intact IDs, cells and warps; importing does not overwrite old content.

**Map visuals** now use an asset catalog generated directly from the checked-in `data/layouts/layouts.json` primary/secondary pairings, source tileset `metatiles.bin`, actual indexed `tiles.png`, `metatile_attributes.bin`, and JASC color palettes. Each pair gets an atlas; invalid/incomplete source metatile subtiles are excluded via a verified per-pair mask rather than rendered as fake tiles. A palette browse panel groups **Towns, Houses, Interiors, Caves, Outdoor & Special**, with filters for game-defined water/ice/doors/grass/paths. It shows image thumbnails rather than an ID search box and adapts its tile grid to available phone width. A source-backed tileset can be chosen per map; changing a painted map requires confirming re-interpretation of existing tile indices. Export writes the *actual* tileset symbols from the layout pair and raw border metatile IDs; collision/elevation bits remain in the map cells. The initial supported NPC choices also have first-frame previews constructed from actual source graphics and NPC palettes.

**Limitations:** This provides all complete pairs currently resolved from the repository's source layout declarations, not every unrelated artwork file or advanced engine asset. A detailed audit manifest records incomplete/omitted pairs. Some tilesets lack source graphics or palettes and are intentionally excluded. Character previews currently cover the curated NPC choices, not the whole Pokémon sprite library. Composite metatile priority layers remain an editor approximation rather than guaranteed hardware-accurate. Real-device GPU performance, touch behavior, gameplay warps, full GBA ROM build, live GitHub sync, and user-driven imports still require validation or implementation.

## Metatile Sheet Preview (experimental branch)

The terrain palette is now a **continuous native-order metatile sheet** with eight
columns, just as the source editor displays it: primary source art and secondary
source art have their own sections, and invalid IDs remain in their original
positions instead of shifting nearby artwork into a compact sorted gallery.
There is no artificial 32-tiles-per-page limit.

- Tap a single metatile to paint it.
- Use **Select group** and drag across a rectangle of the source sheet to
  paint a multi-metatile block, such as a roof or tree, with original adjacency.
  Group mode is deliberate so a normal phone swipe can still scroll the sheet.
- Choose **Both**, **Primary**, or **Secondary** to browse the matching source.
  The active map's primary/secondary pair is always authoritative.
- Behavior and source-label filters now **dim non-matching cells in place**,
  rather than regrouping the sheet or breaking up buildings.
  Buildings*, Trees*, and Rocks* use only metatiles with matching names
  declared in `include/constants/metatile_labels.h`; unnamed artwork remains
  browseable and is **not** claimed to have been recognized automatically.
- Selected multi-cell stamps are clipped to the map under the initial
  touch, without leaking into a different neighboring map or tileset.
- Atlas building now respects indexed palette-zero transparency on **both**
  metatile layers. The flattened preview still does not simulate all GBA
  BG1/BG2/BG3 sprite priority or animated tiles; emulator parity is pending.

**Sheet Preview** uses `dev.despairarc.mapstudio.sheetpreview`, separate
from the existing Spaces Preview package and local autosaves. It is a
development test build, not a verified user-device release.

## UX Fix Preview — event visibility, terrain tools, properties and Android resizing

Development branch: `feature/map-studio-ux-visibility-events-properties`.
Android package `dev.despairarc.mapstudio.uxfixpreview` uses **separate private
autosaves** from the Spaces and Sheet Preview APKs. Do not uninstall those
apps before backing up maps. CI test signing keys change between builds.

### Terrain, filtering and gestures

- The Terrain tool is now **Brush / Bucket / Eraser**. Bucket flood-fills
  only the four-way-connected cells sharing the same metatile ID within
  the touched map; it does not cross map boundaries or overwrite existing
  collision and elevation values. A multi-tile stamp must be reduced to a
  single tile before using Bucket. Undo is supported.
- A tile-sheet **tap** chooses one tile; **hold (~0.46 s) then slide**
  selects a bounded rectangular source-metatile stamp. A quick swipe scrolls.
  There is no separate Select group toggle or Both/Primary/Secondary control.
- The Find filter dims nonmatches **without removing them from their native
  source-sheet positions**, counts actual source-pair matches, and has up/down
  jump controls. Grouping selects the first source tileset of that family;
  changing a painted map's pair requires confirmation to avoid reinterpretation.
- Existing cells retain collision and elevation when terrain is repainted.

### Events

- Event sprites are fixed in **screen points**, not multiplied by world zoom.
  Events only render and accept touches in **Events** mode; their source tiles
  can always be painted in Terrain and Properties. NPC selection outlines
  are thin rather than a tall opaque frame. Signposts, door/warp icons,
  scripted-step diamonds and real selected-item icons are visually distinct.
- Marker hit-testing is screen-space in Events mode, so zooming out does not
  reduce the touch target to a tiny metatile. Blank canvas taps deselect.
- Supported authoring is still deliberately limited to (1) person with simple
  dialogue, (2) sign with simple text, (3) linked/arrival/direct warp,
  (4) immediate step-message trigger, and (5) selected one-time hidden item.
  This is **not all Emerald event scripting**. Trainer battles, NPC movement
  schedules, complex conditional scripts/variables, secret bases, advanced
  field effects, and other specialized background/object events are not fully
  implemented. Do not infer gameplay correctness from APK packaging.

### Properties, window mode and device acceptance

- Properties includes visual collision choices 0–3 and elevation choices
  0–15, live selected-cell art, and source-backed read-only behavior and
  metatile layer type. There is no separate Emerald terrain/encounter-type
  field in the 16-bit attribute file for these maps. Metatile source data
  is not a per-cell editable substitute for the map-block properties.
- Low-zoom property markers and the selected tile retain a visible outline.
- Short/floating/split-screen layouts use a compact header, less fixed inset,
  a bounded bottom dock and resize-triggered gesture cancellation/redraw.
  Android NativeActivity requests resizable handling and supported
  configuration changes. This is **build-tested, not yet physically proven**
  under split-screen/freeform on the owner's Samsung device.

On-device acceptance: launch; resize to split-screen/freeform and back;
pan/pinch and tap after resizing; verify icons and event selection at zoom
minimum; paint under an event; inspect collision/elevation at low zoom;
verify source item graphics; check Bucket boundaries and Undo; ensure Find
arrows scroll to real matches; check that older installations retain autosaves.

## Guides Preview — zoom-relative overlays and compact map navigation

Branch: `feature/map-studio-responsive-visibility-coordinate-ux`. This preview
uses a separate Android package `dev.despairarc.mapstudio.guidepreview`;
existing UX Fix/Spaces/Sheet Preview project data is **not shared automatically**.
Back up or export the old project before changing apps. CI signing keys remain
temporary; this branch's APK is a test build, not a persistent update channel.

- The source-grid is drawn over the decoded metatiles, so map grids remain
  visible during zoom-out instead of being covered by the terrain texture.
- World/editor coordinate labels appear along the canvas top and left edges
  without opaque backplates. They use a 1/2/5/10/20/50/100… UI-only interval
  that adapts to zoom; (0,0) has a small outlined origin target. Source map
  coordinates, exported layouts and Space offsets remain unchanged.
- The minimum zoom is 2 points per metatile (previously 5). The chosen map
  is identified in the header and gets a prominent blue/white outline.
- NPCs and event icons again scale relative to their **map metatiles** instead
  of using fixed screen dimensions. Event hit regions scale with the visuals
  but have touch-friendly minimum dimensions.
- Independent visibility toggles show/hide event sprites and collision/elevation
  overlays regardless of editing mode. Event controls work **only in Events**,
  tile property painting **only in Properties**, and terrain painting **only
  in Terrain**. Turning overlays off also disables invisible event hitboxes.
- At low zoom, property visuals use real per-cell collision/elevation tint and
  grid strokes; fabricated floating color dots have been removed. Collisions
  2 and 3, and elevation values 0–15, retain their source numeric meaning,
  without invented names such as "Special", "All" or "High".
- A quick drag in Events pans the canvas, including when a marker is at the
  touch origin. Long-holding a marker still moves it, and long-holding a map
  still moves the map. "Place" was a heading, not a tool; it now says "Add event".
- "Open destination warp" switches Space if needed, identifies the linked
  destination warp via native warp enumeration (or direct arrival coordinates),
  selects the event or tile when possible, and centers the camera there.
- The bottom panel's resize handle and **one grouped tileset selector** are
  fixed; only its tool controls and metatile artwork scroll. Changing a
  painted map's tileset still requires confirmation.
- The five simplified event-authoring types and source attributes remain
  the existing set; this iteration does not invent missing engine features.

**Validation:** CI checks Android packaging, Rust tests and desktop compiling.
Device acceptance still requires Samsung split-screen/freeform testing, since
a build cannot demonstrate live touch correctness in a resized NativeActivity.

## Resize + Keypad Preview — collision-aware editing

Branch: `feature/map-studio-resize-keypad-and-dock`. Android package
`dev.despairarc.mapstudio.resizepreview` deliberately keeps a separate
private project file from earlier test APKs. Export/backup a project before
switching packages. The CI APK is signed with a temporary test key.

- The **map size dialog uses in-app Width and Height buttons** and an
  on-screen 0–9, Clear and Delete pad instead of the system keyboard for
  those two numeric values. The map name remains a normal text field.
- For each candidate width/height/direction, the UI uses the **same source
  model `preview_resize` validation as Apply**, including map overlap,
  coordinate limits, painted-cell preservation and event preservation.
  Invalid sizes show the reason, a red geometric preview and a disabled
  Apply button. The model computes a maximum valid width for the current
  height and vice versa (up to the current UI limit of 100). Direction
  choices that cannot accommodate the requested growth are disabled.
- A **miniature live geometry preview** shows nearby maps, the original
  map footprint, and the proposed ghost shape (green valid, red invalid).
- On the canvas, **long-press near a map edge then drag** to resize one
  dimension. Long-press inside the map continues to move it. Edge resize
  remains a preview until finger release; rejected geometry never writes
  to map cells/events. At very small on-screen map sizes, edge-resize
  deliberately yields to the regular map gesture to avoid ambiguity.
- The (0,0) indicator is now the outline of the **actual first grid cell**,
  scaling with zoom, rather than a floating fixed-radius target.
- Sign, warp, step-message and hidden-item icon decals now scale smaller
  than the grid cell; NPC sprites retain natural source proportions.
- The Properties tab no longer shows non-editable technical metatile
  details. The actual numeric collision and elevation controls remain.
- The terrain dock keeps its resize handle, tileset chooser, Brush/Bucket/
  Eraser controls and Find navigation fixed. Only the source metatile
  sheet scrolls; quick drag scrolling adds decaying momentum while a
  held selection cancels that momentum.
- Overlay visibility has a persistent small window: checking or
  unchecking a box leaves it open. It closes through X, the Overlays
  toggle, or an outside tap.

Tests and boundaries: numerical geometry guidance has Rust regression
tests and participates in the normal desktop/Android CI. Device gesture
feel and Android multi-window behavior still require real-phone testing.
No new gameplay mechanics or non-source Pokémon metatile properties
are introduced.

## Paint & overlays preview (experimental, source-backed)

Branch: `feature/map-studio-touch-paint-overlays-warp-ux`. APK application ID
`dev.despairarc.mapstudio.paintpreview` is deliberately separate from earlier
preview APKs and their local autosaves. Export or back up old projects to move
data between previews. This is a test build, not an update-in-place package.

- **Tools can be deselected.** Tapping an already selected Brush, Bucket or
  Eraser toggles it off, leaving a neutral pan-only Terrain canvas. A sheet
  metatile tap reselects Brush. Bucket flood fill is one-shot: it reverts to
  Brush after a valid fill. Source metatiles and per-cell properties remain
  untouched by choosing a different tool.
- **Overlay checkboxes persist across all tab switches**; a hidden layer
  remains hidden until explicitly checked again. Only the relevant editing
  tab accepts touches on that layer.
- **Visual-only map properties:** the transparent collision palette keeps
  the actual values 0–3 (green/red/purple/amber). Elevation uses a 16-color
  graduated palette for real source values 0–15. No numeric or behavior
  characters are painted over the terrain; the values are not invented.
- **Art scrolling:** the source metatile sheet uses egui's native touch drag
  scrolling without the previous doubled `scroll_with_delta` plus manual
  momentum. The 0.46-second hold still selects a native-order stamp. The
  broader resize grabber and painting tool / Find controls stay pinned.
- **Visual event choices:** large source-art or representative icon cards
  for Person, Sign, Warp, Step Message, Hidden Item. These are only the
  event kinds currently implemented, not a claim to cover all Emerald
  scripting. NPC and item previews are taken from the checked-in source.
- **Warp connections:** selecting a linked warp draws a line to its actual
  destination tile if both maps share a Space. Across independent Spaces,
  it draws a clearly labelled link stub naming the destination Space and
  map, without inventing adjacent world coordinates.
- The `Rustboro`, `Slateport`, etc. artwork labels are **real source
  tileset names**, not map connections. The selector now clarifies this.

CI validates Rust model behavior, desktop UI build, and Android packaging.
On-device touch/scroll smoothness and resize behavior require phone testing.

## Numbered Android APK artifacts (v0.13 onward)

Each editor release now has a numbered `Cargo.toml` version. The current
Paint Preview is **v0.13** (`0.13.0` in Cargo); advance to `0.14.0`,
`0.15.0`, and so on for subsequent releases. The Android CI workflow
automatically derives the public filename from that version and the
application label, such as `MapStudio-v0.13-Paint-Preview.apk`.

The GitHub Actions **downloadable ZIP contains only the final signed APK**.
`cargo-apk` internally builds an `-unaligned.apk` intermediate; it is not
needed for sideload installation and is intentionally excluded from upload.
The workflow checks APK ZIP integrity, confirms an embedded native library,
and verifies its signature with Android's `apksigner` before publication.

Temporary CI signing certificates still change between runs. Numbered APK
filenames are **not** a fix for signing consistency or automatic upgrades.
Back up/export valuable maps before uninstalling a previous preview.

## v0.14 — two direct artwork selectors

The Terrain dock has **two independent, touch-sized dropdowns** — **Category**
(Towns, Houses, Interiors, Caves, Outdoor & Special) and **Tileset** (the
actual source-backed entries in the selected category). No nested expanders,
"No destinations" explanation, "Use first in category" button, or collapsing
menus are needed. The entire choice row accepts taps.

Changing Category selects the first genuine source tileset in that family,
updates the Tileset control and shows its atlas in the same frame. Changing
Tileset selects that pair directly. If the selected map is already painted,
the new art is previewed without reinterpreting stored IDs until Apply is
confirmed. Cancel returns to the prior map tileset. The source metatile
catalogue and map export rules are unchanged.

Android package: `dev.despairarc.mapstudio.artworkpreview`, kept separate
from prior test installations to preserve private saves. Release v0.14 is
derived from `Cargo.toml`, and CI uploads **only the final signed APK**
without the unaligned intermediate. Live phone touch testing remains needed.

## v0.15 — clearer artwork previews and Android-safe dock

- Browsing a new source tileset on a previously painted map is a **preview**,
  not a request to turn the map into a named town or venue. The two selection
  boxes immediately show the chosen source art, but the underlying map retains
  its old tile IDs until the owner explicitly presses **Use artwork**. The
  UI now states that the named map is unchanged and provides **Revert**.
  The former misleading "Change existing map to Battle Pyramid?" question
  has been removed from the painting toolbar.
- There is exactly **one** bottom-dock grab handle. Its size is constrained
  to available window height after header and a small Android navigation
  safety inset; fullscreen, split-screen and small freeform windows cannot
  ask for a dock taller than the available region.
- In normal windows, the artwork selectors and Brush/Bucket/Eraser/Find
  remain fixed while only the metatile gallery scrolls. In unusually short
  window heights, the grip stays anchored and the remaining controls move
  into a bounded scrolling region instead of overflowing beneath the
  Android gesture/navigation bar.
- New Rust dock-bound regression tests cover very short and full-height
  windows. Physical-device split-screen acceptance is still required.

The v0.15 test APK has its own package
`dev.despairarc.mapstudio.dockpreview` so it cannot overwrite private saves
from older preview installations. The CI artifact contains one signed APK,
not cargo-apk's unaligned intermediate.

## v0.16 — compact controls, real NPC movement and portable projects

- Tapping the active Terrain/Properties/Events tab enters a **neutral Pan**
  state. Explicit Pan button works in any tab; reselect the property brush
  or event card to author again. Overlay checkboxes remain independent.
- Normal collision tools show Passable (0) and Blocked (1) only; legacy
  collision 2/3 values remain intact when loading/re-saving existing maps.
- Art category and tileset dropdowns share a row; header shortcuts and map
  selection are compact and keep a usable touch target.
- NPCs expose a verified subset of the existing
  `MOVEMENT_TYPE_*` source constants plus X/Y movement ranges.
  `map.json` export writes the chosen fields rather than fixed face-down.
- `Project > Export editable ZIP` saves unfinished project state as a
  portable `mapstudio-project.json` archive (no fully painted map
  required). `Import backup / ZIP` restores verified archive/JSON data
  and creates a recovery copy of the current project first.
  `Project > Export game-source ZIP` remains stricter and may reject
  incomplete or unsupported map events.
- CLI supports `export-project` and round-trips the same editable format.
- See [source feature gap audit](docs/SOURCE_FEATURE_GAP_AUDIT.md)
  for a truthful list of complete, limited and absent Emerald features.
- The Android import list currently browses readable project backups
  written under this installation's Downloads/MapStudio, **not arbitrary
  files through the Android system picker**. Cross-preview backups
  must be transferred through a supported readable location.

## v0.17 — artwork library and source-backed prefab collection

- **Collection → swipeable Category → Tileset → Metatiles / Prefabs** replaces
  the previous two-selector-only Terrain browser. Emerald is the only
  built-in graphics collection until complete, verified external resources
  are imported.
- The **71 original Porymap Emerald prefab definitions** are bundled with
  Porymap LGPL-3.0 attribution and rendered from this fork's native source
  atlas; each prefab is filtered by source symbol and valid tile IDs. Tapping
  one stamps its real metatiles, collision and elevation, without reaching
  other maps.
- Export a **single source-complete artwork ZIP** from the Pack menu, or
  import a validated one from this app's accessible Downloads/MapStudio
  folder. Imported external art can be browsed, but cannot paint game maps
  until a native converter/validator exists. A byte-identical Emerald
  package provides an explicit "Use matching Emerald source" action.
- Imported packages persist globally in app-private storage, separate
  from project autosaves, and don't rewrite native map IDs.
- See [artwork library architecture and provenance](docs/ARTWORK_LIBRARY_V1.md)
  for compatibility guarantees, all researched external sources, and work
  remaining before FireRed/Unbound artwork can be inserted into Emerald.

## Additional built-in art after v0.17

The current development branch now bundles **Emerald**, **FireRed/LeafGreen**,
**Ruby** and **ORAS Style** in the APK. The latter two are source-complete,
offline, browseable and exportable source packs, not user-imported placeholders:
Ruby contains Petalburg/Mauville/Rustboro reference pairs with verified source
IDs and Porymap prefab data; ORAS contains the same three towns using LeoB's
credited artwork from Team Aqua's Asset Repo. Native painting remains available
only for this fork's actual Emerald and FRLG tileset pairs. Do not silently
paint Ruby/ORAS IDs into an Emerald map before game-side integration is proven.

Do not describe a passing APK build as a full ROM export or physical-device
test. Full extra-collection conversion and third-party tileset license checks
are tracked in `docs/ARTWORK_LIBRARY_V1.md`.
