"""Auto-rig the unrigged Tripo stalker and bake Crawl/Idle/Walk/Run/Jump clips.

Regenerates assets/models/stalker.glb from the raw Tripo export:
  blender -b --python tools/rig_stalker.py -- raw.glb rigged.glb
  npx @gltf-transform/cli@4 resize rigged.glb assets/models/stalker.glb --width 2048 --height 2048

Blender coords: Z up, model front faces -Y, character's left is +X.
"""
import bpy, math, sys
from mathutils import Vector, Quaternion, Matrix

SRC, DST = sys.argv[sys.argv.index('--')+1:][:2]
FPS = 30
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
sc.render.fps = FPS
bpy.ops.import_scene.gltf(filepath=SRC)
mesh = next(o for o in sc.objects if o.type == 'MESH')
for o in list(sc.objects):
    if o is not mesh and o.type == 'EMPTY':
        mesh.parent = None
bpy.context.view_layer.objects.active = mesh
mesh.select_set(True)
bpy.ops.object.parent_clear(type='CLEAR_KEEP_TRANSFORM')
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
for o in list(sc.objects):
    if o is not mesh:
        bpy.data.objects.remove(o)

verts = [mesh.matrix_world @ v.co for v in mesh.data.vertices]
H = max(v.z for v in verts)

def center(zlo, zhi, xlo=-9, xhi=9):
    """Bounding-box center (x, y) of the mesh slice in a height/side band."""
    s = [v for v in verts if zlo*H <= v.z <= zhi*H and xlo*H <= v.x <= xhi*H]
    xs, ys = [v.x for v in s], [v.y for v in s]
    return Vector(((min(xs)+max(xs))/2, (min(ys)+max(ys))/2))

def at(c, z):
    return Vector((c.x, c.y, z*H))

body = center(0.45, 0.80, -0.12, 0.12)
spine = lambda z: Vector((0, body.y, z*H))
# Limb joints: fitted from slices of the mesh on the character's left (+X).
shoulder = at(center(0.74, 0.78, 0.10, 0.20), 0.775)
elbow = at(center(0.61, 0.65, 0.13, 0.30), 0.63)
wrist = at(center(0.47, 0.50, 0.13, 0.30), 0.485)
hand_end = at(center(0.40, 0.44, 0.13, 0.30), 0.41)
hip = at(center(0.40, 0.44, 0.01, 0.15), 0.50)
knee = at(center(0.26, 0.30, 0.0, 0.15), 0.28)
ankle = at(center(0.08, 0.11, 0.0, 0.15), 0.085)
toe = Vector((ankle.x, min(v.y for v in verts if v.z < 0.05*H and v.x > 0), 0.015*H))
print('JOINTS', dict(shoulder=shoulder, elbow=elbow, wrist=wrist, hip=hip, knee=knee, ankle=ankle, toe=toe))

arm_data = bpy.data.armatures.new('Rig')
rig = bpy.data.objects.new('Stalker', arm_data)
sc.collection.objects.link(rig)
bpy.context.view_layer.objects.active = rig
bpy.ops.object.mode_set(mode='EDIT')
eb = arm_data.edit_bones

def bone(name, head, tail, parent=None, connect=False):
    b = eb.new(name); b.head, b.tail = head, tail
    if parent: b.parent = eb[parent]; b.use_connect = connect
    return b

bone('hips', spine(0.50), spine(0.58))
bone('spine', spine(0.58), spine(0.68), 'hips', True)
bone('chest', spine(0.68), spine(0.79), 'spine', True)
bone('neck', spine(0.79), spine(0.84), 'chest', True)
bone('head', spine(0.84), spine(0.99), 'neck', True)
for side, m in (('L', 1), ('R', -1)):
    f = lambda v: Vector((v.x*m, v.y, v.z))
    bone(f'shoulder.{side}', spine(0.775) + Vector((0.03*m, 0, 0)), f(shoulder), 'chest')
    bone(f'upper_arm.{side}', f(shoulder), f(elbow), f'shoulder.{side}', True)
    bone(f'forearm.{side}', f(elbow), f(wrist), f'upper_arm.{side}', True)
    bone(f'hand.{side}', f(wrist), f(hand_end), f'forearm.{side}', True)
    bone(f'thigh.{side}', f(hip), f(knee), 'hips')
    bone(f'shin.{side}', f(knee), f(ankle), f'thigh.{side}', True)
    bone(f'foot.{side}', f(ankle), f(toe), f'shin.{side}', True)
for b in eb:
    b.roll = 0
bpy.ops.object.mode_set(mode='OBJECT')

bpy.ops.object.select_all(action='DESELECT')
mesh.select_set(True); rig.select_set(True)
bpy.context.view_layer.objects.active = rig
# Bone heat needs welded geometry and fails at small scales, so weight at 10x.
bpy.context.view_layer.objects.active = mesh
bpy.ops.object.mode_set(mode='EDIT')
bpy.ops.mesh.select_all(action='SELECT')
bpy.ops.mesh.remove_doubles(threshold=0.0005)
bpy.ops.object.mode_set(mode='OBJECT')
for o in (mesh, rig):
    o.scale = (10, 10, 10)
bpy.ops.object.select_all(action='DESELECT')
for o in (mesh, rig):
    o.select_set(True)
bpy.context.view_layer.objects.active = rig
bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
bpy.ops.object.parent_set(type='ARMATURE_AUTO')
bpy.ops.object.select_all(action='DESELECT')
rig.select_set(True)
bpy.context.view_layer.objects.active = rig
rig.scale = (0.1, 0.1, 0.1)
mesh.select_set(True)
bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
# Copy weights onto the few vertices bone heat missed (Bevy would collapse them).
from mathutils import kdtree
mv = mesh.data.vertices
done = [v for v in mv if any(g.weight > 1e-3 for g in v.groups)]
tree = kdtree.KDTree(len(done))
for i, v in enumerate(done):
    tree.insert(v.co, i)
tree.balance()
for v in mv:
    if not any(g.weight > 1e-3 for g in v.groups):
        src = done[tree.find(v.co)[1]]
        for g in src.groups:
            mesh.vertex_groups[g.group].add([v.index], g.weight, 'REPLACE')
unweighted = sum(1 for v in mesh.data.vertices if not any(g.weight > 1e-3 for g in v.groups))
print('UNWEIGHTED', unweighted, 'of', len(mesh.data.vertices))

# --- Animation -------------------------------------------------------------
bpy.context.view_layer.update()
rest = {b.name: b.matrix_local.to_quaternion() for b in arm_data.bones}

def rot(name, x=0.0, y=0.0, z=0.0):
    """Delta rotation in armature-rest axes (degrees) -> bone-local quaternion.

    +X swings a hanging limb backward (front is -Y), -X forward; it tips an
    upright bone (spine, head) forward, -X back.
    """
    d = Matrix.Rotation(math.radians(z), 3, 'Z') @ Matrix.Rotation(math.radians(y), 3, 'Y') @ Matrix.Rotation(math.radians(x), 3, 'X')
    r = rest[name]
    return r.inverted() @ d.to_quaternion() @ r

def key(action_name, frames, pose_at, loop=True):
    rig.animation_data_create()
    act = bpy.data.actions.new(action_name)
    act.use_fake_user = True
    rig.animation_data.action = act
    for f in range(frames + 1):
        t = (f % frames if loop else f) / frames
        pose = pose_at(t)
        for pb in rig.pose.bones:
            pb.rotation_mode = 'QUATERNION'
            pb.rotation_quaternion = pose.get(pb.name, Quaternion())
            pb.location = Vector()
            pb.keyframe_insert('rotation_quaternion', frame=f)
        hips = rig.pose.bones['hips']
        hips.location = Vector((0, pose.get('_lift', 0.0), 0))  # bone Y is up
        hips.keyframe_insert('location', frame=f)
    rig.animation_data.action = None

TAU = math.tau

def gait(t, stride, knee_max, arm, elbow, lean, bob, twist):
    p = TAU * t
    q = {}
    for side, ph in (('L', 0.0), ('R', math.pi)):
        fwd = math.sin(p + ph)                  # >0: leg ahead of the body
        swing = max(0.0, math.cos(p + ph))      # leg moving forward
        knee = 6 + knee_max * swing ** 1.5
        q[f'thigh.{side}'] = rot(f'thigh.{side}', x=-stride * fwd - 0.3 * knee * swing)
        q[f'shin.{side}'] = rot(f'shin.{side}', x=knee)
        q[f'foot.{side}'] = rot(f'foot.{side}', x=-0.35 * knee + 8 * fwd)
        q[f'upper_arm.{side}'] = rot(f'upper_arm.{side}', x=arm * fwd)  # opposite leg
        q[f'forearm.{side}'] = rot(f'forearm.{side}', x=-elbow - 0.4 * elbow * max(0.0, -fwd))
    q['spine'] = rot('spine', x=lean * 0.5, z=twist * math.sin(p))
    q['chest'] = rot('chest', x=lean * 0.5, z=-1.5 * twist * math.sin(p))
    q['head'] = rot('head', x=-lean * 0.6)
    q['_lift'] = -bob * math.sin(p) ** 2
    return q

key('Walk', 30, lambda t: gait(t, stride=24, knee_max=40, arm=18, elbow=12, lean=3, bob=0.010, twist=4))
key('Run', 20, lambda t: gait(t, stride=40, knee_max=85, arm=40, elbow=75, lean=12, bob=0.022, twist=7))

def idle(t):
    b = math.sin(TAU * t)
    return {
        'chest': rot('chest', x=-1.5 * b),
        'head': rot('head', x=1.0 * b, z=2.0 * math.sin(TAU * t * 0.5 + 1)),
        'upper_arm.L': rot('upper_arm.L', x=1.5 * b),
        'upper_arm.R': rot('upper_arm.R', x=1.5 * b),
        'forearm.L': rot('forearm.L', x=-8), 'forearm.R': rot('forearm.R', x=-8),
        '_lift': -0.002 * (1 + b),
    }
key('Idle', 90, idle)

def jump(t):
    k = min(1.0, t / 0.3)                       # tuck in over the first 30 %
    s = k * k * (3 - 2 * k)
    q = {}
    for side, m in (('L', 1), ('R', -1)):
        lead = 1.0 if side == 'L' else 0.6
        q[f'thigh.{side}'] = rot(f'thigh.{side}', x=-55 * lead * s)
        q[f'shin.{side}'] = rot(f'shin.{side}', x=80 * lead * s)
        q[f'foot.{side}'] = rot(f'foot.{side}', x=-15 * s)
        q[f'upper_arm.{side}'] = rot(f'upper_arm.{side}', x=-50 * s, y=-15 * m * s)
        q[f'forearm.{side}'] = rot(f'forearm.{side}', x=-50 * s)
    q['spine'] = rot('spine', x=8 * s)
    q['head'] = rot('head', x=-6 * s)
    return q
key('Jump', 15, jump, loop=False)

def crawl(t):
    """Commando crawl, posed standing: the game pitches the body face-down.

    "Overhead" is ahead along the ground; each arm reaches while the leg on
    the same side straightens and the opposite knee draws up the side.
    """
    p = TAU * t
    q = {}
    for side, ph, m in (('L', 0.0, 1), ('R', math.pi, -1)):
        reach = math.sin(p + ph)                # 1: arm stretched ahead
        draw = 0.5 * (1 - reach)                # 1: knee drawn up
        q[f'upper_arm.{side}'] = rot(f'upper_arm.{side}', x=-(115 + 35 * reach), y=-m * 20)
        q[f'forearm.{side}'] = rot(f'forearm.{side}', x=-(70 - 45 * reach))
        q[f'thigh.{side}'] = rot(f'thigh.{side}', x=-40 * draw, y=-m * (8 + 30 * draw))
        q[f'shin.{side}'] = rot(f'shin.{side}', x=5 + 80 * draw)
    q['spine'] = rot('spine', x=-4, z=5 * math.sin(p))
    q['chest'] = rot('chest', z=-5 * math.sin(p))
    q['neck'] = rot('neck', x=-20)
    q['head'] = rot('head', x=-35)
    return q
key('Crawl', 40, crawl)

# Rest pose as the default.
for pb in rig.pose.bones:
    pb.rotation_quaternion = Quaternion(); pb.location = Vector()

bpy.ops.export_scene.gltf(
    filepath=DST, export_format='GLB', export_animation_mode='ACTIONS',
    export_image_format='JPEG', export_jpeg_quality=88, export_apply=False,
    export_force_sampling=True, export_optimize_animation_size=False,
    export_anim_slide_to_zero=True,
)
print('EXPORTED', DST)
