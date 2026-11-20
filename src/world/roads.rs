//! Graded landmark tracks and their shared visual and scenery masks.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::cordon::GATE;
use super::hydrology::FORD_X;
use super::terrain::{BASE, lerp, smoothstep};

const HALF_WIDTH: f32 = 5.0;
const SHOULDER: f32 = 11.0;
const QUARRY_JUNCTION: Vec2 = Vec2::ZERO;
const BALD_HILL_JUNCTION: Vec2 = Vec2::new(25.0, -33.0);
const WEST_JUNCTION: Vec2 = Vec2::new(-12.0, 122.0);
const SWAMP_JUNCTION: Vec2 = Vec2::new(-121.0, 138.0);

const ROUTES: [&[Vec2]; 5] = [
    &[
        Vec2::new(BASE.0, BASE.1 + 18.0),
        GATE,
        BALD_HILL_JUNCTION,
        QUARRY_JUNCTION,
        Vec2::new(-8.0, 57.0),
        WEST_JUNCTION,
        Vec2::new(4.0, 164.0),
        Vec2::new(22.0, 202.0),
        Vec2::new(FORD_X, 245.0),
        Vec2::new(FORD_X, 408.0),
        Vec2::new(51.0, 456.0),
    ],
    &[
        QUARRY_JUNCTION,
        Vec2::new(18.0, 10.0),
        Vec2::new(36.0, 20.0),
    ],
    &[
        BALD_HILL_JUNCTION,
        Vec2::new(9.0, -28.0),
        Vec2::new(-17.0, -27.0),
        Vec2::new(-40.0, -27.0),
        Vec2::new(-62.0, -23.0),
    ],
    &[
        WEST_JUNCTION,
        Vec2::new(-47.0, 131.0),
        Vec2::new(-84.0, 134.0),
        SWAMP_JUNCTION,
        Vec2::new(-178.0, 141.0),
        Vec2::new(-225.0, 149.0),
        Vec2::new(-270.0, 161.0),
        Vec2::new(-308.0, 177.0),
        Vec2::new(-335.0, 196.0),
        Vec2::new(-355.0, 224.0),
    ],
    &[
        SWAMP_JUNCTION,
        Vec2::new(-140.0, 150.0),
        Vec2::new(-155.0, 172.0),
    ],
];

#[derive(Clone)]
struct Leg {
    start: Vec2,
    along: Vec2,
    length: f32,
    levels: [f32; 2],
    min: Vec2,
    max: Vec2,
}

#[derive(Clone)]
pub(super) struct RoadNetwork {
    legs: Vec<Leg>,
    min: Vec2,
    max: Vec2,
}

pub(super) struct RoadSample {
    distance: f32,
    pub height: f32,
}

impl RoadSample {
    pub fn mask(&self) -> f32 {
        smoothstep(SHOULDER, HALF_WIDTH, self.distance)
    }

    fn surface(&self) -> [u8; 2] {
        let road = smoothstep(7.5, 4.6, self.distance);
        let rut = smoothstep(0.83, 0.32, (self.distance - 1.51).abs());
        [(road * 255.0).round() as u8, (rut * 255.0).round() as u8]
    }
}

impl RoadNetwork {
    pub fn new(level: impl Fn(Vec2) -> f32) -> Self {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        let mut legs = Vec::new();
        for path in ROUTES {
            for &[a, b] in path.array_windows() {
                let delta = b - a;
                let length = delta.length();
                let leg = Leg {
                    start: a,
                    along: delta / length,
                    length,
                    levels: [level(a), level(b)],
                    min: a.min(b) - Vec2::splat(SHOULDER),
                    max: a.max(b) + Vec2::splat(SHOULDER),
                };
                min = min.min(leg.min);
                max = max.max(leg.max);
                legs.push(leg);
            }
        }
        Self { legs, min, max }
    }

    pub fn sample(&self, at: Vec2) -> Option<RoadSample> {
        if at.cmplt(self.min).any() || at.cmpgt(self.max).any() {
            return None;
        }
        let mut nearest = None;
        let mut best = SHOULDER * SHOULDER;
        for leg in &self.legs {
            if at.cmplt(leg.min).any() || at.cmpgt(leg.max).any() {
                continue;
            }
            let run = (at - leg.start).dot(leg.along).clamp(0.0, leg.length);
            let distance_squared = at.distance_squared(leg.start + leg.along * run);
            if distance_squared < best {
                nearest = Some((leg, run / leg.length));
                best = distance_squared;
            }
        }
        nearest.map(|(leg, t)| RoadSample {
            distance: best.sqrt(),
            height: lerp(leg.levels[0], leg.levels[1], t),
        })
    }

    pub fn surface_texture(&self) -> (Image, Vec4) {
        // A world-space mask keeps the same road width across coarse and fine terrain tiles.
        const TEXELS_PER_METRE: f32 = 2.0;
        let origin = self.min.floor();
        let size = ((self.max.ceil() - origin) * TEXELS_PER_METRE).as_uvec2();
        let mut pixels = Vec::with_capacity((size.x * size.y * 2) as usize);
        for z in 0..size.y {
            for x in 0..size.x {
                let at = origin + (Vec2::new(x as f32, z as f32) + 0.5) / TEXELS_PER_METRE;
                let texel = self.sample(at).map_or([0; 2], |road| road.surface());
                pixels.extend_from_slice(&texel);
            }
        }
        let image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rg8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        let uv_scale = Vec2::splat(TEXELS_PER_METRE) / size.as_vec2();
        (image, Vec4::new(origin.x, origin.y, uv_scale.x, uv_scale.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Terrain;

    #[test]
    fn every_landmark_track_has_a_continuous_drivable_route_back_to_the_gate() {
        let terrain = Terrain::new();
        assert!(ROUTES[0].contains(&GATE));
        for (i, path) in ROUTES.iter().enumerate() {
            if i > 0 {
                assert!(ROUTES[..i].iter().any(|road| road.contains(&path[0])));
            }
            for &[a, b] in path.array_windows() {
                let steps = a.distance(b).ceil() as usize;
                let across = (b - a).normalize().perp();
                for lane in [-2.4, 0.0, 2.4] {
                    let offset = across * lane;
                    let mut last = terrain.mesh_height_at(a + offset);
                    for step in 0..=steps {
                        let at = a.lerp(b, step as f32 / steps as f32) + offset;
                        assert!(terrain.road_mask(at.x, at.y) > 0.99);
                        let height = terrain.mesh_height_at(at);
                        assert!(
                            (height - last).abs() < 0.48,
                            "route {i} has a steep step at {at}: {last} to {height}"
                        );
                        assert!(
                            terrain.water_depth_at(at.x, at.y) <= if i == 0 { 0.53 } else { 0.05 },
                            "route {i} floods at {at}"
                        );
                        last = height;
                    }
                }
            }
        }
    }

    #[test]
    fn the_material_mask_draws_a_lane_and_two_ruts_without_painting_the_yard() {
        let terrain = Terrain::new();
        let (image, bounds) = terrain.roads.surface_texture();
        let read = |at: Vec2| {
            let uv = (at - bounds.xy()) * bounds.zw();
            let texel = (uv * Vec2::new(image.width() as f32, image.height() as f32))
                .floor()
                .as_uvec2();
            let i = ((texel.y * image.width() + texel.x) * 2) as usize;
            let bytes = image.data.as_ref().unwrap();
            (bytes[i], bytes[i + 1])
        };
        assert_eq!(read(GATE), (255, 0));
        assert_eq!(read(GATE + Vec2::new(-10.0, -4.0)), (0, 0));
        assert_eq!(read(Vec2::new(FORD_X, 300.0)), (255, 0));
        for side in [-1.0, 1.0] {
            assert!(read(Vec2::new(FORD_X + side * 1.5, 300.0)).1 >= 235);
            assert_eq!(read(Vec2::new(FORD_X + side * 8.0, 300.0)), (0, 0));
        }
        for path in ROUTES {
            assert!(path.iter().all(|&at| read(at).0 == 255));
        }
    }
}
