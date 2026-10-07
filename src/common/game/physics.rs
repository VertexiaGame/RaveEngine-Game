use crate::common::game::assets::MeshAssetCache;
use crate::common::game::assets::components::Mesh as MeshComponent;
use avian3d::dynamics::rigid_body::sleeping::TimeToSleep;
use avian3d::dynamics::solver::SolverConfig;
use avian3d::dynamics::solver::schedule::SubstepCount;
use avian3d::prelude::*;
use bevy::prelude::*;

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

const BRICK_LINEAR_DAMPING: f32 = 0.04;
const BRICK_ANGULAR_DAMPING: f32 = 0.05;

const MAX_PHYSICS_POSITION: f32 = 100_000.0;
const MAX_PHYSICS_VELOCITY: f32 = 5_000.0;

pub const SERVER_SUBSTEP_COUNT: u32 = 4;
pub const SERVER_TIME_TO_SLEEP: f32 = 0.3;
pub const SERVER_RESTITUTION_ITERATIONS: usize = 1;

const SANITIZE_EVERY_N_STEPS: u32 = 10;

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
            .add_systems(
                Update,
                (
                    handle_physics_simulation_actions,
                    handle_newly_spawned_bricks,
                ),
            )
            .add_systems(
                PhysicsSchedule,
                sanitize_physics_state.in_set(PhysicsStepSystems::Finalize),
            );
    }
}

fn sanitize_physics_state(
    mut step: Local<u32>,
    mut bodies: Query<
        (
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut AngularVelocity,
        ),
        Or<(With<SleepingDisabled>, With<GravityScale>)>,
    >,
    mut colliders: Query<&mut Transform, (With<Collider>, Changed<Transform>)>,
) {
    *step += 1;
    if *step % SANITIZE_EVERY_N_STEPS == 0 {
        for (mut position, mut rotation, mut linear_velocity, mut angular_velocity) in
            &mut bodies
        {
            if !position.0.is_finite() {
                position.0 = Vec3::ZERO;
            } else {
                let clamped = position.0.clamp(
                    Vec3::splat(-MAX_PHYSICS_POSITION),
                    Vec3::splat(MAX_PHYSICS_POSITION),
                );
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
                let clamped = linear_velocity.0.clamp(
                    Vec3::splat(-MAX_PHYSICS_VELOCITY),
                    Vec3::splat(MAX_PHYSICS_VELOCITY),
                );
                if clamped != linear_velocity.0 {
                    linear_velocity.0 = clamped;
                }
            }

            if !angular_velocity.0.is_finite() {
                angular_velocity.0 = Vec3::ZERO;
            } else {
                let clamped = angular_velocity.0.clamp(
                    Vec3::splat(-MAX_PHYSICS_VELOCITY),
                    Vec3::splat(MAX_PHYSICS_VELOCITY),
                );
                if clamped != angular_velocity.0 {
                    angular_velocity.0 = clamped;
                }
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
    mut commands: Commands,
    mut time_physics: ResMut<Time<Physics>>,
    mut solver_config: ResMut<SolverConfig>,
    mut state: ResMut<PhysicsSimulationState>,
    server_settings: Option<Res<crate::server::ServerSettings>>,
) {
    if server_settings.is_none() {
        time_physics.pause();
    } else {
        *state = PhysicsSimulationState::Running;
        time_physics.unpause();
        commands.insert_resource(SubstepCount(SERVER_SUBSTEP_COUNT));
        commands.insert_resource(TimeToSleep(SERVER_TIME_TO_SLEEP));
        solver_config.restitution_iterations = SERVER_RESTITUTION_ITERATIONS;
    }
}

fn attach_brick_physics(
    commands: &mut Commands,
    entity: Entity,
    transform: &Transform,
    client_mode: bool,
    is_brick: bool,
    shape_opt: Option<&crate::common::game::bricks::components::BrickShapeComponent>,
    phys_opt: Option<&crate::common::game::bricks::components::BrickPhysics>,
    mesh3d_opt: Option<&Mesh3d>,
    mesh_assets: Option<&Assets<Mesh>>,
    mesh_comp_opt: Option<&MeshComponent>,
    collider_cache: Option<&MeshAssetCache>,
) -> bool {
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

    let (enabled, bounciness, player_can_collide, friction, gravity_scale, mass) =
        if let Some(phys) = phys_opt {
            (
                phys.enabled,
                phys.bounciness,
                phys.player_can_collide,
                phys.friction,
                phys.gravity_scale,
                phys.mass,
            )
        } else {
            (true, 0.0, true, 0.3, 1.0, 1.0)
        };

    let mut collider = if is_brick {
        let shape = shape_opt
            .map(|s| s.shape)
            .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
        Some(
            crate::common::game::bricks::brick_collider_for_shape(shape),
        )
    } else if let Some(mesh_comp) = mesh_comp_opt {
        // Mesh entities collide with the actual model geometry, never a brick.
        // The collider is cached once the model data is fetched, so this may
        // not be ready yet; the caller retries until it is.
        let Some(cache) = collider_cache else {
            return false;
        };
        match cache
            .colliders
            .get(&(mesh_comp.asset_id, mesh_comp.normalize))
        {
            Some(collider) => Some(collider.clone()),
            None => return false,
        }
    } else if let Some(mesh3d) = mesh3d_opt {
        match mesh_assets.and_then(|assets| assets.get(mesh3d)) {
            Some(mesh) => match Collider::trimesh_from_mesh(mesh) {
                Some(collider) => Some(collider),
                None => return false,
            },
            None => return false,
        }
    } else {
        let shape = shape_opt
            .map(|s| s.shape)
            .unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
        Some(
            crate::common::game::bricks::brick_collider_for_shape(shape),
        )
    };
    collider.as_mut().unwrap().set_scale(scale, 32);

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
            collider.unwrap(),
            Friction::new(friction),
            Restitution::new(bounciness),
            GravityScale(gravity_scale),
            Mass(mass),
            LinearDamping(BRICK_LINEAR_DAMPING),
            AngularDamping(BRICK_ANGULAR_DAMPING),
            SleepThreshold {
                linear: 0.25,
                angular: 0.35,
            },
            layers,
            PhysicsAttached,
        ));
    } else {
        commands.entity(entity).insert((
            transform,
            RigidBody::Static,
            collider.unwrap(),
            Friction::new(friction),
            Restitution::new(0.0),
            layers,
            PhysicsAttached,
        ));
    }
    true
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
    bricks_query: Query<
        (
            Entity,
            &Transform,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::common::game::bricks::components::BrickShapeComponent>,
            Option<&crate::common::game::bricks::components::BrickPhysics>,
            Option<&TransformBackup>,
            Option<&Mesh3d>,
            Option<&MeshComponent>,
        ),
        Or<(
            With<crate::common::game::bricks::components::Brick>,
            With<MeshComponent>,
        )>,
    >,
    mesh_assets: Res<Assets<Mesh>>,
    mesh_cache: Res<MeshAssetCache>,
) {
    let client_mode = crate::client::is_playtesting(playtest);
    for action in actions.read() {
        match *action {
            PhysicsSimulationAction::Play => {
                if *state == PhysicsSimulationState::Stopped {
                    *state = PhysicsSimulationState::Running;
                    time_physics.unpause();

                    for (
                        entity,
                        transform,
                        brick_opt,
                        shape_opt,
                        phys_opt,
                        backup,
                        mesh3d_opt,
                        mesh_comp_opt,
                    ) in &bricks_query
                    {
                        if attach_brick_physics(
                            &mut commands,
                            entity,
                            transform,
                            client_mode,
                            brick_opt.is_some(),
                            shape_opt,
                            phys_opt,
                            mesh3d_opt,
                            Some(&mesh_assets),
                            mesh_comp_opt,
                            Some(&mesh_cache),
                        ) {
                            if backup.is_none() {
                                commands.entity(entity).insert(TransformBackup(*transform));
                            }
                        }
                    }
                }
            }
            PhysicsSimulationAction::Stop => {
                if *state == PhysicsSimulationState::Running {
                    *state = PhysicsSimulationState::Stopped;
                    time_physics.pause();

                    for (entity, _, _, _, _, backup, _, _) in &bricks_query {
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

                for (
                    entity,
                    transform,
                    brick_opt,
                    shape_opt,
                    phys_opt,
                    backup,
                    mesh3d_opt,
                    mesh_comp_opt,
                ) in &bricks_query
                {
                    if let Some(backup_val) = backup {
                        commands.entity(entity).insert(backup_val.0);
                    } else {
                        commands.entity(entity).insert(TransformBackup(*transform));
                    }
                    detach_brick_physics(&mut commands, entity);
                    if !attach_brick_physics(
                        &mut commands,
                        entity,
                        transform,
                        client_mode,
                        brick_opt.is_some(),
                        shape_opt,
                        phys_opt,
                        mesh3d_opt,
                        Some(&mesh_assets),
                        mesh_comp_opt,
                        Some(&mesh_cache),
                    ) {
                        // Collider data not ready yet; drop the backup so the
                        // entity gets retried once the model finishes loading.
                        commands.entity(entity).remove::<TransformBackup>();
                    }
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
    mesh_assets: Res<Assets<Mesh>>,
    mesh_cache: Res<MeshAssetCache>,
    query: Query<
        (
            Entity,
            &Transform,
            Option<&crate::common::game::bricks::components::Brick>,
            Option<&crate::common::game::bricks::components::BrickShapeComponent>,
            Option<&crate::common::game::bricks::components::BrickPhysics>,
            Option<&Mesh3d>,
            Option<&MeshComponent>,
        ),
        (
            Or<(
                With<crate::common::game::bricks::components::Brick>,
                With<MeshComponent>,
            )>,
            Without<TransformBackup>,
            Without<PhysicsAttached>,
        ),
    >,
) {
    if *state == PhysicsSimulationState::Running {
        let client_mode = crate::client::is_playtesting(playtest);
        for (entity, transform, brick_opt, shape_opt, phys_opt, mesh3d_opt, mesh_comp_opt) in
            &query
        {
            if attach_brick_physics(
                &mut commands,
                entity,
                transform,
                client_mode,
                brick_opt.is_some(),
                shape_opt,
                phys_opt,
                mesh3d_opt,
                Some(&mesh_assets),
                mesh_comp_opt,
                Some(&mesh_cache),
            ) {
                commands.entity(entity).insert(TransformBackup(*transform));
            }
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

    fn server_settings() -> crate::server::ServerSettings {
        crate::server::ServerSettings {
            map_path: String::new(),
            port: 0,
            bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            netcode_key: [0u8; 32],
            protocol_id: 0,
            allow_unauthenticated: true,
        }
    }

    #[test]
    fn server_applies_physics_performance_tuning() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        app.insert_resource(server_settings());
        app.finish();
        app.world_mut().run_schedule(Startup);
        assert_eq!(
            app.world().resource::<SubstepCount>().0,
            SERVER_SUBSTEP_COUNT
        );
        assert_eq!(
            app.world().resource::<TimeToSleep>().0,
            SERVER_TIME_TO_SLEEP
        );
        assert_eq!(
            app.world()
                .resource::<SolverConfig>()
                .restitution_iterations,
            SERVER_RESTITUTION_ITERATIONS
        );
        assert_eq!(
            *app.world().resource::<PhysicsSimulationState>(),
            PhysicsSimulationState::Running
        );
    }

    #[test]
    fn non_server_keeps_default_physics_tuning() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        app.finish();
        app.world_mut().run_schedule(Startup);
        assert_eq!(app.world().resource::<SubstepCount>().0, 6);
        assert_eq!(*app.world().resource::<PhysicsSimulationState>(), PhysicsSimulationState::Stopped);
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
        attach_brick_physics(
            &mut commands,
            entity,
            &transform,
            false,
            true,
            Some(&shape),
            None,
            None,
            None,
            None,
            None,
        );
        app.world_mut().flush();

        let collider = app
            .world_mut()
            .query::<&Collider>()
            .get(app.world(), entity)
            .unwrap();
        assert_eq!(collider.scale(), Vec3::new(2.0, 1.0, 2.0));
    }

    #[test]
    fn brick_with_render_mesh_still_gets_analytic_collider() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let shape = crate::common::game::bricks::components::BrickShapeComponent {
            shape: crate::common::game::bricks::components::BrickShape::Block,
        };
        let entity = app.world_mut().spawn((transform, shape)).id();
        let mut commands = app.world_mut().commands();
        // Bricks carry a Mesh3d for rendering; it must never become a trimesh
        // collider, otherwise large scenes drown in trimesh-trimesh contacts.
        let attached = attach_brick_physics(
            &mut commands,
            entity,
            &transform,
            false,
            true,
            Some(&shape),
            None,
            Some(&Mesh3d(Handle::default())),
            None,
            None,
            None,
        );
        assert!(attached);
        app.world_mut().flush();

        let collider = app
            .world_mut()
            .query::<&Collider>()
            .get(app.world(), entity)
            .unwrap();
        assert!(matches!(
            collider.shape().as_typed_shape(),
            avian3d::parry::shape::TypedShape::Cuboid(_)
        ));
    }

    #[test]
    fn mesh_collider_comes_from_cached_model_geometry() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        let mesh_comp = MeshComponent {
            asset_id: 7,
            normalize: false,
        };
        let entity = app
            .world_mut()
            .spawn((Transform::default(), mesh_comp))
            .id();

        let mut cache = MeshAssetCache::default();
        let collider = Collider::trimesh(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        cache.colliders.insert((7, false), collider);

        let mut commands = app.world_mut().commands();
        let attached = attach_brick_physics(
            &mut commands,
            entity,
            &Transform::default(),
            false,
            false,
            None,
            None,
            None,
            None,
            Some(&mesh_comp),
            Some(&cache),
        );
        assert!(attached);
        app.world_mut().flush();

        let collider = app
            .world_mut()
            .query::<&Collider>()
            .get(app.world(), entity)
            .unwrap();
        assert_eq!(collider.scale(), Vec3::ONE);
        // A brick fallback collider would have a box around (-1.12,-0.28,-0.56)
        // to (1.12,0.28,0.56). The precise model trimesh instead covers exactly
        // the model vertices (0,0,0) to (1,1,1).
        let aabb = collider.aabb(Vec3::ZERO, Quat::IDENTITY);
        assert!(aabb.min.abs_diff_eq(Vec3::ZERO, 1e-4));
        assert!(aabb.max.abs_diff_eq(Vec3::ONE, 1e-4));
    }

    #[test]
    fn mesh_collider_waits_for_loaded_model() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
        let mesh_comp = MeshComponent {
            asset_id: 9,
            normalize: true,
        };
        let entity = app
            .world_mut()
            .spawn((Transform::default(), mesh_comp))
            .id();

        let mut commands = app.world_mut().commands();
        let attached = attach_brick_physics(
            &mut commands,
            entity,
            &Transform::default(),
            false,
            false,
            None,
            None,
            None,
            None,
            Some(&mesh_comp),
            Some(&MeshAssetCache::default()),
        );
        app.world_mut().flush();
        assert!(!attached);
        assert!(
            app.world_mut()
                .query::<&Collider>()
                .get(app.world(), entity)
                .is_err()
        );
    }
}
