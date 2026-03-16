#!/usr/bin/env python3
"""
Terrain Atlas Generator for SlideCity (3D Bevy engine)
======================================================
Generates a 2048x2048 terrain texture atlas with 128x128 tiles (16x16 grid).

Row 0: Base terrain types (8 tiles)
Row 1-4: Transition tiles (blends between adjacent terrain types)
Row 5-6: Zone overlay tiles
Row 7+: Reserved

Usage:
    python scripts/build_terrain_atlas.py

Output:
    assets/textures/terrain_atlas.png
"""

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).parent.parent
OUTPUT = ROOT / "assets" / "textures" / "terrain_atlas.png"

TILE_SIZE = 128
ATLAS_TILES = 16  # 16x16 grid
ATLAS_SIZE = TILE_SIZE * ATLAS_TILES  # 2048

# Base terrain colors (RGB tuples)
TERRAIN_COLORS = {
    "grass":        (76, 140, 51),
    "grass_flower": (89, 148, 64),
    "trees":        (38, 107, 31),
    "trees_sparse": (56, 122, 41),
    "sand":         (194, 179, 128),
    "dirt":         (115, 97, 64),
    "rock":         (128, 122, 115),
    "snow":         (217, 224, 235),
}

# Zone overlay colors
ZONE_COLORS = {
    "residential":  (51, 153, 51),
    "commercial":   (51, 77, 204),
    "industrial":   (179, 153, 51),
    "park":         (26, 179, 77),
    "power_plant":  (153, 51, 51),
    "water_tower":  (51, 128, 179),
    "monument":     (179, 128, 51),
    "road":         (77, 77, 77),
}

# Terrain type order (matches Rust enum order and priority)
TERRAIN_ORDER = [
    "grass", "grass_flower", "trees", "trees_sparse",
    "sand", "dirt", "rock", "snow",
]

ZONE_ORDER = [
    "residential", "commercial", "industrial", "park",
    "power_plant", "water_tower", "monument", "road",
]


def noise_value(x: int, y: int, seed: int = 0) -> float:
    """Simple hash-based noise, returns 0.0-1.0."""
    n = x * 374761393 + y * 668265263 + seed * 1274126177
    n = (n ^ (n >> 13)) * 1103515245
    n = n ^ (n >> 16)
    return (n & 0x7FFFFFFF) / 0x7FFFFFFF


def generate_textured_tile(base_color: tuple, style: str = "terrain") -> Image.Image:
    """Generate a 128x128 textured tile with organic noise patterns."""
    img = Image.new("RGB", (TILE_SIZE, TILE_SIZE))
    draw = ImageDraw.Draw(img)
    r0, g0, b0 = base_color
    seed = hash(base_color) & 0xFFFFFF

    for y in range(TILE_SIZE):
        for x in range(TILE_SIZE):
            # Multi-octave noise for natural look
            n1 = noise_value(x, y, seed) * 0.5
            n2 = noise_value(x * 2, y * 2, seed + 1) * 0.25
            n3 = noise_value(x * 4, y * 4, seed + 2) * 0.125
            noise = (n1 + n2 + n3) - 0.4  # Center around 0

            # Scale variation by terrain type
            if style == "grass":
                variation = noise * 40
                # Add blade-like vertical streaks
                streak = noise_value(x, y // 3, seed + 100) * 15 - 7
                variation += streak
            elif style == "sand":
                variation = noise * 25
                # Ripple pattern
                ripple = math.sin(x * 0.15 + noise_value(x, y, seed + 50) * 3) * 8
                variation += ripple
            elif style == "rock":
                variation = noise * 50
                # Crack-like features
                crack = abs(noise_value(x * 3, y * 3, seed + 200) - 0.5) * 40
                variation -= crack
            elif style == "snow":
                variation = noise * 15  # Subtle variation
                # Sparkle highlights
                if noise_value(x * 7, y * 7, seed + 300) > 0.92:
                    variation += 20
            elif style == "dirt":
                variation = noise * 35
                # Clumpy pattern
                clump = noise_value(x // 4, y // 4, seed + 400) * 20 - 10
                variation += clump
            elif style == "trees":
                variation = noise * 45
                # Dappled canopy light
                dapple = noise_value(x // 3, y // 3, seed + 500)
                if dapple > 0.6:
                    variation += 20
                elif dapple < 0.3:
                    variation -= 15
            elif style == "zone":
                variation = noise * 20
                # Grid-like pattern for developed zones
                if x % 16 < 1 or y % 16 < 1:
                    variation -= 15
            else:
                variation = noise * 30

            r = max(0, min(255, int(r0 + variation)))
            g = max(0, min(255, int(g0 + variation)))
            b = max(0, min(255, int(b0 + variation)))
            draw.point((x, y), fill=(r, g, b))

    # Slight blur for smoothness
    img = img.filter(ImageFilter.GaussianBlur(radius=0.5))

    # Make seamless: blend edges
    img = make_seamless(img)

    return img


def make_seamless(img: Image.Image, margin: int = 16) -> Image.Image:
    """Blend tile edges to make it seamlessly tileable."""
    w, h = img.size
    result = img.copy()
    pixels_src = img.load()
    pixels_dst = result.load()

    for y in range(h):
        for x in range(margin):
            t = x / margin  # 0 at edge, 1 at margin
            # Blend left edge with right side
            rx = w - margin + x
            r1, g1, b1 = pixels_src[x, y]
            r2, g2, b2 = pixels_src[rx, y]
            blended = (
                int(r1 * t + r2 * (1 - t)),
                int(g1 * t + g2 * (1 - t)),
                int(b1 * t + b2 * (1 - t)),
            )
            pixels_dst[x, y] = blended
            pixels_dst[rx, y] = (
                int(r2 * t + r1 * (1 - t)),
                int(g2 * t + g1 * (1 - t)),
                int(b2 * t + b1 * (1 - t)),
            )

    for x in range(w):
        for y in range(margin):
            t = y / margin
            ry = h - margin + y
            r1, g1, b1 = pixels_dst[x, y]
            r2, g2, b2 = pixels_dst[x, ry]
            blended = (
                int(r1 * t + r2 * (1 - t)),
                int(g1 * t + g2 * (1 - t)),
                int(b1 * t + b2 * (1 - t)),
            )
            pixels_dst[x, y] = blended
            pixels_dst[x, ry] = (
                int(r2 * t + r1 * (1 - t)),
                int(g2 * t + g1 * (1 - t)),
                int(b2 * t + b1 * (1 - t)),
            )

    return result


def generate_transition_tile(
    color_a: tuple, color_b: tuple, direction: str, style_a: str, style_b: str
) -> Image.Image:
    """Generate a transition tile blending two terrain types.

    direction: 'n', 'e', 's', 'w' — which edge has the OTHER terrain type.
    The tile is mostly type A, blending to type B at the specified edge.
    """
    tile_a = generate_textured_tile(color_a, style_a)
    tile_b = generate_textured_tile(color_b, style_b)

    result = Image.new("RGB", (TILE_SIZE, TILE_SIZE))
    px_a = tile_a.load()
    px_b = tile_b.load()
    px_r = result.load()

    blend_depth = TILE_SIZE // 3  # How deep the blend goes

    for y in range(TILE_SIZE):
        for x in range(TILE_SIZE):
            # Compute blend factor based on direction
            if direction == "n":
                dist = y  # distance from north edge
            elif direction == "s":
                dist = TILE_SIZE - 1 - y
            elif direction == "w":
                dist = x
            elif direction == "e":
                dist = TILE_SIZE - 1 - x
            else:
                dist = TILE_SIZE

            if dist >= blend_depth:
                t = 1.0  # fully type A
            else:
                t = dist / blend_depth
                # Add noise to the blend boundary
                noise = noise_value(x, y, hash(direction) & 0xFFFF) * 0.3 - 0.15
                t = max(0.0, min(1.0, t + noise))

            ra, ga, ba = px_a[x, y]
            rb, gb, bb = px_b[x, y]
            px_r[x, y] = (
                int(ra * t + rb * (1 - t)),
                int(ga * t + gb * (1 - t)),
                int(ba * t + bb * (1 - t)),
            )

    return result


def main():
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)

    atlas = Image.new("RGB", (ATLAS_SIZE, ATLAS_SIZE), (0, 0, 0))

    # === Row 0: Base terrain tiles ===
    print("Generating base terrain tiles...")
    for i, name in enumerate(TERRAIN_ORDER):
        style = name.split("_")[0]  # "grass_flower" -> "grass" style
        if name == "trees_sparse":
            style = "trees"
        tile = generate_textured_tile(TERRAIN_COLORS[name], style)
        atlas.paste(tile, (i * TILE_SIZE, 0))
        print(f"  [{i}] {name}")

    # === Rows 1-4: Transition tiles ===
    # For each pair of adjacent terrain types, generate N/E/S/W transitions
    print("Generating transition tiles...")
    transition_col = 0
    transition_row = 1

    # Key transitions (most visible boundaries)
    transitions = [
        ("grass", "sand"), ("grass", "dirt"), ("grass", "rock"),
        ("sand", "dirt"), ("sand", "rock"), ("dirt", "rock"),
        ("rock", "snow"), ("snow", "grass"), ("grass", "trees"),
        ("trees", "dirt"), ("sand", "snow"), ("dirt", "snow"),
    ]

    for type_a, type_b in transitions:
        style_a = type_a.split("_")[0]
        style_b = type_b.split("_")[0]
        for direction in ["n", "e", "s", "w"]:
            tile = generate_transition_tile(
                TERRAIN_COLORS[type_a], TERRAIN_COLORS[type_b],
                direction, style_a, style_b,
            )
            x = transition_col * TILE_SIZE
            y = transition_row * TILE_SIZE
            atlas.paste(tile, (x, y))

            transition_col += 1
            if transition_col >= ATLAS_TILES:
                transition_col = 0
                transition_row += 1

    print(f"  Generated {len(transitions) * 4} transition tiles (rows 1-{transition_row})")

    # === Row 5-6: Zone overlay tiles ===
    zone_row = 5
    print("Generating zone overlay tiles...")
    for i, name in enumerate(ZONE_ORDER):
        tile = generate_textured_tile(ZONE_COLORS[name], "zone")
        atlas.paste(tile, (i * TILE_SIZE, zone_row * TILE_SIZE))
        print(f"  [{i}] {name}")

    # Save
    atlas.save(str(OUTPUT), "PNG", optimize=True)
    file_size = OUTPUT.stat().st_size
    print(f"\nAtlas saved: {OUTPUT}")
    print(f"Size: {file_size // 1024} KB ({ATLAS_SIZE}x{ATLAS_SIZE}, {ATLAS_TILES}x{ATLAS_TILES} tiles)")


if __name__ == "__main__":
    main()
