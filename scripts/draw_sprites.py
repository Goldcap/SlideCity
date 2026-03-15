#!/usr/bin/env python3
"""
SlideCity Isometric Sprite Generator — Procedural Pixel Art
============================================================
Draws proper isometric sprites using PIL geometry.
No AI, no APIs — just math and color.
"""

from PIL import Image, ImageDraw
from pathlib import Path
import random

ROOT = Path(__file__).parent.parent
SPRITES_DIR = ROOT / "assets" / "sprites"

# Isometric tile dimensions
TW = 64  # tile width
TH = 32  # tile height (half of width for 2:1 iso)


def diamond_points(w, h):
    """Isometric diamond vertices: top, right, bottom, left."""
    cx, cy = w // 2, h // 2
    return [(cx, 0), (w - 1, cy), (cx, h - 1), (0, cy)]


def save(img, subdir, name):
    path = SPRITES_DIR / subdir / name
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(str(path), format="PNG")
    print(f"  {subdir}/{name}")


def draw_iso_diamond(draw, x, y, w, h, color, outline=None):
    """Draw a filled isometric diamond at (x,y) with size (w,h)."""
    cx = x + w // 2
    cy = y + h // 2
    pts = [(cx, y), (x + w - 1, cy), (cx, y + h - 1), (x, cy)]
    draw.polygon(pts, fill=color, outline=outline)


def draw_iso_box(draw, x, y, w, h, box_h, top_color, left_color, right_color):
    """Draw a 3D isometric box (building) with top diamond + two side faces."""
    cx = x + w // 2
    cy = y + h // 2

    # Top face (diamond, shifted up by box_h)
    top_pts = [
        (cx, y - box_h),
        (x + w - 1, cy - box_h),
        (cx, y + h - 1 - box_h),
        (x, cy - box_h),
    ]
    draw.polygon(top_pts, fill=top_color)

    # Right face
    right_pts = [
        (cx, y + h - 1 - box_h),
        (x + w - 1, cy - box_h),
        (x + w - 1, cy),
        (cx, y + h - 1),
    ]
    draw.polygon(right_pts, fill=right_color)

    # Left face
    left_pts = [
        (cx, y + h - 1 - box_h),
        (x, cy - box_h),
        (x, cy),
        (cx, y + h - 1),
    ]
    draw.polygon(left_pts, fill=left_color)


def shade(color, factor):
    """Darken or lighten a color."""
    return tuple(max(0, min(255, int(c * factor))) for c in color)


# ===== TERRAIN =====

def gen_terrain(name, base_color, detail_fn=None):
    img = Image.new("RGBA", (TW, TH), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 0, TW, TH, base_color)
    if detail_fn:
        detail_fn(draw, img)
    save(img, "terrain", name)


def grass_detail(draw, img):
    # Scattered darker grass spots
    random.seed(42)
    for _ in range(8):
        px = random.randint(16, 48)
        py = random.randint(4, 28)
        # Only draw if inside diamond
        if img.getpixel((px, py))[3] > 0:
            draw.point((px, py), fill=(60, 120, 40, 255))


def grass_flower_detail(draw, img):
    grass_detail(draw, img)
    random.seed(99)
    colors = [(220, 60, 60), (220, 180, 40), (180, 60, 200), (60, 120, 220)]
    for _ in range(5):
        px = random.randint(12, 52)
        py = random.randint(4, 28)
        if img.getpixel((px, py))[3] > 0:
            c = random.choice(colors)
            draw.point((px, py), fill=c + (255,))
            draw.point((px + 1, py), fill=c + (255,))


def tree_detail(draw, img, count=3):
    random.seed(77)
    positions = [(24, 8), (36, 12), (30, 18)] if count >= 3 else [(28, 10), (36, 16)]
    for tx, ty in positions[:count]:
        if img.getpixel((tx, ty))[3] > 0:
            # Trunk
            draw.rectangle([tx, ty, tx + 1, ty + 4], fill=(100, 70, 40, 255))
            # Canopy (small circle)
            draw.ellipse([tx - 3, ty - 4, tx + 4, ty + 1], fill=(30, 100, 25, 255))
            draw.ellipse([tx - 2, ty - 3, tx + 3, ty], fill=(40, 120, 30, 255))


def water_detail(draw, img):
    # Subtle wave lines
    for y in [10, 16, 22]:
        for x in range(16, 48, 4):
            if img.getpixel((x, y))[3] > 0:
                draw.point((x, y), fill=(40, 100, 200, 255))


def gen_all_terrain():
    gen_terrain("grass.png", (75, 140, 55))
    gen_terrain("grass_flower.png", (80, 145, 60), grass_flower_detail)
    gen_terrain("trees_dense.png", (50, 110, 35), lambda d, i: tree_detail(d, i, 3))
    gen_terrain("trees_sparse.png", (65, 130, 45), lambda d, i: tree_detail(d, i, 2))
    gen_terrain("sand.png", (194, 178, 128))
    gen_terrain("dirt.png", (115, 95, 65))
    gen_terrain("rock.png", (128, 122, 115))
    gen_terrain("snow.png", (220, 225, 235))
    gen_terrain("water.png", (35, 80, 170), water_detail)


# ===== ROADS =====

def gen_roads():
    # Straight road
    img = Image.new("RGBA", (TW, TH), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 0, TW, TH, (90, 90, 90))
    # Center lane marking
    draw.line([(TW // 2, 4), (TW // 2, TH - 4)], fill=(200, 200, 80, 255), width=1)
    save(img, "roads", "road_straight.png")

    # Crossroad
    img = Image.new("RGBA", (TW, TH), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 0, TW, TH, (90, 90, 90))
    draw.line([(TW // 2, 4), (TW // 2, TH - 4)], fill=(200, 200, 80, 255), width=1)
    draw.line([(12, TH // 2), (52, TH // 2)], fill=(200, 200, 80, 255), width=1)
    save(img, "roads", "road_cross.png")


# ===== BUILDINGS =====

def gen_building(subdir, name, box_h, top, left, right, detail_fn=None):
    """Generate a building sprite with isometric box shape."""
    img_h = TH + box_h
    img = Image.new("RGBA", (TW, img_h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Draw the 3D box anchored at the bottom of the image
    draw_iso_box(draw, 0, img_h - TH, TW, TH, box_h, top, left, right)

    if detail_fn:
        detail_fn(draw, img, img_h, box_h)

    save(img, subdir, name)


def window_detail(draw, img, img_h, box_h, color=(200, 220, 240)):
    """Add window dots to a building."""
    random.seed(hash(str(img.size)))
    cx = TW // 2
    base_y = img_h - TH // 2

    # Windows on left face
    for row in range(1, max(2, box_h // 8)):
        for col in range(2):
            wx = cx - 12 + col * 8
            wy = base_y - box_h + row * 8
            if 0 <= wx < TW and 0 <= wy < img_h:
                if img.getpixel((wx, wy))[3] > 0:
                    draw.rectangle([wx, wy, wx + 2, wy + 2], fill=color + (255,))

    # Windows on right face
    for row in range(1, max(2, box_h // 8)):
        for col in range(2):
            wx = cx + 4 + col * 8
            wy = base_y - box_h + row * 8
            if 0 <= wx < TW and 0 <= wy < img_h:
                if img.getpixel((wx, wy))[3] > 0:
                    draw.rectangle([wx, wy, wx + 2, wy + 2], fill=color + (255,))


def gen_residential():
    # Stage 1: small houses (4 variants)
    colors = [
        ((180, 60, 50), (140, 50, 40), (160, 55, 45)),   # red roof
        ((60, 100, 180), (50, 80, 140), (55, 90, 160)),   # blue
        ((160, 130, 80), (130, 100, 60), (145, 115, 70)), # brown
        ((180, 170, 80), (140, 130, 60), (160, 150, 70)), # yellow
    ]
    for i, (top, left, right) in enumerate(colors):
        gen_building("residential", f"res_s1_v{i+1}.png", 10, top, left, right)

    # Stage 2: two-story (4 variants)
    colors2 = [
        ((200, 200, 200), (160, 160, 160), (180, 180, 180)),
        ((180, 120, 90), (140, 95, 70), (160, 108, 80)),
        ((160, 160, 180), (130, 130, 150), (145, 145, 165)),
        ((190, 180, 170), (150, 140, 130), (170, 160, 150)),
    ]
    for i, (top, left, right) in enumerate(colors2):
        gen_building("residential", f"res_s2_v{i+1}.png", 22, top, left, right, window_detail)

    # Stage 3: apartments (4 variants)
    colors3 = [
        ((210, 200, 180), (170, 160, 140), (190, 180, 160)),
        ((180, 190, 200), (140, 150, 160), (160, 170, 180)),
        ((190, 170, 150), (150, 130, 110), (170, 150, 130)),
        ((220, 220, 220), (180, 180, 180), (200, 200, 200)),
    ]
    for i, (top, left, right) in enumerate(colors3):
        gen_building("residential", f"res_s3_v{i+1}.png", 45, top, left, right, window_detail)


def gen_commercial():
    # Stage 1: small shops
    colors = [
        ((60, 80, 180), (50, 65, 140), (55, 72, 160)),
        ((80, 60, 160), (65, 50, 130), (72, 55, 145)),
        ((60, 140, 160), (50, 110, 130), (55, 125, 145)),
        ((100, 80, 150), (80, 65, 120), (90, 72, 135)),
    ]
    for i, (top, left, right) in enumerate(colors):
        gen_building("commercial", f"com_s1_v{i+1}.png", 12, top, left, right)

    # Stage 2: offices
    colors2 = [
        ((80, 120, 200), (60, 95, 160), (70, 108, 180)),
        ((100, 100, 180), (80, 80, 140), (90, 90, 160)),
        ((70, 130, 170), (55, 100, 135), (62, 115, 152)),
        ((90, 110, 190), (70, 85, 150), (80, 98, 170)),
    ]
    for i, (top, left, right) in enumerate(colors2):
        gen_building("commercial", f"com_s2_v{i+1}.png", 40, top, left, right,
                     lambda d, i, h, bh: window_detail(d, i, h, bh, (180, 210, 240)))

    # Stage 3: skyscrapers
    colors3 = [
        ((100, 140, 210), (70, 100, 170), (85, 120, 190)),
        ((120, 150, 200), (85, 110, 160), (102, 130, 180)),
        ((90, 130, 190), (65, 95, 150), (78, 112, 170)),
        ((60, 60, 80), (45, 45, 60), (52, 52, 70)),
    ]
    for i, (top, left, right) in enumerate(colors3):
        gen_building("commercial", f"com_s3_v{i+1}.png", 80, top, left, right,
                     lambda d, i, h, bh: window_detail(d, i, h, bh, (160, 200, 240)))


def gen_industrial():
    # Stage 1: warehouses
    colors = [
        ((180, 160, 60), (140, 125, 50), (160, 142, 55)),
        ((170, 150, 80), (135, 118, 62), (152, 134, 71)),
        ((160, 145, 70), (125, 112, 55), (142, 128, 62)),
    ]
    for i, (top, left, right) in enumerate(colors):
        gen_building("industrial", f"ind_s1_v{i+1}.png", 16, top, left, right)

    # Stage 2: factories
    colors2 = [
        ((170, 150, 50), (130, 115, 40), (150, 132, 45)),
        ((160, 140, 60), (125, 108, 48), (142, 124, 54)),
        ((150, 135, 55), (118, 105, 43), (134, 120, 49)),
    ]
    for i, (top, left, right) in enumerate(colors2):
        def smokestack(draw, img, img_h, box_h, idx=i):
            # Small chimney on top
            sx = TW // 2 + 8
            sy = img_h - TH // 2 - box_h - 8
            draw.rectangle([sx, sy, sx + 3, sy + 10], fill=(100, 90, 80, 255))
        gen_building("industrial", f"ind_s2_v{i+1}.png", 30, top, left, right, smokestack)

    # Stage 3: heavy industry
    colors3 = [
        ((160, 140, 40), (120, 105, 30), (140, 122, 35)),
        ((150, 130, 50), (115, 100, 38), (132, 115, 44)),
        ((140, 125, 45), (108, 96, 35), (124, 110, 40)),
    ]
    for i, (top, left, right) in enumerate(colors3):
        def big_smokestack(draw, img, img_h, box_h, idx=i):
            sx = TW // 2 + 10
            sy = img_h - TH // 2 - box_h - 14
            draw.rectangle([sx, sy, sx + 4, sy + 16], fill=(90, 80, 70, 255))
            # Smoke puff
            draw.ellipse([sx - 2, sy - 6, sx + 6, sy], fill=(180, 180, 180, 150))
        gen_building("industrial", f"ind_s3_v{i+1}.png", 50, top, left, right, big_smokestack)


# ===== INFRASTRUCTURE =====

def gen_infrastructure():
    # Power plant
    gen_building("infrastructure", "power_plant.png", 35,
                 (200, 130, 40), (160, 100, 30), (180, 115, 35),
                 lambda d, i, h, bh: d.rectangle(
                     [TW // 2 + 6, h - TH // 2 - bh - 12, TW // 2 + 11, h - TH // 2 - bh + 4],
                     fill=(150, 140, 130, 255)))

    # Power line
    img = Image.new("RGBA", (TW, TH + 16), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 16, TW, TH, (90, 85, 75))
    # Pylon
    draw.line([(TW // 2, 0), (TW // 2, 20)], fill=(120, 110, 100, 255), width=2)
    draw.line([(TW // 2 - 6, 4), (TW // 2 + 6, 4)], fill=(120, 110, 100, 255), width=1)
    save(img, "infrastructure", "power_line.png")

    # Water tower
    gen_building("infrastructure", "water_tower.png", 45,
                 (50, 120, 220), (40, 95, 175), (45, 108, 198))

    # Water main
    img = Image.new("RGBA", (TW, TH), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 0, TW, TH, (60, 110, 170))
    # Pipe marking
    draw.line([(20, TH // 2), (44, TH // 2)], fill=(40, 80, 140, 255), width=2)
    save(img, "infrastructure", "water_main.png")

    # Monument
    img_h = TH + 90
    img = Image.new("RGBA", (TW, img_h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # Base
    draw_iso_box(draw, 0, img_h - TH, TW, TH, 12, (200, 190, 170), (160, 150, 130), (180, 170, 150))
    # Obelisk/spire
    cx = TW // 2
    base_y = img_h - TH // 2 - 12
    draw.polygon([(cx, 4), (cx - 6, base_y), (cx + 6, base_y)], fill=(180, 160, 200, 255))
    draw.polygon([(cx, 4), (cx - 4, base_y), (cx + 4, base_y)], fill=(200, 180, 220, 255))
    # Gold tip
    draw.polygon([(cx, 0), (cx - 3, 8), (cx + 3, 8)], fill=(220, 200, 60, 255))
    save(img, "infrastructure", "monument.png")

    # Park
    img = Image.new("RGBA", (TW, TH + 10), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 10, TW, TH, (50, 160, 50))
    # Trees
    for tx, ty in [(20, 8), (40, 6), (30, 12)]:
        draw.rectangle([tx, ty, tx + 1, ty + 4], fill=(80, 60, 40, 255))
        draw.ellipse([tx - 3, ty - 4, tx + 4, ty + 1], fill=(35, 110, 30, 255))
    # Bench
    draw.rectangle([28, 20, 36, 22], fill=(140, 100, 60, 255))
    save(img, "infrastructure", "park.png")


# ===== EVENTS =====

def gen_events():
    # Fire
    img_h = TH + 24
    img = Image.new("RGBA", (TW, img_h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    # Charred base
    draw_iso_diamond(draw, 0, 24, TW, TH, (60, 40, 30))
    # Flames
    random.seed(55)
    for _ in range(12):
        fx = random.randint(16, 48)
        fy = random.randint(4, 28)
        fh = random.randint(4, 16)
        fw = random.randint(2, 5)
        color = random.choice([(240, 100, 20), (240, 180, 30), (220, 60, 10)])
        draw.ellipse([fx - fw, fy - fh, fx + fw, fy], fill=color + (200,))
    save(img, "events", "fire.png")

    # Rubble
    img = Image.new("RGBA", (TW, TH), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_iso_diamond(draw, 0, 0, TW, TH, (100, 90, 80))
    # Rubble chunks
    random.seed(33)
    for _ in range(8):
        rx = random.randint(14, 50)
        ry = random.randint(6, 26)
        if img.getpixel((rx, ry))[3] > 0:
            rs = random.randint(2, 4)
            c = random.choice([(120, 110, 100), (90, 80, 70), (140, 130, 120)])
            draw.rectangle([rx, ry, rx + rs, ry + rs], fill=c + (255,))
    save(img, "events", "rubble.png")


def main():
    print("SlideCity Procedural Sprite Generator")
    print("=====================================")
    print()

    print("Terrain:")
    gen_all_terrain()
    print()
    print("Roads:")
    gen_roads()
    print()
    print("Residential:")
    gen_residential()
    print()
    print("Commercial:")
    gen_commercial()
    print()
    print("Industrial:")
    gen_industrial()
    print()
    print("Infrastructure:")
    gen_infrastructure()
    print()
    print("Events:")
    gen_events()
    print()
    print("Done! All sprites generated with proper isometric geometry.")


if __name__ == "__main__":
    main()
