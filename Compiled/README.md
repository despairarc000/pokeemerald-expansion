# Compiled — Generated Pokémon game map source

**Not APKs.** This is output-only for a named Map Studio project.

```text
Compiled/projects/<project-uuid>/
  source-manifest.json
  custom_maps/<project-map-symbol>/        # Inputs for the existing textmap builder
  data/layouts/<project-map-symbol>/       # Native map.bin and border.bin
  data/maps/<project-map-symbol>/          # map.json and scripts.inc
```

Only `workspace-publish` writes a project output. It reads and validates
`Editor/projects/<uuid>/project.json` with its manifest, preserves the source
project, generates project-namespaced game map symbols, and stages the full
output before atomically replacing that project's prior publication.

The currently implemented native exporter targets **Emerald Expansion**;
FireRed output is blocked. A valid generated layout is NOT proof of a
successfully compiled or emulated GBA game. The game's map-registration
pipeline (`tools/textmap/build_textmaps.py`) still needs to integrate source
entries into the selected target repository. `source-manifest.json` marks
`rom_build_verified: false` until real game-side verification is performed.

Do not modify `Compiled/` by hand, feed it back into `Editor/`, push it
upstream to pret or the original Pokémon Expansion project, or place Android
APKs here. Git history handles published revision recovery.
