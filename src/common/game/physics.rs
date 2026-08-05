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

const BRICK_LINEAR_DAMPING: f32 = 0.1;
const BRICK_ANGULAR_DAMPING: f32 = 0.1;

pub struct PhysicsSimulationPlugin;

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

fn attach_brick_physics(
    commands: &mut Commands,
    entity: Entity,
    shape_opt: Option<&crate::common::game::bricks::components::BrickShapeComponent>,
    phys_opt: Option<&crate::common::game::bricks::components::BrickPhysics>,
) {
    let (enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = phys_opt {
        (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
    } else {
        (true, 0.3, true, 0.3, 1.0, 1.0)
    };

    let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
    let collider = match shape {
        crate::common::game::bricks::components::BrickShape::Block => {
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
        }
        crate::common::game::bricks::components::BrickShape::Sphere => {
            Collider::sphere(1.0 * 0.28)
        }
    };

    let layers = if player_can_collide {
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
    } else {
        CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
    };

    if enabled {
        commands.entity(entity).insert((
            RigidBody::Dynamic,
            collider,
            Friction::new(friction),
            Restitution::new(bounciness),
            GravityScale(gravity_scale),
            Mass(mass),
            LinearDamping(BRICK_LINEAR_DAMPING),
            AngularDamping(BRICK_ANGULAR_DAMPING),
            layers,
        ));
    } else {
        commands.entity(entity).insert((
            RigidBody::Static,
            collider,
            Friction::new(friction),
            Restitution::new(0.0),
            layers,
        ));
    }
}

fn detach_brick_physics(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).remove::<(
        RigidBody,
        Collider,
        Friction,
        Restitution,
        Mass,
        LinearVelocity,
        AngularVelocity,
        GravityScale,
        CollisionLayers,
        LinearDamping,
        AngularDamping,
        SleepingDisabled,
    )>();
}

fn handle_physics_simulation_actions(
    mut actions: MessageReader<PhysicsSimulationAction>,
    mut state: ResMut<PhysicsSimulationState>,
    mut time_physics: ResMut<Time<Physics>>,
    mut commands: Commands,
    bricks_query: Query<(
        Entity,
        &Transform,
        Option<&crate::common::game::bricks::components::BrickShapeComponent>,
        Option<&crate::common::game::bricks::components::BrickPhysics>,
        Option<&TransformBackup>,
    ), With<crate::common::game::bricks::components::Brick>>,
) {
    for action in actions.read() {
        match *action {
            PhysicsSimulationAction::Play => {
                if *state == PhysicsSimulationState::Stopped {
                    *state = PhysicsSimulationState::Running;
                    time_physics.unpause();

                    for (entity, transform, shape_opt, phys_opt, backup) in &bricks_query {
                        if backup.is_none() {
                            commands.entity(entity).insert(TransformBackup(*transform));
                        }
                        attach_brick_physics(&mut commands, entity, shape_opt, phys_opt);
                    }
                }
            }
            PhysicsSimulationAction::Stop => {
                if *state == PhysicsSimulationState::Running {
                    *state = PhysicsSimulationState::Stopped;
                    time_physics.pause();

                    for (entity, _, _, _, backup) in &bricks_query {
                        if let Some(backup_val) = backup {
                            commands.entity(entity).insert(backup_val.0);
                            commands.entity(entity).remove::<TransformBackup>();
                        }
                        detach_brick_physics(&mut commands, entity);
                    }
                }
            }
            PhysicsSimulationAction::Replay => {
                if *state == PhysicsSimulationState::Running {
                    *state = PhysicsSimulationState::Stopped;
                    time_physics.pause();
                }

                for (entity, transform, shape_opt, phys_opt, backup) in &bricks_query {
                    if let Some(backup_val) = backup {
                        commands.entity(entity).insert(backup_val.0);
                    } else {
                        commands.entity(entity).insert(TransformBackup(*transform));
                    }
                    detach_brick_physics(&mut commands, entity);
                    attach_brick_physics(&mut commands, entity, shape_opt, phys_opt);
                }

                *state = PhysicsSimulationState::Running;
                time_physics.unpause();
            }
        }
    }
}

fn handle_newly_spawned_bricks(
    mut commands: Commands,
    state: Res<PhysicsSimulationState>,
    query: Query<(Entity, &Transform, Option<&crate::common::game::bricks::components::BrickShapeComponent>, Option<&crate::common::game::bricks::components::BrickPhysics>), (With<crate::common::game::bricks::components::Brick>, Without<TransformBackup>)>,
) {
    if *state == PhysicsSimulationState::Running {
        for (entity, transform, shape_opt, phys_opt) in &query {
            commands.entity(entity).insert(TransformBackup(*transform));
            attach_brick_physics(&mut commands, entity, shape_opt, phys_opt);
        }
    }
}
