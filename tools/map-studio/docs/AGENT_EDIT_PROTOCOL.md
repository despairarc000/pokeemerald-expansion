# Map Studio v0.22 — Shared Editing Transactions (AI + touchscreen)

**Status:** Implemented in Rust. Same canonical World model and native Emerald
validators are used by the Android UI and CLI. The JSON transaction interface
is currently a *headless CLI* API, not an on-phone automation endpoint.
It does **not** provide remote WebSocket/live collaboration; that is phase 3.

## Why this exists

A mapping agent should not need to memorize metatile indexes or imitate human
touch gestures. It can discover native art, inspect map cells and a rendered
image, preview a complete change, validate what would happen, commit only a
known project revision, and undo the last local transaction. There is still
one map representation: native project JSON / World / Emerald cells and actual
source-pair art. No alternate or "AI-only" tile data is created.

## Discover and inspect

From tools/map-studio:

    cargo run --bin map-studio-cli -- revision Editor/projects/<id>/project.json
    cargo run --bin map-studio-cli -- assets-json Editor/projects/<id>/project.json 1
    cargo run --bin map-studio-cli -- asset-json Editor/projects/<id>/project.json emerald:environment:pond-bank-route104
    cargo run --bin map-studio-cli -- inspect-region Editor/projects/<id>/project.json 1 0 0 12 12
    cargo run --bin map-studio-cli -- render-region Editor/projects/<id>/project.json 1 0 0 12 12 preview.png

The `asset-json` command returns an actual **row-major matrix**:
`matrix[y][x]` is `null` for a transparent/unowned position or a cell
with `metatile_id`, `collision`, and `elevation`. The asset's original
primary/secondary tileset symbols are included separately; the selected
map's resolved `native_pair` is shown by `inspect-region`.

The origin in inspect/render commands is **map-local**, not the merged editor
world coordinate. Inspect returns each cell's exact numeric ID, collision,
elevation, verified named source label (if one exists), true source behavior,
and tracked prefab UUID (if any). Empty cells remain null. Rendering uses the
matching native atlas, with no invented tiles or borrowed source palettes.

## Transaction JSON

First get the exact revision from the revision command. The agent must use a
**new canonical lowercase UUID** for each proposed transaction; reuse it when
previewing and applying the *same* request. Example:

    {
      "protocol_version": 1,
      "transaction_id": "08b05dd9-c1fe-4fe3-a60d-eb884422fc5a",
      "base_revision": "sha256:REPLACE_WITH_REVISION_FROM_CLI",
      "operations": [
        {"op":"fill_rect","map_id":1,"x":0,"y":0,"width":20,"height":20,"metatile_id":1},
        {"op":"place_asset","map_id":1,
         "asset_id":"emerald:environment:pond-bank-route104","x":5,"y":5}
      ]
    }

This example requires map 1 to already use the original Emerald
general/rustboro tileset and to be large enough. It is **not** portable to
a different pair: discover source-compatible assets before constructing a
request. Do not replace the "base_revision" with a Git commit SHA.

    cargo run --bin map-studio-cli -- preview-transaction project.json plan.json
    cargo run --bin map-studio-cli -- preview-transaction project.json plan.json 1 0 0 20 20 after-preview.png
    cargo run --bin map-studio-cli -- apply-transaction project.json plan.json
    cargo run --bin map-studio-cli -- undo-transaction project.json

An AI can also paint a complete rectangular region as one sparse matrix,
without constructing hundreds of separate `paint_tile` operations:

```json
{
  "op": "paint_matrix",
  "map_id": 1,
  "native_pair": "general/lilycove",
  "x": 8,
  "y": 10,
  "matrix": [
    [{"metatile_id": 1, "collision": 0, "elevation": 3}, null],
    [{"metatile_id": 1, "collision": 0, "elevation": 3},
     {"metatile_id": 1, "collision": 0, "elevation": 3}]
  ]
}
```

This operation belongs inside the transaction's `operations` array.
Each matrix must be rectangular, up to 64 × 64 cells, and fit within
its target map. `null` **preserves** the underlying map cell (including its
native properties); it does not erase or invent grass. IDs and properties
are checked against the exact native pair before any cells are painted.
Use `preview-transaction ... after-preview.png` to see the result before
applying it.

**Preview does not save project.json.** The optional PNG is rendered from the
proposed *draft* World, not the prior committed map. A preview report includes
cell before/after samples, total changed-cell count, object identity changes,
new map IDs, game export warnings, and the deterministic resulting revision.
The sample is limited to 512 cells (with a truncation flag); use inspect-region
for more. UI and CLI operations invoke the same underlying World methods.

**Commit is all-or-nothing.** The engine clones the original World, applies
every operation, validates invariants and native asset pairs, checks
base_revision again from current disk state under a local CLI lock, then saves
atomically. Invalid tile, out-of-bounds position, occupied event tile,
incompatible prefab, failed path transition or stale revision leaves the
project unchanged. No partial matrix stamps.

**Undo:** Each CLI commit writes one hidden local receipt under
.mapstudio/<project-filename>.last-transaction.json, holding the exact
previous World and revision. The next commit replaces this single receipt.
Undo is refused when the currently loaded project no longer matches the
receipt's resulting revision. The editor's existing touchscreen Undo remains
separate. Never delete a live .mapstudio transaction lock; inspect/recover
a stale lock after a crashed process with human supervision.

## Supported operations (protocol version 1)

| op | Required fields |
|---|---|
| create_map | name, space_id, x, y, width, height, native_pair |
| fill_rect | map_id, x, y, width, height, metatile_id |
| paint_tile | map_id, x, y, metatile_id; optional collision, elevation |
| paint_matrix | map_id, native_pair, x, y, matrix: rectangular rows of null or native cell objects |
| erase_tile | map_id, x, y |
| set_collision | map_id, x, y, collision (0–3) |
| set_elevation | map_id, x, y, elevation (0–15) |
| place_asset | map_id, asset_id, x, y |
| move_object | map_id, instance_id, x, y |
| remove_object | map_id, instance_id |
| draw_walkway | map_id, x1, y1, x2, y2 |
| place_event | map_id, x, y, kind: npc/sign, dialogue |

The current source-verified smart walkway is **General/Lilycove only**.
No fabricated shoreline autotiling is promised. Event placement currently
supports simple NPC and sign dialogue; complete scripted warp editing and
real ROM/emulator proof remain separate work. For large edits, split into
transactions of at most 128 operations and 20,000 changed cells, and inspect
again after each commit.

## Safety, concurrency and scope

- The caller must use the exact project's native tileset pair. Prefab
  source identities are stable: 71 Porymap indexes plus additional v0.21
  environment recipes without reordering older indexes.
- In a transaction, placed prefab instance IDs are derived deterministically
  from transaction UUID + operation index. Preview and commit then identify
  the same objects. Reuse an ID only for the identical request.
- A stale World revision is always rejected, preventing ordinary sequential
  lost updates. The CLI lock handles concurrent headless CLI commits on a
  local file. **It does not coordinate with an Android process editing a
  separate copy or with GitHub Sync.** Network transport and cross-device
  operation ordering belong to the separate realtime-sync milestone.
- Project JSON schema stays v1. Older Android previews cannot necessarily
  understand new v0.21 asset indexes; back up projects before cross-version
  imports. Do not push source code upstream to rh-hideout/pokeemerald-expansion.
- GUI touch handlers still call World directly rather than exchanging JSON
  transaction messages. They already share native painting/asset methods,
  but a fully unified operation-history journal requires the later sync stage.

## Acceptance tests

- Preview twice: identical resulting revision and native PNG, no disk changes.
- Commit after preview: resulting revision matches, matrix and object UUID
  match, project still passes integrity validation.
- Reject stale base revision without overwriting the current world.
- Reject second invalid operation without applying the first.
- Render source pair metatiles without substituting a visually similar pair.
- Apply and undo a CLI change only when it is still the latest revision.
- Test basic map creation, rectangles, asset move/remove, event placement
  and Lilycove walkway independently.
- Run Rust unit/CLI tests, desktop compilation, Android ARM64 CI packaging,
  then a real phone + compiled ROM verification later.

The actual Android screenshot/visual QA, direct AI-on-phone automation,
multi-client collaboration, and measured latency are not claimed complete.
