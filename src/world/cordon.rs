//! Concrete cordons around the hangar yard and at the edge of the Zone.

use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfMaterial;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};
use crate::player::Player;

use super::collision::{Collider, Pillar};
use super::hydrology::{WATER_LEVEL, Wetlands};
use super::terrain::{BASE, HALF_SIZE, SPACING, TILE_SIZE, Terrain, smoothstep};
use super::{WorldRoot, detail_range};

const YARD_MIN: Vec2 = Vec2::new(BASE.0 - 64.0, BASE.1 - 64.0);
const YARD_MAX: Vec2 = Vec2::new(BASE.0 + 64.0, BASE.1 + 60.0);
const GATE_CENTER: f32 = 39.0;
pub(super) const GATE: Vec2 = Vec2::new(GATE_CENTER, YARD_MAX.y);
const GATE_HALF: f32 = 8.0;
const CHECKPOINT: Vec2 = Vec2::new(GATE_CENTER - 13.6, YARD_MAX.y - 8.2);
const CHECKPOINT_SCALE: f32 = 10.5;
const EDGE: f32 = HALF_SIZE - 4.0;
const WALL_HEIGHT_SCALE: f32 = 7.4;
const PANEL_SPAN: f32 = 0.86;
const SINK: f32 = 0.09;
#[allow(clippy::approx_constant, reason = "Measured model coordinates")]
const CHECKPOINT_PILLARS: &[Pillar] = include!("pillars/site_checkpoint.rs");
const WALL_PILLAR: &[Pillar] = &[Pillar {
    x: 0.0,
    z: 0.0,
    radius: 0.47,
    bottom: -0.65,
    top: 2.48,
}];

const SOUTHWEST: Vec2 = YARD_MIN;
const SOUTHEAST: Vec2 = Vec2::new(YARD_MAX.x, YARD_MIN.y);
const NORTHWEST: Vec2 = Vec2::new(YARD_MIN.x, YARD_MAX.y);
const NORTHEAST: Vec2 = YARD_MAX;
const WEST_JAMB: Vec2 = Vec2::new(GATE_CENTER - GATE_HALF, YARD_MAX.y);
const EAST_JAMB: Vec2 = Vec2::new(GATE_CENTER + GATE_HALF, YARD_MAX.y);
const YARD_RUNS: [(Vec2, Vec2); 5] = [
    (SOUTHWEST, SOUTHEAST),
    (SOUTHEAST, NORTHEAST),
    (NORTHEAST, EAST_JAMB),
    (WEST_JAMB, NORTHWEST),
    (NORTHWEST, SOUTHWEST),
];

pub(super) fn inside_yard(at: Vec2) -> bool {
    at.cmpge(YARD_MIN).all() && at.cmple(YARD_MAX).all()
}

pub(super) fn yard_grade(at: Vec2) -> f32 {
    let min = Vec2::new(BASE.0 - 46.0, BASE.1 - 40.0);
    let max = Vec2::new(BASE.0 + 46.0, YARD_MAX.y - 2.0);
    let edge = (min - at).max(at - max).max_element();
    smoothstep(16.0, 0.0, edge)
}

pub(super) fn reserve_scenery(at: Vec2, clearance: f32) -> bool {
    let margin = Vec2::splat(clearance);
    (at.cmpge(YARD_MIN - margin).all() && at.cmple(YARD_MAX + margin).all())
        || (at.abs().max_element() - EDGE).abs() < clearance
}

pub(super) fn near_wall_or_checkpoint(at: Vec2, clearance: f32) -> bool {
    if (at - Vec2::from(BASE)).abs().max_element() > 84.0 + clearance {
        return false;
    }
    let checkpoint = (at - CHECKPOINT).abs() - Vec2::new(5.25, 4.87);
    checkpoint.max(Vec2::ZERO).length_squared() < clearance * clearance
        || YARD_RUNS.into_iter().any(|(start, end)| {
            let line = end - start;
            let fraction = ((at - start).dot(line) / line.length_squared()).clamp(0.0, 1.0);
            at.distance_squared(start + line * fraction) < (0.65 + clearance).powi(2)
        })
}

pub(super) fn shelterbelt(at: Vec2) -> f32 {
    let offset = at - Vec2::from(BASE);
    let side = offset.x.abs();
    if side < 103.0 && (-113.0..-44.0).contains(&offset.y) {
        1.0
    } else if (50.0..98.0).contains(&side) && (-58.0..47.0).contains(&offset.y) {
        0.68
    } else {
        0.0
    }
}

pub(super) fn preload_cordon(server: Res<AssetServer>, mut preloaded: ResMut<PreloadedAssets>) {
    let (mesh, paint) = assets::load_single_primitive(&server, assets::SITE_CORDON_WALL);
    preloaded.0.extend([mesh.untyped(), paint.untyped()]);
    preloaded.0.push(
        server
            .load::<WorldAsset>(GltfAssetLabel::Scene(0).from_asset(assets::SITE_CHECKPOINT))
            .untyped(),
    );
}

fn checkpoint_pose(terrain: &Terrain) -> Transform {
    Transform::from_xyz(
        CHECKPOINT.x,
        terrain.mesh_height_at(CHECKPOINT) - 0.035,
        CHECKPOINT.y,
    )
    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
    .with_scale(Vec3::splat(CHECKPOINT_SCALE))
}

pub(super) fn prepare_cordon(
    mut commands: Commands,
    server: Res<AssetServer>,
    terrain: Res<Terrain>,
    meshes: Res<Assets<Mesh>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (mesh, paint) = assets::load_single_primitive(&server, assets::SITE_CORDON_WALL);
    let mesh = meshes.get(&mesh).expect("preloaded cordon mesh");
    let paint = gltf_materials
        .get(&paint)
        .expect("preloaded cordon material");
    let root = commands
        .spawn((WorldRoot, Transform::IDENTITY, Visibility::default()))
        .id();
    commands.spawn((
        ChildOf(root),
        Name::new("Hangar checkpoint"),
        WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset(assets::SITE_CHECKPOINT))),
        checkpoint_pose(&terrain),
        Collider(CHECKPOINT_PILLARS),
    ));
    commands.insert_resource(Cordon {
        root,
        chunks: wall_chunks(),
        active: HashMap::default(),
        center: None,
        wall: WallModel::new(mesh),
        paint: materials.add(standard_material_from_gltf_material(paint)),
    });
}

#[derive(Clone, Copy, Debug)]
struct Panel {
    center: Vec2,
    along: Vec2,
    length: f32,
}

impl Panel {
    fn across(self) -> Vec2 {
        self.along.perp()
    }

    fn pillars(self) -> impl Iterator<Item = Vec2> {
        let count = (self.length / 0.77).ceil() as u32;
        (0..count).map(move |i| {
            self.center + self.along * self.length * ((i as f32 + 0.5) / count as f32 - 0.5)
        })
    }
}

fn tile_at(point: Vec2) -> IVec2 {
    (point / TILE_SIZE).floor().as_ivec2()
}

fn wall_chunks() -> HashMap<IVec2, Vec<Panel>> {
    let a = Vec2::splat(-EDGE);
    let b = Vec2::new(EDGE, -EDGE);
    let c = Vec2::splat(EDGE);
    let d = Vec2::new(-EDGE, EDGE);
    let mut chunks: HashMap<_, Vec<_>> = HashMap::default();
    for (from, to) in YARD_RUNS
        .into_iter()
        .chain([(a, b), (b, c), (c, d), (d, a)])
    {
        let span = from.distance(to);
        let along = (to - from) / span;
        let count = (span / 7.4).round().max(1.0) as u32;
        let length = span / count as f32;
        for i in 0..count {
            let center = from + (i as f32 + 0.5) * length * along;
            chunks.entry(tile_at(center)).or_default().push(Panel {
                center,
                along,
                length,
            });
        }
    }
    chunks
}

#[derive(Resource)]
pub(super) struct Cordon {
    root: Entity,
    chunks: HashMap<IVec2, Vec<Panel>>,
    active: HashMap<IVec2, Entity>,
    center: Option<IVec2>,
    wall: WallModel,
    paint: Handle<StandardMaterial>,
}

pub(super) fn stream_cordon(
    mut commands: Commands,
    player: Single<&Transform, With<Player>>,
    terrain: Res<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cordon: ResMut<Cordon>,
) {
    let center = tile_at(player.translation.xz());
    if cordon.center == Some(center) {
        return;
    }
    cordon.center = Some(center);
    cordon.active.retain(|tile, entity| {
        let keep = (*tile - center).abs().max_element() <= 2;
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for z in -2..=2 {
        for x in -2..=2 {
            let tile = center + IVec2::new(x, z);
            if cordon.active.contains_key(&tile) {
                continue;
            }
            if let Some(panels) = cordon.chunks.get(&tile) {
                let entity =
                    spawn_wall_chunk(&mut commands, &mut meshes, &terrain, &cordon, tile, panels);
                cordon.active.insert(tile, entity);
            }
        }
    }
}

fn spawn_wall_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    terrain: &Terrain,
    cordon: &Cordon,
    tile: IVec2,
    panels: &[Panel],
) -> Entity {
    let origin = tile.as_vec2() * TILE_SIZE;
    let mesh = meshes.add(cordon.wall.batch(terrain, origin, panels));
    let mut root = commands.spawn((
        ChildOf(cordon.root),
        Name::new("Concrete cordon"),
        Transform::from_xyz(origin.x, 0.0, origin.y),
        Mesh3d(mesh),
        MeshMaterial3d(cordon.paint.clone()),
        detail_range(205.0, 25.0),
    ));
    if panels[0].center.abs().max_element() >= EDGE {
        root.insert(NotShadowCaster);
    }
    root.with_children(|wall| {
        for at in panels.iter().flat_map(|panel| panel.pillars()) {
            wall.spawn((Collider(WALL_PILLAR), wall_pillar_pose(terrain, at, origin)));
        }
    })
    .id()
}

fn wall_pillar_pose(terrain: &Terrain, at: Vec2, origin: Vec2) -> Transform {
    let floor = floor_point(terrain, at);
    Transform::from_xyz(at.x - origin.x, floor.x + floor.y, at.y - origin.y)
}

struct WallModel {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl WallModel {
    fn new(mesh: &Mesh) -> Self {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            unreachable!("the wall GLB has float positions");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            unreachable!("the wall GLB has float normals");
        };
        let Some(VertexAttributeValues::Float32x2(uv)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            unreachable!("the wall GLB has float UVs");
        };
        Self {
            positions: positions.clone(),
            normals: normals.clone(),
            uv: uv.clone(),
            indices: mesh
                .indices()
                .into_iter()
                .flat_map(Indices::iter)
                .map(|i| i as u32)
                .collect(),
        }
    }

    fn batch(&self, terrain: &Terrain, origin: Vec2, panels: &[Panel]) -> Mesh {
        let mut positions = Vec::with_capacity(self.positions.len() * panels.len());
        let mut normals = Vec::with_capacity(positions.capacity());
        let mut uv = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(self.indices.len() * panels.len());
        for &panel in panels {
            let scale_x = panel.length / PANEL_SPAN;
            let extents = panel.along.abs() * (0.51 * scale_x) + panel.across().abs() * 0.70;
            let floor = GroundPatch::new(terrain, panel.center - extents, panel.center + extents);
            let offset = positions.len() as u32;
            for (&[x, y, z], &[nx, ny, nz]) in self.positions.iter().zip(&self.normals) {
                let at = panel.center
                    + panel.along * (x * scale_x)
                    + panel.across() * (z * WALL_HEIGHT_SCALE);
                let base = floor.sample(at);
                // Continue over lakes and rivers on a concrete foundation, not a floating row of panels.
                let stretch = (y / 0.15).min(1.0);
                let height = y * WALL_HEIGHT_SCALE + base.value.x + stretch * base.value.y - SINK;
                let horizontal =
                    panel.along * (nx / scale_x) + panel.across() * (nz / WALL_HEIGHT_SCALE);
                let vertical = ny
                    / (WALL_HEIGHT_SCALE
                        + if stretch < 1.0 {
                            base.value.y / 0.15
                        } else {
                            0.0
                        });
                let slope = Vec2::new(
                    base.dx.x + stretch * base.dx.y,
                    base.dz.x + stretch * base.dz.y,
                );
                let normal = horizontal - slope * vertical;
                positions.push([at.x - origin.x, height, at.y - origin.y]);
                normals.push(
                    Vec3::new(normal.x, vertical, normal.y)
                        .normalize()
                        .to_array(),
                );
            }
            uv.extend_from_slice(&self.uv);
            indices.extend(self.indices.iter().map(|&index| index + offset));
        }
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(indices))
    }
}

fn floor_point(terrain: &Terrain, at: Vec2) -> Vec2 {
    let ground = terrain.mesh_height_at(at);
    let rise = if Wetlands::at(at.x, at.y).depth(ground) > 0.02 {
        (WATER_LEVEL - 0.25 - ground).max(0.0)
    } else {
        0.0
    };
    Vec2::new(ground, rise)
}

struct GroundPatch {
    corner: Vec2,
    width: usize,
    vertices: Vec<Vec2>,
}

struct GroundSample {
    value: Vec2,
    dx: Vec2,
    dz: Vec2,
}

impl GroundPatch {
    fn new(terrain: &Terrain, low: Vec2, high: Vec2) -> Self {
        let low = (low / SPACING).floor().as_ivec2();
        let high = (high / SPACING).floor().as_ivec2() + IVec2::ONE;
        let width = (high.x - low.x + 1) as usize;
        let vertices = (low.y..=high.y)
            .flat_map(|z| {
                (low.x..=high.x)
                    .map(move |x| floor_point(terrain, Vec2::new(x as f32, z as f32) * SPACING))
            })
            .collect();
        Self {
            corner: low.as_vec2(),
            width,
            vertices,
        }
    }

    fn sample(&self, at: Vec2) -> GroundSample {
        let grid = at / SPACING - self.corner;
        let cell = grid.floor().as_uvec2();
        let uv = grid - cell.as_vec2();
        let i = cell.y as usize * self.width + cell.x as usize;
        let (b, c) = (self.vertices[i + 1], self.vertices[i + self.width]);
        let (value, dx, dz) = if uv.x + uv.y <= 1.0 {
            let a = self.vertices[i];
            (a + (b - a) * uv.x + (c - a) * uv.y, b - a, c - a)
        } else {
            let d = self.vertices[i + self.width + 1];
            (
                d + (c - d) * (1.0 - uv.x) + (b - d) * (1.0 - uv.y),
                d - c,
                d - b,
            )
        };
        GroundSample {
            value,
            dx: dx / SPACING,
            dz: dz / SPACING,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::player::vehicle::{Driving, Vehicle, blocked, drive_vehicle};
    use crate::world::{Body, push_out};

    fn solids_near(
        terrain: &Terrain,
        origin: Vec2,
        radius: f32,
    ) -> Vec<(Collider, GlobalTransform)> {
        wall_chunks()
            .into_values()
            .flatten()
            .filter(|panel| panel.center.distance(origin) < radius + panel.length)
            .flat_map(Panel::pillars)
            .map(|at| {
                (
                    Collider(WALL_PILLAR),
                    wall_pillar_pose(terrain, at, Vec2::ZERO).into(),
                )
            })
            .collect()
    }

    #[test]
    fn the_checkpoint_leaves_a_level_lane_both_way_for_the_stalker_and_vehicle() {
        let terrain = Terrain::new();
        let gate = Vec2::new(GATE_CENTER, YARD_MAX.y);
        let mut solids = solids_near(&terrain, gate, 30.0);
        solids.push((
            Collider(CHECKPOINT_PILLARS),
            checkpoint_pose(&terrain).into(),
        ));
        let obstacles = || solids.iter().map(|(solid, pose)| (solid, pose));
        let body = Body {
            radius: 0.35,
            height: 1.8,
        };
        let mut prior = None;
        for i in -165..=165 {
            let z = gate.y + i as f32 * 0.1;
            let y = terrain.mesh_height_at(Vec2::new(gate.x, z));
            if let Some(prior) = prior {
                let dy: f32 = y - prior;
                assert!(dy.abs() < 0.045, "sharp approach step at {z}");
            }
            prior = Some(y);
            for dx in [-2.2, 0.0, 2.2] {
                let at = Vec2::new(gate.x + dx, z);
                assert_eq!(push_out(at, y, body, obstacles()), at);
                assert!(!blocked(
                    &Transform::from_xyz(at.x, y + 0.03, at.y),
                    obstacles()
                ));
            }
        }
        let bunker = checkpoint_pose(&terrain).transform_point(Vec3::new(0.06, 0.0, 0.31));
        assert!(blocked(&Transform::from_translation(bunker), obstacles()));
        let yard = Vec2::from(BASE);
        for x in [-42.0, 42.0] {
            for z in [-38.0, 48.0] {
                let at = yard + Vec2::new(x, z);
                assert!(inside_yard(at));
                assert!((terrain.mesh_height_at(at) - terrain.mesh_height_at(yard)).abs() < 0.05);
                assert_eq!(shelterbelt(at), 0.0);
            }
        }
    }

    #[test]
    fn the_armored_vehicle_drives_from_the_yard_through_the_checkpoint() {
        let terrain = Terrain::new();
        let gate = Vec2::new(GATE_CENTER, YARD_MAX.y);
        let checkpoint = checkpoint_pose(&terrain);
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_millis(16));
        app.insert_resource(time)
            .insert_resource(terrain.clone())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .add_systems(Update, drive_vehicle);
        let world = app.world_mut();
        for (collider, pose) in solids_near(&terrain, gate, 52.0) {
            world.spawn((collider, pose));
        }
        world.spawn((
            Collider(CHECKPOINT_PILLARS),
            GlobalTransform::from(checkpoint),
        ));
        let start = gate - Vec2::Y * 21.0;
        let pose = Transform::from_xyz(start.x, terrain.mesh_height_at(start) + 0.035, start.y);
        let car = world.spawn((Vehicle::default(), pose)).id();
        world.spawn((Player::default(), Driving(car), pose));
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        for _ in 0..310 {
            app.update();
        }
        let end = app.world().get::<Transform>(car).unwrap().translation;
        assert!(end.z > gate.y + 10.0, "vehicle stopped at {end}");
        assert!(
            (end.x - gate.x).abs() < 1.0,
            "vehicle drifted out of the lane: {end}"
        );
    }

    #[test]
    fn the_outer_cordon_is_continuous_at_seams_and_corners_before_the_map_limit() {
        let terrain = Terrain::new();
        let chunks = wall_chunks();
        for (&tile, panels) in &chunks {
            assert!(panels.iter().all(|panel| tile_at(panel.center) == tile));
        }
        let panels = chunks.values().flatten().copied().collect::<Vec<_>>();
        for across in [Vec2::Y, Vec2::X, Vec2::NEG_Y, Vec2::NEG_X] {
            let along = across.perp();
            let mut disks = panels
                .iter()
                .flat_map(|panel| panel.pillars())
                .filter(|at| at.dot(across) == EDGE)
                .map(|at| at.dot(along))
                .collect::<Vec<_>>();
            disks.sort_unstable_by(f32::total_cmp);
            assert!(disks[0] + EDGE < 0.40);
            assert!(EDGE - disks.last().copied().unwrap() < 0.40);
            assert!(disks.windows(2).all(|pair| pair[1] - pair[0] < 0.78));
        }
        let corner = Vec2::splat(EDGE) - Vec2::splat(0.12);
        let pillars = solids_near(&terrain, corner, 10.0);
        let y = terrain.mesh_height_at(corner);
        let obstacles = pillars.iter().map(|(solid, pose)| (solid, pose));
        assert!(blocked(
            &Transform::from_xyz(corner.x, y + 0.04, corner.y),
            obstacles
        ));
    }

    #[test]
    fn walls_and_their_colliders_stream_together_between_the_yard_and_frontier() {
        let mut app = App::new();
        app.insert_resource(Terrain::new())
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, stream_cordon);
        let world = app.world_mut();
        let root = world.spawn(Transform::IDENTITY).id();
        let player = world
            .spawn((Player::default(), Transform::from_xyz(BASE.0, 0.0, BASE.1)))
            .id();
        world.insert_resource(Cordon {
            root,
            chunks: wall_chunks(),
            active: HashMap::default(),
            center: None,
            wall: WallModel {
                positions: vec![[-0.4, 0.0, 0.0], [0.4, 0.0, 0.0], [0.0, 0.3, 0.0]],
                normals: vec![[0.0, 0.0, 1.0]; 3],
                uv: vec![[0.0, 0.0]; 3],
                indices: vec![0, 1, 2],
            },
            paint: Handle::default(),
        });
        app.update();
        let original = app.world().resource::<Cordon>().active.clone();
        assert!(!original.is_empty());
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .x = EDGE - 5.0;
        app.update();
        let cordon = app.world().resource::<Cordon>();
        assert!(!cordon.active.is_empty());
        assert!(
            cordon
                .active
                .keys()
                .all(|tile| !original.contains_key(tile))
        );
        assert!(
            original
                .values()
                .all(|&entity| app.world().get_entity(entity).is_err())
        );
        for &entity in cordon.active.values() {
            assert!(app.world().get::<Mesh3d>(entity).is_some());
            let children = app.world().get::<Children>(entity).unwrap();
            assert!(
                children
                    .iter()
                    .all(|child| app.world().get::<Collider>(child).is_some())
            );
        }
    }

    #[test]
    fn wall_profiles_track_the_rendered_floor_and_rise_above_deep_crossings() {
        let terrain = Terrain::new();
        let river = (175..480)
            .map(|z| Vec2::new(-EDGE, z as f32))
            .max_by(|&a, &b| {
                floor_point(&terrain, a)
                    .y
                    .total_cmp(&floor_point(&terrain, b).y)
            })
            .expect("a sample of the west cordon");
        assert!(floor_point(&terrain, river).y > 3.5);
        let dry = Vec2::new(EDGE, 1047.5);
        for at in [river, dry, Vec2::new(-21.0, -192.5)] {
            let patch = GroundPatch::new(&terrain, at - Vec2::splat(5.1), at + Vec2::splat(5.1));
            for sample in [at, at - Vec2::new(2.6, 0.81), at + Vec2::new(2.75, 0.92)] {
                let base = patch.sample(sample);
                assert!((base.value.x - terrain.mesh_height_at(sample)).abs() < 0.002);
                if at == river {
                    assert!(base.value.x < WATER_LEVEL - 2.0);
                    assert!(base.value.x + base.value.y + 2.48 > WATER_LEVEL + 2.0);
                }
            }
        }
    }
}
