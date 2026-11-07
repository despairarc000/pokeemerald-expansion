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