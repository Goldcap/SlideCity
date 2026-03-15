#!/usr/bin/env python3
"""
Mask ground-level sprites to isometric diamond shape.
Buildings keep their rectangular shape (they extend upward).
Adds RGBA alpha channel with diamond mask to ground tiles.
"""

import numpy as np
from PIL import Image, ImageDraw
from pathlib import Path

ROOT = Path(__file__).parent.parent
SPRITES_DIR = ROOT / "assets" / "sprites"


def make_diamond_mask(w, h):
    """Create an isometric diamond-shaped alpha mask."""
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    # Diamond: top-center, right-center, bottom-center, left-center
    cx, cy = w // 2, h // 2
    points = [
        (cx, 0),      # top
        (w - 1, cy),   # right
        (cx, h - 1),   # bottom
        (0, cy),       # left
    ]
    draw.polygon(points, fill=255)
    return mask


def make_building_mask(w, h):
    """Mask for buildings: diamond base at bottom, rectangular body above."""
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)

    # The bottom portion is the isometric diamond (base_h = width/2 for 2:1 iso)
    base_h = w // 2  # 32px for 64px wide
    base_top = h - base_h

    # Diamond at the bottom
    cx = w // 2
    diamond = [
        (cx, base_top),       # top of diamond
        (w - 1, base_top + base_h // 2),  # right
        (cx, h - 1),           # bottom
        (0, base_top + base_h // 2),      # left
    ]
    draw.polygon(diamond, fill=255)

    # Rectangular body above the diamond, narrower than full width
    # Buildings taper: 80% of tile width
    body_w = int(w * 0.8)
    body_x = (w - body_w) // 2
    if base_top > 0:
        draw.rectangle([body_x, 0, body_x + body_w, base_top + base_h // 4], fill=255)

    return mask


def apply_mask(img_path, mask_fn):
    """Apply a mask function to an image, converting to RGBA."""
    img = Image.open(img_path)
    if img.mode != "RGBA":
        img = img.convert("RGBA")

    mask = mask_fn(img.width, img.height)
    # Apply mask to alpha channel
    r, g, b, a = img.split()
    # Combine existing alpha with diamond mask
    new_a = Image.fromarray(np.minimum(np.array(a), np.array(mask)))
    img = Image.merge("RGBA", (r, g, b, new_a))
    img.save(str(img_path), format="PNG")


def main():
    print("Masking sprites to isometric shapes")
    print("====================================")
    count = 0

    # Ground tiles: diamond mask
    ground_dirs = ["terrain", "roads"]
    for subdir in ground_dirs:
        d = SPRITES_DIR / subdir
        for f in sorted(d.glob("*.png")):
            if f.name == ".gitkeep":
                continue
            print(f"  DIAMOND  {subdir}/{f.name}")
            apply_mask(f, make_diamond_mask)
            count += 1

    # Infrastructure ground-level: diamond mask
    ground_infra = ["water_main.png", "park.png", "power_line.png"]
    for name in ground_infra:
        f = SPRITES_DIR / "infrastructure" / name
        if f.exists():
            print(f"  DIAMOND  infrastructure/{name}")
            apply_mask(f, make_diamond_mask)
            count += 1

    # Events ground-level: diamond for rubble
    rubble = SPRITES_DIR / "events" / "rubble.png"
    if rubble.exists():
        print(f"  DIAMOND  events/rubble.png")
        apply_mask(rubble, make_diamond_mask)
        count += 1

    # Buildings: building mask (diamond base + rectangular body)
    building_dirs = ["residential", "commercial", "industrial"]
    for subdir in building_dirs:
        d = SPRITES_DIR / subdir
        for f in sorted(d.glob("*.png")):
            if f.name == ".gitkeep":
                continue
            print(f"  BUILDING {subdir}/{f.name}")
            apply_mask(f, make_building_mask)
            count += 1

    # Tall infrastructure: building mask
    tall_infra = ["power_plant.png", "water_tower.png", "monument.png"]
    for name in tall_infra:
        f = SPRITES_DIR / "infrastructure" / name
        if f.exists():
            print(f"  BUILDING infrastructure/{name}")
            apply_mask(f, make_building_mask)
            count += 1

    # Fire: building mask (flames extend upward)
    fire = SPRITES_DIR / "events" / "fire.png"
    if fire.exists():
        print(f"  BUILDING events/fire.png")
        apply_mask(fire, make_building_mask)
        count += 1

    print(f"\nDone! Masked {count} sprites.")


if __name__ == "__main__":
    main()
