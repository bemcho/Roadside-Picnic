//! The stalkers' base: an old industrial hangar on a levelled pad at the
//! south edge of the Zone. A concrete hall of corrugated walls, steel
//! columns, roof trusses and an overhead crane shelters a yard of catwalks,
//! crates and machinery. Runs start inside, by the door that opens north
//! into the Zone, and the Zone sends dead stalkers back here.

use std::f32::consts::FRAC_PI_2;
use std::sync::LazyLock;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::assets;

use super::WorldRoot;
use super::collision::{Collider, Pillar};
use super::terrain::{BASE, Terrain};
use super::terrain_material::load_repeating;

/// Hall footprint and height in meters, around [`BASE`]. The door is in the
/// north (+Z) wall.
pub(super) const HALF_WIDTH: f32 = 17.0;
pub(super) const HALF_DEPTH: f32 = 19.0;
const HEIGHT: f32 = 13.0;
const WALL_THICKNESS: f32 = 0.3;
const DOOR_HALF_WIDTH: f32 = 6.0;
const DOOR_HEIGHT: f32 = 9.0;

/// The floor sits a hair above the levelled pad so no terrain shows through.
const FLOOR_LIFT: f32 = 0.02;
const SLAB_DEPTH: f32 = 0.6;

/// Bays between the columns and roof trusses, along the hall.
const BAY: f32 = 6.0;
const CRANE_HEIGHT: f32 = 9.5;

/// Meters per yard model unit: a ~20 m yard with walkways ~5 m up, set at
/// the back of the hall so the front stays clear around the door.
const YARD_SCALE: f32 = 20.0;
const YARD_OFFSET: f32 = -8.5;

/// The yard's collision, measured by tools/trim_tree_base.py.
const YARD: &[Pillar] = include!("pillars/warehouse.rs");

/// The hall walls as a fence of overlapping posts, open at the door.
static WALLS: LazyLock<Vec<Pillar>> = LazyLock::new(|| {
    const SPACING: f32 = 0.5;
    let post = |x, z| Pillar {
        x,
        z,
        radius: 0.4,
        bottom: -1.0,
        top: HEIGHT,
    };
    let steps = |from: f32, to: f32| {
        let count = ((to - from) / SPACING).round() as usize;
        (0..=count).map(move |i| from + (to - from) * i as f32 / count as f32)
    };
    let sides =
        steps(-HALF_DEPTH, HALF_DEPTH).flat_map(|z| [post(-HALF_WIDTH, z), post(HALF_WIDTH, z)]);
    let back = steps(-HALF_WIDTH, HALF_WIDTH).map(|x| post(x, -HALF_DEPTH));
    let front = steps(DOOR_HALF_WIDTH, HALF_WIDTH)
        .flat_map(|x| [post(-x, HALF_DEPTH), post(x, HALF_DEPTH)]);
    sides.chain(back).chain(front).collect()
});

/// The hall's surfaces.
struct Paints {
    concrete: Handle<StandardMaterial>,
    corrugated: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    yellow: Handle<StandardMaterial>,
    hazard: Handle<StandardMaterial>,
    grate: Handle<StandardMaterial>,
    skylight: Handle<StandardMaterial>,
    bulb: Handle<StandardMaterial>,
}

impl Paints {
    fn new(
        server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
        images: &mut Assets<Image>,
    ) -> Self {
        let scanned = |albedo, normal, arm, tint: Color, roughness, metallic| {
            let arm = load_repeating(server, arm, false);
            StandardMaterial {
                base_color: tint,
                base_color_texture: Some(load_repeating(server, albedo, true)),
                normal_map_texture: Some(load_repeating(server, normal, false)),
                metallic_roughness_texture: Some(arm.clone()),
                occlusion_texture: Some(arm),
                perceptual_roughness: roughness,
                metallic,
                ..default()
            }
        };
        let painted = |color: Color, roughness, metallic| StandardMaterial {
            base_color: color,
            perceptual_roughness: roughness,
            metallic,
            ..default()
        };
        let glowing = |nits: f32| StandardMaterial {
            base_color: Color::WHITE,
            emissive: LinearRgba::rgb(1.0, 0.95, 0.85) * nits,
            ..default()
        };
        Self {
            // Polished, worn slab: the scan's roughness, a little smoother.
            concrete: materials.add(scanned(
                assets::BASE_CONCRETE_ALBEDO,
                assets::BASE_CONCRETE_NORMAL,
                assets::BASE_CONCRETE_ARM,
                Color::srgb(0.85, 0.85, 0.82),
                0.7,
                0.0,
            )),
            // Pale blue-grey painted sheeting.
            corrugated: materials.add(scanned(
                assets::BASE_CORRUGATED_ALBEDO,
                assets::BASE_CORRUGATED_NORMAL,
                assets::BASE_CORRUGATED_ARM,
                Color::srgb(1.6, 1.75, 1.9),
                0.8,
                0.15,
            )),
            steel: materials.add(painted(Color::srgb(0.16, 0.17, 0.18), 0.5, 0.8)),
            yellow: materials.add(painted(Color::srgb(0.9, 0.62, 0.04), 0.55, 0.1)),
            hazard: materials.add(StandardMaterial {
                base_color_texture: Some(images.add(hazard_stripes())),
                perceptual_roughness: 0.6,
                ..default()
            }),
            grate: materials.add(painted(Color::srgb(0.04, 0.04, 0.04), 0.4, 0.9)),
            skylight: materials.add(glowing(6_000.0)),
            bulb: materials.add(glowing(60_000.0)),
        }
    }
}

/// Spawns parts of the hall as children of its root.
struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    hall: Entity,
}

impl Builder<'_, '_, '_> {
    fn add(&mut self, mesh: Mesh, material: &Handle<StandardMaterial>, transform: Transform) {
        self.commands.spawn((
            Mesh3d(self.meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            transform,
            ChildOf(self.hall),
        ));
    }

    /// A box of `size` centered at `center`, its texture tiled every `tile` m.
    fn block(&mut self, size: Vec3, tile: f32, material: &Handle<StandardMaterial>, center: Vec3) {
        self.add(
            tiled_box(size, tile),
            material,
            Transform::from_translation(center),
        );
    }
}

pub(super) fn spawn_base(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let floor = terrain.height_at(BASE.0, BASE.1) + FLOOR_LIFT;
    let hall = commands
        .spawn((
            WorldRoot,
            Transform::from_xyz(BASE.0, floor, BASE.1),
            Visibility::default(),
            Collider(WALLS.as_slice()),
        ))
        .id();
    // The yard's open side (model +Z) faces the door.
    commands.spawn((
        WorldAssetRoot(asset_server.load(format!("{}#Scene0", assets::BASE_YARD))),
        Transform::from_xyz(0.0, 0.0, YARD_OFFSET).with_scale(Vec3::splat(YARD_SCALE)),
        Collider(YARD),
        ChildOf(hall),
    ));

    let paints = Paints::new(&asset_server, &mut materials, &mut images);
    let mut builder = Builder {
        commands: &mut commands,
        meshes: &mut meshes,
        hall,
    };
    spawn_shell(&mut builder, &paints);
    spawn_frame(&mut builder, &paints);
    spawn_crane(&mut builder, &paints);
    spawn_lamps(&mut builder, &paints);
    spawn_markings(&mut builder, &paints);
}

/// Floor slab and apron, walls with the door opening, and the roof.
fn spawn_shell(b: &mut Builder, paints: &Paints) {
    let (w, d) = (HALF_WIDTH, HALF_DEPTH);
    let slab = Vec3::new(2.0 * w + 1.0, SLAB_DEPTH, 2.0 * d + 1.0);
    b.block(slab, 3.0, &paints.concrete, Vec3::Y * -SLAB_DEPTH / 2.0);
    let apron = Vec3::new(2.0 * DOOR_HALF_WIDTH + 4.0, SLAB_DEPTH, 6.0);
    b.block(
        apron,
        3.0,
        &paints.concrete,
        Vec3::new(0.0, -SLAB_DEPTH / 2.0, d + 3.0),
    );

    // Walls reach a meter below the floor, so no gap shows at the pad's edge.
    let tall = HEIGHT + 1.0;
    let mid = (HEIGHT - 1.0) / 2.0;
    let sheet = &paints.corrugated;
    for side in [-1.0, 1.0] {
        b.block(
            Vec3::new(WALL_THICKNESS, tall, 2.0 * d),
            2.7,
            sheet,
            Vec3::new(side * w, mid, 0.0),
        );
        let jamb = w - DOOR_HALF_WIDTH;
        let at = Vec3::new(side * (DOOR_HALF_WIDTH + jamb / 2.0), mid, d);
        b.block(Vec3::new(jamb, tall, WALL_THICKNESS), 2.7, sheet, at);
    }
    b.block(
        Vec3::new(2.0 * w, tall, WALL_THICKNESS),
        2.7,
        sheet,
        Vec3::new(0.0, mid, -d),
    );
    let lintel = Vec3::new(2.0 * DOOR_HALF_WIDTH, HEIGHT - DOOR_HEIGHT, WALL_THICKNESS);
    b.block(
        lintel,
        2.7,
        sheet,
        Vec3::new(0.0, (HEIGHT + DOOR_HEIGHT) / 2.0, d),
    );

    let roof = Vec3::new(2.0 * w + 1.0, 0.3, 2.0 * d + 1.0);
    b.block(roof, 2.7, sheet, Vec3::Y * (HEIGHT + 0.15));
    // Skylights: glowing panes along the hall, just under the roof.
    for x in [-9.0, 0.0, 9.0] {
        let pane = Vec3::new(2.4, 0.05, 2.0 * d - 4.0);
        b.block(
            pane,
            1.0,
            &paints.skylight,
            Vec3::new(x, HEIGHT - 0.03, 0.0),
        );
    }
}

/// Steel columns with hazard-striped feet, roof trusses, and the door frame
/// with its roll-up drum.
fn spawn_frame(b: &mut Builder, paints: &Paints) {
    let column_x = HALF_WIDTH - 0.45;
    let chord = HEIGHT - 1.6;
    let mut z = -HALF_DEPTH + 1.0;
    while z <= HALF_DEPTH - 1.0 {
        for side in [-1.0, 1.0] {
            let x = side * column_x;
            b.block(
                Vec3::new(0.5, HEIGHT, 0.5),
                1.0,
                &paints.steel,
                Vec3::new(x, HEIGHT / 2.0, z),
            );
            b.block(
                Vec3::splat(0.56).with_y(1.6),
                0.6,
                &paints.hazard,
                Vec3::new(x, 0.8, z),
            );
        }
        spawn_truss(b, paints, z, column_x, chord);
        z += BAY;
    }

    let jamb_x = DOOR_HALF_WIDTH + 0.25;
    for side in [-1.0, 1.0] {
        let at = Vec3::new(side * jamb_x, DOOR_HEIGHT / 2.0, HALF_DEPTH - 0.4);
        b.block(Vec3::new(0.5, DOOR_HEIGHT, 0.5), 0.6, &paints.hazard, at);
    }
    let drum = Cylinder::new(0.55, 2.0 * DOOR_HALF_WIDTH + 1.0);
    let at = Vec3::new(0.0, DOOR_HEIGHT + 0.6, HALF_DEPTH - 0.8);
    b.add(
        drum.into(),
        &paints.steel,
        Transform::from_translation(at).with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
    );
}

/// A roof truss across the hall at `z`: two chords braced in a zigzag.
fn spawn_truss(b: &mut Builder, paints: &Paints, z: f32, half_span: f32, chord: f32) {
    let rise = HEIGHT - 0.25 - chord;
    let span = 2.0 * half_span;
    b.block(
        Vec3::new(span, 0.3, 0.25),
        1.0,
        &paints.steel,
        Vec3::new(0.0, chord, z),
    );
    b.block(
        Vec3::new(span, 0.3, 0.25),
        1.0,
        &paints.steel,
        Vec3::new(0.0, chord + rise, z),
    );
    let panels = 10;
    let panel = span / panels as f32;
    let brace = Vec2::new(panel, rise);
    for i in 0..panels {
        let x = -half_span + panel * (i as f32 + 0.5);
        let tilt = if i % 2 == 0 { 1.0 } else { -1.0 } * brace.to_angle();
        let at = Transform::from_xyz(x, chord + rise / 2.0, z)
            .with_rotation(Quat::from_rotation_z(tilt));
        b.add(
            Cuboid::new(brace.length(), 0.12, 0.12).into(),
            &paints.steel,
            at,
        );
    }
}

/// The overhead gantry crane: runways along the walls, a yellow bridge
/// across the hall, a trolley, and a hook hanging over the floor.
fn spawn_crane(b: &mut Builder, paints: &Paints) {
    let rail_x = HALF_WIDTH - 0.9;
    let bridge_z = 7.0;
    let yellow = &paints.yellow;
    for side in [-1.0, 1.0] {
        let rail = Vec3::new(0.45, 0.7, 2.0 * HALF_DEPTH - 1.0);
        b.block(
            rail,
            1.0,
            &paints.steel,
            Vec3::new(side * rail_x, CRANE_HEIGHT, 0.0),
        );
        let truck = Vec3::new(0.8, 0.6, 2.4);
        b.block(
            truck,
            1.0,
            yellow,
            Vec3::new(side * rail_x, CRANE_HEIGHT + 0.65, bridge_z),
        );
    }
    let bridge = Vec3::new(2.0 * rail_x, 0.9, 0.8);
    b.block(
        bridge,
        1.0,
        yellow,
        Vec3::new(0.0, CRANE_HEIGHT + 0.8, bridge_z),
    );
    let trolley_x = 3.0;
    let trolley = Vec3::new(1.6, 0.7, 1.4);
    b.block(
        trolley,
        1.0,
        yellow,
        Vec3::new(trolley_x, CRANE_HEIGHT + 1.6, bridge_z),
    );

    let hook_y = 3.4;
    let cable_top = CRANE_HEIGHT + 0.35;
    let cable = Cylinder::new(0.03, cable_top - hook_y);
    let at = Transform::from_xyz(trolley_x, (cable_top + hook_y) / 2.0, bridge_z);
    b.add(cable.into(), &paints.steel, at);
    b.block(
        Vec3::new(0.5, 0.7, 0.3),
        1.0,
        yellow,
        Vec3::new(trolley_x, hook_y, bridge_z),
    );
    let hook = Transform::from_xyz(trolley_x, hook_y - 0.6, bridge_z)
        .with_rotation(Quat::from_rotation_x(FRAC_PI_2));
    b.add(Torus::new(0.12, 0.2).into(), &paints.steel, hook);
}

/// Pendant lamps hanging from the trusses, each lighting a pool of floor.
fn spawn_lamps(b: &mut Builder, paints: &Paints) {
    let chord = HEIGHT - 1.6;
    let lamp_y = 8.4;
    for x in [-9.0, 0.0, 9.0] {
        for z in [-11.0, 1.0, 13.0] {
            let cord = Cylinder::new(0.02, chord - lamp_y);
            b.add(
                cord.into(),
                &paints.steel,
                Transform::from_xyz(x, (chord + lamp_y) / 2.0, z),
            );
            b.add(
                Cone::new(0.5, 0.4).into(),
                &paints.steel,
                Transform::from_xyz(x, lamp_y, z),
            );
            let bulb = Cylinder::new(0.4, 0.02);
            b.add(
                bulb.into(),
                &paints.bulb,
                Transform::from_xyz(x, lamp_y - 0.2, z),
            );
            let light = Vec3::new(x, lamp_y - 0.25, z);
            b.commands.spawn((
                SpotLight {
                    color: Color::srgb(1.0, 0.93, 0.8),
                    intensity: 3.0e7,
                    range: 30.0,
                    radius: 0.4,
                    outer_angle: 1.0,
                    inner_angle: 0.6,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(light).looking_at(light - Vec3::Y, Vec3::Z),
                ChildOf(b.hall),
            ));
        }
    }
}

/// Yellow lane lines, a hazard band across the door, and drain grates.
fn spawn_markings(b: &mut Builder, paints: &Paints) {
    let paint_y = 0.005;
    let lane_start = YARD_OFFSET + 10.5;
    let lane = HALF_DEPTH - 1.0 - lane_start;
    for x in [-DOOR_HALF_WIDTH + 0.5, DOOR_HALF_WIDTH - 0.5] {
        let at = Vec3::new(x, paint_y, lane_start + lane / 2.0);
        b.block(Vec3::new(0.15, 0.01, lane), 1.0, &paints.yellow, at);
    }
    let across = Vec3::new(2.0 * HALF_WIDTH - 2.0, 0.01, 0.15);
    b.block(
        across,
        1.0,
        &paints.yellow,
        Vec3::new(0.0, paint_y, lane_start),
    );
    let band = Vec3::new(2.0 * DOOR_HALF_WIDTH, 0.01, 0.8);
    b.block(
        band,
        0.6,
        &paints.hazard,
        Vec3::new(0.0, paint_y, HALF_DEPTH - 0.7),
    );
    for z in [4.0, 9.0] {
        b.block(
            Vec3::new(1.2, 0.01, 1.2),
            1.0,
            &paints.grate,
            Vec3::new(0.0, paint_y, z),
        );
    }
}

/// A box mesh with world-scale UVs, so textures tile every `tile` meters
/// on every face instead of stretching.
fn tiled_box(size: Vec3, tile: f32) -> Mesh {
    let mut mesh = Mesh::from(Cuboid::from_size(size));
    let uvs: Vec<[f32; 2]> = match (
        mesh.attribute(Mesh::ATTRIBUTE_POSITION),
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL),
    ) {
        (
            Some(VertexAttributeValues::Float32x3(positions)),
            Some(VertexAttributeValues::Float32x3(normals)),
        ) => positions
            .iter()
            .zip(normals)
            .map(|(p, n)| {
                let uv = if n[0].abs() > 0.5 {
                    [p[2], -p[1]]
                } else if n[1].abs() > 0.5 {
                    [p[0], p[2]]
                } else {
                    [p[0], -p[1]]
                };
                [uv[0] / tile, uv[1] / tile]
            })
            .collect(),
        _ => return mesh,
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if let Err(error) = mesh.generate_tangents() {
        warn!("hall mesh without tangents: {error}");
    }
    mesh
}

/// Diagonal yellow and black warning stripes, tiling.
fn hazard_stripes() -> Image {
    const SIZE: u32 = 64;
    let data = (0..SIZE * SIZE)
        .flat_map(|i| {
            let (x, y) = (i % SIZE, i / SIZE);
            if (x + y) % SIZE < SIZE / 2 {
                [235, 170, 10, 255]
            } else {
                [18, 18, 18, 255]
            }
        })
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Body, push_out};

    #[test]
    fn hall_walls_block_the_stalker_but_the_door_lets_it_out() {
        let body = Body {
            radius: 0.3,
            height: 1.8,
        };
        let walls = Collider(WALLS.as_slice());
        let at = GlobalTransform::IDENTITY;
        let push = |point: Vec2| push_out(point, 0.0, body, [(&walls, &at)]);
        for wall in [
            Vec2::new(HALF_WIDTH - 0.2, 0.0),
            Vec2::new(0.0, -HALF_DEPTH + 0.2),
            Vec2::new(HALF_WIDTH - 2.0, HALF_DEPTH - 0.2),
        ] {
            assert_ne!(push(wall), wall, "walked through the wall at {wall}");
        }
        for door in [
            Vec2::new(0.0, HALF_DEPTH),
            Vec2::new(DOOR_HALF_WIDTH - 1.0, HALF_DEPTH),
        ] {
            assert_eq!(push(door), door, "door blocked at {door}");
        }
    }
}
