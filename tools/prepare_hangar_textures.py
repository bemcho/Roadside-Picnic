"""Crop hangar reference swatches (requires Pillow).

python3 tools/prepare_hangar_textures.py image_3.png image_2.png image_1.png \
    image_0.png assets/textures/base
"""

import argparse
from pathlib import Path

from PIL import Image, ImageOps

from prepare_terrain_textures import tileable


SIZE = 192
# Board, then source bounds of albedo, normal, roughness, metallic and AO.
SURFACES = {
    "concrete": (
        1, (20, 659, 97, 736), (112, 659, 189, 736), (203, 659, 280, 736),
        (295, 659, 372, 736), (387, 659, 464, 736),
    ),
    "wall": (
        2, (54, 647, 110, 759), (220, 647, 277, 759), (388, 647, 445, 759),
        (555, 647, 612, 759), (722, 647, 779, 759),
    ),
    "steel": (
        0, (47, 608, 91, 665), (216, 608, 260, 665), (384, 608, 428, 665),
        (550, 608, 594, 665), (717, 608, 761, 665),
    ),
}


def build(boards, output):
    def sample(index, box, mode="RGB", seamless=True):
        image = boards[index].crop(box).convert(mode).resize((SIZE, SIZE), Image.Resampling.LANCZOS)
        return tileable(image) if seamless else image

    def save(image, name):
        image.save(output / f"{name}.png", optimize=True)

    for name, (board, albedo, normal, roughness, metallic, ao) in SURFACES.items():
        save(sample(board, albedo), name + "_albedo")
        save(sample(board, normal), name + "_normal")
        rough = sample(board, roughness, "L").point(lambda x: round(46 + x * 0.74))
        metal = sample(board, metallic, "L")
        ambient = sample(board, ao, "L").point(lambda x: round(78 + x * 0.69))
        save(Image.merge("RGB", (ambient, rough, metal)), name + "_arm")

    save(sample(3, (11, 579, 45, 625)), "grate_albedo")
    wear = sample(3, (523, 930, 617, 992), "L")
    save(ImageOps.colorize(wear, "#603c1d", "#e7ae31"), "yellow_albedo")
    clean = ImageOps.colorize(wear, "#644721", "#f7ba36")
    dark = ImageOps.colorize(wear, "#191718", "#3c3631")
    mask = Image.new("L", (SIZE, SIZE))
    mask.putdata([255 if ((x + y) % SIZE) < SIZE // 2 else 0 for y in range(SIZE) for x in range(SIZE)])
    save(Image.composite(clean, dark, mask), "hazard_albedo")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("boards", nargs=4, type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    boards = [Image.open(path).convert("RGB") for path in args.boards]
    if any(board.size != (1024, 1024) for board in boards):
        parser.error("Expected four 1024 x 1024 reference sheets")
    args.output.mkdir(parents=True, exist_ok=True)
    build(boards, args.output)
