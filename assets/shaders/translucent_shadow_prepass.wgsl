#import bevy_pbr::{
    pbr_bindings,
    prepass_io,
    mesh_bindings,
}
#import bevy_pbr::pbr_types::{
    STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT,
}

#ifdef BINDLESS
#import bevy_pbr::pbr_bindings::material_indices
#endif

const BAYER4: array<f32, 16> = array(
    0.0, 8.0, 2.0, 10.0,
    12.0, 4.0, 14.0, 6.0,
    3.0, 11.0, 1.0, 9.0,
    15.0, 7.0, 13.0, 5.0,
);

fn dither_shadow_discard(position: vec4<f32>, instance_index: u32) {
    let alpha =
#ifdef BINDLESS
        pbr_bindings::material_array[material_indices[mesh[instance_index].material_and_lightmap_bind_group_slot & 0xffffu].material].base_color.a;
#else
        pbr_bindings::material.base_color.a;
#endif
    if alpha >= 1.0 {
        return;
    }

    let ix = i32(position.x) & 3;
    let iy = i32(position.y) & 3;
    let threshold = (BAYER4[iy * 4 + ix] + 0.5) / 16.0;
    if threshold >= alpha {
        discard;
    }
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: prepass_io::VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> prepass_io::FragmentOutput {
    dither_shadow_discard(in.position, in.instance_index);
    var out: prepass_io::FragmentOutput;

#ifdef NORMAL_PREPASS
    let flags =
#ifdef BINDLESS
        pbr_bindings::material_array[material_indices[in.instance_index].material].flags;
#else
        pbr_bindings::material.flags;
#endif
    if (flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        let double_sided = (flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;
        let normal = (f32(!double_sided || is_front) * 2.0 - 1.0) * in.world_normal;
        out.normal = vec4(normal * 0.5 + vec3(0.5), 1.0);
    } else {
        out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    }
#endif

    return out;
}
#else
@fragment
fn fragment(
    in: prepass_io::VertexOutput,
) {
    dither_shadow_discard(in.position, in.instance_index);
}
#endif
