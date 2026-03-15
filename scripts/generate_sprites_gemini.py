#!/usr/bin/env python3
"""Generate SlideCity isometric sprites using Gemini image generation."""

import os
import sys
from pathlib import Path

from google import genai
from google.genai import types
from PIL import Image

ROOT = Path(__file__).parent.parent
SPRITES_DIR = ROOT / "assets" / "sprites"

client = genai.Client(api_key=os.environ["GEMINI_API_KEY"])
MODEL = "gemini-3-pro-image-preview"

STYLE = (
    "isometric pixel art, SimCity 2000 style, 16-bit retro game asset, "
    "clean pixel edges, limited color palette, transparent background, "
    "crisp and detailed"
)

SPRITES = [
    # (filename, subdir, prompt, target_w, target_h)
    # === TERRAIN ===
    ("grass.png", "terrain", f"Single isometric grass tile, lush green field, {STYLE}", 64, 32),
    ("grass_flower.png", "terrain", f"Single isometric grass tile with scattered wildflowers, {STYLE}", 64, 32),
    ("trees_dense.png", "terrain", f"Single isometric tile with dense forest trees, dark green canopy, {STYLE}", 64, 48),
    ("trees_sparse.png", "terrain", f"Single isometric tile with a few scattered trees on grass, {STYLE}", 64, 48),
    ("sand.png", "terrain", f"Single isometric sandy beach tile, golden tan sand, {STYLE}", 64, 32),
    ("dirt.png", "terrain", f"Single isometric dirt tile, brown earth, {STYLE}", 64, 32),
    ("rock.png", "terrain", f"Single isometric rocky terrain tile, grey boulders, {STYLE}", 64, 32),
    ("snow.png", "terrain", f"Single isometric snow-covered tile, white pristine snow, {STYLE}", 64, 32),
    ("water.png", "terrain", f"Single isometric water tile, blue water with subtle ripples, {STYLE}", 64, 32),

    # === ROADS ===
    ("road_straight.png", "roads", f"Single isometric straight road tile, grey asphalt with lane markings, {STYLE}", 64, 32),
    ("road_cross.png", "roads", f"Single isometric road intersection crossroads tile, grey asphalt, {STYLE}", 64, 32),

    # === RESIDENTIAL ===
    ("res_s1_v1.png", "residential", f"Single isometric small cottage house, one floor, red roof, green lawn, {STYLE}", 64, 48),
    ("res_s1_v2.png", "residential", f"Single isometric small suburban home, one floor, blue roof, {STYLE}", 64, 48),
    ("res_s1_v3.png", "residential", f"Single isometric ranch house, one floor, brown roof, {STYLE}", 64, 48),
    ("res_s1_v4.png", "residential", f"Single isometric small bungalow, one floor, yellow walls, {STYLE}", 64, 48),
    ("res_s2_v1.png", "residential", f"Single isometric two-story house, residential, white walls red roof, {STYLE}", 64, 64),
    ("res_s2_v2.png", "residential", f"Single isometric duplex building, two floors, brick facade, {STYLE}", 64, 64),
    ("res_s2_v3.png", "residential", f"Single isometric townhouse, two floors, connected row house style, {STYLE}", 64, 64),
    ("res_s2_v4.png", "residential", f"Single isometric two-story home, modern style, flat roof, {STYLE}", 64, 64),
    ("res_s3_v1.png", "residential", f"Single isometric apartment building, four floors, beige concrete, many windows, {STYLE}", 64, 96),
    ("res_s3_v2.png", "residential", f"Single isometric condo tower, four floors, glass and steel modern, {STYLE}", 64, 96),
    ("res_s3_v3.png", "residential", f"Single isometric apartment complex, four floors, brick with balconies, {STYLE}", 64, 96),
    ("res_s3_v4.png", "residential", f"Single isometric luxury apartments, four floors, white modern, {STYLE}", 64, 96),

    # === COMMERCIAL ===
    ("com_s1_v1.png", "commercial", f"Single isometric small shop, one floor, blue awning, storefront, {STYLE}", 64, 48),
    ("com_s1_v2.png", "commercial", f"Single isometric corner store, one floor, neon sign, {STYLE}", 64, 48),
    ("com_s1_v3.png", "commercial", f"Single isometric small cafe, one floor, outdoor seating, {STYLE}", 64, 48),
    ("com_s1_v4.png", "commercial", f"Single isometric small office, one floor, glass front, {STYLE}", 64, 48),
    ("com_s2_v1.png", "commercial", f"Single isometric office building, four floors, blue glass windows, {STYLE}", 64, 96),
    ("com_s2_v2.png", "commercial", f"Single isometric shopping mall, three floors, large commercial, {STYLE}", 64, 96),
    ("com_s2_v3.png", "commercial", f"Single isometric hotel building, four floors, fancy entrance, {STYLE}", 64, 96),
    ("com_s2_v4.png", "commercial", f"Single isometric office tower, four floors, modern steel and glass, {STYLE}", 64, 96),
    ("com_s3_v1.png", "commercial", f"Single isometric tall skyscraper, ten floors, glass and steel corporate tower, {STYLE}", 64, 128),
    ("com_s3_v2.png", "commercial", f"Single isometric modern skyscraper, ten floors, reflective blue glass, {STYLE}", 64, 128),
    ("com_s3_v3.png", "commercial", f"Single isometric art deco skyscraper, ten floors, ornate top, {STYLE}", 64, 128),
    ("com_s3_v4.png", "commercial", f"Single isometric downtown tower, ten floors, sleek black glass, {STYLE}", 64, 128),

    # === INDUSTRIAL ===
    ("ind_s1_v1.png", "industrial", f"Single isometric small factory warehouse, corrugated metal walls, {STYLE}", 64, 64),
    ("ind_s1_v2.png", "industrial", f"Single isometric workshop building, small industrial, loading dock, {STYLE}", 64, 64),
    ("ind_s1_v3.png", "industrial", f"Single isometric storage facility, industrial shed, {STYLE}", 64, 64),
    ("ind_s2_v1.png", "industrial", f"Single isometric medium factory with smokestack, industrial plant, {STYLE}", 64, 80),
    ("ind_s2_v2.png", "industrial", f"Single isometric manufacturing plant, conveyor belts visible, {STYLE}", 64, 80),
    ("ind_s2_v3.png", "industrial", f"Single isometric refinery, tanks and pipes, industrial, {STYLE}", 64, 80),
    ("ind_s3_v1.png", "industrial", f"Single isometric large factory complex, multiple smokestacks belching smoke, heavy industry, {STYLE}", 64, 96),
    ("ind_s3_v2.png", "industrial", f"Single isometric steel mill, large industrial with glowing furnace, {STYLE}", 64, 96),
    ("ind_s3_v3.png", "industrial", f"Single isometric chemical plant, large industrial with cooling towers, {STYLE}", 64, 96),

    # === INFRASTRUCTURE ===
    ("power_plant.png", "infrastructure", f"Single isometric coal power plant, large building with cooling tower, orange glow, {STYLE}", 64, 80),
    ("power_line.png", "infrastructure", f"Single isometric electrical power line pylon, steel tower with wires, {STYLE}", 64, 48),
    ("water_tower.png", "infrastructure", f"Single isometric water tower, blue tank on tall steel stilts, {STYLE}", 64, 80),
    ("water_main.png", "infrastructure", f"Single isometric water pipe marker, blue pipe segment on ground, {STYLE}", 64, 32),
    ("monument.png", "infrastructure", f"Single isometric grand civic monument, tall golden statue on marble pedestal, impressive landmark, {STYLE}", 64, 128),
    ("park.png", "infrastructure", f"Single isometric city park, green trees benches fountain, peaceful, {STYLE}", 64, 48),

    # === EVENTS ===
    ("fire.png", "events", f"Single isometric building engulfed in flames, bright orange red fire, dramatic, {STYLE}", 64, 64),
    ("rubble.png", "events", f"Single isometric pile of rubble debris, destroyed building remains, grey brown, {STYLE}", 64, 32),
]


def generate_sprite(name, subdir, prompt, target_w, target_h):
    output_path = SPRITES_DIR / subdir / name
    if output_path.exists():
        print(f"  SKIP {subdir}/{name}")
        return True

    print(f"  GEN  {subdir}/{name} ...", end=" ", flush=True)

    try:
        response = client.models.generate_content(
            model=MODEL,
            contents=[prompt],
            config=types.GenerateContentConfig(
                response_modalities=["TEXT", "IMAGE"],
                image_config=types.ImageConfig(
                    aspect_ratio="1:1",
                    image_size="1K",
                ),
                http_options=types.HttpOptions(timeout=30_000),
            ),
        )

        for part in response.parts:
            if part.inline_data:
                # Save to temp file first (Gemini returns JPEG)
                import io
                img_bytes = part.inline_data.data
                pil_img = Image.open(io.BytesIO(img_bytes))
                # Resize to target dimensions with nearest-neighbor for pixel art
                pil_img = pil_img.resize((target_w, target_h), Image.NEAREST)
                # Save as PNG
                output_path.parent.mkdir(parents=True, exist_ok=True)
                pil_img.save(str(output_path), format="PNG")
                print("OK")
                return True

        print("FAIL (no image in response)")
        return False

    except Exception as e:
        print(f"FAIL ({e})")
        return False


def main():
    print(f"SlideCity Sprite Generator (Gemini)")
    print(f"===================================")
    print(f"Model: {MODEL}")
    print(f"Sprites: {len(SPRITES)}")
    print()

    ok = 0
    fail = 0
    skip = 0
    for name, subdir, prompt, w, h in SPRITES:
        path = SPRITES_DIR / subdir / name
        if path.exists():
            skip += 1
            print(f"  SKIP {subdir}/{name}")
            continue
        if generate_sprite(name, subdir, prompt, w, h):
            ok += 1
        else:
            fail += 1

    print(f"\nDone! Generated: {ok}, Skipped: {skip}, Failed: {fail}")


if __name__ == "__main__":
    main()
