import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var grass_albedo: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var ground_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var grass_normal: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var grass_arm: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var earth_albedo: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var earth_normal: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var earth_arm: texture_2d<f32>;

// Project onto all three axes so quarry walls do not stretch the ground textures.
fn sample_ground(tex: texture_2d<f32>, p: vec3<f32>, weights: vec3<f32>) -> vec3<f32> {
    return textureSample(tex, ground_sampler, p.zy).rgb * weights.x
        + textureSample(tex, ground_sampler, p.xz).rgb * weights.y
        + textureSample(tex, ground_sampler, p.xy).rgb * weights.z;
}

fn ground_normal(
    tex: texture_2d<f32>,
    p: vec3<f32>,
    weights: vec3<f32>,
    normal: vec3<f32>,
) -> vec3<f32> {
    let nx = textureSample(tex, ground_sampler, p.zy).xyz * 2.0 - 1.0;
    let ny = textureSample(tex, ground_sampler, p.xz).xyz * 2.0 - 1.0;
    let nz = textureSample(tex, ground_sampler, p.xy).xyz * 2.0 - 1.0;
    let gradient = vec3<f32>(0.0, nx.y, nx.x) / max(nx.z, 0.2) * weights.x
        + vec3<f32>(ny.x, 0.0, ny.y) / max(ny.z, 0.2) * weights.y
        + vec3<f32>(nz.x, nz.y, 0.0) / max(nz.z, 0.2) * weights.z;
    return normalize(normal + (gradient - normal * dot(gradient, normal)) * 0.45);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let normal = normalize(in.world_normal);
    let axis_weights = pow(abs(normal), vec3<f32>(4.0));
    let weights = axis_weights / dot(axis_weights, vec3<f32>(1.0));
    let p = in.world_position.xyz;
    // The scans cover 15 m and 20 m respectively, keeping stones life-sized.
    let grass_uv = p / 15.0;
    let earth_uv = p / 20.0;
    let grass = sample_ground(grass_albedo, grass_uv, weights);
    let earth = sample_ground(earth_albedo, earth_uv, weights);
    let exposed = clamp(in.color.r, 0.0, 1.0);
    let wetness = clamp(in.color.g, 0.0, 1.0);
    let ash = clamp(in.color.b, 0.0, 1.0);
    var albedo = mix(grass, earth, exposed);
    let luminance = dot(albedo, vec3<f32>(0.2126, 0.7152, 0.0722));
    albedo = mix(albedo, vec3<f32>(luminance) * vec3<f32>(0.72, 0.74, 0.70), ash * 0.9);
    albedo *= mix(vec3<f32>(1.0), vec3<f32>(0.48, 0.56, 0.42), wetness);
    pbr_input.material.base_color = vec4<f32>(albedo * in.color.a, 1.0);

    // Packed maps: ambient occlusion, roughness, metallic (nonmetallic ground).
    let arm = mix(
        sample_ground(grass_arm, grass_uv, weights),
        sample_ground(earth_arm, earth_uv, weights),
        exposed,
    );
    pbr_input.material.perceptual_roughness = clamp(arm.g * mix(1.0, 0.6, wetness), 0.4, 1.0);
    pbr_input.material.metallic = 0.0;
    pbr_input.diffuse_occlusion *= vec3<f32>(mix(1.0, arm.r, 0.7));
    pbr_input.N = normalize(mix(
        ground_normal(grass_normal, grass_uv, weights, normal),
        ground_normal(earth_normal, earth_uv, weights, normal),
        exposed,
    ));

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
