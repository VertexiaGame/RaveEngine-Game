#import bevy_sprite::{
    mesh2d_view_bindings::viewport,
    mesh2d_view_bindings::globals,
    mesh2d_functions::{get_world_from_local, mesh2d_position_local_to_clip},
}
#import bevy_pbr::{
    mesh_view_bindings::view,
    utils::coords_to_viewport_uv,
}

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(3) @binding(100) var clouds_render_texture: texture_2d<f32>;
@group(3) @binding(101) var clouds_render_sampler: sampler;

@group(1) @binding(102) var clouds_atlas_texture: texture_2d<f32>;
@group(1) @binding(103) var clouds_atlas_sampler: sampler;

@group(1) @binding(104) var clouds_worley_texture: texture_3d<f32>;
@group(1) @binding(105) var clouds_worley_sampler: sampler;

@group(3) @binding(106) var sky_texture: texture_2d<f32>;
@group(3) @binding(107) var sky_sampler: sampler;

fn catmull_rom_weights(t: f32) -> vec4f {
    let t2 = t * t;
    let t3 = t2 * t;
    return vec4f(
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2
    );
}

fn upsample_bicubic(cloud_tex: texture_2d<f32>, cloud_sampler: sampler, uv: vec2f) -> vec4f {
    let dims = vec2f(textureDimensions(cloud_tex));
    let texel = 1.0 / dims;
    let grid = uv * dims - 0.5;
    let base = floor(grid);
    let f = fract(grid);
    let wx = catmull_rom_weights(f.x);
    let wy = catmull_rom_weights(f.y);
    let min_uv = texel * 0.5;
    let max_uv = vec2f(1.0, 1.0) - texel * 0.5;
    let row0 = textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(-0.5, -0.5)) * texel, min_uv, max_uv), 0.0) * wx.x
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(0.5, -0.5)) * texel, min_uv, max_uv), 0.0) * wx.y
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(1.5, -0.5)) * texel, min_uv, max_uv), 0.0) * wx.z
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(2.5, -0.5)) * texel, min_uv, max_uv), 0.0) * wx.w;
    let row1 = textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(-0.5, 0.5)) * texel, min_uv, max_uv), 0.0) * wx.x
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(0.5, 0.5)) * texel, min_uv, max_uv), 0.0) * wx.y
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(1.5, 0.5)) * texel, min_uv, max_uv), 0.0) * wx.z
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(2.5, 0.5)) * texel, min_uv, max_uv), 0.0) * wx.w;
    let row2 = textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(-0.5, 1.5)) * texel, min_uv, max_uv), 0.0) * wx.x
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(0.5, 1.5)) * texel, min_uv, max_uv), 0.0) * wx.y
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(1.5, 1.5)) * texel, min_uv, max_uv), 0.0) * wx.z
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(2.5, 1.5)) * texel, min_uv, max_uv), 0.0) * wx.w;
    let row3 = textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(-0.5, 2.5)) * texel, min_uv, max_uv), 0.0) * wx.x
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(0.5, 2.5)) * texel, min_uv, max_uv), 0.0) * wx.y
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(1.5, 2.5)) * texel, min_uv, max_uv), 0.0) * wx.z
        + textureSampleLevel(cloud_tex, cloud_sampler, clamp((base + vec2f(2.5, 2.5)) * texel, min_uv, max_uv), 0.0) * wx.w;
    return row0 * wy.x + row1 * wy.y + row2 * wy.z + row3 * wy.w;
}

fn hash12(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let viewport_uv = coords_to_viewport_uv(mesh.position.xy, view.viewport);
    var clouds = upsample_bicubic(clouds_render_texture, clouds_render_sampler, vec2(viewport_uv));
    clouds = vec4f(clouds.rgb, clamp(clouds.a, 0.0, 1.0));
    var sky = upsample_bicubic(sky_texture, sky_sampler, vec2(viewport_uv));
    sky = max(sky, vec4f(0.0, 0.0, 0.0, 0.0));

    var rgb = max(clouds.rgb + sky.rgb * clouds.a, vec3f(0.0, 0.0, 0.0));
    rgb += (hash12(mesh.position.xy) - 0.5) * (1.0 / 255.0);

    return vec4(rgb, 1.0);
}
