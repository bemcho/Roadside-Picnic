//! Procedural terrain for the Zone.
//!
//! Topography inspired by the Zone from *Roadside Picnic*: rolling, overgrown
//! hills around Harmont, the old quarry, a mountain range, and a river basin.
//!
//! Everything is deterministic — same seed, same Zone on every run.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::hydrology::Wetlands;

/// World-space extent of the terrain, in units, centered on the origin.
const SIZE: f32 = 1536.0;

/// Distance from the origin to each terrain edge along X and Z.
pub const HALF_SIZE: f32 = SIZE / 2.0;

pub(super) const TILE_SIZE: f32 = 128.0;
pub(super) const TILE_RADIUS: i32 = (HALF_SIZE / TILE_SIZE) as i32;
pub(super) const CELLS: usize = 64;
pub(super) const SPACING: f32 = TILE_SIZE / CELLS as f32;

/// Fixed noise seed.
const SEED: u32 = 0x1972_2016;

/// The quarry: an elliptical pit dug out of the hills.
const QUARRY_CENTER: (f32, f32) = (70.0, 40.0);
const QUARRY_RADII: (f32, f32) = (45.0, 30.0);
const QUARRY_DEPTH: f32 = 14.0;

/// The bald hill: a scorched, bare mound.
const BALD_CENTER: (f32, f32) = (-75.0, -55.0);
const BALD_HEIGHT: f32 = 12.0;
const BALD_SIGMA: f32 = 22.0;

/// The marsh: a broad, stagnant lowland.
const MARSH_CENTER: (f32, f32) = (-35.0, 78.0);
const MARSH_RADIUS: f32 = 48.0;
const MARSH_DEPTH: f32 = 2.5;

const ROAD_HALF_WIDTH: f32 = 5.0;
type RoadLeg = (Vec2, Vec2);
const OLD_TRACK: RoadLeg = (Vec2::ZERO, Vec2::new(36.0, 20.0));
const ROADS: [RoadLeg; 7] = [
    OLD_TRACK,
    (Vec2::new(40.0, -104.0), Vec2::new(39.0, -68.0)),
    (Vec2::new(39.0, -68.0), Vec2::new(25.0, -33.0)),
    (Vec2::new(25.0, -33.0), Vec2::ZERO),
    (Vec2::ZERO, Vec2::new(-12.0, 122.0)),
    (Vec2::new(-12.0, 122.0), Vec2::new(35.0, 245.0)),
    (Vec2::new(35.0, 245.0), Vec2::new(35.0, 408.0)),
];

/// The stalkers' base: an old industrial hangar in the southern lowlands, on a
/// levelled pad (wide enough for the whole hall) that fades back into the hills.
pub const BASE: (f32, f32) = (40.0, -128.0);
const BASE_PAD_RADIUS: f32 = 27.0;
const BASE_PAD_BLEND: f32 = 12.0;

/// Where the stalker starts and respawns: inside the hangar, facing its bay.
pub const SPAWN: (f32, f32) = (40.0, -123.0);

/// Deterministic terrain model. Pure math until [`Terrain::build_tile`] is
/// called — insert as a resource so any system can sample the ground.
#[derive(Resource, Clone)]
pub struct Terrain {
    road_heights: [(f32, f32); ROADS.len()],
}

impl Terrain {
    pub fn new() -> Self {
        Self {
            road_heights: ROADS.map(|(a, b)| (hills(a.x, a.y), hills(b.x, b.y))),
        }
    }

    /// Height of the terrain surface at world position `(x, z)`.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.sample(x, z).0
    }

    pub(crate) fn water_depth_at(&self, x: f32, z: f32) -> f32 {
        let wetlands = Wetlands::at(x, z);
        let h = self.land_height(x, z, &wetlands);
        wetlands.depth(h).max(0.0)
    }

    pub(crate) fn can_wade(&self, from: Vec2, to: Vec2, limit: f32) -> bool {
        let depth = self.water_depth_at(to.x, to.y);
        depth <= limit || depth < self.water_depth_at(from.x, from.y)
    }

    fn sample(&self, x: f32, z: f32) -> (f32, Wetlands) {
        let wetlands = Wetlands::at(x, z);
        (self.land_height(x, z, &wetlands), wetlands)
    }

    fn land_height(&self, x: f32, z: f32, wetlands: &Wetlands) -> f32 {
        let mut h = hills(x, z) + mountains(x, z);

        // Bald hill: a smooth scorched mound.
        let bdx = x - BALD_CENTER.0;
        let bdz = z - BALD_CENTER.1;
        let bd2 = bdx * bdx + bdz * bdz;
        h += BALD_HEIGHT * (-bd2 / (2.0 * BALD_SIGMA * BALD_SIGMA)).exp();

        // Marsh: broad, nearly flat depression.
        let mdx = (x - MARSH_CENTER.0) / MARSH_RADIUS;
        let mdz = (z - MARSH_CENTER.1) / MARSH_RADIUS;
        let marsh_d = (mdx * mdx + mdz * mdz).sqrt();
        h -= MARSH_DEPTH * smoothstep(1.0, 0.35, marsh_d);

        // Quarry: excavated bowl with a raised lip and micro-relief on the floor.
        let qdx = (x - QUARRY_CENTER.0) / QUARRY_RADII.0;
        let qdz = (z - QUARRY_CENTER.1) / QUARRY_RADII.1;
        let quarry_d = (qdx * qdx + qdz * qdz).sqrt();
        let cut = QUARRY_DEPTH * smoothstep(1.08, 0.64, quarry_d);
        let lip = 0.9 * (-((quarry_d - 1.08) / 0.13).powi(2)).exp();
        let floor_noise = if quarry_d < 0.9 {
            (value_noise(x * 0.3, z * 0.3, SEED + 11) - 0.5) * 0.9 * smoothstep(0.9, 0.2, quarry_d)
        } else {
            0.0
        };
        h += -cut + lip + floor_noise;

        // Road: the track is flattened toward a gentle ramp between its ends.
        if let Some((leg, t, mask)) = road_at(x, z) {
            let (a, b) = self.road_heights[leg];
            h = lerp(h, lerp(a, b, t), mask * 0.85);
        }
        h = wetlands.soil_height(x, z, h);

        // Base: levelled flat for the yard.
        let pad = self.base_mask(x, z);
        if pad > 0.0 {
            h = lerp(h, hills(BASE.0, BASE.1), pad);
        }

        h
    }

    /// 1.0 on the base's levelled pad, fading to 0.0 around it.
    pub(super) fn base_mask(&self, x: f32, z: f32) -> f32 {
        let distance = Vec2::new(x - BASE.0, z - BASE.1).length();
        smoothstep(BASE_PAD_RADIUS + BASE_PAD_BLEND, BASE_PAD_RADIUS, distance)
    }

    /// 1.0 on the road centerline, 0.0 away from it.
    pub(super) fn road_mask(&self, x: f32, z: f32) -> f32 {
        road_at(x, z).map_or(0.0, |(_, _, weight)| weight)
    }

    pub(super) fn normal_at(&self, x: f32, z: f32) -> Vec3 {
        let e = 0.25;
        let hx = (self.height_at(x + e, z) - self.height_at(x - e, z)) / (2.0 * e);
        let hz = (self.height_at(x, z + e) - self.height_at(x, z - e)) / (2.0 * e);
        Vec3::new(-hx, 1.0, -hz).normalize()
    }

    /// How readily trees grow at `(x, z)`, from 0.0 (never) to 1.0: none on
    /// the road, in the quarry, at the base, or on steep ground; few on the
    /// scorched hill.
    pub fn woodland_at(&self, x: f32, z: f32) -> f32 {
        let qdx = (x - QUARRY_CENTER.0) / QUARRY_RADII.0;
        let qdz = (z - QUARRY_CENTER.1) / QUARRY_RADII.1;
        let outside_quarry = smoothstep(1.1, 1.3, (qdx * qdx + qdz * qdz).sqrt());

        let bdx = x - BALD_CENTER.0;
        let bdz = z - BALD_CENTER.1;
        let ash = (-(bdx * bdx + bdz * bdz) / (2.0 * BALD_SIGMA * BALD_SIGMA)).exp();

        let off_road = 1.0 - smoothstep(0.05, 0.3, self.road_mask(x, z));
        let off_base = 1.0 - self.base_mask(x, z);
        if outside_quarry * off_road * off_base == 0.0 {
            return 0.0;
        }
        let (h, wetlands) = self.sample(x, z);
        let dry = smoothstep(0.05, -0.5, wetlands.depth(h));
        let normal = self.normal_at(x, z);
        let soil = smoothstep(1.05, 0.54, normal.xz().length() / normal.y);
        let alpine = smoothstep(119.0, 75.0, h);

        outside_quarry * soil * dry * alpine * off_road * off_base * (1.0 - 0.85 * ash)
    }

    /// Shader data, not a tint: exposed earth, wetness, ash, macro variation.
    fn surface_weights_at(
        &self,
        x: f32,
        z: f32,
        h: f32,
        slope: f32,
        wetlands: &Wetlands,
    ) -> [f32; 4] {
        let qdx = (x - QUARRY_CENTER.0) / QUARRY_RADII.0;
        let qdz = (z - QUARRY_CENTER.1) / QUARRY_RADII.1;
        let quarry = smoothstep(1.12, 0.85, (qdx * qdx + qdz * qdz).sqrt());

        let mdx = (x - MARSH_CENTER.0) / MARSH_RADIUS;
        let mdz = (z - MARSH_CENTER.1) / MARSH_RADIUS;
        let marsh = smoothstep(1.0, 0.6, (mdx * mdx + mdz * mdz).sqrt());

        let bdx = x - BALD_CENTER.0;
        let bdz = z - BALD_CENTER.1;
        let ash = (-(bdx * bdx + bdz * bdz) / (2.0 * BALD_SIGMA * BALD_SIGMA)).exp();

        let exposed = smoothstep(0.35, 0.85, slope)
            .max(self.road_mask(x, z))
            .max(quarry)
            .max(ash * 0.95)
            .max(smoothstep(42.0, 110.0, h) * 0.92)
            .max(self.base_mask(x, z));
        let wetness = wetlands.wetness().max(marsh * 0.72);
        let variation = lerp(0.78, 1.12, value_noise(x * 0.04, z * 0.04, SEED + 5));
        [exposed, wetness, ash, variation]
    }

    /// A shared lattice and normal halo make neighboring chunks meet exactly.
    pub(super) fn build_tile(&self, tile: IVec2) -> (Mesh, Vec<WaterVertex>) {
        let n = CELLS + 1;
        let halo = n + 2;
        let origin = tile.as_vec2() * TILE_SIZE;
        let samples: Vec<_> = (0..halo)
            .flat_map(|z| {
                (0..halo).map(move |x| {
                    self.sample(
                        origin.x + (x as f32 - 1.0) * SPACING,
                        origin.y + (z as f32 - 1.0) * SPACING,
                    )
                })
            })
            .collect();
        let mut positions = Vec::with_capacity(n * n);
        let mut normals = Vec::with_capacity(n * n);
        let mut colors = Vec::with_capacity(n * n);
        let mut water = Vec::with_capacity(n * n);

        for iz in 0..n {
            let z = origin.y + iz as f32 * SPACING;
            for ix in 0..n {
                let x = origin.x + ix as f32 * SPACING;
                let i = (iz + 1) * halo + ix + 1;
                let (h, wet) = &samples[i];
                let normal = Vec3::new(
                    samples[i - 1].0 - samples[i + 1].0,
                    2.0 * SPACING,
                    samples[i - halo].0 - samples[i + halo].0,
                )
                .normalize();
                let slope = normal.xz().length() / normal.y;
                positions.push([x, *h, z]);
                normals.push(normal.to_array());
                colors.push(self.surface_weights_at(x, z, *h, slope, wet));
                water.push(WaterVertex {
                    depth: wet.depth(*h),
                    marsh: wet.marsh(),
                    flow: wet.flow(),
                });
            }
        }

        let mut indices = Vec::with_capacity(CELLS * CELLS * 6);
        for iz in 0..CELLS {
            for ix in 0..CELLS {
                let a = (iz * n + ix) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        (mesh, water)
    }
}

#[derive(Clone, Copy)]
pub(super) struct WaterVertex {
    pub depth: f32,
    pub marsh: f32,
    pub flow: Vec2,
}

fn road_at(x: f32, z: f32) -> Option<(usize, f32, f32)> {
    if !(-27.0..49.0).contains(&x) || !(-118.0..421.0).contains(&z) {
        return None;
    }
    let at = Vec2::new(x, z);
    let mut closest = None;
    let mut best = (ROAD_HALF_WIDTH + 6.0).powi(2);
    for (i, (a, b)) in ROADS.into_iter().enumerate() {
        let leg = b - a;
        let t = ((at - a).dot(leg) / leg.length_squared()).clamp(0.0, 1.0);
        let d2 = at.distance_squared(a + leg * t);
        if d2 < best {
            best = d2;
            closest = Some((i, t));
        }
    }
    closest.map(|(i, t)| {
        (
            i,
            t,
            smoothstep(ROAD_HALF_WIDTH + 6.0, ROAD_HALF_WIDTH, best.sqrt()),
        )
    })
}

fn hills(x: f32, z: f32) -> f32 {
    let local = 6.5 * fbm(x, z, SEED, 3, 1.0 / 48.0);
    let highlands = smoothstep(150.0, 330.0, x.abs().max(z.abs()));
    if highlands > 0.0 {
        local + highlands * 17.0 * fbm(x, z, SEED + 4, 3, 1.0 / 225.0)
    } else {
        local
    }
}

fn mountains(x: f32, z: f32) -> f32 {
    if x.abs() < 300.0 && z < 364.0 {
        return 0.0;
    }
    let p = Vec2::new(x, z);
    let mut summit: f32 = 0.0;
    for (center, radii, height) in [
        (Vec2::new(134.0, 675.0), Vec2::new(215.0, 258.0), 157.0),
        (Vec2::new(470.0, 605.0), Vec2::new(211.0, 255.0), 211.0),
        (Vec2::new(649.0, 498.0), Vec2::new(187.0, 209.0), 188.0),
        (Vec2::new(-466.0, 659.0), Vec2::new(246.0, 220.0), 162.0),
        (Vec2::new(-601.0, -290.0), Vec2::new(223.0, 286.0), 132.0),
    ] {
        let d2 = ((p - center) / radii).length_squared();
        let profile = smoothstep(1.0, 0.0, d2).powf(1.85);
        summit = smooth_max(summit, profile * height);
    }
    if summit < 0.001 {
        return 0.0;
    }
    let ridges = 1.0 - fbm(x + z * 0.4, z, SEED + 23, 4, 1.0 / 92.0).abs();
    summit * (0.67 + 0.32 * ridges.powi(3))
        + smoothstep(0.0, 60.0, summit) * 8.5 * fbm(x, z, SEED + 24, 3, 1.0 / 33.0)
}

fn smooth_max(a: f32, b: f32) -> f32 {
    let overlap = (1.0 - (a - b).abs() / 12.0).max(0.0);
    a.max(b) + 3.0 * overlap * overlap * smoothstep(0.0, 10.0, a.min(b))
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub(super) fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Deterministic integer-lattice hash in `[0, 1)`.
pub(super) fn lattice_hash(ix: i32, iz: i32, seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x8da6_b343)
        ^ (iz as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h as f32 / u32::MAX as f32
}

/// Bilinearly interpolated value noise in `[0, 1)`.
pub(super) fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor();
    let iz = z.floor();
    let fx = x - ix;
    let fz = z - iz;
    let (ix, iz) = (ix as i32, iz as i32);

    let a = lattice_hash(ix, iz, seed);
    let b = lattice_hash(ix + 1, iz, seed);
    let c = lattice_hash(ix, iz + 1, seed);
    let d = lattice_hash(ix + 1, iz + 1, seed);

    // Quintic interpolation keeps curvature continuous across lattice boundaries.
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let ux = fade(fx);
    let uz = fade(fz);
    let ab = a + (b - a) * ux;
    let cd = c + (d - c) * ux;
    ab + (cd - ab) * uz
}

/// Fractal Brownian motion: octaves of value noise, normalized to `[-1, 1]`.
pub(super) fn fbm(x: f32, z: f32, seed: u32, octaves: u32, base_freq: f32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut norm = 0.0;
    let mut freq = base_freq;

    for octave in 0..octaves {
        // Per-octave offsets decorrelate the lattice sampling.
        let ox = octave as f32 * 7.13;
        let oz = octave as f32 * 3.17;
        sum += amp * (value_noise(x * freq + ox, z * freq + oz, seed) * 2.0 - 1.0);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }

    sum / norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_is_deterministic() {
        let terrain = Terrain::new();
        let a = terrain.height_at(12.3, -45.6);
        let b = terrain.height_at(12.3, -45.6);
        assert_eq!(a, b);
    }

    #[test]
    fn value_noise_is_in_unit_range() {
        for i in 0..100 {
            let v = value_noise(i as f32 * 0.7, -i as f32 * 0.3, SEED);
            assert!((0.0..1.0).contains(&v), "value {v} out of range");
        }
    }

    #[test]
    fn quarry_floor_is_deeper_than_rim() {
        let terrain = Terrain::new();
        // Just inside the quarry wall vs. just outside it.
        let floor = terrain.height_at(QUARRY_CENTER.0 + 0.64 * QUARRY_RADII.0, QUARRY_CENTER.1);
        let rim = terrain.height_at(QUARRY_CENTER.0 + 1.05 * QUARRY_RADII.0, QUARRY_CENTER.1);
        assert!(floor < rim - 5.0, "floor {floor} vs rim {rim}");
    }

    #[test]
    fn road_is_flatter_than_its_surroundings() {
        let terrain = Terrain::new();
        let mid = (30.0, 17.3); // on the road centerline
        let off = (30.0, 35.0); // well off the road
        let normal_on = terrain.normal_at(mid.0, mid.1);
        let normal_off = terrain.normal_at(off.0, off.1);
        let slope_on = normal_on.xz().length() / normal_on.y;
        let slope_off = normal_off.xz().length() / normal_off.y;
        assert!(
            slope_on < slope_off * 0.5,
            "road slope {slope_on} vs {slope_off}"
        );
    }

    #[test]
    fn road_entrance_meets_the_surrounding_hills() {
        let terrain = Terrain::new();
        let (start, _) = OLD_TRACK;
        let road_height = terrain.height_at(start.x, start.y);
        let hill_height = hills(start.x, start.y);
        assert!(
            (road_height - hill_height).abs() < 0.1,
            "road entrance should not form a raised platform"
        );
    }

    #[test]
    fn adjacent_chunks_share_positions_normals_and_surface_weights() {
        let terrain = Terrain::new();
        let (mesh, _) = terrain.build_tile(IVec2::new(-1, 2));
        let (neighbor, _) = terrain.build_tile(IVec2::new(0, 2));
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("positions present");
        assert_eq!(positions.len(), (CELLS + 1) * (CELLS + 1));
        assert_eq!(
            mesh.indices().expect("shared grid indices").len(),
            CELLS * CELLS * 6
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("smooth normals present");
        };
        for normal in normals {
            let normal = Vec3::from_array(*normal);
            assert!((normal.length() - 1.0).abs() < 1e-5);
            assert!(normal.y > 0.0);
        }
        let n = CELLS + 1;
        use bevy::mesh::VertexAttributeValues::{Float32x3, Float32x4};
        for attribute in [
            Mesh::ATTRIBUTE_POSITION,
            Mesh::ATTRIBUTE_NORMAL,
            Mesh::ATTRIBUTE_COLOR,
        ] {
            match (mesh.attribute(attribute), neighbor.attribute(attribute)) {
                (Some(Float32x3(left)), Some(Float32x3(right))) => {
                    for z in 0..n {
                        assert_eq!(left[z * n + CELLS], right[z * n]);
                    }
                }
                (Some(Float32x4(left)), Some(Float32x4(right))) => {
                    for z in 0..n {
                        assert_eq!(left[z * n + CELLS], right[z * n]);
                    }
                }
                _ => panic!("missing terrain seam attributes"),
            }
        }
    }

    #[test]
    fn mountain_range_rises_above_the_traversable_lowlands() {
        let terrain = Terrain::new();
        let summits = [
            (134.0, 675.0),
            (470.0, 605.0),
            (649.0, 498.0),
            (-466.0, 659.0),
        ];
        for (x, z) in summits {
            assert!(terrain.height_at(x, z) > 115.0);
        }
        assert!(terrain.height_at(SPAWN.0, SPAWN.1).abs() < 5.0);
        assert!(terrain.height_at(35.0, 340.0) < 10.0);
    }

    #[test]
    fn terrain_normals_are_continuous_across_noise_and_quarry_boundaries() {
        let terrain = Terrain::new();
        for (x, z) in [
            (48.0, -48.0),
            (0.0, 0.0),
            (BALD_CENTER.0, BALD_CENTER.1),
            (QUARRY_CENTER.0 + QUARRY_RADII.0 * 1.08, QUARRY_CENTER.1),
            (QUARRY_CENTER.0 + QUARRY_RADII.0 * 0.64, QUARRY_CENTER.1),
        ] {
            for offset in [Vec2::X, Vec2::Y] {
                let before = terrain.normal_at(x - offset.x * 0.01, z - offset.y * 0.01);
                let after = terrain.normal_at(x + offset.x * 0.01, z + offset.y * 0.01);
                assert!(before.dot(after) > 0.999, "normal seam near ({x}, {z})");
            }
        }
    }

    #[test]
    fn surface_layers_blend_continuously_with_slope_and_elevation() {
        let terrain = Terrain::new();
        for slope in [0.35, 0.55, 0.85] {
            let wet = Wetlands::at(100.0, -100.0);
            let before = terrain.surface_weights_at(100.0, -100.0, 1.499, slope - 0.001, &wet);
            let after = terrain.surface_weights_at(100.0, -100.0, 1.501, slope + 0.001, &wet);
            for channel in 0..4 {
                assert!((before[channel] - after[channel]).abs() < 0.01);
            }
        }
    }

    #[test]
    fn base_stands_on_a_level_pad_clear_of_trees() {
        let terrain = Terrain::new();
        let level = terrain.height_at(BASE.0, BASE.1);
        for (dx, dz) in [(-17.5, -19.5), (17.5, 19.5), (0.0, 22.0), (-10.0, 12.0)] {
            let h = terrain.height_at(BASE.0 + dx, BASE.1 + dz);
            assert!((h - level).abs() < 1e-4, "pad not level at ({dx}, {dz})");
            assert_eq!(terrain.woodland_at(BASE.0 + dx, BASE.1 + dz), 0.0);
        }
    }
}
