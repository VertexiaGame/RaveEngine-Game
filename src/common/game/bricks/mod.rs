pub mod components;
pub mod studs;
pub mod data;

use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin};
use bevy::light::NotShadowCaster;
use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, PrimitiveTopology};

#[derive(Resource, Default)]
pub struct BrickMaterialCache {
    pub studs_materials: std::collections::HashMap<[u8; 4], Handle<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    pub plain_materials: std::collections::HashMap<[u8; 4], Handle<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>>,
    pub block_meshes: std::collections::HashMap<[u32; 3], Handle<Mesh>>,
    pub sphere_mesh: Option<Handle<Mesh>>,
}

#[derive(Resource)]
pub struct WorkspaceShowStuds {
    pub enabled: bool,
}

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
            app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>::default())
                .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>::default())
                .add_systems(Startup, studs::setup_studs)
                .add_systems(Update, (
                    studs::configure_studs_samplers,
                    update_brick_meshes_on_shape_change,
                    apply_workspace_show_studs,
                    links_optimizer_system,
                    optimize_brick_visibility,
                ));
        } else {
            app.add_systems(Update, apply_workspace_show_studs);
        }
    }
}

pub fn apply_workspace_show_studs(
    workspace: Res<WorkspaceShowStuds>,
    mut commands: Commands,
    query: Query<(Entity, Option<&components::BrickStuds>), With<components::Brick>>,
) {
    if !workspace.is_changed() {
        return;
    }
    for (entity, studs) in &query {
        let enabled = studs.map(|s| s.enabled).unwrap_or(true);
        commands.entity(entity).insert(components::BrickStuds { enabled });
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
                perceptual_roughness: 0.85,
                alpha_mode: if quantized.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                ..default()
            },
            extension: studs::StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
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
                perceptual_roughness: 0.85,
                alpha_mode: if quantized.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
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

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

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
            for (corner, (su, sv)) in corners
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
                    let normal_len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
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
                        let normal_len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
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

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
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
    bricks: Query<(
        Entity,
        &components::BrickShapeComponent,
        &Transform,
        Option<&components::BrickMeshKey>,
    ), Or<(
        Changed<components::BrickShapeComponent>,
        Changed<Transform>,
        Added<components::BrickMeshKey>,
    )>>,
) {
    for (entity, brick_shape_comp, transform, mesh_key) in &bricks {
        let scale_key = brick_scale_key(transform.scale);
        if mesh_key.is_some_and(|key| key.shape == brick_shape_comp.shape && key.scale_key == scale_key) {
            continue;
        }
        let mesh_handle = match brick_shape_comp.shape {
            components::BrickShape::Block => block_mesh_for_scale(&mut cache, &mut meshes, transform.scale),
            components::BrickShape::Sphere => {
                if cache.sphere_mesh.is_none() {
                    cache.sphere_mesh = Some(meshes.add(Sphere::new(1.0 * 0.28)));
                }
                cache.sphere_mesh.clone().unwrap()
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
                let shape = if index % 9 == 0 {
                    components::BrickShape::Sphere
                } else {
                    components::BrickShape::Block
                };
                let collider = match shape {
                    components::BrickShape::Block => Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
                    components::BrickShape::Sphere => Collider::sphere(1.0 * 0.28),
                };
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
            shape.shape = if shape.shape == components::BrickShape::Block {
                components::BrickShape::Sphere
            } else {
                components::BrickShape::Block
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
    app.init_resource::<BrickMaterialCache>()
        .init_asset::<StandardMaterial>()
        .add_systems(Startup, spawn_bricks_benchmark)
        .add_systems(Update, (
            update_brick_meshes_on_shape_change,
            toggle_brick_shapes,
        ))
        .add_systems(Last, record_bricks_assets.before(crate::common::core::bench::bench_finish_frame));
}

fn links_optimizer_system() {} // dummy hook for common optimization module

const STUD_LOD_DISTANCE_SQ: f32 = 28.0 * 28.0;
const SHADOW_LOD_DISTANCE_SQ: f32 = 80.0 * 80.0;
const HIDE_LOD_DISTANCE_SQ: f32 = 160.0 * 160.0;
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
        cmd.try_insert(MeshMaterial3d(
            studs_material_for_color(cache, studs_materials, studs_assets, base_color),
        ));
    } else {
        cmd.try_remove::<MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>();
        cmd.try_insert(MeshMaterial3d(
            plain_material_for_color(cache, plain_materials, base_color),
        ));
    }
}

pub fn optimize_brick_visibility(
    mut commands: Commands,
    mut studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
    mut plain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>>,
    studs_assets: Res<studs::StudsAssets>,
    camera_query: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickColor,
        Option<&components::BrickStuds>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::StudsExtension>>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, studs::ShadowOpacityExtension>>>,
        Option<&NotShadowCaster>,
        Option<&Visibility>,
    ), With<components::Brick>>,
    workspace_studs: Option<Res<WorkspaceShowStuds>>,
    mut cache: ResMut<BrickMaterialCache>,
    mut last_camera_position: Local<Option<Vec3>>,
) {
    let Some((camera_transform, camera)) = camera_query.iter().next() else {
        return;
    };
    if !camera.is_active {
        return;
    }

    let cam_pos = camera_transform.translation();
    let show_studs_globally = workspace_studs.as_ref().map(|w| w.enabled).unwrap_or(true);
    let workspace_changed = workspace_studs.as_ref().map(|w| w.is_changed()).unwrap_or(false);
    let moved = last_camera_position
        .map(|previous| previous.distance_squared(cam_pos) > LOD_CAMERA_MOVE_SQ)
        .unwrap_or(true);
    *last_camera_position = Some(cam_pos);
    if !moved && !workspace_changed {
        return;
    }

    for (entity, transform, color, studs, studs_material, plain_material, not_shadow_caster, visibility) in &bricks_query {
        let dist_sq = transform.translation().distance_squared(cam_pos);
        let brick_wants_studs = studs.map(|s| s.enabled).unwrap_or(true);
        let want_studs = show_studs_globally && brick_wants_studs && dist_sq <= STUD_LOD_DISTANCE_SQ;

        if want_studs != studs_material.is_some() {
            let base_color = if let Some(studs_mat_handle) = studs_material {
                studs_materials
                    .get(&studs_mat_handle.0)
                    .map(|mat| mat.base.base_color)
                    .unwrap_or(color.color)
            } else if let Some(plain_mat_handle) = plain_material {
                plain_materials
                    .get(&plain_mat_handle.0)
                    .map(|mat| mat.base.base_color)
                    .unwrap_or(color.color)
            } else {
                color.color
            };
            swap_brick_material(
                &mut commands,
                entity,
                want_studs,
                &mut cache,
                &mut studs_materials,
                &mut plain_materials,
                &studs_assets,
                base_color,
            );
        }

        let want_shadow_caster = dist_sq <= SHADOW_LOD_DISTANCE_SQ;
        if want_shadow_caster == not_shadow_caster.is_some() {
            if want_shadow_caster {
                commands.entity(entity).remove::<NotShadowCaster>();
            } else {
                commands.entity(entity).insert(NotShadowCaster);
            }
        }

        let want_visible = dist_sq <= HIDE_LOD_DISTANCE_SQ;
        let is_visible = visibility.is_none_or(|v| *v != Visibility::Hidden);
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
        for scale in [Vec3::ONE, Vec3::new(3.0, 3.0, 3.0), Vec3::new(2.5, 1.0, 1.5)] {
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
                (v[0] - p[0]).abs() < 1e-5 && (v[1] - p[1]).abs() < 1e-5 && (v[2] - p[2]).abs() < 1e-5
            })
        };
        assert!(contains([h[0], inner[1], inner[2]]));
        let c = std::f32::consts::FRAC_1_SQRT_2;
        assert!(contains([inner[0] + r[0] * c, inner[1] + r[1] * c, inner[2]]));
        let alpha = std::f32::consts::FRAC_PI_2 / BLOCK_EDGE_SEGMENTS as f32;
        let n = [alpha.sin() * alpha.cos(), alpha.sin() * alpha.sin(), alpha.cos()];
        assert!(contains([inner[0] + r[0] * n[0], inner[1] + r[1] * n[1], inner[2] + r[2] * n[2]]));
        assert!(!contains([h[0], h[1], h[2]]));
    }

    #[test]
    fn beveled_block_mesh_keeps_constant_world_radius() {
        for scale in [Vec3::ONE, Vec3::new(3.0, 3.0, 3.0), Vec3::new(2.5, 1.0, 1.5)] {
            let mesh = block_brick_mesh(scale);
            let (positions, _, _) = read_mesh_attributes(&mesh);
            let h = BLOCK_HALF_EXTENTS;
            let r = BLOCK_EDGE_RADIUS.min(scale.x * h[0]).min(scale.y * h[1]).min(scale.z * h[2]);
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
}
