use bevy::prelude::*;
use RaveEngineLib::common::game::bricks::block_brick_mesh;
use RaveEngineLib::common::game::bricks::components::{
    Brick, BrickPhysics, BrickShape, BrickShapeComponent,
};
use RaveEngineLib::common::game::physics::{
    PhysicsSimulationAction, PhysicsSimulationPlugin, PhysicsSimulationState,
};

fn spawn_brick(app: &mut App, name: &str, tf: Transform, physics_enabled: bool) {
    let mesh = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.add(block_brick_mesh(tf.scale))
    };
    let _ = app.world_mut()
        .spawn((
            Name::new(name.to_string()),
            Brick,
            BrickShapeComponent {
                shape: BrickShape::Block,
            },
            BrickPhysics {
                enabled: physics_enabled,
                ..Default::default()
            },
            tf,
            Mesh3d(mesh),
        ))
        .id();
}

fn new_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(RaveEngineLib::common::assets_path::asset_plugin());
    app.init_asset::<Mesh>();
    app.init_resource::<RaveEngineLib::common::game::assets::MeshAssetCache>();
    app.init_resource::<avian3d::spatial_query::SpatialQueryDiagnostics>();
    app.init_resource::<avian3d::collider_tree::ColliderTreeDiagnostics>();
    app.init_resource::<avian3d::collision::CollisionDiagnostics>();
    app.init_resource::<avian3d::dynamics::solver::SolverDiagnostics>();
    app.add_plugins(PhysicsSimulationPlugin);
    app
}

#[test]
fn studio_edit_mode_play_runs_without_hanging() {
    let mut app = new_app();

    spawn_brick(
        &mut app,
        "Baseplate",
        Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(25.0, 1.0, 50.0)),
        false,
    );
    for i in 0..8 {
        spawn_brick(
            &mut app,
            &format!("Part{i}"),
            Transform::from_xyz(i as f32 * 0.6 - 2.0, 1.0 + i as f32 * 0.4, 0.0),
            true,
        );
    }

    app.update();

    app.world_mut()
        .write_message(PhysicsSimulationAction::Play)
        .unwrap();

    for frame in 0..600 {
        app.update();
        if frame == 5 {
            assert_eq!(
                *app.world().resource::<PhysicsSimulationState>(),
                PhysicsSimulationState::Running
            );
        }
    }

    let mut positions = app
        .world_mut()
        .query::<(&Name, &Transform)>()
        .iter(app.world())
        .map(|(n, t)| (n.to_string(), t.translation))
        .collect::<Vec<_>>();
    positions.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, pos) in &positions {
        assert!(pos.is_finite(), "{name} has non-finite position");
    }

    app.world_mut()
        .write_message(PhysicsSimulationAction::Stop)
        .unwrap();
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<PhysicsSimulationState>(),
        PhysicsSimulationState::Stopped
    );

    app.world_mut()
        .write_message(PhysicsSimulationAction::Play)
        .unwrap();
    for _ in 0..300 {
        app.update();
    }
}

#[test]
fn large_brick_scene_play_stays_stable() {
    let mut app = new_app();

    spawn_brick(
        &mut app,
        "Baseplate",
        Transform::from_xyz(0.0, -0.14, 0.0).with_scale(Vec3::new(64.0, 1.0, 64.0)),
        false,
    );
    // A dense multi-layer stack of bricks resembling an imported model.
    let mut index = 0u32;
    for y in 0..6u32 {
        for x in 0..24u32 {
            for z in 0..8u32 {
                spawn_brick(
                    &mut app,
                    &format!("ImportedBrick{index}"),
                    Transform::from_xyz(
                        (x as f32 - 11.5) * 0.85,
                        0.15 + y as f32 * 0.3,
                        (z as f32 - 3.5) * 0.6,
                    ),
                    true,
                );
                index += 1;
            }
        }
    }
    assert_eq!(index, 1152);

    app.update();

    app.world_mut()
        .write_message(PhysicsSimulationAction::Play)
        .unwrap();

    for _ in 0..240 {
        app.update();
    }

    assert_eq!(
        *app.world().resource::<PhysicsSimulationState>(),
        PhysicsSimulationState::Running
    );

    let trimesh_bricks = app
        .world_mut()
        .query_filtered::<Entity, (
            With<Brick>,
            With<avian3d::prelude::Collider>,
        )>()
        .iter(app.world())
        .filter(|e| {
            matches!(
                app.world().get::<avian3d::prelude::Collider>(*e).unwrap().shape().as_typed_shape(),
                avian3d::parry::shape::TypedShape::TriMesh(_)
            )
        })
        .count();
    assert_eq!(trimesh_bricks, 0, "bricks must not get trimesh colliders");

    let bodies = app
        .world_mut()
        .query_filtered::<Entity, With<avian3d::prelude::RigidBody>>()
        .iter(app.world())
        .count();
    assert_eq!(bodies, index as usize + 1);
}
