use RaveEngineLib::common::game::physics::{
    PhysicsSimulationPlugin, SERVER_RESTITUTION_ITERATIONS, SERVER_SUBSTEP_COUNT,
    SERVER_TIME_TO_SLEEP,
};
use avian3d::dynamics::rigid_body::sleeping::TimeToSleep;
use avian3d::dynamics::solver::SolverConfig;
use avian3d::dynamics::solver::schedule::SubstepCount;
use avian3d::prelude::*;
use bevy::asset::{AssetEvent, Assets};
use bevy::prelude::*;
use bevy::render::mesh::Mesh;
use bevy::time::TimeUpdateStrategy;
use std::time::{Duration, Instant};

//try to fuck up the physics engine with a bunch of bricks and see if it can handle it

fn stress_app(tuned: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, PhysicsSimulationPlugin));
    app.insert_resource(RaveEngineLib::server::ServerSettings {
        map_path: String::new(),
        port: 0,
        bind_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        netcode_key: [0u8; 32],
        protocol_id: 0,
        allow_unauthenticated: true,
    });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<RaveEngineLib::common::game::assets::MeshAssetCache>();
    app.add_message::<AssetEvent<Mesh>>();
    app.init_resource::<avian3d::collider_tree::ColliderTreeDiagnostics>();
    app.init_resource::<avian3d::collision::CollisionDiagnostics>();
    app.init_resource::<avian3d::dynamics::solver::SolverDiagnostics>();
    app.init_resource::<avian3d::spatial_query::SpatialQueryDiagnostics>();
    app.finish();
    app.world_mut().run_schedule(Startup);
    if tuned {
        assert_eq!(app.world().resource::<SubstepCount>().0, SERVER_SUBSTEP_COUNT);
        assert_eq!(app.world().resource::<TimeToSleep>().0, SERVER_TIME_TO_SLEEP);
        assert_eq!(
            app.world()
                .resource::<SolverConfig>()
                .restitution_iterations,
            SERVER_RESTITUTION_ITERATIONS
        );
    } else {
        app.world_mut().insert_resource(SubstepCount(6));
        app.world_mut().insert_resource(TimeToSleep(0.5));
        app.world_mut()
            .resource_mut::<SolverConfig>()
            .restitution_iterations = 2;
    }

    app.world_mut().spawn((
        Transform::from_xyz(0.0, -0.14, 0.0),
        RigidBody::Static,
        Collider::cuboid(30.0, 0.28, 30.0),
        CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
    ));

    let mut index = 0u32; // 12x12x4 = 576 bricks
    for x in 0..12u32 { 
        for z in 0..12u32 {
            for y in 0..4u32 {
                let pos = Vec3::new(
                    (x as f32 - 5.5) * 1.2,
                    0.6 + y as f32 * 0.34,
                    (z as f32 - 5.5) * 0.65,
                );
                app.world_mut().spawn(( // 
                    Name::new(format!("StressBrick{index}")),
                    Transform::from_translation(pos),
                    RigidBody::Dynamic,
                    Collider::cuboid(4.0 * 0.28, 1.0 * 0.28, 2.0 * 0.28),
                    CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                    Friction::new(0.3),
                    Restitution::new(0.3),
                    GravityScale(1.0),
                    Mass(1.0),
                    LinearDamping(0.1),
                    AngularDamping(0.1),
                    SleepThreshold {
                        linear: 0.5,
                        angular: 0.5,
                    },
                    SleepingDisabled,
                ));
                index += 1;
            }
        }
    }
    app
}

fn awake_count(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, (With<RigidBody>, Without<Sleeping>)>();
    query.iter(app.world()).count()
}

#[test]
fn scratch_physics_stress_report() { //scratch test to see how many bricks can be simulated in a single frame
    for tuned in [false, true] {
        let mut app = stress_app(tuned);
        let start = Instant::now();
        let mut mid_awake = 0;
        for frame in 0..600 {
            app.update();
            if frame == 300 {
                mid_awake = awake_count(&mut app);
            }
        }
        let elapsed = start.elapsed();
        let end_awake = awake_count(&mut app);
        println!(
            "STRESS tuned={tuned} frames=600 wall={elapsed:?} per_frame={:?} awake_mid={mid_awake} awake_end={end_awake}",
            elapsed / 600,
        );
    }
}
