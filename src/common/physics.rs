use bevy::prelude::*;
use avian3d::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhysicsSimulationState {
    #[default]
    Stopped,
    Running,
}

#[derive(Message, Clone, Copy, Debug)]
pub enum PhysicsSimulationAction {
    Play,
    Stop,
    Replay,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct TransformBackup(pub Transform);

pub struct PhysicsSimulationPlugin;


////////
//Physics simulation MOSTLY  for the studio for now. TODO:
//IMPLEMENT proper physics plugin for server & client replication
//move physics to folder
//remove things specific to the current playtest (this is basically just a showcase), like "baseplate" sdhould just become a checkbox "enable physics"... :3




impl Plugin for PhysicsSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(Gravity(Vec3::new(0.0, -186.9 * 0.28, 0.0)))
            .init_resource::<PhysicsSimulationState>()
            .add_message::<PhysicsSimulationAction>()
            .add_systems(Startup, setup_physics)
            .add_systems(Update, (
                handle_physics_simulation_actions,
                handle_newly_spawned_bricks,
            ));
    }
}

fn setup_physics(
    mut time_physics: ResMut<Time<Physics>>,
    mut state: ResMut<PhysicsSimulationState>,
    server_settings: Option<Res<crate::server::ServerSettings>>,
) {
    if server_settings.is_none() {
        time_physics.pause();
    } else {
        *state = PhysicsSimulationState::Running;
        time_physics.unpause();
    }
}

fn handle_physics_simulation_actions(
    mut actions: MessageReader<PhysicsSimulationAction>,
    mut state: ResMut<PhysicsSimulationState>,
    mut time_physics: ResMut<Time<Physics>>,
    mut commands: Commands,
    mut bricks_query: Query<(
        Entity,
        &mut Transform,
        &Name,
        Option<&crate::common::bricks::components::BrickShapeComponent>,
        Option<&crate::common::bricks::components::BrickPhysics>,
        Option<&TransformBackup>,
    ), With<crate::common::bricks::components::Brick>>,
) {
    for action in actions.read() {
        match *action {
            PhysicsSimulationAction::Play => {
                if *state == PhysicsSimulationState::Stopped {
                    *state = PhysicsSimulationState::Running;
                    time_physics.unpause();

                    for (entity, transform, _name, shape_opt, phys_opt, backup) in &bricks_query {
                        if backup.is_none() {
                            commands.entity(entity).insert(TransformBackup(*transform));
                        }

                        let (enabled, bounciness) = if let Some(phys) = phys_opt {
                            (phys.enabled, phys.bounciness)
                        } else {
                            (true, 0.3)
                        };

                        let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::bricks::components::BrickShape::Block);
                        let collider = match shape {
                            crate::common::bricks::components::BrickShape::Block => {
                                Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                            }
                            crate::common::bricks::components::BrickShape::Sphere => {
                                Collider::sphere(1.0 * 0.28)
                            }
                        };

                        if enabled {
                            commands.entity(entity).insert((
                                RigidBody::Dynamic,
                                collider,
                                Friction::new(0.3),
                                Restitution::new(bounciness),
                                SleepingDisabled,
                            ));
                        } else {
                            commands.entity(entity).insert((
                                RigidBody::Static,
                                collider,
                                Friction::new(0.3),
                                Restitution::new(0.0),
                            ));
                        }
                    }
                }
            }
            PhysicsSimulationAction::Stop => {
                if *state == PhysicsSimulationState::Running {
                    *state = PhysicsSimulationState::Stopped;
                    time_physics.pause();

                    for (entity, mut transform, _, _, _, backup) in &mut bricks_query {
                        if let Some(backup_val) = backup {
                            *transform = backup_val.0;
                            commands.entity(entity).remove::<TransformBackup>();
                        }
                        commands.entity(entity).remove::<(
                            RigidBody,
                            Collider,
                            Friction,
                            Restitution,
                            Mass,
                            LinearVelocity,
                            AngularVelocity,
                            SleepingDisabled,
                        )>();
                    }
                }
            }
            PhysicsSimulationAction::Replay => {
                if *state == PhysicsSimulationState::Running {
                    for (entity, mut transform, _, _, _, backup) in &mut bricks_query {
                        if let Some(backup_val) = backup {
                            *transform = backup_val.0;
                        }
                        commands.entity(entity).remove::<(
                            RigidBody,
                            Collider,
                            Friction,
                            Restitution,
                            Mass,
                            LinearVelocity,
                            AngularVelocity,
                            SleepingDisabled,
                        )>();
                    }
                } else {
                    *state = PhysicsSimulationState::Running;
                    time_physics.unpause();
                }

                for (entity, transform, _name, shape_opt, phys_opt, backup) in &bricks_query {
                    if backup.is_none() {
                        commands.entity(entity).insert(TransformBackup(*transform));
                    }

                    let (enabled, bounciness) = if let Some(phys) = phys_opt {
                        (phys.enabled, phys.bounciness)
                    } else {
                        (true, 0.3)
                    };

                    let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::bricks::components::BrickShape::Block);
                    let collider = match shape {
                        crate::common::bricks::components::BrickShape::Block => {
                            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                        }
                        crate::common::bricks::components::BrickShape::Sphere => {
                            Collider::sphere(1.0 * 0.28)
                        }
                    };

                    if enabled {
                        commands.entity(entity).insert((
                            RigidBody::Dynamic,
                            collider,
                            Friction::new(0.3),
                            Restitution::new(bounciness),
                            SleepingDisabled,
                        ));
                    } else {
                        commands.entity(entity).insert((
                            RigidBody::Static,
                            collider,
                            Friction::new(0.3),
                            Restitution::new(0.0),
                        ));
                    }
                }
            }
        }
    }
}

fn handle_newly_spawned_bricks(
    mut commands: Commands,
    state: Res<PhysicsSimulationState>,
    query: Query<(Entity, &Transform, &Name, Option<&crate::common::bricks::components::BrickShapeComponent>, Option<&crate::common::bricks::components::BrickPhysics>), (With<crate::common::bricks::components::Brick>, Without<TransformBackup>)>,
) {
    if *state == PhysicsSimulationState::Running {
        for (entity, transform, _name, shape_opt, phys_opt) in &query {
            commands.entity(entity).insert(TransformBackup(*transform));

            let (enabled, bounciness) = if let Some(phys) = phys_opt {
                (phys.enabled, phys.bounciness)
            } else {
                (true, 0.3)
            };

            let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::bricks::components::BrickShape::Block);
            let collider = match shape {
                crate::common::bricks::components::BrickShape::Block => {
                    Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
                }
                crate::common::bricks::components::BrickShape::Sphere => {
                    Collider::sphere(1.0 * 0.28)
                }
            };

            if enabled {
                commands.entity(entity).insert((
                    RigidBody::Dynamic,
                    collider,
                    Friction::new(0.3),
                    Restitution::new(bounciness),
                    SleepingDisabled,
                ));
            } else {
                commands.entity(entity).insert((
                    RigidBody::Static,
                    collider,
                    Friction::new(0.3),
                    Restitution::new(0.0),
                ));
            }
        }
    }
}