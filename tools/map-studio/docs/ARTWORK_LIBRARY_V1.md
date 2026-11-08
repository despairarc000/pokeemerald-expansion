# Artwork Library v0.17 — source audit and architecture

## Source-backed built-ins

| Collection | Source | Count | Bundled? | Editor capability |
| --- | --- | ---: | --- | --- |
| Emerald + FRLG native metatile pairs | This fork's `data/layouts/layouts.json` and `data/tilesets/{primary,secondary}/*`, built by `scripts/build_catalog.py` | 129 usable pairs in the previous CI, across Emerald and FireRed/LeafGreen; current build is authoritative | Yes | Separate built-in collections, original IDs, source attributes, layers and palettes |
| Emerald prefab definitions | [huderlem/Porymap](https://github.com/huderlem/porymap/blob/master/resources/text/prefabs_default_emerald.json) | 71 source definitions | Yes | Filter by actual source tileset symbols + valid metatile ID; paint full 2D structure with collision and elevation |
| FireRed Porymap prefabs | [Porymap default FireRed](https://github.com/huderlem/porymap/blob/master/resources/text/prefabs_default_firered.json) | 45 | Yes, on this source branch after the v0.17 APK | Source-verified against native FRLG pairs already present in this fork; incompatible pairs never receive the raw IDs |
| Ruby Porymap prefabs | [Porymap default Ruby](https://github.com/huderlem/porymap/blob/master/resources/text/prefabs_default_ruby.json) | 55 | No | Original Ruby tileset needs importing before prefabs can be used |

Porymap's prefab data carries `name`, `width`, `height`, primary and
secondary source symbols, and an array of `{x,y,metatile_id,collision,elevation}`.
A blank source symbol is a wildcard. All 71 default Emerald prefab definitions
match at least one of our fork's **layout-declared symbol pairings**, but
**not all 71 are available on every map**. The editor applies an additional
per-metatile validity check against the compiled pair. Original metatile IDs
are never renumbered. Porymap license/attribution is included in
`../assets/PORYMAP-ATTRIBUTION.md` and `../assets/PORYMAP-LICENSE.md`.

## Further Pokémon artwork sources discovered

| Source | What is available | Safe integration status |
| --- | --- | --- |
| [Team Aqua's Asset Repo / Full Tilesets](https://github.com/TeamAquasHideout/Team-Aquas-Asset-Repo/tree/main/Tilesets/The%20Great%20Tileset%20Exchange/Full%20Tilesets) | Multiple full, titled primary and secondary tilesets, such as Gen 4 Cave Secondary, Brick City, Gatehouses, underwater sets, Autumn Ruins, Sewers and LeoB ORAS | Strong import candidate. Their README describes default free use/edit with creator-specific exceptions. Check each directory's README and retain credits before redistribution. |
| [LeoB ORAS Pokémon Emerald](https://github.com/lbsbezerra/pokeemerald-modern-oras) and [public-use announcement](https://www.reddit.com/r/PokemonROMhacks/comments/1de5m2z) | ORAS-inspired overworld, paths, trees, water and building artwork in an Emerald project | Explicit author public-use statement with attribution; requires compatible source resources and deliberate conversion |
| [Liamjd14 FireRed Tileset Port](https://github.com/Liamjd14/Firered-Tileset-Port) | WIP native FireRed-to-Emerald source port including tilesets | Technical reference, not presumed redistribution permission for the art; verify assets and project licensing first |
| [Pokémon Heart and Soul prefabs](https://github.com/PokemonHnS-Development/pokemonHnS/blob/main/prefabs.json) | Named Johto prefabs with custom `gTileset_Johto_*` symbols | Compatible *data structure*, incompatible native IDs without matching Johto tile graphics |
| [Porytiles](https://github.com/grunt-lucas/porytiles) | Generation III metatile/tileset compiler | Good next integration target for user-created source PNG/attribute conversion |
| [Hypercutter](https://github.com/cuebitt/hypercutter) | Extraction/library utility for Emerald/FireRed/LeafGreen/Ruby/Sapphire tile/palette resources | Tooling reference; extracted ROM artwork is not automatically authorized for redistribution |
| Pokémon Unbound | ROM patch and contributor art credits; no verified complete openly redistributable tileset archive | Not bundled and not shown as an empty collection |

The optional external artwork packages (Team Aqua, LeoB ORAS, Unbound, etc.) are **not** bundled by this patch. Only real source-backed Emerald and FRLG graphics already in the game fork are exposed; Porymap's 45 FireRed prefab definitions are newly added.
Public GitHub access is not universal relicensing of original Pokémon artwork.
An imported pack must contain explicit provenance/attribution; each source
still requires separate permission verification where applicable.

## App UI hierarchy

1. **Collection**: Emerald, FireRed/LeafGreen (native assets from this game fork), or a real imported collection. No fake entries for Ruby or Unbound.
2. **Category**: swipe-scrollable categories (Towns, Houses, Interiors,
   Caves, Outdoor & Special), chosen from actual tileset metadata.
3. **Tileset**: source pair in that category, with previous/next controls;
   original IDs and source order remain unchanged.
4. **Metatiles / Prefabs**: source tile sheet or visual prefab cards. Emerald
   (71 definitions) and FireRed/LeafGreen (45 definitions) show compatible
   source-prefab cards for the active pair, and also let users browse the
   other definitions and jump to their corresponding source tileset.
   Native painting stamps the correct ID, collision and elevation.
   Imported reference packs remain unpaintable.

Touch swipes scroll categories **only inside that strip** and art scrolls
vertically in the sheet. Swipes do not silently switch the collection or
tileset while painting. Full-window height and system navigation reserve
remain bounded by the existing responsive dock code.

## Portable artwork ZIP format (v1)

The current export creates a **source-complete ZIP for the selected pair**,
not a huge archive containing all 129 pairs. Paths inside the ZIP:

- `manifest.json`: `format = "mapstudio-artwork-pack"`, version 1;
  collection, engine, source, attribution, pair key, source symbols,
  category, native counts and 128-byte validity mask.
- `atlas.png`: 256×1024 sheet of 1024 positions (16 columns ×64 rows),
  containing the locally compiled 16×16 source metatiles.
- `primary/tiles.png`, `secondary/tiles.png`: original indexed 8×8 art.
- `primary/metatiles.bin`, `secondary/metatiles.bin`: true 8-subtile
  native metatile definitions.
- `primary/metatile_attributes.bin`,
  `secondary/metatile_attributes.bin`: native attribute data.
- `primary/palettes/*.pal`, `secondary/palettes/*.pal`: native palette banks.
- `prefabs.json`: compatible Porymap prefabs from the selected native source collection
  with source IDs and collision/elevation.

An imported ZIP must parse, pass bounded input checks, contain all required
source files, contain a valid-sized atlas and consistent counts, and have
a supported manifest version. It is stored globally in the app-private
artwork library; maps are not changed by import.

**Native compatibility** is deliberately strict: the imported atlas,
source symbols, valid mask and pair key must match the built-in Emerald
pair before the user can explicitly choose "Use matching Emerald source".
Otherwise the imported collection is **browse-only**. The source-complete
package gives the next compiler stage all core resources, but full
import-and-compile of foreign palettes, behavioral constants, layer modes,
tile count budgets and map bank allocation remains future work.

On Android, the current import UI browses ZIP archives readable in this
app's `Downloads/MapStudio` MediaStore folder. A general system document
picker is not yet present. Export publishes to that same Downloads folder.
CLI/desktop programmatic imports can use the format directly.

## Tested behavior & known validation gaps

Unit tests cover source prefab count, compatibility checks, paint bounds,
collision/elevation preservation, portable package export/reimport, malformed
ZIP rejection, CLI/editor regression tests, and original map export.
The Android workflow includes an ARM64 signature check plus desktop build.

**Not established**: physical Samsung swipe/resize usability, full ROM
engine round-trip for any *new* external tileset, preservation of animation
tiles beyond copied native source files, or legal entitlement to distribute
any third-party source without license review. Before making imported
art game-paintable, build and test its native tileset in this fork.

## Follow-up phases

- P0: direct Android Storage Access Framework picker and safe ZIP browsing.
- P1: convert a complete external pack (ORAS or FRLG) against explicit
  Emerald palette/ID/layer limits with a playable ROM integration test.
- P1: user-curated local prefab creation + ZIP export, names and revisions.
- P2: cross-project collection version references, deduplicated asset
  storage, import conflict resolution and dependency backups.
- P2: larger collection gallery, install/uninstall, and package previews.

## Source follow-up after the v0.17 preview

The saved `MapStudio-v0.17-Art-Library-Preview.apk` predates this source
revision. Its Collection menu has only Emerald and installed ZIPs; this
revision exposes the already-compiled FRLG assets as **FireRed/LeafGreen**
and bundles Porymap's 45 FireRed prefab definitions. The previous empty-looking
prefab gallery occurred because it displayed only the active pair's matching
prefabs; now it also provides a browse-and-switch list for valid prefabs of
other pairs. The new APK is not considered verified until CI compiles and
its UI is checked on a phone.

The next external candidates remain Team Aqua's credited Great Tileset
Exchange, LeoB's ORAS-inspired art, Ruby original assets, and Johto packs.
Those require provenance checks and complete palette/metatile conversion.
Unbound has no verified redistributable full source collection and must not
appear as a fake built-in.

## Subsequent offline built-in source collections

The app now compiles these genuine source assets **into the APK** from files
vendored inside `tools/map-studio/assets/builtin-sources`:

| Collection | Built-in scope | Prefabs | Can paint native maps? |
| --- | --- | --- | --- |
| Emerald | All source-resolvable Emerald layout pairs | Porymap 71 definitions; per-pair validity checked | Yes |
| FireRed/LeafGreen | All source-resolvable FRLG layout pairs in this expansion fork | Porymap FireRed 45 definitions; per-pair validity checked | Yes |
| Ruby | Original `pret/pokeruby` general primary + Petalburg, Mauville, Rustboro secondaries, original palettes and attributes | Porymap Ruby 55 definitions available in source; only compatible ones are embedded in each of three source packages | **Browse/export only** pending native source integration |
| ORAS Style | LeoB/Team Aqua General primary + Petalburg, Mauville, Rustboro secondaries, original indexed assets and palettes | No authored cross-project prefabs bundled; do not label a random rectangle as one | **Browse/export only** pending native source integration |

Imported art remains supported. Every built-in reference package contains real
`atlas.png`, original `tiles.png`, original metatile binaries, native
attributes, palettes, source credit and any verified prefab definitions.
No network, Downloads import or installation is required to view them.

The remaining Team Aqua Exchange artwork contains independently authored packs
that need asset-by-asset review. Many rely on triple-layer or expanded metatile
limits and cannot safely be loaded into the current two-layer Emerald game
compiler. Pokémon Unbound is not included without source and redistribution
permission. Do not add placeholder collection names.

The editor must not reinterpret an Emerald map's native metatile IDs as Ruby
or ORAS IDs. Reference collections intentionally reject painting until a
corresponding game-source compiler, export and emulator smoke test exists.
