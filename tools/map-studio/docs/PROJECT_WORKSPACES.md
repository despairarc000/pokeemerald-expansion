# Project isolation and source publication — review and simulation

## Requirements and invariants

1. Top-level `Editor/` is authoritative editable data, `Compiled/` only
   validated output. The Android editor uses equivalent local directories.
2. Every project has a stable random UUID independent of its name, map IDs
   and target GBA game. Two worlds can have identical in-world map IDs.
3. Every project is permanently bound to a declared target engine, repository,
   and optional 40-character source commit. Cross-engine compilation is
   blocked until a compatible exporter exists. External artwork collections
   do not silently remap original numeric metatile IDs.
4. The first launch copies (never deletes) the old single autosave.
5. Switching projects first validates the destination, saves the current
   world, and only then changes the active project ID. Undo and gesture
   state must not leak between projects.
6. A corrupt project, malformed manifest, invalid UUID or missing
   destination stops the operation. Do not replace it with an empty world.
7. Output maps are namespaced by project ID to prevent game symbol collisions,
   and `Compiled/projects/<id>` is swapped atomically only after complete
   validation. Failed publication must preserve the previous output.
8. Native map files are **source artifacts**. A successful ROM build/emulator
   test is a separate acceptance gate.

## Simulated workflows

| Scenario | Expected result |
| --- | --- |
| Existing Android install with one map | Loads and copies old JSON into one UUID; old file unchanged |
| New Android install | Creates an empty, named Emerald project; no demo maps |
| Two projects with map ID 1 | Both remain independent; switching loads only chosen world |
| Switching away with unsaved changes | Saves current JSON before active selector changes; failure aborts |
| AI modifies a GitHub project | Edits only that UUID under Editor; phone needs separately implemented authenticated sync to receive it |
| FireRed project attempts Emerald compilation | Explicit error; Compiled has no FireRed output |
| Incomplete map attempts source publish | Validation fails before source directory changes |
| Same project published twice | Single stable Compiled/<uuid>; replaces only that project's output |
| Corrupted legacy autosave | Bootstrap stops, no empty file silently replaces it |
| Map named Town in two different projects | Export names include different stable UUID namespaces |

Automated Rust tests in `projects.rs` and `compiled.rs` cover isolation,
migration, corrupt input, malformed IDs, repeat publication and blocked
FireRed export. GitHub Actions checks build the Android and desktop targets.

## Scope not yet delivered

- GitHub credential pairing or a phone-to-GitHub transport: the repository
  folders and CLI are ready, but the phone is not remotely connected.
- Optimistic GitHub file-blob SHA conflict handling, automatically pulling
  project revisions, secure GitHub writeback, collaboration.
- FireRed-native game-source compiler, additional event types and a verified
  ROM-build/emulator smoke test.

Do not report local project switching as live GitHub sync. Do not report
generated native files as a successfully built Pokémon ROM.
