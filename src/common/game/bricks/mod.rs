pub mod components;
pub mod data;
pub mod studs;

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

#[derive(Resource, Default)]
pub struct BrickMaterialCache {
    pub studs_materials: std::collections::HashMap<
        [u8; 4],
        Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>,
    >,
    pub plain_materials: std::collections::HashMap<
        [u8; 4],
        Handle<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    >,
    pub block_meshes: std::collections::HashMap<[u32; 3], Handle<Mesh>>,
    pub sphere_mesh: Option<Handle<Mesh>>,
    pub cylinder_mesh: Option<Handle<Mesh>>,
    pub wedge_mesh: Option<Handle<Mesh>>,
    pub corner_wedge_mesh: Option<Handle<Mesh>>,
}

#[derive(Resource)]
pub struct WorkspaceShowStuds {
    pub enabled: bool,
}

pub const BRICK_PERCEPTUAL_ROUGHNESS: f32 = 0.5;
pub const BRICK_METALLIC: f32 = 0.0;
pub const BRICK_REFLECTANCE: f32 = 0.5;
pub const STUD_FADE_END_DISTANCE: f32 = 16.0;

impl Default for WorkspaceShowStuds {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub struct BricksPlugin;

impl Plugin for BricksPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::BrickPhysics>()
            .register_type::<components::Brick>()
            .register_type::<components::BrickShapeComponent>()
            .register_type::<components::BrickColor>()
            .register_type::<components::BrickStuds>()
            .init_resource::<data::BrickSpawnerCount>()
            .init_resource::<BrickMaterialCache>()
            .init_resource::<WorkspaceShowStuds>();

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, studs::StudsExtension>,
            >::default())
                .add_plugins(MaterialPlugin::<
                    ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>,
                >::default())
                .add_systems(Startup, studs::setup_studs)
                .add_systems(
                    Update,
                    (
                        studs::configure_studs_samplers,
                        update_brick_meshes_on_shape_change,
                        optimize_brick_visibility,
                        sync_transparent_brick_shadow_receive,
                    ),
                );
        }
    }
}

pub fn quantize_color(base_color: Color) -> Color {
    let srgba = base_color.to_srgba();
    let quantize = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() / 255.0;
    Color::srgba(
        quantize(srgba.red),
        quantize(srgba.green),
        quantize(srgba.blue),
        quantize(srgba.alpha),
    )
}

pub fn studs_material_for_color(
    cache: &mut BrickMaterialCache,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>,
    studs_assets: &studs::StudsAssets,
    base_color: Color,
) -> Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>> {
    let quantized = quantize_color(base_color);
    let srgba = quantized.to_srgba();
    let cache_key = [
        (srgba.red * 255.0).round() as u8,
        (srgba.green * 255.0).round() as u8,
        (srgba.blue * 255.0).round() as u8,
        (srgba.alpha * 255.0).round() as u8,
    ];

    if let Some(existing) = cache.studs_materials.get(&cache_key) {
        existing.clone()
    } else {
        let new_mat = studs_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: quantized,
                perceptual_roughness: BRICK_PERCEPTUAL_ROUGHNESS,
                metallic: BRICK_METALLIC,
                reflectance: BRICK_REFLECTANCE,
                alpha_mode: if quantized.alpha() < 1.0 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                ..default()
            },
            extension: studs::StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                stud_ambient_texture: studs_assets.stud_ambient.clone(),
                stud_height_texture: studs_assets.stud_height.clone(),
                inlet_ambient_texture: studs_assets.inlet_ambient.clone(),
                inlet_height_texture: studs_assets.inlet_height.clone(),
            },
        });
        cache.studs_materials.insert(cache_key, new_mat.clone());
        new_mat
    }
}

pub fn plain_material_for_color(
    cache: &mut BrickMaterialCache,
    plain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    base_color: Color,
) -> Handle<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>> {
    let quantized = quantize_color(base_color);
    let srgba = quantized.to_srgba();
    let cache_key = [
        (srgba.red * 255.0).round() as u8,
        (srgba.green * 255.0).round() as u8,
        (srgba.blue * 255.0).round() as u8,
        (srgba.alpha * 255.0).round() as u8,
    ];

    if let Some(existing) = cache.plain_materials.get(&cache_key) {
        existing.clone()
    } else {
        let new_mat = plain_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: quantized,
                perceptual_roughness: BRICK_PERCEPTUAL_ROUGHNESS,
                metallic: BRICK_METALLIC,
                reflectance: BRICK_REFLECTANCE,
                alpha_mode: if quantized.alpha() < 1.0 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                ..default()
            },
            extension: studs::ShadowOpacityExtension::default(),
        });
        cache.plain_materials.insert(cache_key, new_mat.clone());
        new_mat
    }
}

const BLOCK_HALF_EXTENTS: [f32; 3] = [2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28];
const BLOCK_EDGE_RADIUS: f32 = 0.04 * 0.28;
const BLOCK_EDGE_SEGMENTS: usize = 6;
const CYLINDER_RADIUS: f32 = 1.0 * 0.28;
const CYLINDER_HALF_HEIGHT: f32 = 2.0 * 0.28;

pub fn wedge_hull_points() -> Vec<Vec3> {
    let hx = BLOCK_HALF_EXTENTS[0];
    let hy = BLOCK_HALF_EXTENTS[1];
    let hz = BLOCK_HALF_EXTENTS[2];
    vec![
        Vec3::new(-hx, hy, hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, -hy, hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
    ]
}

pub fn corner_wedge_hull_points() -> Vec<Vec3> {
    let hx = BLOCK_HALF_EXTENTS[0];
    let hy = BLOCK_HALF_EXTENTS[1];
    let hz = BLOCK_HALF_EXTENTS[2];
    vec![
        Vec3::new(hx, hy, -hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(-hx, -hy, hz),
        Vec3::new(-hx, -hy, -hz),
    ]
}

pub fn brick_collider_for_shape(shape: components::BrickShape) -> avian3d::prelude::Collider {
    use avian3d::prelude::Collider;
    match shape {
        components::BrickShape::Block => {
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
        }
        components::BrickShape::Sphere => Collider::sphere(1.0 * 0.28),
        components::BrickShape::Cylinder => Collider::compound(vec![(
            Vec3::ZERO,
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            Collider::cylinder(CYLINDER_RADIUS, CYLINDER_HALF_HEIGHT * 2.0),
        )]),
        components::BrickShape::Wedge => Collider::convex_hull(wedge_hull_points())
            .unwrap_or_else(|| Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)),
        components::BrickShape::CornerWedge => {
            Collider::convex_hull(corner_wedge_hull_points())
                .unwrap_or_else(|| Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))
        }
    }
}

pub fn brick_bounding_half_extents(shape: components::BrickShape) -> Vec3 {
    match shape {
        components::BrickShape::Block
        | components::BrickShape::Wedge
        | components::BrickShape::CornerWedge => {
            Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28)
        }
        components::BrickShape::Sphere => Vec3::splat(1.0 * 0.28),
        components::BrickShape::Cylinder => Vec3::new(2.0 * 0.28, 1.0 * 0.28, 1.0 * 0.28),
    }
}

pub fn cylinder_brick_mesh() -> Mesh {
    let mut mesh: Mesh = Cylinder::new(CYLINDER_RADIUS, CYLINDER_HALF_HEIGHT * 2.0).into();
    if let Some(positions) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(values) = positions {
            for p in values.iter_mut() {
                let (x, y, z) = (p[0], p[1], p[2]);
                p[0] = -y;
                p[1] = x;
                p[2] = z;
            }
        }
    }
    if let Some(normals) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(values) = normals {
            for n in values.iter_mut() {
                let (x, y, z) = (n[0], n[1], n[2]);
                n[0] = -y;
                n[1] = x;
                n[2] = z;
            }
        }
    }
    mesh
}

fn push_flat_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    quad: [Vec3; 4],
) {
    let edge_a = quad[1] - quad[0];
    let edge_b = quad[2] - quad[0];
    let normal = edge_a.cross(edge_b).normalize_or_zero();
    let base = positions.len() as u32;
    for v in quad {
        positions.push(v.to_array());
        normals.push(normal.to_array());
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_flat_tri(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    tri: [Vec3; 3],
) {
    let edge_a = tri[1] - tri[0];
    let edge_b = tri[2] - tri[0];
    let normal = edge_a.cross(edge_b).normalize_or_zero();
    let base = positions.len() as u32;
    for v in tri {
        positions.push(v.to_array());
        normals.push(normal.to_array());
    }
    indices.extend_from_slice(&[base, base + 1, base + 2]);
}

pub fn wedge_brick_mesh() -> Mesh {
    let hx = BLOCK_HALF_EXTENTS[0];
    let hy = BLOCK_HALF_EXTENTS[1];
    let hz = BLOCK_HALF_EXTENTS[2];
    let top_back_left = Vec3::new(-hx, hy, hz);
    let top_back_right = Vec3::new(hx, hy, hz);
    let bottom_back_left = Vec3::new(-hx, -hy, hz);
    let bottom_back_right = Vec3::new(hx, -hy, hz);
    let bottom_front_left = Vec3::new(-hx, -hy, -hz);
    let bottom_front_right = Vec3::new(hx, -hy, -hz);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(24);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(24);
    let mut indices: Vec<u32> = Vec::with_capacity(24);
    push_flat_quad(
        &mut positions,
        &mut normals,
        &mut indices,
        [
            bottom_back_left,
            bottom_front_left,
            bottom_front_right,
            bottom_back_right,
        ],
    );
    push_flat_quad(
        &mut positions,
        &mut normals,
        &mut indices,
        [
            top_back_left,
            bottom_back_left,
            bottom_back_right,
            top_back_right,
        ],
    );
    push_flat_tri(
        &mut positions,
        &mut normals,
        &mut indices,
        [top_back_left, bottom_front_left, bottom_back_left],
    );
    push_flat_tri(
        &mut positions,
        &mut normals,
        &mut indices,
        [top_back_right, bottom_back_right, bottom_front_right],
    );
    push_flat_quad(
        &mut positions,
        &mut normals,
        &mut indices,
        [
            top_back_left,
            top_back_right,
            bottom_front_right,
            bottom_front_left,
        ],
    );
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn corner_wedge_brick_mesh() -> Mesh {
    let hx = BLOCK_HALF_EXTENTS[0];
    let hy = BLOCK_HALF_EXTENTS[1];
    let hz = BLOCK_HALF_EXTENTS[2];
    let apex = Vec3::new(hx, hy, -hz);
    let back_right = Vec3::new(hx, -hy, hz);
    let front_right = Vec3::new(hx, -hy, -hz);
    let back_left = Vec3::new(-hx, -hy, hz);
    let front_left = Vec3::new(-hx, -hy, -hz);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(18);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(18);
    let mut indices: Vec<u32> = Vec::with_capacity(18);
    push_flat_quad(
        &mut positions,
        &mut normals,
        &mut indices,
        [back_right, back_left, front_left, front_right],
    );
    push_flat_tri(
        &mut positions,
        &mut normals,
        &mut indices,
        [apex, back_right, front_right],
    );
    push_flat_tri(
        &mut positions,
        &mut normals,
        &mut indices,
        [apex, front_right, front_left],
    );
    push_flat_tri(
        &mut positions,
        &mut normals,
        &mut indices,
        [apex, back_left, back_right],
    );
    push_flat_tri(
        &mut positions,
        &mut normals,
        &mut indices,
        [apex, front_left, back_left],
    );
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn brick_scale_key(scale: Vec3) -> [u32; 3] {
    let quantize = |v: f32| (v.clamp(0.01, 1000.0) * 100.0).round().to_bits();
    [quantize(scale.x), quantize(scale.y), quantize(scale.z)]
}

pub fn block_brick_mesh(scale: Vec3) -> Mesh {
    let h = BLOCK_HALF_EXTENTS;
    let s = [scale.x.max(0.01), scale.y.max(0.01), scale.z.max(0.01)];
    let r = [
        (BLOCK_EDGE_RADIUS / s[0]).min(h[0]),
        (BLOCK_EDGE_RADIUS / s[1]).min(h[1]),
        (BLOCK_EDGE_RADIUS / s[2]).min(h[2]),
    ];
    let inner = [h[0] - r[0], h[1] - r[1], h[2] - r[2]];
    let segments = BLOCK_EDGE_SEGMENTS;

    let mut sin_t = [0.0f32; BLOCK_EDGE_SEGMENTS + 1];
    let mut cos_t = [0.0f32; BLOCK_EDGE_SEGMENTS + 1];
    for t in 0..=segments {
        let theta = t as f32 / segments as f32 * std::f32::consts::FRAC_PI_2;
        sin_t[t] = theta.sin();
        cos_t[t] = theta.cos();
    }

    let expected_vertices = 6 * 4 + 12 * 2 * (segments + 1) + 8 * (segments + 1) * (segments + 1);
    let expected_indices = 6 * 6 + 12 * segments * 6 + 8 * (segments * segments * 6 - segments * 3);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(expected_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(expected_vertices);
    let mut indices: Vec<u32> = Vec::with_capacity(expected_indices);

    let push_quad = |positions: &mut Vec<[f32; 3]>,
                     normals: &mut Vec<[f32; 3]>,
                     indices: &mut Vec<u32>,
                     corners: [[f32; 3]; 4],
                     normal: [f32; 3]| {
        let base = positions.len() as u32;
        positions.extend_from_slice(&corners);
        normals.extend_from_slice(&[normal; 4]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    for axis in 0..3usize {
        let u_axis = (axis + 1) % 3;
        let v_axis = (axis + 2) % 3;
        for sign in [-1.0f32, 1.0f32] {
            let (u_axis, v_axis) = if sign > 0.0 {
                (u_axis, v_axis)
            } else {
                (v_axis, u_axis)
            };
            let mut corners = [[0.0f32; 3]; 4];
            for (corner, (su, sv)) in
                corners
                    .iter_mut()
                    .zip([(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)])
            {
                let mut p = [0.0f32; 3];
                p[axis] = sign * (inner[axis] + r[axis]);
                p[u_axis] = su * inner[u_axis];
                p[v_axis] = sv * inner[v_axis];
                *corner = p;
            }
            let mut normal = [0.0f32; 3];
            normal[axis] = sign;
            push_quad(&mut positions, &mut normals, &mut indices, corners, normal);
        }
    }

    for a in 0..3usize {
        let b = (a + 1) % 3;
        let k = (a + 2) % 3;
        for da in [-1.0f32, 1.0f32] {
            for db in [-1.0f32, 1.0f32] {
                let base = positions.len() as u32;
                for t in 0..=segments {
                    let mut normal = [0.0f32; 3];
                    normal[a] = da * r[b] * cos_t[t];
                    normal[b] = db * r[a] * sin_t[t];
                    let normal_len =
                        (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                            .sqrt();
                    normal[0] /= normal_len;
                    normal[1] /= normal_len;
                    normal[2] /= normal_len;
                    for ksign in [-1.0f32, 1.0f32] {
                        let mut p = [0.0f32; 3];
                        p[a] = da * (inner[a] + r[a] * cos_t[t]);
                        p[b] = db * (inner[b] + r[b] * sin_t[t]);
                        p[k] = ksign * inner[k];
                        positions.push(p);
                        normals.push(normal);
                    }
                }
                let swap_k = da * db < 0.0;
                for t in 0..segments {
                    let low0 = base + (t * 2) as u32;
                    let low1 = base + ((t + 1) * 2) as u32;
                    let high0 = low0 + 1;
                    let high1 = low1 + 1;
                    if swap_k {
                        indices.extend_from_slice(&[high0, high1, low1, high0, low1, low0]);
                    } else {
                        indices.extend_from_slice(&[low0, low1, high1, low0, high1, high0]);
                    }
                }
            }
        }
    }

    for sx in [-1.0f32, 1.0f32] {
        for sy in [-1.0f32, 1.0f32] {
            for sz in [-1.0f32, 1.0f32] {
                let base = positions.len() as u32;
                for i in 0..=segments {
                    for j in 0..=segments {
                        let n = [sin_t[j] * cos_t[i], sin_t[j] * sin_t[i], cos_t[j]];
                        let p = [
                            sx * (inner[0] + r[0] * n[0]),
                            sy * (inner[1] + r[1] * n[1]),
                            sz * (inner[2] + r[2] * n[2]),
                        ];
                        positions.push(p);
                        let normal = [n[0] / r[0], n[1] / r[1], n[2] / r[2]];
                        let normal_len =
                            (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                                .sqrt();
                        normals.push([
                            sx * normal[0] / normal_len,
                            sy * normal[1] / normal_len,
                            sz * normal[2] / normal_len,
                        ]);
                    }
                }
                let flip = sx * sy * sz < 0.0;
                for i in 0..segments {
                    for j in 0..segments {
                        let v00 = base + (i * (segments + 1) + j) as u32;
                        let v01 = base + (i * (segments + 1) + j + 1) as u32;
                        let v10 = base + ((i + 1) * (segments + 1) + j) as u32;
                        let v11 = base + ((i + 1) * (segments + 1) + j + 1) as u32;
                        if j == 0 {
                            if flip {
                                indices.extend_from_slice(&[v11, v01, v00]);
                            } else {
                                indices.extend_from_slice(&[v00, v01, v11]);
                            }
                        } else if flip {
                            indices.extend_from_slice(&[v11, v01, v00, v10, v11, v00]);
                        } else {
                            indices.extend_from_slice(&[v00, v01, v11, v00, v11, v10]);
                        }
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn block_mesh_for_scale(
    cache: &mut BrickMaterialCache,
    meshes: &mut Assets<Mesh>,
    scale: Vec3,
) -> Handle<Mesh> {
    let key = brick_scale_key(scale);
    if let Some(existing) = cache.block_meshes.get(&key) {
        existing.clone()
    } else {
        let mesh = meshes.add(block_brick_mesh(scale));
        cache.block_meshes.insert(key, mesh.clone());
        mesh
    }
}

pub fn update_brick_meshes_on_shape_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<BrickMaterialCache>,
    bricks: Query<
        (
            Entity,
            &components::BrickShapeComponent,
            &Transform,
            Option<&components::BrickMeshKey>,
        ),
        Or<(
            Changed<components::BrickShapeComponent>,
            Changed<Transform>,
            Added<components::BrickMeshKey>,
        )>,
    >,
) {
    for (entity, brick_shape_comp, transform, mesh_key) in &bricks {
        let scale_key = brick_scale_key(transform.scale);
        if mesh_key
            .is_some_and(|key| key.shape == brick_shape_comp.shape && key.scale_key == scale_key)
        {
            continue;
        }
        let mesh_handle = match brick_shape_comp.shape {
            components::BrickShape::Block => {
                block_mesh_for_scale(&mut cache, &mut meshes, transform.scale)
            }
            components::BrickShape::Sphere => {
                if cache.sphere_mesh.is_none() {
                    cache.sphere_mesh = Some(meshes.add(Sphere::new(1.0 * 0.28)));
                }
                cache.sphere_mesh.clone().unwrap()
            }
            components::BrickShape::Cylinder => {
                if cache.cylinder_mesh.is_none() {
                    cache.cylinder_mesh = Some(meshes.add(cylinder_brick_mesh()));
                }
                cache.cylinder_mesh.clone().unwrap()
            }
            components::BrickShape::Wedge => {
                if cache.wedge_mesh.is_none() {
                    cache.wedge_mesh = Some(meshes.add(wedge_brick_mesh()));
                }
                cache.wedge_mesh.clone().unwrap()
            }
            components::BrickShape::CornerWedge => {
                if cache.corner_wedge_mesh.is_none() {
                    cache.corner_wedge_mesh = Some(meshes.add(corner_wedge_brick_mesh()));
                }
                cache.corner_wedge_mesh.clone().unwrap()
            }
        };
        commands.entity(entity).insert(Mesh3d(mesh_handle));
        commands.entity(entity).insert(components::BrickMeshKey {
            shape: brick_shape_comp.shape,
            scale_key,
        });
    }
}

#[cfg(feature = "bench")]
fn spawn_bricks_benchmark(mut commands: Commands) {
    use avian3d::prelude::*;

    commands.spawn((
        Name::new("BenchGround"),
        Transform::from_xyz(0.0, -0.56, 0.0),
        RigidBody::Static,
        Collider::cuboid(24.0, 0.56, 24.0),
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
    ));

    let mut index = 0u32;
    for x in 0..12u32 {
        for z in 0..12u32 {
            for y in 0..4u32 {
                let pos = Vec3::new(
                    (x as f32 - 5.5) * 1.2,
                    0.6 + y as f32 * 0.34,
                    (z as f32 - 5.5) * 0.65,
                );
                let shape = match index % 5 {
                    0 => components::BrickShape::Sphere,
                    1 => components::BrickShape::Cylinder,
                    2 => components::BrickShape::Wedge,
                    3 => components::BrickShape::CornerWedge,
                    _ => components::BrickShape::Block,
                };
                let collider = brick_collider_for_shape(shape);
                commands.spawn((
                    Name::new(format!("BenchBrick{}", index)),
                    components::Brick,
                    components::BrickShapeComponent { shape },
                    components::BrickPhysics::default(),
                    components::BrickColor::default(),
                    Transform::from_translation(pos),
                    RigidBody::Dynamic,
                    collider,
                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                ));
                index += 1;
            }
        }
    }
    info!("BENCH: Spawned {} dynamic bricks", index);
}

#[cfg(feature = "bench")]
fn toggle_brick_shapes(
    mut frame: Local<u64>,
    mut query: Query<&mut components::BrickShapeComponent, With<components::Brick>>,
) {
    *frame += 1;
    if *frame % 15 != 0 {
        return;
    }
    for (i, mut shape) in query.iter_mut().enumerate() {
        if i % 4 == 0 {
            shape.shape = match shape.shape {
                components::BrickShape::Block => components::BrickShape::Sphere,
                components::BrickShape::Sphere => components::BrickShape::Cylinder,
                components::BrickShape::Cylinder => components::BrickShape::Wedge,
                components::BrickShape::Wedge => components::BrickShape::CornerWedge,
                components::BrickShape::CornerWedge => components::BrickShape::Block,
            };
        }
    }
}

#[cfg(feature = "bench")]
fn record_bricks_assets(
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    mut stats: ResMut<crate::common::core::bench::BenchStats>,
) {
    stats.set_asset_counts(meshes.len(), materials.len());
}

#[cfg(feature = "bench")]
pub fn add_bricks_benchmark(app: &mut App) {
    app.insert_resource(crate::server::ServerSettings {
        map_path: String::new(),
        port: 0,
        bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        netcode_key: [0u8; 32],
        protocol_id: 0,
        allow_unauthenticated: true,
    })
    .init_resource::<BrickMaterialCache>()
    .init_asset::<StandardMaterial>()
    .add_systems(Startup, spawn_bricks_benchmark)
    .add_systems(
        Update,
        (update_brick_meshes_on_shape_change, toggle_brick_shapes),
    )
    .add_systems(
        Last,
        record_bricks_assets.before(crate::common::core::bench::bench_finish_frame),
    );
}

const LOD_CAMERA_MOVE_SQ: f32 = 16.0 * 16.0;

pub fn swap_brick_material(
    commands: &mut Commands,
    entity: Entity,
    want_studs: bool,
    cache: &mut BrickMaterialCache,
    studs_materials: &mut Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>,
    plain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    studs_assets: &studs::StudsAssets,
    base_color: Color,
) {
    let mut cmd = commands.entity(entity);
    if want_studs {
        cmd.try_remove::<MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>>();
        cmd.try_insert(MeshMaterial3d(studs_material_for_color(
            cache,
            studs_materials,
            studs_assets,
            base_color,
        )));
    } else {
        cmd.try_remove::<MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>();
        cmd.try_insert(MeshMaterial3d(plain_material_for_color(
            cache,
            plain_materials,
            base_color,
        )));
    }
}

pub fn sync_transparent_brick_shadow_receive(
    mut commands: Commands,
    query: Query<
        (Entity, &components::BrickColor, Option<&NotShadowReceiver>),
        (With<components::Brick>, Changed<components::BrickColor>),
    >,
) {
    for (entity, color, no_receive) in &query {
        let transparent = color.color.to_srgba().alpha < 1.0;
        if transparent && no_receive.is_none() {
            commands.entity(entity).insert(NotShadowReceiver);
        } else if !transparent && no_receive.is_some() {
            commands.entity(entity).remove::<NotShadowReceiver>();
        }
    }
}

pub fn optimize_brick_visibility(
    mut commands: Commands,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    mut plain_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
    >,
    studs_assets: Res<studs::StudsAssets>,
    camera_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    bricks_query: Query<
        (
            Entity,
            &GlobalTransform,
            &components::BrickColor,
            &components::BrickShapeComponent,
            Option<&components::BrickStuds>,
            Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
            Option<
                &MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>,
            >,
            Option<&NotShadowCaster>,
            Option<&Visibility>,
        ),
        With<components::Brick>,
    >,
    workspace_studs: Option<Res<WorkspaceShowStuds>>,
    mut cache: ResMut<BrickMaterialCache>,
    mut last_camera_position: Local<Option<Vec3>>,
    graphics_settings: Option<Res<crate::common::core::performance::GraphicsSettings>>,
) {
    let Some((camera_transform, _)) = camera_query.iter().find(|(_, cam)| cam.is_active)
    else {
        return;
    };

    let (shadow_lod_distance, hide_lod_distance) = graphics_settings
        .map(|settings| settings.view_distance.brick_lod_distances())
        .unwrap_or((80.0, 160.0));

    let cam_pos = camera_transform.translation();
    let show_studs_globally = workspace_studs.as_ref().map(|w| w.enabled).unwrap_or(true);
    let workspace_changed = workspace_studs
        .as_ref()
        .map(|w| w.is_changed())
        .unwrap_or(false);
    let moved = last_camera_position
        .map(|previous| previous.distance_squared(cam_pos) > LOD_CAMERA_MOVE_SQ)
        .unwrap_or(true);
    if moved {
        *last_camera_position = Some(cam_pos);
    }
    let skip_distance_lod = !moved && !workspace_changed;

    for (
        entity,
        transform,
        color,
        shape_comp,
        studs,
        studs_material,
        plain_material,
        not_shadow_caster,
        visibility,
    ) in &bricks_query
    {
        let has_studs = studs_material.is_some();
        let has_plain = plain_material.is_some();
        let brick_wants_studs = studs.map(|s| s.enabled).unwrap_or(true);
        let want_studs = show_studs_globally && brick_wants_studs;

        if want_studs != has_studs || (!has_studs && !has_plain) {
            swap_brick_material(
                &mut commands,
                entity,
                want_studs,
                &mut cache,
                &mut studs_materials,
                &mut plain_materials,
                &studs_assets,
                color.color,
            );
        }

        if skip_distance_lod {
            continue;
        }

        let brick_radius =
            (transform.scale() * brick_bounding_half_extents(shape_comp.shape)).length();
        let dist_sq = transform.translation().distance_squared(cam_pos);
        let is_visible = visibility.is_none_or(|v| *v != Visibility::Hidden);
        let hide_eff = hide_lod_distance * if is_visible { 1.1 } else { 0.9 } + brick_radius;
        let want_visible = dist_sq <= hide_eff * hide_eff;
        if !want_visible && !is_visible {
            continue;
        }

        let shadow_eff = shadow_lod_distance
            * if not_shadow_caster.is_some() { 0.9 } else { 1.1 }
            + brick_radius;
        let want_shadow_caster = dist_sq <= shadow_eff * shadow_eff;
        if want_shadow_caster == not_shadow_caster.is_some() {
            if want_shadow_caster {
                commands.entity(entity).remove::<NotShadowCaster>();
            } else {
                commands.entity(entity).insert(NotShadowCaster);
            }
        }

        if want_visible != is_visible {
            if want_visible {
                commands.entity(entity).insert(Visibility::Inherited);
            } else {
                commands.entity(entity).insert(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

    #[test]
    fn brick_finish_matches_satin_plastic() {
        assert!(BRICK_PERCEPTUAL_ROUGHNESS <= 0.6);
        assert!(BRICK_PERCEPTUAL_ROUGHNESS >= 0.4);
        assert_eq!(BRICK_METALLIC, 0.0);
        assert!(BRICK_REFLECTANCE >= 0.4 && BRICK_REFLECTANCE <= 0.6);
    }

    #[test]
    fn studs_fade_before_shadow_and_hide_lod() {
        for distance in [
            crate::common::core::performance::ViewDistance::Low,
            crate::common::core::performance::ViewDistance::Medium,
            crate::common::core::performance::ViewDistance::High,
        ] {
            let (shadow, hide) = distance.brick_lod_distances();
            assert!(shadow > 0.0);
            assert!(hide > shadow);
            assert!(STUD_FADE_END_DISTANCE < shadow);
        }
    }

    #[test]
    fn studs_shader_preserves_base_color_when_faded() {
        let wgsl = include_str!("../../../../assets/shaders/studs.wgsl");
        assert!(wgsl.contains("fade > 0.0005"));
        assert!(wgsl.contains("mix(in.world_normal"));
        assert!(wgsl.contains("stud_mask"));
        assert!(wgsl.contains("* fade"));
        assert!(wgsl.contains("inlet_mask"));
    }

    #[test]
    fn studs_shader_keeps_brick_finish() {
        let wgsl = include_str!("../../../../assets/shaders/studs.wgsl");
        assert!(!wgsl.contains("perceptual_roughness"));
        assert!(!wgsl.contains("metallic"));
        assert!(!wgsl.contains("reflectance"));
    }

    #[test]
    fn studs_shader_softens_steep_normals() {
        let wgsl = include_str!("../../../../assets/shaders/studs.wgsl");
        assert!(wgsl.contains("raw_normal_ts"));
        assert!(wgsl.contains("* 0.55"));
    }

    #[test]
    fn stud_fade_end_matches_shader() {
        let wgsl = include_str!("../../../../assets/shaders/studs.wgsl");
        let line = wgsl
            .lines()
            .find(|l| l.contains("- dist) /"))
            .expect("studs fade");
        let end: f32 = line
            .split("- dist")
            .next()
            .unwrap()
            .rsplit('(')
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!((end - STUD_FADE_END_DISTANCE).abs() < 1e-6);
    }

    fn read_mesh_attributes(mesh: &Mesh) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(values) => values.clone(),
            other => panic!("unexpected position format: {:?}", other),
        };
        let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap() {
            VertexAttributeValues::Float32x3(values) => values.clone(),
            other => panic!("unexpected normal format: {:?}", other),
        };
        let indices = match mesh.indices().unwrap() {
            Indices::U32(values) => values.clone(),
            other => panic!("unexpected index format: {:?}", other),
        };
        (positions, normals, indices)
    }

    #[test]
    fn transparent_bricks_skip_shadow_receive() {
        let mut app = App::new();
        app.add_systems(Update, sync_transparent_brick_shadow_receive);
        let opaque = app
            .world_mut()
            .spawn((
                components::Brick,
                components::BrickColor {
                    color: Color::srgb(0.84, 0.24, 0.16),
                },
            ))
            .id();
        let faded = app
            .world_mut()
            .spawn((
                components::Brick,
                components::BrickColor {
                    color: Color::srgba(0.84, 0.24, 0.16, 0.5),
                },
            ))
            .id();
        app.update();
        assert!(app.world().get::<NotShadowReceiver>(opaque).is_none());
        assert!(app.world().get::<NotShadowReceiver>(faded).is_some());

        app.world_mut()
            .get_mut::<components::BrickColor>(opaque)
            .unwrap()
            .color = Color::srgba(0.84, 0.24, 0.16, 0.25);
        app.update();
        assert!(app.world().get::<NotShadowReceiver>(opaque).is_some());

        app.world_mut()
            .get_mut::<components::BrickColor>(opaque)
            .unwrap()
            .color = Color::srgb(0.84, 0.24, 0.16);
        app.update();
        assert!(app.world().get::<NotShadowReceiver>(opaque).is_none());
    }

    #[test]
    fn beveled_block_mesh_has_expected_topology() {
        let mesh = block_brick_mesh(Vec3::ONE);
        let (positions, normals, indices) = read_mesh_attributes(&mesh);
        let s = BLOCK_EDGE_SEGMENTS;
        let expected_vertices = 6 * 4 + 12 * 2 * (s + 1) + 8 * (s + 1) * (s + 1);
        let expected_indices = 6 * 6 + 12 * s * 6 + 8 * (s * s * 6 - s * 3);
        assert_eq!(positions.len(), expected_vertices);
        assert_eq!(normals.len(), expected_vertices);
        assert_eq!(indices.len(), expected_indices);
    }

    fn assert_winding_matches_normals(scale: Vec3) {
        let mesh = block_brick_mesh(scale);
        let (positions, normals, indices) = read_mesh_attributes(&mesh);
        for tri in indices.chunks(3) {
            let a = Vec3::from(positions[tri[0] as usize]);
            let b = Vec3::from(positions[tri[1] as usize]);
            let c = Vec3::from(positions[tri[2] as usize]);
            let face_normal = (b - a).cross(c - a).normalize();
            assert!(face_normal.is_finite());
            let mut average_normal = Vec3::ZERO;
            for vertex_index in tri {
                let vertex_normal = Vec3::from(normals[*vertex_index as usize]);
                assert!(vertex_normal.length() > 0.999 && vertex_normal.length() < 1.001);
                average_normal += vertex_normal;
            }
            assert!(face_normal.dot(average_normal.normalize()) > 0.9);
        }
    }

    #[test]
    fn beveled_block_mesh_winding_matches_normals() {
        assert_winding_matches_normals(Vec3::ONE);
        assert_winding_matches_normals(Vec3::new(2.0, 2.0, 2.0));
        assert_winding_matches_normals(Vec3::new(2.5, 1.0, 1.5));
    }

    #[test]
    fn beveled_block_mesh_stays_inside_box() {
        for scale in [
            Vec3::ONE,
            Vec3::new(3.0, 3.0, 3.0),
            Vec3::new(2.5, 1.0, 1.5),
        ] {
            let mesh = block_brick_mesh(scale);
            let (positions, _, _) = read_mesh_attributes(&mesh);
            for p in positions {
                assert!(p[0].abs() <= BLOCK_HALF_EXTENTS[0] + 1e-5);
                assert!(p[1].abs() <= BLOCK_HALF_EXTENTS[1] + 1e-5);
                assert!(p[2].abs() <= BLOCK_HALF_EXTENTS[2] + 1e-5);
            }
        }
    }

    #[test]
    fn beveled_block_mesh_rounds_edges() {
        let mesh = block_brick_mesh(Vec3::ONE);
        let (positions, _, _) = read_mesh_attributes(&mesh);
        let h = BLOCK_HALF_EXTENTS;
        let r = [
            (BLOCK_EDGE_RADIUS / 1.0).min(h[0]),
            (BLOCK_EDGE_RADIUS / 1.0).min(h[1]),
            (BLOCK_EDGE_RADIUS / 1.0).min(h[2]),
        ];
        let inner = [h[0] - r[0], h[1] - r[1], h[2] - r[2]];
        let contains = |p: [f32; 3]| {
            positions.iter().any(|v| {
                (v[0] - p[0]).abs() < 1e-5
                    && (v[1] - p[1]).abs() < 1e-5
                    && (v[2] - p[2]).abs() < 1e-5
            })
        };
        assert!(contains([h[0], inner[1], inner[2]]));
        let c = std::f32::consts::FRAC_1_SQRT_2;
        assert!(contains([
            inner[0] + r[0] * c,
            inner[1] + r[1] * c,
            inner[2]
        ]));
        let alpha = std::f32::consts::FRAC_PI_2 / BLOCK_EDGE_SEGMENTS as f32;
        let n = [
            alpha.sin() * alpha.cos(),
            alpha.sin() * alpha.sin(),
            alpha.cos(),
        ];
        assert!(contains([
            inner[0] + r[0] * n[0],
            inner[1] + r[1] * n[1],
            inner[2] + r[2] * n[2]
        ]));
        assert!(!contains([h[0], h[1], h[2]]));
    }

    #[test]
    fn beveled_block_mesh_keeps_constant_world_radius() {
        for scale in [
            Vec3::ONE,
            Vec3::new(3.0, 3.0, 3.0),
            Vec3::new(2.5, 1.0, 1.5),
        ] {
            let mesh = block_brick_mesh(scale);
            let (positions, _, _) = read_mesh_attributes(&mesh);
            let h = BLOCK_HALF_EXTENTS;
            let r = BLOCK_EDGE_RADIUS
                .min(scale.x * h[0])
                .min(scale.y * h[1])
                .min(scale.z * h[2]);
            let world = positions
                .iter()
                .map(|p| [p[0] * scale.x, p[1] * scale.y, p[2] * scale.z])
                .collect::<Vec<_>>();
            let c = std::f32::consts::FRAC_1_SQRT_2;
            let expected = [
                scale.x * h[0] - r + r * c,
                scale.y * h[1] - r + r * c,
                scale.z * (h[2] - r / scale.z),
            ];
            assert!(world.iter().any(|v| {
                (v[0] - expected[0]).abs() < 1e-5
                    && (v[1] - expected[1]).abs() < 1e-5
                    && (v[2] - expected[2]).abs() < 1e-5
            }));
        }
    }

    fn assert_flat_mesh_winding_matches_normals(mesh: &Mesh) {
        let (positions, normals, indices) = read_mesh_attributes(mesh);
        for tri in indices.chunks(3) {
            let a = Vec3::from(positions[tri[0] as usize]);
            let b = Vec3::from(positions[tri[1] as usize]);
            let c = Vec3::from(positions[tri[2] as usize]);
            let face_normal = (b - a).cross(c - a).normalize();
            assert!(face_normal.is_finite());
            let mut average_normal = Vec3::ZERO;
            for vertex_index in tri {
                let vertex_normal = Vec3::from(normals[*vertex_index as usize]);
                assert!(vertex_normal.length() > 0.999 && vertex_normal.length() < 1.001);
                average_normal += vertex_normal;
            }
            assert!(face_normal.dot(average_normal.normalize()) > 0.9);
        }
    }

    fn assert_mesh_inside_half_extents(mesh: &Mesh, half: Vec3) {
        let (positions, _, _) = read_mesh_attributes(mesh);
        for p in positions {
            assert!(p[0].abs() <= half.x + 1e-5);
            assert!(p[1].abs() <= half.y + 1e-5);
            assert!(p[2].abs() <= half.z + 1e-5);
        }
    }

    #[test]
    fn wedge_mesh_matches_roblox_ramp_geometry() {
        let mesh = wedge_brick_mesh();
        let half = Vec3::new(
            BLOCK_HALF_EXTENTS[0],
            BLOCK_HALF_EXTENTS[1],
            BLOCK_HALF_EXTENTS[2],
        );
        assert_mesh_inside_half_extents(&mesh, half);
        assert_flat_mesh_winding_matches_normals(&mesh);
        let (positions, _, _) = read_mesh_attributes(&mesh);
        let top_count = positions
            .iter()
            .filter(|p| (p[1] - half.y).abs() < 1e-5)
            .count();
        assert!(top_count > 0);
        for p in positions.iter().filter(|p| (p[1] - half.y).abs() < 1e-5) {
            assert!((p[2] - half.z).abs() < 1e-5);
        }
        let bottom_count = positions
            .iter()
            .filter(|p| (p[1] + half.y).abs() < 1e-5)
            .count();
        assert!(bottom_count >= 4);
    }

    #[test]
    fn corner_wedge_mesh_matches_roblox_corner_geometry() {
        let mesh = corner_wedge_brick_mesh();
        let half = Vec3::new(
            BLOCK_HALF_EXTENTS[0],
            BLOCK_HALF_EXTENTS[1],
            BLOCK_HALF_EXTENTS[2],
        );
        assert_mesh_inside_half_extents(&mesh, half);
        assert_flat_mesh_winding_matches_normals(&mesh);
        let (positions, _, _) = read_mesh_attributes(&mesh);
        let apex = [half.x, half.y, -half.z];
        assert!(positions.iter().any(|v| {
            (v[0] - apex[0]).abs() < 1e-5
                && (v[1] - apex[1]).abs() < 1e-5
                && (v[2] - apex[2]).abs() < 1e-5
        }));
        let top_count = positions
            .iter()
            .filter(|p| (p[1] - half.y).abs() < 1e-5)
            .count();
        assert_eq!(top_count, 4);
    }

    #[test]
    fn cylinder_mesh_axis_matches_roblox_x_axis() {
        let mesh = cylinder_brick_mesh();
        let (positions, _, _) = read_mesh_attributes(&mesh);
        let min_x = positions.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = positions.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((min_x + CYLINDER_HALF_HEIGHT).abs() < 1e-4);
        assert!((max_x - CYLINDER_HALF_HEIGHT).abs() < 1e-4);
        for p in &positions {
            let radial = (p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!(radial <= CYLINDER_RADIUS + 1e-4);
        }
        let half = Vec3::new(
            CYLINDER_HALF_HEIGHT,
            CYLINDER_RADIUS,
            CYLINDER_RADIUS,
        );
        assert_mesh_inside_half_extents(&mesh, half);
    }

    #[test]
    fn brick_colliders_are_exact_convex_shapes() {
        use avian3d::parry::shape::TypedShape;
        use avian3d::prelude::SimpleCollider;
        let block = brick_collider_for_shape(components::BrickShape::Block);
        assert!(matches!(
            block.shape().as_typed_shape(),
            TypedShape::Cuboid(_)
        ));
        let ball = brick_collider_for_shape(components::BrickShape::Sphere);
        assert!(matches!(
            ball.shape().as_typed_shape(),
            TypedShape::Ball(_)
        ));
        let cylinder = brick_collider_for_shape(components::BrickShape::Cylinder);
        assert!(matches!(
            cylinder.shape().as_typed_shape(),
            TypedShape::Compound(_)
        ));
        let wedge = brick_collider_for_shape(components::BrickShape::Wedge);
        assert!(matches!(
            wedge.shape().as_typed_shape(),
            TypedShape::ConvexPolyhedron(_)
        ));
        let corner = brick_collider_for_shape(components::BrickShape::CornerWedge);
        assert!(matches!(
            corner.shape().as_typed_shape(),
            TypedShape::ConvexPolyhedron(_)
        ));
        for shape in [
            components::BrickShape::Block,
            components::BrickShape::Wedge,
            components::BrickShape::CornerWedge,
        ] {
            let collider = brick_collider_for_shape(shape);
            let aabb = collider.aabb(Vec3::ZERO, Quat::IDENTITY);
            let half = brick_bounding_half_extents(shape);
            assert!((aabb.min + half).length() < 1e-3);
            assert!((aabb.max - half).length() < 1e-3);
        }
    }

    #[test]
    fn brick_shape_serialization_round_trips() {
        for shape in [
            components::BrickShape::Block,
            components::BrickShape::Sphere,
            components::BrickShape::Cylinder,
            components::BrickShape::Wedge,
            components::BrickShape::CornerWedge,
        ] {
            assert_eq!(
                components::BrickShape::from_u8(shape.to_u8()),
                Some(shape)
            );
            assert_eq!(
                components::BrickShape::from_name(shape.display_name()),
                Some(shape)
            );
        }
        assert_eq!(components::BrickShape::from_name("Ball"), Some(components::BrickShape::Sphere));
        assert_eq!(components::BrickShape::from_name("Cube"), Some(components::BrickShape::Block));
        assert_eq!(components::BrickShape::from_u8(42), None);
    }
}
