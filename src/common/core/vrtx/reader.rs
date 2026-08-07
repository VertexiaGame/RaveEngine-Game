use std::fs::File;
use std::io::{BufReader, Read};

use bevy::prelude::*;

use super::godot::parse_godot_vrtx;
use super::types::*;

fn read_u8(r: &mut impl Read) -> std::io::Result<u8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

fn read_u16(r: &mut impl Read) -> std::io::Result<u16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

fn read_u32(r: &mut impl Read) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_f32(r: &mut impl Read) -> std::io::Result<f32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}

fn read_vec3(r: &mut impl Read) -> std::io::Result<Vec3> {
    Ok(Vec3::new(read_f32(r)?, read_f32(r)?, read_f32(r)?))
}

fn read_quat(r: &mut impl Read) -> std::io::Result<Quat> {
    Ok(Quat::from_xyzw(
        read_f32(r)?,
        read_f32(r)?,
        read_f32(r)?,
        read_f32(r)?,
    ))
}

fn read_string_u16(r: &mut impl Read) -> std::io::Result<String> {
    let len = read_u16(r)? as usize;
    let mut bytes = vec![0u8; len];
    r.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn read_string_u32(r: &mut impl Read) -> std::io::Result<String> {
    let len = read_u32(r)? as usize;
    let mut bytes = vec![0u8; len];
    r.read_exact(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn read_transform(r: &mut impl Read) -> std::io::Result<Transform> {
    Ok(Transform {
        translation: read_vec3(r)?,
        rotation: read_quat(r)?,
        scale: read_vec3(r)?,
    })
}

fn read_lighting(r: &mut impl Read) -> std::io::Result<VrtxLighting> {
    let time_of_day = read_f32(r)?;
    let latitude = read_f32(r)?;
    let sun_angular_radius = read_f32(r)?;
    let moon_angular_radius = read_f32(r)?;

    let night_ambient = Color::Srgba(Srgba::new(
        read_f32(r)?,
        read_f32(r)?,
        read_f32(r)?,
        read_f32(r)?,
    ));

    let star_density = read_f32(r)?;
    let sun_illuminance = read_f32(r)?;
    let moon_illuminance = read_f32(r)?;
    let ambient_brightness = read_f32(r)?;
    let fog_density = read_f32(r)?;

    let volumetric_clouds = read_u8(r)? != 0;
    let cloud_render_scale = read_f32(r)?;
    let cloud_raymarch_steps = read_u32(r)?;
    let cloud_shadow_steps = read_u32(r)?;
    let planet_radius = read_f32(r)?;
    let cloud_bottom_height = read_f32(r)?;
    let cloud_top_height = read_f32(r)?;
    let cloud_coverage = read_f32(r)?;
    let cloud_density = read_f32(r)?;
    let cloud_detail_strength = read_f32(r)?;
    let cloud_base_edge_softness = read_f32(r)?;
    let cloud_bottom_softness = read_f32(r)?;
    let cloud_base_scale = read_f32(r)?;
    let cloud_detail_scale = read_f32(r)?;
    let cloud_shadow_step_size = read_f32(r)?;
    let cloud_shadow_step_multiply = read_f32(r)?;
    let cloud_forward_scattering_g = read_f32(r)?;
    let cloud_backward_scattering_g = read_f32(r)?;
    let cloud_scattering_lerp = read_f32(r)?;
    let cloud_min_transmittance = read_f32(r)?;
    let cloud_reprojection_strength = read_f32(r)?;
    let cloud_wind_velocity = read_vec3(r)?;

    Ok(VrtxLighting {
        time_of_day,
        latitude,
        sun_angular_radius,
        moon_angular_radius,
        night_ambient,
        star_density,
        sun_illuminance,
        moon_illuminance,
        ambient_brightness,
        fog_density,
        volumetric_clouds,
        cloud_render_scale,
        cloud_raymarch_steps,
        cloud_shadow_steps,
        planet_radius,
        cloud_bottom_height,
        cloud_top_height,
        cloud_coverage,
        cloud_density,
        cloud_detail_strength,
        cloud_base_edge_softness,
        cloud_bottom_softness,
        cloud_base_scale,
        cloud_detail_scale,
        cloud_shadow_step_size,
        cloud_shadow_step_multiply,
        cloud_forward_scattering_g,
        cloud_backward_scattering_g,
        cloud_scattering_lerp,
        cloud_min_transmittance,
        cloud_reprojection_strength,
        cloud_wind_velocity,
    })
}

fn read_brick(r: &mut impl Read, version: u32) -> std::io::Result<VrtxBrick> {
    let name = read_string_u16(r)?;
    let transform = read_transform(r)?;

    let shape = match read_u8(r)? {
        0 => crate::common::game::bricks::components::BrickShape::Block,
        1 => crate::common::game::bricks::components::BrickShape::Sphere,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid brick shape enum value",
            ));
        }
    };

    let color = Color::Srgba(Srgba::new(
        read_f32(r)?,
        read_f32(r)?,
        read_f32(r)?,
        read_f32(r)?,
    ));
    let physics_enabled = read_u8(r)? != 0;
    let bounciness = read_f32(r)?;

    let player_can_collide = if version >= 2 {
        read_u8(r)? != 0
    } else {
        true
    };

    let (friction, gravity_scale, mass) = if version >= 3 {
        (read_f32(r)?, read_f32(r)?, read_f32(r)?)
    } else {
        (0.3, 1.0, 1.0)
    };

    let show_studs = if version >= 6 {
        read_u8(r)? != 0
    } else {
        true
    };

    Ok(VrtxBrick {
        name,
        transform,
        shape,
        color,
        physics_enabled,
        bounciness,
        player_can_collide,
        friction,
        gravity_scale,
        mass,
        show_studs,
    })
}

fn read_script(r: &mut impl Read, version: u32) -> std::io::Result<VrtxScript> {
    let name = read_string_u16(r)?;
    let script_type = read_u8(r)?;
    let code = read_string_u32(r)?;

    let p_len = read_u16(r)? as usize;
    let parent_name = if p_len > 0 {
        let mut p_bytes = vec![0u8; p_len];
        r.read_exact(&mut p_bytes)?;
        Some(String::from_utf8(p_bytes).unwrap_or_default())
    } else {
        None
    };

    let enabled = if version >= 5 {
        read_u8(r)? != 0
    } else {
        true
    };

    Ok(VrtxScript {
        name,
        script_type,
        code,
        parent_name,
        enabled,
    })
}

fn read_v1_header(r: &mut impl Read) -> std::io::Result<(Vec3, VrtxSettings, Transform, u32)> {
    let gravity = read_vec3(r)?;
    let mut settings_bytes = [0u8; 3];
    r.read_exact(&mut settings_bytes)?;
    let settings = VrtxSettings {
        ssao: settings_bytes[0] != 0,
        contact_shadows: settings_bytes[1] != 0,
        bloom: settings_bytes[2] != 0,
    };

    let camera_transform = Transform {
        translation: read_vec3(r)?,
        rotation: read_quat(r)?,
        scale: Vec3::ONE,
    };

    let count = read_u32(r)?;
    Ok((gravity, settings, camera_transform, count))
}

pub fn load_from_file(path: &str) -> std::io::Result<VrtxFileState> {
    debug!("load_from_file: Attempting to open file: {}", path);
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    debug!("load_from_file: Read {} bytes from {}", data.len(), path);

    if data.len() >= 4 && &data[0..4] == b"VRTX" {
        let mut reader = BufReader::new(&data[4..]);
        let version = read_u32(&mut reader)?;
        debug!("load_from_file: VRTX format version is {}", version);

        if version > FORMAT_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported .VRTX file version",
            ));
        }

        let (gravity, settings, camera_transform, count) = if version >= 1 {
            read_v1_header(&mut reader)?
        } else if version == 0 {
            let gravity = read_vec3(&mut reader)?;
            let settings = VrtxSettings {
                ssao: false,
                contact_shadows: false,
                bloom: true,
            };
            let camera_transform =
                Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y);
            let count = read_u32(&mut reader)?;
            (gravity, settings, camera_transform, count)
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported .VRTX file version",
            ));
        };

        debug!("load_from_file: Expecting {} bricks", count);
        let mut bricks = Vec::with_capacity(count as usize);
        for _ in 0..count {
            bricks.push(read_brick(&mut reader, version)?);
        }

        let mut scripts = Vec::new();
        if version >= 4 {
            let script_count = read_u32(&mut reader)?;
            for _ in 0..script_count {
                scripts.push(read_script(&mut reader, version)?);
            }
        }

        let lighting = if version >= 7 {
            read_lighting(&mut reader)?
        } else {
            VrtxLighting::default()
        };

        debug!(
            "load_from_file: Successfully parsed {} bricks and {} scripts from standard VRTX file",
            bricks.len(),
            scripts.len()
        );
        Ok(VrtxFileState {
            version,
            gravity,
            settings,
            lighting,
            camera_transform,
            bricks,
            scripts,
        })
    } else if data.len() >= 4 && &data[0..4] == b"GCPF" {
        debug!("load_from_file: Detected legacy GCPF (Godot) file format");
        let decompressed = super::godot::decompress_gcpf_file(&data)?;
        debug!(
            "load_from_file: Successfully decompressed GCPF file into {} bytes",
            decompressed.len()
        );
        let parsed_state = parse_godot_vrtx(&decompressed)?;
        debug!(
            "load_from_file: Successfully parsed Godot VRTX map with {} bricks",
            parsed_state.bricks.len()
        );
        Ok(parsed_state)
    } else {
        error!("load_from_file: Unknown or invalid file signature");
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unknown or invalid file signature",
        ))
    }
}
