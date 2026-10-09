use std::collections::HashMap;
use std::io::Cursor;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::Indices;

const GLB_MAGIC: u32 = 0x4654_6C67;
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

const COMPONENT_BYTE: u32 = 5121;
const COMPONENT_UNSIGNED_SHORT: u32 = 5123;
const COMPONENT_UNSIGNED_INT: u32 = 5125;

const MODE_TRIANGLES: u32 = 4;

#[derive(Debug, Default, Clone)]
pub struct LoadedMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct LoadedMeshAsset {
    pub mesh: LoadedMeshData,
    pub base_color: [f32; 4],
    pub texture_bytes: Option<Vec<u8>>,
    pub double_sided: bool,
    pub alpha_mode: u8,
    pub size: Vec3,
    pub rotation: Quat,
}

fn parse_glb_chunks(bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    if bytes.len() < 12 {
        return Err("glb too short".to_string());
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != GLB_MAGIC {
        return Err("invalid glb magic".to_string());
    }
    let mut json_bytes: Option<Vec<u8>> = None;
    let mut bin_bytes: Option<Vec<u8>> = None;
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let chunk_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let chunk_type = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;
        let chunk_end = (offset + chunk_len).min(bytes.len());
        if chunk_end < offset {
            break;
        }
        let chunk_data = &bytes[offset..chunk_end];
        if chunk_type == CHUNK_JSON && json_bytes.is_none() {
            json_bytes = Some(chunk_data.to_vec());
        } else if chunk_type == CHUNK_BIN && bin_bytes.is_none() {
            bin_bytes = Some(chunk_data.to_vec());
        }
        offset = chunk_end;
    }
    let json = json_bytes.ok_or_else(|| "missing json chunk in glb".to_string())?;
    let bin = bin_bytes.unwrap_or_default();
    Ok((json, bin))
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfJson {
    #[serde(default)]
    accessors: Vec<GltfAccessor>,
    #[serde(default)]
    buffer_views: Vec<GltfBufferView>,
    #[serde(default)]
    meshes: Vec<GltfMesh>,
    #[serde(default)]
    materials: Vec<GltfMaterial>,
    #[serde(default)]
    textures: Vec<GltfTexture>,
    #[serde(default)]
    images: Vec<GltfImage>,
    #[serde(default)]
    nodes: Vec<GltfNode>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfNode {
    mesh: Option<usize>,
    #[serde(default)]
    rotation: Option<[f32; 4]>,
    #[serde(default)]
    scale: Option<[f32; 3]>,
    #[serde(default)]
    matrix: Option<[f32; 16]>,
    #[serde(default)]
    children: Vec<usize>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfAccessor {
    buffer_view: Option<usize>,
    byte_offset: Option<u32>,
    component_type: Option<u32>,
    count: usize,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfBufferView {
    buffer: usize,
    byte_offset: Option<u32>,
    byte_length: Option<u32>,
    byte_stride: Option<u32>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfMesh {
    #[serde(default)]
    primitives: Vec<GltfPrimitive>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfPrimitive {
    #[serde(default)]
    attributes: HashMap<String, usize>,
    indices: Option<usize>,
    mode: Option<u32>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfMaterial {
    double_sided: Option<bool>,
    alpha_mode: Option<String>,
    pbr_metallic_roughness: Option<GltfPbr>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfPbr {
    #[serde(default = "default_base_color")]
    base_color_factor: [f32; 4],
    base_color_texture: Option<GltfTextureRef>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfTextureRef {
    index: usize,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfTexture {
    source: Option<usize>,
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GltfImage {
    buffer_view: Option<usize>,
    uri: Option<String>,
}

fn default_base_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn accessor_byte_stride(acc: &GltfAccessor, bv: &GltfBufferView) -> usize {
    if let Some(stride) = bv.byte_stride {
        if stride > 0 {
            return stride as usize;
        }
    }
    match (acc.ty.as_str(), acc.component_type.unwrap_or(0)) {
        ("VEC3", COMPONENT_UNSIGNED_INT | 5126) => 12,
        ("VEC2", COMPONENT_UNSIGNED_INT | 5126) => 8,
        ("SCALAR", COMPONENT_BYTE) => 1,
        ("SCALAR", COMPONENT_UNSIGNED_SHORT) => 2,
        ("SCALAR", COMPONENT_UNSIGNED_INT) => 4,
        _ => 0,
    }
}

fn read_vec3(bin: &[u8], acc: &GltfAccessor, bv: &GltfBufferView) -> Vec<[f32; 3]> {
    if bv.buffer != 0 {
        return Vec::new();
    }
    let stride = accessor_byte_stride(acc, bv);
    if stride == 0 {
        return Vec::new();
    }
    let start = bv.byte_offset.unwrap_or(0) as usize + acc.byte_offset.unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(acc.count);
    for i in 0..acc.count {
        let offset = start + i * stride;
        if offset + 12 > bin.len() {
            break;
        }
        let x = f32::from_le_bytes([
            bin[offset],
            bin[offset + 1],
            bin[offset + 2],
            bin[offset + 3],
        ]);
        let y = f32::from_le_bytes([
            bin[offset + 4],
            bin[offset + 5],
            bin[offset + 6],
            bin[offset + 7],
        ]);
        let z = f32::from_le_bytes([
            bin[offset + 8],
            bin[offset + 9],
            bin[offset + 10],
            bin[offset + 11],
        ]);
        out.push([x, y, z]);
    }
    out
}

fn read_vec2(bin: &[u8], acc: &GltfAccessor, bv: &GltfBufferView) -> Vec<[f32; 2]> {
    if bv.buffer != 0 {
        return Vec::new();
    }
    let stride = accessor_byte_stride(acc, bv);
    if stride == 0 {
        return Vec::new();
    }
    let start = bv.byte_offset.unwrap_or(0) as usize + acc.byte_offset.unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(acc.count);
    for i in 0..acc.count {
        let offset = start + i * stride;
        if offset + 8 > bin.len() {
            break;
        }
        let u = f32::from_le_bytes([
            bin[offset],
            bin[offset + 1],
            bin[offset + 2],
            bin[offset + 3],
        ]);
        let v = f32::from_le_bytes([
            bin[offset + 4],
            bin[offset + 5],
            bin[offset + 6],
            bin[offset + 7],
        ]);
        out.push([u, v]);
    }
    out
}

fn read_indices(bin: &[u8], acc: &GltfAccessor, bv: &GltfBufferView) -> Vec<u32> {
    if bv.buffer != 0 {
        return Vec::new();
    }
    let stride = accessor_byte_stride(acc, bv);
    if stride == 0 {
        return Vec::new();
    }
    let start = bv.byte_offset.unwrap_or(0) as usize + acc.byte_offset.unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(acc.count);
    match acc.component_type {
        Some(COMPONENT_BYTE) => {
            for i in 0..acc.count {
                let offset = start + i * stride;
                if offset >= bin.len() {
                    break;
                }
                out.push(bin[offset] as u32);
            }
        }
        Some(COMPONENT_UNSIGNED_SHORT) => {
            for i in 0..acc.count {
                let offset = start + i * stride;
                if offset + 2 > bin.len() {
                    break;
                }
                out.push(u16::from_le_bytes([bin[offset], bin[offset + 1]]) as u32);
            }
        }
        Some(COMPONENT_UNSIGNED_INT) => {
            for i in 0..acc.count {
                let offset = start + i * stride;
                if offset + 4 > bin.len() {
                    break;
                }
                out.push(u32::from_le_bytes([
                    bin[offset],
                    bin[offset + 1],
                    bin[offset + 2],
                    bin[offset + 3],
                ]));
            }
        }
        _ => {}
    }
    out
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-8 {
        [n[0] / len, n[1] / len, n[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

pub fn load_glb(bytes: &[u8]) -> Result<LoadedMeshAsset, String> {
    let (json_bytes, bin) = parse_glb_chunks(bytes)?;
    let doc: GltfJson = serde_json::from_slice(&json_bytes)
        .map_err(|e| format!("failed to parse glb json: {e}"))?;

    let mut out = LoadedMeshData::default();
    for mesh in &doc.meshes {
        for prim in &mesh.primitives {
            if let Some(mode) = prim.mode {
                if mode != MODE_TRIANGLES {
                    continue;
                }
            }
            let Some(&pos_idx) = prim.attributes.get("POSITION") else {
                continue;
            };
            let Some(pos_acc) = doc.accessors.get(pos_idx) else {
                continue;
            };
            let Some(pos_bv_idx) = pos_acc.buffer_view else {
                continue;
            };
            let Some(pos_bv) = doc.buffer_views.get(pos_bv_idx) else {
                continue;
            };
            let positions = read_vec3(&bin, pos_acc, pos_bv);
            if positions.is_empty() {
                continue;
            }

            let normals = prim
                .attributes
                .get("NORMAL")
                .and_then(|&n_idx| doc.accessors.get(n_idx))
                .and_then(|acc| {
                    acc.buffer_view
                        .and_then(|bv_idx| doc.buffer_views.get(bv_idx))
                })
                .map(|bv| {
                    let acc = &doc.accessors[*prim.attributes.get("NORMAL").unwrap()];
                    read_vec3(&bin, acc, bv)
                })
                .unwrap_or_default();

            let uvs = prim
                .attributes
                .get("TEXCOORD_0")
                .and_then(|&t_idx| doc.accessors.get(t_idx))
                .and_then(|acc| {
                    acc.buffer_view
                        .and_then(|bv_idx| doc.buffer_views.get(bv_idx))
                })
                .map(|bv| {
                    let acc = &doc.accessors[*prim.attributes.get("TEXCOORD_0").unwrap()];
                    read_vec2(&bin, acc, bv)
                })
                .unwrap_or_default();

            let indices: Vec<u32> = match prim.indices {
                Some(idx) => {
                    let Some(acc) = doc.accessors.get(idx) else {
                        continue;
                    };
                    let Some(bv) = acc
                        .buffer_view
                        .and_then(|bv_idx| doc.buffer_views.get(bv_idx))
                    else {
                        continue;
                    };
                    read_indices(&bin, acc, bv)
                }
                None => (0..positions.len() as u32).collect(),
            };

            let base = out.positions.len() as u32;
            for &i in &indices {
                let i = i as usize;
                if i >= positions.len() {
                    continue;
                }
                out.positions.push(positions[i]);
                if i < normals.len() {
                    out.normals.push(normals[i]);
                }
                if i < uvs.len() {
                    out.uvs.push(uvs[i]);
                }
                out.indices.push(base + out.indices.len() as u32);
            }
        }
    }

    if out.positions.is_empty() {
        return Err("no triangles found in glb".to_string());
    }

    if out.normals.is_empty() {
        let pos = &out.positions;
        for tri in 0..out.indices.len() / 3 {
            let a = pos[(tri * 3) as usize];
            let b = pos[(tri * 3 + 1) as usize];
            let c = pos[(tri * 3 + 2) as usize];
            let n = triangle_normal(a, b, c);
            out.normals.push(n);
            out.normals.push(n);
            out.normals.push(n);
        }
    }

    let material = doc.materials.first();
    let base_color = material
        .and_then(|m| m.pbr_metallic_roughness.as_ref())
        .map(|p| p.base_color_factor)
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let double_sided = material.and_then(|m| m.double_sided).unwrap_or(false);
    let alpha_mode = material
        .and_then(|m| m.alpha_mode.as_deref())
        .map(|mode| match mode {
            "BLEND" => 2u8,
            "MASK" => 1u8,
            _ => 0u8,
        })
        .unwrap_or(0u8);

    let mut texture_bytes = None;
    if let Some(material) = material {
        if let Some(pbr) = material.pbr_metallic_roughness.as_ref() {
            if let Some(tex) = pbr.base_color_texture.as_ref() {
                if let Some(gltf_tex) = doc.textures.get(tex.index) {
                    if let Some(img_idx) = gltf_tex.source {
                        if let Some(img) = doc.images.get(img_idx) {
                            if img.uri.is_none() {
                                if let Some(bv_idx) = img.buffer_view {
                                    if let Some(bv) = doc.buffer_views.get(bv_idx) {
                                        let start = bv.byte_offset.unwrap_or(0) as usize;
                                        let len = bv.byte_length.unwrap_or(0) as usize;
                                        if start + len <= bin.len() {
                                            texture_bytes = Some(bin[start..start + len].to_vec());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let (model_rotation, node_scale) = glb_model_transform(&doc);
    bake_node_scale(&mut out, node_scale);
    center_mesh_data(&mut out);
    let size = mesh_bounds(&out);

    Ok(LoadedMeshAsset {
        mesh: out,
        base_color,
        texture_bytes,
        double_sided,
        alpha_mode,
        size,
        rotation: model_rotation,
    })
}

pub fn load_obj(bytes: &[u8]) -> Result<LoadedMeshData, String> {
    let load_options = tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ..Default::default()
    };
    let (models, _mtl) = tobj::load_obj_buf(&mut Cursor::new(bytes), &load_options, |_| {
        Ok((vec![tobj::Material::default()], Default::default()))
    })
    .map_err(|e| format!("failed to parse obj: {e}"))?;

    let mut out = LoadedMeshData::default();
    for model in models {
        let m = &model.mesh;
        let base = out.positions.len() as u32;
        let mut positions = Vec::with_capacity(m.positions.len() / 3);
        for i in 0..m.positions.len() / 3 {
            positions.push([
                m.positions[3 * i],
                m.positions[3 * i + 1],
                m.positions[3 * i + 2],
            ]);
        }
        let mut normals = Vec::with_capacity(m.normals.len() / 3);
        for i in 0..m.normals.len() / 3 {
            normals.push([m.normals[3 * i], m.normals[3 * i + 1], m.normals[3 * i + 2]]);
        }
        let mut uvs = Vec::with_capacity(m.texcoords.len() / 2);
        for i in 0..m.texcoords.len() / 2 {
            uvs.push([m.texcoords[2 * i], m.texcoords[2 * i + 1]]);
        }
        for &idx in &m.indices {
            let idx = idx as usize;
            if idx >= positions.len() {
                continue;
            }
            out.positions.push(positions[idx]);
            if idx < normals.len() {
                out.normals.push(normals[idx]);
            }
            if idx < uvs.len() {
                out.uvs.push(uvs[idx]);
            }
            out.indices.push(base + out.indices.len() as u32);
        }
    }

    if out.positions.is_empty() {
        return Err("no geometry found in obj".to_string());
    }

    if out.normals.is_empty() {
        let pos = &out.positions;
        for tri in 0..out.indices.len() / 3 {
            let a = pos[(tri * 3) as usize];
            let b = pos[(tri * 3 + 1) as usize];
            let c = pos[(tri * 3 + 2) as usize];
            let n = triangle_normal(a, b, c);
            out.normals.push(n);
            out.normals.push(n);
            out.normals.push(n);
        }
    }

    center_mesh_data(&mut out);

    Ok(out)
}

pub fn center_mesh_data(data: &mut LoadedMeshData) {
    if data.positions.is_empty() {
        return;
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &data.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    for p in &mut data.positions {
        p[0] -= center[0];
        p[1] -= center[1];
        p[2] -= center[2];
    }
}

pub fn mesh_bounds(data: &LoadedMeshData) -> Vec3 {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &data.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    Vec3::new(max[0] - min[0], max[1] - min[1], max[2] - min[2]).max(Vec3::ZERO)
}

fn glb_model_transform(doc: &GltfJson) -> (Quat, Vec3) {
    let mut parents: HashMap<usize, usize> = HashMap::new();
    for (parent_idx, node) in doc.nodes.iter().enumerate() {
        for &child in &node.children {
            parents.insert(child, parent_idx);
        }
    }
    let Some(first_mesh_node) = doc.nodes.iter().position(|n| n.mesh.is_some()) else {
        return (Quat::IDENTITY, Vec3::ONE);
    };
    let mut rotation = Quat::IDENTITY;
    let mut scale = Vec3::ONE;
    let mut current = first_mesh_node;
    for _ in 0..64 {
        let Some(node) = doc.nodes.get(current) else {
            break;
        };
        let (node_rotation, node_scale) = if let Some(matrix) = node.matrix {
            let mat = bevy::math::Mat4::from_cols_array(&matrix);
            let (_translation, r, s) = mat.to_scale_rotation_translation();
            (r, s)
        } else {
            let r = node
                .rotation
                .map(|[x, y, z, w]| Quat::from_xyzw(x, y, z, w))
                .unwrap_or(Quat::IDENTITY);
            let s = node
                .scale
                .map(|[x, y, z]| Vec3::new(x, y, z))
                .unwrap_or(Vec3::ONE);
            (r, s)
        };
        rotation = node_rotation * rotation;
        scale *= node_scale;
        match parents.get(&current) {
            Some(&parent) => current = parent,
            None => break,
        }
    }
    (rotation, scale)
}

fn bake_node_scale(data: &mut LoadedMeshData, node_scale: Vec3) {
    if node_scale == Vec3::ONE {
        return;
    }
    let safe_scale = node_scale.max(Vec3::splat(1e-8));
    let inv = Vec3::ONE / safe_scale;
    for p in &mut data.positions {
        p[0] *= safe_scale.x;
        p[1] *= safe_scale.y;
        p[2] *= safe_scale.z;
    }
    for n in &mut data.normals {
        let v = Vec3::new(n[0], n[1], n[2]) * inv;
        let len = v.length();
        if len > 1e-8 {
            *n = [v.x / len, v.y / len, v.z / len];
        }
    }
}

pub fn normalize_mesh_data(data: &mut LoadedMeshData) {
    let size = mesh_bounds(data);
    let mut extent = 0.0f32;
    for axis in 0..3 {
        extent = extent.max(size[axis]);
    }
    if extent <= 1e-8 {
        return;
    }
    let scale = 1.0 / extent;
    for p in &mut data.positions {
        p[0] *= scale;
        p[1] *= scale;
        p[2] *= scale;
    }
}

pub fn build_bevy_mesh(data: &LoadedMeshData) -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals.clone());
    if !data.uvs.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs.clone());
    }
    mesh.insert_indices(Indices::U32(data.indices.clone()));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_glb(json: &str, bin: &[u8]) -> Vec<u8> {
        let json = json.as_bytes();
        let json_pad = (4 - (json.len() % 4)) % 4;
        let bin_pad = (4 - (bin.len() % 4)) % 4;
        let mut out = Vec::new();
        out.extend_from_slice(&0x4654_6C67u32.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        let total = 12 + 8 + json.len() + json_pad + 8 + bin.len() + bin_pad;
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&((json.len() + json_pad) as u32).to_le_bytes());
        out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
        out.extend_from_slice(json);
        out.extend(std::iter::repeat(b' ').take(json_pad));
        out.extend_from_slice(&((bin.len() + bin_pad) as u32).to_le_bytes());
        out.extend_from_slice(&0x004E_4942u32.to_le_bytes());
        out.extend_from_slice(bin);
        out.extend(std::iter::repeat(0u8).take(bin_pad));
        out
    }

    fn triangle_bin() -> Vec<u8> {
        let mut bin = Vec::new();
        for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in v {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        bin
    }

    fn assert_vec3_eq(a: Vec3, b: Vec3) {
        for axis in 0..3 {
            assert!(
                (a[axis] - b[axis]).abs() < 1e-4,
                "vec3 mismatch: {a} != {b}"
            );
        }
    }

    #[test]
    fn load_glb_inherits_node_size_and_rotation() {
        let json = r#"{
            "asset": {"version": "2.0"},
            "buffers": [{"byteLength": 42}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 6}
            ],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
            "nodes": [{"mesh": 0, "scale": [2.0, 2.0, 2.0], "rotation": [0.0, 0.70710677, 0.0, 0.70710677]}]
        }"#;
        let loaded = load_glb(&build_glb(json, &triangle_bin())).expect("glb should load");
        assert_vec3_eq(loaded.size, Vec3::new(2.0, 2.0, 0.0));
        assert!((loaded.rotation.w - 0.70710677).abs() < 1e-4);
        assert!((loaded.rotation.y - 0.70710677).abs() < 1e-4);
        assert_eq!(loaded.mesh.indices.len(), 3);
        assert_vec3_eq(mesh_bounds(&loaded.mesh), Vec3::new(2.0, 2.0, 0.0));
    }

    #[test]
    fn mesh_bounds_and_normalize_scale_to_unit_extent() {
        let data = LoadedMeshData {
            positions: vec![
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 4.0, 0.0],
                [0.0, 0.0, 6.0],
            ],
            ..Default::default()
        };
        assert_vec3_eq(mesh_bounds(&data), Vec3::new(2.0, 4.0, 6.0));
        let mut normalized = data;
        normalize_mesh_data(&mut normalized);
        assert_vec3_eq(
            mesh_bounds(&normalized),
            Vec3::new(2.0 / 6.0, 4.0 / 6.0, 1.0),
        );
    }

    #[test]
    fn center_mesh_data_keeps_bounds_but_centers_pivot() {
        let mut data = LoadedMeshData {
            positions: vec![[1.0, 2.0, 3.0], [3.0, 6.0, 9.0], [5.0, 4.0, 7.0]],
            ..Default::default()
        };
        center_mesh_data(&mut data);
        assert_vec3_eq(mesh_bounds(&data), Vec3::new(4.0, 4.0, 6.0));
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &data.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        for axis in 0..3 {
            assert!(
                (min[axis] + max[axis]).abs() < 1e-4,
                "box not centered on axis {axis}"
            );
        }
    }
}
