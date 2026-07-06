use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct VrtxBrick {
    pub name: String,
    pub transform: Transform,
    pub shape: crate::common::bricks::components::BrickShape,
    pub color: Color,
    pub physics_enabled: bool,
    pub bounciness: f32,
}

#[derive(Debug, Clone)]
pub struct VrtxSettings {
    pub ssao: bool,
    pub contact_shadows: bool,
    pub bloom: bool,
}

#[derive(Debug, Clone)]
pub struct VrtxFileState {
    pub version: u32,
    pub gravity: Vec3,
    pub settings: VrtxSettings,
    pub camera_transform: Transform,
    pub bricks: Vec<VrtxBrick>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum GodotVariant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vector2(Vec2),
    Vector3(Vec3),
    Color(Color),
    Dictionary(std::collections::HashMap<String, GodotVariant>),
    Array(Vec<GodotVariant>),
}

struct GodotParser<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> GodotParser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        if self.offset + 4 > self.data.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF"));
        }
        let val = u32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(val)
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        if self.offset + 4 > self.data.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF"));
        }
        let val = f32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(val)
    }

    fn read_f64(&mut self) -> std::io::Result<f64> {
        if self.offset + 8 > self.data.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF"));
        }
        let val = f64::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
            self.data[self.offset + 4],
            self.data[self.offset + 5],
            self.data[self.offset + 6],
            self.data[self.offset + 7],
        ]);
        self.offset += 8;
        Ok(val)
    }

    fn read_bytes(&mut self, len: usize) -> std::io::Result<&'a [u8]> {
        if self.offset + len > self.data.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Unexpected EOF"));
        }
        let slice = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(slice)
    }

    fn parse_variant(&mut self) -> std::io::Result<GodotVariant> {
        let start_offset = self.offset;
        let type_header = self.read_u32()?;
        let type_id = type_header & 0xFFFF;
        let flags = (type_header >> 16) & 0xFF;
        let is_64 = flags == 1;

        trace!("parse_variant at offset {}: type_id={}, flags={}, is_64={}", start_offset, type_id, flags, is_64);

        let var = match type_id {
            0 | 25 | 26 => Ok(GodotVariant::Nil),
            1 => {
                let val = self.read_u32()?;
                Ok(GodotVariant::Bool(val != 0))
            }
            2 => {
                let bytes_to_read = if is_64 { 8 } else { 4 };
                let bytes = self.read_bytes(bytes_to_read)?;
                if is_64 {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(bytes);
                    Ok(GodotVariant::Int(i64::from_le_bytes(b)))
                } else {
                    let mut b = [0u8; 4];
                    b.copy_from_slice(bytes);
                    Ok(GodotVariant::Int(i32::from_le_bytes(b) as i64))
                }
            }
            3 => {
                if is_64 {
                    let val = self.read_f64()?;
                    Ok(GodotVariant::Float(val))
                } else {
                    let val = self.read_f32()?;
                    Ok(GodotVariant::Float(val as f64))
                }
            }
            4 | 21 | 22 => {
                let len = self.read_u32()? as usize;
                let padded_len = (len + 3) & !3;
                let str_bytes = self.read_bytes(len)?;
                let _padding = self.read_bytes(padded_len - len)?;
                let string = String::from_utf8_lossy(str_bytes).into_owned();
                Ok(GodotVariant::String(string))
            }
            5 => {
                let x = if is_64 { self.read_f64()? as f32 } else { self.read_f32()? };
                let y = if is_64 { self.read_f64()? as f32 } else { self.read_f32()? };
                Ok(GodotVariant::Vector2(Vec2::new(x, y)))
            }
            6 => {
                let len = if is_64 { 32 } else { 16 };
                let _bytes = self.read_bytes(len)?;
                Ok(GodotVariant::Nil)
            }
            7 => {
                let _bytes = self.read_bytes(8)?;
                Ok(GodotVariant::Nil)
            }
            8 => {
                let _bytes = self.read_bytes(16)?;
                Ok(GodotVariant::Nil)
            }
            9 => {
                let x = if is_64 { self.read_f64()? as f32 } else { self.read_f32()? };
                let y = if is_64 { self.read_f64()? as f32 } else { self.read_f32()? };
                let z = if is_64 { self.read_f64()? as f32 } else { self.read_f32()? };
                Ok(GodotVariant::Vector3(Vec3::new(x, y, z)))
            }
            10 => {
                let _bytes = self.read_bytes(12)?;
                Ok(GodotVariant::Nil)
            }
            11 => {
                let _bytes = if is_64 { self.read_bytes(32)? } else { self.read_bytes(16)? };
                Ok(GodotVariant::Nil)
            }
            12 => {
                let _bytes = if is_64 { self.read_bytes(32)? } else { self.read_bytes(16)? };
                Ok(GodotVariant::Nil)
            }
            13 => {
                let _bytes = if is_64 { self.read_bytes(32)? } else { self.read_bytes(16)? };
                Ok(GodotVariant::Nil)
            }
            14 => {
                let _bytes = if is_64 { self.read_bytes(48)? } else { self.read_bytes(24)? };
                Ok(GodotVariant::Nil)
            }
            15 => {
                let _bytes = if is_64 { self.read_bytes(72)? } else { self.read_bytes(36)? };
                Ok(GodotVariant::Nil)
            }
            16 => {
                let _bytes = if is_64 { self.read_bytes(96)? } else { self.read_bytes(48)? };
                Ok(GodotVariant::Nil)
            }
            17 => {
                let _bytes = if is_64 { self.read_bytes(128)? } else { self.read_bytes(64)? };
                Ok(GodotVariant::Nil)
            }
            18 => {
                let len = if is_64 { 96 } else { 48 };
                let _bytes = self.read_bytes(len)?;
                Ok(GodotVariant::Nil)
            }
            19 => {
                let len = if is_64 { 128 } else { 64 };
                let _bytes = self.read_bytes(len)?;
                Ok(GodotVariant::Nil)
            }
            20 => {
                let r = self.read_f32()?;
                let g = self.read_f32()?;
                let b = self.read_f32()?;
                let a = self.read_f32()?;
                Ok(GodotVariant::Color(Color::Srgba(Srgba::new(r, g, b, a))))
            }
            23 => {
                let _bytes = self.read_bytes(8)?;
                Ok(GodotVariant::Nil)
            }
            24 => {
                let object_type = self.read_u32()?;
                if object_type == 1 {
                    let len = self.read_u32()? as usize;
                    let padded_len = (len + 3) & !3;
                    let _class_name = self.read_bytes(len)?;
                    let _padding = self.read_bytes(padded_len - len)?;
                    let prop_count = self.read_u32()?;
                    for _ in 0..prop_count {
                        let _name = self.parse_variant()?;
                        let _val = self.parse_variant()?;
                    }
                } else if object_type == 2 {
                    let _bytes = self.read_bytes(8)?;
                }
                Ok(GodotVariant::Nil)
            }
            27 => {
                let count_header = self.read_u32()?;
                let count = count_header & 0x7FFFFFFF;
                trace!("parse_variant at offset {}: parsing dictionary with {} elements", start_offset, count);
                let mut dict = std::collections::HashMap::new();
                for i in 0..count {
                    let key_var = self.parse_variant()?;
                    let val_var = self.parse_variant()?;
                    trace!("parse_variant dictionary element {}: key={:?}, val_type={:?}", i, key_var, val_var);
                    if let GodotVariant::String(key_str) = key_var {
                        dict.insert(key_str, val_var);
                    }
                }
                Ok(GodotVariant::Dictionary(dict))
            }
            28 => {
                let count_header = self.read_u32()?;
                let count = count_header & 0x7FFFFFFF;
                trace!("parse_variant at offset {}: parsing array with {} elements", start_offset, count);
                let mut arr = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let val_var = self.parse_variant()?;
                    arr.push(val_var);
                }
                Ok(GodotVariant::Array(arr))
            }
            _ => {
                error!("parse_variant at offset {}: Unsupported Godot variant type: {}", start_offset, type_id);
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unsupported Godot variant type: {}", type_id),
                ))
            }
        };

        if let Ok(ref _value) = var {
            trace!("parse_variant at offset {} successfully parsed", start_offset);
        }
        var
    }
}

fn decompress_gcpf_file(data: &[u8]) -> std::io::Result<Vec<u8>> {
    if data.len() < 16 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "GCPF: File too short"));
    }
    if &data[0..4] != b"GCPF" {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "GCPF: Invalid magic"));
    }

    let comp_mode = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let block_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let uncompressed_size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

    debug!("GCPF decompress: mode={}, block_size={}, uncompressed_size={}", comp_mode, block_size, uncompressed_size);

    if block_size == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "GCPF: Block size is zero"));
    }

    let num_blocks = (uncompressed_size + block_size - 1) / block_size;
    let header_size = 16 + num_blocks * 4;
    debug!("GCPF decompress: num_blocks={}, header_size={}", num_blocks, header_size);

    if data.len() < header_size {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "GCPF: Header size exceeds file length"));
    }

    let mut block_sizes = Vec::with_capacity(num_blocks);
    for i in 0..num_blocks {
        let offset = 16 + i * 4;
        let size = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
        block_sizes.push(size);
    }

    let mut current_offset = header_size;
    let mut uncompressed_data = Vec::with_capacity(uncompressed_size);

    for (i, size) in block_sizes.into_iter().enumerate() {
        trace!("GCPF decompressing block {}: offset={}, size={}", i, current_offset, size);
        if current_offset + size > data.len() {
            if current_offset + 4 == data.len() && &data[current_offset..current_offset + 4] == b"GCPF" {
                debug!("GCPF footer magic reached, stopping decompression cleanly");
                break;
            }
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "GCPF: Block data truncated"));
        }
        let compressed_block = &data[current_offset..current_offset + size];
        current_offset += size;

        let decompressed_block = match comp_mode {
            0 | 2 => {
                zstd::decode_all(compressed_block)?
            }
            _ => {
                match zstd::decode_all(compressed_block) {
                    Ok(decoded) => decoded,
                    Err(_) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("GCPF: Unsupported compression mode {}", comp_mode),
                        ));
                    }
                }
            }
        };
        uncompressed_data.extend_from_slice(&decompressed_block);
    }

    if uncompressed_data.len() > uncompressed_size {
        uncompressed_data.truncate(uncompressed_size);
    }

    Ok(uncompressed_data)
}

fn collect_bricks_recursive(
    nodes: &[GodotVariant],
    parent_transform: Transform,
    bricks: &mut Vec<VrtxBrick>,
) {
    for node in nodes {
        if let GodotVariant::Array(data) = node {
            if data.len() < 10 { continue; }

            let name = match &data[0] {
                GodotVariant::String(s) => s.clone(),
                _ => "Part".to_string(),
            };

            let local_pos = match &data[1] {
                GodotVariant::Vector3(v) => *v,
                _ => Vec3::ZERO,
            };

            let local_rot = match &data[2] {
                GodotVariant::Vector3(v) => Quat::from_euler(EulerRot::YXZ, v.y, v.x, v.z),
                _ => Quat::IDENTITY,
            };

            let local_scale = match &data[3] {
                GodotVariant::Vector3(v) => *v,
                _ => Vec3::ONE,
            };

            let color = match &data[4] {
                GodotVariant::Color(c) => *c,
                _ => Color::WHITE,
            };

            let physics_enabled = match &data[6] {
                GodotVariant::Bool(b) => *b,
                _ => false,
            };

            let bounciness = match &data[7] {
                GodotVariant::Float(f) => *f as f32,
                GodotVariant::Int(i) => *i as f32,
                _ => 0.3,
            };

            let shape_type = if data.len() > 10 {
                match &data[10] {
                    GodotVariant::String(s) => s.as_str(),
                    _ => "Cube",
                }
            } else {
                "Cube"
            };

            let shape = if shape_type == "Sphere" {
                crate::common::bricks::components::BrickShape::Sphere
            } else {
                crate::common::bricks::components::BrickShape::Block
            };

            let is_standard_brick = data.len() >= 15 && match &data[0] {
                GodotVariant::String(s) => {
                    s != "NPC" && s != "UIImage" && s != "UIButton" && s != "UIText" && s != "Decal" &&
                    s != "RemoteEvent" && s != "Terrain" && s != "Model" && s != "Weld" &&
                    s != "Hinge" && s != "Label3D" && s != "Sound" && s != "Script" && s != "LocalScript"
                }
                _ => false,
            };

            let bevy_scale = if is_standard_brick {
                match shape {
                    crate::common::bricks::components::BrickShape::Block => {
                        Vec3::new(local_scale.x / 4.0, local_scale.y / 1.0, local_scale.z / 2.0)
                    }
                    crate::common::bricks::components::BrickShape::Sphere => {
                        local_scale / 2.0
                    }
                }
            } else {
                local_scale
            };

            let local_transform = Transform {
                translation: local_pos,
                rotation: local_rot,
                scale: bevy_scale,
            };

            let global_translation = parent_transform.translation + parent_transform.rotation.mul_vec3(local_transform.translation * 0.28);
            let global_rotation = parent_transform.rotation * local_transform.rotation;
            let global_scale = parent_transform.scale * local_transform.scale;

            let global_transform = Transform {
                translation: global_translation,
                rotation: global_rotation,
                scale: global_scale,
            };

            if is_standard_brick {
                bricks.push(VrtxBrick {
                    name,
                    transform: global_transform,
                    shape,
                    color,
                    physics_enabled,
                    bounciness,
                });
            }

            let is_custom_node = match &data[0] {
                GodotVariant::String(s) => {
                    s == "RemoteEvent" || s == "Terrain" || s == "NPC" || s == "Model" || s == "Weld" ||
                    s == "Decal" || s == "Hinge" || s == "Label3D" || s == "Sound" || s == "Script" || s == "LocalScript"
                }
                _ => false,
            };

            let children_var = if is_custom_node {
                match &data[0] {
                    GodotVariant::String(s) => {
                        if s == "Model" { data.get(5) }
                        else if s == "Script" || s == "LocalScript" { data.get(4) }
                        else if s == "Weld" { data.get(8) }
                        else if s == "Decal" || s == "Terrain" { data.get(7) }
                        else if s == "NPC" { data.get(16) }
                        else if s == "Hinge" || s == "Label3D" || s == "Sound" { data.get(10) }
                        else { None }
                    }
                    _ => None,
                }
            } else {
                data.get(9)
            };

            if let Some(GodotVariant::Array(children)) = children_var {
                collect_bricks_recursive(children, global_transform, bricks);
            }
        }
    }
}

fn parse_godot_vrtx(decompressed: &[u8]) -> std::io::Result<VrtxFileState> {
    debug!("Parsing Godot VRTX, decompressed length={}", decompressed.len());
    if decompressed.len() >= 4 {
        let first_u32 = u32::from_le_bytes([decompressed[0], decompressed[1], decompressed[2], decompressed[3]]);
        debug!("First 4 bytes of decompressed payload: {} (0x{:X})", first_u32, first_u32);
    }

    let mut parser = GodotParser::new(decompressed);

    if decompressed.len() >= 8 {
        let prefix = u32::from_le_bytes([decompressed[0], decompressed[1], decompressed[2], decompressed[3]]) as usize;
        if prefix == decompressed.len() - 4 {
            debug!("Detected Godot store_var length prefix: {} bytes. Skipping prefix.", prefix);
            parser.offset = 4;
        }
    }

    let variant = parser.parse_variant()?;
    if let GodotVariant::Dictionary(dict) = variant {
        let version = match dict.get("v") {
            Some(GodotVariant::Int(v)) => *v as u32,
            _ => 0,
        };

        let gravity = Vec3::new(0.0, -186.9 * 0.28, 0.0);

        let mut bricks = Vec::new();
        if let Some(GodotVariant::Array(nodes)) = dict.get("n") {
            collect_bricks_recursive(nodes, Transform::IDENTITY, &mut bricks);
        }

        let settings = VrtxSettings {
            ssao: false,
            contact_shadows: false,
            bloom: true,
        };

        let camera_transform = Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y);

        debug!("Parsing complete: version={}, bricks={}", version, bricks.len());
        Ok(VrtxFileState {
            version,
            gravity,
            settings,
            camera_transform,
            bricks,
        })
    } else {
        error!("Parsing failed: Root element is not a Godot dictionary");
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Root element is not a Godot dictionary",
        ))
    }
}

impl VrtxFileState {
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writer.write_all(b"VRTX")?;
        writer.write_all(&self.version.to_le_bytes())?;

        writer.write_all(&self.gravity.x.to_le_bytes())?;
        writer.write_all(&self.gravity.y.to_le_bytes())?;
        writer.write_all(&self.gravity.z.to_le_bytes())?;

        writer.write_all(&[
            if self.settings.ssao { 1 } else { 0 },
            if self.settings.contact_shadows { 1 } else { 0 },
            if self.settings.bloom { 1 } else { 0 },
        ])?;

        writer.write_all(&self.camera_transform.translation.x.to_le_bytes())?;
        writer.write_all(&self.camera_transform.translation.y.to_le_bytes())?;
        writer.write_all(&self.camera_transform.translation.z.to_le_bytes())?;

        writer.write_all(&self.camera_transform.rotation.x.to_le_bytes())?;
        writer.write_all(&self.camera_transform.rotation.y.to_le_bytes())?;
        writer.write_all(&self.camera_transform.rotation.z.to_le_bytes())?;
        writer.write_all(&self.camera_transform.rotation.w.to_le_bytes())?;

        writer.write_all(&(self.bricks.len() as u32).to_le_bytes())?;

        for brick in &self.bricks {
            let name_bytes = brick.name.as_bytes();
            writer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            writer.write_all(name_bytes)?;

            writer.write_all(&brick.transform.translation.x.to_le_bytes())?;
            writer.write_all(&brick.transform.translation.y.to_le_bytes())?;
            writer.write_all(&brick.transform.translation.z.to_le_bytes())?;

            writer.write_all(&brick.transform.rotation.x.to_le_bytes())?;
            writer.write_all(&brick.transform.rotation.y.to_le_bytes())?;
            writer.write_all(&brick.transform.rotation.z.to_le_bytes())?;
            writer.write_all(&brick.transform.rotation.w.to_le_bytes())?;

            writer.write_all(&brick.transform.scale.x.to_le_bytes())?;
            writer.write_all(&brick.transform.scale.y.to_le_bytes())?;
            writer.write_all(&brick.transform.scale.z.to_le_bytes())?;

            let shape_val = match brick.shape {
                crate::common::bricks::components::BrickShape::Block => 0u8,
                crate::common::bricks::components::BrickShape::Sphere => 1u8,
            };
            writer.write_all(&[shape_val])?;

            let srgba = brick.color.to_srgba();
            writer.write_all(&srgba.red.to_le_bytes())?;
            writer.write_all(&srgba.green.to_le_bytes())?;
            writer.write_all(&srgba.blue.to_le_bytes())?;
            writer.write_all(&srgba.alpha.to_le_bytes())?;

            writer.write_all(&[if brick.physics_enabled { 1 } else { 0 }])?;
            writer.write_all(&brick.bounciness.to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        debug!("load_from_file: Attempting to open file: {}", path);
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        debug!("load_from_file: Read {} bytes from {}", data.len(), path);

        if data.len() >= 4 && &data[0..4] == b"VRTX" {
            let mut reader = BufReader::new(&data[4..]);
            let mut version_bytes = [0u8; 4];
            reader.read_exact(&mut version_bytes)?;
            let version = u32::from_le_bytes(version_bytes);
            debug!("load_from_file: VRTX format version is {}", version);

            let (gravity, settings, camera_transform, count) = if version == 1 {
                debug!("load_from_file: Parsing version 1 header");
                let mut gx = [0u8; 4]; reader.read_exact(&mut gx)?;
                let mut gy = [0u8; 4]; reader.read_exact(&mut gy)?;
                let mut gz = [0u8; 4]; reader.read_exact(&mut gz)?;
                let gravity = Vec3::new(
                    f32::from_le_bytes(gx),
                    f32::from_le_bytes(gy),
                    f32::from_le_bytes(gz),
                );

                let mut settings_bytes = [0u8; 3];
                reader.read_exact(&mut settings_bytes)?;
                let settings = VrtxSettings {
                    ssao: settings_bytes[0] != 0,
                    contact_shadows: settings_bytes[1] != 0,
                    bloom: settings_bytes[2] != 0,
                };

                let mut cx = [0u8; 4]; reader.read_exact(&mut cx)?;
                let mut cy = [0u8; 4]; reader.read_exact(&mut cy)?;
                let mut cz = [0u8; 4]; reader.read_exact(&mut cz)?;
                let camera_translation = Vec3::new(
                    f32::from_le_bytes(cx),
                    f32::from_le_bytes(cy),
                    f32::from_le_bytes(cz),
                );

                let mut crx = [0u8; 4]; reader.read_exact(&mut crx)?;
                let mut cry = [0u8; 4]; reader.read_exact(&mut cry)?;
                let mut crz = [0u8; 4]; reader.read_exact(&mut crz)?;
                let mut crw = [0u8; 4]; reader.read_exact(&mut crw)?;
                let camera_rotation = Quat::from_xyzw(
                    f32::from_le_bytes(crx),
                    f32::from_le_bytes(cry),
                    f32::from_le_bytes(crz),
                    f32::from_le_bytes(crw),
                );

                let camera_transform = Transform {
                    translation: camera_translation,
                    rotation: camera_rotation,
                    scale: Vec3::ONE,
                };

                let mut count_bytes = [0u8; 4];
                reader.read_exact(&mut count_bytes)?;
                let count = u32::from_le_bytes(count_bytes);

                (gravity, settings, camera_transform, count)
            } else if version == 0 {
                debug!("load_from_file: Parsing version 0 header");
                let mut gx = [0u8; 4]; reader.read_exact(&mut gx)?;
                let mut gy = [0u8; 4]; reader.read_exact(&mut gy)?;
                let mut gz = [0u8; 4]; reader.read_exact(&mut gz)?;
                let gravity = Vec3::new(
                    f32::from_le_bytes(gx),
                    f32::from_le_bytes(gy),
                    f32::from_le_bytes(gz),
                );

                let settings = VrtxSettings {
                    ssao: false,
                    contact_shadows: false,
                    bloom: true,
                };

                let camera_transform = Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y);

                let mut count_bytes = [0u8; 4];
                reader.read_exact(&mut count_bytes)?;
                let count = u32::from_le_bytes(count_bytes);

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
                let mut name_len_bytes = [0u8; 2];
                reader.read_exact(&mut name_len_bytes)?;
                let name_len = u16::from_le_bytes(name_len_bytes) as usize;
                let mut name_bytes = vec![0u8; name_len];
                reader.read_exact(&mut name_bytes)?;
                let name = String::from_utf8(name_bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                let mut tx = [0u8; 4]; reader.read_exact(&mut tx)?;
                let mut ty = [0u8; 4]; reader.read_exact(&mut ty)?;
                let mut tz = [0u8; 4]; reader.read_exact(&mut tz)?;
                let translation = Vec3::new(
                    f32::from_le_bytes(tx),
                    f32::from_le_bytes(ty),
                    f32::from_le_bytes(tz),
                );

                let mut rx = [0u8; 4]; reader.read_exact(&mut rx)?;
                let mut ry = [0u8; 4]; reader.read_exact(&mut ry)?;
                let mut rz = [0u8; 4]; reader.read_exact(&mut rz)?;
                let mut rw = [0u8; 4]; reader.read_exact(&mut rw)?;
                let rotation = Quat::from_xyzw(
                    f32::from_le_bytes(rx),
                    f32::from_le_bytes(ry),
                    f32::from_le_bytes(rz),
                    f32::from_le_bytes(rw),
                );

                let mut sx = [0u8; 4]; reader.read_exact(&mut sx)?;
                let mut sy = [0u8; 4]; reader.read_exact(&mut sy)?;
                let mut sz = [0u8; 4]; reader.read_exact(&mut sz)?;
                let scale = Vec3::new(
                    f32::from_le_bytes(sx),
                    f32::from_le_bytes(sy),
                    f32::from_le_bytes(sz),
                );

                let transform = Transform {
                    translation,
                    rotation,
                    scale,
                };

                let mut shape_bytes = [0u8; 1];
                reader.read_exact(&mut shape_bytes)?;
                let shape = match shape_bytes[0] {
                    0 => crate::common::bricks::components::BrickShape::Block,
                    1 => crate::common::bricks::components::BrickShape::Sphere,
                    _ => {
                        error!("load_from_file: Invalid brick shape enum value");
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid brick shape enum value",
                        ));
                    }
                };

                let mut cr = [0u8; 4]; reader.read_exact(&mut cr)?;
                let mut cg = [0u8; 4]; reader.read_exact(&mut cg)?;
                let mut cb = [0u8; 4]; reader.read_exact(&mut cb)?;
                let mut ca = [0u8; 4]; reader.read_exact(&mut ca)?;
                let color = Color::Srgba(Srgba::new(
                    f32::from_le_bytes(cr),
                    f32::from_le_bytes(cg),
                    f32::from_le_bytes(cb),
                    f32::from_le_bytes(ca),
                ));

                let mut phys_enabled_bytes = [0u8; 1];
                reader.read_exact(&mut phys_enabled_bytes)?;
                let physics_enabled = phys_enabled_bytes[0] != 0;

                let mut bounciness_bytes = [0u8; 4];
                reader.read_exact(&mut bounciness_bytes)?;
                let bounciness = f32::from_le_bytes(bounciness_bytes);

                bricks.push(VrtxBrick {
                    name,
                    transform,
                    shape,
                    color,
                    physics_enabled,
                    bounciness,
                });
            }

            debug!("load_from_file: Successfully parsed {} bricks from standard VRTX file", bricks.len());
            Ok(Self {
                version,
                gravity,
                settings,
                camera_transform,
                bricks,
            })
        } else if data.len() >= 4 && &data[0..4] == b"GCPF" {
            debug!("load_from_file: Detected legacy GCPF (Godot) file format");
            let decompressed = decompress_gcpf_file(&data)?;
            debug!("load_from_file: Successfully decompressed GCPF file into {} bytes", decompressed.len());
            let parsed_state = parse_godot_vrtx(&decompressed)?;
            debug!("load_from_file: Successfully parsed Godot VRTX map with {} bricks", parsed_state.bricks.len());
            Ok(parsed_state)
        } else {
            error!("load_from_file: Unknown or invalid file signature");
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown or invalid file signature",
            ))
        }
    }
}