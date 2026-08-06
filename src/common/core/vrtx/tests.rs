use super::*;
use bevy::prelude::*;

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn temp_path(name: &str) -> String {
    let salt = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir()
        .join(format!("vrtx_test_{}_{}_{}.vrtx", std::process::id(), salt, name))
        .display()
        .to_string()
}

fn round_trip(state: &VrtxFileState) -> VrtxFileState {
    let path = temp_path("roundtrip");
    state.save_to_file(&path).expect("save should succeed");
    let loaded = VrtxFileState::load_from_file(&path).expect("load should succeed");
    let _ = std::fs::remove_file(&path);
    loaded
}

fn assert_f32_eq(a: f32, b: f32) {
    assert!(
        (a - b).abs() < 1e-4,
        "f32 mismatch: {a} != {b}"
    );
}

fn assert_vec3_eq(a: Vec3, b: Vec3) {
    assert_f32_eq(a.x, b.x);
    assert_f32_eq(a.y, b.y);
    assert_f32_eq(a.z, b.z);
}

fn assert_transform_eq(a: &Transform, b: &Transform) {
    assert_vec3_eq(a.translation, b.translation);
    assert_f32_eq(a.rotation.x, b.rotation.x);
    assert_f32_eq(a.rotation.y, b.rotation.y);
    assert_f32_eq(a.rotation.z, b.rotation.z);
    assert_f32_eq(a.rotation.w, b.rotation.w);
    assert_vec3_eq(a.scale, b.scale);
}

fn assert_lighting_eq(a: &VrtxLighting, b: &VrtxLighting) {
    assert_f32_eq(a.time_of_day, b.time_of_day);
    assert_f32_eq(a.latitude, b.latitude);
    assert_f32_eq(a.sun_angular_radius, b.sun_angular_radius);
    assert_f32_eq(a.moon_angular_radius, b.moon_angular_radius);
    assert_f32_eq(a.star_density, b.star_density);
    assert_f32_eq(a.sun_illuminance, b.sun_illuminance);
    assert_f32_eq(a.moon_illuminance, b.moon_illuminance);
    assert_f32_eq(a.ambient_brightness, b.ambient_brightness);
    assert_f32_eq(a.fog_density, b.fog_density);
    assert_eq!(a.volumetric_clouds, b.volumetric_clouds);
    assert_f32_eq(a.cloud_render_scale, b.cloud_render_scale);
    assert_eq!(a.cloud_raymarch_steps, b.cloud_raymarch_steps);
    assert_eq!(a.cloud_shadow_steps, b.cloud_shadow_steps);
    assert_f32_eq(a.planet_radius, b.planet_radius);
    assert_f32_eq(a.cloud_bottom_height, b.cloud_bottom_height);
    assert_f32_eq(a.cloud_top_height, b.cloud_top_height);
    assert_f32_eq(a.cloud_coverage, b.cloud_coverage);
    assert_f32_eq(a.cloud_density, b.cloud_density);
    assert_f32_eq(a.cloud_detail_strength, b.cloud_detail_strength);
    assert_f32_eq(a.cloud_base_edge_softness, b.cloud_base_edge_softness);
    assert_f32_eq(a.cloud_bottom_softness, b.cloud_bottom_softness);
    assert_f32_eq(a.cloud_base_scale, b.cloud_base_scale);
    assert_f32_eq(a.cloud_detail_scale, b.cloud_detail_scale);
    assert_f32_eq(a.cloud_shadow_step_size, b.cloud_shadow_step_size);
    assert_f32_eq(a.cloud_shadow_step_multiply, b.cloud_shadow_step_multiply);
    assert_f32_eq(a.cloud_forward_scattering_g, b.cloud_forward_scattering_g);
    assert_f32_eq(a.cloud_backward_scattering_g, b.cloud_backward_scattering_g);
    assert_f32_eq(a.cloud_scattering_lerp, b.cloud_scattering_lerp);
    assert_f32_eq(a.cloud_min_transmittance, b.cloud_min_transmittance);
    assert_f32_eq(a.cloud_reprojection_strength, b.cloud_reprojection_strength);
    assert_vec3_eq(a.cloud_wind_velocity, b.cloud_wind_velocity);
    assert_eq!(a.night_ambient, b.night_ambient);
}

fn sample_state() -> VrtxFileState {
    VrtxFileState {
        version: FORMAT_VERSION,
        gravity: Vec3::new(0.0, -52.332, 0.0),
        settings: VrtxSettings {
            ssao: true,
            contact_shadows: false,
            bloom: true,
        },
        lighting: VrtxLighting {
            time_of_day: 9.75,
            latitude: 33.0,
            sun_angular_radius: 0.04,
            moon_angular_radius: 0.025,
            night_ambient: Color::srgb(0.2, 0.3, 0.5),
            star_density: 0.3,
            sun_illuminance: 9500.0,
            moon_illuminance: 250.0,
            ambient_brightness: 1.4,
            fog_density: 0.6,
            volumetric_clouds: false,
            cloud_render_scale: 0.75,
            cloud_raymarch_steps: 24,
            cloud_shadow_steps: 12,
            planet_radius: 3_000_000.0,
            cloud_bottom_height: 800.0,
            cloud_top_height: 1800.0,
            cloud_coverage: 0.6,
            cloud_density: 0.05,
            cloud_detail_strength: 0.4,
            cloud_base_edge_softness: 0.2,
            cloud_bottom_softness: 0.5,
            cloud_base_scale: 2.0,
            cloud_detail_scale: 55.0,
            cloud_shadow_step_size: 20.0,
            cloud_shadow_step_multiply: 1.5,
            cloud_forward_scattering_g: 0.7,
            cloud_backward_scattering_g: -0.1,
            cloud_scattering_lerp: 0.6,
            cloud_min_transmittance: 0.2,
            cloud_reprojection_strength: 0.8,
            cloud_wind_velocity: Vec3::new(-3.0, 0.0, 5.0),
        },
        camera_transform: Transform::from_xyz(3.0, 7.0, -2.0)
            .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.5, -0.3, 0.1)),
        bricks: vec![
            VrtxBrick {
                name: "Baseplate".to_string(),
                transform: Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
                shape: crate::common::game::bricks::components::BrickShape::Block,
                color: Color::Srgba(Srgba::new(0.22, 0.52, 0.28, 1.0)),
                physics_enabled: false,
                bounciness: 0.1,
                player_can_collide: true,
                friction: 0.9,
                gravity_scale: 0.5,
                mass: 12.0,
                show_studs: true,
            },
            VrtxBrick {
                name: "Ball".to_string(),
                transform: Transform::from_xyz(1.0, 2.0, 3.0)
                    .with_rotation(Quat::from_rotation_y(1.25)),
                shape: crate::common::game::bricks::components::BrickShape::Sphere,
                color: Color::srgba(1.0, 0.1, 0.2, 0.5),
                physics_enabled: true,
                bounciness: 0.7,
                player_can_collide: false,
                friction: 0.2,
                gravity_scale: 2.0,
                mass: 3.5,
                show_studs: false,
            },
        ],
        scripts: vec![
            VrtxScript {
                name: "Main".to_string(),
                script_type: 0,
                code: "print('hello')".to_string(),
                parent_name: None,
                enabled: true,
            },
            VrtxScript {
                name: "ChildScript".to_string(),
                script_type: 1,
                code: "local a = 1".to_string(),
                parent_name: Some("Baseplate".to_string()),
                enabled: false,
            },
            VrtxScript {
                name: "Shared".to_string(),
                script_type: 2,
                code: "return {} ".repeat(50),
                parent_name: Some("Ball".to_string()),
                enabled: true,
            },
        ],
    }
}

#[test]
fn bricks_round_trip_preserves_all_properties() {
    let state = sample_state();
    let loaded = round_trip(&state);

    assert_eq!(loaded.bricks.len(), 2);
    let (b0, b1) = (&loaded.bricks[0], &loaded.bricks[1]);
    assert_eq!(b0.name, "Baseplate");
    assert_transform_eq(&b0.transform, &state.bricks[0].transform);
    assert_eq!(b0.shape, crate::common::game::bricks::components::BrickShape::Block);
    assert_eq!(b0.color, state.bricks[0].color);
    assert_eq!(b0.physics_enabled, false);
    assert_f32_eq(b0.bounciness, 0.1);
    assert_eq!(b0.player_can_collide, true);
    assert_f32_eq(b0.friction, 0.9);
    assert_f32_eq(b0.gravity_scale, 0.5);
    assert_f32_eq(b0.mass, 12.0);
    assert_eq!(b0.show_studs, true);

    assert_eq!(b1.name, "Ball");
    assert_eq!(b1.shape, crate::common::game::bricks::components::BrickShape::Sphere);
    assert_transform_eq(&b1.transform, &state.bricks[1].transform);
    assert_eq!(b1.color, state.bricks[1].color);
    assert_eq!(b1.physics_enabled, true);
    assert_f32_eq(b1.bounciness, 0.7);
    assert_eq!(b1.player_can_collide, false);
    assert_f32_eq(b1.friction, 0.2);
    assert_f32_eq(b1.gravity_scale, 2.0);
    assert_f32_eq(b1.mass, 3.5);
    assert_eq!(b1.show_studs, false);
}

#[test]
fn scripts_round_trip_preserves_all_properties() {
    let state = sample_state();
    let loaded = round_trip(&state);

    assert_eq!(loaded.scripts.len(), 3);
    assert_eq!(loaded.scripts[0].name, "Main");
    assert_eq!(loaded.scripts[0].script_type, 0);
    assert_eq!(loaded.scripts[0].code, "print('hello')");
    assert_eq!(loaded.scripts[0].parent_name, None);
    assert_eq!(loaded.scripts[0].enabled, true);

    assert_eq!(loaded.scripts[1].name, "ChildScript");
    assert_eq!(loaded.scripts[1].script_type, 1);
    assert_eq!(loaded.scripts[1].parent_name, Some("Baseplate".to_string()));
    assert_eq!(loaded.scripts[1].enabled, false);

    assert_eq!(loaded.scripts[2].name, "Shared");
    assert_eq!(loaded.scripts[2].script_type, 2);
    assert_eq!(loaded.scripts[2].parent_name, Some("Ball".to_string()));
    assert!(loaded.scripts[2].code.ends_with(' '));
}

#[test]
fn settings_gravity_and_camera_round_trip() {
    let state = sample_state();
    let loaded = round_trip(&state);

    assert_eq!(loaded.version, FORMAT_VERSION);
    assert_eq!(loaded.settings.ssao, true);
    assert_eq!(loaded.settings.contact_shadows, false);
    assert_eq!(loaded.settings.bloom, true);
    assert_vec3_eq(loaded.gravity, state.gravity);
    assert_transform_eq(&loaded.camera_transform, &state.camera_transform);
}

#[test]
fn lighting_round_trip_preserves_every_cloud_property() {
    let state = sample_state();
    let loaded = round_trip(&state);
    assert_lighting_eq(&loaded.lighting, &state.lighting);
}

#[test]
fn lighting_round_trip_with_defaults_is_stable() {
    let mut state = sample_state();
    state.lighting = VrtxLighting::default();
    let loaded = round_trip(&state);
    assert_lighting_eq(&loaded.lighting, &VrtxLighting::default());
}

#[test]
fn empty_state_round_trip() {
    let state = VrtxFileState {
        version: FORMAT_VERSION,
        gravity: Vec3::ZERO,
        settings: VrtxSettings {
            ssao: false,
            contact_shadows: false,
            bloom: false,
        },
        lighting: VrtxLighting::default(),
        camera_transform: Transform::IDENTITY,
        bricks: Vec::new(),
        scripts: Vec::new(),
    };
    let loaded = round_trip(&state);
    assert_eq!(loaded.version, FORMAT_VERSION);
    assert!(loaded.bricks.is_empty());
    assert!(loaded.scripts.is_empty());
    assert_eq!(loaded.settings.bloom, false);
}

fn write_u8_to(v: &mut Vec<u8>, b: u8) {
    v.push(b);
}

fn write_u16_to(v: &mut Vec<u8>, n: u16) {
    v.extend_from_slice(&n.to_le_bytes());
}

fn write_u32_to(v: &mut Vec<u8>, n: u32) {
    v.extend_from_slice(&n.to_le_bytes());
}

fn write_f32_to(v: &mut Vec<u8>, f: f32) {
    v.extend_from_slice(&f.to_le_bytes());
}

fn write_str_u16_to(v: &mut Vec<u8>, s: &str) {
    write_u16_to(v, s.len() as u16);
    v.extend_from_slice(s.as_bytes());
}

fn write_str_u32_to(v: &mut Vec<u8>, s: &str) {
    write_u32_to(v, s.len() as u32);
    v.extend_from_slice(s.as_bytes());
}

fn write_vec3_to(v: &mut Vec<u8>, x: f32, y: f32, z: f32) {
    write_f32_to(v, x);
    write_f32_to(v, y);
    write_f32_to(v, z);
}

fn write_quat_to(v: &mut Vec<u8>, q: Quat) {
    write_f32_to(v, q.x);
    write_f32_to(v, q.y);
    write_f32_to(v, q.z);
    write_f32_to(v, q.w);
}

fn write_brick_to(v: &mut Vec<u8>, name: &str, version: u32) {
    write_str_u16_to(v, name);
    write_vec3_to(v, 1.0, 2.0, 3.0);
    write_quat_to(v, Quat::IDENTITY);
    write_vec3_to(v, 1.0, 1.0, 1.0);
    write_u8_to(v, 0);
    write_f32_to(v, 0.5);
    write_f32_to(v, 0.5);
    write_f32_to(v, 0.5);
    write_f32_to(v, 1.0);
    write_u8_to(v, 1);
    write_f32_to(v, 0.4);
    if version >= 2 {
        write_u8_to(v, 1);
    }
    if version >= 3 {
        write_f32_to(v, 0.6);
        write_f32_to(v, 1.5);
        write_f32_to(v, 7.0);
    }
    if version >= 6 {
        write_u8_to(v, 0);
    }
}

fn load_bytes(bytes: Vec<u8>) -> std::io::Result<VrtxFileState> {
    let path = temp_path("bytes");
    std::fs::write(&path, &bytes).expect("write temp file");
    let result = VrtxFileState::load_from_file(&path);
    let _ = std::fs::remove_file(&path);
    result
}

#[test]
fn version_zero_file_loads_with_expected_defaults() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VRTX");
    write_u32_to(&mut bytes, 0);
    write_vec3_to(&mut bytes, 0.0, -52.332, 0.0);
    write_u32_to(&mut bytes, 0);

    let state = load_bytes(bytes).expect("v0 should load");
    assert_eq!(state.version, 0);
    assert_vec3_eq(state.gravity, Vec3::new(0.0, -52.332, 0.0));
    assert_eq!(state.settings.ssao, false);
    assert_eq!(state.settings.contact_shadows, false);
    assert_eq!(state.settings.bloom, true);
    assert!(state.bricks.is_empty());
    assert_lighting_eq(&state.lighting, &VrtxLighting::default());
}

#[test]
fn version_three_file_applies_field_gates_and_defaults() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VRTX");
    write_u32_to(&mut bytes, 3);
    write_vec3_to(&mut bytes, 0.0, -1.0, 0.0);
    write_u8_to(&mut bytes, 1);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 1);
    write_vec3_to(&mut bytes, 10.0, 5.0, 0.0);
    write_quat_to(&mut bytes, Quat::IDENTITY);
    write_u32_to(&mut bytes, 1);
    write_brick_to(&mut bytes, "LegacyBrick", 3);

    let state = load_bytes(bytes).expect("v3 should load");
    assert_eq!(state.bricks.len(), 1);
    let brick = &state.bricks[0];
    assert_eq!(brick.name, "LegacyBrick");
    assert_eq!(brick.show_studs, true, "show_studs defaults true before v6");
    assert_eq!(brick.player_can_collide, true);
    assert_f32_eq(brick.friction, 0.6);
    assert_f32_eq(brick.gravity_scale, 1.5);
    assert_f32_eq(brick.mass, 7.0);
    assert_lighting_eq(&state.lighting, &VrtxLighting::default());
}

#[test]
fn version_five_script_enabled_byte_is_read() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VRTX");
    write_u32_to(&mut bytes, 5);
    write_vec3_to(&mut bytes, 0.0, -1.0, 0.0);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 1);
    write_vec3_to(&mut bytes, 0.0, 0.0, 0.0);
    write_quat_to(&mut bytes, Quat::IDENTITY);
    write_u32_to(&mut bytes, 0);
    write_u32_to(&mut bytes, 1);
    write_str_u16_to(&mut bytes, "OldScript");
    write_u8_to(&mut bytes, 0);
    write_str_u32_to(&mut bytes, "print(1)");
    write_u16_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 0);

    let state = load_bytes(bytes).expect("v5 should load");
    assert_eq!(state.scripts.len(), 1);
    assert_eq!(state.scripts[0].name, "OldScript");
    assert_eq!(state.scripts[0].enabled, false);
}

#[test]
fn version_six_file_keeps_show_studs() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VRTX");
    write_u32_to(&mut bytes, 6);
    write_vec3_to(&mut bytes, 0.0, -1.0, 0.0);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 1);
    write_vec3_to(&mut bytes, 0.0, 0.0, 0.0);
    write_quat_to(&mut bytes, Quat::IDENTITY);
    write_u32_to(&mut bytes, 1);
    write_brick_to(&mut bytes, "Studless", 6);
    write_u32_to(&mut bytes, 0);

    let state = load_bytes(bytes).expect("v6 should load");
    assert_eq!(state.bricks[0].show_studs, false);
    assert_lighting_eq(&state.lighting, &VrtxLighting::default());
}

#[test]
fn rejects_invalid_signature() {
    let err = load_bytes(vec![b'N', b'O', b'P', b'E', 0, 0, 0, 0]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn rejects_unsupported_version() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VRTX");
    write_u32_to(&mut bytes, 99);
    let err = load_bytes(bytes).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn rejects_truncated_v7_file() {
    let state = sample_state();
    let path = temp_path("truncated");
    state.save_to_file(&path).expect("save should succeed");
    let full = std::fs::read(&path).expect("read back");
    let _ = std::fs::remove_file(&path);

    let truncated = full[..full.len() - 8].to_vec();
    let err = load_bytes(truncated).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
}

#[test]
fn rejects_invalid_brick_shape_enum() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"VRTX");
    write_u32_to(&mut bytes, 6);
    write_vec3_to(&mut bytes, 0.0, -1.0, 0.0);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 0);
    write_u8_to(&mut bytes, 1);
    write_vec3_to(&mut bytes, 0.0, 0.0, 0.0);
    write_quat_to(&mut bytes, Quat::IDENTITY);
    write_u32_to(&mut bytes, 1);
    write_str_u16_to(&mut bytes, "Bad");
    write_vec3_to(&mut bytes, 0.0, 0.0, 0.0);
    write_quat_to(&mut bytes, Quat::IDENTITY);
    write_vec3_to(&mut bytes, 1.0, 1.0, 1.0);
    write_u8_to(&mut bytes, 42);
    let err = load_bytes(bytes).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn godot_parser_handles_primitive_variants() {
    use super::godot::{GodotParser, GodotVariant};

    let mut bool_bytes = Vec::new();
    write_u32_to(&mut bool_bytes, 1);
    write_u32_to(&mut bool_bytes, 1);
    let mut p = GodotParser::new(&bool_bytes);
    assert!(matches!(p.parse_variant().unwrap(), GodotVariant::Bool(true)));

    let mut int_bytes = Vec::new();
    write_u32_to(&mut int_bytes, 2);
    write_u32_to(&mut int_bytes, 42);
    let mut p = GodotParser::new(&int_bytes);
    assert!(matches!(p.parse_variant().unwrap(), GodotVariant::Int(42)));

    let mut float_bytes = Vec::new();
    write_u32_to(&mut float_bytes, 3);
    write_f32_to(&mut float_bytes, 3.5);
    let mut p = GodotParser::new(&float_bytes);
    assert!(matches!(p.parse_variant().unwrap(), GodotVariant::Float(f) if (f - 3.5).abs() < 1e-6));

    let mut str_bytes = Vec::new();
    write_u32_to(&mut str_bytes, 4);
    write_u32_to(&mut str_bytes, 2);
    str_bytes.extend_from_slice(b"ab");
    str_bytes.extend_from_slice(&[0, 0]);
    let mut p = GodotParser::new(&str_bytes);
    assert!(matches!(p.parse_variant().unwrap(), GodotVariant::String(s) if s == "ab"));

    let mut vec3_bytes = Vec::new();
    write_u32_to(&mut vec3_bytes, 9);
    write_f32_to(&mut vec3_bytes, 1.0);
    write_f32_to(&mut vec3_bytes, 2.0);
    write_f32_to(&mut vec3_bytes, 3.0);
    let mut p = GodotParser::new(&vec3_bytes);
    assert!(matches!(p.parse_variant().unwrap(), GodotVariant::Vector3(v) if v == Vec3::new(1.0, 2.0, 3.0)));

    let mut color_bytes = Vec::new();
    write_u32_to(&mut color_bytes, 20);
    write_f32_to(&mut color_bytes, 0.1);
    write_f32_to(&mut color_bytes, 0.2);
    write_f32_to(&mut color_bytes, 0.3);
    write_f32_to(&mut color_bytes, 1.0);
    let mut p = GodotParser::new(&color_bytes);
    assert!(matches!(p.parse_variant().unwrap(), GodotVariant::Color(_)));

    let mut bad_bytes = Vec::new();
    write_u32_to(&mut bad_bytes, 999);
    let mut p = GodotParser::new(&bad_bytes);
    assert!(p.parse_variant().is_err());
}

#[test]
fn godot_parser_handles_dictionary_and_array() {
    use super::godot::{GodotParser, GodotVariant};

    let mut arr_bytes = Vec::new();
    write_u32_to(&mut arr_bytes, 28);
    write_u32_to(&mut arr_bytes, 2);
    write_u32_to(&mut arr_bytes, 1);
    write_u32_to(&mut arr_bytes, 0);
    write_u32_to(&mut arr_bytes, 1);
    write_u32_to(&mut arr_bytes, 7);
    let mut p = GodotParser::new(&arr_bytes);
    let parsed = p.parse_variant().unwrap();
    assert!(matches!(&parsed, GodotVariant::Array(a) if a.len() == 2));

    let mut dict_bytes = Vec::new();
    write_u32_to(&mut dict_bytes, 27);
    write_u32_to(&mut dict_bytes, 1);
    write_u32_to(&mut dict_bytes, 4);
    write_u32_to(&mut dict_bytes, 1);
    dict_bytes.extend_from_slice(b"v");
    dict_bytes.extend_from_slice(&[0, 0, 0]);
    write_u32_to(&mut dict_bytes, 2);
    write_u32_to(&mut dict_bytes, 3);
    let mut p = GodotParser::new(&dict_bytes);
    let parsed = p.parse_variant().unwrap();
    assert!(matches!(&parsed, GodotVariant::Dictionary(d) if d.get("v") == Some(&GodotVariant::Int(3))));
}

#[test]
fn gcpf_rejects_invalid_magic() {
    let err = super::godot::decompress_gcpf_file(&[0u8; 16]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn gcpf_rejects_short_file() {
    let err = super::godot::decompress_gcpf_file(b"GCPF").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn gcpf_rejects_zero_block_size() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GCPF");
    write_u32_to(&mut bytes, 0);
    write_u32_to(&mut bytes, 0);
    write_u32_to(&mut bytes, 100);
    let err = super::godot::decompress_gcpf_file(&bytes).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
