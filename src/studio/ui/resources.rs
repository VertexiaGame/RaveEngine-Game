use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct CopiedEntityBuffer {
    pub transform: Option<Transform>,
    pub mesh: Option<Mesh3d>,
    pub material: Option<MeshMaterial3d<StandardMaterial>>,
    pub studs_material: Option<MeshMaterial3d<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    pub name: Option<String>,
    pub is_brick: bool,
    pub shape: crate::common::game::bricks::components::BrickShape,
    pub physics: Option<crate::common::game::bricks::components::BrickPhysics>,
    pub show_studs: bool,
}

#[derive(Resource, Default)]
pub struct HierarchyDraggedEntity {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SettingsWindow {
    pub open: bool,
}

#[derive(Resource, Default)]
pub struct PlayInClientProcesses {
    pub client_process: Option<std::process::Child>,
}

#[derive(Resource, Default)]
pub struct PlaytestBackup {
    pub bricks: Vec<crate::common::game::bricks::data::BrickData>,
    pub scripts: Vec<crate::common::core::vrtx::VrtxScript>,
    pub gravity: Option<Vec3>,
    pub players_service: Option<crate::studio::tools::PlayersService>,
}

#[derive(Component)]
pub struct InEditorPlaytestClient;

#[derive(Resource, Default)]
pub struct ActiveScriptEditor {
    pub entity: Option<Entity>,
    pub open: bool,
    pub buffer: String,
    pub error: Option<String>,
    pub open_entities: Vec<Entity>,
}

pub enum FileDialogResult {
    BrowseSavePath(std::path::PathBuf),
    OpenFile(std::path::PathBuf),
    SaveAs(std::path::PathBuf),
    Cancel,
}

#[derive(Resource)]
pub struct FileDialogState {
    pub tx: std::sync::mpsc::Sender<FileDialogResult>,
    pub rx: std::sync::Mutex<std::sync::mpsc::Receiver<FileDialogResult>>,
    pub is_open: std::sync::atomic::AtomicBool,
}

impl Default for FileDialogState {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: std::sync::Mutex::new(rx),
            is_open: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

pub fn cleanup_play_processes_on_exit(
    events: MessageReader<AppExit>,
    mut play_processes: ResMut<PlayInClientProcesses>,
) {
    if !events.is_empty() {
        crate::app::server::bootstrap::SHUTDOWN_SERVER.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(mut child) = play_processes.client_process.take() {
            let _ = child.kill();
        }
    }
}

pub fn handle_file_dialog_results(
    mut commands: Commands,
    file_dialog_state: Res<FileDialogState>,
    mut onboarding_data: ResMut<crate::studio::ui::panels::onboarding::OnboardingData>,
    mut graphics_settings: ResMut<crate::common::core::performance::GraphicsSettings>,
    mut gravity: Option<ResMut<avian3d::prelude::Gravity>>,
    mut camera_transform_query: Query<&mut Transform, With<Camera3d>>,
    materials: ResMut<Assets<StandardMaterial>>,
    studs_materials: ResMut<Assets<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    entities_query: Query<(Entity, Option<&crate::common::game::bricks::components::Brick>), Without<Camera3d>>,
    explorer_query: Query<(Entity, Option<&crate::scripting::ecs::ServerScript>, Option<&crate::scripting::ecs::LocalScript>, Option<&crate::scripting::ecs::ModuleScript>), Without<Camera3d>>,
    save_query: Query<(
        Entity,
        &Transform,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&crate::common::game::bricks::components::Brick>,
        Option<&crate::common::game::bricks::components::BrickShapeComponent>,
        &GlobalTransform,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<bevy::pbr::ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
        Option<&crate::common::game::bricks::components::BrickPhysics>,
    ), Without<Camera3d>>,
    save_explorer_query: Query<(
        Entity,
        &Name,
        Option<&ChildOf>,
        Option<&Children>,
        Option<&crate::common::game::bricks::components::Brick>,
        Option<&crate::scripting::ecs::ServerScript>,
        Option<&crate::scripting::ecs::LocalScript>,
        Option<&crate::scripting::ecs::ModuleScript>,
    ), Without<Camera3d>>,
    studs_query: Query<&crate::common::game::bricks::components::BrickStuds>,
) {
    let rx = file_dialog_state.rx.lock().unwrap();
    while let Ok(result) = rx.try_recv() {
        match result {
            FileDialogResult::BrowseSavePath(path) => {
                onboarding_data.save_path = path.display().to_string();
                file_dialog_state.is_open.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::OpenFile(path) => {
                let open_path_str = path.display().to_string();
                if let Ok(state) = crate::common::core::vrtx::VrtxFileState::load_from_file(&open_path_str) {
                    onboarding_data.save_path = open_path_str;
                    for (entity, brick_opt) in &entities_query {
                        if brick_opt.is_some() {
                            commands.entity(entity).try_despawn();
                        }
                    }
                    for (entity, s_opt, l_opt, m_opt) in &explorer_query {
                        if s_opt.is_some() || l_opt.is_some() || m_opt.is_some() {
                            commands.entity(entity).try_despawn();
                        }
                    }
                    graphics_settings.ssao = state.settings.ssao;
                    graphics_settings.contact_shadows = state.settings.contact_shadows;
                    graphics_settings.bloom = state.settings.bloom;
                    if let Some(ref mut g) = gravity {
                        g.0 = state.gravity;
                    }
                    if let Some(mut cam_t) = camera_transform_query.iter_mut().next() {
                        *cam_t = state.camera_transform;
                    }
                    let mut named_entities = std::collections::HashMap::new();
                    for brick in state.bricks {
                        let layers = if brick.player_can_collide {
                            avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF)
                        } else {
                            avian3d::prelude::CollisionLayers::from_bits(0b0100, 0xFFFF_FFFD)
                        };
                        let new_id = commands.spawn((
                            brick.transform,
                            crate::common::game::bricks::components::Brick,
                            crate::common::game::bricks::components::BrickShapeComponent { shape: brick.shape },
                            crate::common::game::bricks::components::BrickPhysics {
                                enabled: brick.physics_enabled,
                                bounciness: brick.bounciness,
                                player_can_collide: brick.player_can_collide,
                                friction: brick.friction,
                                gravity_scale: brick.gravity_scale,
                                mass: brick.mass,
                            },
                            crate::common::game::bricks::components::BrickColor { color: brick.color },
                            layers,
                            Pickable::default(),
                            Name::new(brick.name.clone()),
                        )).id();
                        named_entities.insert(brick.name, new_id);
                    }
                    for script in state.scripts {
                        let mut cmd = commands.spawn(Name::new(script.name));
                        match script.script_type {
                            0 => {
                                cmd.insert(crate::scripting::ecs::ServerScript {
                                    code: script.code,
                                    enabled: script.enabled,
                                    started: false,
                                    running_code: String::new(),
                                });
                            }
                            1 => {
                                cmd.insert((
                                    crate::scripting::ecs::LocalScript {
                                        code: script.code,
                                        enabled: script.enabled,
                                        started: false,
                                        running_code: String::new(),
                                    },
                                    lightyear::prelude::Replicate::default(),
                                ));
                            }
                            _ => {
                                cmd.insert((
                                    crate::scripting::ecs::ModuleScript {
                                        code: script.code,
                                    },
                                    lightyear::prelude::Replicate::default(),
                                ));
                            }
                        }
                        let new_script_entity = cmd.id();
                        if let Some(ref p_name) = script.parent_name {
                            if let Some(&parent_entity) = named_entities.get(p_name) {
                                commands.entity(parent_entity).add_child(new_script_entity);
                            }
                        }
                    }
                }
                file_dialog_state.is_open.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::SaveAs(path) => {
                let save_path_str = path.display().to_string();
                onboarding_data.save_path = save_path_str.clone();

                let mut bricks_data = Vec::new();
                for (entity, transform, name, _, _, brick_opt, shape_opt, _, _, mat_opt, studs_mat_opt, phys_opt) in &save_query {
                    if brick_opt.is_some() {
                        let shape = shape_opt.as_ref().map(|s| s.shape).unwrap_or(crate::common::game::bricks::components::BrickShape::Block);
                        let mut current_color = Color::Srgba(Srgba::new(0.84, 0.24, 0.16, 1.0));
                        if let Some(studs_mat_handle) = studs_mat_opt {
                            if let Some(mat) = studs_materials.get(&studs_mat_handle.0) {
                                current_color = mat.base.base_color;
                            }
                        } else if let Some(mat_handle) = mat_opt {
                            if let Some(mat) = materials.get(&mat_handle.0) {
                                current_color = mat.base_color;
                            }
                        }
                        let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) = if let Some(phys) = phys_opt {
                            (phys.enabled, phys.bounciness, phys.player_can_collide, phys.friction, phys.gravity_scale, phys.mass)
                        } else {
                            (true, 0.3, true, 0.3, 1.0, 1.0)
                        };
                        bricks_data.push(crate::common::core::vrtx::VrtxBrick {
                            name: name.to_string(),
                            transform: *transform,
                            shape,
                            color: current_color,
                            physics_enabled,
                            bounciness,
                            player_can_collide,
                            friction,
                            gravity_scale,
                            mass,
                            show_studs: studs_query.get(entity).map(|s| s.enabled).unwrap_or(true),
                        });
                    }
                }

                let mut scripts_data = Vec::new();
                for (_entity, name, child_of_opt, _, _, s_opt, l_opt, m_opt) in &save_explorer_query {
                    let mut script_type_opt = None;
                    let mut code = String::new();
                    let mut enabled = true;
                    if let Some(s) = s_opt {
                        script_type_opt = Some(0);
                        code = s.code.clone();
                        enabled = s.enabled;
                    } else if let Some(l) = l_opt {
                        script_type_opt = Some(1);
                        code = l.code.clone();
                        enabled = l.enabled;
                    } else if let Some(m) = m_opt {
                        script_type_opt = Some(2);
                        code = m.code.clone();
                    }
                    if let Some(script_type) = script_type_opt {
                        let mut parent_name = None;
                        if let Some(child_of) = child_of_opt {
                            if let Ok((_, p_name, _, _, _, _, _, _)) = save_explorer_query.get(child_of.parent()) {
                                parent_name = Some(p_name.to_string());
                            }
                        }
                        scripts_data.push(crate::common::core::vrtx::VrtxScript {
                            name: name.to_string(),
                            script_type,
                            code,
                            parent_name,
                            enabled,
                        });
                    }
                }

                let gravity_val = if let Some(g) = gravity.as_ref() {
                    g.0
                } else {
                    Vec3::new(0.0, -186.9 * 0.28, 0.0)
                };
                let cam_transform = if let Some(cam_t) = camera_transform_query.iter().next() {
                    *cam_t
                } else {
                    Transform::IDENTITY
                };
                let state = crate::common::core::vrtx::VrtxFileState {
                    version: 6,
                    gravity: gravity_val,
                    settings: crate::common::core::vrtx::VrtxSettings {
                        ssao: graphics_settings.ssao,
                        contact_shadows: graphics_settings.contact_shadows,
                        bloom: graphics_settings.bloom,
                    },
                    camera_transform: cam_transform,
                    bricks: bricks_data,
                    scripts: scripts_data,
                };
                let _ = state.save_to_file(&save_path_str);
                file_dialog_state.is_open.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            FileDialogResult::Cancel => {
                file_dialog_state.is_open.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}