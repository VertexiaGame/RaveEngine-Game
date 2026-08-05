#import bevy_pbr::{
    mesh_functions,
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::view,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var stud_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var stud_texture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var inlet_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var inlet_texture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var stud_ambient_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var stud_ambient_texture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var stud_height_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var stud_height_texture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var inlet_ambient_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var inlet_ambient_texture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var inlet_height_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var inlet_height_texture_sampler: sampler;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    let model = mesh_functions::get_world_from_local(in.instance_index);

    let scale = vec3<f32>(
        length(model[0].xyz),
        length(model[1].xyz),
        length(model[2].xyz)
    );

    let r0 = model[0].xyz / scale.x;
    let r1 = model[1].xyz / scale.y;
    let r2 = model[2].xyz / scale.z;

    let rot_t = transpose(mat3x3<f32>(r0, r1, r2));

    let translation = model[3].xyz;

    let world_size_coord = rot_t * (in.world_position.xyz - translation);

    let stud_count = scale.xz * vec2<f32>(4.0, 2.0);
    let uv = world_size_coord.xz / 0.28 + stud_count * 0.5;

    let local_normal = rot_t * in.world_normal;

    let dist = distance(in.world_position.xyz, view.world_position);
    let fade = clamp((80.0 - dist) / 40.0, 0.0, 1.0);

    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let local_view = rot_t * view_dir;

    let num_layers = 32u;
    let layer_height = 1.0 / f32(num_layers);

    if (local_normal.y > 0.85 && fade > 0.0005) {
        let parallax_dir = local_view.xz / max(local_view.y, 0.1);
        let parallax_vec = normalize(parallax_dir) * min(length(parallax_dir) * 0.2, 0.25);

        var current_uv = uv + parallax_vec;
        var current_height = pow(textureSample(stud_height_texture, stud_height_texture_sampler, current_uv).r, 1.0 / 2.2);
        var current_layer = 0.0;
        for (var i = 0u; i < num_layers; i++) {
            if (current_height >= 1.0 - current_layer) {
                break;
            }
            current_uv -= parallax_vec / f32(num_layers);
            current_height = pow(textureSample(stud_height_texture, stud_height_texture_sampler, current_uv).r, 1.0 / 2.2);
            current_layer += layer_height;
        }

        let parallax_stud_sample = textureSample(stud_texture, stud_texture_sampler, current_uv);
        let ambient_sample = textureSample(stud_ambient_texture, stud_ambient_texture_sampler, current_uv);

        let normal_srgb = pow(parallax_stud_sample.rgb, vec3<f32>(1.0 / 2.2));
        let normal_ts = normal_srgb * 2.0 - 1.0;
        pbr_input.N = normalize(r0 * normal_ts.x + r2 * normal_ts.y + r1 * normal_ts.z);

        let stud_mask = smoothstep(0.12, 0.45, current_height) * fade;
        let ambient_mult = mix(vec3<f32>(1.0), ambient_sample.rgb, stud_mask * 0.6);
        let blended_rgb = clamp(pbr_input.material.base_color.rgb * ambient_mult, vec3<f32>(0.0), vec3<f32>(1.0));
        pbr_input.material.base_color = vec4<f32>(
            blended_rgb,
            pbr_input.material.base_color.a
        );
        pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 1.0, stud_mask * 0.15);
    } else if (local_normal.y < -0.85 && fade > 0.0005) {
        let inlet_dir = local_view.xz / max(abs(local_view.y), 0.1);
        let inlet_vec = normalize(inlet_dir) * min(length(inlet_dir) * 0.2, 0.25);

        var inlet_uv = uv;
        var inlet_height = pow(textureSample(inlet_height_texture, inlet_height_texture_sampler, inlet_uv).r, 1.0 / 2.2);
        var inlet_layer = 0.0;
        for (var i = 0u; i < num_layers; i++) {
            if (inlet_height >= 1.0 - inlet_layer) {
                break;
            }
            inlet_uv -= inlet_vec / f32(num_layers);
            inlet_height = pow(textureSample(inlet_height_texture, inlet_height_texture_sampler, inlet_uv).r, 1.0 / 2.2);
            inlet_layer += layer_height;
        }

        let inlet_ambient_sample = textureSample(inlet_ambient_texture, inlet_ambient_texture_sampler, inlet_uv);
        let inlet_mask = (1.0 - smoothstep(0.55, 0.9, inlet_height)) * fade;
        let inlet_mult = mix(vec3<f32>(1.0), inlet_ambient_sample.rgb, inlet_mask);
        let blended_rgb = clamp(pbr_input.material.base_color.rgb * inlet_mult, vec3<f32>(0.0), vec3<f32>(1.0));
        pbr_input.material.base_color = vec4<f32>(
            blended_rgb,
            pbr_input.material.base_color.a
        );
    }

    #ifdef PREPASS_PIPELINE
        return deferred_output(in, pbr_input);
    #else
        var out: FragmentOutput;
        if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
            out.color = apply_pbr_lighting(pbr_input);
        } else {
            out.color = pbr_input.material.base_color;
        }

        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        return out;
    #endif
}
