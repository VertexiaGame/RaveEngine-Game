use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

#[derive(Component)]
pub struct SpatialSoundBillboard;

pub const SOUND_ICON_SCALE: f32 = 2.5;

#[derive(Resource)]
pub struct StudioShowAudio {
    pub enabled: bool,
}

impl Default for StudioShowAudio {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Resource, Default)]
pub struct SpatialSoundIconCache {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
    icon: Option<Handle<Image>>,
}

pub fn spatial_range_radius(
    sound: &crate::common::game::assets::components::Sound,
    scale: Vec3,
) -> f32 {
    let volume = crate::common::game::assets::components::Sound::clamp_volume(sound.volume);
    let s = scale.max_element().max(0.01);
    volume * 0.28 * s
}

fn make_quad() -> Mesh {
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

fn ensure_cache(
    cache: &mut SpatialSoundIconCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    ui_assets: Option<&crate::studio::ui::assets::StudioUiAssets>,
) -> bool {
    if cache.mesh.is_none() {
        cache.mesh = Some(meshes.add(make_quad()));
    }
    if let Some(ui) = ui_assets {
        let changed = cache.icon.as_ref() != Some(&ui.sound_icon);
        if changed || cache.material.is_none() {
            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(ui.sound_icon.clone()),
                unlit: true,
                double_sided: true,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                ..default()
            });
            cache.material = Some(material);
            cache.icon = Some(ui.sound_icon.clone());
        }
    }
    cache.mesh.is_some() && cache.material.is_some()
}

pub fn ensure_spatial_sound_icons(
    mut commands: Commands,
    mut cache: ResMut<SpatialSoundIconCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ui_assets: Option<Res<crate::studio::ui::assets::StudioUiAssets>>,
    sounds: Query<(Entity, &crate::common::game::assets::components::Sound, Option<&Children>)>,
    billboard_query: Query<(), With<SpatialSoundBillboard>>,
    orphan_query: Query<(Entity, &ChildOf), With<SpatialSoundBillboard>>,
    sound_lookup: Query<(), With<crate::common::game::assets::components::Sound>>,
) {
    if !ensure_cache(&mut cache, &mut meshes, &mut materials, ui_assets.as_deref()) {
        return;
    }
    let mesh_handle = cache.mesh.clone().unwrap();
    let material_handle = cache.material.clone().unwrap();
    for (entity, sound, children_opt) in &sounds {
        if sound.spatial {
            let mut has_icon = false;
            if let Some(children) = children_opt {
                for child in children.iter() {
                    if billboard_query.get(child).is_ok() {
                        has_icon = true;
                        break;
                    }
                }
            }
            if !has_icon {
                let child = commands
                    .spawn((
                        Transform {
                            translation: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                            scale: Vec3::splat(SOUND_ICON_SCALE),
                        },
                        Mesh3d(mesh_handle.clone()),
                        MeshMaterial3d(material_handle.clone()),
                        Pickable::default(),
                        Visibility::Visible,
                        SpatialSoundBillboard,
                    ))
                    .id();
                commands.entity(entity).add_child(child);
            }
        } else if let Some(children) = children_opt {
            for child in children.iter() {
                if billboard_query.get(child).is_ok() {
                    commands.entity(child).despawn();
                }
            }
        }
    }
    for (entity, child_of) in &orphan_query {
        if sound_lookup.get(child_of.parent()).is_err() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_spatial_sound_billboards(
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<crate::studio::camera::GizmoCamera>)>,
    parent_query: Query<&GlobalTransform>,
    mut billboards: Query<(&mut Transform, &ChildOf), With<SpatialSoundBillboard>>,
) {
    let mut camera_rotation = None;
    for camera_global in &camera_query {
        camera_rotation = Some(camera_global.rotation());
        break;
    }
    let Some(camera_rotation) = camera_rotation else {
        return;
    };
    for (mut transform, child_of) in &mut billboards {
        let Ok(parent_global) = parent_query.get(child_of.parent()) else {
            continue;
        };
        let parent_rotation = parent_global.rotation();
        transform.translation = Vec3::ZERO;
        transform.scale = Vec3::splat(SOUND_ICON_SCALE);
        transform.rotation = parent_rotation.inverse() * camera_rotation;
    }
}

pub fn update_spatial_sound_visibility(
    physics_state: Res<crate::common::game::physics::PhysicsSimulationState>,
    playtest: Option<Res<crate::client::PlaytestState>>,
    show_audio: Res<StudioShowAudio>,
    mut billboards: Query<&mut Visibility, With<SpatialSoundBillboard>>,
) {
    let hidden = !show_audio.enabled
        || *physics_state == crate::common::game::physics::PhysicsSimulationState::Running
        || playtest.map_or(false, |p| p.active);
    let target = if hidden {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut visibility in &mut billboards {
        if *visibility != target {
            *visibility = target;
        }
    }
}

pub fn draw_spatial_sound_ranges(
    sounds: Query<(Entity, &GlobalTransform, &crate::common::game::assets::components::Sound)>,
    selection: Res<crate::studio::tools::Selection>,
    physics_state: Res<crate::common::game::physics::PhysicsSimulationState>,
    playtest: Option<Res<crate::client::PlaytestState>>,
    show_audio: Res<StudioShowAudio>,
    mut gizmos: Gizmos,
) {
    if !show_audio.enabled {
        return;
    }
    if *physics_state == crate::common::game::physics::PhysicsSimulationState::Running {
        return;
    }
    if playtest.map_or(false, |p| p.active) {
        return;
    }
    for (entity, global, sound) in &sounds {
        if !sound.spatial {
            continue;
        }
        let radius = spatial_range_radius(sound, global.scale());
        if !(radius > 0.01) {
            continue;
        }
        let translation = global.translation();
        let color = if selection.entities.contains(&entity) {
            Color::srgb(1.0, 1.0, 1.0)
        } else {
            Color::srgb(0.2, 0.55, 1.0)
        };
        gizmos.sphere(
            Isometry3d::new(translation, Quat::IDENTITY),
            radius,
            color,
        );
    }
}
