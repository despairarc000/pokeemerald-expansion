# Map Studio — Rust Android foundation

Implementation branch: feature/map-studio-native-android

This is **the first native prototype**, not a functional Pokémon map exporter.
The visual cells are provisional colors, **not** decoded GBA metatiles.

## Structure

- src/model.rs — real tile-grid geometry, collision-bit packing, no-overlap checks, adjacency, tests.
- src/ui.rs — native egui canvas, painting, modes, pinch/pan, frames, tile palette.
- src/lib.rs — Android NativeActivity entry point and shared app.
- src/main.rs — desktop entry point for visual debugging.

## Build

The GitHub Actions workflow \`.github/workflows/map-studio-android.yml\` builds the Rust crate for desktop checking and Android ARM64 APK. Use the workflow artifact to install the debug APK after CI succeeds.

For local Android build, install Rust, an Android SDK/NDK and cargo-apk, then:

\`\`\`sh
cd tools/map-studio
cargo test
cargo apk build --lib --target aarch64-linux-android
\`\`\`

The APK will be unsigned or debug-signed depending on cargo-apk's packaging configuration. This is for sideload testing, not Play Store publication.

## Not included yet

- Real tilesets, binary map import/export, warp/script generation
- Persistent autosave, GitHub sync, live collaboration
- Game engine camera-buffer fix for tiny maps
- Device-verified 60 FPS or correct Android suspend/resume

See [the implementation plan](../../design/map-studio-implementation.md).

This tool is isolated from Emerald runtime and does not change ROM code or its build.
