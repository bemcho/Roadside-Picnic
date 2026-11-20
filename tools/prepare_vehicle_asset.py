"""Reproject the reference swatches and split the four wheel pivots.

blender -b --python tools/prepare_vehicle_asset.py -- source.glb out.glb \
  textture_vehicle.png 'image_3 (1).png' 'image_2 (1).png' 'image_1 (1).png'
"""

import math
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

import bpy
import numpy as np

SOURCE, OUTPUT, *BOARDS = sys.argv[sys.argv.index("--") + 1 :]
if len(BOARDS) != 4:
    raise ValueError("Expected the primary texture sheet, then image_3, image_2, and image_1")

SIZE, PAD = 256, 4
PLAIN, SIDE, FRONT, REAR, WHEEL, TREAD = range(6)
VIEWS = {SIDE: (-150, 30, 850, 540), FRONT: (60, 30, 640, 540), REAR: (60, 30, 640, 540)}

# Crops are (sheet, left, top, right, bottom) at the reference boards' 1024 px size.
# A swatch has base color, normal, roughness, metallic and occlusion, in that order.
ARMOR = ((0, 123, 48, 147, 68), (0, 379, 48, 403, 68), (0, 635, 48, 659, 68),
         (0, 891, 48, 915, 68), (0, 123, 304, 147, 324))
GLASS = ((0, 16, 761, 104, 794), (0, 272, 761, 360, 794), (0, 528, 761, 616, 794),
         0.05, 0.95)
GRILLE = ((2, 102, 351, 191, 379), (2, 443, 351, 532, 379), (2, 785, 351, 874, 379),
          0.55, 0.75)
HEADLIGHT = ((1, 316, 284, 338, 302), (128, 128, 255), 0.3, 0.05, 1.0)
TAIL_LIGHT = ((1, 520, 853, 540, 881), (128, 128, 255), 0.31, 0.04, 1.0)
WHEEL_FACE = ((3, 10, 597, 181, 769), (3, 266, 597, 437, 769), (3, 522, 597, 693, 769),
              0.16, 0.92)
RUBBER = ((0, 143, 527, 164, 704), (0, 399, 527, 420, 704), (0, 655, 527, 676, 704),
          0.0, 1.0)


def load_board(path):
    image = bpy.data.images.load(path)
    if tuple(image.size) != (1024, 1024):
        raise ValueError(f"{path} must be a 1024 x 1024 reference sheet")
    pixels = np.empty(1024 * 1024 * 4, dtype=np.float32)
    image.pixels.foreach_get(pixels)
    bpy.data.images.remove(image)
    return pixels.reshape(1024, 1024, 4)[::-1, :, :3]


def paint(tile, swatch, boards, points=None):
    if points is None:
        rows, cols = slice(None), slice(None)
        height = width = SIZE - 2 * PAD
        mask = np.ones((height, width), dtype=bool)
    else:
        points = np.asarray(points, dtype=float)
        lo, hi = np.floor(points.min(axis=0)).astype(int), np.ceil(points.max(axis=0)).astype(int)
        width, height = hi - lo
        cols, rows = slice(lo[0], hi[0]), slice(lo[1], hi[1])
        y, x = np.mgrid[rows, cols] + 0.5
        mask = np.zeros((height, width), dtype=bool)
        for (x1, y1), (x2, y2) in zip(points, np.roll(points, -1, axis=0)):
            if y1 != y2:
                mask ^= ((y1 > y) != (y2 > y)) & (x < (x2 - x1) * (y - y1) / (y2 - y1) + x1)

    def sample(spec):
        if not isinstance(spec, tuple) or len(spec) == 3:
            return np.asarray(spec, dtype=np.float32) / (255 if isinstance(spec, tuple) else 1)
        board, left, top, right, bottom = spec
        x = np.linspace(left, right - 1, width)
        y = np.linspace(top, bottom - 1, height)
        # Bilinear resampling keeps enlarged thumbnail crops from pixelating.
        xi, yi = x.astype(int), y.astype(int)
        dx, dy = (x - xi)[None, :, None], (y - yi)[:, None, None]
        src = boards[board]
        a = src[yi[:, None], xi] * (1 - dx) + src[yi[:, None], xi + 1] * dx
        b = src[(yi + 1)[:, None], xi] * (1 - dx) + src[(yi + 1)[:, None], xi + 1] * dx
        return a * (1 - dy) + b * dy

    def channel(spec):
        pixels = sample(spec)
        return pixels.mean(axis=-1) if pixels.ndim else pixels

    albedo, normal, roughness, metallic, occlusion = swatch
    rough = channel(roughness)
    metal = channel(metallic)
    if swatch is ARMOR:
        rough = 0.45 + 0.45 * rough
        metal *= 0.22
    elif swatch is RUBBER:
        rough = 0.53 + 0.42 * rough
    data = np.stack(np.broadcast_arrays(channel(occlusion), rough, metal), axis=-1)
    for surface, values in zip(tile, (sample(albedo), sample(normal), data)):
        surface[rows, cols][mask] = np.broadcast_to(values, (height, width, 3))[mask]


def box(left, top, right, bottom):
    return ((left, top), (right, top), (right, bottom), (left, bottom))


def circle(x, y, radius):
    return [(x + radius * math.cos(angle), y + radius * math.sin(angle))
            for angle in np.linspace(0, math.tau, 24, endpoint=False)]


def make_atlas():
    boards = [load_board(path) for path in BOARDS]
    inner = SIZE - 2 * PAD
    tiles = np.empty((6, 3, inner, inner, 3), dtype=np.float32)
    for tile in (PLAIN, SIDE, FRONT, REAR):
        paint(tiles[tile], ARMOR, boards)
    paint(tiles[WHEEL], WHEEL_FACE, boards)
    paint(tiles[TREAD], RUBBER, boards)

    def detail(view, swatch, points):
        left, top, right, bottom = VIEWS[view]
        projected = [((x - left) * inner / (right - left), (y - top) * inner / (bottom - top))
                     for x, y in points]
        paint(tiles[view], swatch, boards, projected)

    # Outline the recessed panes and lamps in orthographic views of this mesh.
    for view, windows in (
        (SIDE, [((225, 185), (253, 155), (326, 155), (331, 220), (225, 220)),
                ((376, 150), (494, 150), (501, 220), (375, 220))]),
        (FRONT, [((202, 141), (336, 141), (338, 208), (177, 208)),
                 ((362, 141), (500, 141), (531, 208), (361, 208))]),
        (REAR, [((195, 150), (229, 148), (232, 211), (175, 211)),
                ((284, 154), (415, 154), (421, 211), (281, 211)),
                ((473, 148), (505, 150), (527, 211), (470, 211))]),
    ):
        for pane in windows:
            detail(view, GLASS, pane)
    detail(FRONT, GRILLE, box(249, 265, 451, 320))
    for x in (198, 503):
        detail(FRONT, HEADLIGHT, circle(x, 282, 20))
    for left in (184, 485):
        detail(REAR, TAIL_LIGHT, box(left, 274, left + 31, 315))
    padded = np.pad(tiles, ((0, 0), (0, 0), (PAD, PAD), (PAD, PAD), (0, 0)), mode="edge")
    return padded.reshape(2, 3, 3, SIZE, SIZE, 3).transpose(2, 0, 3, 1, 4, 5).reshape(3, 2 * SIZE, 3 * SIZE, 3)


def vehicle_material(atlas, directory):
    material = bpy.data.materials.new("Weathered armor")
    material.use_nodes = True
    nodes, links = material.node_tree.nodes, material.node_tree.links
    surface = nodes.get("Principled BSDF")
    surface.inputs["Metallic"].default_value = 1.0
    surface.inputs["Roughness"].default_value = 1.0
    color, normal, arm = (nodes.new("ShaderNodeTexImage") for _ in range(3))
    for slot, pixels, name in zip((color, normal, arm), atlas, ("albedo", "normal", "arm")):
        h, w, _ = pixels.shape
        rgba = np.ones((h, w, 4), dtype=np.float32)
        rgba[:, :, :3] = pixels[::-1]
        image = bpy.data.images.new(name, width=w, height=h)
        image.pixels.foreach_set(rgba.ravel())
        path = str(Path(directory) / f"armored_vehicle_{name}.png")
        image.filepath_raw = path
        image.file_format = "PNG"
        image.save()
        bpy.data.images.remove(image)
        image = bpy.data.images.load(path)
        if name != "albedo":
            image.colorspace_settings.name = "Non-Color"
        image.pack()
        slot.image = image
        slot.extension = "EXTEND"

    links.new(color.outputs["Color"], surface.inputs["Base Color"])
    normal_map = nodes.new("ShaderNodeNormalMap")
    normal_map.inputs["Strength"].default_value = 0.45
    links.new(normal.outputs["Color"], normal_map.inputs["Color"])
    links.new(normal_map.outputs["Normal"], surface.inputs["Normal"])
    channels = nodes.new("ShaderNodeSeparateColor")
    links.new(arm.outputs["Color"], channels.inputs["Color"])
    links.new(channels.outputs["Green"], surface.inputs["Roughness"])
    links.new(channels.outputs["Blue"], surface.inputs["Metallic"])
    settings = nodes.new("ShaderNodeGroup")
    settings.node_tree = bpy.data.node_groups.new("glTF Material Output", "ShaderNodeTree")
    settings.node_tree.interface.new_socket(name="Occlusion", in_out="INPUT", socket_type="NodeSocketFloat")
    links.new(channels.outputs["Red"], settings.inputs["Occlusion"])
    return material


def mesh_parts(mesh):
    adjacency = [[] for _ in mesh.vertices]
    for edge in mesh.edges:
        a, b = edge.vertices
        adjacency[a].append(b)
        adjacency[b].append(a)
    remaining = set(range(len(mesh.vertices)))
    wheels = {}
    while remaining:
        stack = [remaining.pop()]
        island = []
        while stack:
            vertex = stack.pop()
            island.append(vertex)
            for neighbor in adjacency[vertex]:
                if neighbor in remaining:
                    remaining.remove(neighbor)
                    stack.append(neighbor)
        points = np.array([mesh.vertices[i].co[:] for i in island])
        if len(island) > 500 and points[:, 2].max() < 0.21:
            center = (points.min(axis=0) + points.max(axis=0)) / 2
            name = "Wheel" + ("Front" if center[1] < 0 else "Rear") + ("Left" if center[0] > 0 else "Right")
            wheels[name] = (set(island), center)
    if len(wheels) != 4:
        raise ValueError(f"Expected four wheel islands, found {list(wheels)}")
    hull = set(range(len(mesh.vertices))).difference(*(vertices for vertices, _ in wheels.values()))
    return {"Chassis": (hull, np.zeros(3)), **wheels}


def map_face(face, points, wheel):
    nx, ny, nz = map(abs, face.normal)
    if wheel:
        if nx > 0.62:
            return WHEEL, [(0.5 + y / 0.205, 0.5 - z / 0.205) for x, y, z in points]
        angles = [(math.atan2(y, z) / math.tau) % 1.0 for _, y, z in points]
        if max(angles) - min(angles) > 0.5:
            angles = [angle if angle < 0.5 else angle - 1.0 for angle in angles]
        return TREAD, [(0.5 + p[0] / 0.09, angle) for p, angle in zip(points, angles)]

    _, _, z = face.center
    if ny > nx * 0.85 and (ny > nz or (0.32 < z < 0.415 and ny > 0.45)):
        front = face.normal.y < 0
        kind = FRONT if front else REAR
        coords = [(350 + (x if front else -x) * 1000, 540 - z * 1000) for x, _, z in points]
    elif nx >= nz:
        kind = SIDE
        coords = [(350 + y * 1000, 540 - z * 1000) for _, y, z in points]
    else:
        return PLAIN, [(0.5 + x / 0.58, 0.5 - y) for x, y, _ in points]
    left, top, right, bottom = VIEWS[kind]
    return kind, [((x - left) / (right - left), (y - top) / (bottom - top)) for x, y in coords]


def export_part(source, name, selection, center, material):
    faces = [face for face in source.polygons if face.vertices[0] in selection]
    indices = sorted({index for face in faces for index in face.vertices})
    remap = {index: i for i, index in enumerate(indices)}
    points = [np.asarray(source.vertices[i].co[:]) - center for i in indices]
    data = bpy.data.meshes.new(name)
    data.from_pydata(points, [], [tuple(remap[i] for i in f.vertices) for f in faces])
    data.update()
    data.materials.append(material)
    data.normals_split_custom_set([source.corner_normals[i].vector for f in faces for i in f.loop_indices])
    uv = data.uv_layers.new().data
    for group, face in zip(data.polygons, faces):
        group.use_smooth = True
        tile, mapped = map_face(face, [points[i] for i in group.vertices], name != "Chassis")
        for i, (u, v) in zip(group.loop_indices, np.clip(mapped, 0.0, 1.0)):
            px = (tile % 3) * SIZE + PAD + u * (SIZE - 2 * PAD)
            py = (tile // 3) * SIZE + PAD + v * (SIZE - 2 * PAD)
            uv[i].uv = (px / (3 * SIZE), 1 - py / (2 * SIZE))
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    obj.location = center
    obj.select_set(True)


bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=SOURCE)
source = next(obj for obj in bpy.context.scene.objects if obj.type == "MESH")
bpy.ops.object.select_all(action="DESELECT")
with TemporaryDirectory() as directory:
    material = vehicle_material(make_atlas(), directory)
    for name, (selection, center) in mesh_parts(source.data).items():
        export_part(source.data, name, selection, center, material)
    bpy.ops.export_scene.gltf(
        filepath=OUTPUT, export_format="GLB", use_selection=True, export_tangents=True,
    )
