#import bevy_pbr::{
    mesh_functions,
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
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
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var stud_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var inlet_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var inlet_sampler: sampler;

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

    let uv = world_size_coord.xz / 0.28 + scale.xz * vec2<f32>(2.0, 1.0);

    let local_normal = rot_t * in.world_normal;

    if (local_normal.y > 0.9) {
        let stud_color = textureSample(stud_texture, stud_sampler, uv);
        let shading = stud_color.rgb - vec3<f32>(0.5);
        let blended_rgb = pbr_input.material.base_color.rgb + shading * 0.5;
        pbr_input.material.base_color = vec4<f32>(
            mix(pbr_input.material.base_color.rgb, clamp(blended_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), stud_color.a),
            pbr_input.material.base_color.a
        );
    } else if (local_normal.y < -0.9) {
        let inlet_color = textureSample(inlet_texture, inlet_sampler, uv);
        let shading = inlet_color.rgb - vec3<f32>(0.5);
        let blended_rgb = pbr_input.material.base_color.rgb + shading * 0.5;
        pbr_input.material.base_color = vec4<f32>(
            mix(pbr_input.material.base_color.rgb, clamp(blended_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), inlet_color.a),
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