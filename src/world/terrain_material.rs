//! Tiling ground layers, blended in world space by the terrain shader.

use bevy::image::{ImageAddressMode, ImageSampler};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureFormat};
use bevy::shader::ShaderRef;

use crate::assets::{self, load_repeating};

pub(super) type TerrainMaterial = ExtendedMaterial<StandardMaterial, GroundLayers>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct GroundLayers {
    #[texture(100)]
    #[sampler(101)]
    grass_albedo: Handle<Image>,
    #[texture(102)]
    grass_normal: Handle<Image>,
    #[texture(103)]
    grass_surface: Handle<Image>,
    #[texture(104)]
    earth_albedo: Handle<Image>,
    #[texture(105)]
    earth_normal: Handle<Image>,
    #[texture(106)]
    earth_surface: Handle<Image>,
}

impl MaterialExtension for GroundLayers {
    fn fragment_shader() -> ShaderRef {
        assets::TERRAIN_SHADER.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        assets::TERRAIN_SHADER.into()
    }
}

pub(super) fn load_terrain_material(server: &AssetServer) -> TerrainMaterial {
    let load = |path, is_srgb| load_repeating(server, path, is_srgb);
    TerrainMaterial {
        base: StandardMaterial {
            perceptual_roughness: 1.0,
            ..default()
        },
        extension: GroundLayers {
            grass_albedo: load(assets::TERRAIN_GRASS_ALBEDO, true),
            grass_normal: load(assets::TERRAIN_GRASS_NORMAL, false),
            grass_surface: load(assets::TERRAIN_GRASS_SURFACE, false),
            earth_albedo: load(assets::TERRAIN_EARTH_ALBEDO, true),
            earth_normal: load(assets::TERRAIN_EARTH_NORMAL, false),
            earth_surface: load(assets::TERRAIN_EARTH_SURFACE, false),
        },
    }
}

/// Builds mip chains for repeating textures and surfaces with anisotropic filtering.
pub(super) fn mipmap_filtered_textures(mut images: ResMut<Assets<Image>>) {
    let pending: Vec<_> = images
        .iter()
        .filter(|(_, image)| {
            image.texture_descriptor.mip_level_count == 1
                && image.data.is_some()
                && matches!(
                    &image.sampler,
                    ImageSampler::Descriptor(sampler)
                        if sampler.address_mode_u == ImageAddressMode::Repeat
                            || sampler.anisotropy_clamp > 1
                )
        })
        .map(|(id, _)| id)
        .collect();
    for id in pending {
        if let Some(mut image) = images.get_mut(id) {
            generate_mipmaps(&mut image);
        }
    }
}

fn generate_mipmaps(image: &mut Image) {
    let format = image.texture_descriptor.format;
    if !matches!(
        format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    ) {
        return;
    }
    let (mut width, mut height) = (image.width() as usize, image.height() as usize);
    let Some(data) = image.data.as_mut() else {
        return;
    };
    let decode: [f32; 256] = std::array::from_fn(|i| {
        let v = i as f32 / 255.0;
        if format.is_srgb() {
            LinearRgba::from(Srgba::rgb(v, v, v)).red
        } else {
            v
        }
    });
    let mut offset = 0;
    while width > 1 || height > 1 {
        let next_offset = data.len();
        let (next_width, next_height) = ((width / 2).max(1), (height / 2).max(1));
        for y in 0..next_height {
            for x in 0..next_width {
                let samples = [
                    (2 * x, 2 * y),
                    ((2 * x + 1).min(width - 1), 2 * y),
                    (2 * x, (2 * y + 1).min(height - 1)),
                    ((2 * x + 1).min(width - 1), (2 * y + 1).min(height - 1)),
                ];
                for channel in 0..4 {
                    let mut value = samples
                        .iter()
                        .map(|&(sx, sy)| {
                            let v = data[offset + (sy * width + sx) * 4 + channel];
                            if channel == 3 {
                                v as f32 / 255.0
                            } else {
                                decode[v as usize]
                            }
                        })
                        .sum::<f32>()
                        * 0.25;
                    if format.is_srgb() && channel < 3 {
                        value = Srgba::from(LinearRgba::rgb(value, value, value)).red;
                    }
                    data.push((value * 255.0).round() as u8);
                }
            }
        }
        (width, height) = (next_width, next_height);
        offset = next_offset;
        image.texture_descriptor.mip_level_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension};

    use super::*;

    #[test]
    fn mipmaps_opt_in_for_anisotropic_clamped_atlases() {
        let mut app = App::new();
        let mut images = Assets::<Image>::default();
        let baseline = Image::new_fill(
            Extent3d {
                width: 3,
                height: 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[128, 128, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        let mut atlas = baseline.clone();
        atlas.sampler = ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            anisotropy_clamp: 8,
            ..bevy::image::ImageSamplerDescriptor::linear()
        });
        let ordinary = images.add(baseline);
        let filtered = images.add(atlas);
        app.insert_resource(images)
            .add_systems(Update, mipmap_filtered_textures);

        app.update();
        let images = app.world().resource::<Assets<Image>>();
        let levels = |handle| {
            images
                .get(handle)
                .expect("test image")
                .texture_descriptor
                .mip_level_count
        };
        assert_eq!(levels(&ordinary), 1);
        assert_eq!(levels(&filtered), 2);
    }

    #[test]
    fn mipmaps_average_color_in_linear_space_but_preserve_data_maps() {
        for (format, expected) in [
            (TextureFormat::Rgba8UnormSrgb, 188),
            (TextureFormat::Rgba8Unorm, 128),
        ] {
            let mut image = Image::new(
                Extent3d {
                    width: 2,
                    height: 2,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                vec![
                    0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
                ],
                format,
                RenderAssetUsages::default(),
            );
            generate_mipmaps(&mut image);
            assert_eq!(image.texture_descriptor.mip_level_count, 2);
            let data = image.data.expect("mip pixels");
            assert_eq!(data.len(), 20);
            assert_eq!(&data[16..], &[expected, expected, expected, 255]);
        }
    }

    #[test]
    fn mipmaps_extend_to_one_pixel_for_square_and_narrow_images() {
        for (width, height) in [(4, 4), (1, 4), (4, 1)] {
            let mut image = Image::new_fill(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[128, 128, 255, 255],
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            );
            generate_mipmaps(&mut image);
            assert_eq!(image.texture_descriptor.mip_level_count, 3);
            let pixels = (width * height + (width / 2).max(1) * (height / 2).max(1) + 1) as usize;
            let data = image.data.expect("mip pixels");
            assert_eq!(data.len(), pixels * 4);
            assert!(data.chunks_exact(4).all(|p| p == [128, 128, 255, 255]));
        }
    }
}
