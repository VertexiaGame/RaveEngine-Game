use std::fs::File;
use std::io::{BufWriter, Write};

use bevy::prelude::*;

use super::types::*;

fn write_u8(w: &mut impl Write, v: u8) -> std::io::Result<()> {
    w.write_all(&[v])
}

fn write_u16(w: &mut impl Write, v: u16) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_u32(w: &mut impl Write, v: u32) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_f32(w: &mut impl Write, v: f32) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_vec3(w: &mut impl Write, v: Vec3) -> std::io::Result<()> {
    write_f32(w, v.x)?;
    write_f32(w, v.y)?;
    write_f32(w, v.z)
}

fn write_quat(w: &mut impl Write, q: Quat) -> std::io::Result<()> {
    write_f32(w, q.x)?;
    write_f32(w, q.y)?;
    write_f32(w, q.z)?;
    write_f32(w, q.w)
}

fn write_string_u16(w: &mut impl Write, s: &str) -> std::io::Result<()> {
    let bytes = s.as_bytes();
    write_u16(w, bytes.len() as u16)?;
    w.write_all(bytes)
}

fn write_string_u32(w: &mut impl Write, s: &str) -> std::io::Result<()> {
    let bytes = s.as_bytes();
    write_u32(w, bytes.len() as u32)?;
    w.write_all(bytes)
}

fn write_transform(w: &mut impl Write, transform: &Transform) -> std::io::Result<()> {
    write_vec3(w, transform.translation)?;
    write_quat(w, transform.rotation)?;
    write_vec3(w, transform.scale)
}

fn write_settings(w: &mut impl Write, settings: &VrtxSettings) -> std::io::Result<()> {
    w.write_all(&[
        if settings.ssao { 1 } else { 0 },
        if settings.contact_shadows { 1 } else { 0 },
        if settings.bloom { 1 } else { 0 },
    ])
}

fn write_lighting(w: &mut impl Write, lighting: &VrtxLighting) -> std::io::Result<()> {
    write_f32(w, lighting.time_of_day)?;
    write_f32(w, lighting.latitude)?;
    write_f32(w, lighting.sun_angular_radius)?;
    write_f32(w, lighting.moon_angular_radius)?;

    let srgba = lighting.night_ambient.to_srgba();
    write_f32(w, srgba.red)?;
    write_f32(w, srgba.green)?;
    write_f32(w, srgba.blue)?;
    write_f32(w, srgba.alpha)?;

    write_f32(w, lighting.star_density)?;
    write_f32(w, lighting.sun_illuminance)?;
    write_f32(w, lighting.moon_illuminance)?;
    write_f32(w, lighting.ambient_brightness)?;
    write_f32(w, lighting.fog_density)?;

    write_u8(w, if lighting.volumetric_clouds { 1 } else { 0 })?;
    write_f32(w, lighting.cloud_render_scale)?;
    write_u32(w, lighting.cloud_raymarch_steps)?;
    write_u32(w, lighting.cloud_shadow_steps)?;
    write_f32(w, lighting.planet_radius)?;
    write_f32(w, lighting.cloud_bottom_height)?;
    write_f32(w, lighting.cloud_top_height)?;
    write_f32(w, lighting.cloud_coverage)?;
    write_f32(w, lighting.cloud_density)?;
    write_f32(w, lighting.cloud_detail_strength)?;
    write_f32(w, lighting.cloud_base_edge_softness)?;
    write_f32(w, lighting.cloud_bottom_softness)?;
    write_f32(w, lighting.cloud_base_scale)?;
    write_f32(w, lighting.cloud_detail_scale)?;
    write_f32(w, lighting.cloud_shadow_step_size)?;
    write_f32(w, lighting.cloud_shadow_step_multiply)?;
    write_f32(w, lighting.cloud_forward_scattering_g)?;
    write_f32(w, lighting.cloud_backward_scattering_g)?;
    write_f32(w, lighting.cloud_scattering_lerp)?;
    write_f32(w, lighting.cloud_min_transmittance)?;
    write_f32(w, lighting.cloud_reprojection_strength)?;
    write_vec3(w, lighting.cloud_wind_velocity)
}

fn write_brick(w: &mut impl Write, brick: &VrtxBrick, version: u32) -> std::io::Result<()> {
    write_string_u16(w, &brick.name)?;
    write_transform(w, &brick.transform)?;

    let shape_val = match brick.shape {
        crate::common::game::bricks::components::BrickShape::Block => 0u8,
        crate::common::game::bricks::components::BrickShape::Sphere => 1u8,
    };
    write_u8(w, shape_val)?;

    let srgba = brick.color.to_srgba();
    write_f32(w, srgba.red)?;
    write_f32(w, srgba.green)?;
    write_f32(w, srgba.blue)?;
    write_f32(w, srgba.alpha)?;

    write_u8(w, if brick.physics_enabled { 1 } else { 0 })?;
    write_f32(w, brick.bounciness)?;
    write_u8(w, if brick.player_can_collide { 1 } else { 0 })?;
    write_f32(w, brick.friction)?;
    write_f32(w, brick.gravity_scale)?;
    write_f32(w, brick.mass)?;

    if version >= 6 {
        write_u8(w, if brick.show_studs { 1 } else { 0 })?;
    }
    Ok(())
}

fn write_script(w: &mut impl Write, script: &VrtxScript, version: u32) -> std::io::Result<()> {
    write_string_u16(w, &script.name)?;
    write_u8(w, script.script_type)?;
    write_string_u32(w, &script.code)?;

    if let Some(ref parent) = script.parent_name {
        write_string_u16(w, parent)?;
    } else {
        write_u16(w, 0)?;
    }

    if version >= 5 {
        write_u8(w, if script.enabled { 1 } else { 0 })?;
    }
    Ok(())
}

pub fn save_to_file(state: &VrtxFileState, path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(b"VRTX")?;
    write_u32(&mut writer, state.version)?;

    write_vec3(&mut writer, state.gravity)?;
    write_settings(&mut writer, &state.settings)?;
    write_vec3(&mut writer, state.camera_transform.translation)?;
    write_quat(&mut writer, state.camera_transform.rotation)?;

    write_u32(&mut writer, state.bricks.len() as u32)?;
    for brick in &state.bricks {
        write_brick(&mut writer, brick, state.version)?;
    }

    if state.version >= 4 {
        write_u32(&mut writer, state.scripts.len() as u32)?;
        for script in &state.scripts {
            write_script(&mut writer, script, state.version)?;
        }
    }

    if state.version >= 7 {
        write_lighting(&mut writer, &state.lighting)?;
    }

    writer.flush()?;
    Ok(())
}
