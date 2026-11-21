use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{FogVolume, VolumetricFog};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::player::Player;
use crate::player::PlayerCamera;

use super::terrain::Terrain;
use super::terrain_material::{TerrainMaterial, update_terrain_fog_settings};
use super::{LandscapeMaterials, WorldRoot};

pub(crate) const CAMERA_FAR: f32 = 16.0 * 128.0;
const MIN_VISIBILITY: f32 = 10.0 * 128.0;
const FOG_WIDTH: u32 = 128;
const FOG_HEIGHT: u32 = 24;
const FOG_DEPTH: u32 = 128;
const WORLD_WIDTH: f32 = 4_096.0;
const WORLD_HEIGHT: f32 = 10.0;
const CENTER_OFFSET: f32 = 1.0;

#[derive(Component)]
pub(in crate::world) struct GroundFog;

pub(crate) fn camera_fog() -> VolumetricFog {
    VolumetricFog {
        ambient_color: Color::linear_rgb(0.48, 0.57, 0.64),
        ambient_intensity: 0.30,
        jitter: 0.0,
        step_count: 48,
    }
}

pub(crate) fn camera_distance_fog() -> DistanceFog {
    DistanceFog {
        color: Color::linear_rgb(0.72, 0.80, 0.76),
        directional_light_color: Color::linear_rgba(1.0, 0.74, 0.38, 0.18),
        directional_light_exponent: 18.0,
        falloff: FogFalloff::from_visibility_squared(CAMERA_FAR),
    }
}

pub(super) fn spawn_ground_fog(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let density_texture = images.add(fog_density_texture());
    commands.spawn((
        WorldRoot,
        GroundFog,
        Transform::from_xyz(0.0, CENTER_OFFSET, 0.0).with_scale(Vec3::new(
            WORLD_WIDTH,
            WORLD_HEIGHT,
            WORLD_WIDTH,
        )),
        FogVolume {
            fog_color: Color::linear_rgb(0.72, 0.80, 0.76),
            density_factor: 0.075,
            density_texture: Some(density_texture),
            absorption: 0.08,
            scattering: 0.68,
            scattering_asymmetry: 0.36,
            light_tint: Color::linear_rgb(1.0, 0.91, 0.74),
            light_intensity: 1.2,
            ..default()
        },
    ));
}

pub(super) fn scroll_ground_fog(time: Res<Time>, mut fog: Query<&mut FogVolume, With<GroundFog>>) {
    for mut fog in &mut fog {
        fog.density_texture_offset += Vec3::new(0.0014, 0.0, -0.0007) * time.delta_secs();
    }
}

pub(super) fn follow_ground_fog(
    player: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
    mut fog: Query<&mut Transform, (With<GroundFog>, Without<Player>)>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let ground = terrain.mesh_height_at(player.translation.xz());
    for mut transform in &mut fog {
        transform.translation.y = ground + CENTER_OFFSET;
    }
}

pub(super) fn follow_terrain_fog(
    player: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
    landscape_materials: Res<LandscapeMaterials>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let fog_top =
        terrain.mesh_height_at(player.translation.xz()) + CENTER_OFFSET + WORLD_HEIGHT * 0.5;
    update_terrain_fog_settings(
        &mut materials,
        &landscape_materials.terrain,
        fog_top,
        WORLD_HEIGHT,
    );
}

pub(super) fn update_distance_fog(
    player: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
    mut cameras: Query<&mut DistanceFog, With<PlayerCamera>>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let visibility = visibility_at(player.translation.xz(), &terrain);
    for mut fog in &mut cameras {
        fog.falloff = FogFalloff::from_visibility_squared(visibility);
    }
}

fn visibility_at(at: Vec2, terrain: &Terrain) -> f32 {
    let u = at.x / WORLD_WIDTH + 0.5;
    let v = at.y / WORLD_WIDTH + 0.5;
    let cloud = cloud_noise(u.rem_euclid(1.0), v.rem_euclid(1.0), 0.5);
    let ground = terrain.mesh_height_at(at);
    let lowland = 1.0 - super::terrain::smoothstep(16.0, 95.0, ground);
    let density = (cloud * 0.72 + lowland * 0.18).clamp(0.0, 1.0);
    CAMERA_FAR - (CAMERA_FAR - MIN_VISIBILITY) * density
}

fn fog_density_texture() -> Image {
    let mut data = Vec::with_capacity((FOG_WIDTH * FOG_HEIGHT * FOG_DEPTH) as usize);
    for z in 0..FOG_DEPTH {
        let v = z as f32 / FOG_DEPTH as f32;
        for y in 0..FOG_HEIGHT {
            let height = y as f32 / (FOG_HEIGHT - 1) as f32;
            for x in 0..FOG_WIDTH {
                let u = x as f32 / FOG_WIDTH as f32;
                let cloud = cloud_noise(u, v, height);
                let value = (cloud * 255.0).round() as u8;
                data.push(value);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: FOG_WIDTH,
            height: FOG_HEIGHT,
            depth_or_array_layers: FOG_DEPTH,
        },
        TextureDimension::D3,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn cloud_noise(u: f32, v: f32, height: f32) -> f32 {
    let broad = periodic_noise(u, v + height * 0.025, 6.0, 0.7);
    let medium = periodic_noise(u + height * 0.04, v, 15.0, 2.3);
    let fine = periodic_noise(u, v - height * 0.055, 39.0, 4.1);
    let cloud = (broad * 0.57 + medium * 0.31 + fine * 0.12).clamp(0.0, 1.0);
    let billow = smoothstep(0.38, 0.76, cloud);
    let upper_edge = 0.58 + billow * 0.30;
    let vertical = smoothstep(0.0, 0.16, height) * (1.0 - smoothstep(upper_edge, 1.0, height));
    (0.04 + billow * 0.96) * vertical
}

fn periodic_noise(u: f32, v: f32, frequency: f32, phase: f32) -> f32 {
    let x = TAU * u * frequency;
    let z = TAU * v * frequency;
    0.5 + 0.25
        * (f32::sin(x + phase) * f32::cos(z * 0.93 - phase * 0.7)
            + f32::sin((x + z) * 0.73 + phase * 1.9) * f32::cos((z - x) * 0.61 - phase))
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_density_is_deterministic_layered_and_tapers_below_the_sky() {
        let image = fog_density_texture();
        assert_eq!(image.width(), FOG_WIDTH);
        assert_eq!(image.height(), FOG_HEIGHT);
        assert_eq!(image.texture_descriptor.dimension, TextureDimension::D3);
        assert_eq!(image.texture_descriptor.format, TextureFormat::R8Unorm);

        let data = image.data.expect("fog density pixels");
        assert!(data.iter().any(|&value| value < 20));
        assert!(data.iter().any(|&value| value > 180));

        let at = |x, y, z| {
            data[z * FOG_WIDTH as usize * FOG_HEIGHT as usize + y * FOG_WIDTH as usize + x] as u8
        };
        assert!((0..FOG_DEPTH as usize).all(|z| (0..FOG_WIDTH as usize).all(|x| at(x, 0, z) == 0)));
        assert!(
            (0..FOG_DEPTH as usize).all(|z| {
                (0..FOG_WIDTH as usize).all(|x| at(x, FOG_HEIGHT as usize - 1, z) == 0)
            })
        );
        assert!(
            (0..FOG_DEPTH as usize).any(|z| {
                (0..FOG_WIDTH as usize).any(|x| at(x, FOG_HEIGHT as usize / 2, z) > 20)
            })
        );
        assert_eq!(cloud_noise(0.13, 0.62, 0.31), cloud_noise(0.13, 0.62, 0.31));
    }

    #[test]
    fn fog_volume_follows_the_players_ground_elevation() {
        let terrain = Terrain::new();
        let at = Vec2::new(470.0, 605.0);
        let mut app = App::new();
        app.insert_resource(terrain.clone())
            .add_systems(Update, follow_ground_fog);
        let volume = app
            .world_mut()
            .spawn((GroundFog, Transform::from_scale(Vec3::ONE)))
            .id();
        app.world_mut().spawn((
            Player::default(),
            Transform::from_xyz(at.x, terrain.mesh_height_at(at) + 1.2, at.y),
        ));
        app.update();
        let transform = app.world().get::<Transform>(volume).expect("fog volume");
        assert_eq!(
            transform.translation.y,
            terrain.mesh_height_at(at) + CENTER_OFFSET
        );
        assert_eq!(transform.scale.y, 1.0);
    }

    #[test]
    fn fog_visibility_stays_between_ten_and_sixteen_tiles() {
        let terrain = Terrain::new();
        let visibilities: Vec<_> = (-2..=2)
            .flat_map(|x| (-2..=2).map(move |z| Vec2::new(x as f32 * 384.0, z as f32 * 384.0)))
            .map(|at| visibility_at(at, &terrain))
            .collect();
        assert!(
            visibilities
                .iter()
                .all(|visibility| (MIN_VISIBILITY..=CAMERA_FAR).contains(visibility))
        );
        let minimum = visibilities.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let maximum = visibilities
            .iter()
            .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        assert!(maximum - minimum >= 128.0, "fog banks must vary by region");
    }

    #[test]
    fn distance_fog_builds_denser_cloud_banks_toward_visibility_limit() {
        assert!(matches!(
            camera_distance_fog().falloff,
            FogFalloff::ExponentialSquared { .. }
        ));
    }
}
