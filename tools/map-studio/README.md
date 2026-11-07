# Map Studio — Native Rust Android editor

Development branch: `feature/map-studio-native-android`.
Engineering baseline: [design/map-studio-implementation.md](../../design/map-studio-implementation.md).

This is a native `eframe` / `egui` / `wgpu` Android editor with source-generated visual tileset atlases for 129 of the 137 layout-declared primary/secondary combinations currently resolvable in the repository. Its nested editing Spaces and per-map tilesets do not change the Emerald engine's map-local coordinates. It is **not yet** a fully playtested GBA ROM tool or a live GitHub collaboration client.

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
- Add an Android document picker/project importer and full edit workflow for multiple saved projects.
- Expand per-tileset metadata, script editing, warps and native events; prove exported maps in an actual GBA ROM/emulator.
- Fix and verify safe small-map camera-neighbor copying before promising 5x5 map traversal.
- Implement **GitHub Pull/Publish** with secure user authorization, then a separate live collaboration service and API. **No automatic AI-to-phone sync exists yet.**

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
