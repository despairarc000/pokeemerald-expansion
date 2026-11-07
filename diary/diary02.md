# Diary 02 (logs 11 to 20)

The story of the project in order. Short on purpose. Dates are by session. For the routine, see `ONBOARDING.md`. Earlier history is in `diary01.md`.

## Log 11: The world should feel real (2026-11-05)
The owner clarified what "free exploration" means for the map: the world should feel like the real world, not like Pokemon-coded maps. Terrain restricts movement because of what it is (a dense forest is slow, a waterfall drop is insane), never to guide or gate the player. We recorded this as "world feel rules" in `features/regions-and-exploration.md`: barriers are costs, not locks, no cut-tree or boulder gates, paths follow valleys and rivers, settlements sit where people would live. The mainland is continuous, with natural barriers that cost something.

## Log 12: Fix shared-edge map connection boundary (2026-11-06)
The owner asked to fix the Emerald engine's off-by-one connection lookup directly on `master`, without a branch, so it would not be forgotten during mobile map-editor work. In `src/fieldmap.c`, `IsCoordInIncomingConnectingMap` now treats the upper bound as exclusive (`coord < max`), matching the engine's other connection range check. Example: a 10-high map with two 5-high neighbors on the right now selects the first for rows 0–4 and the second for rows 5–9 without double-matching row 5. Engine commit: `eeec45dc613d8486c3cf1639619ce8e1a9f466a6`. Boundary-case logic was checked; native build and emulator behavior are **not yet verified**. This direct-to-master engine fix was an explicit owner-requested exception to the usual branch rule.

## Log 13: The AI becomes the author, and the pipeline (2026-11-06)
The owner made the AI the author of the lore and of the look and feel of each region, and said they want to experience the world as a player. The AI keeps new lore out of the chat. The first Keepers draft was already shown, so it is spoiled and may change. For maps, we chose a text-grid approach: `tools/textmap/build_textmaps.py` turns a text map into the engine's binary data during the build, and a first test map was added. The pull request was merged by the owner about 11 minutes after opening, with no CI checks recorded, so **the pipeline is on `master` untested**. The AI cannot write binary files, so new tiles will be generated elsewhere (ChatGPT), and the owner and ChatGPT are building a separate map maker.

## Log 14: Meadowmere designed down to the natural maps (2026-11-06)
At a high level the AI wrote the region brief, then the seven areas, the four main places plus nineteen smaller ones, and 28 interiors. The owner chose to do natural lands and waters first, with no houses or buildings. The AI then designed 20 natural maps (tile roles instead of tile ids, size classes, exits, terrain mix, wildlife behavior, curiosity-slot counts) with reserved gaps where villages will sit. The owner confirmed that natural lands and waters are done. Villages, species per zone, curiosity contents and real tiles are not part of that step.

## Log 15: Next regions stay shallow (2026-11-06)
The owner decided that from here on each region is designed only as natural lands and waters, and only coarsely, so that a revision does not waste effort on intricate detail. No places, interiors or per-map terrain percentages for now. Mistvale was done this way: a region brief, six areas, and a coarse list of 17 natural maps (name, size, danger, one line) in `design/region-mistvale.md`. The depth of Meadowmere stays as the reference for how deep a region can later go.

## Log 16: Map Studio Rust APK design consolidated (2026-11-07)
The owner asked for a final review before implementation of the Android map maker. We selected a native Rust approach (eframe/egui with wgpu), retaining the HTML v3 UI as visual reference, with one Rust model for editing, autosave, AI commands and real Emerald export. The implementation baseline is `design/map-studio-implementation.md`: first prove a minimal APK on the phone, then the canvas, actual tilesets, native export and transitions. GitHub snapshot collaboration precedes a possible hosted live service. Known uncertainties—device performance, untested native export, and the engine's unsafe small-map neighbor copying—remain tests, not completed features. **No APK code, service or ROM build was produced in this session.**

## Log 17: First native Map Studio APK compiled (2026-11-07)
The owner explicitly initiated implementation of the native Rust map maker. The first working code is on branch `feature/map-studio-native-android` under `tools/map-studio/`: `eframe`/`egui` with `wgpu`, a white grid, three gray adjoining frames, provisional color-tile painting, collision overlays, event markers and touch pan/pinch UI. The pure Rust world model prevents map overlap, packs Emerald collision/elevation fields and calculates shared-edge connections. GitHub Actions run `42319371555` passed its Rust unit tests, desktop compile and ARM64 Android APK packaging; downloadable artifact `map-studio-arm64-debug` is available in that run. This is **not device-tested**: launch, touchscreen behavior, performance and lifecycle handling remain unverified. The tiles are placeholders, and the app does not yet support persistent saves, actual tilesets, GBA export, GitHub synchronization, or live collaboration.

## State (as of log 17)

**Done**
- Meadowmere natural environment design, down to 20 natural maps (`design/region-meadowmere.md`, `meadowmere-areas.md`, `meadowmere-places.md`, `meadowmere-interiors.md`, `meadowmere-maps-natural.md`).
- Mistvale natural lands and waters at the coarse level (`design/region-mistvale.md`).

**Open**
- The map pipeline (`tools/textmap/`, hook in `map_data_rules.mk`) is on `master` with no CI result seen. Owner to check the Actions tab, or the AI to find another way to trigger a run.
- Owner to confirm the world clock policy (in-game time only, no offline catch-up).
- Emulator smoke test to close Gate A.
- Verify the shared-edge connection fix (log 12) in a native build and emulator. It is committed but not gameplay-tested.
- Pull request #2 is obsolete and should be closed unmerged. Upstream pull request #11032 should be confirmed closed. Old branches can be deleted.
- Villages and other buildings are not designed in any region.
- Species per zone not assigned. The species list has not been checked in the repo. Rarity tier numbers are AI estimates.
- Real tile art and the tileset pipeline. The map maker's format is unknown to the AI.
- Map Studio: ARM64 debug APK compiled successfully in Actions run `42319371555` from the feature branch. Owner to install and smoke-test the app; actual Pokémon tilesets, saves/export and collaboration are not yet implemented. See `design/map-studio-implementation.md`.
- Healing and rest rules, where the player begins, and which regions have gyms are undecided.
- Owner to confirm the world layout sketch (diary log 10) and whether the "world feel rules" become a core design rule in `design/CORE.md`.
- Ladder rows were drafted by the AI and need owner review.
- `design/trainers-and-gyms.md` still needs the "minority are persistent battlers" line.
- Regions still to do at the natural level: Cragmoor, Sunreach Steppe, Neonport (coast and surroundings), Sandara, Tundrael, Verdant Maw, Veyra, The Abyss, Duskhollow, Undervale, Skyreach, Ur-Kalah.

**Next**
- Owner picks the next region, or the AI proposes one by adjacency (Cragmoor and Sunreach Steppe border Meadowmere).
