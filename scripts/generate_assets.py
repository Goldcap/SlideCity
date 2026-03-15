#!/usr/bin/env python3
"""
SlideCity Asset Generator
=========================
Generates isometric pixel art sprites via Replicate's retro-diffusion model.
Requires: pip install replicate requests

Usage:
    export REPLICATE_API_TOKEN=your_token
    python scripts/generate_assets.py [--dry-run]

Sprites are saved to assets/sprites/ in the correct subdirectories.
The game falls back to colored rectangles if sprites are missing.
"""

import os
import sys
import json
import time
import argparse
from pathlib import Path

try:
    import replicate
except ImportError:
    print("ERROR: 'replicate' package not installed. Run: pip install replicate")
    sys.exit(1)

try:
    import requests
except ImportError:
    print("ERROR: 'requests' package not installed. Run: pip install requests")
    sys.exit(1)

# Project root
ROOT = Path(__file__).parent.parent
SPRITES_DIR = ROOT / "assets" / "sprites"

# Sprite definitions: (filename, subdirectory, prompt, width, height)
SPRITES = [
    # === TERRAIN (64x32 isometric ground tiles) ===
    ("grass.png", "terrain",
     "isometric pixel art grass tile, green field, game asset, transparent background, 64x32",
     64, 32),
    ("grass_flower.png", "terrain",
     "isometric pixel art grass tile with wildflowers, colorful flowers on green, game asset, transparent background, 64x32",
     64, 32),
    ("trees_dense.png", "terrain",
     "isometric pixel art dense forest tile, lush green trees, game asset, transparent background, 64x32",
     64, 32),
    ("trees_sparse.png", "terrain",
     "isometric pixel art scattered trees tile, few trees on grass, game asset, transparent background, 64x32",
     64, 32),
    ("sand.png", "terrain",
     "isometric pixel art sandy beach tile, golden sand, game asset, transparent background, 64x32",
     64, 32),
    ("dirt.png", "terrain",
     "isometric pixel art dirt tile, brown earth, game asset, transparent background, 64x32",
     64, 32),
    ("rock.png", "terrain",
     "isometric pixel art rocky terrain tile, grey rocks and boulders, game asset, transparent background, 64x32",
     64, 32),
    ("snow.png", "terrain",
     "isometric pixel art snow covered tile, white snow, game asset, transparent background, 64x32",
     64, 32),
    ("water.png", "terrain",
     "isometric pixel art water tile, blue water with gentle ripples, game asset, transparent background, 64x32",
     64, 32),

    # === ROADS (64x32) ===
    ("road_straight.png", "roads",
     "isometric pixel art road tile, grey asphalt straight road, SimCity style, game asset, transparent background, 64x32",
     64, 32),
    ("road_cross.png", "roads",
     "isometric pixel art road intersection, grey asphalt crossroads, SimCity style, game asset, transparent background, 64x32",
     64, 32),

    # === RESIDENTIAL — 3 stages × 4 variants (64x32 to 64x96) ===
    ("res_s1_v1.png", "residential",
     "isometric pixel art small house, single floor cottage, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("res_s1_v2.png", "residential",
     "isometric pixel art small house variant, tiny home with garden, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("res_s1_v3.png", "residential",
     "isometric pixel art small ranch house, suburban home, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("res_s1_v4.png", "residential",
     "isometric pixel art small bungalow, cozy house, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("res_s2_v1.png", "residential",
     "isometric pixel art two-story house, medium residential building, SimCity 2000 style, game asset, transparent background, 64x64",
     64, 64),
    ("res_s2_v2.png", "residential",
     "isometric pixel art duplex building, two floor home, SimCity 2000 style, game asset, transparent background, 64x64",
     64, 64),
    ("res_s2_v3.png", "residential",
     "isometric pixel art townhouse, medium residential, SimCity 2000 style, game asset, transparent background, 64x64",
     64, 64),
    ("res_s2_v4.png", "residential",
     "isometric pixel art row houses, two story residential, SimCity 2000 style, game asset, transparent background, 64x64",
     64, 64),
    ("res_s3_v1.png", "residential",
     "isometric pixel art apartment building, four story residential tower, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("res_s3_v2.png", "residential",
     "isometric pixel art condo building, tall residential, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("res_s3_v3.png", "residential",
     "isometric pixel art apartment complex, high-rise residential, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("res_s3_v4.png", "residential",
     "isometric pixel art luxury apartments, tall residential tower, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),

    # === COMMERCIAL — 3 stages × 4 variants ===
    ("com_s1_v1.png", "commercial",
     "isometric pixel art small shop, single floor store, SimCity 2000 style, blue roof, game asset, transparent background, 64x48",
     64, 48),
    ("com_s1_v2.png", "commercial",
     "isometric pixel art corner store, small retail, SimCity 2000 style, blue accents, game asset, transparent background, 64x48",
     64, 48),
    ("com_s1_v3.png", "commercial",
     "isometric pixel art cafe, small commercial building, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("com_s1_v4.png", "commercial",
     "isometric pixel art small office, one floor commercial, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("com_s2_v1.png", "commercial",
     "isometric pixel art office building, four floor commercial tower, SimCity 2000 style, blue glass, game asset, transparent background, 64x96",
     64, 96),
    ("com_s2_v2.png", "commercial",
     "isometric pixel art mall, medium commercial building, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("com_s2_v3.png", "commercial",
     "isometric pixel art hotel, medium commercial, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("com_s2_v4.png", "commercial",
     "isometric pixel art office complex, commercial tower, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("com_s3_v1.png", "commercial",
     "isometric pixel art skyscraper, ten floor office tower, SimCity 2000 style, glass and steel, game asset, transparent background, 64x128",
     64, 128),
    ("com_s3_v2.png", "commercial",
     "isometric pixel art tall office tower, corporate skyscraper, SimCity 2000 style, game asset, transparent background, 64x128",
     64, 128),
    ("com_s3_v3.png", "commercial",
     "isometric pixel art modern skyscraper, tall commercial, SimCity 2000 style, game asset, transparent background, 64x128",
     64, 128),
    ("com_s3_v4.png", "commercial",
     "isometric pixel art downtown tower, glass skyscraper, SimCity 2000 style, game asset, transparent background, 64x128",
     64, 128),

    # === INDUSTRIAL — 3 stages × 3 variants ===
    ("ind_s1_v1.png", "industrial",
     "isometric pixel art small factory, warehouse, SimCity 2000 style, yellow-brown, game asset, transparent background, 64x64",
     64, 64),
    ("ind_s1_v2.png", "industrial",
     "isometric pixel art workshop, small industrial building, SimCity 2000 style, game asset, transparent background, 64x64",
     64, 64),
    ("ind_s1_v3.png", "industrial",
     "isometric pixel art storage facility, small warehouse, SimCity 2000 style, game asset, transparent background, 64x64",
     64, 64),
    ("ind_s2_v1.png", "industrial",
     "isometric pixel art medium factory, industrial plant with smokestack, SimCity 2000 style, game asset, transparent background, 64x80",
     64, 80),
    ("ind_s2_v2.png", "industrial",
     "isometric pixel art manufacturing plant, medium industrial, SimCity 2000 style, game asset, transparent background, 64x80",
     64, 80),
    ("ind_s2_v3.png", "industrial",
     "isometric pixel art refinery, medium industrial complex, SimCity 2000 style, game asset, transparent background, 64x80",
     64, 80),
    ("ind_s3_v1.png", "industrial",
     "isometric pixel art large factory complex, heavy industry with multiple smokestacks, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("ind_s3_v2.png", "industrial",
     "isometric pixel art steel mill, large industrial, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),
    ("ind_s3_v3.png", "industrial",
     "isometric pixel art power factory, large industrial plant, SimCity 2000 style, game asset, transparent background, 64x96",
     64, 96),

    # === INFRASTRUCTURE (64x80) ===
    ("power_plant.png", "infrastructure",
     "isometric pixel art power plant, coal power station with cooling tower, SimCity 2000 style, orange, game asset, transparent background, 64x80",
     64, 80),
    ("power_line.png", "infrastructure",
     "isometric pixel art power line, electrical pylon, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),
    ("water_tower.png", "infrastructure",
     "isometric pixel art water tower, blue water tank on stilts, SimCity 2000 style, game asset, transparent background, 64x80",
     64, 80),
    ("water_main.png", "infrastructure",
     "isometric pixel art water pipe, underground pipe marker, SimCity 2000 style, blue, game asset, transparent background, 64x32",
     64, 32),
    ("monument.png", "infrastructure",
     "isometric pixel art monument, grand civic monument with statue, SimCity 2000 style, purple and gold, game asset, transparent background, 64x128",
     64, 128),
    ("park.png", "infrastructure",
     "isometric pixel art city park, green park with trees and bench, SimCity 2000 style, game asset, transparent background, 64x48",
     64, 48),

    # === EVENTS (64x64) ===
    ("fire.png", "events",
     "isometric pixel art fire, burning building flames, SimCity 2000 style, orange red flames, game asset, transparent background, 64x64",
     64, 64),
    ("rubble.png", "events",
     "isometric pixel art rubble, destroyed building debris, SimCity 2000 style, grey brown ruins, game asset, transparent background, 64x32",
     64, 32),
]


def generate_sprite(name, subdir, prompt, width, height, dry_run=False):
    """Generate a single sprite via Replicate's retro-diffusion model."""
    output_path = SPRITES_DIR / subdir / name

    if output_path.exists():
        print(f"  SKIP {subdir}/{name} (already exists)")
        return True

    print(f"  GEN  {subdir}/{name} ({width}x{height})")

    if dry_run:
        print(f"       Prompt: {prompt[:80]}...")
        return True

    try:
        output = replicate.run(
            "retro-diffusion/rd-plus",
            input={
                "prompt": prompt,
                "style": "isometric_asset",
                "remove_bg": True,
                "width": width,
                "height": height,
                "num_outputs": 1,
            },
        )

        # Output is a list of URLs
        if isinstance(output, list) and len(output) > 0:
            url = str(output[0])
        elif hasattr(output, '__iter__'):
            url = str(next(iter(output)))
        else:
            url = str(output)

        # Download the image
        response = requests.get(url, timeout=30)
        response.raise_for_status()

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "wb") as f:
            f.write(response.content)

        print(f"       Saved: {output_path}")
        return True

    except replicate.exceptions.ReplicateError as e:
        print(f"       ERROR (Replicate): {e}")
        return False
    except requests.exceptions.RequestException as e:
        print(f"       ERROR (Download): {e}")
        return False
    except Exception as e:
        print(f"       ERROR: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(description="Generate SlideCity sprite assets via Replicate")
    parser.add_argument("--dry-run", action="store_true",
                        help="Print what would be generated without making API calls")
    parser.add_argument("--subset", type=str, default=None,
                        help="Only generate sprites in this subdirectory (e.g., 'terrain', 'residential')")
    args = parser.parse_args()

    if not args.dry_run:
        token = os.environ.get("REPLICATE_API_TOKEN")
        if not token:
            # Try .env file
            env_path = ROOT / ".env"
            if env_path.exists():
                for line in env_path.read_text().splitlines():
                    if line.startswith("REPLICATE_API_TOKEN="):
                        token = line.split("=", 1)[1].strip().strip('"').strip("'")
                        if token and not token.startswith("your_"):
                            os.environ["REPLICATE_API_TOKEN"] = token
                        else:
                            token = None

            if not token:
                print("ERROR: REPLICATE_API_TOKEN not set.")
                print("  Set it via: export REPLICATE_API_TOKEN=your_token")
                print("  Or add it to .env file")
                sys.exit(1)

    print(f"SlideCity Asset Generator")
    print(f"========================")
    print(f"Output: {SPRITES_DIR}")
    print(f"Sprites: {len(SPRITES)} total")
    if args.dry_run:
        print("Mode: DRY RUN (no API calls)")
    if args.subset:
        print(f"Subset: {args.subset}")
    print()

    success = 0
    failed = 0
    skipped = 0

    for name, subdir, prompt, w, h in SPRITES:
        if args.subset and subdir != args.subset:
            continue

        result = generate_sprite(name, subdir, prompt, w, h, dry_run=args.dry_run)
        if result:
            if (SPRITES_DIR / subdir / name).exists() and not args.dry_run:
                skipped += 1
            else:
                success += 1
        else:
            failed += 1

        # Rate limit: don't hammer the API
        if not args.dry_run and result:
            time.sleep(1)

    print()
    print(f"Done! Generated: {success}, Skipped: {skipped}, Failed: {failed}")

    if failed > 0:
        print()
        print("NOTE: If retro-diffusion produces inconsistent results, consider:")
        print("  1. Adjusting prompts for more specific style keywords")
        print("  2. Using a different model (e.g., stable-diffusion with ControlNet)")
        print("  3. Using an isometric asset pack from itch.io as a base")
        print("  4. The game works perfectly with colored rectangles (no sprites needed)")


if __name__ == "__main__":
    main()
