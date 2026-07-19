use bevy::prelude::*;
use bevy::log::LogPlugin;
use bevy::state::app::StatesPlugin;
use RaveEngineLib::server::ServerPlugin;
use RaveEngineLib::common::CommonPlugin;
#[cfg(feature = "bench")]
use avian3d::prelude::*;
#[cfg(feature = "bench")]
use RaveEngineLib::common::net::components::Player;
#[cfg(feature = "bench")]
use RaveEngineLib::common::net::components::NetworkTransform;

#[cfg(feature = "bench")]
fn spawn_bench_players(mut commands: Commands) {
    for i in 0..10u64 {
        commands.spawn((
            Name::new(format!("BenchPlayer{}", i)),
            Player {
                client_id: 1000 + i,
                speed: 16.0 * 0.28,
                jump_power: 50.0 * 0.28,
                username: format!("BenchPlayer{}", i),
            },
            Transform::from_xyz(i as f32 * 2.0, 5.0 + i as f32 * 0.5, 0.0),
            NetworkTransform {
                translation: Vec3::new(i as f32 * 2.0, 5.0 + i as f32 * 0.5, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            RigidBody::Dynamic,
            Collider::capsule(1.0 * 0.28, 3.0 * 0.28),
            CollisionLayers::from_bits(0b0010, 0b0011),
            LockedAxes::ROTATION_LOCKED,
            CollidingEntities::default(),
            SleepingDisabled,
            lightyear::prelude::Replicate::default(),
        ));
    }
    info!("BENCH: Spawned 10 simulated players");
}

#[cfg(feature = "bench")]
fn bench_move_players(
    mut query: Query<(&mut Transform, &mut LinearVelocity, &Player)>,
    mut frame: Local<u64>,
) {
    *frame += 1;
    let t = *frame as f32 * 0.05;
    for (mut transform, mut lin_vel, player) in &mut query {
        let idx = (player.client_id - 1000) as f32;
        let dx = (t + idx).sin() * 3.0;
        let dz = (t + idx * 1.5).cos() * 3.0;
        lin_vel.x = dx * 0.28;
        lin_vel.z = dz * 0.28;
        if *frame % 60 == idx as u64 {
            lin_vel.y = player.jump_power;
        }
        transform.rotation = Quat::from_rotation_y(t + idx);
    }
}

fn main() {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
    let new_rust_log = if rust_log.is_empty() {
        #[cfg(feature = "bench")]
        { "info,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=error,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string() }
        #[cfg(not(feature = "bench"))]
        { "debug,wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string() }
    } else if !rust_log.contains("offset_allocator") {
        format!("{rust_log},offset_allocator=off")
    } else {
        rust_log
    };
    unsafe {
        std::env::set_var("VERTIGO_APP", "server");
        std::env::set_var("RUST_LOG", new_rust_log);
    }

    let mut port = 5000;
    let mut map_path = "assets/maps/default.vrtx".to_string();
    #[cfg(feature = "bench")]
    let mut bench_mode = false;
    #[cfg(feature = "bench")]
    let mut bench_frames: u64 = 500;
    #[cfg(feature = "bench")]
    let mut bench_warmup: u64 = 100;
    #[cfg(feature = "bench")]
    let mut bench_scenario = "server".to_string();

    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(p) = args[i + 1].parse::<u16>() {
                port = p;
            }
        }
        if args[i] == "--map" && i + 1 < args.len() {
            map_path = args[i + 1].clone();
        }
        #[cfg(feature = "bench")]
        if args[i] == "--benchmark" {
            bench_mode = true;
        }
        #[cfg(feature = "bench")]
        if args[i] == "--bench-frames" && i + 1 < args.len() {
            if let Ok(f) = args[i + 1].parse::<u64>() {
                bench_frames = f;
            }
        }
        #[cfg(feature = "bench")]
        if args[i] == "--bench-warmup" && i + 1 < args.len() {
            if let Ok(f) = args[i + 1].parse::<u64>() {
                bench_warmup = f;
            }
        }
        #[cfg(feature = "bench")]
        if args[i] == "--bench-scenario" && i + 1 < args.len() {
            bench_scenario = args[i + 1].clone();
        }
    }

    let mut app = App::new();
    app.add_plugins(LogPlugin {
        level: bevy::log::Level::DEBUG,
        filter: "wgpu=error,bevy_render=error,bevy_ecs=warn,lightyear=debug,lightyear_udp=trace,lightyear_netcode=trace,naga=warn,wgpu_hal=warn,wgpu_core=warn,offset_allocator=off".to_string(),
        ..default()
    });
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.add_plugins(StatesPlugin);
    app.add_plugins(TransformPlugin);
    app.add_plugins(CommonPlugin);
    #[cfg(feature = "bench")]
    if !bench_mode || bench_scenario == "server" {
        app.add_plugins(ServerPlugin { map_path: map_path.clone(), port });
    }
    #[cfg(not(feature = "bench"))]
    app.add_plugins(ServerPlugin { map_path, port });

    #[cfg(feature = "bench")]
    if bench_mode {
        if bench_scenario != "server" && bench_scenario != "client" {
            panic!("unsupported benchmark scenario: {bench_scenario}");
        }
        app.world_mut()
            .resource_mut::<RaveEngineLib::common::core::bench::BenchStats>()
            .configure(&bench_scenario, bench_warmup, bench_frames);
        if bench_scenario == "server" {
            app.add_systems(Startup, spawn_bench_players);
            app.add_systems(Update, bench_move_players);
        } else {
            RaveEngineLib::client::add_client_benchmark(&mut app);
        }
        info!("BENCH: Running {} with {} warmup and {} measured frames", bench_scenario, bench_warmup, bench_frames);
    }

    app.run();
}
