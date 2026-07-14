use bevy::prelude::*;
use bevy::pbr::ExtendedMaterial;
use crate::common::game::bricks::components::{Brick, BrickShape, BrickShapeComponent};
use crate::common::game::bricks::studs::{StudsAssets, StudsExtension};
use avian3d::prelude::CollisionLayers;

#[derive(Resource, Default)]
pub struct BrickSpawnerCount {
    pub count: u32,
}

pub fn spawn_brick(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>,
    studs_assets: &StudsAssets,
    count: &mut BrickSpawnerCount,
    spawn_pos: Vec3,
    shape: BrickShape,
) -> Entity {
    let current_index = count.count;
    count.count += 1;

    let mesh_handle = match shape {
        BrickShape::Block => {
            meshes.add(Cuboid::new(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28))
        }
        BrickShape::Sphere => {
            meshes.add(Sphere::new(1.0 * 0.28))
        }
    };

    let name_prefix = match shape {
        BrickShape::Block => "Part",
        BrickShape::Sphere => "Sphere",
    };

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.84, 0.24, 0.16),
                perceptual_roughness: 0.95,
                reflectance: 0.1,
                ..default()
            },
            extension: StudsExtension {
                stud_texture: studs_assets.stud.clone(),
                inlet_texture: studs_assets.inlet.clone(),
            },
        })),
        Transform::from_translation(spawn_pos),
        Brick,
        BrickShapeComponent { shape },
        crate::common::game::bricks::components::BrickPhysics::default(),
        crate::common::game::bricks::components::BrickColor { color: Color::srgb(0.84, 0.24, 0.16) },
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
        Pickable::default(),
        Name::new(format!("{}{}", name_prefix, current_index)),
    )).id()
}

#[derive(Clone, Debug)]
pub struct BrickData {
    pub transform: Transform,
    pub name: String,
    pub is_brick: bool,
    pub shape: BrickShape,
    pub mesh: Option<Mesh3d>,
    pub standard_material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<MeshMaterial3d<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
    pub parent: Option<Entity>,
    pub physics: Option<crate::common::game::bricks::components::BrickPhysics>,
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

pub fn spawn_from_data(
    commands: &mut Commands,
    data: &BrickData,
) -> Entity {
    let mut spawned = commands.spawn((
        data.transform,
        Name::new(data.name.clone()),
        Pickable::default(),
    ));
    if data.is_brick {
        spawned.insert((
            Brick,
            BrickShapeComponent { shape: data.shape },
            crate::common::game::bricks::components::BrickColor::default(),
        ));
    }
    if let Some(ref m) = data.mesh {
        spawned.insert(m.clone());
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
    query: &Query<(
        Entity,
        &mut Transform,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&Brick>,
        Option<&mut BrickShapeComponent>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
        Option<&mut crate::common::game::bricks::components::BrickPhysics>,
    ), Without<Camera3d>>,
) -> Option<BrickData> {
    if let Ok((_, transform, name, child_of_opt, _, brick_opt, shape_opt, _, mesh_opt, mat_opt, studs_mat_opt, phys_opt)) = query.get(entity) {
        let is_brick = brick_opt.is_some();
        let shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(BrickShape::Block);
        Some(BrickData {
            transform: *transform,
            name: name.to_string(),
            is_brick,
            shape,
            mesh: mesh_opt.cloned(),
            standard_material: mat_opt.cloned(),
            studs_material: studs_mat_opt.cloned(),
            parent: child_of_opt.map(|co| co.parent()),
            physics: phys_opt.cloned(),
        })
    } else {
        None
    }
}