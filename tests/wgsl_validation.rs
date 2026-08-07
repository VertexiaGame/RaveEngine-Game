use naga::front::wgsl::parse_str;
use naga::valid::{Capabilities, ValidationFlags, Validator};

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
