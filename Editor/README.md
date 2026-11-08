# Editor — Canonical Map Studio project workspace

This directory holds the editable state shared between the Android editor, the
headless Rust CLI and authorized AI agents. A project is a **whole GBA game
world**, potentially containing many maps, spaces, NPCs, connections and events.

```text
Editor/
  active-project.txt                 # Local/app selection, not a remote command
  projects/
    <project-uuid>/
      manifest.json                  # Permanent project ID and target source
      project.json                   # Versioned Map Studio World data
```

The Android app creates the same relative structure in its private storage.
It does **not** automatically synchronize private Android files with GitHub.
An explicitly authorized GitHub transport is needed for remote synchronization.

## Identity and target safety

- The UUID, not the display name or GitHub branch, determines project identity.
- `World.maps[].id` is only unique within that UUID. No accidental cross-project
  warp/connection references.
- `target` is currently `emerald_expansion` or `fire_red` (Rust enum uses
  `fire_red` in JSON); the manifest also stores the target repository and an
  optional pinned full commit SHA. **FireRed publishing remains disabled**
  until a compatible target game compiler is proven.
- The editor's metatile IDs and real tileset symbols are retained exactly.
  Using FRLG graphics already present in an Emerald fork does not turn an
  Emerald export into a FireRed ROM export.
- AI agents must read and validate the matching manifest **before editing**
  that project's `project.json`, then use the existing map-studio-cli for
  supported operations. Preserve the stable UUID and schema version.
- Project files are validated, capped at 20 MiB, saved atomically and never
  replaced on an invalid import.
- A legacy `map-studio-project.json` is copied to a new UUID workspace on the
  first run; the original remains untouched for recovery.

The active-project selector is **device-local preference**, not a GitHub remote
instruction. Do not change another user's active project by updating it.

## CLI

From `tools/map-studio`, with <root> pointing at this repository root:

```sh
cargo run --bin map-studio-cli -- workspace-new ../.. "Emerald World" emerald
cargo run --bin map-studio-cli -- workspace-new ../.. "FireRed World" firered
cargo run --bin map-studio-cli -- workspace-list ../..
cargo run --bin map-studio-cli -- workspace-validate ../.. PROJECT_UUID
cargo run --bin map-studio-cli -- workspace-publish ../.. PROJECT_UUID
```

Ordinary editing commands still operate on the single canonical JSON path:
`Editor/projects/PROJECT_UUID/project.json`.

No stub projects or sample game maps are committed here. Only user-created
projects should populate `Editor/projects`.
