use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;
use bevy::picking::backend::{ray::RayMap, HitData, PointerHits};
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::picking::mesh_picking::{MeshPickingCamera, MeshPickingSettings};
use bevy::picking::{Pickable, PickingSettings, PickingSystems};

pub struct GatedMeshPickingPlugin;

impl Plugin for GatedMeshPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeshPickingSettings>()
            .add_systems(PreUpdate, update_hits.in_set(PickingSystems::Backend));
    }
}

pub fn update_hits(
    backend_settings: Res<MeshPickingSettings>,
    picking_settings: Res<PickingSettings>,
    ray_map: Res<RayMap>,
    picking_cameras: Query<(&Camera, Has<MeshPickingCamera>, Option<&RenderLayers>)>,
    pickables: Query<&Pickable>,
    marked_targets: Query<&Pickable>,
    layers: Query<&RenderLayers>,
    mut ray_cast: MeshRayCast,
    mut pointer_hits_writer: MessageWriter<PointerHits>,
) {
    if !picking_settings.is_enabled {
        return;
    }
    for (&ray_id, &ray) in ray_map.iter() {
        let Ok((camera, cam_can_pick, cam_layers)) = picking_cameras.get(ray_id.camera) else {
            continue;
        };
        if backend_settings.require_markers && !cam_can_pick {
            continue;
        }

        let cam_layers = cam_layers.to_owned().unwrap_or_default();

        let settings = MeshRayCastSettings {
            visibility: backend_settings.ray_cast_visibility,
            filter: &|entity| {
                let marker_requirement =
                    !backend_settings.require_markers || marked_targets.get(entity).is_ok();

                let entity_layers = layers.get(entity).cloned().unwrap_or_default();
                let render_layers_match = cam_layers.intersects(&entity_layers);

                let is_pickable = pickables.get(entity).ok().is_none_or(|p| p.is_hoverable);

                marker_requirement && render_layers_match && is_pickable
            },
            early_exit_test: &|entity_hit| {
                pickables
                    .get(entity_hit)
                    .is_ok_and(|pickable| pickable.should_block_lower)
            },
        };
        let picks = ray_cast
            .cast_ray(ray, &settings)
            .iter()
            .map(|(entity, hit)| {
                let hit_data = HitData::new(
                    ray_id.camera,
                    hit.distance,
                    Some(hit.point),
                    Some(hit.normal),
                );
                (*entity, hit_data)
            })
            .collect::<Vec<_>>();
        let order = camera.order as f32;
        if !picks.is_empty() {
            pointer_hits_writer.write(PointerHits::new(ray_id.pointer, picks, order));
        }
    }
}
