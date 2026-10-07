use naga::front::wgsl::parse_str;
use naga::valid::{Capabilities, ValidationFlags, Validator};
//this saves so much time!
//validate the shaders everywhere !


#[test]
fn clouds_shaders_parse_and_validate() {
    let common = include_str!("../src/client/sky/clouds/shaders/common.wgsl")
        .trim_start_matches('\u{FEFF}')
        .lines()
        .filter(|l| !l.contains("#define_import_path"))
        .collect::<Vec<_>>()
        .join("\n");

    let main = include_str!("../src/client/sky/clouds/shaders/clouds_compute.wgsl")
        .replace("#import bevy_open_world::common", &common)
        .replace("common::", "");

    let module = parse_str(&main).expect("WGSL parse failed");
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("WGSL validation failed");
}

#[test]
fn translucent_shadow_prepass_parses_and_validates() {
    //i love hardcoding things
    let stubs = r#"
#define_import_path bevy_pbr::pbr_types
const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 0u;
const STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32 = 0u;

#define_import_path bevy_pbr::pbr_bindings
struct StandardMaterial { base_color: vec4<f32>, flags: u32, }
struct StandardMaterialBindings { material: u32, }
@group(2) @binding(0) var<uniform> material: StandardMaterial;
@group(2) @binding(0) var<storage, read> material_indices: array<StandardMaterialBindings>;
@group(2) @binding(0) var<storage, read> material_array: array<StandardMaterial>;

#define_import_path bevy_pbr::mesh_bindings
struct MeshInstance { material_and_lightmap_bind_group_slot: u32, }
@group(1) @binding(0) var<storage, read> mesh: array<MeshInstance>;

#define_import_path bevy_pbr::prepass_io
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(7) instance_index: u32,
    @location(2) world_normal: vec3<f32>,
}
#ifdef PREPASS_FRAGMENT
struct FragmentOutput {
    @location(0) normal: vec4<f32>,
}
#endif
"#;
    let stubs = stubs
        .lines()
        .filter(|l| !l.contains("#define_import_path"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut in_import = false;
    let mut shader_lines = Vec::new();
    for line in include_str!("../assets/shaders/translucent_shadow_prepass.wgsl").lines() {
        let t = line.trim_start();
        if t.starts_with("#import") {
            in_import = t.contains('{');
            continue;
        }
        if in_import {
            if t.starts_with('}') {
                in_import = false;
            }
            continue;
        }
        shader_lines.push(line);
    }

    let mut merged = shader_lines;
    merged.extend(stubs.lines());
    let merged = merged
        .join("\n")
        .replace("pbr_bindings::", "")
        .replace("prepass_io::", "")
        .replace("mesh_bindings::", "")
        .replace("pbr_types::", "");

    for enabled in [&["PREPASS_FRAGMENT", "NORMAL_PREPASS"][..], &[][..]] {
        let preprocessed = preprocess(&merged, enabled);
        let module = parse_str(&preprocessed).expect("WGSL parse failed");
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("WGSL validation failed");
    }
}

#[test]
fn studs_shader_parses_and_validates() {
    let stubs = r#"
struct StandardMaterial {
    base_color: vec4<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
    flags: u32,
}
struct PbrInput {
    material: StandardMaterial,
    N: vec3<f32>,
}
struct ViewUniform {
    world_position: vec3<f32>,
}
@group(0) @binding(0) var<uniform> view: ViewUniform;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) instance_index: u32,
}
struct FragmentOutput {
    @location(0) color: vec4<f32>,
}
const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 1u;
fn get_world_from_local(instance_index: u32) -> mat4x4<f32> {
    return mat4x4<f32>(vec4<f32>(1.0, 0.0, 0.0, 0.0), vec4<f32>(0.0, 1.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, 1.0, 0.0), vec4<f32>(0.0, 0.0, 0.0, 1.0));
}
fn pbr_input_from_standard_material(in: VertexOutput, is_front: bool) -> PbrInput {
    let m = StandardMaterial(vec4<f32>(1.0), 0.25, 0.0, 0.5, 0u);
    return PbrInput(m, vec3<f32>(0.0, 1.0, 0.0));
}
fn alpha_discard(material: StandardMaterial, base_color: vec4<f32>) -> vec4<f32> {
    return base_color;
}
fn apply_pbr_lighting(input: PbrInput) -> vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
fn main_pass_post_lighting_processing(input: PbrInput, color: vec4<f32>) -> vec4<f32> {
    return color;
}
fn deferred_output(in: VertexOutput, input: PbrInput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    return out;
}
"#;

    let mut in_import = false;
    let mut shader_lines = Vec::new();
    for line in include_str!("../assets/shaders/studs.wgsl").lines() {
        let t = line.trim_start();
        if t.starts_with("#import") {
            in_import = t.contains('{');
            continue;
        }
        if in_import {
            if t.starts_with('}') {
                in_import = false;
            }
            continue;
        }
        shader_lines.push(line);
    }

    let mut merged = shader_lines;
    merged.extend(stubs.lines());
    let merged = merged
        .join("\n")
        .replace("#{MATERIAL_BIND_GROUP}", "1")
        .replace("mesh_functions::", "")
        .replace("pbr_types::", "")
        .replace("pbr_fragment::", "")
        .replace("pbr_functions::", "")
        .replace("mesh_view_bindings::", "")
        .replace("forward_io::", "")
        .replace("prepass_io::", "")
        .replace("pbr_deferred_functions::", "");

    for enabled in [&["PREPASS_PIPELINE"][..], &[][..]] {
        let preprocessed = preprocess(&merged, enabled);
        let module = parse_str(&preprocessed).expect("WGSL parse failed");
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("WGSL validation failed");
    }
}

fn preprocess(source: &str, enabled: &[&str]) -> String {
    let mut out = Vec::new();
    let mut stack: Vec<(bool, bool)> = Vec::new();
    let mut active = true;
    let mut taken = false;
    for line in source.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#ifdef ") {
            let cond = enabled.contains(&rest.trim());
            stack.push((active, taken));
            active = active && cond;
            taken = cond;
        } else if let Some(rest) = t.strip_prefix("#ifndef ") {
            let cond = !enabled.contains(&rest.trim());
            stack.push((active, taken));
            active = active && cond;
            taken = cond;
        } else if t.starts_with("#else") {
            active = stack.last().map(|(p, _)| *p).unwrap_or(true) && !taken;
            taken = true;
        } else if t.starts_with("#endif") {
            if let Some((parent_active, parent_taken)) = stack.pop() {
                active = parent_active;
                taken = parent_taken;
            }
        } else if active {
            out.push(line);
        }
    }
    out.join("\n")
}
