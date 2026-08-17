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

#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsAttached;

const BRICK_LINEAR_DAMPING: f32 = 0.1;
const BRICK_ANGULAR_DAMPING: f32 = 0.1;

const MAX_PHYSICS_POSITION: f32 = 100_000.0;
const MAX_PHYSICS_VELOCITY: f32 = 5_000.0;

pub struct PhysicsSimulationPlugin;

impl Plugin for PhysicsSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsLengthUnit(0.28))
            .insert_resource(avian3d::dynamics::solver::SolverConfig {
                max_overlap_solve_speed: 16.0,
                restitution_iterations: 2,
                ..default()
            })
            .add_plugins(PhysicsPlugins::default())
            .insert_resource(Gravity(Vec3::new(0.0, -186.9 * 0.28, 0.0)))
            .init_resource::<PhysicsSimulationState>()
            .add_message::<PhysicsSimulationAction>()
            .add_systems(Startup, setup_physics)
            .add_systems(Update, (
                handle_physics_simulation_actions,
                handle_newly_spawned_bricks,
            ))
            .add_systems(
                PhysicsSchedule,
                sanitize_physics_state.in_set(PhysicsStepSystems::Finalize),
            );
    }
}

fn sanitize_physics_state(
    mut bodies: Query<(
        &mut Position,
        &mut Rotation,
        &mut LinearVelocity,
        &mut AngularVelocity,
    ), Or<(With<SleepingDisabled>, With<GravityScale>)>>,
    mut colliders: Query<&mut Transform, (With<Collider>, Changed<Transform>)>,
) {
    for (mut position, mut rotation, mut linear_velocity, mut angular_velocity) in &mut bodies {
        if !position.0.is_finite() {
            position.0 = Vec3::ZERO;
        } else {
            let clamped = position
                .0
                .clamp(Vec3::splat(-MAX_PHYSICS_POSITION), Vec3::splat(MAX_PHYSICS_POSITION));
            if clamped != position.0 {
                position.0 = clamped;
            }
        }

        if !rotation.0.is_finite() {
            rotation.0 = Quat::IDENTITY;
        }

        if !linear_velocity.0.is_finite() {
            linear_velocity.0 = Vec3::ZERO;
        } else {
            let clamped = linear_velocity.0
                .clamp(Vec3::splat(-MAX_PHYSICS_VELOCITY), Vec3::splat(MAX_PHYSICS_VELOCITY));
            if clamped != linear_velocity.0 {
                linear_velocity.0 = clamped;
            }
        }

        if !angular_velocity.0.is_finite() {
            angular_velocity.0 = Vec3::ZERO;
        } else {
            let clamped = angular_velocity.0
                .clamp(Vec3::splat(-MAX_PHYSICS_VELOCITY), Vec3::splat(MAX_PHYSICS_VELOCITY));
            if clamped != angular_velocity.0 {
                angular_velocity.0 = clamped;
            }
        }
    }

    for mut transform in &mut colliders {
        if !transform.translation.is_finite() {
            transform.translation = Vec3::ZERO;
        }
        let scale = transform.scale;
        let clamped_scale = if !scale.is_finite() {
            Vec3::ONE
        } else {
            scale.max(Vec3::splat(0.01))
        };
        if clamped_scale != scale {
            transform.scale = clamped_scale;
        }
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
    transform: &Transform,
    client_mode: bool,
    shape_opt: Option<&crate::common::game::bricks::components::BrickShapeComponent>,
    phys_opt: Option<&crate::common::game::bricks::components::BrickPhysics>,
) {
    let mut transform = *transform;
    if !transform.translation.is_finite() {
        transform.translation = Vec3::ZERO;
    }
    let scale = transform.scale;
    if !scale.is_finite() {
        transform.scale = Vec3::ONE;
    } else {
        transform.scale = scale.max(Vec3::splat(0.01));
    }

    let (enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = phys_opt {
        (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
    } else {
        (true, 0.0, true, 0.3, 1.0, 1.0)
    };

    let shape = shape_opt.map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
    let mut collider = match shape {
        crate::common::game::bricks::components::BrickShape::Block => {
            Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28)
        }
        crate::common::game::bricks::components::BrickShape::Sphere => {
            Collider::sphere(1.0 * 0.28)
        }
    };
    collider.set_scale(scale, 32);

    let layers = if player_can_collide {
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
    } else {
        CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
    };

    let client_body_type = if client_mode {
        RigidBody::Static
    } else {
        RigidBody::Dynamic
    };

    if enabled {
        commands.entity(entity).insert((
            transform,
            client_body_type,
            collider,
            Friction::new(friction),
            Restitution::new(bounciness),
            GravityScale(gravity_scale),
            Mass(mass),
            LinearDamping(BRICK_LINEAR_DAMPING),
            AngularDamping(BRICK_ANGULAR_DAMPING),
            SleepThreshold {
                linear: 0.5,
                angular: 0.5,
            },
            layers,
            PhysicsAttached,
        ));
    } else {
        commands.entity(entity).insert((
            transform,
            RigidBody::Static,
            collider,
            Friction::new(friction),
            Restitution::new(0.0),
            layers,
            PhysicsAttached,
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
        SleepThreshold,
        PhysicsAttached,
    )>();
}

fn handle_physics_simulation_actions(
    mut actions: MessageReader<PhysicsSimulationAction>,
    mut state: ResMut<PhysicsSimulationState>,
    mut time_physics: ResMut<Time<Physics>>,
    mut commands: Commands,
    playtest: Option<Res<crate::client::PlaytestState>>,
    bricks_query: Query<(
        Entity,
        &Transform,
        Option<&crate::common::game::bricks::components::BrickShapeComponent>,
        Option<&crate::common::game::bricks::components::BrickPhysics>,
        Option<&TransformBackup>,
    ), With<crate::common::game::bricks::components::Brick>>,
) {
    let client_mode = crate::client::is_playtesting(playtest);
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
                        attach_brick_physics(&mut commands, entity, transform, client_mode, shape_opt, phys_opt);
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
                    attach_brick_physics(&mut commands, entity, transform, client_mode, shape_opt, phys_opt);
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
    playtest: Option<Res<crate::client::PlaytestState>>,
    query: Query<(Entity, &Transform, Option<&crate::common::game::bricks::components::BrickShapeComponent>, Option<&crate::common::game::bricks::components::BrickPhysics>), (With<crate::common::game::bricks::components::Brick>, Without<TransformBackup>, Without<PhysicsAttached>)>,
) {
    if *state == PhysicsSimulationState::Running {
        let client_mode = crate::client::is_playtesting(playtest);
        for (entity, transform, shape_opt, phys_opt) in &query {
            commands.entity(entity).insert(TransformBackup(*transform));
            attach_brick_physics(&mut commands, entity, transform, client_mode, shape_opt, phys_opt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_schedule_initializes_without_ambiguity_errors() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        app.finish();
        app.world_mut().run_schedule(PhysicsSchedule);
    }

    #[test]
    fn brick_collider_scale_matches_transform_scale() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        let transform = Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(2.0, 1.0, 2.0));
        let shape = crate::common::game::bricks::components::BrickShapeComponent {
            shape: crate::common::game::bricks::components::BrickShape::Sphere,
        };
        let entity = app.world_mut().spawn((transform, shape)).id();
        let mut commands = app.world_mut().commands();
        attach_brick_physics(&mut commands, entity, &transform, false, Some(&shape), None);
        app.world_mut().flush();

        let collider = app.world_mut().query::<&Collider>().get(app.world(), entity).unwrap();
        assert_eq!(collider.scale(), Vec3::new(2.0, 1.0, 2.0));
    }
}
