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

- GitHub account OAuth/device flow or fully automatic private-repo access.
  The app currently supports **manual, in-memory fine-grained GitHub token**
  entry. It intentionally does not save tokens on the phone, so private-repo
  access must be authorized after each restart.
- Actual end-to-end Android/device networking verification on both Wi-Fi and
  mobile data and a remote GitHub integration test.
- Live collaboration or webhook push; the app currently checks project
  revisions periodically, using GitHub's Contents API and SHA guards.
- FireRed-native game-source compiler, additional event types and a verified
  ROM-build/emulator smoke test.

Do not report local project switching as live GitHub sync. Do not report
generated native files as a successfully built Pokémon ROM.

## GitHub sync V1 implementation

In **Project → GitHub sync**, set the repository (owner/repo) and branch.
Use a fine-grained GitHub token restricted to that repository with **Contents
Read and Write** to publish, or **Contents Read** to pull a private repo.
Public repos can be read anonymously at reduced rate limits; project data
in a public repo is public too. The access token lives only in process memory
and is never included in the project manifest, GitHub link file or sync cursor.

The app publishes *only* `Editor/projects/<uuid>/manifest.json` and
`Editor/projects/<uuid>/project.json`. The project manifest must match the
active local project exactly before syncing. Existing file SHA is supplied
to GitHub during update, and locally the last synced SHA-256 content hash is
checked before applying any downloaded revision. On remote conflicts, no
project is overwritten. Incoming valid changes are backed up before install,
and can be auto-checked while Map Studio is open. A remote project must first
be initialized from the phone (Publish) or have matching UUID/content.

This is **not** a streaming session or direct ChatGPT-to-phone API; AI agents
with authorized GitHub access can update the same GitHub project folder.
Map Studio will then find the new project revision on its next check.
