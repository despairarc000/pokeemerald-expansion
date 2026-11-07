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
