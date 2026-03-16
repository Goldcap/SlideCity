# Brainstorm: SC4-Level Visual Upgrade

**Date:** 2026-03-16
**Status:** Complete
**Next step:** `/ce:plan`

## What We're Building

A major visual overhaul of SlideCity to reach SimCity 4-level graphical quality, plus mouse camera controls and a larger window. The game currently has vertex-colored flat terrain, plain cube buildings, no vegetation, and no road geometry.

### Goals

1. **Terrain textures** — Replace flat vertex colors with a UV-mapped texture atlas (grass, dirt, rock, sand, snow)
2. **GLTF building models** — Replace cube primitives with proper 3D models sourced from Kenney + OpenGameArt/itch.io
3. **GLTF tree models** — Real 3D tree models instanced across forest terrain cells, with LOD for performance
4. **Full road network mesh** — Continuous road geometry with proper intersections, lane markings, and connected segments
5. **Mouse camera controls** — Add middle-click drag to pan (supplement existing WASD + right-drag rotate + scroll zoom)
6. **1920x1080 window** — Upgrade from 1280x720

## Why This Approach

The user wants SC4-quality visuals, not incremental improvements over colored boxes. This means:
- **Texture atlas over procedural shaders** — Gives the hand-crafted tile look SC4 is known for. More work to set up but more authentic.
- **GLTF models over procedural geometry** — Real 3D models give dramatically better results than code-generated shapes. Sourcing from free asset packs (Kenney CC0 + OpenGameArt) avoids modeling from scratch.
- **Full road mesh over simple strips** — SC4's road network was a defining visual feature. Proper intersection geometry and continuous mesh is worth the complexity.
- **GLTF trees with LOD over billboards/low-poly** — Matches the quality bar of the rest of the upgrade. LOD system keeps performance viable for large forests.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Terrain texturing | Texture atlas + UV mapping | Classic SC4 tile look, per-cell UV coordinates mapped to terrain type |
| Building visuals | GLTF/GLB 3D models | Source from Kenney + OpenGameArt/itch.io, curate for style consistency |
| Tree/vegetation | GLTF tree models + LOD | Quality matches building models, GPU instancing for density |
| Road geometry | Full road network mesh | Connected segments, proper intersections, textured asphalt + lane markings |
| Mouse controls | Middle-click pan + keep WASD/QE | Both input methods, maximum flexibility |
| Window size | 1920x1080 windowed | Full HD, taskbar visible |
| Model sourcing | Kenney CC0 + OpenGameArt/itch.io | Browse both, curate for visual coherence. No licensing concerns with Kenney. |
| Terrain transitions | Blend tiles from the start | Include grass-dirt, dirt-rock, etc. transition tiles in atlas. Neighbor-aware UV selection. |
| Building variety | 6+ per zone type (~25+ total) | Full SC4-level variety across Residential, Commercial, Industrial + utilities |
| Road intersections | All types including curves | Straight, T, 4-way, curves, dead-ends. Full road toolkit. |
| Asset storage | Separate download, not in git | Keep repo lean. Download assets at build time or first run. |

## Scope & Phasing

This is a large upgrade. Suggested implementation order:

1. **Window + mouse controls** — Quick wins, immediate UX improvement
2. **Terrain texture atlas** — Biggest visual bang, foundational for everything else
3. **GLTF building models** — Replace cubes with real models per zone type/density
4. **Tree models + instancing** — Populate forest cells with 3D trees
5. **Road network mesh** — Most complex geometry generation, saved for last

## Resolved Questions

1. **Terrain transitions** — Include blend/transition tiles from the start. Neighbor-aware UV selection for smooth boundaries.
2. **Building variety** — 6+ distinct models per zone type (~25+ total). Full SC4-level variety.
3. **Road intersections** — All types: straight, T, 4-way, curves, dead-ends. Full road toolkit.
4. **Asset storage** — Separate download system, not committed to git. Keep repo lean.

## Open Questions

1. **Tree LOD distances** — At what camera distances do we swap LOD levels? Need to test with actual models.
2. **Asset download mechanism** — Script in repo? First-run downloader? Build.rs? Need to decide the exact pipeline.
