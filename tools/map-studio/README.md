# Map Studio — Native Rust Android editor

Development branch: `feature/map-studio-native-android`.
Engineering baseline: [design/map-studio-implementation.md](../../design/map-studio-implementation.md).

This is a working native `eframe` / `egui` / `wgpu` Android prototype with real Pokémon Emerald General + Petalburg metatile art. It is **not** a fully validated GBA ROM tool or a live GitHub collaboration client.

## What's implemented

- White-grid canvas, adjoining colored map frames, touch painting, pinch zoom, panning and frame creation/movement/resizing.
- Real Emerald metatile-ID palette (compiled from actual tileset graphics and metatile binaries), collision and elevation editing, frame renaming and safe deletion.
- Native NPC, sign, warp and trigger event records; basic NPC/sign dialogue script generation, with explicit validation for incomplete script references or missing warp targets.
- Offline versioned JSON project autosaves, undo/redo, and a **Backup** action for unfinished projects.
- **Export** creates a ZIP with `custom_maps/<MapName>/layout.mapstudio.json`, map headers, scripts, connections and the project snapshot. All cells must be painted and validation must pass.
- The existing Python custom-map builder accepts numeric layouts and emits real map binaries and repository map registrations.
- A headless Rust CLI supports reproducible map edits and exports for AI agents/CI. The Android app and CLI share the same map model.

## Side-by-side Android preview builds

The current Android package is `dev.despairarc.mapstudio.preview`, displayed as **Map Studio Preview**. This is deliberately separate from the older `dev.despairarc.mapstudio` app, so the owner can compare UX changes without removing previously autosaved work. The two packages **do not share** saved projects. The old test APK and this Preview can remain installed simultaneously.

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
