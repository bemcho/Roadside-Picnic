"""Extract four fauna models and apply cropped maps from the supplied boards."""
import argparse
import os
import sys

import bmesh
import bpy

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source")
parser.add_argument("models")
parser.add_argument("textures")
parser.add_argument(
    "--boards",
    nargs=4,
    metavar=("IMAGE_3", "IMAGE_2", "IMAGE_1", "IMAGE_0"),
    help="crop maps from the reference sheets; otherwise reuse the saved PNG maps",
)
args = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :])
SRC, MODEL_DIR, TEXTURE_DIR = args.source, args.models, args.textures
os.makedirs(MODEL_DIR, exist_ok=True)
os.makedirs(TEXTURE_DIR, exist_ok=True)

MODELS = [
    ("dead_forest", ("dead_forest", "petrified", "mud_bog")),
    ("overgrowth", ("overgrowth", "mud_bog")),
    ("mud_bog", ("mud_bog",)),
    ("slime_ditch", ("slime_ditch",)),
]

# Pixel bounds on the 1024px sheets, excluding frames, labels and previews.
TEXTURE_CROPS = {
    "petrified": (
        (3, (397, 44, 533, 174)),
        (0, (423, 103, 497, 164)),
        (0, (423, 187, 497, 246)),
    ),
    "dead_forest": (
        (2, (627, 406, 689, 591)),
        (1, (628, 595, 693, 674)),
        (1, (701, 595, 769, 674)),
    ),
    "overgrowth": (
        (1, (600, 206, 691, 283)),
        (1, (703, 206, 789, 283)),
        (1, (800, 206, 885, 283)),
    ),
    "mud_bog": (
        (1, (20, 595, 108, 674)),
        (1, (119, 595, 207, 674)),
        (1, (216, 595, 307, 674)),
    ),
    "slime_ditch": (
        (1, (314, 884, 392, 969)),
        (1, (406, 884, 484, 969)),
        (1, (498, 884, 575, 969)),
    ),
}
MATERIALS = {
    "dead_forest": ("Dead birch bark", 0.0),
    "petrified": ("Petrified vitreous bark", 0.12),
    "overgrowth": ("Anomalous foliage", 0.0),
    "mud_bog": ("Mud and oil", 0.0),
    "slime_ditch": ("Slime and puddles", 0.16),
}
MAP_TYPES = ("albedo", "normal", "roughness")


def save_map(image, path):
    image.filepath_raw = path
    image.file_format = "PNG"
    image.save()
    image.pack()


def crop_texture_maps():
    boards = [bpy.data.images.load(path) for path in args.boards]
    board_pixels = [list(image.pixels) for image in boards]
    for asset_name, crops in TEXTURE_CROPS.items():
        for map_type, (board_index, box) in zip(MAP_TYPES, crops):
            source_width, source_height = boards[board_index].size
            left, top, right, bottom = (
                round(value * (source_width if axis % 2 == 0 else source_height) / 1024)
                for axis, value in enumerate(box)
            )
            width, height = right - left, bottom - top
            pixels = []
            for y in range(source_height - bottom, source_height - top):
                offset = (y * source_width + left) * 4
                pixels.extend(board_pixels[board_index][offset : offset + width * 4])
            image = bpy.data.images.new(
                f"{asset_name}_{map_type}", width=width, height=height
            )
            image.pixels = pixels
            save_map(image, os.path.join(TEXTURE_DIR, f"{asset_name}_{map_type}.png"))
            bpy.data.images.remove(image)
    for board in boards:
        bpy.data.images.remove(board)


def textured_material(asset_name):
    name, metallic = MATERIALS[asset_name]
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    principled = material.node_tree.nodes.get("Principled BSDF")
    principled.inputs["Base Color"].default_value = (1.0, 1.0, 1.0, 1.0)
    principled.inputs["Metallic"].default_value = metallic
    for map_type in MAP_TYPES:
        image = bpy.data.images.load(
            os.path.join(TEXTURE_DIR, f"{asset_name}_{map_type}.png"),
            check_existing=True,
        )
        if map_type != "albedo":
            image.colorspace_settings.name = "Non-Color"
        image.pack()
        texture = material.node_tree.nodes.new("ShaderNodeTexImage")
        texture.image = image
        if map_type == "normal":
            normal_map = material.node_tree.nodes.new("ShaderNodeNormalMap")
            normal_map.inputs["Strength"].default_value = 0.6
            material.node_tree.links.new(texture.outputs["Color"], normal_map.inputs["Color"])
            material.node_tree.links.new(normal_map.outputs["Normal"], principled.inputs["Normal"])
        else:
            target = "Base Color" if map_type == "albedo" else "Roughness"
            material.node_tree.links.new(texture.outputs["Color"], principled.inputs[target])
    return material


def apply_texture_uvs(data, materials):
    height = max(v.co.z for v in data.vertices) - min(v.co.z for v in data.vertices)
    span = max(
        max(v.co[axis] for v in data.vertices) - min(v.co[axis] for v in data.vertices)
        for axis in (0, 1)
    )
    uv_data = data.uv_layers.new(name="UVMap").data
    for polygon in data.polygons:
        axis = max(range(3), key=lambda axis: abs(polygon.normal[axis]))
        horizontal = 1 if axis == 0 else 0
        vertical = 1 if axis == 2 else 2
        bark = materials[polygon.material_index] in ("dead_forest", "petrified")
        u_scale, v_scale = (10.0 / height, 2.0 / height) if bark else (3.0 / span,) * 2
        for loop_index in polygon.loop_indices:
            point = data.vertices[data.loops[loop_index].vertex_index].co
            uv_data[loop_index].uv = (point[horizontal] * u_scale, point[vertical] * v_scale)


bpy.ops.wm.read_factory_settings(use_empty=True)
if args.boards:
    crop_texture_maps()
materials = {name: textured_material(name) for name in MATERIALS}
bpy.ops.import_scene.gltf(filepath=SRC)
source = next(obj for obj in bpy.context.scene.objects if obj.type == "MESH")
source_mesh = source.data

adjacency = [set() for _ in source_mesh.vertices]
for edge in source_mesh.edges:
    a, b = edge.vertices
    adjacency[a].add(b)
    adjacency[b].add(a)

components = []
remaining = set(range(len(source_mesh.vertices)))
while remaining:
    stack = [remaining.pop()]
    vertices = []
    while stack:
        vertex = stack.pop()
        vertices.append(vertex)
        for neighbor in adjacency[vertex]:
            if neighbor in remaining:
                remaining.remove(neighbor)
                stack.append(neighbor)
    points = [source_mesh.vertices[index].co for index in vertices]
    center = tuple(sum(point[axis] for point in points) / len(points) for axis in range(3))
    dimensions = tuple(
        max(point[axis] for point in points) - min(point[axis] for point in points)
        for axis in range(3)
    )
    components.append((vertices, center, dimensions))


def model_for(center):
    x, y, _ = center
    # Only the main clusters are scenery: the sheet also reconstructed texture
    # cards, a flat birch illustration, material balls and miniature LOD previews.
    if x < -0.12 and y > 0.3:
        return 0
    if 0.03 < x < 0.36 and y > 0.3:
        return 1
    if x < -0.12 and -0.16 < y < 0.2:
        return 2
    if x > 0.12 and -0.14 < y < 0.16:
        return 3
    return None


groups = [[] for _ in MODELS]
petrified_vertices = set()
for vertices, center, dimensions in components:
    smallest, largest = min(dimensions), max(dimensions)
    if 85 <= len(vertices) <= 105 and smallest > 0.04 and largest / smallest < 1.2:
        continue
    model_index = model_for(center)
    if model_index is None:
        continue
    groups[model_index].append(set(vertices))
    if model_index == 0 and center[0] > -0.3:
        petrified_vertices.update(vertices)

if any(not group for group in groups):
    raise ValueError(f"source GLB did not contain all four models: {[len(g) for g in groups]}")

for index, (filename, material_names) in enumerate(MODELS):
    selected = set().union(*groups[index])
    low = min(source_mesh.vertices[vertex].co.z for vertex in selected)
    high = max(source_mesh.vertices[vertex].co.z for vertex in selected)
    ground_level = low + (high - low) * 0.12
    vertices, faces, face_materials = [], [], []
    remap = {}
    for polygon in source_mesh.polygons:
        if polygon.vertices[0] not in selected:
            continue
        face = []
        for source_index in polygon.vertices:
            if source_index not in remap:
                remap[source_index] = len(vertices)
                vertices.append(source_mesh.vertices[source_index].co[:])
            face.append(remap[source_index])
        faces.append(face)
        material_index = 0
        if index <= 1:
            center_height = sum(
                source_mesh.vertices[vertex].co.z for vertex in polygon.vertices
            ) / len(polygon.vertices)
            if center_height <= ground_level:
                material_index = material_names.index("mud_bog")
            elif index == 0 and polygon.vertices[0] in petrified_vertices:
                material_index = material_names.index("petrified")
        face_materials.append(material_index)

    origin = (
        (min(v[0] for v in vertices) + max(v[0] for v in vertices)) / 2.0,
        (min(v[1] for v in vertices) + max(v[1] for v in vertices)) / 2.0,
        min(v[2] for v in vertices),
    )
    vertices = [tuple(v[axis] - origin[axis] for axis in range(3)) for v in vertices]
    data = bpy.data.meshes.new(filename)
    data.from_pydata(vertices, [], faces)
    for polygon, material_index in zip(data.polygons, face_materials):
        polygon.material_index = material_index
    data.update()
    bmesh_data = bmesh.new()
    bmesh_data.from_mesh(data)
    bmesh.ops.recalc_face_normals(bmesh_data, faces=bmesh_data.faces)
    bmesh_data.to_mesh(data)
    bmesh_data.free()

    obj = bpy.data.objects.new(filename, data)
    bpy.context.collection.objects.link(obj)
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    for name in material_names:
        data.materials.append(materials[name])
    apply_texture_uvs(data, material_names)
    bpy.ops.export_scene.gltf(
        filepath=os.path.join(MODEL_DIR, f"{filename}.glb"),
        export_format="GLB",
        export_image_format="AUTO",
        use_selection=True,
        export_apply=True,
    )
    bpy.data.objects.remove(obj, do_unlink=True)
