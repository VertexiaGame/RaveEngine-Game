use crate::common::game::bricks::components::{Brick, BrickShape, BrickShapeComponent};
use crate::common::game::bricks::studs::{ShadowOpacityExtension, StudsAssets, StudsExtension};
use avian3d::prelude::CollisionLayers;
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct BrickSpawnerCount {
    pub count: u32,
}

pub fn spawn_brick(
    commands: &mut Commands,
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>,
    _studs_assets: &StudsAssets,
    count: &mut BrickSpawnerCount,
    spawn_pos: Vec3,
    shape: BrickShape,
) -> Entity {
    let current_index = count.count;
    count.count += 1;

    let name_prefix = shape.default_name_prefix();

    commands
        .spawn((
            Transform::from_translation(spawn_pos),
            Brick,
            BrickShapeComponent { shape },
            crate::common::game::bricks::components::BrickPhysics::default(),
            crate::common::game::bricks::components::BrickColor {
                color: Color::srgb(0.84, 0.24, 0.16),
            },
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
            Pickable::default(),
            Name::new(format!("{}{}", name_prefix, current_index)),
        ))
        .id()
}

#[derive(Clone, Debug)]
pub struct BrickData {
    pub transform: Transform,
    pub name: String,
    pub is_brick: bool,
    pub shape: BrickShape,
    pub mesh: Option<Mesh3d>,
    pub mesh_asset: Option<crate::common::game::assets::components::Mesh>,
    pub sound_asset: Option<crate::common::game::assets::components::Sound>,
    pub standard_material:
        Option<MeshMaterial3d<ExtendedMaterial<StandardMaterial, ShadowOpacityExtension>>>,
    pub studs_material: Option<MeshMaterial3d<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    pub parent: Option<Entity>,
    pub physics: Option<crate::common::game::bricks::components::BrickPhysics>,
    pub studs: bool,
    pub color: Option<Color>,
}

impl BrickData {
    pub fn remap(&mut self, old: Entity, new: Entity) {
        if let Some(p) = &mut self.parent {
            if *p == old {
                *p = new;
            }
        }
    }
}

pub fn spawn_from_data(commands: &mut Commands, data: &BrickData) -> Entity {
    let mut spawned = commands.spawn((
        data.transform,
        Name::new(data.name.clone()),
        Pickable::default(),
    ));
    if data.is_brick {
        spawned.insert((
            Brick,
            BrickShapeComponent { shape: data.shape },
            crate::common::game::bricks::components::BrickColor {
                color: data.color.unwrap_or(Color::srgb(0.84, 0.24, 0.16)),
            },
            crate::common::game::bricks::components::BrickStuds {
                enabled: data.studs,
            },
        ));
    }
    if let Some(ref m) = data.mesh {
        spawned.insert(m.clone());
    }
    if let Some(ref mesh_asset) = data.mesh_asset {
        spawned.insert(*mesh_asset);
    }
    if let Some(ref sound_asset) = data.sound_asset {
        spawned.insert(*sound_asset);
    }
    if let Some(ref mat) = data.standard_material {
        spawned.insert(mat.clone());
    }
    if let Some(ref studs_mat) = data.studs_material {
        spawned.insert(studs_mat.clone());
    }
    if let Some(phys) = data.physics {
        spawned.insert(phys);
        let layers = if phys.player_can_collide {
            CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
        } else {
            CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
        };
        spawned.insert(layers);
    } else if data.is_brick {
        spawned.insert(crate::common::game::bricks::components::BrickPhysics::default());
        spawned.insert(CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF));
    }
    let new_entity = spawned.id();
    if let Some(parent) = data.parent {
        if let Ok(mut p_cmd) = commands.get_entity(parent) {
            p_cmd.add_child(new_entity);
        }
    }
    new_entity
}

pub fn capture_brick_data(
    entity: Entity,
    query: &Query<
        (
            Entity,
            &mut Transform,
            &Name,
            Option<&ChildOf>,
            Option<&Children>,
            Option<&Brick>,
            Option<&mut BrickShapeComponent>,
            &GlobalTransform,
            Option<&Mesh3d>,
            Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, ShadowOpacityExtension>>>,
            Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
            Option<&mut crate::common::game::bricks::components::BrickPhysics>,
        ),
        Without<Camera3d>,
    >,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
    brick_colors: &Query<&mut crate::common::game::bricks::components::BrickColor>,
    mesh_assets: &Query<&crate::common::game::assets::components::Mesh>,
    sound_assets: &Query<&crate::common::game::assets::components::Sound>,
) -> Option<BrickData> {
    if let Ok((
        _,
        transform,
        name,
        child_of_opt,
        _,
        brick_opt,
        shape_opt,
        _,
        mesh_opt,
        mat_opt,
        studs_mat_opt,
        phys_opt,
    )) = query.get(entity)
    {
        let is_brick = brick_opt.is_some();
        let shape = shape_opt
            .as_ref()
            .map(|s| s.shape)
            .unwrap_or(BrickShape::Block);
        Some(BrickData {
            transform: *transform,
            name: name.to_string(),
            is_brick,
            shape,
            mesh: mesh_opt.cloned(),
            mesh_asset: mesh_assets.get(entity).ok().copied(),
            sound_asset: sound_assets.get(entity).ok().copied(),
            standard_material: mat_opt.cloned(),
            studs_material: studs_mat_opt.cloned(),
            parent: child_of_opt.map(|co| co.parent()),
            physics: phys_opt.cloned(),
            studs: studs_query.get(entity).map(|s| s.enabled).unwrap_or(true),
            color: brick_colors.get(entity).ok().map(|bc| bc.color),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use std::sync::{Arc, Mutex};

    #[test]
    fn capture_and_spawn_preserve_mesh_assets() {
        let mut app = App::new();
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_xyz(1.0, 2.0, 3.0),
                Name::new("TestMesh"),
                crate::common::game::assets::components::Mesh {
                    asset_id: 7,
                    normalize: true,
                },
                crate::common::game::bricks::components::BrickPhysics {
                    enabled: true,
                    bounciness: 0.5,
                    player_can_collide: true,
                    friction: 0.2,
                    gravity_scale: 1.0,
                    mass: 2.0,
                },
            ))
            .id();

        let captured: Arc<Mutex<Option<BrickData>>> = Arc::new(Mutex::new(None));
        let sink = captured.clone();
        app.world_mut().run_system_once(
            move |query: Query<
                '_,
                '_,
                (
                    Entity,
                    &mut Transform,
                    &Name,
                    Option<&ChildOf>,
                    Option<&Children>,
                    Option<&Brick>,
                    Option<&mut BrickShapeComponent>,
                    &GlobalTransform,
                    Option<&Mesh3d>,
                    Option<&MeshMaterial3d<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::ShadowOpacityExtension>>>,
                    Option<&MeshMaterial3d<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
                    Option<&mut crate::common::game::bricks::components::BrickPhysics>,
                ),
                Without<Camera3d>,
            >,
                  studs_query: Query<&crate::common::game::bricks::components::BrickStuds>,
                  brick_colors: Query<&mut crate::common::game::bricks::components::BrickColor>,
                  mesh_assets: Query<&crate::common::game::assets::components::Mesh>,
                  sound_assets: Query<&crate::common::game::assets::components::Sound>| {
                *sink.lock().unwrap() = capture_brick_data(entity, &query, &studs_query, &brick_colors, &mesh_assets, &sound_assets);
            },
        ).unwrap();

        let data = captured
            .lock()
            .unwrap()
            .take()
            .expect("mesh entity should be capturable");
        assert_eq!(
            data.mesh_asset,
            Some(crate::common::game::assets::components::Mesh {
                asset_id: 7,
                normalize: true
            })
        );

        let restored = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| spawn_from_data(&mut commands, &data))
            .unwrap();
        app.world_mut().flush();

        let mesh_asset = app
            .world_mut()
            .get::<crate::common::game::assets::components::Mesh>(restored)
            .copied();
        assert_eq!(
            mesh_asset,
            Some(crate::common::game::assets::components::Mesh {
                asset_id: 7,
                normalize: true
            })
        );
        let transform = app.world_mut().get::<Transform>(restored).unwrap();
        assert_eq!(*transform, Transform::from_xyz(1.0, 2.0, 3.0));
    }
}
