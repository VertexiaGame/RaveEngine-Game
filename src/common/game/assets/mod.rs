pub mod components;
pub mod fetch;
pub mod meshdata;
pub mod status;

use std::collections::{HashMap, HashSet};

use crate::common::game::bricks::components::{Brick, BrickShape, BrickShapeComponent};
use avian3d::prelude::{Collider, TrimeshFlags};
use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy::log::{info, warn};
use bevy::pbr::StandardMaterial;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::TextureFormat;
use components::ImageFace;
use meshdata::{
    LoadedMeshAsset, LoadedMeshData, build_bevy_mesh, mesh_bounds, normalize_mesh_data,
};

const FACE_INSET: f32 = 0.04 * 0.28;
const FACE_OFFSET: f32 = 0.001;
const OVERLAY_THICKNESS: f32 = 0.001;
const BLOCK_MESH_HALF_EXTENTS: Vec3 = Vec3::new(2.0 * 0.28, 0.5 * 0.28, 1.0 * 0.28);

#[derive(Resource, Default)]
pub struct ImageAssetCache {
    pub images: HashMap<u32, Handle<Image>>,
    pub materials: HashMap<u32, Handle<StandardMaterial>>,
    pub plane_mesh: Option<Handle<Mesh>>,
}

#[derive(Resource, Default)]
pub struct PendingImageFetches(pub HashSet<u32>);

#[derive(Resource, Default)]
pub struct MeshAssetCache {
    pub meshes: HashMap<(u32, bool), Handle<Mesh>>,
    pub materials: HashMap<(u32, bool), Handle<StandardMaterial>>,
    pub meta: HashMap<u32, MeshAssetMeta>,
    /// Precise trimesh colliders built from the raw / normalized model data,
    /// keyed by (asset_id, normalize).
    pub colliders: HashMap<(u32, bool), Collider>,
}

#[derive(Clone, Debug)]
pub struct MeshAssetMeta {
    /// Natural size of the model in studs (world units / 0.28).
    pub size_studs: Vec3,
    /// Natural size of the normalized model in studs.
    pub normalized_size_studs: Vec3,
    /// Rotation inherited from the model itself (glb node transform).
    pub rotation: Quat,
}

#[derive(Resource, Default)]
pub struct PendingMeshFetches(pub HashSet<u32>);

#[derive(Resource, Default)]
pub struct PendingMeshTextureFetches(pub HashSet<u32>);

/// Cached uploaded texture sidecars of mesh assets, keyed by mesh asset id.
/// A `None` value means the sidecar was fetched and does not exist, so the
/// model's embedded texture should be used instead.
#[derive(Resource, Default)]
pub struct MeshTextureCache {
    pub sidecars: HashMap<u32, Option<Handle<Image>>>,
    /// Override materials built from a mesh's default material with a
    /// replaced base color texture, keyed by
    /// (mesh asset id, normalize, texture source id).
    pub materials: HashMap<(u32, bool, u32), Handle<StandardMaterial>>,
}

/// Tracks which mesh entities currently render with a texture-override
/// material so the default material can be restored when the override goes
/// away.
#[derive(Resource, Default)]
pub struct AppliedTextureOverrides(pub HashMap<Entity, Handle<StandardMaterial>>);

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::Image>()
            .register_type::<components::ImageFace>()
            .register_type::<components::Mesh>()
            .register_type::<components::Texture>()
            .init_resource::<ImageAssetCache>()
            .init_resource::<PendingImageFetches>()
            .init_resource::<MeshAssetCache>()
            .init_resource::<PendingMeshFetches>()
            .init_resource::<PendingMeshTextureFetches>()
            .init_resource::<MeshTextureCache>()
            .init_resource::<AppliedTextureOverrides>()
            .init_resource::<fetch::AssetFetchPool>()
            .init_resource::<status::AssetStatusCache>()
            .init_resource::<status::PendingStatusFetches>()
            .init_resource::<status::AssetStatusPool>();

        if !app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.init_asset::<StandardMaterial>();
            app.init_asset::<bevy::image::Image>();
        }

        app.add_systems(
            Update,
            (
                fetch_missing_meshes,
                apply_fetched_assets,
                apply_cached_mesh_visuals,
                fetch_missing_textures,
                apply_texture_overrides,
                status::request_missing_statuses,
                status::poll_status_results,
            ),
        );

        if app.is_plugin_added::<bevy::render::RenderPlugin>() {
            app.add_systems(
                Update,
                (
                    setup_image_meshes,
                    fetch_missing_images,
                    apply_cached_image_materials,
                    update_image_visuals,
                ),
            );
        }
    }
}

fn plane_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

fn setup_image_meshes(
    mut commands: Commands,
    mut cache: ResMut<ImageAssetCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Query<Entity, (With<components::Image>, Without<Mesh3d>)>,
) {
    if cache.plane_mesh.is_none() {
        cache.plane_mesh = Some(meshes.add(plane_mesh()));
    }
    if !cache.materials.contains_key(&0) {
        cache.materials.insert(
            0,
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                perceptual_roughness: 1.0,
                double_sided: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        );
    }
    let Some(handle) = cache.plane_mesh.clone() else {
        return;
    };
    for entity in &images {
        commands.entity(entity).insert(Mesh3d(handle.clone()));
    }
}

fn current_user_ukey(
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
) -> Option<String> {
    if let Some(ukey) = auth_store
        .as_ref()
        .and_then(|store| store.credentials.clone())
        .map(|creds| creds.ukey)
        .filter(|ukey| !ukey.is_empty())
    {
        return Some(ukey);
    }
    client_ukey
        .map(|ukey| ukey.0.clone())
        .filter(|ukey| !ukey.is_empty())
}

fn fetch_missing_images(
    images: Query<&components::Image>,
    cache: Res<ImageAssetCache>,
    mut pending: ResMut<PendingImageFetches>,
    mut pool: ResMut<fetch::AssetFetchPool>,
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
) {
    let user_ukey = current_user_ukey(auth_store, client_ukey);
    for image in &images {
        let asset_id = image.asset_id;
        if asset_id == 0 || cache.images.contains_key(&asset_id) || pending.0.contains(&asset_id) {
            continue;
        }
        pool.ensure_started();
        if pool.submit(asset_id, user_ukey.clone()) {
            pending.0.insert(asset_id);
        }
    }
}

fn apply_fetched_assets(
    mut commands: Commands,
    pool: Res<fetch::AssetFetchPool>,
    mut pending_images: ResMut<PendingImageFetches>,
    mut pending_meshes: ResMut<PendingMeshFetches>,
    mut pending_mesh_textures: ResMut<PendingMeshTextureFetches>,
    mut image_cache: ResMut<ImageAssetCache>,
    mut mesh_cache: ResMut<MeshAssetCache>,
    mut mesh_texture_cache: ResMut<MeshTextureCache>,
    mut image_assets: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Query<(Entity, &components::Image)>,
    mesh_entities: Query<(Entity, &components::Mesh)>,
    mut transforms: Query<&mut Transform>,
) {
    for (asset_id, kind, result) in pool.drain_results() {
        match kind {
            fetch::AssetKind::Image => {
                pending_images.0.remove(&asset_id);
                match result {
                    Ok(bytes) => {
                        let Some(image_handle) = decode_image_asset(&bytes, &mut image_assets)
                        else {
                            warn!("Asset {asset_id}: failed to decode image bytes");
                            continue;
                        };
                        let material = materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            base_color_texture: Some(image_handle.clone()),
                            perceptual_roughness: 1.0,
                            double_sided: true,
                            alpha_mode: AlphaMode::Blend,
                            ..default()
                        });
                        info!("Asset {asset_id}: loaded image");
                        image_cache.images.insert(asset_id, image_handle);
                        image_cache.materials.insert(asset_id, material.clone());
                        for (entity, image) in &images {
                            if image.asset_id == asset_id {
                                commands
                                    .entity(entity)
                                    .insert(MeshMaterial3d(material.clone()));
                            }
                        }
                    }
                    Err(e) => warn!("Asset {asset_id}: image fetch failed: {e}"),
                }
            }
            fetch::AssetKind::Mesh => {
                pending_meshes.0.remove(&asset_id);
                let loaded: Option<LoadedMeshAsset> = match result {
                    Ok(bytes) => {
                        let is_glb = bytes.len() >= 4 && bytes[0..4] == *b"glTF";
                        if is_glb {
                            match meshdata::load_glb(&bytes) {
                                Ok(loaded) => Some(loaded),
                                Err(e) => {
                                    warn!("Asset {asset_id}: glb load failed: {e}");
                                    None
                                }
                            }
                        } else {
                            match meshdata::load_obj(&bytes) {
                                Ok(mesh) => Some(LoadedMeshAsset {
                                    size: mesh_bounds(&mesh),
                                    rotation: Quat::IDENTITY,
                                    mesh,
                                    base_color: [0.8, 0.8, 0.8, 1.0],
                                    texture_bytes: None,
                                    double_sided: true,
                                    alpha_mode: 0,
                                }),
                                Err(e) => {
                                    warn!("Asset {asset_id}: obj load failed: {e}");
                                    None
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Asset {asset_id}: mesh fetch failed: {e}");
                        None
                    }
                };
                let Some(loaded) = loaded else {
                    continue;
                };
                let mut normalized = loaded.mesh.clone();
                normalize_mesh_data(&mut normalized);
                let normalized_bounds = mesh_bounds(&normalized);
                let mesh_handle = meshes.add(build_bevy_mesh(&loaded.mesh));
                let normalized_handle = meshes.add(build_bevy_mesh(&normalized));
                let material = build_mesh_material(&loaded, &mut image_assets, &mut materials);
                info!("Asset {asset_id}: loaded mesh");
                mesh_cache
                    .meshes
                    .insert((asset_id, false), mesh_handle.clone());
                mesh_cache
                    .meshes
                    .insert((asset_id, true), normalized_handle.clone());
                mesh_cache
                    .materials
                    .insert((asset_id, false), material.clone());
                mesh_cache
                    .materials
                    .insert((asset_id, true), material.clone());
                if let Some(collider) = build_mesh_collider(&loaded.mesh) {
                    mesh_cache.colliders.insert((asset_id, false), collider);
                } else {
                    warn!("Asset {asset_id}: no usable collision geometry in raw model");
                }
                if let Some(collider) = build_mesh_collider(&normalized) {
                    mesh_cache.colliders.insert((asset_id, true), collider);
                } else {
                    warn!("Asset {asset_id}: no usable collision geometry in normalized model");
                }
                mesh_cache.meta.insert(
                    asset_id,
                    MeshAssetMeta {
                        size_studs: (loaded.size / 0.28).max(Vec3::splat(0.001)),
                        normalized_size_studs: (normalized_bounds / 0.28).max(Vec3::splat(0.001)),
                        rotation: loaded.rotation,
                    },
                );
                let meta = mesh_cache.meta.get(&asset_id).cloned();
                for (entity, mesh_comp) in &mesh_entities {
                    if mesh_comp.asset_id == asset_id {
                        let handle = if mesh_comp.normalize {
                            normalized_handle.clone()
                        } else {
                            mesh_handle.clone()
                        };
                        commands
                            .entity(entity)
                            .insert((Mesh3d(handle), MeshMaterial3d(material.clone())));
                        if let Some(meta) = &meta {
                            apply_mesh_natural_transform(
                                &mut commands,
                                entity,
                                mesh_comp.normalize,
                                meta,
                                &mut transforms,
                            );
                        }
                    }
                }
            }
            fetch::AssetKind::MeshTexture => {
                pending_mesh_textures.0.remove(&asset_id);
                let decoded = match result {
                    Ok(bytes) => {
                        if bytes.is_empty() {
                            info!("Asset {asset_id}: no uploaded texture");
                            None
                        } else {
                            match decode_image_asset(&bytes, &mut image_assets) {
                                Some(handle) => Some(handle),
                                None => {
                                    warn!("Asset {asset_id}: failed to decode texture bytes");
                                    None
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Asset {asset_id}: texture fetch failed: {e}");
                        None
                    }
                };
                mesh_texture_cache.sidecars.insert(asset_id, decoded);
            }
        }
    }
}

fn build_mesh_material(
    loaded: &LoadedMeshAsset,
    image_assets: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    let mut material = StandardMaterial {
        base_color: Color::srgb(
            loaded.base_color[0],
            loaded.base_color[1],
            loaded.base_color[2],
        ),
        perceptual_roughness: 0.7,
        double_sided: loaded.double_sided,
        ..default()
    };
    if let Some(texture_bytes) = &loaded.texture_bytes {
        if let Some(texture) = decode_image_asset(texture_bytes, image_assets) {
            material.base_color_texture = Some(texture);
        }
    }
    if loaded.base_color[3] < 1.0 {
        material.alpha_mode = AlphaMode::Blend;
    } else {
        match loaded.alpha_mode {
            1 => material.alpha_mode = AlphaMode::Mask(0.5),
            2 => material.alpha_mode = AlphaMode::Blend,
            _ => {}
        }
    }
    materials.add(material)
}

fn fetch_missing_meshes(
    meshes: Query<&components::Mesh>,
    cache: Res<MeshAssetCache>,
    mut pending: ResMut<PendingMeshFetches>,
    mut pool: ResMut<fetch::AssetFetchPool>,
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
) {
    let user_ukey = current_user_ukey(auth_store, client_ukey);
    for mesh in &meshes {
        let asset_id = mesh.asset_id;
        if asset_id == 0 || pending.0.contains(&asset_id) {
            continue;
        }
        if cache.meshes.contains_key(&(asset_id, mesh.normalize)) {
            continue;
        }
        pool.ensure_started();
        if pool.submit_kind(asset_id, fetch::AssetKind::Mesh, user_ukey.clone()) {
            pending.0.insert(asset_id);
        }
    }
}

fn apply_cached_mesh_visuals(
    mut commands: Commands,
    cache: Res<MeshAssetCache>,
    meshes: Query<(Entity, &components::Mesh, Option<&Mesh3d>)>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, mesh_comp, existing) in &meshes {
        let key = (mesh_comp.asset_id, mesh_comp.normalize);
        if mesh_comp.asset_id == 0 {
            continue;
        }
        let (Some(mesh_handle), Some(material)) =
            (cache.meshes.get(&key), cache.materials.get(&key))
        else {
            continue;
        };
        let needs_attach = match existing {
            Some(current) => current.0 != *mesh_handle,
            None => true,
        };
        if needs_attach {
            commands.entity(entity).insert((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material.clone()),
            ));
            if let Some(meta) = cache.meta.get(&mesh_comp.asset_id) {
                apply_mesh_natural_transform(
                    &mut commands,
                    entity,
                    mesh_comp.normalize,
                    meta,
                    &mut transforms,
                );
            }
        }
    }
}

/// Submits background fetches for texture assets: uploaded texture sidecars
/// for mesh-sourced textures and regular image assets for decals.
fn fetch_missing_textures(
    textures: Query<(Entity, &components::Texture, Option<&ChildOf>)>,
    meshes: Query<&components::Mesh>,
    image_cache: Res<ImageAssetCache>,
    mesh_texture_cache: Res<MeshTextureCache>,
    mut pending_images: ResMut<PendingImageFetches>,
    mut pending_sidecars: ResMut<PendingMeshTextureFetches>,
    mut pool: ResMut<fetch::AssetFetchPool>,
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
) {
    let user_ukey = current_user_ukey(auth_store, client_ukey);
    for (_entity, texture, child_of) in &textures {
        if texture.is_decal {
            let asset_id = texture.asset_id;
            if asset_id == 0
                || image_cache.images.contains_key(&asset_id)
                || pending_images.0.contains(&asset_id)
            {
                continue;
            }
            pool.ensure_started();
            if pool.submit(asset_id, user_ukey.clone()) {
                pending_images.0.insert(asset_id);
            }
        } else {
            // Sidecars are only relevant for textures actually attached to a
            // mesh; the sidecar id is the parent mesh's asset id.
            let Some(child_of) = child_of else {
                continue;
            };
            let Ok(parent_mesh) = meshes.get(child_of.parent()) else {
                continue;
            };
            let asset_id = parent_mesh.asset_id;
            if asset_id == 0
                || mesh_texture_cache.sidecars.contains_key(&asset_id)
                || pending_sidecars.0.contains(&asset_id)
            {
                continue;
            }
            pool.ensure_started();
            if pool.submit_kind(asset_id, fetch::AssetKind::MeshTexture, user_ukey.clone()) {
                pending_sidecars.0.insert(asset_id);
            }
        }
    }
}

/// Applies texture overrides to parent meshes. The first (by entity order)
/// Texture child of a mesh decides its appearance:
/// - decal textures ("image/N") replace the base color texture with image N;
/// - mesh textures ("mesh/N") use the mesh's uploaded texture sidecar when it
///   exists, falling back to the model's embedded texture.
///
/// When no override resolves anymore, the mesh's default material is
/// restored.
fn apply_texture_overrides(
    mut commands: Commands,
    textures: Query<(Entity, &components::Texture, Option<&ChildOf>)>,
    meshes: Query<(
        Entity,
        &components::Mesh,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
    image_cache: Res<ImageAssetCache>,
    mesh_cache: Res<MeshAssetCache>,
    mut mesh_texture_cache: ResMut<MeshTextureCache>,
    mut applied: ResMut<AppliedTextureOverrides>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut children_by_parent: HashMap<Entity, Vec<(Entity, components::Texture)>> =
        HashMap::new();
    for (entity, texture, child_of) in &textures {
        if let Some(child_of) = child_of {
            children_by_parent
                .entry(child_of.parent())
                .or_default()
                .push((entity, *texture));
        }
    }

    // Meshes that should render with an override material this frame.
    let mut overridden: HashMap<Entity, Handle<StandardMaterial>> = HashMap::new();

    for (parent, mut children) in children_by_parent {
        children.sort_by_key(|(e, _)| *e);
        let (_, texture) = children[0];
        let Ok((_, mesh_comp, _)) = meshes.get(parent) else {
            continue;
        };
        if mesh_comp.asset_id == 0 {
            continue;
        }
        let Some(default_material) = mesh_cache
            .materials
            .get(&(mesh_comp.asset_id, mesh_comp.normalize))
            .cloned()
        else {
            continue;
        };

        // None means "still loading" (leave the mesh untouched), Some(None)
        // means "resolved to no custom texture".
        let effective: Option<Option<Handle<Image>>> = if texture.is_decal {
            if texture.asset_id == 0 {
                Some(None)
            } else {
                image_cache.images.get(&texture.asset_id).cloned().map(Some)
            }
        } else {
            mesh_texture_cache.sidecars.get(&mesh_comp.asset_id).cloned()
        };
        let Some(effective) = effective else {
            continue;
        };

        let target = match effective {
            Some(tex_handle) => {
                let key = (mesh_comp.asset_id, mesh_comp.normalize, texture.asset_id);
                if let Some(existing) = mesh_texture_cache.materials.get(&key) {
                    Some(existing.clone())
                } else if let Some(base) = materials.get(&default_material).cloned() {
                    let mut mat = base;
                    mat.base_color_texture = Some(tex_handle);
                    let handle = materials.add(mat);
                    mesh_texture_cache.materials.insert(key, handle.clone());
                    Some(handle)
                } else {
                    None
                }
            }
            None => None,
        };

        match target {
            Some(handle) => {
                overridden.insert(parent, handle);
            }
            None => {
                if applied.0.remove(&parent).is_some() {
                    commands
                        .entity(parent)
                        .insert(MeshMaterial3d(default_material));
                }
            }
        }
    }

    for (parent, handle) in overridden.iter() {
        let needs_apply = meshes
            .get(*parent)
            .ok()
            .and_then(|(_, _, mat)| mat)
            .is_none_or(|m| m.0 != *handle);
        if needs_apply {
            commands
                .entity(*parent)
                .insert(MeshMaterial3d(handle.clone()));
        }
        applied.0.insert(*parent, handle.clone());
    }

    // Restore meshes whose texture children went away or stopped resolving.
    let stale: Vec<Entity> = applied
        .0
        .keys()
        .copied()
        .filter(|e| !overridden.contains_key(e))
        .collect();
    for parent in stale {
        let previous = applied.0.remove(&parent);
        let default_material = meshes
            .get(parent)
            .ok()
            .and_then(|(_, mesh_comp, _)| {
                mesh_cache
                    .materials
                    .get(&(mesh_comp.asset_id, mesh_comp.normalize))
            })
            .cloned();
        let Some(default_material) = default_material else {
            continue;
        };
        let needs_restore = previous.is_some_and(|h| {
            meshes
                .get(parent)
                .ok()
                .and_then(|(_, _, m)| m)
                .is_some_and(|m| m.0 == h)
        });
        if needs_restore {
            commands.entity(parent).insert(MeshMaterial3d(default_material));
        }
    }
}

/// Builds a precise trimesh collider from the loaded model data, dropping
/// degenerate triangles and merging duplicate vertices so any valid model can
/// be turned into collision geometry without panicking.
fn build_mesh_collider(data: &LoadedMeshData) -> Option<Collider> {
    let vertices: Vec<Vec3> = data.positions.iter().map(|p| Vec3::from(*p)).collect();
    if vertices.is_empty() {
        return None;
    }
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    for tri in data.indices.chunks_exact(3) {
        let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if a < vertices.len() && b < vertices.len() && c < vertices.len() {
            let area = (vertices[b] - vertices[a]).cross(vertices[c] - vertices[a]);
            if area.length_squared() > 1e-12 {
                triangles.push([tri[0], tri[1], tri[2]]);
            }
        }
    }
    if triangles.is_empty() {
        return None;
    }
    Collider::try_trimesh_with_config(vertices, triangles, TrimeshFlags::MERGE_DUPLICATE_VERTICES)
        .ok()
}

/// Returns true when the entity has at least one Texture child.
pub fn has_texture_child(world: &World, mesh_entity: Entity) -> bool {
    world.get::<Children>(mesh_entity).is_some_and(|children| {
        children
            .iter()
            .any(|c| world.get::<components::Texture>(c).is_some())
    })
}

/// Ensures the mesh entity has a Texture child, creating a default one named
/// "Texture" with id "mesh/{asset_id}" when missing. Returns true when a
/// child was created.
pub fn ensure_texture_child(world: &World, commands: &mut Commands, mesh_entity: Entity) -> bool {
    if has_texture_child(world, mesh_entity) {
        return false;
    }
    let asset_id = world
        .get::<components::Mesh>(mesh_entity)
        .map(|m| m.asset_id)
        .unwrap_or(0);
    let texture_entity = commands
        .spawn((
            Name::new("Texture"),
            Transform::default(),
            components::Texture {
                asset_id,
                is_decal: false,
            },
            lightyear::prelude::Replicate::default(),
        ))
        .id();
    commands.entity(mesh_entity).add_child(texture_entity);
    true
}

/// Same as [`ensure_texture_child`] but spawning directly through the world.
pub fn ensure_texture_child_world(world: &mut World, mesh_entity: Entity) -> bool {
    if has_texture_child(world, mesh_entity) {
        return false;
    }
    let asset_id = world
        .get::<components::Mesh>(mesh_entity)
        .map(|m| m.asset_id)
        .unwrap_or(0);
    let texture_entity = world
        .spawn((
            Name::new("Texture"),
            Transform::default(),
            components::Texture {
                asset_id,
                is_decal: false,
            },
            lightyear::prelude::Replicate::default(),
        ))
        .id();
    world.entity_mut(mesh_entity).add_child(texture_entity);
    true
}

/// Moves texture children whose id still equals the old default
/// ("mesh/{old_asset_id}") to the new default so untouched textures follow
/// their parent mesh's id changes. Customized (decal) ids are left alone.
pub fn follow_mesh_id_change(
    world: &mut World,
    mesh_entity: Entity,
    old_asset_id: u32,
    new_asset_id: u32,
) {
    if old_asset_id == new_asset_id {
        return;
    }
    let Some(children) = world.get::<Children>(mesh_entity).map(|c| c.to_vec()) else {
        return;
    };
    for child in children {
        let follows = world
            .get::<components::Texture>(child)
            .is_some_and(|texture| !texture.is_decal && texture.asset_id == old_asset_id);
        if follows
            && let Some(mut texture) = world.get_mut::<components::Texture>(child)
        {
            texture.asset_id = new_asset_id;
        }
    }
}

/// Gives a freshly placed mesh entity its natural model size (in studs) and the
/// rotation the model itself carries, but only while the transform is still at
/// its untouched defaults so user edits are never overwritten.
fn apply_mesh_natural_transform(
    commands: &mut Commands,
    entity: Entity,
    normalize: bool,
    meta: &MeshAssetMeta,
    transforms: &mut Query<&mut Transform>,
) {
    let Ok(transform) = transforms.get_mut(entity) else {
        return;
    };
    let mut new_transform = *transform;
    let mut changed = false;
    if new_transform.scale == Vec3::ONE {
        // Model data is authored in meters. An unmodified model renders at its
        // true meter size; a normalized model (max extent 1 m) renders at
        // exactly 1 stud (0.28 m).
        let scale = if normalize {
            Vec3::splat(0.28)
        } else {
            Vec3::ONE
        };
        new_transform.scale = scale.max(Vec3::splat(0.001));
        changed = true;
    }
    if new_transform.rotation == Quat::IDENTITY && meta.rotation != Quat::IDENTITY {
        new_transform.rotation = meta.rotation;
        changed = true;
    }
    if changed {
        commands.entity(entity).insert(new_transform);
    }
}

fn apply_cached_image_materials(
    mut commands: Commands,
    cache: Res<ImageAssetCache>,
    images: Query<
        (Entity, &components::Image),
        (With<Mesh3d>, Without<MeshMaterial3d<StandardMaterial>>),
    >,
) {
    for (entity, image) in &images {
        if let Some(material) = cache.materials.get(&image.asset_id) {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(material.clone()));
        }
    }
}

fn decode_image_asset(bytes: &[u8], images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let mut image = Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::all(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .ok();
    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            ImageType::Extension("jpg"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .ok();
    }
    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            ImageType::Extension("webp"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .ok();
    }
    if image.is_none() {
        image = Image::from_buffer(
            bytes,
            ImageType::Extension("gif"),
            CompressedImageFormats::all(),
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .ok();
    }
    let mut final_image = image?;

    let format = final_image.texture_descriptor.format;
    if (format == TextureFormat::Rgba8UnormSrgb || format == TextureFormat::Rgba8Unorm)
        && let Some(ref mut data) = final_image.data
    {
        for chunk in data.chunks_exact_mut(4) {
            let a = chunk[3] as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 * a) as u8;
            chunk[1] = (chunk[1] as f32 * a) as u8;
            chunk[2] = (chunk[2] as f32 * a) as u8;
        }
    }

    final_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });

    Some(images.add(final_image))
}

fn face_normal(face: ImageFace) -> Vec3 {
    match face {
        ImageFace::Top => Vec3::Y,
        ImageFace::Bottom => Vec3::NEG_Y,
        ImageFace::Left => Vec3::NEG_X,
        ImageFace::Right => Vec3::X,
        ImageFace::Front => Vec3::Z,
        ImageFace::Back => Vec3::NEG_Z,
    }
}

fn update_image_visuals(
    mut images: Query<(
        &components::Image,
        Option<&ChildOf>,
        &mut Transform,
        Option<&mut Visibility>,
    )>,
    parents: Query<&BrickShapeComponent, (With<Brick>, Without<components::Image>)>,
) {
    for (image, child_of, mut transform, mut visibility) in &mut images {
        let Some(child_of) = child_of else {
            if let Some(visibility) = visibility.as_deref_mut() {
                *visibility = Visibility::Visible;
            }
            continue;
        };
        let Ok(parent_shape) = parents.get(child_of.parent()) else {
            if let Some(visibility) = visibility.as_deref_mut() {
                *visibility = Visibility::Visible;
            }
            continue;
        };
        if parent_shape.shape != BrickShape::Block {
            if let Some(visibility) = visibility.as_deref_mut() {
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        if let Some(visibility) = visibility.as_deref_mut() {
            *visibility = Visibility::Visible;
        }

        let normal = face_normal(image.face.unwrap_or(ImageFace::Front));
        let (half_extent, width, height) = if normal.x != 0.0 {
            (
                BLOCK_MESH_HALF_EXTENTS.x,
                BLOCK_MESH_HALF_EXTENTS.y * 2.0,
                BLOCK_MESH_HALF_EXTENTS.z * 2.0,
            )
        } else if normal.y != 0.0 {
            (
                BLOCK_MESH_HALF_EXTENTS.y,
                BLOCK_MESH_HALF_EXTENTS.x * 2.0,
                BLOCK_MESH_HALF_EXTENTS.z * 2.0,
            )
        } else {
            (
                BLOCK_MESH_HALF_EXTENTS.z,
                BLOCK_MESH_HALF_EXTENTS.x * 2.0,
                BLOCK_MESH_HALF_EXTENTS.y * 2.0,
            )
        };
        transform.translation = normal * half_extent + normal * FACE_OFFSET;
        transform.rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        transform.scale = Vec3::new(
            (width - FACE_INSET * 2.0).max(0.01),
            (height - FACE_INSET * 2.0).max(0.01),
            OVERLAY_THICKNESS,
        );
    }
}
